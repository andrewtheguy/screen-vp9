//! The VP9 encoder in Rust that `docs/rust-encoder-plan.md` sets out: 8-bit
//! 4:4:4, profile 1, the subset of VP9 the plan's table names and no more.
//! Reachable only from tests until it passes the plan's gate.
//!
//! So far it codes keyframes and inter frames against the frame before, each
//! superblock split as far as its source varies, every block either DC
//! predicted or the previous frame at a whole-pixel vector, at a transform
//! size of its own, its residual coded at the dial's quantizer, and the frame
//! reconstructed exactly as libvpx's decoder reconstructs it, its loop filter
//! run over it before it is kept. Where the source has not changed since the
//! frame before, a block is that frame's, skipped; where it moved, a hash
//! search finds where it came from. A frame's bools are recorded as its
//! blocks are decided and coded once its header has updated the
//! probabilities to what they count, which a decoder keeps for the next
//! frame. Tile columns, which a decoder reads independently of each other,
//! are decided and coded on threads of their own.

mod bool_writer;
mod convolve;
mod dct;
mod header;
mod lanes;
mod loopfilter;
mod motion;
mod mv;
mod pixels;
mod predict;
mod probs;
mod quantize;
mod tables;
mod tape;
mod tile;
mod tokens;
mod transform;

use crate::{Chroma, Error, MAX_THREADS, Picture, QUALITY_MAX, QUALITY_MIN, loop_filtered, quality_to_q, tile_columns_log2};

/// The share of what the blocks it refines would cost that the search
/// between pixels has to save, as `tile::laplacian` models it, for it to
/// refine every block of the next frame; where it saves less, it refines one
/// block in `SEARCH_PROBE`, to see whether it pays again. On the plan's
/// captures it saved a median of 3% a frame on proxmox and mac-hp, whose
/// bytes it barely moved for half again the time, and 11–14% on hidpi and
/// the mac recordings, whose bytes it cut by 3–10%. Probing one block in 8
/// cost proxmox and mac-hp 7% of their time, one in 32 cost 2–3%.
const SEARCH_PAYS: f64 = 1.0 / 16.0;
const SEARCH_PROBE: usize = 32;

/// The widest a tile column may be, in superblocks.
const MAX_TILE_WIDTH_B64: usize = 64;
/// The narrowest a tile column may be, in superblocks.
const MIN_TILE_WIDTH_B64: usize = 4;

/// Run `work` on each of `jobs` on up to `threads` threads, this one among
/// them, as many jobs to a thread as it takes to leave none over, and give
/// back what each gives, in order.
fn on_threads<J: Send, R: Send>(threads: usize, mut jobs: impl ExactSizeIterator<Item = J>, work: impl Fn(J) -> R + Sync) -> Vec<R> {
    let per = jobs.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let first: Vec<J> = jobs.by_ref().take(per).collect();
        let work = &work;
        let others: Vec<_> = (0..)
            .map_while(|_| {
                let chunk: Vec<J> = jobs.by_ref().take(per).collect();
                (!chunk.is_empty()).then(|| scope.spawn(move || chunk.into_iter().map(work).collect::<Vec<R>>()))
            })
            .collect();
        let mut done: Vec<R> = first.into_iter().map(work).collect();
        for other in others {
            done.extend(other.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)));
        }
        done
    })
}

