//! The VP9 encoder in Rust that `docs/rust-encoder-plan.md` sets out: 8-bit
//! 4:4:4, profile 1, the subset of VP9 the plan's table names and no more.
//! Reachable only from tests until it passes the plan's gate.
//!
//! So far it codes keyframes and inter frames against the frame before, each
//! superblock split as far as its source varies, every block either DC
//! predicted or the previous frame at a whole-pixel vector, at the largest
//! transform that fits it, its residual coded at the dial's quantizer, and the
//! frame reconstructed exactly as libvpx's decoder reconstructs it. Where the
//! source has not changed since the frame before, a block is that frame's,
//! skipped; where it moved, a hash search finds where it came from. Tile
//! columns, which a decoder reads independently of each other, are coded on
//! threads of their own.

mod bool_writer;
mod dct;
mod header;
mod motion;
mod mv;
mod predict;
mod quantize;
mod tables;
mod tile;
mod tokens;
mod transform;

use crate::{Chroma, Error, MAX_THREADS, Picture, QUALITY_MAX, QUALITY_MIN, quality_to_q, tile_columns_log2};

/// The widest a tile column may be, in superblocks.
const MAX_TILE_WIDTH_B64: usize = 64;
/// The narrowest a tile column may be, in superblocks.
const MIN_TILE_WIDTH_B64: usize = 4;

/// A frame's measures in the units the bitstream counts in: mode info units
/// of 8×8 and superblocks of 64×64.
pub(super) struct Geometry {
    width: u16,
    height: u16,
    mi_cols: usize,
    mi_rows: usize,
    sb_cols: usize,
    sb_rows: usize,
}

impl Geometry {
    fn new(width: u16, height: u16) -> Self {
        let mi_cols = usize::from(width).div_ceil(8);
        let mi_rows = usize::from(height).div_ceil(8);
        Self { width, height, mi_cols, mi_rows, sb_cols: mi_cols.div_ceil(8), sb_rows: mi_rows.div_ceil(8) }
    }

    /// The fewest tile columns the width allows, as log2.
    fn min_log2_tile_cols(&self) -> u32 {
        (0..).find(|&n| MAX_TILE_WIDTH_B64 << n >= self.sb_cols).expect("a finite width")
    }

    /// The most tile columns the width allows, as log2.
    fn max_log2_tile_cols(&self) -> u32 {
        (1..).find(|&n| self.sb_cols >> n < MIN_TILE_WIDTH_B64).expect("a finite width") - 1
    }

    /// The 8×8 columns of tile `i` of `1 << log2`.
    fn tile_cols(&self, i: usize, log2: u32) -> std::ops::Range<usize> {
        let start = |i: usize| (((i * self.sb_cols) >> log2) * 8).min(self.mi_cols);
        start(i)..start(i + 1)
    }
}

/// The frame as a decoder reconstructs it, a tile column at a time so the
/// columns can be coded at once.
pub(super) struct Reconstruction {
    columns: Vec<Column>,
}

/// One tile column of a `Reconstruction`, from pixel column `x` of the
/// frame: three planes at its width and the frame's height rounded up to
/// whole superblocks, so a transform block that reaches past the frame's edge
/// has somewhere to go.
pub(super) struct Column {
    x: usize,
    planes: [Vec<u8>; 3],
    stride: usize,
}

impl Column {
    /// `len` pixels of row `y` of `plane` from pixel column `x` of the frame.
    fn row(&self, plane: usize, y: usize, x: usize, len: usize) -> &[u8] {
        &self.planes[plane][y * self.stride + x - self.x..][..len]
    }
}

impl Reconstruction {
    /// A blank frame in the `1 << log2` tile columns of `geometry`.
    fn new(geometry: &Geometry, log2: u32) -> Self {
        let rows = geometry.sb_rows * 64;
        let columns = (0..1 << log2)
            .map(|i| {
                let cols = geometry.tile_cols(i, log2);
                let stride = (cols.len() * 8).next_multiple_of(64);
                Column { x: cols.start * 8, planes: std::array::from_fn(|_| vec![0; stride * rows]), stride }
            })
            .collect();
        Self { columns }
    }

    /// The column that pixel column `x` of the frame is in.
    fn column(&self, x: usize) -> &Column {
        &self.columns[self.columns.partition_point(|c| c.x <= x) - 1]
    }

    /// The column that holds pixel columns `x..x + len` of the frame, if one
    /// holds them all.
    pub(super) fn holding(&self, x: usize, len: usize) -> Option<&Column> {
        let column = self.column(x);
        (x + len <= column.x + column.stride).then_some(column)
    }

    /// Fill `out` from row `y` of `plane` at pixel column `x` on, as a
    /// decoder reads a reference frame `width` wide: past either end of the
    /// picture, the pixel at that end.
    pub(super) fn read_row(&self, plane: usize, y: usize, x: isize, width: usize, out: &mut [u8]) {
        let len = out.len() as isize;
        // Before `first`, left of the picture; from `end`, right of it.
        let first = (-x).clamp(0, len) as usize;
        let end = (width as isize - x).clamp(0, len) as usize;
        if first > 0 {
            out[..first].fill(self.column(0).row(plane, y, 0, 1)[0]);
        }
        let mut i = first;
        while i < end {
            let px = (x + i as isize) as usize;
            let column = self.column(px);
            let n = (column.x + column.stride - px).min(end - i);
            out[i..i + n].copy_from_slice(column.row(plane, y, px, n));
            i += n;
        }
        if end < out.len() {
            out[end..].fill(self.column(width - 1).row(plane, y, width - 1, 1)[0]);
        }
    }
}

