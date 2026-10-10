//! The VP9 encoder: 8-bit 4:4:4, profile 1, the subset of VP9 a desktop is
//! coded in (`docs/encoder.md`) and no more.
//!
//! It codes keyframes and inter frames against the frame before, each
//! superblock split as far as its source varies, every block predicted by an
//! intra mode or from the previous frame at a whole-pixel vector, at a
//! transform size of its own, its residual coded at the dial's quantizer, and
//! the frame reconstructed exactly as a decoder reconstructs it, its loop
//! filter run over it before it is kept. Where the source has not changed
//! since the frame before, a block is that frame's, skipped; where it moved,
//! a hash search finds where it came from. An inter frame's blocks are in two
//! segments, as libvpx's are under an active map: the blocks left as the
//! previous frame has them are the inactive segment, which a decoder reads
//! nothing of but the segment and does not loop filter, and the rest the
//! active one. A frame's bools are recorded as its blocks are decided and
//! coded once its header has updated the probabilities to what they count,
//! which a decoder keeps for the next frame. Tile columns, which a decoder
//! reads independently of each other, are decided and coded on threads of
//! their own.

mod bool_writer;
mod dct;
mod header;
mod lanes;
mod loopfilter;
mod motion;
mod mv;
mod pixels;
mod pool;
mod predict;
mod probs;
mod quantize;
mod tables;
mod tape;
mod tile;
mod tokens;
mod transform;

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{Error, MAX_THREADS, Picture, QUALITY_MAX, QUALITY_MIN, Rect, Speed, loop_filtered, quality_to_q, tile_columns_log2};
use pool::Pool;


/// The widest a tile column may be, in superblocks.
const MAX_TILE_WIDTH_B64: usize = 64;
/// The narrowest a tile column may be, in superblocks.
const MIN_TILE_WIDTH_B64: usize = 4;
/// The records a tile column's tapes hold for each tile row the frame is
/// in, two rows from this many and four from four times it: a busy
/// keyframe's bools at 1920×1080 are coded in 8 ms of 18 in four rows, for
/// 0.02% more bytes, and a line of text stays in one.
const TILE_ROW_RECORDS: usize = 1 << 14;
/// The 8×8s a frame may code at most to be coded on one thread: a key
/// typed or a line of text, which the threads would not have woken for
/// before it is done. A line of text at 1920×1080 takes 0.57 ms on one
/// thread and 1.02 on four; from 4096 on, a scroll takes half as long
/// again on one.
const LIGHT_FRAME: usize = 1024;

/// Run `work` on each of `jobs` on up to `threads` of `pool`'s threads,
/// this one among them, as many jobs to a turn as leave none over if every
/// thread takes one turn, each thread taking the next turn as it finishes
/// one; and give back what each gives, in order.
fn on_threads<J: Send, R: Send>(pool: &Pool, threads: usize, jobs: impl ExactSizeIterator<Item = J>, work: impl Fn(J) -> R + Sync) -> Vec<R> {
    let per = jobs.len().div_ceil(threads.max(1)).max(1);
    let mut chunks: Vec<Mutex<Vec<J>>> = Vec::new();
    for (i, job) in jobs.enumerate() {
        if i % per == 0 {
            chunks.push(Mutex::new(Vec::with_capacity(per)));
        }
        chunks.last_mut().expect("pushed above").get_mut().unwrap_or_else(std::sync::PoisonError::into_inner).push(job);
    }
    let done: Vec<Mutex<Vec<R>>> = chunks.iter().map(|_| Mutex::new(Vec::new())).collect();
    let next = AtomicUsize::new(0);
    pool.run(chunks.len(), |_| {
        while let Some((chunk, done)) = Some(next.fetch_add(1, Ordering::Relaxed)).and_then(|i| chunks.get(i).zip(done.get(i))) {
            let chunk = std::mem::take(&mut *chunk.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
            *done.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = chunk.into_iter().map(&work).collect();
        }
    });
    done.into_iter().flat_map(|d| d.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner)).collect()
}

/// Run `work` on each of `jobs` on as many of `pool`'s threads as there are
/// `scratch`es, this one among them, each with a scratch of its own and
/// taking the next job as it finishes one, so that a thread whose jobs have
/// little to do takes more of them; and give back what each gives, in
/// order.
fn on_workers<J: Send, S: Send, R: Send>(pool: &Pool, jobs: impl Iterator<Item = J> + Send, scratch: &mut [S], work: impl Fn(J, &mut S) -> R + Sync) -> Vec<R> {
    let queue = Mutex::new(jobs.enumerate());
    // The lock is let go of before the job is worked on.
    let next = || queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner).next();
    let slots: Vec<Mutex<&mut S>> = scratch.iter_mut().map(Mutex::new).collect();
    let done = Mutex::new(Vec::new());
    pool.run(slots.len(), |i| {
        let mut scratch = slots[i].lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut mine = Vec::new();
        while let Some((k, job)) = next() {
            mine.push((k, work(job, &mut scratch)));
        }
        done.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(mine);
    });
    let mut done = done.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
    done.sort_unstable_by_key(|&(i, _)| i);
    done.into_iter().map(|(_, r)| r).collect()
}

/// A tile column's part of a plane of the reconstruction: its first pixel
/// column of the frame, its stride and its pixels.
type Strip<'a> = (usize, usize, &'a mut Vec<u8>);