/// Run `work` on each of `jobs` on as many threads as there are `scratch`es,
/// this one among them, each with a scratch of its own and taking the next
/// job as it finishes one, so that a thread whose jobs have little to do
/// takes more of them; and give back what each gives, in order.
fn on_workers<J: Send, S: Send, R: Send>(jobs: impl Iterator<Item = J> + Send, scratch: &mut [S], work: impl Fn(J, &mut S) -> R + Sync) -> Vec<R> {
    let queue = std::sync::Mutex::new(jobs.enumerate());
    // The lock is let go of before the job is worked on.
    let next = || queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner).next();
    let run = |scratch: &mut S| {
        let mut done = Vec::new();
        while let Some((i, job)) = next() {
            done.push((i, work(job, scratch)));
        }
        done
    };
    let run = &run;
    let (first, others) = scratch.split_first_mut().expect("a scratch");
    let mut done = std::thread::scope(|scope| {
        let others: Vec<_> = others.iter_mut().map(|scratch| scope.spawn(move || run(scratch))).collect();
        let mut done = run(first);
        for other in others {
            done.extend(other.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)));
        }
        done
    });
    done.sort_unstable_by_key(|&(i, _)| i);
    done.into_iter().map(|(_, r)| r).collect()
}

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
    /// `plane` from pixel column `x` of the frame in row `y` on.
    fn from(&self, plane: usize, y: usize, x: usize) -> &[u8] {
        &self.planes[plane][y * self.stride + x - self.x..]
    }

    /// `len` pixels of row `y` of `plane` from pixel column `x` of the frame.
    fn row(&self, plane: usize, y: usize, x: usize, len: usize) -> &[u8] {
        &self.from(plane, y, x)[..len]
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

    /// The `w`×`h` of `plane` whose corner is at `(x, y)`, as a decoder reads
    /// a reference frame `width`×`height`, past the picture's edges the pixel
    /// at the edge, and how far apart its rows are: as it is where it is
    /// inside the picture and in one column, and otherwise copied into
    /// `scratch`.
    pub(super) fn block<'s>(&'s self, plane: usize, (x, y): (isize, isize), (w, h): (usize, usize), (width, height): (usize, usize), scratch: &'s mut Vec<u8>) -> (&'s [u8], usize) {
        let inside = x >= 0 && y >= 0 && x as usize + w <= width && y as usize + h <= height;
        if let Some(column) = inside.then(|| self.holding(x as usize, w)).flatten() {
            return (column.from(plane, y as usize, x as usize), column.stride);
        }
        scratch.resize(w * h, 0);
        for (r, row) in scratch.chunks_exact_mut(w).enumerate() {
            self.read_row(plane, (y + r as isize).clamp(0, height as isize - 1) as usize, x, width, row);
        }
        (&scratch[..w * h], w)
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
    /// Whether frames are loop filtered: where the libvpx encoder's are.
    filtered: bool,
    /// The most threads the tile columns are coded on.
    threads: usize,
    quality: u8,
    /// Per tile column, what a decoder keeps while it reads it.
    contexts: Vec<tile::Contexts>,
    /// The probabilities a decoder keeps from the frame coded last.
    probs: probs::Probs,
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
    /// Of the blocks the search between pixels could refine in the next
    /// frame, the one in this many it does, and what the blocks it refined
    /// would cost and what it saved (`tile::Contexts::searched`), summed over
    /// the frames so far, each frame's half again of the one after.
    search_every: usize,
    searched: [f64; 2],
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
            filtered: loop_filtered(Chroma::Full, width, threads),
            threads,
            quality: quality.clamp(QUALITY_MIN, QUALITY_MAX),
            contexts: (0..1 << tile_cols_log2).map(|i| tile::Contexts::new(&geometry, geometry.tile_cols(i, tile_cols_log2))).collect(),
            probs: probs::Probs::default(),
            recon: Reconstruction::new(&geometry, tile_cols_log2),
            reference: Reconstruction::new(&geometry, tile_cols_log2),
            previous: std::array::from_fn(|_| vec![0; pixels]),
            unchanged: vec![false; geometry.mi_rows * geometry.mi_cols],
            search: motion::Search::new(&geometry, threads),
            started: false,
            last_inter: false,
            requantize: false,
            search_every: 1,
            searched: [0.0; 2],
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
        let vectors = self.search.update(&self.previous[0], (!everything).then_some(&self.unchanged[..]), (!keyframe).then_some(&self.reference));
        for contexts in &mut self.contexts {
            contexts.start(self.last_inter);
        }
        let g = &self.geometry;
        let q_index = tables::QUANTIZER_TO_QINDEX[quality_to_q(self.quality) as usize];
        let quantizer = quantize::Quantizer::new(q_index);
        let source = tile::Source { planes: picture.planes(), stride: picture.strides()[0], width: usize::from(g.width), height: usize::from(g.height) };
        let previous = (!keyframe).then(|| tile::Previous { reference: &self.reference, unchanged: &self.unchanged, vectors: &vectors, search_every: self.search_every });
        // A keyframe starts a decoder's probabilities over from the defaults;
        // an inter frame takes the ones the frame before left.
        let mut probs = if keyframe { probs::Probs::default() } else { self.probs.clone() };
        let costs = tokens::Costs::new(&probs);
        let frame = tile::Frame { geometry: g, source: &source, previous, quantizer: &quantizer, costs: &costs };
        tile::code(self.threads, &frame, &mut self.contexts, &mut self.recon.columns);
        // The header updates the probabilities from what the frame's bools
        // count, and then the bools are coded.
        let mut counts = probs::Counts::new();
        for tape in self.contexts.iter().flat_map(|contexts| &contexts.tapes) {
            counts.add(&tape.counts);
        }
        let compressed = header::compressed(keyframe, &mut probs, &counts);
        // Each frame's half again of what came before, so that a frame with
        // little to search says little.
        self.searched = self.contexts.iter().fold(self.searched.map(|v| v / 2.0), |[c, s], contexts| [c + contexts.searched[0], s + contexts.searched[1]]);
        let [came, saved] = self.searched;
        if came > 0.0 {
            self.search_every = if saved < came * SEARCH_PAYS { SEARCH_PROBE } else { 1 };
        }
        let filter_level = if self.filtered { loopfilter::level(quantizer.ac_step(), keyframe) } else { 0 };
        let table = probs.table();
        let tiles = on_threads(self.threads, self.contexts.iter(), |contexts| tape::code(&contexts.tapes, &table));
        let uncompressed = header::Uncompressed {
            keyframe,
            width: g.width,
            height: g.height,
            q_index,
            filter_level,
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
        if filter_level > 0 {
            self.filter(filter_level);
        }
        std::mem::swap(&mut self.recon, &mut self.reference);
        self.probs = probs;
        self.started = true;
        self.last_inter = !keyframe;
        self.requantize = false;
        Ok(keyframe)
    }

    /// Run the loop filter at `level` over the frame just coded, as a
    /// decoder does before it keeps the frame: each plane on a thread of its
    /// own, as far as there are threads.
    fn filter(&mut self, level: u8) {
        let g = &self.geometry;
        let log2 = self.tile_cols_log2;
        let tiles: Vec<_> = (0..1 << log2).map(|i| g.tile_cols(i, log2).start).zip(&self.contexts).collect();
        let masks: Vec<loopfilter::Masks> = (0..g.sb_rows * g.sb_cols)
            .map(|i| {
                let (mi_row, mi_col) = (i / g.sb_cols * 8, i % g.sb_cols * 8);
                let &(start, contexts) = &tiles[tiles.partition_point(|&(start, _)| start <= mi_col) - 1];
                loopfilter::Masks::new(g, mi_row, mi_col, level, |r, c| contexts.filtered(r, c - start))
            })
            .collect();
        let mut planes: [Vec<(usize, usize, &mut Vec<u8>)>; 3] = Default::default();
        for column in &mut self.recon.columns {
            for (p, plane) in column.planes.iter_mut().enumerate() {
                planes[p].push((column.x, column.stride, plane));
            }
        }
        on_threads(self.threads.min(3), planes.into_iter(), |mut columns| {
            // A superblock that starts a tile column has its vertical pass
            // run on a copy of it with the eight pixels left of it, which
            // lie in the column before.
            let mut copy = vec![0u8; 72 * 64];
            for sb_row in 0..g.sb_rows {
                let mi_row = sb_row * 8;
                let rows = (g.mi_rows - mi_row).min(8);
                for sb_col in 0..g.sb_cols {
                    let m = &masks[sb_row * g.sb_cols + sb_col];
                    let x = sb_col * 64;
                    let t = columns.partition_point(|&(start, ..)| start <= x) - 1;
                    let (start, stride) = (columns[t].0, columns[t].1);
                    let at = mi_row * 8 * stride + x - start;
                    if x == start && t > 0 {
                        let (before, after) = columns.split_at_mut(t);
                        let (left, left_stride) = (&mut *before[t - 1].2, before[t - 1].1);
                        let this = &mut *after[0].2;
                        for y in 0..rows * 8 {
                            copy[y * 72..][..8].copy_from_slice(&left[(mi_row * 8 + y + 1) * left_stride - 8..][..8]);
                            copy[y * 72 + 8..][..64].copy_from_slice(&this[at + y * stride..][..64]);
                        }
                        loopfilter::vertical(&mut copy, 8, 72, rows, m);
                        for y in 0..rows * 8 {
                            left[(mi_row * 8 + y + 1) * left_stride - 8..][..8].copy_from_slice(&copy[y * 72..][..8]);
                            this[at + y * stride..][..64].copy_from_slice(&copy[y * 72 + 8..][..64]);
                        }
                    } else {
                        loopfilter::vertical(columns[t].2, at, stride, rows, m);
                    }
                    loopfilter::horizontal(columns[t].2, at, stride, rows, m);
                }
            }
        });
    }

    /// Mark each 8×8 of the frame unchanged where `picture` is the previous
    /// source over the part of it the picture holds, and make the previous
    /// source `picture`, in a band of 8×8 rows on each thread. With
    /// `everything`, every 8×8 is changed.
    fn compare(&mut self, picture: &Picture, everything: bool) {
        let g = &self.geometry;
        let width = usize::from(g.width);
        let (planes, stride) = (picture.planes(), picture.strides()[0]);
        let band = g.mi_rows.div_ceil(self.threads);
        let [p0, p1, p2] = &mut self.previous;
        let bands = self.unchanged.chunks_mut(band * g.mi_cols).zip(p0.chunks_mut(band * 8 * width)).zip(p1.chunks_mut(band * 8 * width)).zip(p2.chunks_mut(band * 8 * width));
        on_threads(self.threads, bands.enumerate(), |(b, (((unchanged, p0), p1), p2))| {
            let mut previous = [p0, p1, p2];
            let y0 = b * band * 8;
            let rows = previous[0].len() / width;
            let source = |plane: usize, y: usize| &planes[plane][(y0 + y) * stride..][..width];
            if everything {
                for (plane, previous) in previous.iter_mut().enumerate() {
                    for y in 0..rows {
                        previous[y * width..][..width].copy_from_slice(source(plane, y));
                    }
                }
                unchanged.fill(false);
                return;
            }
            for (r, unchanged) in unchanged.chunks_exact_mut(g.mi_cols).enumerate() {
                let rows = r * 8..(r * 8 + 8).min(rows);
                // Eight pixels at a time, each run of them an 8×8's row.
                unchanged.fill(true);
                for (plane, previous) in previous.iter().enumerate() {
                    for y in rows.clone() {
                        let (old, old_rest) = previous[y * width..][..width].as_chunks::<8>();
                        let (new, new_rest) = source(plane, y).as_chunks::<8>();
                        for (same, (a, b)) in unchanged.iter_mut().zip(old.iter().zip(new)) {
                            *same &= u64::from_ne_bytes(*a) == u64::from_ne_bytes(*b);
                        }
                        if !old_rest.is_empty() {
                            unchanged[g.mi_cols - 1] &= old_rest == new_rest;
                        }
                    }
                }
                // Where the source is unchanged it is the previous source
                // already, so a row with a change is taken whole.
                if unchanged.contains(&false) {
                    for (plane, previous) in previous.iter_mut().enumerate() {
                        for y in rows.clone() {
                            previous[y * width..][..width].copy_from_slice(source(plane, y));
                        }
                    }
                }
            }
        });
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
        /// The frame coded last.
        last: Vec<u8>,
    }

    impl Stream {
        fn new(w: u16, h: u16, quality: u8, threads: usize) -> Self {
            Self { encoder: Encoder::new(w, h, quality, threads).expect("an encoder"), decoder: Decoder::new(1).expect("a decoder"), frames: 0, last: Vec::new() }
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
            let mse = mse(picture, &decoded);
            self.last = frame;
            (self.last.len(), keyframe, mse)
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

    /// Where the libvpx encoder leaves the loop filter out, of 4:4:4 in four
    /// tile columns, so does this one, and every frame's header says so.
    #[test]
    fn the_loop_filter_is_left_out_where_libvpx_leaves_it_out() {
        for (w, threads, filtered) in [(2047, 4, true), (2048, 4, false), (2048, 2, true)] {
            let rgb = desktop_rgb(w, 64);
            let mut stream = Stream::new(w, 64, 50, threads);
            for (n, rgb) in [scroll(&rgb, w, 5), rgb].iter().enumerate() {
                stream.frame(&picture(w, 64, rgb), false);
                assert_eq!(crate::tests::filter_level(&stream.last) > 0, filtered, "frame {n} of {w} wide on {threads} threads");
            }
        }
    }

    #[test]
    fn tile_columns_follow_the_width_and_the_threads() {
        // 3456 wide on 4 threads is four columns of 13 or 14 superblocks; on
        // 2, two of 27; on 1, one.
        let four = Encoder::new(3456, 8, 90, 4).expect("an encoder");
        let two = Encoder::new(3456, 8, 90, 2).expect("an encoder");
        let one = Encoder::new(3456, 8, 90, 1).expect("an encoder");
        assert_eq!((four.tile_cols_log2, two.tile_cols_log2, one.tile_cols_log2), (2, 1, 0));
        assert_eq!(four.geometry.tile_cols(1, 2), 104..216);
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
        // Past 4096 there are two tile columns on one thread as on two or
        // three, while the bands the source is compared and hashed in are
        // one per thread.
        let (w, h) = (4160, 72);
        let rgb = desktop_rgb(w, h);
        let frames = [rgb.clone(), scroll(&rgb, w, 5), drag(&rgb, w, (1900, 8, 400, 40), (300, 3)), edit(rgb, w, (2000, 30, 300, 30), 7)];
        let [one, two, three] = [1, 2, 3].map(|threads| {
            let mut encoder = Encoder::new(w, h, 90, threads).expect("an encoder");
            assert_eq!(encoder.tile_cols_log2, 1);
            let mut out = Vec::new();
            for rgb in &frames {
                encoder.encode(&picture(w, h, rgb), false, &mut out).expect("a frame");
            }
            out
        });
        assert!(one == two && one == three, "{} bytes on one thread, {} on two, {} on three", one.len(), two.len(), three.len());
    }

    #[test]
    fn the_rows_of_one_tile_column_change_nothing_but_the_time() {
        // Under 1440 wide a frame is one tile column, whose superblock rows
        // more threads code as a wavefront: each row reads only what the rows
        // above it leave it, so the frames are the same on any number of
        // threads and decode as the encoder made them.
        let (w, h) = (640, 360);
        let rgb = desktop_rgb(w, h);
        let frames = [rgb.clone(), scroll(&rgb, w, 5), drag(&rgb, w, (100, 60, 300, 200), (17, 9)), edit(rgb, w, (300, 200, 200, 100), 7)];
        let [one, two, five] = [1, 2, 5].map(|threads| {
            let mut stream = Stream::new(w, h, 90, threads);
            assert_eq!(stream.encoder.tile_cols_log2, 0);
            frames.iter().map(|rgb| {
                stream.frame(&picture(w, h, rgb), false);
                stream.last.clone()
            }).collect::<Vec<_>>()
        });
        assert!(one == two && one == five, "{} frames on one thread, {} on two, {} on five", one.len(), two.len(), five.len());
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

    /// A `w`×`h` in RGB of thirds that one intra mode each predicts best:
    /// stripes down, which V predicts; stripes across, H; and a plane
    /// sloping both ways, TM. Each `k` makes others, none of them in
    /// another's picture.
    fn lines_rgb(w: u16, h: u16, k: usize) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(usize::from(w) * usize::from(h) * 3);
        for y in 0..usize::from(h) {
            for x in 0..usize::from(w) {
                let v = match (x * 3 / usize::from(w) + k) % 3 {
                    0 => (x * (37 + 6 * k)) % 251,
                    1 => (y * (53 + 10 * k)) % 241,
                    _ => (x % 64) * (k + 1) + (2 + k) * (y % 32),
                } as u8;
                rgb.extend([v, v / 2 + 40, 255 - v]);
            }
        }
        rgb
    }

    #[test]
    fn the_intra_modes_decode_where_they_predict_best() {
        // In a keyframe, and in an inter frame whose picture the frame
        // before has nothing of; in one tile column and in four, whose
        // blocks along a column's left edge have no column to their left.
        for (w, h, threads) in [(203, 141, 1), (2048, 72, 4)] {
            let mut stream = Stream::new(w, h, 90, threads);
            for (i, keyframe) in [true, false].into_iter().enumerate() {
                stream.frame(&picture(w, h, &lines_rgb(w, h, i)), keyframe);
                let mut used = [0; 4];
                for contexts in &stream.encoder.contexts {
                    for mode in contexts.intra_modes() {
                        used[predict::IntraMode::ALL.iter().position(|&m| m == mode).expect("a mode")] += 1;
                    }
                }
                assert!(used[1..].iter().all(|&n| n > 0), "{w}x{h}, frame {i}: DC, V, H and TM coded {used:?} times");
            }
        }
    }

    /// A `w`×`h` in RGB of soft waves, as video is and a desktop is not,
    /// moved `shift` pixels right and down: what only a vector between
    /// pixels predicts well.
    fn waves_rgb(w: u16, h: u16, (sx, sy): (f64, f64)) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(usize::from(w) * usize::from(h) * 3);
        for y in 0..h {
            for x in 0..w {
                let (x, y) = (f64::from(x) - sx, f64::from(y) - sy);
                let wave = |a: f64, b: f64, c: f64| (128.0 + 90.0 * (x * a + y * b + c).sin()).round() as u8;
                rgb.extend([wave(0.21, 0.07, 0.0), wave(-0.05, 0.17, 1.0), wave(0.13, -0.11, 2.0)]);
            }
        }
        rgb
    }

    #[test]
    fn motion_between_pixels_decodes() {
        // Small pictures and their edges, and four tile columns.
        for (w, h, threads) in [(130, 67, 1), (200, 120, 1), (3840, 136, 4)] {
            let mut stream = Stream::new(w, h, 90, threads);
            let mut fractions = 0;
            for (i, shift) in [(0.0, 0.0), (0.5, 0.0), (0.75, 0.25), (1.5, 1.0), (-2.25, 0.5), (-2.25, 0.5)].into_iter().enumerate() {
                let (bytes, keyframe, _) = stream.frame(&picture(w, h, &waves_rgb(w, h, shift)), false);
                assert_eq!(keyframe, i == 0, "{w}x{h}, frame {i}, {bytes} bytes");
                fractions += stream.encoder.contexts.iter().flat_map(|c| c.vectors()).filter(|mv| mv.row % 8 != 0 || mv.col % 8 != 0).count();
            }
            assert!(fractions > 0, "{w}x{h}: no vector between pixels");
            assert_eq!(stream.encoder.search_every, 1, "{w}x{h}: the search pays on waves that move");
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
        // previous frame, whose edges the loop filter runs over again.
        assert!(bytes < 40, "{bytes} bytes");
        assert!((again - first).abs() < first / 100.0, "squared error {again} against {first}");
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
        // Outside the superblock the edit is in, the frame is the one before,
        // not coded again, but for the seven pixels each side of the
        // superblocks' edges that the loop filter runs over again.
        let after = stream.encoder.reconstruction();
        let stride = usize::from(w);
        let edge = |v: usize| !(7..57).contains(&(v % 64));
        for (p, (a, b)) in after.iter().zip(&before).enumerate() {
            for y in 0..usize::from(h) {
                for x in 0..usize::from(w) {
                    let touched = (64..128).contains(&x) && (64..128).contains(&y) || edge(x) || edge(y);
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
    fn a_scroll_that_arrives_with_noise_is_coded_as_motion() {
        let (w, h) = (640, 384);
        let mut stream = Stream::new(w, h, 100, 1);
        let rgb = desktop_rgb(w, h);
        let (key_bytes, _, _) = stream.frame(&picture(w, h, &rgb), false);
        // Two of a terminal's rows, and every value a step or two off, as a
        // picture that was coded with loss has them: no window of it is
        // the same as one before.
        let mut scrolled = scroll(&rgb, w, 38);
        for (v, n) in scrolled.iter_mut().zip(noise_rgb(w, h, 7)) {
            *v = v.saturating_add(n % 3).saturating_sub(1);
        }
        let (bytes, keyframe, mse) = stream.frame(&picture(w, h, &scrolled), false);
        assert!(!keyframe);
        // Coded as it is, it costs nearly a keyframe.
        assert!(bytes * 5 < key_bytes, "{bytes} bytes against a keyframe of {key_bytes}");
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
        // But for the loop filter run over the skipped blocks' edges again.
        assert!((same - coarse).abs() < coarse / 100.0, "squared error {same} against {coarse}");
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

    /// The 4:4:4 samples `docs/rust-encoder-plan.md` measures on, each
    /// with the quality and the threads the remotex gateway codes them at.
    const SAMPLES: [(&str, u8, usize); 18] = [
        ("quiet-1280x800-1col-lf", 90, 5),
        ("quiet-1440x900-2col-lf", 90, 5),
        ("quiet-1600x1000-2col-lf", 90, 5),
        ("quiet-1920x1080-2col-lf", 90, 5),
        ("quiet-2048x1536-4col-nolf", 90, 5),
        ("quiet-2560x1600-4col-nolf", 90, 5),
        ("quiet-2880x1800-4col-nolf", 90, 5),
        ("quiet-3456x2168-4col-nolf", 90, 5),
        ("quiet-3840x2160-4col-nolf", 90, 5),
        ("busy-1280x800-1col-lf", 90, 5),
        ("busy-1440x900-2col-lf", 90, 5),
        ("busy-1600x1000-2col-lf", 90, 5),
        ("busy-1920x1080-2col-lf", 90, 5),
        ("busy-2048x1536-4col-nolf", 90, 5),
        ("busy-2560x1600-4col-nolf", 90, 5),
        ("busy-2880x1800-4col-nolf", 90, 5),
        ("busy-3456x2168-4col-nolf", 90, 5),
        ("busy-3840x2160-4col-nolf", 90, 5),
    ];

    /// Hand `f` each frame of an IVF sample in turn, as the picture a user
    /// would hand an encoder, until it returns false.
    fn each_picture(ivf: &[u8], mut f: impl FnMut(usize, Picture) -> bool) {
        assert_eq!(&ivf[..4], b"DKIF", "not an IVF file");
        let mut decoder = Decoder::new(4).expect("a decoder");
        let (mut at, mut n) = (32, 0);
        while at + 12 <= ivf.len() {
            let len = u32::from_le_bytes(ivf[at..at + 4].try_into().expect("four bytes")) as usize;
            let decoded = decoder.decode(&ivf[at + 12..at + 12 + len]).expect("a sample libvpx decodes");
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

    /// Every `step`th frame of an IVF sample, up to `count` of them.
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

    /// The first 60 frames of the sample `SAMPLE` names in `tmp/`, coded
    /// by this encoder alone, `PASSES` times over (3 if unset), on `THREADS`
    /// threads (the sample's if unset): its milliseconds per frame. For a
    /// profiler to watch; run it in release:
    ///
    /// ```text
    /// SAMPLE=busy-1920x1080-2col-lf cargo test --release rust_alone -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads a sample in tmp/ and wants a release build"]
    fn rust_alone() {
        let name = std::env::var("SAMPLE").expect("SAMPLE naming a sample");
        let &(_, quality, threads) = SAMPLES.iter().find(|c| c.0 == name).expect("one of the samples");
        let threads = std::env::var("THREADS").map_or(threads, |t| t.parse().expect("a thread count"));
        let passes: usize = std::env::var("PASSES").map_or(3, |p| p.parse().expect("a pass count"));
        let ivf = std::fs::read(format!("tmp/{name}.ivf")).expect("the sample in tmp/");
        let pictures = sample(&ivf, 1, 60);
        let (w, h) = pictures[0].size();
        let mut out = Vec::new();
        let mut bytes = 0;
        let started = std::time::Instant::now();
        for _ in 0..passes {
            let mut encoder = Encoder::new(w, h, quality, threads).expect("an encoder");
            bytes = 0;
            for picture in &pictures {
                out.clear();
                encoder.encode(picture, false, &mut out).expect("a frame");
                bytes += out.len();
            }
        }
        println!("{name} on {threads}: {:.2} ms per frame, {bytes} bytes", started.elapsed().as_secs_f64() * 1000.0 / (passes * pictures.len()) as f64);
    }

    fn psnr(mse: f64) -> f64 {
        10.0 * (255.0 * 255.0 / mse).log10()
    }

    /// Keyframes of frames taken from the samples in `tmp/`, coded by this
    /// encoder and by libvpx on the sample's threads: bytes, the least time over
    /// three runs, and the PSNR of libvpx's decode of each. A measurement,
    /// not a check; run it in release:
    ///
    /// ```text
    /// cargo test --release keyframes_against_libvpx -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads the samples in tmp/ and wants a release build"]
    fn keyframes_against_libvpx() {
        for (name, quality, threads) in SAMPLES {
            let Ok(ivf) = std::fs::read(format!("tmp/{name}.ivf")) else {
                println!("{name}: not in tmp/");
                continue;
            };
            let pictures = sample(&ivf, 15, 8);
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

    /// Every frame of each sample in `tmp/`, coded as a stream by this
    /// encoder and by libvpx, on the sample's threads, frame by frame, which of
    /// them goes first alternating: bytes, time and the PSNR of libvpx's
    /// decode of each. The per-frame time ratio's median is what the plan's
    /// gate reads. A measurement, not a check; run it in release:
    ///
    /// ```text
    /// cargo test --release streams_against_libvpx -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads the samples in tmp/ and wants a release build"]
    fn streams_against_libvpx() {
        for (name, quality, threads) in SAMPLES {
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
                true
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