/// One 4:4:4 VP9 stream at one picture size.
pub(crate) struct Encoder {
    geometry: Geometry,
    tile_cols_log2: u32,
    /// The most threads the tile columns are coded on.
    threads: usize,
    quality: u8,
    /// Per tile column, what a decoder keeps while it reads it.
    contexts: Vec<tile::Contexts>,
    /// The frame being coded.
    recon: Reconstruction,
    /// The frame coded last, as a decoder has it: what an inter frame is
    /// coded against.
    reference: Reconstruction,
    /// The source of the frame coded last, tight at the picture's width.
    previous: [Vec<u8>; 3],
    /// Per 8×8 of the frame, whether the source is `previous` there.
    unchanged: Vec<bool>,
    search: motion::Search,
    /// Whether a frame has been coded, so the next can be an inter frame.
    started: bool,
    /// Whether the frame coded last was an inter frame, whose vectors a
    /// decoder predicts the next frame's from.
    last_inter: bool,
    /// Whether every block is to be coded again, unchanged or not: after the
    /// quantizer moves.
    requantize: bool,
}

impl Encoder {
    /// An encoder for a `width`×`height` picture at `quality` (1–100,
    /// clamped), its tile columns chosen as the libvpx encoder's are for
    /// `threads` threads.
    pub(crate) fn new(width: u16, height: u16, quality: u8, threads: usize) -> Result<Self, Error> {
        if width == 0 || height == 0 {
            return Err(Error::Empty(width, height));
        }
        if !(1..=MAX_THREADS).contains(&threads) {
            return Err(Error::Threads(threads));
        }
        let geometry = Geometry::new(width, height);
        let tile_cols_log2 = tile_columns_log2(width, threads).clamp(geometry.min_log2_tile_cols(), geometry.max_log2_tile_cols());
        let pixels = usize::from(width) * usize::from(height);
        Ok(Self {
            tile_cols_log2,
            threads,
            quality: quality.clamp(QUALITY_MIN, QUALITY_MAX),
            contexts: (0..1 << tile_cols_log2).map(|i| tile::Contexts::new(&geometry, geometry.tile_cols(i, tile_cols_log2))).collect(),
            recon: Reconstruction::new(&geometry, tile_cols_log2),
            reference: Reconstruction::new(&geometry, tile_cols_log2),
            previous: std::array::from_fn(|_| vec![0; pixels]),
            unchanged: vec![false; geometry.mi_rows * geometry.mi_cols],
            search: motion::Search::new(&geometry),
            started: false,
            last_inter: false,
            requantize: false,
            geometry,
        })
    }

    /// The dial this encoder is coding at.
    pub(crate) fn quality(&self) -> u8 {
        self.quality
    }

    /// Move the dial (1–100, clamped) without a keyframe: the next frame
    /// codes every block again at the new quantizer, changed or not.
    pub(crate) fn set_quality(&mut self, quality: u8) {
        let quality = quality.clamp(QUALITY_MIN, QUALITY_MAX);
        if quality_to_q(quality) != quality_to_q(self.quality) {
            self.requantize = true;
        }
        self.quality = quality;
    }

