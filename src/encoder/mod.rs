//! The VP9 encoder in Rust that `docs/rust-encoder-plan.md` sets out: 8-bit
//! 4:4:4, profile 1, the subset of VP9 the plan's table names and no more.
//! Reachable only from tests until it passes the plan's gate.
//!
//! So far it writes keyframes and nothing else: each superblock split as far
//! as its source varies, every block DC predicted at the largest transform
//! that fits it, its residual coded at the dial's quantizer, and the frame
//! reconstructed exactly as libvpx's decoder reconstructs it.

mod bool_writer;
mod dct;
mod header;
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

/// The frame as a decoder reconstructs it, three planes at the frame's size
/// rounded up to whole superblocks, so a transform block that reaches past
/// the frame's edge has somewhere to go.
pub(super) struct Reconstruction {
    planes: [Vec<u8>; 3],
    stride: usize,
}

/// One 4:4:4 VP9 stream at one picture size.
pub(crate) struct Encoder {
    geometry: Geometry,
    tile_cols_log2: u32,
    quality: u8,
    contexts: tile::FrameContexts,
    recon: Reconstruction,
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
        let stride = geometry.sb_cols * 64;
        let recon = Reconstruction { planes: std::array::from_fn(|_| vec![0; stride * geometry.sb_rows * 64]), stride };
        let contexts = tile::FrameContexts::new(&geometry);
        Ok(Self { geometry, tile_cols_log2, quality: quality.clamp(QUALITY_MIN, QUALITY_MAX), contexts, recon })
    }

    /// Code `picture` as a keyframe and append it to `out`.
    pub(crate) fn keyframe(&mut self, picture: &Picture, out: &mut Vec<u8>) -> Result<(), Error> {
        let g = &self.geometry;
        if picture.size() != (g.width, g.height) || picture.chroma() != Chroma::Full {
            let (pw, ph) = picture.size();
            return Err(Error::Mismatch(g.width, g.height, Chroma::Full.name(), pw, ph, picture.chroma().name()));
        }
        self.contexts.clear();
        let q_index = tables::QUANTIZER_TO_QINDEX[quality_to_q(self.quality) as usize];
        let quantizer = quantize::Quantizer::new(q_index);
        let source = tile::Source { planes: picture.planes(), stride: picture.strides()[0], width: usize::from(g.width), height: usize::from(g.height) };
        let tiles: Vec<Vec<u8>> = (0..1 << self.tile_cols_log2)
            .map(|i| tile::Tile::new(g, g.tile_cols(i, self.tile_cols_log2), &mut self.contexts, &mut self.recon, &source, &quantizer).code())
            .collect();
        let compressed = header::keyframe_compressed_header();
        let uncompressed = header::Keyframe {
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
        Ok(())
    }

    /// The reconstruction's planes and their stride.
    fn reconstruction(&self) -> (&[Vec<u8>; 3], usize) {
        (&self.recon.planes, self.recon.stride)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Decoder;

    /// A picture of noise, so nothing in it is what the frame decodes to.
    fn noise(w: u16, h: u16) -> Picture {
        let mut state = 0x9e37_79b9_u32;
        let rgb: Vec<u8> = (0..usize::from(w) * usize::from(h) * 3)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        let mut picture = Picture::new(w, h, Chroma::Full).expect("a picture");
        picture.read_rgb(&rgb).expect("its own picture");
        picture
    }

    /// A desktop-like picture: flat panels, hard edges, a gradient, text-like
    /// strokes of every colour and a patch of noise.
    fn desktop(w: u16, h: u16) -> Picture {
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
        let mut picture = Picture::new(w, h, Chroma::Full).expect("a picture");
        picture.read_rgb(&rgb).expect("its own picture");
        picture
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

    /// Encode `picture` as a keyframe at `quality` on `threads`, decode it
    /// with libvpx, hold the decode to the encoder's own reconstruction
    /// pixel for pixel, and give back the frame's size and the decode's
    /// mean squared error.
    fn round_trip(picture: &Picture, quality: u8, threads: usize) -> (usize, f64) {
        let (w, h) = picture.size();
        let mut encoder = Encoder::new(w, h, quality, threads).expect("an encoder");
        let mut frame = Vec::new();
        encoder.keyframe(picture, &mut frame).expect("a keyframe");
        assert_eq!(crate::frame_header(&frame), Some(crate::FrameHeader { profile: 1, keyframe: true }));
        let mut decoder = Decoder::new(1).expect("a decoder");
        let decoded = decoder.decode(&frame).unwrap_or_else(|e| panic!("{w}x{h} on {threads} threads: {e}"));
        assert_eq!(decoded.size(), (u32::from(w), u32::from(h)));
        assert_eq!(decoded.chroma(), Chroma::Full);
        assert!(decoded.declares_bt601_studio_swing());
        let (recon, stride) = encoder.reconstruction();
        for (p, ((plane, decoded_stride), recon)) in decoded.planes().into_iter().zip(decoded.strides()).zip(recon).enumerate() {
            for y in 0..usize::from(h) {
                let got = &plane[y * decoded_stride..][..usize::from(w)];
                let want = &recon[y * stride..][..usize::from(w)];
                if let Some(x) = got.iter().zip(want).position(|(a, b)| a != b) {
                    panic!("{w}x{h} at quality {quality}, plane {p}: the decode is {} at ({x}, {y}), the encoder's {}", got[x], want[x]);
                }
            }
        }
        (frame.len(), mse(picture, &decoded))
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
    fn keyframes_follow_each_other_through_one_decoder() {
        let (w, h) = (200, 120);
        let mut encoder = Encoder::new(w, h, 50, 1).expect("an encoder");
        let mut decoder = Decoder::new(1).expect("a decoder");
        for _ in 0..3 {
            let mut frame = Vec::new();
            encoder.keyframe(&noise(w, h), &mut frame).expect("a keyframe");
            decoder.decode(&frame).expect("a decode");
        }
    }

    /// The 4:4:4 captures `docs/rust-encoder-plan.md` measures on, each
    /// with the quality it was made at.
    const CAPTURES: [(&str, u8); 6] = [
        ("desktop-q100-444", 100),
        ("proxmox-q100-444", 100),
        ("proxmox-hidpi-q100-444", 100),
        ("mac-hp-444", 90),
        ("mac-recording-q100-444", 100),
        ("mac-recording-q90-444", 90),
    ];

    /// Every `step`th frame of an IVF capture, up to `count` of them, as the
    /// pictures a user would hand an encoder.
    fn sample(ivf: &[u8], step: usize, count: usize) -> Vec<Picture> {
        assert_eq!(&ivf[..4], b"DKIF", "not an IVF file");
        let mut decoder = Decoder::new(4).expect("a decoder");
        let mut pictures = Vec::new();
        let mut at = 32;
        let mut n = 0;
        while at + 12 <= ivf.len() && pictures.len() < count {
            let len = u32::from_le_bytes(ivf[at..at + 4].try_into().expect("four bytes")) as usize;
            let decoded = decoder.decode(&ivf[at + 12..at + 12 + len]).expect("a capture libvpx decodes");
            if n % step == 0 {
                let (w, h) = decoded.size();
                let mut bgrx = vec![0; w as usize * h as usize * 4];
                decoded.write_bgrx(&mut bgrx, w as usize * 4).expect("a buffer of the frame's size");
                let mut picture = Picture::new(w as u16, h as u16, Chroma::Full).expect("a picture");
                picture.read_bgrx(&bgrx, w as usize * 4).expect("a buffer of the picture's size");
                pictures.push(picture);
            }
            at += 12 + len;
            n += 1;
        }
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
        for (name, quality) in CAPTURES {
            let Ok(ivf) = std::fs::read(format!("tmp/{name}.ivf")) else {
                println!("{name}: not in tmp/");
                continue;
            };
            let pictures = sample(&ivf, 60, 8);
            let (w, h) = pictures[0].size();
            let mut libvpx = crate::Encoder::new(w, h, Chroma::Full, quality, 1).expect("a libvpx encoder");
            let mut rust = Encoder::new(w, h, quality, 1).expect("an encoder");
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
                            rust.keyframe(picture, frame).expect("a keyframe");
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

    #[test]
    fn a_picture_that_is_not_the_encoders_is_refused() {
        let mut encoder = Encoder::new(64, 64, 90, 1).expect("an encoder");
        let mut out = Vec::new();
        let subsampled = Picture::new(64, 64, Chroma::Subsampled).expect("a picture");
        assert!(matches!(encoder.keyframe(&subsampled, &mut out), Err(Error::Mismatch(..))));
        assert!(matches!(encoder.keyframe(&noise(64, 32), &mut out), Err(Error::Mismatch(..))));
        assert!(out.is_empty());
        assert!(matches!(Encoder::new(0, 64, 90, 1), Err(Error::Empty(..))));
        assert!(matches!(Encoder::new(64, 64, 90, 0), Err(Error::Threads(0))));
    }
}