/// What is left of a frame once its blocks are decided, a job each.
enum Finish<'a> {
    /// A tile's tapes to code: those of its superblock rows.
    Tapes(&'a [tape::Tape]),
    /// A plane to loop filter, in its tile columns.
    Plane(Vec<Strip<'a>>),
}

/// Loop filter one plane, in the tile columns `columns`, over the
/// superblocks that have `masks`; `copy` is room for a superblock and the
/// eight pixels left of it.
fn filter_plane(g: &Geometry, masks: &[Option<loopfilter::Masks>], mut columns: Vec<Strip<'_>>, copy: &mut [u8]) {
    for sb_row in 0..g.sb_rows {
        let mi_row = sb_row * 8;
        let rows = (g.mi_rows - mi_row).min(8);
        for sb_col in 0..g.sb_cols {
            let Some(m) = &masks[sb_row * g.sb_cols + sb_col] else { continue };
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
                loopfilter::vertical(copy, 8, 72, rows, m);
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

/// One VP9 stream at one picture size: 8-bit 4:4:4, its quantizer pinned to
/// the dial. A picture of another size needs another encoder, whose first
/// frame is a keyframe by construction. Within one encoder, every inter frame
/// is expressed as a change from the frame before.
pub struct Encoder {
    geometry: Geometry,
    tile_cols_log2: u32,
    /// Whether frames are loop filtered (`crate::loop_filtered`).
    filtered: bool,
    /// The most threads the tile columns are coded on.
    threads: usize,
    /// The threads, this one among them, for the encoder's life.
    pool: Pool,
    /// How many of them the frame being coded is coded on: one, where it
    /// may code so little that waking the rest costs more than they do.
    at_once: usize,
    quality: u8,
    speed: Speed,
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
    /// Per 8×8 of the frame, whether the source is `previous` there, or the
    /// frame may not code it: what the index and the projections go stale
    /// by.
    still: Vec<bool>,
    /// Per 8×8 of the frame, whether it is `still` and was last coded at
    /// the dial's quantizer: what the frame skips. One still at another
    /// quantizer settles, coded where it is with its residual.
    unchanged: Vec<bool>,
    /// Per 8×8 of the frame, whether the frame being coded may code it: it
    /// is in a rectangle the frame was told the picture changed in, or the
    /// frame was told nothing.
    named: Vec<bool>,
    /// Per superblock, whether the frame touches it: one of its 8×8s is
    /// named, or one of a neighbour's, whose intra prediction or loop filter
    /// reaches into it. The rest the frame leaves in the reference as they
    /// are, and the ones it touches it writes back once coded and filtered;
    /// a frame that touches every superblock keeps its reconstruction as
    /// the reference instead.
    touched: Vec<bool>,
    /// Per 8×8 of the frame, the dial it was last coded at.
    coded_at: Vec<u8>,
    search: motion::Search,
    /// Whether a frame has been coded, so the next can be an inter frame.
    started: bool,
    /// Whether the frame coded last was an inter frame, whose vectors a
    /// decoder predicts the next frame's from.
    last_inter: bool,
    /// Whether a decoder holds the inactive segment's features: an inter
    /// frame since the last keyframe wrote them.
    segmented: bool,
}

impl Encoder {
    /// An encoder for a `width`×`height` picture, starting at `quality`
    /// (1–100, clamped), coded by `threads` threads. The thread count is the
    /// caller's: how many cores a machine can spare is a question about what
    /// else it runs. It also bounds the tile columns, and so decides with the
    /// width whether a stream is loop filtered: from 2048 wide on four
    /// threads or more it is not, nor past 8192 wide on any, where the
    /// format asks for four columns itself.
    pub fn new(width: u16, height: u16, quality: u8, threads: usize) -> Result<Self, Error> {
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
            filtered: loop_filtered(tile_cols_log2),
            threads,
            pool: Pool::new(threads),
            at_once: threads,
            quality: quality.clamp(QUALITY_MIN, QUALITY_MAX),
            speed: Speed::default(),
            contexts: (0..1 << tile_cols_log2).map(|i| tile::Contexts::new(&geometry, geometry.tile_cols(i, tile_cols_log2))).collect(),
            probs: probs::Probs::default(),
            recon: Reconstruction::new(&geometry, tile_cols_log2),
            reference: Reconstruction::new(&geometry, tile_cols_log2),
            previous: std::array::from_fn(|_| vec![0; pixels]),
            still: vec![false; geometry.mi_rows * geometry.mi_cols],
            unchanged: vec![false; geometry.mi_rows * geometry.mi_cols],
            named: vec![true; geometry.mi_rows * geometry.mi_cols],
            touched: vec![true; geometry.sb_rows * geometry.sb_cols],
            coded_at: vec![quality; geometry.mi_rows * geometry.mi_cols],
            search: motion::Search::new(&geometry, threads),
            started: false,
            last_inter: false,
            segmented: false,
            geometry,
        })
    }

    /// The picture size this encoder codes.
    pub fn size(&self) -> (u16, u16) {
        (self.geometry.width, self.geometry.height)
    }

    /// The dial this encoder is coding at.
    pub fn quality(&self) -> u8 {
        self.quality
    }

    /// Move the dial (1–100, clamped) without a keyframe: the next frame
    /// codes every block it may code again at the new quantizer, changed or
    /// not. This is how a link that is behind gives quality up, and "without
    /// a keyframe" is the whole reason: a keyframe is the most bytes a frame
    /// can be, at the exact moment the link has run out of room.
    pub fn set_quality(&mut self, quality: u8) {
        self.quality = quality.clamp(QUALITY_MIN, QUALITY_MAX);
    }

    /// The speed the encoder is coding at.
    pub fn speed(&self) -> Speed {
        self.speed
    }

    /// Move the speed: `Fastest` decides by the proxies alone, `Usual` by
    /// coding each way where that pays.
    pub fn set_speed(&mut self, speed: Speed) {
        self.speed = speed;
    }

    /// The coarsest dial any block of the picture a decoder now holds was
    /// last coded at: [`Self::quality`] as each frame was encoded, for the
    /// blocks that frame coded. A frame told where its picture changed leaves
    /// the rest at the dial they had, so a picture the dial was walked down
    /// for is coarse here until every block of it has been coded finer,
    /// which a settle does at once. Before the first frame, the dial.
    pub fn coarsest(&self) -> u8 {
        if !self.started {
            return self.quality;
        }
        self.coded_at.iter().copied().min().unwrap_or(self.quality)
    }

    /// Settle the picture at `quality`: code `picture` whole at that dial
    /// and leave the dial where it was. What a desktop that went quiet while the link had it coarse is
    /// owed (`crate::walk::QualityWalk::settle_at`): as an inter frame every
    /// 8×8 still at a coarser dial is coded where it is with its residual,
    /// so [`Self::coarsest`] is `quality` after it, and the frames of the
    /// motion that follows are at what the link bears again. `keyframe`
    /// makes it one a decoder can start from, at `quality` all the same.
    /// Returns what [`Self::encode`] does; a picture that is not this
    /// encoder's is refused with the dial as it was.
    pub fn settle(&mut self, picture: &Picture, quality: u8, keyframe: bool, out: &mut Vec<u8>) -> Result<bool, Error> {
        let walked = self.quality;
        self.set_quality(quality);
        let encoded = self.encode(picture, keyframe, None, out);
        // Back whatever the encode came to: the dial is the walk's again.
        self.quality = walked;
        encoded
    }

    /// Code `picture` and append the frame to `out`: a keyframe when
    /// `keyframe` asks for one or nothing has been coded yet, otherwise an
    /// inter frame against the frame before. Returns whether it is a
    /// keyframe.
    ///
    /// `changed` is where the picture differs from the one coded before it,
    /// or `None` for a picture that may differ anywhere: the 8×8s outside
    /// every rectangle are skipped, left as the decoder has them and at the
    /// dial they were coded at, whatever the picture holds there. It is the
    /// caller's word and not checked: a change outside the rectangles is not
    /// coded, by this frame or by a later one that does not name it. A
    /// keyframe codes everything.
    pub fn encode(&mut self, picture: &Picture, keyframe: bool, changed: Option<&[Rect]>, out: &mut Vec<u8>) -> Result<bool, Error> {
        let g = &self.geometry;
        if picture.size() != (g.width, g.height) {
            let (pw, ph) = picture.size();
            return Err(Error::Mismatch(g.width, g.height, pw, ph));
        }
        let keyframe = keyframe || !self.started;
        self.name(changed.filter(|_| !keyframe));
        self.at_once = if self.named.iter().filter(|&&named| named).count() > LIGHT_FRAME { self.threads } else { 1 };
        // The search's anchors are of the source coded last, which the
        // comparison is about to write over.
        if keyframe {
            self.search.forget();
        } else {
            self.search.catch_up(&self.pool, &self.previous[0]);
        }
        self.compare(picture, keyframe);
        let vectors = if keyframe { Vec::new() } else { self.search.update(&self.pool, self.at_once, &self.previous[0], &self.still, &self.reference) };
        for contexts in &mut self.contexts {
            contexts.start(self.last_inter);
        }
        let g = &self.geometry;
        let q_index = tables::QUANTIZER_TO_QINDEX[quality_to_q(self.quality) as usize];
        let quantizer = quantize::Quantizer::new(q_index);
        let source = tile::Source { planes: picture.planes(), stride: picture.strides()[0], width: usize::from(g.width), height: usize::from(g.height) };
        let previous = (!keyframe).then(|| tile::Previous { reference: &self.reference, still: &self.still, unchanged: &self.unchanged, touched: &self.touched, vectors: &vectors });
        // A keyframe starts a decoder's probabilities over from the defaults;
        // an inter frame takes the ones the frame before left.
        let mut probs = if keyframe { probs::Probs::default() } else { self.probs.clone() };
        let costs = tokens::Costs::new(&probs);
        let frame = tile::Frame { geometry: g, source: &source, previous, quantizer: &quantizer, costs: &costs, fast: self.speed == Speed::Fastest };
        // The header updates the probabilities from what the frame's bools
        // count, and then the bools are coded.
        let counts = tile::code(&self.pool, self.at_once, &frame, &mut self.contexts, &mut self.recon.columns);
        let compressed = header::compressed(keyframe, &mut probs, &counts);
        // An inter frame's blocks are in segments, the inactive segment's
        // features written once after each keyframe.
        let segmentation = (!keyframe).then(|| header::Segmentation::choose(&counts.segments, !self.segmented));
        let filter_level = if self.filtered { loopfilter::level(quantizer.ac_step(), keyframe) } else { 0 };
        let table = probs.table();
        // The bools of a tile column are coded by one thread, one after
        // another, which of a busy frame is the longest thing left: in tile
        // rows each is a bool coder of its own, and a decoder predicts
        // across them as if they were not there. A row costs its size and
        // its coder's last bytes, so a frame is in rows by how much its
        // heaviest column recorded.
        let heaviest = self.contexts.iter().map(|contexts| contexts.tapes.iter().map(tape::Tape::len).sum::<usize>()).max().unwrap_or(0);
        let tile_rows_log2 = match heaviest / TILE_ROW_RECORDS {
            4.. if self.geometry.sb_rows >= 4 => 2,
            1.. if self.geometry.sb_rows >= 2 => 1,
            _ => 0,
        };
        let tiles = self.finish(&table, segmentation.as_ref(), filter_level, tile_rows_log2);
        let g = &self.geometry;
        let uncompressed = header::Uncompressed {
            keyframe,
            width: g.width,
            height: g.height,
            q_index,
            filter_level,
            tile_cols_log2: self.tile_cols_log2,
            min_log2_tile_cols: g.min_log2_tile_cols(),
            max_log2_tile_cols: g.max_log2_tile_cols(),
            tile_rows_log2,
            segmentation,
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
        if self.touched.contains(&false) {
            self.merge();
        } else {
            std::mem::swap(&mut self.recon, &mut self.reference);
        }
        self.probs = probs;
        for (at, _) in self.coded_at.iter_mut().zip(&self.named).filter(|(_, named)| **named) {
            *at = self.quality;
        }
        self.started = true;
        self.last_inter = !keyframe;
        // A keyframe clears the features a decoder holds.
        self.segmented = !keyframe;
        Ok(keyframe)
    }

    /// Mark the 8×8s `changed` touches as the ones the frame may code, or
    /// with `None` all of them. What lies past the picture's edge is not
    /// part of it.
    fn name(&mut self, changed: Option<&[Rect]>) {
        let Some(rects) = changed else {
            self.named.fill(true);
            self.touched.fill(true);
            return;
        };
        let g = &self.geometry;
        self.named.fill(false);
        self.touched.fill(false);
        for rect in rects {
            let (x, y) = (usize::from(rect.x), usize::from(rect.y));
            let right = (x + usize::from(rect.width)).min(usize::from(g.width));
            let bottom = (y + usize::from(rect.height)).min(usize::from(g.height));
            if x >= right || y >= bottom {
                continue;
            }
            for row in y / 8..=(bottom - 1) / 8 {
                self.named[row * g.mi_cols + x / 8..=row * g.mi_cols + (right - 1) / 8].fill(true);
            }
            // The superblocks the rectangle lies in, and the ring around
            // them.
            let (first_row, last_row) = ((y / 64).saturating_sub(1), ((bottom - 1) / 64 + 1).min(g.sb_rows - 1));
            let (first_col, last_col) = ((x / 64).saturating_sub(1), ((right - 1) / 64 + 1).min(g.sb_cols - 1));
            for row in first_row..=last_row {
                self.touched[row * g.sb_cols + first_col..=row * g.sb_cols + last_col].fill(true);
            }
        }
    }

    /// Write the superblocks the frame touched from its reconstruction, coded
    /// and filtered, into the reference, a superblock row to a thread.
    fn merge(&mut self) {
        let Self { geometry: g, pool, at_once: threads, recon, reference, touched, .. } = self;
        // Per superblock row, per tile column and plane: the row's pixels in
        // the reconstruction and in the reference, the column's first pixel
        // column of the frame and its stride.
        type Strip<'a> = (&'a [u8], &'a mut [u8], usize, usize);
        let mut rows: Vec<Vec<Strip<'_>>> = (0..g.sb_rows).map(|_| Vec::new()).collect();
        for (from, to) in recon.columns.iter().zip(&mut reference.columns) {
            let (x, stride) = (from.x, from.stride);
            for (from, to) in from.planes.iter().zip(&mut to.planes) {
                for (row, (f, t)) in from.chunks(64 * stride).zip(to.chunks_mut(64 * stride)).enumerate() {
                    rows[row].push((f, t, x, stride));
                }
            }
        }
        on_threads(pool, *threads, rows.into_iter().enumerate(), |(sb_row, planes)| {
            for (from, to, x, stride) in planes {
                for sb_col in x / 64..((x + stride) / 64).min(g.sb_cols) {
                    if !touched[sb_row * g.sb_cols + sb_col] {
                        continue;
                    }
                    let at = sb_col * 64 - x;
                    for r in 0..64 {
                        to[r * stride + at..][..64].copy_from_slice(&from[r * stride + at..][..64]);
                    }
                }
            }
        });
    }

    /// Code each tile column's tapes at the probabilities of `table`, the
    /// segments as `segmentation` says, and with a `level` above 0 run the
    /// loop filter at it over the frame just coded, as a decoder does
    /// before it keeps the frame: neither waits on the other, so a tile
    /// column and a plane are each a job, a thread taking the next as it
    /// finishes one. A superblock with no active block has no edge to
    /// filter and is passed over. Gives back each tile column's bytes.
    fn finish(&mut self, table: &[u8], segmentation: Option<&header::Segmentation>, level: u8, tile_rows_log2: u32) -> Vec<Vec<u8>> {
        let Self { geometry: g, pool, at_once: threads, recon, contexts, tile_cols_log2: log2, .. } = self;
        let tiles: Vec<_> = (0..1 << *log2).map(|i| g.tile_cols(i, *log2).start).zip(&*contexts).collect();
        let masks: Vec<Option<loopfilter::Masks>> = (0..if level > 0 { g.sb_rows * g.sb_cols } else { 0 })
            .map(|i| {
                let (mi_row, mi_col) = (i / g.sb_cols * 8, i % g.sb_cols * 8);
                let &(start, contexts) = &tiles[tiles.partition_point(|&(start, _)| start <= mi_col) - 1];
                contexts.coded(mi_row / 8, (mi_col - start) / 8).then(|| loopfilter::Masks::new(g, mi_row, mi_col, level, |r, c| contexts.filtered(r, c - start)))
            })
            .collect();
        let mut planes: [Vec<Strip<'_>>; 3] = Default::default();
        if level > 0 {
            for column in &mut recon.columns {
                for (p, plane) in column.planes.iter_mut().enumerate() {
                    planes[p].push((column.x, column.stride, plane));
                }
            }
        }
        // The tapes first: a tile column's is the longer job.
        // The tiles in the order a frame holds them: a tile row's after
        // the row's above.
        let row = |i: usize| (i * g.sb_rows) >> tile_rows_log2;
        let coded = (0..1 << tile_rows_log2).flat_map(|i| contexts.iter().map(move |contexts| Finish::Tapes(&contexts.tapes[row(i)..row(i + 1)])));
        let jobs: Vec<Finish<'_>> = coded.chain(planes.into_iter().filter(|plane| !plane.is_empty()).map(Finish::Plane)).collect();
        // A superblock that starts a tile column has its vertical pass run
        // on a copy of it with the eight pixels left of it, which lie in
        // the column before.
        let mut copies = vec![vec![0u8; 72 * 64]; (*threads).min(jobs.len())];
        let done = on_workers(pool, jobs.into_iter(), &mut copies, |job, copy| match job {
            Finish::Tapes(tapes) => Some(tape::code(tapes, table, segmentation)),
            Finish::Plane(columns) => {
                filter_plane(g, &masks, columns, copy);
                None
            }
        });
        done.into_iter().flatten().collect()
    }

    /// Mark each 8×8 of the frame still where `picture` is the previous
    /// source over the part of it the picture holds, or where the frame may
    /// not code it, and unchanged where it is still and was last coded at
    /// the dial's quantizer; and make the previous source `picture` where
    /// the frame may code it, in a band of 8×8 rows on each thread. With
    /// `everything`, every 8×8 is changed.
    fn compare(&mut self, picture: &Picture, everything: bool) {
        let g = &self.geometry;
        let width = usize::from(g.width);
        let (planes, stride) = (picture.planes(), picture.strides()[0]);
        let q = quality_to_q(self.quality);
        let band = g.mi_rows.div_ceil(self.threads);
        let [p0, p1, p2] = &mut self.previous;
        let marks = self.still.chunks_mut(band * g.mi_cols).zip(self.unchanged.chunks_mut(band * g.mi_cols)).zip(self.named.chunks(band * g.mi_cols)).zip(self.coded_at.chunks(band * g.mi_cols));
        let bands = marks.zip(p0.chunks_mut(band * 8 * width)).zip(p1.chunks_mut(band * 8 * width)).zip(p2.chunks_mut(band * 8 * width));
        on_threads(&self.pool, self.at_once, bands.enumerate(), |(b, ((((((still, unchanged), named), coded_at), p0), p1), p2))| {
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
                still.fill(false);
                unchanged.fill(false);
                return;
            }
            let marks = still.chunks_exact_mut(g.mi_cols).zip(unchanged.chunks_exact_mut(g.mi_cols)).zip(named.chunks_exact(g.mi_cols)).zip(coded_at.chunks_exact(g.mi_cols));
            for (r, (((still, unchanged), named), coded_at)) in marks.enumerate() {
                let rows = r * 8..(r * 8 + 8).min(rows);
                still.fill(true);
                unchanged.fill(true);
                if !named.contains(&true) {
                    continue;
                }
                // Eight pixels at a time, each run of them an 8×8's row.
                for (plane, previous) in previous.iter().enumerate() {
                    for y in rows.clone() {
                        let (old, old_rest) = previous[y * width..][..width].as_chunks::<8>();
                        let (new, new_rest) = source(plane, y).as_chunks::<8>();
                        for (same, (a, b)) in still.iter_mut().zip(old.iter().zip(new)) {
                            *same &= u64::from_ne_bytes(*a) == u64::from_ne_bytes(*b);
                        }
                        if !old_rest.is_empty() {
                            still[g.mi_cols - 1] &= old_rest == new_rest;
                        }
                    }
                }
                for (((same, still), named), &at) in unchanged.iter_mut().zip(still.iter_mut()).zip(named).zip(coded_at) {
                    *still = !named || *still;
                    *same = !named || *still && quality_to_q(at) == q;
                }
                // Where the source is still it is the previous source
                // already, so a row with a change is taken whole where the
                // frame may code it: elsewhere the previous source is what
                // was coded.
                if !still.contains(&false) {
                    continue;
                }
                let mut c = 0;
                while let Some(start) = named[c..].iter().position(|&n| n).map(|i| c + i) {
                    c = named[start..].iter().position(|&n| !n).map_or(g.mi_cols, |i| start + i);
                    let span = start * 8..(c * 8).min(width);
                    for (plane, previous) in previous.iter_mut().enumerate() {
                        for y in rows.clone() {
                            previous[y * width..][span.clone()].copy_from_slice(&source(plane, y)[span.clone()]);
                        }
                    }
                }
            }
        });
    }

    /// The last frame coded, as a decoder has it: the planes, tight at the
    /// picture's width.
    #[cfg(test)]
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
    use crate::decoder::{Decoded, Decoder};
    use crate::reference::Libvpx;

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
        let mut picture = Picture::new(w, h, 1).expect("a picture");
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
    fn mse(picture: &Picture, decoded: &Decoded<'_>) -> f64 {
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
            self.told(picture, keyframe, None)
        }

        /// `frame`, told where the picture changed.
        fn told(&mut self, picture: &Picture, keyframe: bool, changed: Option<&[Rect]>) -> (usize, bool, f64) {
            let mut frame = Vec::new();
            let keyframe = self.encoder.encode(picture, keyframe, changed, &mut frame).expect("a frame");
            self.decoded(picture, frame, keyframe)
        }

        /// `frame`, as a settle at `quality`: the whole picture at that
        /// dial, and the encoder's left where it was.
        fn settle(&mut self, picture: &Picture, quality: u8, keyframe: bool) -> (usize, bool, f64) {
            let mut frame = Vec::new();
            let keyframe = self.encoder.settle(picture, quality, keyframe, &mut frame).expect("a settle");
            self.decoded(picture, frame, keyframe)
        }

        /// Decode `frame`, which the encoder made of `picture`, and hold it
        /// to the encoder's reconstruction.
        fn decoded(&mut self, picture: &Picture, frame: Vec<u8>, keyframe: bool) -> (usize, bool, f64) {
            let (w, h) = picture.size();
            let quality = self.encoder.quality();
            let n = self.frames;
            self.frames += 1;
            assert_eq!(crate::frame_header(&frame), Some(crate::FrameHeader { profile: 1, keyframe }));
            let decoded = self.decoder.decode(&frame).unwrap_or_else(|e| panic!("{w}x{h} at quality {quality}, frame {n}: {e}"));
            assert_eq!(decoded.size(), (u32::from(w), u32::from(h)));
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
        let mut flat = Picture::new(w, h, 1).expect("a picture");
        flat.read_rgb(&[40, 90, 200].repeat(usize::from(w) * usize::from(h))).expect("its own picture");
        let (bytes, mse) = round_trip(&flat, 100, 1);
        // Sixty superblocks of a partition symbol, a skip flag and two modes
        // each, and the first one's residual; split to 8×8 it is thousands
        // of blocks.
        assert!(bytes < 120, "{bytes} bytes");
        assert!(mse < 1.0, "squared error {mse}");
    }

    /// The loop filter is left out in four tile columns, and every frame's
    /// header says so.
    #[test]
    fn the_loop_filter_is_left_out_of_four_tile_columns() {
        // Past 8192 wide the format asks for four columns on one thread too,
        // and it is the columns a frame is in that the filter follows.
        assert_eq!(Encoder::new(8256, 64, 50, 1).expect("an encoder").tile_cols_log2, 2);
        for (w, threads, filtered) in [(2047, 4, true), (2048, 4, false), (2048, 2, true), (8256, 1, false)] {
            let rgb = desktop_rgb(w, 64);
            let mut stream = Stream::new(w, 64, 50, threads);
            assert_eq!(stream.encoder.filtered, filtered);
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
            let mut stream = Stream::new(w, h, 90, threads);
            assert_eq!(stream.encoder.tile_cols_log2, 1);
            frames.iter().map(|rgb| {
                stream.frame(&picture(w, h, rgb), false);
                stream.last.clone()
            }).collect::<Vec<_>>()
        });
        assert!(one == two && one == three, "{} frames on one thread, {} on two, {} on three", one.len(), two.len(), three.len());
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

    /// A `w`×`h` in RGB of quarters that one intra mode each predicts best:
    /// stripes down, which V predicts; stripes across, H; a hatch two
    /// pixels wide down and to the right, D135, or with `k` odd down and
    /// to the left, D45 (a ramp that way TM predicts too, and anything
    /// coarser makes an 8×8's quarters differ, which codes it as 4×4s in
    /// DC, V or H); and a plane sloping both ways, TM. Each `k` makes
    /// others, none of them in another's picture, nor the hatch at its
    /// previous one's slant.
    fn lines_rgb(w: u16, h: u16, k: usize) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(usize::from(w) * usize::from(h) * 3);
        for y in 0..usize::from(h) {
            for x in 0..usize::from(w) {
                let v = match (x * 4 / usize::from(w) + k) % 4 {
                    0 => (x * (37 + 6 * k)) % 251,
                    1 => (y * (53 + 10 * k)) % 241,
                    2 => 100 + 10 * k + ((if k.is_multiple_of(2) { x + usize::from(h) - y } else { x + y }) % 4 / 2) * (60 + 10 * k),
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
        // At `Fastest` the 8×8s are not coded each way, and so take DC, V,
        // H and TM alone. In an inter frame a vector past the picture's
        // edge predicts a flat block, which ties DC's residual and is
        // taken before any intra mode is tried; at 72 high every block
        // can reach past the edge, so the hatch is coded that way there
        // but for an 8×8 of the bottom row, and that frame is held to no
        // diagonal: the taller picture's inter frame is the one that must
        // code them.
        for (w, h, threads, speed) in [(203, 141, 1, Speed::Usual), (2048, 72, 4, Speed::Usual), (2048, 72, 4, Speed::Fastest)] {
            let mut stream = Stream::new(w, h, 90, threads);
            stream.encoder.set_speed(speed);
            assert_eq!(stream.encoder.speed(), speed);
            for (i, keyframe) in [true, false].into_iter().enumerate() {
                stream.frame(&picture(w, h, &lines_rgb(w, h, i)), keyframe);
                let mut used = [0; 10];
                for contexts in &stream.encoder.contexts {
                    for mode in contexts.intra_modes() {
                        used[mode as usize] += 1;
                    }
                }
                let diagonal: usize = used[3..9].iter().sum();
                assert!(used[..3].iter().chain(&used[9..]).all(|&n| n > 0), "{w}x{h}, frame {i}: DC, V, H and TM coded {used:?} times");
                match speed {
                    Speed::Fastest => assert_eq!(diagonal, 0, "{w}x{h}, frame {i} at {speed:?}: the diagonal modes coded {used:?} times"),
                    Speed::Usual if keyframe || h > 128 => assert!(diagonal > 0, "{w}x{h}, frame {i} at {speed:?}: the diagonal modes coded {used:?} times"),
                    Speed::Usual => {}
                }
            }
        }
    }

    #[test]
    fn an_unchanged_picture_costs_a_few_bytes() {
        let (w, h) = (640, 384);
        let mut stream = Stream::new(w, h, 100, 1);
        let (_, _, first) = stream.frame(&desktop(w, h), false);
        let (bytes, keyframe, again) = stream.frame(&desktop(w, h), false);
        assert!(!keyframe);
        // The headers, the inactive segment's features, and sixty
        // superblocks each a partition and a segment, at a probability
        // that makes the sixty a few bits.
        assert!(bytes < 32, "{bytes} bytes");
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
        // not coded again and not filtered again: the superblock's own left
        // and top edges reach eight pixels into its neighbours, and its
        // right and bottom edges are its inactive neighbours', unfiltered.
        let after = stream.encoder.reconstruction();
        let stride = usize::from(w);
        for (p, (a, b)) in after.iter().zip(&before).enumerate() {
            for y in 0..usize::from(h) {
                for x in 0..usize::from(w) {
                    let touched = (56..128).contains(&x) && (56..128).contains(&y);
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

    /// A frame told where its picture changed codes those 8×8s and no
    /// others: a change it was not told of stays out of the decoder's
    /// picture until a frame names it. A frame told nothing codes whatever
    /// changed, and a rectangle past the picture's edge is no error.
    #[test]
    fn a_frame_told_where_it_changed_codes_that_and_nothing_else() {
        // Neither side a multiple of 8.
        let (w, h) = (333u16, 250u16);
        let wu = usize::from(w);
        const GREY: [u8; 3] = [200, 200, 200];
        let square = |x: u16, y: u16| Rect { x, y, width: 32, height: 32 };
        let (first, second, third) = (square(32, 32), square(200, 150), square(96, 200));
        let paint = |rgb: &mut [u8], rect: Rect, colour: [u8; 3]| {
            for y in usize::from(rect.y)..usize::from(rect.y + rect.height) {
                for x in usize::from(rect.x)..usize::from(rect.x + rect.width) {
                    rgb[(y * wu + x) * 3..][..3].copy_from_slice(&colour);
                }
            }
        };
        let mut stream = Stream::new(w, h, 90, 2);
        let mut rgb = GREY.repeat(wu * usize::from(h));
        stream.frame(&picture(w, h, &rgb), false);
        // What the decoder should hold at the middle of each square.
        let mut step = |rgb: &[u8], changed: Option<&[Rect]>, want: [(Rect, [u8; 3]); 3]| {
            let (bytes, keyframe, _) = stream.told(&picture(w, h, rgb), false, changed);
            assert!(!keyframe, "told {changed:?}: a keyframe");
            let recon = stream.encoder.reconstruction();
            for (rect, colour) in want {
                let at = (usize::from(rect.y) + 16) * wu + usize::from(rect.x) + 16;
                let expected = picture(w, h, &colour.repeat(wu * usize::from(h)));
                for (p, (plane, expected)) in recon.iter().zip(expected.planes()).enumerate() {
                    assert!(plane[at].abs_diff(expected[at]) <= 4, "told {changed:?}: {rect:?}, plane {p}, is {} for {}", plane[at], expected[at]);
                }
            }
            bytes
        };

        // Two squares change and the frame is told of one.
        paint(&mut rgb, first, [220, 40, 40]);
        paint(&mut rgb, second, [40, 40, 220]);
        step(&rgb, Some(&[first]), [(first, [220, 40, 40]), (second, GREY), (third, GREY)]);
        // The other, which the frame before skipped, is coded when named.
        step(&rgb, Some(&[second]), [(first, [220, 40, 40]), (second, [40, 40, 220]), (third, GREY)]);
        // Told nothing, a frame codes whatever changed.
        paint(&mut rgb, third, [40, 180, 40]);
        step(&rgb, None, [(first, [220, 40, 40]), (second, [40, 40, 220]), (third, [40, 180, 40])]);
        // Rectangles are clipped to the picture.
        paint(&mut rgb, first, GREY);
        paint(&mut rgb, third, GREY);
        let edge = Rect { x: 320, y: 240, width: 100, height: 100 };
        let outside = Rect { x: 400, y: 300, width: 16, height: 16 };
        step(&rgb, Some(&[first, edge, outside]), [(first, GREY), (second, [40, 40, 220]), (third, [40, 180, 40])]);
        // Nothing named is nothing coded.
        let bytes = step(&rgb, Some(&[]), [(first, GREY), (second, [40, 40, 220]), (third, [40, 180, 40])]);
        assert!(bytes < 40, "a frame told of no change took {bytes} bytes");
        step(&rgb, Some(&[third]), [(first, GREY), (second, [40, 40, 220]), (third, GREY)]);
    }

    /// Frames told where the picture changed, in two tile columns: each
    /// writes back what it touched and leaves the rest, through edits in
    /// each column, an edit over the column boundary, an edit where nothing
    /// was touched before, and then a dial change that codes everything,
    /// every frame decoding to the encoder's own picture.
    #[test]
    fn frames_told_where_they_changed_keep_one_picture_across_tile_columns() {
        let (w, h) = (1500u16, 200u16);
        let mut stream = Stream::new(w, h, 70, 4);
        assert_eq!(stream.encoder.tile_cols_log2, 1);
        let mut rgb = desktop_rgb(w, h);
        stream.frame(&picture(w, h, &rgb), false);
        let edits = [(100, 20, 50, 30), (1200, 100, 60, 40), (700, 60, 120, 20), (1400, 150, 90, 45), (40, 150, 30, 30), (0, 0, 1500, 8)];
        for (i, &(x, y, rw, rh)) in edits.iter().enumerate() {
            rgb = edit(rgb, w, (x, y, rw, rh), 10 + i as u32);
            let rect = Rect { x: x as u16, y: y as u16, width: rw as u16, height: rh as u16 };
            let (bytes, keyframe, mse) = stream.told(&picture(w, h, &rgb), false, Some(&[rect]));
            assert!(!keyframe && mse < 30.0, "edit {i}: {bytes} bytes, squared error {mse}");
        }
        stream.encoder.set_quality(95);
        let (_, _, mse) = stream.frame(&picture(w, h, &rgb), false);
        assert!(mse < 4.0, "squared error {mse} after the dial moved");
        rgb = edit(rgb, w, (600, 100, 40, 40), 99);
        let (_, _, mse) = stream.told(&picture(w, h, &rgb), false, Some(&[Rect { x: 600, y: 100, width: 40, height: 40 }]));
        assert!(mse < 4.0, "squared error {mse}");
    }

    /// An 8×8 a frame skips keeps the quantizer it was last coded at, so a
    /// frame at a finer one that is told where the picture changed sharpens
    /// that and leaves the rest coarse, until every 8×8 has been coded at
    /// the finer one since; and `coarsest` says so.
    #[test]
    fn a_skipped_block_keeps_its_quantizer_until_a_whole_frame() {
        let (w, h) = (320u16, 240u16);
        let (wu, hu) = (usize::from(w), usize::from(h));
        let source = noise(w, h);
        let named = Rect { x: 0, y: 0, width: 64, height: 64 };
        // The error of everything the rectangle leaves out.
        let error = |recon: &[Vec<u8>; 3]| {
            let rest = (0..hu).flat_map(|y| (0..wu).map(move |x| (x, y))).filter(|&(x, y)| x >= 64 || y >= 64);
            let sum: u64 = rest.clone().map(|(x, y)| u64::from(recon[0][y * wu + x].abs_diff(source.planes()[0][y * wu + x]))).sum();
            sum as f64 / rest.count() as f64
        };

        let mut unused = Encoder::new(w, h, 20, 1).expect("an encoder");
        unused.set_quality(90);
        assert_eq!(unused.coarsest(), 90, "before a frame, the dial as it stands");
        let mut stream = Stream::new(w, h, QUALITY_MIN, 1);
        stream.frame(&source, false);
        let coarse = error(&stream.encoder.reconstruction());
        assert_eq!(stream.encoder.coarsest(), QUALITY_MIN);

        stream.encoder.set_quality(90);
        assert_eq!(stream.encoder.coarsest(), QUALITY_MIN, "a dial that moved has coded nothing yet");
        stream.told(&source, false, Some(&[named]));
        let skipped = error(&stream.encoder.reconstruction());
        assert!(skipped > coarse * 0.9, "a frame told of one corner left the rest at error {skipped:.2}, from {coarse:.2}: it coded 8×8s it was to skip");
        assert_eq!(stream.encoder.coarsest(), QUALITY_MIN, "the 8×8s a frame skipped are as coarse as they were");

        stream.frame(&source, false);
        let settled = error(&stream.encoder.reconstruction());
        assert!(settled < coarse / 4.0, "a whole frame at quality 90 left the picture at error {settled:.2}, from {coarse:.2}");
        assert_eq!(stream.encoder.coarsest(), 90);

        // Coarse where a frame coded and nowhere else, until frames told
        // where have between them coded every 8×8 finer: the corner
        // changes and is coded at 40, the rest is coded at 90, and then
        // the corner as it stands.
        let mut rgb = noise_rgb(w, h, 0x9e37_79b9);
        for i in (0..64).flat_map(|y| (0..64).map(move |x| (y * wu + x) * 3)) {
            for v in &mut rgb[i..i + 3] {
                *v = !*v;
            }
        }
        let changed = picture(w, h, &rgb);
        let corner = |recon: &[Vec<u8>; 3]| {
            let sum: u64 = (0..64).flat_map(|y| (0..64).map(move |x| (x, y))).map(|(x, y)| u64::from(recon[0][y * wu + x].abs_diff(changed.planes()[0][y * wu + x]))).sum();
            sum as f64 / (64.0 * 64.0)
        };
        stream.encoder.set_quality(40);
        stream.told(&changed, false, Some(&[named]));
        assert_eq!(stream.encoder.coarsest(), 40);
        let coarse = corner(&stream.encoder.reconstruction());
        stream.encoder.set_quality(90);
        stream.told(&changed, false, Some(&[Rect { x: 64, y: 0, width: 256, height: 240 }, Rect { x: 0, y: 64, width: 64, height: 176 }]));
        assert_eq!(stream.encoder.coarsest(), 40, "the corner coded at 40 was not in that frame");
        stream.told(&changed, false, Some(&[Rect { x: 4, y: 4, width: 56, height: 56 }]));
        assert_eq!(stream.encoder.coarsest(), 90, "every 8×8 has been coded at 90 since");
        let settled = corner(&stream.encoder.reconstruction());
        assert!(settled < coarse / 2.0, "the corner, coded at 40, was left at error {settled:.2} by the frame at 90, from {coarse:.2}");
    }

    /// A settle codes nothing where a decoder holds the picture exactly
    /// already, however coarse the dial that left it so: a flat picture
    /// settles in the bytes of its segment map, and one flat but for a
    /// patch of noise in the patch's, the flat part left exact. On one tile
    /// column and on four.
    #[test]
    fn a_settle_leaves_what_a_decoder_holds_exactly() {
        // The second is no whole number of 8×8s either way: its last
        // superblocks are held exactly as far as the picture holds them.
        for (w, h, threads) in [(320u16, 240u16, 1), (322, 243, 2), (2048, 136, 4)] {
            // A grey the keyframe's DC lands on at this dial, looked for
            // since which one it is depends on the conversion in front: the
            // NEON one makes a luma of 217 of the 235 its AVX2 makes 218 of.
            let exact = |flat: &Vec<u8>| Stream::new(w, h, 40, threads).frame(&picture(w, h, flat), true).2 == 0.0;
            let flat = (0..=255u8).rev().map(|grey| vec![grey; usize::from(w) * usize::from(h) * 3]).find(exact).expect("a grey a keyframe at this dial holds exactly");
            let mut stream = Stream::new(w, h, 40, threads);
            stream.frame(&picture(w, h, &flat), true);
            let (bytes, keyframe, mse) = stream.settle(&picture(w, h, &flat), 90, false);
            assert!(!keyframe && mse == 0.0);
            assert!(bytes < 100, "{w}x{h}: a settle of a picture held exactly took {bytes} bytes");
            assert_eq!(stream.encoder.coarsest(), 90);

            let patched = picture(w, h, &edit(flat.clone(), w, (64, 64, 64, 64), 7));
            let mut stream = Stream::new(w, h, 40, threads);
            let (_, _, coarse) = stream.frame(&patched, true);
            let (bytes, _, settled) = stream.settle(&patched, 90, false);
            assert!(settled < coarse / 4.0, "{w}x{h}: the settle left the patch at error {settled:.2} of {coarse:.2}");
            let (whole, _, _) = Stream::new(w, h, 90, threads).frame(&patched, true);
            assert!(bytes < whole, "{w}x{h}: a settle of one patch took {bytes} bytes, a keyframe {whole}");
            let recon = stream.encoder.reconstruction();
            let width = usize::from(w);
            for (plane, source) in recon.iter().zip(patched.planes()) {
                for y in 136..usize::from(h) {
                    assert_eq!(plane[y * width..][..width], source[y * patched.strides()[0]..][..width], "{w}x{h}: row {y}, flat and exact before the settle, is not after it");
                }
            }
        }
    }

    /// A settle is one frame: whole and at the quality asked whatever the
    /// frames before it were told, an inter frame unless a keyframe is
    /// asked, and with the dial back where the link had it for the frame
    /// after. A walk told what each frame left owes the settle until that
    /// one has gone. On one tile column and on four.
    #[test]
    fn a_settle_sharpens_a_coarse_picture_and_keeps_the_dial() {
        use crate::walk::{QualityWalk, SETTLE_IDLE};
        for (w, h, threads) in [(320u16, 240u16, 1), (2048, 72, 4)] {
            let source = noise(w, h);
            let planes = source.planes();
            let error = |recon: &[Vec<u8>; 3]| recon[0].iter().zip(planes[0]).map(|(a, b)| u64::from(a.abs_diff(*b))).sum::<u64>() as f64 / planes[0].len() as f64;
            let start = std::time::Instant::now();
            let mut walk = QualityWalk::new(90, std::time::Duration::from_millis(33), true);
            let mut stream = Stream::new(w, h, 90, threads);
            stream.encoder.set_quality(QUALITY_MIN);
            stream.frame(&source, false);
            walk.sent(stream.encoder.coarsest(), start);
            assert_eq!((stream.encoder.coarsest(), walk.settle_at()), (QUALITY_MIN, Some(start + SETTLE_IDLE)));
            let coarse = error(&stream.encoder.reconstruction());
            // Frames told where leave the rest as coarse as it was.
            let corner = Rect { x: 0, y: 0, width: 16, height: 16 };
            stream.told(&source, false, Some(&[corner]));

            walk.settle(start + SETTLE_IDLE);
            let (_, keyframe, _) = stream.settle(&source, walk.ceiling(), false);
            assert!(!keyframe, "{w}x{h}: the settle cost a keyframe");
            walk.sent(stream.encoder.coarsest(), start + SETTLE_IDLE);
            assert_eq!((stream.encoder.quality(), stream.encoder.coarsest(), walk.settle_at()), (QUALITY_MIN, 90, None), "{w}x{h}: the settle left the dial or the picture where it was not");
            let settled = error(&stream.encoder.reconstruction());
            assert!(settled < coarse / 4.0, "{w}x{h}: the settle left the picture at error {settled:.2} of {coarse:.2}");

            // The motion after it is at what the link bears, and where it
            // is: the rest keeps what the settle gave it.
            stream.told(&source, false, Some(&[corner]));
            walk.sent(stream.encoder.coarsest(), start + SETTLE_IDLE);
            assert_eq!((stream.encoder.coarsest(), walk.settle_at().is_some()), (QUALITY_MIN, true), "{w}x{h}: a coarse frame after the settle owes none");
            let after = error(&stream.encoder.reconstruction());
            assert!(after < coarse / 4.0, "{w}x{h}: a frame of one corner left the settled picture at error {after:.2} of {coarse:.2}");

            // A settle that is to be a keyframe is one, at the settle's quality.
            let (_, keyframe, _) = stream.settle(&source, 90, true);
            assert!(keyframe);
            assert_eq!((stream.encoder.quality(), stream.encoder.coarsest()), (QUALITY_MIN, 90));

            // A picture that is not the encoder's moves nothing.
            let other = Picture::new(w, h + 8, 1).expect("a picture");
            assert!(matches!(stream.encoder.settle(&other, 90, false, &mut Vec::new()), Err(Error::Mismatch(..))));
            assert_eq!(stream.encoder.quality(), QUALITY_MIN);
        }
    }

    #[test]
    fn keyframes_follow_each_other_through_one_decoder() {
        let (w, h) = (200, 120);
        let mut stream = Stream::new(w, h, 50, 1);
        for _ in 0..3 {
            assert!(stream.frame(&noise(w, h), true).1);
        }
    }

    fn psnr(mse: f64) -> f64 {
        10.0 * (255.0 * 255.0 / mse).log10()
    }

    /// One frame of a wlshare capture, what a session handed its stream:
    /// the file `crates/wlshare-rfb/src/capture.rs` writes there, read here
    /// from its documented layout.
    struct Captured<'a> {
        size: (u16, u16),
        quality: u8,
        keyframe: bool,
        changed: Option<&'a [Rect]>,
        /// The whole picture as it stands with this frame, `B, G, R, X`,
        /// rows tight at the width.
        pixels: &'a [u8],
    }

    /// The rows `rects` span of a picture `height` high, each once, merged
    /// and in order: the rows a frame of rectangles carries.
    fn spans(rects: &[Rect], height: u16) -> Vec<std::ops::Range<u16>> {
        let mut spans: Vec<std::ops::Range<u16>> = rects.iter().map(|r| r.y.min(height)..r.y.saturating_add(r.height).min(height)).filter(|s| !s.is_empty()).collect();
        spans.sort_by_key(|s| s.start);
        let mut merged: Vec<std::ops::Range<u16>> = Vec::new();
        for span in spans {
            match merged.last_mut() {
                Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
                _ => merged.push(span),
            }
        }
        merged
    }

    /// Hand `f` each frame of the capture at `path`: a `.wlcap`, or one
    /// `zstd` compressed as `.wlcap.zst`, read through `zstd -dc`.
    fn each_captured(path: &std::path::Path, mut f: impl FnMut(&Captured<'_>) -> bool) {
        use std::io::Read as _;
        let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut zstd = None;
        let mut input: Box<dyn std::io::Read> = if path.extension().is_some_and(|e| e == "zst") {
            // Its complaint when the read stops short is nothing to print.
            let mut child = std::process::Command::new("zstd").args(["-dc", "-q"]).stdin(file).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().expect("zstd on the path");
            let out = child.stdout.take().expect("piped");
            zstd = Some(child);
            Box::new(std::io::BufReader::with_capacity(1 << 20, out))
        } else {
            Box::new(std::io::BufReader::with_capacity(1 << 20, file))
        };
        let mut magic = [0; 8];
        input.read_exact(&mut magic).expect("a capture's magic");
        assert_eq!(&magic, b"WLSHCAP1", "{}: not a wlshare capture", path.display());
        let (mut pixels, mut rects, mut size) = (Vec::new(), Vec::new(), (0u16, 0u16));
        let mut header = [0u8; 16];
        loop {
            match input.read_exact(&mut header) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => panic!("{}: {e}", path.display()),
            }
            let u16_at = |at: usize| u16::from_le_bytes([header[at], header[at + 1]]);
            let (w, h, quality, flags, count) = (u16_at(8), u16_at(10), header[12], header[13], u16_at(14));
            rects.clear();
            for _ in 0..count {
                let mut r = [0; 8];
                input.read_exact(&mut r).expect("a rectangle");
                let field = |at: usize| u16::from_le_bytes([r[at], r[at + 1]]);
                rects.push(Rect { x: field(0), y: field(2), width: field(4), height: field(6) });
            }
            let row = usize::from(w) * 4;
            let spans = if flags & 2 == 0 {
                if size != (w, h) {
                    size = (w, h);
                    pixels.clear();
                    pixels.resize(row * usize::from(h), 0);
                }
                std::iter::once(0..h).collect()
            } else {
                assert_eq!(size, (w, h), "rectangles of a picture no frame before holds");
                spans(&rects, h)
            };
            for span in spans {
                input.read_exact(&mut pixels[usize::from(span.start) * row..usize::from(span.end) * row]).expect("a whole frame");
            }
            if !f(&Captured { size, quality, keyframe: flags & 1 != 0, changed: (flags & 2 != 0).then_some(&rects[..]), pixels: &pixels }) {
                break;
            }
        }
        drop(input);
        if let Some(mut child) = zstd {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// The mean squared error of a decode's luma against `picture`'s.
    fn luma_mse(picture: &Picture, decoded: &Decoded<'_>) -> f64 {
        let (w, h) = picture.size();
        let (w, h) = (usize::from(w), usize::from(h));
        let (source, plane, stride) = (picture.planes()[0], decoded.planes()[0], decoded.strides()[0]);
        let mut sum = 0.0;
        for y in 0..h {
            for (a, b) in source[y * w..][..w].iter().zip(&plane[y * stride..][..w]) {
                sum += (f64::from(*a) - f64::from(*b)).powi(2);
            }
        }
        sum / (w * h) as f64
    }

    /// What a captured frame was, the kinds measured apart.
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Kind {
        /// A keyframe: asked, or the first at a size.
        Key,
        /// The whole picture as an inter frame: a settle, or a frame its
        /// session knew no damage for.
        Whole,
        /// Rectangles within a superblock's area.
        Small,
        /// Rectangles within a sixteenth of the picture.
        Mid,
        /// Rectangles over more.
        Large,
    }

    impl Kind {
        fn name(self) -> &'static str {
            match self {
                Kind::Key => "key",
                Kind::Whole => "whole",
                Kind::Small => "small",
                Kind::Mid => "mid",
                Kind::Large => "large",
            }
        }
    }

    /// Both encoders' bytes, times and luma PSNRs over the frames of one
    /// kind, libvpx's first, and the ratio of the two times frame by frame.
    #[derive(Default)]
    struct Tally {
        frames: usize,
        bytes: [usize; 2],
        times: [Vec<f64>; 2],
        psnr: [f64; 2],
        ratios: Vec<f64>,
    }

    fn median(v: &mut [f64]) -> f64 {
        if v.is_empty() {
            return f64::NAN;
        }
        v.sort_unstable_by(f64::total_cmp);
        v[v.len() / 2]
    }

    /// `frames`, each a VP9 frame of a `width`×`height` stream, as an IVF
    /// file at `path`, for another decoder to read.
    fn write_ivf(path: &std::path::Path, (width, height): (u16, u16), frames: &[Vec<u8>]) {
        let mut ivf = Vec::new();
        ivf.extend(b"DKIF");
        ivf.extend(0u16.to_le_bytes());
        ivf.extend(32u16.to_le_bytes());
        ivf.extend(b"VP90");
        ivf.extend(width.to_le_bytes());
        ivf.extend(height.to_le_bytes());
        ivf.extend(30u32.to_le_bytes());
        ivf.extend(1u32.to_le_bytes());
        ivf.extend((frames.len() as u32).to_le_bytes());
        ivf.extend(0u32.to_le_bytes());
        for (i, frame) in frames.iter().enumerate() {
            ivf.extend((frame.len() as u32).to_le_bytes());
            ivf.extend((i as u64).to_le_bytes());
            ivf.extend(frame);
        }
        std::fs::write(path, ivf).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }

    /// Whether `ALONE` is set: the measurements then leave libvpx out, for
    /// a profiler to watch this encoder or for one build of it to be set
    /// against another.
    fn alone() -> bool {
        std::env::var_os("ALONE").is_some()
    }

    /// Play the capture at `path` into this encoder and into libvpx as the
    /// session played it into its stream: the same rectangles, the same
    /// dial, a keyframe where one was asked, on `threads`, which of the two
    /// goes first alternating frame by frame so that the machine's load
    /// lands on both, every frame of each through a libvpx decoder of its
    /// own. Per kind of frame, what each made of it.
    fn play(path: &std::path::Path, threads: usize) -> (std::collections::BTreeMap<Kind, Tally>, (u16, u16), usize) {
        let against = !alone();
        let mut stream: Option<(Libvpx, Encoder, [Decoder; 2], Picture)> = None;
        let mut tallies: std::collections::BTreeMap<Kind, Tally> = std::collections::BTreeMap::new();
        let (mut n, mut size, mut libvpx_carried) = (0usize, (0u16, 0u16), false);
        // With `IVF` set to a directory, this encoder's stream is kept and
        // written there as `<capture>-<WxH>.ivf`, a file per size.
        let ivf = std::env::var_os("IVF").map(std::path::PathBuf::from);
        let frames = std::env::var_os("FRAMES").is_some();
        let fastest = std::env::var("SPEED").is_ok_and(|s| s == "fastest");
        type Kept = ((u16, u16), Vec<Vec<u8>>);
        let mut kept: Vec<Kept> = Vec::new();
        each_captured(path, |frame| {
            let (w, h) = frame.size;
            let fresh = size != (w, h);
            if fresh {
                size = (w, h);
                let (mut libvpx, mut rust) = (Libvpx::new(w, h, frame.quality, threads), Encoder::new(w, h, frame.quality, threads).expect("an encoder"));
                if fastest {
                    libvpx.set_speed(Speed::Fastest);
                    rust.set_speed(Speed::Fastest);
                }
                stream = Some((libvpx, rust, [Decoder::new(1).expect("a decoder"), Decoder::new(1).expect("a decoder")], Picture::new(w, h, threads).expect("a picture")));
                libvpx_carried = false;
            }
            let (libvpx, rust, decoders, picture) = stream.as_mut().expect("made above");
            let stride = usize::from(w) * 4;
            match frame.changed {
                Some(rects) => {
                    for r in rects {
                        picture.read_bgrx_rect(frame.pixels, stride, *r).expect("a rectangle of the picture");
                    }
                }
                None => picture.read_bgrx(frame.pixels, stride).expect("the picture"),
            }
            if libvpx.quality() != frame.quality {
                libvpx.set_quality(frame.quality);
            }
            if rust.quality() != frame.quality {
                rust.set_quality(frame.quality);
            }
            let kind = match frame.changed {
                _ if frame.keyframe || fresh => Kind::Key,
                None => Kind::Whole,
                Some(rects) => {
                    let area: usize = rects.iter().map(|r| usize::from(r.width.min(w.saturating_sub(r.x))) * usize::from(r.height.min(h.saturating_sub(r.y)))).sum();
                    if area <= 64 * 64 {
                        Kind::Small
                    } else if area * 16 <= usize::from(w) * usize::from(h) {
                        Kind::Mid
                    } else {
                        Kind::Large
                    }
                }
            };
            let mut out = [Vec::new(), Vec::new()];
            let mut times = [0.0; 2];
            let mut produced = true;
            for c in if n % 2 == 0 { [0, 1] } else { [1, 0] } {
                if c == 0 && !against {
                    continue;
                }
                let started = std::time::Instant::now();
                if c == 0 {
                    // libvpx is handed the rectangles as its stream would
                    // be: not after a frame it produced nothing for.
                    produced = libvpx.encode(picture, frame.keyframe, frame.changed.filter(|_| libvpx_carried), &mut out[0]).is_some();
                } else {
                    rust.encode(picture, frame.keyframe, frame.changed, &mut out[1]).expect("a frame");
                }
                times[c] = started.elapsed().as_secs_f64() * 1000.0;
            }
            libvpx_carried = produced;
            let tally = tallies.entry(kind).or_default();
            tally.frames += 1;
            let mut quality = [0.0; 2];
            for c in 0..2 {
                if c == 0 && (!against || !produced) {
                    continue;
                }
                let decoded = decoders[c].decode(&out[c]).expect("a frame libvpx decodes");
                tally.bytes[c] += out[c].len();
                tally.times[c].push(times[c]);
                quality[c] = psnr(luma_mse(picture, &decoded).max(1e-3));
                tally.psnr[c] += quality[c];
            }
            if against && produced {
                tally.ratios.push(times[1] / times[0]);
            }
            // With `FRAMES` set, each frame on a line of its own: its
            // number, dial and kind, and both encoders' bytes, time and PSNR.
            if frames {
                println!(
                    "    #{n:<4} q{:<3} {:<5} | libvpx {:>8} B {:>6.2} ms {:>5.2} dB | rust {:>8} B {:>6.2} ms {:>5.2} dB",
                    frame.quality,
                    kind.name(),
                    out[0].len(),
                    times[0],
                    quality[0],
                    out[1].len(),
                    times[1],
                    quality[1]
                );
            }
            if ivf.is_some() {
                if fresh {
                    kept.push(((w, h), Vec::new()));
                }
                kept.last_mut().expect("pushed above").1.push(std::mem::take(&mut out[1]));
            }
            n += 1;
            true
        });
        if let Some(dir) = ivf {
            for ((w, h), stream) in &kept {
                write_ivf(&dir.join(format!("{}-{w}x{h}.ivf", capture_name(path))), (*w, *h), stream);
            }
        }
        (tallies, size, n)
    }

    /// The captures in the directory `CAPTURES` names (`tmp/captures` if
    /// unset), in name order.
    fn captures() -> Vec<std::path::PathBuf> {
        let dir = std::env::var_os("CAPTURES").map_or_else(|| std::path::PathBuf::from("tmp/captures"), std::path::PathBuf::from);
        let only = std::env::var("ONLY").ok();
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|entry| entry.expect("an entry").path())
            .filter(|p| p.to_str().is_some_and(|s| s.ends_with(".wlcap") || s.ends_with(".wlcap.zst")))
            .filter(|p| only.as_ref().is_none_or(|only| p.to_str().is_some_and(|s| s.contains(only.as_str()))))
            .collect();
        paths.sort();
        assert!(!paths.is_empty(), "no captures in {}", dir.display());
        paths
    }

    fn capture_name(path: &std::path::Path) -> String {
        path.file_name().expect("a file").to_string_lossy().trim_end_matches(".zst").trim_end_matches(".wlcap").to_owned()
    }

    /// The first picture of each capture: what the capture's first frame
    /// holds.
    fn first_picture(path: &std::path::Path) -> Picture {
        let mut picture = None;
        each_captured(path, |frame| {
            let (w, h) = frame.size;
            let mut p = Picture::new(w, h, 1).expect("a picture");
            p.read_bgrx(frame.pixels, usize::from(w) * 4).expect("the picture");
            picture = Some(p);
            false
        });
        picture.expect("a frame in the capture")
    }

    /// One frame's bytes, least time and PSNR, luma's and over all three
    /// planes, from each encoder, libvpx's first, on a line.
    fn pair_line(label: &str, r: &[(usize, f64, f64, f64); 2]) {
        println!(
            "  {label:<8} | libvpx {:>8} B {:>6.2} ms {:>5.2} dB | rust {:>8} B {:>6.2} ms {:>5.2} dB | bytes {:.3} time {:.3} dB {:+.2} all {:+.2}",
            r[0].0, r[0].1, r[0].2, r[1].0, r[1].1, r[1].2,
            r[1].0 as f64 / r[0].0.max(1) as f64, r[1].1 / r[0].1, r[1].2 - r[0].2, r[1].3 - r[0].3,
        );
    }

    /// The first picture of each capture (the same knobs as
    /// `captures_against_libvpx`) coded as a keyframe by both encoders at
    /// the dials `DIALS` names (`40,65,90` if unset), each `PASSES` times
    /// (5 if unset) turn about for the least time: bytes, time and PSNR per
    /// dial, luma's and over all three planes. `SPEED=fastest` codes both
    /// at their fastest. With `IVF` set to a directory, each keyframe is
    /// written there as `<capture>-q<dial>-<encoder>.ivf`. Run it in release:
    ///
    /// ```text
    /// ONLY=1920 cargo test --release keyframes_against_libvpx -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads the captures in tmp/captures/ and wants a release build"]
    fn keyframes_against_libvpx() {
        let threads: usize = std::env::var("THREADS").map_or(4, |t| t.parse().expect("a thread count"));
        let dials: Vec<u8> = std::env::var("DIALS").map_or_else(|_| vec![40, 65, 90], |d| d.split(',').map(|v| v.parse().expect("a dial")).collect());
        let passes: usize = std::env::var("PASSES").map_or(5, |t| t.parse().expect("a pass count"));
        let fastest = std::env::var("SPEED").is_ok_and(|s| s == "fastest");
        for path in captures() {
            let picture = first_picture(&path);
            let (w, h) = picture.size();
            println!("{} {w}x{h} on {threads}", capture_name(&path));
            for &dial in &dials {
                let mut libvpx = Libvpx::new(w, h, dial, threads);
                let mut rust = Encoder::new(w, h, dial, threads).expect("an encoder");
                if fastest {
                    libvpx.set_speed(Speed::Fastest);
                    rust.set_speed(Speed::Fastest);
                }
                let mut result = [(0usize, f64::INFINITY, 0.0f64, 0.0f64); 2];
                let mut frames = [Vec::new(), Vec::new()];
                for _ in 0..passes {
                    for c in 0..2 {
                        if c == 0 && alone() {
                            continue;
                        }
                        frames[c].clear();
                        let started = std::time::Instant::now();
                        if c == 0 {
                            libvpx.encode(&picture, true, None, &mut frames[0]).expect("a frame");
                        } else {
                            rust.encode(&picture, true, None, &mut frames[1]).expect("a keyframe");
                        }
                        result[c].1 = result[c].1.min(started.elapsed().as_secs_f64() * 1000.0);
                    }
                }
                for c in 0..2 {
                    if c == 0 && alone() {
                        continue;
                    }
                    let mut decoder = Decoder::new(1).expect("a decoder");
                    let decoded = decoder.decode(&frames[c]).expect("a frame libvpx decodes");
                    result[c].0 = frames[c].len();
                    result[c].2 = psnr(luma_mse(&picture, &decoded).max(1e-3));
                    result[c].3 = psnr(mse(&picture, &decoded).max(1e-3));
                    if let Some(dir) = std::env::var_os("IVF") {
                        write_ivf(&std::path::Path::new(&dir).join(format!("{}-q{dial}-{}.ivf", capture_name(&path), ["libvpx", "rust"][c])), (w, h), std::slice::from_ref(&frames[c]));
                    }
                }
                pair_line(&format!("q{dial}"), &result);
            }
        }
    }

    /// The first picture of each capture (the same knobs as
    /// `captures_against_libvpx`) coded as a keyframe at the first dial of
    /// each pair `SETTLES` names (`20:90,65:90,80:90,89:90` if unset), and
    /// then settled at the second, the picture unchanged, as a caller
    /// sharpens a quiet picture after the dial walked: this encoder's
    /// `settle`, and libvpx's whole frame at the second dial, which is what
    /// `screen-vp9`'s settle is. Each is coded `PASSES` times (5 if unset)
    /// from the same keyframe, turn about, for the least time; its bytes,
    /// time and PSNR per pair. Run it in release:
    ///
    /// ```text
    /// ONLY=1920 cargo test --release settles_against_libvpx -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads the captures in tmp/captures/ and wants a release build"]
    fn settles_against_libvpx() {
        let threads: usize = std::env::var("THREADS").map_or(4, |t| t.parse().expect("a thread count"));
        let pairs: Vec<(u8, u8)> = std::env::var("SETTLES").map_or_else(
            |_| vec![(20, 90), (65, 90), (80, 90), (89, 90)],
            |d| d.split(',').map(|p| p.split_once(':').map(|(a, b)| (a.parse().expect("a dial"), b.parse().expect("a dial"))).expect("from:to")).collect(),
        );
        let passes: usize = std::env::var("PASSES").map_or(5, |t| t.parse().expect("a pass count"));
        for path in captures() {
            let picture = first_picture(&path);
            let (w, h) = picture.size();
            println!("{} {w}x{h} on {threads}", capture_name(&path));
            for &(from, to) in &pairs {
                let mut result = [(0usize, f64::INFINITY, 0.0f64, 0.0f64); 2];
                for _ in 0..passes {
                    for (c, r) in result.iter_mut().enumerate() {
                        if c == 0 && alone() {
                            continue;
                        }
                        let mut frames = [Vec::new(), Vec::new()];
                        let time = if c == 0 {
                            let mut libvpx = Libvpx::new(w, h, from, threads);
                            libvpx.encode(&picture, true, None, &mut frames[0]).expect("a frame");
                            libvpx.set_quality(to);
                            let started = std::time::Instant::now();
                            libvpx.encode(&picture, false, None, &mut frames[1]).expect("a frame");
                            started.elapsed()
                        } else {
                            let mut rust = Encoder::new(w, h, from, threads).expect("an encoder");
                            rust.encode(&picture, true, None, &mut frames[0]).expect("a keyframe");
                            let started = std::time::Instant::now();
                            rust.settle(&picture, to, false, &mut frames[1]).expect("a settle");
                            started.elapsed()
                        };
                        r.1 = r.1.min(time.as_secs_f64() * 1000.0);
                        let mut decoder = Decoder::new(1).expect("a decoder");
                        decoder.decode(&frames[0]).expect("a keyframe libvpx decodes");
                        let decoded = decoder.decode(&frames[1]).expect("a settle libvpx decodes");
                        r.0 = frames[1].len();
                        r.2 = psnr(luma_mse(&picture, &decoded).max(1e-3));
                        r.3 = psnr(mse(&picture, &decoded).max(1e-3));
                    }
                }
                pair_line(&format!("q{from}:{to}"), &result);
            }
        }
    }

    /// Every frame of each wlshare capture in `tmp/captures/` (or the
    /// directory `CAPTURES` names; `ONLY` keeps the captures whose name
    /// holds it), played into this encoder and into libvpx as the session
    /// played it into its stream, on `THREADS` threads (4 if unset),
    /// `PASSES` times over (1 if unset). Per capture, for the frames of
    /// each kind — keyframes, whole inter frames, and rectangles within a
    /// superblock, within a sixteenth of the picture and over more — each
    /// encoder's bytes, median time and the luma PSNR of libvpx's decode,
    /// and the ratios `tools/tables.py` makes the tables of
    /// `docs/benchmarks.md` from. `ALONE` leaves libvpx out, `FRAMES`
    /// prints every frame, `SPEED=fastest` codes both at their fastest.
    /// With `IVF` set to a directory, this encoder's stream of each capture
    /// is written there too, for the browsers' own decoders to read through
    /// ffmpeg (`-c:v libvpx-vp9` is Chrome's, `-c:v vp9` Firefox's;
    /// `-f framemd5` of each should agree), one file per size the capture
    /// runs at, since ffmpeg rescales every frame after a size change to
    /// the first frame's size before hashing it. A measurement, not a
    /// check; run it in release:
    ///
    /// ```text
    /// ONLY=terminal cargo test --release captures_against_libvpx -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads the captures in tmp/captures/ and wants a release build"]
    fn captures_against_libvpx() {
        let threads: usize = std::env::var("THREADS").map_or(4, |t| t.parse().expect("a thread count"));
        let passes: usize = std::env::var("PASSES").map_or(1, |p| p.parse().expect("a pass count"));
        for path in captures() {
            for _ in 0..passes {
                let (mut tallies, size, n) = play(&path, threads);
                println!("{} {}x{} on {threads}, {n} frames", capture_name(&path), size.0, size.1);
                let mut all = Tally::default();
                let line = |name: &str, tally: &mut Tally| {
                    let made = [tally.times[0].len().max(1) as f64, tally.frames as f64];
                    println!(
                        "  {name:<5} {:>5} | libvpx {:>9} B {:>6.2} ms {:>5.2} dB | rust {:>9} B {:>6.2} ms {:>5.2} dB | bytes {:.3} time median {:.3} total {:.3}",
                        tally.frames,
                        tally.bytes[0],
                        median(&mut tally.times[0]),
                        tally.psnr[0] / made[0],
                        tally.bytes[1],
                        median(&mut tally.times[1]),
                        tally.psnr[1] / made[1],
                        tally.bytes[1] as f64 / tally.bytes[0].max(1) as f64,
                        median(&mut tally.ratios),
                        tally.times[1].iter().sum::<f64>() / tally.times[0].iter().sum::<f64>().max(1e-9),
                    );
                };
                for (kind, tally) in &mut tallies {
                    line(kind.name(), tally);
                    all.frames += tally.frames;
                    for c in 0..2 {
                        all.bytes[c] += tally.bytes[c];
                        all.times[c].extend_from_slice(&tally.times[c]);
                        all.psnr[c] += tally.psnr[c];
                    }
                    all.ratios.extend_from_slice(&tally.ratios);
                }
                line("all", &mut all);
            }
        }
    }

    #[test]
    fn a_picture_that_is_not_the_encoders_is_refused() {
        let mut encoder = Encoder::new(64, 64, 90, 1).expect("an encoder");
        let mut out = Vec::new();
        assert!(matches!(encoder.encode(&noise(64, 32), false, None, &mut out), Err(Error::Mismatch(..))));
        assert!(out.is_empty());
        assert!(matches!(Encoder::new(0, 64, 90, 1), Err(Error::Empty(..))));
        assert!(matches!(Encoder::new(64, 64, 90, 0), Err(Error::Threads(0))));
    }
}