    /// Code `picture` and append the frame to `out`: a keyframe when
    /// `keyframe` asks for one or nothing has been coded yet, otherwise an
    /// inter frame against the frame before. Returns whether it is a
    /// keyframe.
    pub(crate) fn encode(&mut self, picture: &Picture, keyframe: bool, out: &mut Vec<u8>) -> Result<bool, Error> {
        let g = &self.geometry;
        if picture.size() != (g.width, g.height) || picture.chroma() != Chroma::Full {
            let (pw, ph) = picture.size();
            return Err(Error::Mismatch(g.width, g.height, Chroma::Full.name(), pw, ph, picture.chroma().name()));
        }
        let keyframe = keyframe || !self.started;
        let everything = keyframe || self.requantize;
        self.compare(picture, everything);
        let vectors = self.search.update(&self.previous[0], (!everything).then_some(&self.unchanged[..]), !keyframe);
        for contexts in &mut self.contexts {
            contexts.start(self.last_inter);
        }
        let g = &self.geometry;
        let q_index = tables::QUANTIZER_TO_QINDEX[quality_to_q(self.quality) as usize];
        let quantizer = quantize::Quantizer::new(q_index);
        let source = tile::Source { planes: picture.planes(), stride: picture.strides()[0], width: usize::from(g.width), height: usize::from(g.height) };
        let previous = (!keyframe).then(|| tile::Previous { reference: &self.reference, unchanged: &self.unchanged, vectors: &vectors });
        let code = |(contexts, column): (&mut tile::Contexts, &mut Column)| tile::Tile::new(g, contexts, column, &source, previous, &quantizer).code();
        let mut jobs = self.contexts.iter_mut().zip(&mut self.recon.columns);
        // As many tile columns to a thread as it takes to leave none over,
        // the first thread this one.
        let per = jobs.len().div_ceil(self.threads);
        let tiles: Vec<Vec<u8>> = std::thread::scope(|scope| {
            let first: Vec<_> = jobs.by_ref().take(per).collect();
            let code = &code;
            let others: Vec<_> = (0..)
                .map_while(|_| {
                    let chunk: Vec<_> = jobs.by_ref().take(per).collect();
                    (!chunk.is_empty()).then(|| scope.spawn(move || chunk.into_iter().map(code).collect::<Vec<_>>()))
                })
                .collect();
            let mut tiles: Vec<_> = first.into_iter().map(code).collect();
            for other in others {
                tiles.extend(other.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)));
            }
            tiles
        });
        let compressed = header::compressed(keyframe);
        let uncompressed = header::Uncompressed {
            keyframe,
            width: g.width,
            height: g.height,
            q_index,
            tile_cols_log2: self.tile_cols_log2,
            min_log2_tile_cols: g.min_log2_tile_cols(),
            max_log2_tile_cols: g.max_log2_tile_cols(),
        };
        out.extend(uncompressed.write(compressed.len()));
        out.extend(compressed);
        // Every tile but the last is preceded by its size.
        let last = tiles.len() - 1;
        for (i, tile) in tiles.iter().enumerate() {
            if i < last {
                out.extend((tile.len() as u32).to_be_bytes());
            }
            out.extend(tile);
        }
        std::mem::swap(&mut self.recon, &mut self.reference);
        self.started = true;
        self.last_inter = !keyframe;
        self.requantize = false;
        Ok(keyframe)
    }

    /// Mark each 8×8 of the frame unchanged where `picture` is the previous
    /// source over the part of it the picture holds, and make the previous
    /// source `picture`. With `everything`, every 8×8 is changed.
    fn compare(&mut self, picture: &Picture, everything: bool) {
        let g = &self.geometry;
        let (width, height) = (usize::from(g.width), usize::from(g.height));
        let (planes, stride) = (picture.planes(), picture.strides()[0]);
        if everything {
            for (previous, plane) in self.previous.iter_mut().zip(planes) {
                for y in 0..height {
                    previous[y * width..][..width].copy_from_slice(&plane[y * stride..][..width]);
                }
            }
            self.unchanged.fill(false);
            return;
        }
        for mi_row in 0..g.mi_rows {
            let rows = mi_row * 8..(mi_row * 8 + 8).min(height);
            for mi_col in 0..g.mi_cols {
                let x = mi_col * 8;
                let wide = 8.min(width - x);
                let same = self.previous.iter().zip(planes).all(|(previous, plane)| {
                    rows.clone().all(|y| previous[y * width + x..][..wide] == plane[y * stride + x..][..wide])
                });
                self.unchanged[mi_row * g.mi_cols + mi_col] = same;
                if !same {
                    for (previous, plane) in self.previous.iter_mut().zip(planes) {
                        for y in rows.clone() {
                            previous[y * width + x..][..wide].copy_from_slice(&plane[y * stride + x..][..wide]);
                        }
                    }
                }
            }
        }
    }

    /// The last frame coded, as a decoder has it: the planes, tight at the
    /// picture's width.
    fn reconstruction(&self) -> [Vec<u8>; 3] {
        let (width, height) = (usize::from(self.geometry.width), usize::from(self.geometry.height));
        std::array::from_fn(|plane| {
            let mut out = vec![0; width * height];
            for (y, row) in out.chunks_exact_mut(width).enumerate() {
                self.reference.read_row(plane, y, 0, width, row);
            }
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Decoder;

    /// `w`×`h` RGB noise from `seed`.
    fn noise_rgb(w: u16, h: u16, seed: u32) -> Vec<u8> {
        let mut state = seed;
        (0..usize::from(w) * usize::from(h) * 3)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect()
    }

    /// A desktop-like `w`×`h` in RGB: flat panels, hard edges, a gradient,
    /// text-like strokes of every colour and a patch of noise.
    fn desktop_rgb(w: u16, h: u16) -> Vec<u8> {
        let mut state = 0x2545_f491_u32;
        let mut rgb = Vec::with_capacity(usize::from(w) * usize::from(h) * 3);
        for y in 0..usize::from(h) {
            for x in 0..usize::from(w) {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let pixel = if x % 97 < 3 || y % 61 == 0 {
                    [20, 20, 30]
                } else if (x / 5 + y / 9) % 7 == 0 && x % 2 == 0 {
                    [(x * 7) as u8, (y * 3) as u8, 200]
                } else if x * 3 < usize::from(w) {
                    [(x % 256) as u8, (y % 256) as u8, 128]
                } else if y * 2 > usize::from(h) && x * 3 > usize::from(w) * 2 {
                    [state as u8, (state >> 8) as u8, (state >> 16) as u8]
                } else {
                    [235, 235, 240]
                };
                rgb.extend(pixel);
            }
        }
        rgb
    }

    /// `rgb` with the rectangle at `(x, y)` of `rw`×`rh`, as far as the
    /// picture holds it, painted over with noise from `seed`.
    fn edit(mut rgb: Vec<u8>, w: u16, (x, y, rw, rh): (usize, usize, usize, usize), seed: u32) -> Vec<u8> {
        let w = usize::from(w);
        let h = rgb.len() / 3 / w;
        let paint = noise_rgb(rw as u16, rh as u16, seed);
        for r in 0..rh.min(h.saturating_sub(y)) {
            for c in 0..rw.min(w.saturating_sub(x)) {
                rgb[((y + r) * w + x + c) * 3..][..3].copy_from_slice(&paint[(r * rw + c) * 3..][..3]);
            }
        }
        rgb
    }

    fn picture(w: u16, h: u16, rgb: &[u8]) -> Picture {
        let mut picture = Picture::new(w, h, Chroma::Full).expect("a picture");
        picture.read_rgb(rgb).expect("its own picture");
        picture
    }

    /// A picture of noise, so nothing in it is what the frame decodes to.
    fn noise(w: u16, h: u16) -> Picture {
        picture(w, h, &noise_rgb(w, h, 0x9e37_79b9))
    }

    fn desktop(w: u16, h: u16) -> Picture {
        picture(w, h, &desktop_rgb(w, h))
    }

    /// The mean squared error between the planes of `picture` and those of a
    /// decode, over the picture.
    fn mse(picture: &Picture, decoded: &crate::Decoded<'_>) -> f64 {
        let (w, h) = picture.size();
        let (w, h) = (usize::from(w), usize::from(h));
        let mut sum = 0.0;
        for ((source, plane), stride) in picture.planes().into_iter().zip(decoded.planes()).zip(decoded.strides()) {
            for y in 0..h {
                for x in 0..w {
                    sum += (f64::from(source[y * w + x]) - f64::from(plane[y * stride + x])).powi(2);
                }
            }
        }
        sum / (3 * w * h) as f64
    }

    /// An encoder and a libvpx decoder following its stream.
    struct Stream {
        encoder: Encoder,
        decoder: Decoder,
        frames: usize,
    }

    impl Stream {
        fn new(w: u16, h: u16, quality: u8, threads: usize) -> Self {
            Self { encoder: Encoder::new(w, h, quality, threads).expect("an encoder"), decoder: Decoder::new(1).expect("a decoder"), frames: 0 }
        }

        /// Code `picture`, decode the frame, hold the decode to the
        /// encoder's own reconstruction pixel for pixel, and give back the
        /// frame's size, whether it is a keyframe, and the decode's mean
        /// squared error.
        fn frame(&mut self, picture: &Picture, keyframe: bool) -> (usize, bool, f64) {
            let (w, h) = picture.size();
            let quality = self.encoder.quality();
            let n = self.frames;
            self.frames += 1;
            let mut frame = Vec::new();
            let keyframe = self.encoder.encode(picture, keyframe, &mut frame).expect("a frame");
            assert_eq!(crate::frame_header(&frame), Some(crate::FrameHeader { profile: 1, keyframe }));
            let decoded = self.decoder.decode(&frame).unwrap_or_else(|e| panic!("{w}x{h} at quality {quality}, frame {n}: {e}"));
            assert_eq!(decoded.size(), (u32::from(w), u32::from(h)));
            assert_eq!(decoded.chroma(), Chroma::Full);
            assert!(decoded.declares_bt601_studio_swing());
            let recon = self.encoder.reconstruction();
            let stride = usize::from(w);
            for (p, ((plane, decoded_stride), recon)) in decoded.planes().into_iter().zip(decoded.strides()).zip(recon).enumerate() {
                for y in 0..usize::from(h) {
                    let got = &plane[y * decoded_stride..][..usize::from(w)];
                    let want = &recon[y * stride..][..usize::from(w)];
                    if let Some(x) = got.iter().zip(want).position(|(a, b)| a != b) {
                        panic!("{w}x{h} at quality {quality}, frame {n}, plane {p}: the decode is {} at ({x}, {y}), the encoder's {}", got[x], want[x]);
                    }
                }
            }
            (frame.len(), keyframe, mse(picture, &decoded))
        }
    }

    /// Code `picture` as a keyframe at `quality` on `threads` and give back
    /// the frame's size and the decode's mean squared error.
    fn round_trip(picture: &Picture, quality: u8, threads: usize) -> (usize, f64) {
        let (w, h) = picture.size();
        let (bytes, keyframe, mse) = Stream::new(w, h, quality, threads).frame(picture, true);
        assert!(keyframe);
        (bytes, mse)
    }

    #[test]
    fn a_keyframe_decodes_to_the_encoders_own_reconstruction() {
        for (w, h) in [(1, 1), (7, 5), (8, 8), (64, 64), (65, 65), (130, 67), (1440, 900)] {
            for quality in [1, 60, 100] {
                round_trip(&desktop(w, h), quality, 1);
                round_trip(&noise(w, h), quality, 1);
            }
        }
    }

    #[test]
    fn a_finer_quality_costs_bytes_and_buys_fidelity() {
        let picture = desktop(640, 360);
        let (coarse_bytes, coarse_mse) = round_trip(&picture, 1, 1);
        let (fine_bytes, fine_mse) = round_trip(&picture, 100, 1);
        assert!(fine_bytes > coarse_bytes, "{fine_bytes} bytes at 100, {coarse_bytes} at 1");
        assert!(fine_mse < coarse_mse / 4.0, "squared error {fine_mse} at 100, {coarse_mse} at 1");
        // Quality 100 is a quantizer of 8: close to the picture, not equal.
        assert!(fine_mse < 4.0, "squared error {fine_mse} at 100");
    }

    #[test]
    fn a_flat_picture_is_coded_in_whole_superblocks() {
        let (w, h) = (640, 384);
        let mut flat = Picture::new(w, h, Chroma::Full).expect("a picture");
        flat.read_rgb(&[40, 90, 200].repeat(usize::from(w) * usize::from(h))).expect("its own picture");
        let (bytes, mse) = round_trip(&flat, 100, 1);
        // Sixty superblocks of a partition symbol, a skip flag and two modes
        // each, and the first one's residual; split to 8×8 it is thousands
        // of blocks.
        assert!(bytes < 120, "{bytes} bytes");
        assert!(mse < 1.0, "squared error {mse}");
    }

    #[test]
    fn tile_columns_follow_the_width_and_the_threads() {
        // 3456 wide on 4 threads is two columns of 27 superblocks; on 1, one.
        let two = Encoder::new(3456, 8, 90, 4).expect("an encoder");
        let one = Encoder::new(3456, 8, 90, 1).expect("an encoder");
        assert_eq!((two.tile_cols_log2, one.tile_cols_log2), (1, 0));
        assert_eq!(two.geometry.tile_cols(1, 1), 216..432);
        round_trip(&desktop(3456, 8), 90, 4);
        // Past 4096 the format itself asks for a second column.
        let wide = Encoder::new(4160, 8, 90, 1).expect("an encoder");
        assert_eq!(wide.tile_cols_log2, 1);
        round_trip(&desktop(4160, 8), 90, 1);
        // Too narrow for two columns, however many threads.
        assert_eq!(Encoder::new(255, 8, 90, 64).expect("an encoder").tile_cols_log2, 0);
    }

    #[test]
    fn threads_change_nothing_but_the_time() {
        // Past 4096 there are two tile columns on one thread as on two.
        let (w, h) = (4160, 72);
        let rgb = desktop_rgb(w, h);
        let frames = [rgb.clone(), scroll(&rgb, w, 5), drag(&rgb, w, (1900, 8, 400, 40), (300, 3)), edit(rgb, w, (2000, 30, 300, 30), 7)];
        let [one, two] = [1, 2].map(|threads| {
            let mut encoder = Encoder::new(w, h, 90, threads).expect("an encoder");
            assert_eq!(encoder.tile_cols_log2, 1);
            let mut out = Vec::new();
            for rgb in &frames {
                encoder.encode(&picture(w, h, rgb), false, &mut out).expect("a frame");
            }
            out
        });
        assert!(one == two, "{} bytes on one thread, {} on two", one.len(), two.len());
    }

    /// `rgb`, `w` wide, with its rectangle at `(x, y)` of `rw`×`rh` moved
    /// by `(dx, dy)`, as far as the picture holds either, what it uncovers
    /// left as it was.
    fn drag(rgb: &[u8], w: u16, (x, y, rw, rh): (usize, usize, usize, usize), (dx, dy): (isize, isize)) -> Vec<u8> {
        let w = usize::from(w);
        let h = rgb.len() / 3 / w;
        let mut out = rgb.to_vec();
        for r in y..(y + rh).min(h) {
            for c in x..(x + rw).min(w) {
                let (tx, ty) = (c as isize + dx, r as isize + dy);
                if (0..w as isize).contains(&tx) && (0..h as isize).contains(&ty) {
                    out[(ty as usize * w + tx as usize) * 3..][..3].copy_from_slice(&rgb[(r * w + c) * 3..][..3]);
                }
            }
        }
        out
    }

    /// `rgb`, `w` wide, each row turned left by `cols`, wrapping.
    fn pan(rgb: &[u8], w: u16, cols: usize) -> Vec<u8> {
        let mut out = rgb.to_vec();
        for row in out.chunks_exact_mut(usize::from(w) * 3) {
            row.rotate_left(cols % usize::from(w) * 3);
        }
        out
    }

    /// `rgb`, `w` wide, scrolled up by `rows` with a flat colour coming in
    /// below.
    fn scroll(rgb: &[u8], w: u16, rows: usize) -> Vec<u8> {
        let h = rgb.len() / 3 / usize::from(w);
        let mut out = rgb[rows * usize::from(w) * 3..].to_vec();
        out.extend([235, 235, 240].repeat(rows * usize::from(w)));
        assert_eq!(out.len(), usize::from(w) * h * 3);
        out
    }

    #[test]
    fn inter_frames_decode_to_the_encoders_own_reconstruction() {
        for (w, h) in [(1, 1), (7, 5), (8, 8), (65, 65), (130, 67), (200, 120), (40, 360), (640, 360)] {
            let original = desktop_rgb(w, h);
            let (wu, hu) = (usize::from(w), usize::from(h));
            let turned = |rows: usize, left: bool| {
                let mut rgb = original.clone();
                if left {
                    rgb.rotate_left(rows % hu * wu * 3);
                } else {
                    rgb.rotate_right(rows % hu * wu * 3);
                }
                rgb
            };
            // Motion every way, by little and by much, across the frame's
            // edges and back, each frame against the one before.
            let frames = [
                original.clone(),
                original.clone(),
                edit(original.clone(), w, (5, 3, 40, 21), 1),
                edit(original.clone(), w, (60, 30, 300, 200), 2),
                turned(3, true),
                turned(3, false),
                turned(70, false),
                scroll(&original, w, 13 % hu),
                pan(&original, w, 5),
                pan(&original, w, wu * 9 / 10),
                drag(&original, w, (10, 20, 100, 70), (-30, 17)),
                drag(&original, w, (wu / 2, hu / 2, 120, 90), (45, 60)),
                drag(&original, w, (0, 0, wu, hu), (-9, -70)),
                noise_rgb(w, h, 3),
                edit(noise_rgb(w, h, 3), w, (0, 0, 9, 9), 4),
                original,
            ];
            for quality in [1, 60, 100] {
                let mut stream = Stream::new(w, h, quality, 1);
                for (i, rgb) in frames.iter().enumerate() {
                    let (_, keyframe, _) = stream.frame(&picture(w, h, rgb), false);
                    assert_eq!(keyframe, i == 0, "frame {i}");
                }
                // A keyframe asked for mid-stream.
                assert!(stream.frame(&desktop(w, h), true).1);
                assert!(!stream.frame(&noise(w, h), false).1);
            }
        }
    }

    #[test]
    fn motion_decodes_across_tile_columns() {
        // Four tile columns: vectors are predicted only from inside a tile,
        // and one may point into another's part of the frame before.
        let (w, h) = (3840, 136);
        let original = desktop_rgb(w, h);
        let mut stream = Stream::new(w, h, 90, 4);
        assert_eq!(stream.encoder.tile_cols_log2, 2);
        let frames = [
            original.clone(),
            pan(&original, w, 77),
            scroll(&pan(&original, w, 70), w, 9),
            drag(&original, w, (700, 10, 400, 100), (500, 6)),
            drag(&original, w, (2000, 10, 600, 120), (-900, -6)),
            pan(&original, w, 3840 - 1000),
        ];
        for (i, rgb) in frames.iter().enumerate() {
            let (bytes, keyframe, _) = stream.frame(&picture(w, h, rgb), false);
            assert_eq!(keyframe, i == 0, "frame {i}, {bytes} bytes");
        }
    }

    #[test]
    fn an_unchanged_picture_costs_a_few_bytes() {
        let (w, h) = (640, 384);
        let mut stream = Stream::new(w, h, 100, 1);
        let (_, _, first) = stream.frame(&desktop(w, h), false);
        let (bytes, keyframe, again) = stream.frame(&desktop(w, h), false);
        assert!(!keyframe);
        // The headers, and sixty superblocks each one skipped block of the
        // previous frame.
        assert!(bytes < 40, "{bytes} bytes");
        assert_eq!(again, first);
    }

    #[test]
    fn an_edit_is_coded_and_the_rest_is_skipped() {
        let (w, h) = (640, 384);
        let mut stream = Stream::new(w, h, 100, 1);
        let rgb = desktop_rgb(w, h);
        let (key_bytes, _, _) = stream.frame(&picture(w, h, &rgb), false);
        let before = stream.encoder.reconstruction();
        // A pixel nudged: a change small enough that its superblock varies
        // little against the frame before.
        let mut nudged = rgb;
        for v in &mut nudged[(100 * usize::from(w) + 100) * 3..][..3] {
            *v += 6;
        }
        let (bytes, _, mse) = stream.frame(&picture(w, h, &nudged), false);
        assert!(bytes * 10 < key_bytes, "{bytes} bytes against a keyframe of {key_bytes}");
        assert!(mse < 4.0, "squared error {mse}");
        // Outside the 8×8 the edit is in, the frame is the one before, not
        // coded again.
        let after = stream.encoder.reconstruction();
        let stride = usize::from(w);
        for (p, (a, b)) in after.iter().zip(&before).enumerate() {
            for y in 0..usize::from(h) {
                for x in 0..usize::from(w) {
                    let touched = (96..104).contains(&x) && (96..104).contains(&y);
                    assert!(touched || a[y * stride + x] == b[y * stride + x], "plane {p} moved at ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn a_scroll_is_coded_as_motion() {
        let (w, h) = (640, 384);
        let mut stream = Stream::new(w, h, 100, 1);
        let rgb = desktop_rgb(w, h);
        let (key_bytes, _, _) = stream.frame(&picture(w, h, &rgb), false);
        let scrolled = scroll(&rgb, w, 37);
        let (bytes, keyframe, mse) = stream.frame(&picture(w, h, &scrolled), false);
        assert!(!keyframe);
        assert!(bytes * 10 < key_bytes, "{bytes} bytes against a keyframe of {key_bytes}");
        assert!(mse < 4.0, "squared error {mse}");
    }

    #[test]
    fn a_dragged_window_is_coded_as_motion() {
        let (w, h) = (640, 384);
        let mut stream = Stream::new(w, h, 100, 1);
        let rgb = desktop_rgb(w, h);
        let (key_bytes, _, _) = stream.frame(&picture(w, h, &rgb), false);
        let dragged = drag(&rgb, w, (40, 30, 300, 200), (53, 21));
        let (bytes, _, mse) = stream.frame(&picture(w, h, &dragged), false);
        assert!(bytes * 10 < key_bytes, "{bytes} bytes against a keyframe of {key_bytes}");
        assert!(mse < 4.0, "squared error {mse}");
    }

    #[test]
    fn a_quality_change_codes_every_block_again() {
        let (w, h) = (256, 256);
        let mut stream = Stream::new(w, h, 1, 1);
        let picture = noise(w, h);
        let (_, _, coarse) = stream.frame(&picture, false);
        // A dial that moves less than a quantizer step changes nothing.
        stream.encoder.set_quality(2);
        assert_eq!(stream.encoder.quality(), 2);
        let (bytes, _, same) = stream.frame(&picture, false);
        assert!(bytes < 30, "{bytes} bytes");
        assert_eq!(same, coarse);
        stream.encoder.set_quality(100);
        let (_, keyframe, fine) = stream.frame(&picture, false);
        assert!(!keyframe);
        assert!(fine < coarse / 4.0, "squared error {fine} at 100, {coarse} at 1");
    }

    #[test]
    fn keyframes_follow_each_other_through_one_decoder() {
        let (w, h) = (200, 120);
        let mut stream = Stream::new(w, h, 50, 1);
        for _ in 0..3 {
            assert!(stream.frame(&noise(w, h), true).1);
        }
    }

    /// The 4:4:4 captures `docs/rust-encoder-plan.md` measures on, each
    /// with the quality it was made at and the threads its program codes on.
    const CAPTURES: [(&str, u8, usize); 6] = [
        ("desktop-q100-444", 100, 4),
        ("proxmox-q100-444", 100, 4),
        ("proxmox-hidpi-q100-444", 100, 4),
        ("mac-hp-444", 90, 5),
        ("mac-recording-q100-444", 100, 4),
        ("mac-recording-q90-444", 90, 5),
    ];

    /// Hand `f` each frame of an IVF capture in turn, as the picture a user
    /// would hand an encoder, until it returns false.
    fn each_picture(ivf: &[u8], mut f: impl FnMut(usize, Picture) -> bool) {
        assert_eq!(&ivf[..4], b"DKIF", "not an IVF file");
        let mut decoder = Decoder::new(4).expect("a decoder");
        let (mut at, mut n) = (32, 0);
        while at + 12 <= ivf.len() {
            let len = u32::from_le_bytes(ivf[at..at + 4].try_into().expect("four bytes")) as usize;
            let decoded = decoder.decode(&ivf[at + 12..at + 12 + len]).expect("a capture libvpx decodes");
            let (w, h) = decoded.size();
            let mut bgrx = vec![0; w as usize * h as usize * 4];
            decoded.write_bgrx(&mut bgrx, w as usize * 4).expect("a buffer of the frame's size");
            let mut picture = Picture::new(w as u16, h as u16, Chroma::Full).expect("a picture");
            picture.read_bgrx(&bgrx, w as usize * 4).expect("a buffer of the picture's size");
            if !f(n, picture) {
                return;
            }
            at += 12 + len;
            n += 1;
        }
    }

    /// Every `step`th frame of an IVF capture, up to `count` of them.
    fn sample(ivf: &[u8], step: usize, count: usize) -> Vec<Picture> {
        let mut pictures = Vec::new();
        each_picture(ivf, |n, picture| {
            if n % step == 0 {
                pictures.push(picture);
            }
            pictures.len() < count
        });
        pictures
    }

    fn psnr(mse: f64) -> f64 {
        10.0 * (255.0 * 255.0 / mse).log10()
    }

    /// Keyframes of frames sampled from the captures in `tmp/`, coded by this
    /// encoder and by libvpx on one thread each: bytes, the least time over
    /// three runs, and the PSNR of libvpx's decode of each. A measurement,
    /// not a check; run it in release:
    ///
    /// ```text
    /// cargo test --release keyframes_against_libvpx -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads the captures in tmp/ and wants a release build"]
    fn keyframes_against_libvpx() {
        for (name, quality, threads) in CAPTURES {
            let Ok(ivf) = std::fs::read(format!("tmp/{name}.ivf")) else {
                println!("{name}: not in tmp/");
                continue;
            };
            let pictures = sample(&ivf, 60, 8);
            let (w, h) = pictures[0].size();
            let mut libvpx = crate::Encoder::new(w, h, Chroma::Full, quality, threads).expect("a libvpx encoder");
            let mut rust = Encoder::new(w, h, quality, threads).expect("an encoder");
            let mut totals = [(0usize, 0.0f64, 0.0f64); 2];
            for picture in &pictures {
                let mut frames = [Vec::new(), Vec::new()];
                for (c, frame) in frames.iter_mut().enumerate() {
                    let mut least = f64::INFINITY;
                    for _ in 0..3 {
                        frame.clear();
                        let started = std::time::Instant::now();
                        if c == 0 {
                            libvpx.encode(picture, true, frame).expect("an encode").expect("a frame");
                        } else {
                            rust.encode(picture, true, frame).expect("a keyframe");
                        }
                        least = least.min(started.elapsed().as_secs_f64() * 1000.0);
                    }
                    let mut decoder = Decoder::new(1).expect("a decoder");
                    let decoded = decoder.decode(frame).expect("a frame libvpx decodes");
                    totals[c].0 += frame.len();
                    totals[c].1 += least;
                    totals[c].2 += psnr(mse(picture, &decoded));
                }
            }
            let n = pictures.len() as f64;
            println!(
                "{name} {w}x{h} q{quality}, {} keyframes | libvpx {} B {:.1} ms {:.2} dB | rust {} B {:.1} ms {:.2} dB | bytes {:.3} time {:.3}",
                pictures.len(), totals[0].0, totals[0].1 / n, totals[0].2 / n, totals[1].0, totals[1].1 / n, totals[1].2 / n,
                totals[1].0 as f64 / totals[0].0 as f64, totals[1].1 / totals[0].1,
            );
        }
    }

    /// The first frames of each capture in `tmp/`, coded as a stream by this
    /// encoder and by libvpx, each on one thread, frame by frame, which of
    /// them goes first alternating: bytes, time and the PSNR of libvpx's
    /// decode of each. The per-frame time ratio's median is what the plan's
    /// gate reads. A measurement, not a check; run it in release:
    ///
    /// ```text
    /// cargo test --release streams_against_libvpx -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads the captures in tmp/ and wants a release build"]
    fn streams_against_libvpx() {
        const FRAMES: usize = 200;
        for (name, quality, threads) in CAPTURES {
            let Ok(ivf) = std::fs::read(format!("tmp/{name}.ivf")) else {
                println!("{name}: not in tmp/");
                continue;
            };
            let mut encoders: Option<(crate::Encoder, Encoder)> = None;
            let mut decoders = [Decoder::new(1).expect("a decoder"), Decoder::new(1).expect("a decoder")];
            let (mut bytes, mut ms, mut psnrs) = ([0usize; 2], [0.0f64; 2], [0.0f64; 2]);
            let (mut ratios, mut frames, mut size) = (Vec::new(), 0, (0, 0));
            each_picture(&ivf, |n, picture| {
                let picture = &picture;
                size = picture.size();
                let (libvpx, rust) = encoders.get_or_insert_with(|| {
                    let (w, h) = picture.size();
                    (crate::Encoder::new(w, h, Chroma::Full, quality, threads).expect("a libvpx encoder"), Encoder::new(w, h, quality, threads).expect("an encoder"))
                });
                let mut out = [Vec::new(), Vec::new()];
                let mut times = [0.0; 2];
                for c in if n % 2 == 0 { [0, 1] } else { [1, 0] } {
                    let started = std::time::Instant::now();
                    if c == 0 {
                        libvpx.encode(picture, false, &mut out[0]).expect("an encode").expect("a frame");
                    } else {
                        rust.encode(picture, false, &mut out[1]).expect("a frame");
                    }
                    times[c] = started.elapsed().as_secs_f64() * 1000.0;
                }
                for c in 0..2 {
                    let decoded = decoders[c].decode(&out[c]).expect("a frame libvpx decodes");
                    bytes[c] += out[c].len();
                    ms[c] += times[c];
                    psnrs[c] += psnr(mse(picture, &decoded).max(1e-3));
                }
                ratios.push(times[1] / times[0]);
                frames += 1;
                frames < FRAMES
            });
            ratios.sort_unstable_by(f64::total_cmp);
            let n = frames as f64;
            println!(
                "{name} {}x{} q{quality} on {threads}, {frames} frames | libvpx {} B {:.2} ms {:.2} dB | rust {} B {:.2} ms {:.2} dB | bytes {:.3} time median {:.3} total {:.3}",
                size.0, size.1, bytes[0], ms[0] / n, psnrs[0] / n, bytes[1], ms[1] / n, psnrs[1] / n,
                bytes[1] as f64 / bytes[0] as f64, ratios[ratios.len() / 2], ms[1] / ms[0],
            );
        }
    }

    #[test]
    fn a_picture_that_is_not_the_encoders_is_refused() {
        let mut encoder = Encoder::new(64, 64, 90, 1).expect("an encoder");
        let mut out = Vec::new();
        let subsampled = Picture::new(64, 64, Chroma::Subsampled).expect("a picture");
        assert!(matches!(encoder.encode(&subsampled, true, &mut out), Err(Error::Mismatch(..))));
        assert!(matches!(encoder.encode(&noise(64, 32), false, &mut out), Err(Error::Mismatch(..))));
        assert!(out.is_empty());
        assert!(matches!(Encoder::new(0, 64, 90, 1), Err(Error::Empty(..))));
        assert!(matches!(Encoder::new(64, 64, 90, 0), Err(Error::Threads(0))));
    }
}
