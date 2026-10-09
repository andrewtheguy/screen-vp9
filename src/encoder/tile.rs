//! One tile column of a frame: its superblocks in raster order, each a
//! partition tree down to the blocks that are coded, and each block's
//! reconstruction made as a decoder makes it.
//!
//! The tree is chosen from the source alone, as libvpx's real-time encoder
//! chooses a keyframe's (`choose_partitioning`): a square is coded whole when
//! its pixels vary little against the quantizer's step, and split when they
//! vary more, since a smaller block's DC prediction comes from nearer pixels
//! and its transform keeps an edge's energy to itself. In an inter frame a
//! square is measured against both of its predictions, the previous frame
//! at the vectors a decoder predicts for it and DC, and takes the one it
//! varies least from. A square whose source is what it was in the previous
//! frame is the previous frame's, skipped, without being measured; one that
//! is so only in part is split.
//!
//! A tile column is coded a superblock row at a time, the rows as a
//! wavefront: a row codes a superblock once the row above has coded the one
//! above it, which leaves it what a block below reads across its top edge
//! (libvpx's row-based multithreading). Rows are what threads take, so a
//! frame of one tile column is coded on as many of them as it has rows; the
//! bools of each row are kept apart and coded one row after another.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use super::bool_writer;
use super::convolve::{self, TAPS};
use super::motion;
use super::mv::{self, Mv};
use super::pool::Pool;
use super::predict::{self, Edges, IntraMode};
use super::probs::{skip_slot, tx_slot};
use super::quantize::Quantizer;
use super::tape::Tape;
use super::transform::TxType;
use super::tables::{
    INTER_MODE_PROBS, INTRA_INTER_PROBS, KF_PARTITION_PROBS, KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, MV_REF_BLOCKS, PARTITION_PROBS, SINGLE_REF_PROBS,
    UV_MODE_PROBS, Y_MODE_PROBS,
};
use super::{Column, Geometry, Reconstruction, loopfilter, pixels, tokens, transform};

/// A superblock's side in mode info units of 8×8, as log2.
const SB_LOG2: u32 = 3;

/// The variance per pixel, summed over the planes, above which a square is
/// split, as a fraction of the AC step: `1 / SPLIT_VARIANCE`. Measured on the
/// plan's captures, a keyframe's bytes and PSNR barely move between splitting
/// everything with any variance and keeping squares whole up to half a step,
/// and around an eighth was best; keeping every square whole cost up to a
/// third more bytes and 1–3 dB.
const SPLIT_VARIANCE: i64 = 8;

/// What a bit is worth in squared error when an inter block's transform size
/// is chosen, in thousandths of the AC step squared over 64, a step in
/// pixels being an eighth of one in coefficients. Of 0.07, 0.14 and 0.28 of
/// a step squared, measured on the plan's captures, 0.14 made the fewest
/// bytes on five of the six.
const RATE_WEIGHT: i64 = 140;

/// A vector the search offers is taken over the vectors a decoder predicts
/// only where its residual's squares are `1 / NEW_VECTOR_GAIN` less than
/// theirs.
const NEW_VECTOR_GAIN: i64 = 4;

/// An intra mode other than DC is taken only where its residual varies
/// `1 / INTRA_MODE_GAIN` less than DC's, for the bits it costs over DC's.
const INTRA_MODE_GAIN: i64 = 16;

/// What an intra neighbour adds to the mode context's counter (libvpx's
/// `mode_2_counter`); `InterMode::counter` gives the rest.
const INTRA_COUNT: usize = 9;

/// libvpx's `counter_to_context`; 9 marks a counter two neighbours cannot
/// add up to.
const COUNTER_TO_CONTEXT: [usize; 19] = [2, 3, 4, 1, 3, 9, 0, 9, 9, 5, 5, 9, 5, 9, 9, 9, 9, 9, 6];

/// The partition contexts a decoder keeps (libvpx's
/// `partition_context_lookup`): bit `n` of an entry is set when the block
/// there is narrower than `8 << n`.
fn partition_context_of(n: u32) -> u8 {
    (0xf << (n + 1)) & 0xf
}

/// What the ways of coding a block's vector cost, in 256ths of a bit, for a
/// block a decoder predicts `predicted`, `[nearest, near]`, for.
struct Prices {
    predicted: [Mv; 2],
    /// Per inter mode, its own bits.
    modes: [u32; 4],
}

impl Prices {
    /// At the inter mode probabilities `probs`.
    fn new(predicted: [Mv; 2], probs: &[u8; 3]) -> Self {
        Self { predicted, modes: [InterMode::Zero, InterMode::Nearest, InterMode::Near, InterMode::New].map(|m| m.cost(probs)) }
    }

    /// The cheapest way to code `mv`: as zero, as one of the vectors
    /// predicted, or as a new vector against the nearest; and its bits.
    fn of(&self, mv: Mv) -> (u32, InterMode) {
        let [nearest, near] = self.predicted;
        let mut best = None;
        for (mode, vector) in [(InterMode::Zero, Mv::ZERO), (InterMode::Nearest, nearest), (InterMode::Near, near)] {
            let cost = self.modes[mode as usize];
            if vector == mv && best.is_none_or(|(least, _)| cost < least) {
                best = Some((cost, mode));
            }
        }
        if mv != nearest {
            let cost = self.modes[InterMode::New as usize] + mv::cost(mv - nearest);
            if best.is_none_or(|(least, _)| cost < least) {
                best = Some((cost, InterMode::New));
            }
        }
        best.expect("a new vector when no other way fits")
    }
}

/// The best so far of a search between pixels: what it costs, the vector,
/// and its residual's squared deviation.
type Found = (i64, Mv, i64);

/// Try, by `cost`, the vector `row` and `col` eighths from `best`'s, which
/// it becomes if it costs less; and give back what it costs.
fn try_at(cost: &mut impl FnMut(Mv) -> (i64, i64), best: &mut Found, row: i32, col: i32) -> i64 {
    let v = Mv { row: best.1.row + row, col: best.1.col + col };
    let (c, deviation) = cost(v);
    if c < best.0 {
        *best = (c, v, deviation);
    }
    c
}

/// Try the vectors `step` eighths each way across and down from `best`'s,
/// and then the corner between the better of each (libvpx's
/// `FIRST_LEVEL_CHECKS`).
fn around(cost: &mut impl FnMut(Mv) -> (i64, i64), best: &mut Found, step: i32) {
    let centre = *best;
    let mut from = |row, col| {
        let mut there = (best.0, centre.1, 0);
        let c = try_at(cost, &mut there, row, col);
        if there.1 != centre.1 {
            *best = there;
        }
        c
    };
    let (left, right, up, down) = (from(0, -step), from(0, step), from(-step, 0), from(step, 0));
    from(if up < down { -step } else { step }, if left < right { -step } else { step });
}

/// `n / d` to the nearest whole number, halves away from zero (libvpx's
/// `divide_and_round`).
fn rounded(n: i64, d: i64) -> i64 {
    if (n < 0) != (d < 0) { (n - d / 2) / d } else { (n + d / 2) / d }
}

/// The squared error and the bits of a residual whose squared deviation is
/// `deviation` over `count` pixels, quantized at `step`, as a Laplacian
/// source quantized uniformly (Hang and Chen, 1997): from the closed forms
/// libvpx tables for `vp9_model_rd_from_var_lapndz`.
fn laplacian(deviation: f64, count: f64, step: f64) -> (f64, f64) {
    if deviation == 0.0 {
        return (0.0, 0.0);
    }
    let x = step / (deviation / count).sqrt();
    let r = (-std::f64::consts::SQRT_2 * x).exp();
    let entropy = |p: f64| if p <= 0.0 || p >= 1.0 { 0.0 } else { -(p * p.log2() + (1.0 - p) * (1.0 - p).log2()) };
    let bits = entropy(r.sqrt()) + r.sqrt() * (1.0 + entropy(r) / (1.0 - r));
    let y = x / std::f64::consts::SQRT_2;
    (deviation * (1.0 - y / y.sinh()), count * bits)
}

/// The picture being coded: three planes at its own size.
pub(super) struct Source<'a> {
    pub(super) planes: [&'a [u8]; 3],
    pub(super) stride: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl<'a> Source<'a> {
    /// The row `y` of `plane` as it is coded: past the picture's bottom, its
    /// last row.
    fn row(&self, plane: usize, y: usize) -> &'a [u8] {
        &self.planes[plane][y.min(self.height - 1) * self.stride..][..self.width]
    }
}

/// What an inter frame is coded against: the previous frame as a decoder
/// has it, per 8×8 of the frame whether the source there is the previous
/// frame's, and per superblock the vectors the motion search offers it.
#[derive(Clone, Copy)]
pub(super) struct Previous<'a> {
    pub(super) reference: &'a Reconstruction,
    pub(super) unchanged: &'a [bool],
    /// Per superblock of the frame, whether the frame touches it: it or a
    /// neighbour of it may be coded. One it does not touch is left as it
    /// is, in the reference, and not written to the reconstruction.
    pub(super) touched: &'a [bool],
    pub(super) vectors: &'a [[Mv; motion::VECTORS]],
    /// Of the blocks the search between pixels could refine, the one in
    /// this many it does.
    pub(super) search_every: usize,
}

/// How a block is coded.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Predicted from the pixels around it, and the residual.
    Intra(IntraMode),
    /// The previous frame at a vector from where the block is, and the
    /// residual against it.
    Inter(Mv),
    /// The previous frame where the block is, as it is: the source has not
    /// changed there, so nothing is coded but the skip.
    Unchanged,
}

/// How an inter block's vector is coded, in the order of the inter mode
/// tree's leaves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InterMode {
    /// The zero vector.
    Zero,
    /// The first vector a decoder predicts.
    Nearest,
    /// The second.
    Near,
    /// A vector of its own, coded against the first predicted.
    New,
}

impl InterMode {
    /// What a neighbour coded this way adds to the mode context's counter
    /// (libvpx's `mode_2_counter`).
    fn counter(self) -> usize {
        match self {
            InterMode::Nearest | InterMode::Near => 0,
            InterMode::Zero => 3,
            InterMode::New => 1,
        }
    }

    /// The bools of the inter mode tree down to this leaf: one per node
    /// passed, the last node's taken only when this is not the deepest leaf.
    fn bits(self) -> impl Iterator<Item = (bool, usize)> {
        let depth = self as usize;
        (0..(depth + 1).min(3)).map(move |node| (node < depth, node))
    }

    /// What coding this mode costs at the probabilities of a mode context,
    /// in 256ths of a bit.
    fn cost(self, probs: &[u8; 3]) -> u32 {
        self.bits().map(|(b, node)| bool_writer::cost(b, probs[node])).sum()
    }
}

/// What a decoder keeps of a coded 8×8 that the blocks after it read, and
/// the next frame's read of its vector.
#[derive(Clone, Copy, Default)]
pub(super) struct ModeInfo {
    skip: bool,
    /// An intra block's mode, which a keyframe codes the next blocks' modes
    /// by; DC for an inter block, as a decoder takes it.
    mode: IntraMode,
    /// How an inter block's vector was coded, and the vector; none for an
    /// intra block.
    inter: Option<(InterMode, Mv)>,
    /// The transform size, as log2 less 2.
    tx: usize,
    /// The side of the block it is in, in 8×8s as log2.
    n: u32,
    /// Whether the block is in the inactive segment: the previous frame as
    /// it is, which a decoder reads nothing else of and does not filter.
    inactive: bool,
    /// Whether its segment is what the last map predicted (libvpx's
    /// `seg_id_predicted`), which the blocks after it read as the context
    /// of their own prediction flag.
    predicted: bool,
}

/// What a decoder keeps of a tile column, which nothing outside the column
/// reads: the mode info of every 8×8 of it coded so far, and of the frame
/// before; and, as the column is coded a superblock row at a time, each
/// row's bools and what each superblock leaves the row below it. Each is the
/// column's alone, from its first 8×8 column.
pub(super) struct Contexts {
    mode_info: Vec<ModeInfo>,
    last_mode_info: Vec<ModeInfo>,
    /// The column's width in 8×8s, and in superblocks.
    width: usize,
    sb_cols: usize,
    /// Whether a decoder predicts vectors from the frame before's too:
    /// when that was an inter frame (libvpx's `use_prev_frame_mvs`).
    use_last: bool,
    /// Of the blocks the search between pixels refined, what their luma
    /// would cost coded at the vectors they came with, and what the search
    /// saved, as `laplacian` models them: the column's, and per superblock
    /// row.
    pub(super) searched: [f64; 2],
    searched_rows: Vec<[f64; 2]>,
    /// Per superblock row, its bools, to be coded once the frame's
    /// probabilities are chosen.
    pub(super) tapes: Vec<Tape>,
    /// Per superblock, what it leaves the superblock below it, once its row
    /// has coded it.
    edges: Vec<OnceLock<Edge>>,
    /// Per superblock, whether any block of it was coded in the active
    /// segment, so that the loop filter has an edge in it.
    coded: Vec<bool>,
}

impl Contexts {
    /// The contexts of the tile column of the 8×8 columns `cols`.
    pub(super) fn new(geometry: &Geometry, cols: std::ops::Range<usize>) -> Self {
        let sb_cols = cols.len().div_ceil(8);
        Self {
            mode_info: vec![ModeInfo::default(); geometry.mi_rows * cols.len()],
            last_mode_info: vec![ModeInfo::default(); geometry.mi_rows * cols.len()],
            width: cols.len(),
            sb_cols,
            use_last: false,
            searched: [0.0; 2],
            searched_rows: vec![[0.0; 2]; geometry.sb_rows],
            tapes: (0..geometry.sb_rows).map(|_| Tape::new()).collect(),
            edges: (0..geometry.sb_rows * sb_cols).map(|_| OnceLock::new()).collect(),
            coded: vec![false; geometry.sb_rows * sb_cols],
        }
    }

    /// The vectors of the frame coded last's inter blocks, one per 8×8.
    #[cfg(test)]
    pub(super) fn vectors(&self) -> impl Iterator<Item = Mv> + '_ {
        self.mode_info.iter().filter_map(|m| m.inter.map(|(_, mv)| mv))
    }

    /// The intra modes of the frame coded last's intra blocks, one per 8×8.
    #[cfg(test)]
    pub(super) fn intra_modes(&self) -> impl Iterator<Item = IntraMode> + '_ {
        self.mode_info.iter().filter(|m| m.inter.is_none()).map(|m| m.mode)
    }

    /// What the loop filter reads of the block coded last at the 8×8 in row
    /// `mi_row` and the column's 8×8 column `col`.
    pub(super) fn filtered(&self, mi_row: usize, col: usize) -> loopfilter::Block {
        let m = self.mode_info[mi_row * self.width + col];
        loopfilter::Block { n: m.n, tx: m.tx, intra: m.inter.is_none(), skip: m.skip, inactive: m.inactive }
    }

    /// Whether the loop filter has an edge in the superblock at superblock
    /// row `sb_row` and the column's superblock column `sb_col` of the frame
    /// coded last: any block of it is in the active segment.
    pub(super) fn coded(&self, sb_row: usize, sb_col: usize) -> bool {
        self.coded[sb_row * self.sb_cols + sb_col]
    }

    /// Back to how a decoder starts a frame, the mode info coded last kept
    /// as the frame before's. `last_inter` says whether that frame was an
    /// inter frame.
    pub(super) fn start(&mut self, last_inter: bool) {
        // Every 8×8's mode info is written as the frame is coded, before
        // anything reads it, so none is cleared.
        std::mem::swap(&mut self.mode_info, &mut self.last_mode_info);
        self.use_last = last_inter;
        self.searched = [0.0; 2];
        self.searched_rows.fill([0.0; 2]);
        for tape in &mut self.tapes {
            tape.clear();
        }
        for edge in &mut self.edges {
            edge.take();
        }
        self.coded.fill(false);
    }

    /// A job for each superblock row of the column, which `column` holds
    /// the pixels of.
    fn jobs<'a>(&'a mut self, column: &'a mut Column) -> impl Iterator<Item = Job<'a>> {
        let Contexts { mode_info, last_mode_info, width, sb_cols, use_last, searched_rows, tapes, edges, coded, .. } = self;
        let (width, sb_cols, use_last) = (*width, *sb_cols, *use_last);
        let (last_mode_info, edges) = (&last_mode_info[..], &edges[..]);
        let (x, stride) = (column.x, column.stride);
        let [p0, p1, p2] = &mut column.planes;
        let rows = 64 * stride;
        let planes = p0.chunks_mut(rows).zip(p1.chunks_mut(rows)).zip(p2.chunks_mut(rows));
        let rows = mode_info.chunks_mut(8 * width).zip(planes).zip(tapes.iter_mut().zip(searched_rows)).zip(coded.chunks_mut(sb_cols));
        rows.enumerate().map(move |(sb_row, (((mode_info, ((p0, p1), p2)), (tape, searched)), coded))| Job {
            sb_row,
            x,
            stride,
            width,
            sb_cols,
            use_last,
            last_mode_info,
            edges,
            mode_info,
            planes: [p0, p1, p2],
            tape,
            searched,
            coded,
        })
    }
}

/// What a superblock leaves the superblock below it once it is coded: the
/// partition contexts and whether the transform blocks along its bottom
/// edge had coefficients, the mode info of its last three rows of 8×8s, as
/// far up as a block below predicts its vectors from, and its last row of
/// pixels, which DC below it predicts from.
struct Edge {
    partition: [u8; 8],
    nonzero: [[u8; 16]; 3],
    mode_info: [[ModeInfo; 8]; 3],
    pixels: [[u8; 64]; 3],
}

/// What every superblock row of a frame is coded with.
pub(super) struct Frame<'a> {
    pub(super) geometry: &'a Geometry,
    pub(super) source: &'a Source<'a>,
    /// What an inter frame is coded against; none in a keyframe.
    pub(super) previous: Option<Previous<'a>>,
    pub(super) quantizer: &'a Quantizer,
    /// What tokens cost at the probabilities the frame starts from.
    pub(super) costs: &'a tokens::Costs,
}

/// One superblock row of a tile column: what it codes into, and what it
/// reads of the column.
struct Job<'a> {
    sb_row: usize,
    /// The column's first pixel column of the frame, the rows' stride, and
    /// its width in 8×8s and in superblocks.
    x: usize,
    stride: usize,
    width: usize,
    sb_cols: usize,
    use_last: bool,
    last_mode_info: &'a [ModeInfo],
    /// The column's, per superblock.
    edges: &'a [OnceLock<Edge>],
    /// The mode info and the pixels of the row's own 8×8s.
    mode_info: &'a mut [ModeInfo],
    planes: [&'a mut [u8]; 3],
    tape: &'a mut Tape,
    searched: &'a mut [f64; 2],
    /// Per superblock of the row, whether a block of it is active.
    coded: &'a mut [bool],
}

/// Code every superblock row of every tile column of a frame, on `threads`
/// of `pool`'s threads: each takes the next row in the order rows are
/// across the frame, and codes a superblock once the row above it in its
/// column has coded the one above it. A row reads of the rows above only
/// what they leave it, so the frame is the same on any number of threads.
pub(super) fn code(pool: &Pool, threads: usize, frame: &Frame, contexts: &mut [Contexts], columns: &mut [Column]) {
    let mut columns: Vec<_> = contexts.iter_mut().zip(columns).map(|(contexts, column)| contexts.jobs(column)).collect();
    let mut queue = Vec::new();
    for _ in 0..frame.geometry.sb_rows {
        queue.extend(columns.iter_mut().map(|jobs| Mutex::new(jobs.next())));
    }
    drop(columns);
    let (next, failed) = (AtomicUsize::new(0), AtomicBool::new(false));
    let work = || {
        let _failed = Failed(&failed);
        let mut scratch = Scratch::new();
        while let Some(job) = queue.get(next.fetch_add(1, Ordering::Relaxed)) {
            let job = job.lock().expect("a job taken whole").take().expect("a row for every job");
            scratch = Tile::new(frame, job, scratch, &failed).code();
        }
    };
    pool.run(threads.min(queue.len()), |_| work());
    for contexts in contexts {
        contexts.searched = contexts.searched_rows.iter().fold([0.0; 2], |[c, s], row| [c + row[0], s + row[1]]);
    }
}

/// Raises the flag it holds if its thread unwinds, so that the rows waiting
/// on that thread's give up rather than wait for ever.
struct Failed<'a>(&'a AtomicBool);

impl Drop for Failed<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.store(true, Ordering::Relaxed);
        }
    }
}

/// Each plane's sum and the sum of the squares of all three, over some
/// pixels.
#[derive(Clone, Copy, Default)]
struct Moments {
    sums: [i64; 3],
    squares: i64,
}

impl Moments {
    fn add(&mut self, other: &Moments) {
        for (s, o) in self.sums.iter_mut().zip(other.sums) {
            *s += o;
        }
        self.squares += other.squares;
    }

    /// The variance per pixel, summed over the planes, times `count`
    /// squared, of `count` pixels.
    fn variance(&self, count: i64) -> i64 {
        self.squares * count - self.sums.iter().map(|s| s * s).sum::<i64>()
    }
}

/// What the partition decision reads of an 8×8: the moments of its source
/// and, in an inter frame, of its source less the previous frame there.
#[derive(Clone, Copy, Default)]
struct Measure {
    source: Moments,
    residual: Moments,
}

/// The eight pixels of a row of the source from `x`, as it is coded: past
/// the row's end, its last pixel.
fn eight(row: &[u8], x: usize) -> [u8; 8] {
    match row.get(x..x + 8) {
        Some(eight) => eight.try_into().expect("eight pixels"),
        None => std::array::from_fn(|i| row[(x + i).min(row.len() - 1)]),
    }
}

/// What coding a superblock row works in and leaves nothing in that the
/// next row reads, kept from row to row on a thread.
struct Scratch {
    moved: Vec<(Mv, [Moments; 64], u64)>,
    levels: [Vec<i32>; 3],
    eobs: [Vec<usize>; 3],
    best_levels: Vec<i32>,
    best_eobs: Vec<usize>,
    patch: Vec<u8>,
    planes: [Vec<u8>; 16],
    predicted: Box<[u8; 64 * 64]>,
    residual: Box<[i32; 1024]>,
    coeffs: Box<[i32; 1024]>,
    energy: Box<[u8; 1024]>,
}

impl Scratch {
    fn new() -> Self {
        Self {
            moved: Vec::new(),
            levels: std::array::from_fn(|_| vec![0; 64 * 64]),
            eobs: std::array::from_fn(|_| Vec::with_capacity(256)),
            best_levels: vec![0; 64 * 64],
            best_eobs: Vec::with_capacity(256),
            patch: Vec::new(),
            planes: std::array::from_fn(|_| Vec::new()),
            predicted: Box::new([0; 64 * 64]),
            residual: Box::new([0; 1024]),
            coeffs: Box::new([0; 1024]),
            energy: Box::new([0; 1024]),
        }
    }
}

/// The coder of one superblock row of a tile column.
struct Tile<'a> {
    geometry: &'a Geometry,
    mi_col_start: usize,
    mi_col_end: usize,
    /// The row's first row of 8×8s.
    top: usize,
    contexts: Job<'a>,
    /// Per column of 8×8, and per plane per column of 4×4, of the column:
    /// the partition context, and whether the transform block above had a
    /// coefficient, read across a block's top edge.
    above_partition: Vec<u8>,
    above_nonzero: [Vec<u8>; 3],
    /// The three rows of mode info above the row, and per plane the row of
    /// pixels, as the row above left them.
    above_mode_info: Vec<ModeInfo>,
    above_pixels: [Vec<u8>; 3],
    left_partition: [u8; 8],
    /// Per plane, per row of 4×4 in the superblock row.
    left_nonzero: [[u8; 16]; 3],
    source: &'a Source<'a>,
    /// What an inter frame is coded against; none in a keyframe.
    previous: Option<Previous<'a>>,
    quantizer: &'a Quantizer,
    /// What tokens cost at the probabilities the frame starts from.
    costs: &'a tokens::Costs,
    /// Whether a thread coding a row above has failed.
    failed: &'a AtomicBool,
    /// The variance a square is split above, `SPLIT_VARIANCE` of the step.
    split_variance: i64,
    /// The superblock being coded, per 8×8 in raster order.
    measures: [Measure; 64],
    /// Per 8×8 of the superblock and per plane, the sums of its source's
    /// rows and of its columns, once an intra mode's residual has asked for
    /// them, and which are there yet.
    line_sums: [[([u16; 8], [u16; 8]); 3]; 64],
    summed: u64,
    /// The blocks the search between pixels could have refined so far.
    searches: usize,
    /// The superblock being coded, per vector other than zero measured at
    /// so far: per 8×8, the moments of its residual at that vector, and
    /// which of them are measured yet.
    moved: Vec<(Mv, [Moments; 64], u64)>,
    /// The block being coded: per plane, the levels of its transform blocks
    /// one after another, each in raster order, and their ends of block.
    levels: [Vec<i32>; 3],
    eobs: [Vec<usize>; 3],
    /// The luma levels and ends of block of the best transform size tried so
    /// far.
    best_levels: Vec<i32>,
    best_eobs: Vec<usize>,
    /// Scratch for a block of the previous frame copied out past its edges,
    /// for it filtered across at each sixteenth, and for a prediction
    /// filtered from it.
    patch: Vec<u8>,
    planes: [Vec<u8>; 16],
    predicted: Box<[u8; 64 * 64]>,
    /// Scratch for a transform block's residual and coefficients, and for
    /// the token coder.
    residual: Box<[i32; 1024]>,
    coeffs: Box<[i32; 1024]>,
    energy: Box<[u8; 1024]>,
}

impl<'a> Tile<'a> {
    /// The coder of the superblock row `job`, working in `scratch`.
    fn new(frame: &Frame<'a>, job: Job<'a>, scratch: Scratch, failed: &'a AtomicBool) -> Self {
        let Scratch { mut moved, levels, eobs, best_levels, best_eobs, patch, planes, predicted, residual, coeffs, energy } = scratch;
        moved.clear();
        let mi_col_start = job.x / 8;
        Self {
            geometry: frame.geometry,
            mi_col_start,
            mi_col_end: mi_col_start + job.width,
            top: job.sb_row * 8,
            above_partition: vec![0; job.sb_cols * 8],
            above_nonzero: std::array::from_fn(|_| vec![0; job.sb_cols * 16]),
            above_mode_info: vec![ModeInfo::default(); 3 * job.width],
            above_pixels: std::array::from_fn(|_| vec![0; job.stride]),
            contexts: job,
            left_partition: [0; 8],
            left_nonzero: [[0; 16]; 3],
            source: frame.source,
            previous: frame.previous,
            quantizer: frame.quantizer,
            costs: frame.costs,
            failed,
            split_variance: i64::from(frame.quantizer.ac_step()) / SPLIT_VARIANCE,
            measures: [Measure::default(); 64],
            line_sums: [[([0; 8], [0; 8]); 3]; 64],
            summed: 0,
            searches: 0,
            moved,
            levels,
            eobs,
            best_levels,
            best_eobs,
            patch,
            planes,
            predicted,
            residual,
            coeffs,
            energy,
        }
    }

    /// Decide every superblock of the row, its bools onto its tape, and give
    /// back the scratch it worked in.
    fn code(mut self) -> Scratch {
        let mi_row = self.top;
        for (sb_col, mi_col) in (self.mi_col_start..self.mi_col_end).step_by(8).enumerate() {
            if mi_row > 0 {
                self.take_edge(sb_col);
            }
            if !self.unchanged(mi_row, mi_col, SB_LOG2) {
                self.measure(mi_row, mi_col);
                self.moved.clear();
                self.summed = 0;
            }
            self.partition(mi_row, mi_col, SB_LOG2);
            if mi_row + 8 < self.geometry.mi_rows {
                self.leave_edge(sb_col);
            }
        }
        let Tile { moved, levels, eobs, best_levels, best_eobs, patch, planes, predicted, residual, coeffs, energy, .. } = self;
        Scratch { moved, levels, eobs, best_levels, best_eobs, patch, planes, predicted, residual, coeffs, energy }
    }

    /// Wait for the row above to code the superblock above superblock
    /// `sb_col` of the row, and take what it leaves.
    fn take_edge(&mut self, sb_col: usize) {
        let edges = self.contexts.edges;
        let cell = &edges[(self.top / 8 - 1) * self.contexts.sb_cols + sb_col];
        let edge = loop {
            if let Some(edge) = cell.get() {
                break edge;
            }
            assert!(!self.failed.load(Ordering::Relaxed), "a row above failed");
            std::thread::yield_now();
        };
        let (w, x) = (self.contexts.width, sb_col * 8);
        let cols = (w - x).min(8);
        self.above_partition[x..x + 8].copy_from_slice(&edge.partition);
        for plane in 0..3 {
            self.above_nonzero[plane][2 * x..2 * x + 16].copy_from_slice(&edge.nonzero[plane]);
            self.above_pixels[plane][8 * x..8 * x + 64].copy_from_slice(&edge.pixels[plane]);
        }
        for (r, row) in edge.mode_info.iter().enumerate() {
            self.above_mode_info[r * w + x..][..cols].copy_from_slice(&row[..cols]);
        }
    }

    /// Leave the row below what superblock `sb_col` of the row, coded,
    /// leaves it.
    fn leave_edge(&self, sb_col: usize) {
        let (w, x, stride) = (self.contexts.width, sb_col * 8, self.contexts.stride);
        let cols = (w - x).min(8);
        let mut edge = Edge {
            partition: self.above_partition[x..x + 8].try_into().expect("a superblock's"),
            nonzero: std::array::from_fn(|plane| self.above_nonzero[plane][2 * x..2 * x + 16].try_into().expect("a superblock's")),
            mode_info: [[ModeInfo::default(); 8]; 3],
            pixels: std::array::from_fn(|plane| self.contexts.planes[plane][63 * stride + 8 * x..][..64].try_into().expect("a superblock's")),
        };
        for (r, row) in edge.mode_info.iter_mut().enumerate() {
            row[..cols].copy_from_slice(&self.contexts.mode_info[(5 + r) * w + x..][..cols]);
        }
        let left = self.contexts.edges[self.top / 8 * self.contexts.sb_cols + sb_col].set(edge);
        assert!(left.is_ok(), "a superblock's edge left once");
    }

    /// The mode info of the 8×8 at `(mi_row, mi_col)` of the tile, coded
    /// already: in the row, or in the three rows above it, as they left it.
    fn mode_info(&self, mi_row: usize, mi_col: usize) -> ModeInfo {
        let (w, col) = (self.contexts.width, mi_col - self.mi_col_start);
        match mi_row.checked_sub(self.top) {
            Some(r) => self.contexts.mode_info[r * w + col],
            None => self.above_mode_info[(mi_row + 3 - self.top) * w + col],
        }
    }

    /// Where the tile keeps the mode info of the 8×8 at `(mi_row, mi_col)`
    /// of the frame before.
    fn at(&self, mi_row: usize, mi_col: usize) -> usize {
        mi_row * self.contexts.width + mi_col - self.mi_col_start
    }

    /// The 8×8s of the square of `1 << n` at `(mi_row, mi_col)` that the
    /// frame holds, as rows and columns of the superblock.
    fn inside(&self, mi_row: usize, mi_col: usize, n: u32) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
        let g = self.geometry;
        let (r, c) = (mi_row & 7, mi_col & 7);
        let bw = 1 << n;
        (r..r + bw.min(g.mi_rows - mi_row), c..c + bw.min(g.mi_cols - mi_col))
    }

    /// Whether, in an inter frame, the source of the square of `1 << n`
    /// 8×8s at `(mi_row, mi_col)` is the previous frame's wherever the frame
    /// holds it.
    fn unchanged(&self, mi_row: usize, mi_col: usize, n: u32) -> bool {
        self.count_unchanged(mi_row, mi_col, n) == Some(self.count_inside(mi_row, mi_col, n))
    }

    /// In an inter frame, how many of the 8×8s of the square of `1 << n` at
    /// `(mi_row, mi_col)` that the frame holds are the previous frame's.
    fn count_unchanged(&self, mi_row: usize, mi_col: usize, n: u32) -> Option<usize> {
        let previous = self.previous.as_ref()?;
        let mi_cols = self.geometry.mi_cols;
        let (rows, cols) = self.inside(mi_row, mi_col, n);
        let (row0, col0) = (mi_row - (mi_row & 7), mi_col - (mi_col & 7));
        Some(rows.into_iter().map(|r| previous.unchanged[(row0 + r) * mi_cols + col0..][cols.clone()].iter().filter(|&&u| u).count()).sum())
    }

    /// How many 8×8s of the square of `1 << n` at `(mi_row, mi_col)` the
    /// frame holds.
    fn count_inside(&self, mi_row: usize, mi_col: usize, n: u32) -> usize {
        let (rows, cols) = self.inside(mi_row, mi_col, n);
        rows.len() * cols.len()
    }

    /// The measures of every 8×8 of the superblock at `(mi_row, mi_col)`
    /// that the frame holds, the residual's against the previous frame where
    /// the 8×8 is.
    fn measure(&mut self, mi_row: usize, mi_col: usize) {
        let (rows, cols) = self.inside(mi_row, mi_col, SB_LOG2);
        for r in rows {
            for c in cols.clone() {
                let (px, py) = ((mi_col + c) * 8, (mi_row + r) * 8);
                let source = self.source_moments(px, py);
                let residual = if self.previous.is_some() { self.residual_moments(px, py, Mv::ZERO) } else { Moments::default() };
                self.measures[r * 8 + c] = Measure { source, residual };
            }
        }
    }

    /// The moments of the source's 8×8 at `(px, py)` as it is coded: past
    /// the picture's edges, its last pixels repeated.
    fn source_moments(&self, px: usize, py: usize) -> Moments {
        let mut m = Moments::default();
        let src = self.source;
        if px + 8 <= src.width && py + 8 <= src.height {
            for plane in 0..3 {
                let (sum, squares) = pixels::moments(&src.planes[plane][py * src.stride + px..], src.stride);
                m.sums[plane] = i64::from(sum);
                m.squares += i64::from(squares);
            }
            return m;
        }
        for plane in 0..3 {
            for y in py..py + 8 {
                for v in eight(src.row(plane, y), px) {
                    m.sums[plane] += i64::from(v);
                    m.squares += i64::from(v) * i64::from(v);
                }
            }
        }
        m
    }

    /// The moments of the source's 8×8 at `(px, py)` less the previous frame
    /// at `mv`, a vector of whole pixels, from it, read as a decoder reads it.
    fn residual_moments(&mut self, px: usize, py: usize, mv: Mv) -> Moments {
        let reference = self.previous.as_ref().expect("an inter frame").reference;
        let src = self.source;
        let ((dx, dy), _) = mv.split();
        let mut m = Moments::default();
        for plane in 0..3 {
            let (prediction, stride) = reference.block(plane, (px as isize + dx, py as isize + dy), (8, 8), (src.width, src.height), &mut self.patch);
            if px + 8 <= src.width && py + 8 <= src.height {
                let (sum, squares) = pixels::difference_moments(&src.planes[plane][py * src.stride + px..], src.stride, prediction, stride);
                m.sums[plane] = i64::from(sum);
                m.squares += i64::from(squares);
                continue;
            }
            for r in 0..8 {
                for (a, &b) in eight(src.row(plane, py + r), px).into_iter().zip(&prediction[r * stride..][..8]) {
                    let d = i64::from(a) - i64::from(b);
                    m.sums[plane] += d;
                    m.squares += d * d;
                }
            }
        }
        m
    }

    /// The moments of the square of `1 << n` 8×8s at `(mi_row, mi_col)`, as
    /// far as the frame holds it, less the previous frame at `mv`, a vector
    /// other than zero, from it: at its nearest whole pixels, which measure
    /// a vector between them for nothing and nearly as well as filtering
    /// would (filtering found 0.3–0.6% fewer bytes on the video captures for
    /// a sixth more time). Each 8×8 is measured at a vector once per
    /// superblock, however many squares it is in.
    fn square_residual(&mut self, mi_row: usize, mi_col: usize, n: u32, mv: Mv) -> Moments {
        let (rows, cols) = self.inside(mi_row, mi_col, n);
        let (row0, col0) = (mi_row - (mi_row & 7), mi_col - (mi_col & 7));
        let mv = mv.round();
        let at = match self.moved.iter().position(|m| m.0 == mv) {
            Some(at) => at,
            None => {
                self.moved.push((mv, [Moments::default(); 64], 0));
                self.moved.len() - 1
            }
        };
        let mut total = Moments::default();
        for r in rows {
            for c in cols.clone() {
                let i = r * 8 + c;
                if self.moved[at].2 & 1 << i == 0 {
                    let m = self.residual_moments((col0 + c) * 8, (row0 + r) * 8, mv);
                    let (_, moments, measured) = &mut self.moved[at];
                    moments[i] = m;
                    *measured |= 1 << i;
                }
                total.add(&self.moved[at].1[i]);
            }
        }
        total
    }

    /// How the square of `1 << n` 8×8s at `(mi_row, mi_col)` is coded
    /// whole, and the moments of its source less that prediction (of the
    /// source alone for DC), or none when it varies more from its best
    /// prediction than a square is coded whole at.
    fn choose(&mut self, mi_row: usize, mi_col: usize, n: u32) -> Option<(Mode, Moments)> {
        let partly = match self.count_unchanged(mi_row, mi_col, n) {
            Some(u) if u == self.count_inside(mi_row, mi_col, n) => return Some((Mode::Unchanged, Moments::default())),
            Some(u) => u > 0,
            None => false,
        };
        let (rows, cols) = self.inside(mi_row, mi_col, n);
        let mut total = Measure::default();
        let mut count = 0;
        for r in rows {
            for m in &self.measures[r * 8..][cols.clone()] {
                total.source.add(&m.source);
                total.residual.add(&m.residual);
                count += 64;
            }
        }
        let dc = total.source.variance(count);
        let (mode, variance, moments) = match &self.previous {
            Some(previous) => {
                // Zero first, then the vectors a decoder predicts, then the
                // search's, which are new vectors to code and so have to do
                // clearly better than the others to be taken. Each is ranked
                // by the squares of its residual, its mean with them: what a
                // vector leaves as an offset is coded too.
                let searched = previous.vectors[(mi_row >> SB_LOG2) * self.geometry.sb_cols + (mi_col >> SB_LOG2)];
                let predicted = self.candidates(mi_row, mi_col, n);
                let mut best = (Mv::ZERO, total.residual.squares, total.residual);
                let mut tried = [Mv::ZERO; 1 + 2 + motion::VECTORS];
                for (i, mv) in predicted.into_iter().chain(searched).enumerate() {
                    if best.1 == 0 || tried.contains(&mv) {
                        continue;
                    }
                    tried[1 + i] = mv;
                    let moments = self.square_residual(mi_row, mi_col, n, mv);
                    let better = if i < predicted.len() { moments.squares < best.1 } else { moments.squares < best.1 - best.1 / NEW_VECTOR_GAIN };
                    if better {
                        best = (mv, moments.squares, moments);
                    }
                }
                let best = (best.0, best.2.variance(count), best.2);
                // The other intra modes are tried only where the previous
                // frame cannot code the square whole, or DC does better.
                let (intra, variance, moments) =
                    if best.1 > self.split_variance * count * count || dc < best.1 { self.intra_mode(mi_row, mi_col, n, &total.source, count) } else { (IntraMode::Dc, dc, total.source) };
                if best.1 <= variance { (Mode::Inter(best.0), best.1, best.2) } else { (Mode::Intra(intra), variance, moments) }
            }
            None => {
                let (intra, variance, moments) = self.intra_mode(mi_row, mi_col, n, &total.source, count);
                (Mode::Intra(intra), variance, moments)
            }
        };
        // A square that is unchanged in part is split unless what moved
        // into it is coded as moving, so the unchanged part stays the
        // previous frame's rather than coding again what the previous frame
        // got wrong there. What moved takes the unchanged part with it: it
        // is as likely to be where the motion left it as where it was.
        if partly && matches!(mode, Mode::Intra(_) | Mode::Inter(Mv::ZERO)) {
            return None;
        }
        // A 64×64 is predicted from the previous frame in its four 32×32s,
        // each with a vector of its own: on the shader animations of the
        // quiet samples, whose light moves over a pattern that stays, one
        // vector for the whole cost 1–2% of their bytes.
        if n == 3 && matches!(mode, Mode::Inter(_)) {
            return None;
        }
        (n == 0 || variance <= self.split_variance * count * count).then_some((mode, moments))
    }

    /// Of the intra modes, the one whose residual over the square of
    /// `1 << n` 8×8s at `(mi_row, mi_col)`, as far as the frame holds it,
    /// varies least, with its variance as `Moments::variance` gives it and
    /// its moments. DC's residual is taken as the source's own variance,
    /// `source` its moments over `count` pixels; the others' are exact but
    /// for TM's clipping, predicted from the source around the square
    /// rather than its reconstruction, from the sums of the rows and
    /// columns of each of its 8×8s.
    fn intra_mode(&mut self, mi_row: usize, mi_col: usize, n: u32, source: &Moments, count: i64) -> (IntraMode, i64, Moments) {
        let dc = source.variance(count);
        let (rows, cols) = self.inside(mi_row, mi_col, n);
        let (row0, col0) = (mi_row - (mi_row & 7), mi_col - (mi_col & 7));
        let (px, py, w, h) = (mi_col * 8, mi_row * 8, cols.len() * 8, rows.len() * 8);
        let src = self.source;
        let (up, left) = (mi_row > 0, mi_col > self.mi_col_start);
        let (mut colsum, mut rowsum) = ([0i64; 64], [0i64; 64]);
        // Per plane the residual's sum, and the sum of its squares over the
        // planes, each per mode: V, H and TM.
        let mut sums = [[0i64; 3]; 3];
        let mut squares = [source.squares; 3];
        for (plane, sums) in sums.iter_mut().enumerate() {
            colsum[..w].fill(0);
            rowsum[..h].fill(0);
            for r in rows.clone() {
                for c in cols.clone() {
                    let (row_sums, col_sums) = self.line_sums(row0 + r, col0 + c)[plane];
                    let (y, x) = ((r - rows.start) * 8, (c - cols.start) * 8);
                    for k in 0..8 {
                        rowsum[y + k] += i64::from(row_sums[k]);
                        colsum[x + k] += i64::from(col_sums[k]);
                    }
                }
            }
            let above = |c: usize| if up { i64::from(src.row(plane, py - 1)[(px + c).min(src.width - 1)]) } else { 127 };
            let beside = |r: usize| if left { i64::from(src.row(plane, py + r)[px - 1]) } else { 129 };
            let corner = match (up, left) {
                (true, true) => i64::from(src.row(plane, py - 1)[px - 1]),
                (true, false) => 129,
                (false, _) => 127,
            };
            let (mut a, mut a2, mut ac) = (0, 0, 0);
            for (c, &colsum) in colsum[..w].iter().enumerate() {
                let v = above(c);
                (a, a2, ac) = (a + v, a2 + v * v, ac + v * colsum);
            }
            let (mut l, mut l2, mut lr) = (0, 0, 0);
            for (r, &rowsum) in rowsum[..h].iter().enumerate() {
                let v = beside(r);
                (l, l2, lr) = (l + v, l2 + v * v, lr + v * rowsum);
            }
            let (w, h, s, k) = (w as i64, h as i64, source.sums[plane], corner);
            // V: each pixel less the one above the square in its column.
            sums[0] = s - h * a;
            squares[0] += -2 * ac + h * a2;
            // H: less the one left of it in its row.
            sums[1] = s - w * l;
            squares[1] += -2 * lr + w * l2;
            // TM: less those two less the corner.
            sums[2] = s - (w * l + h * a - w * h * k);
            squares[2] += -2 * (lr + ac - k * s) + w * l2 + h * a2 + w * h * k * k + 2 * l * a - 2 * k * (w * l + h * a);
        }
        let mut best = (IntraMode::Dc, dc, *source);
        for (i, mode) in [IntraMode::V, IntraMode::H, IntraMode::Tm].into_iter().enumerate() {
            let moments = Moments { sums: sums.map(|plane| plane[i]), squares: squares[i] };
            let variance = moments.variance(count);
            if variance < best.1.min(dc - dc / INTRA_MODE_GAIN) {
                best = (mode, variance, moments);
            }
        }
        best
    }

    /// The sums of the rows and of the columns of the source's 8×8 at
    /// `(mi_row, mi_col)`, per plane: past the picture's edges, its last
    /// pixels repeated.
    fn line_sums(&mut self, mi_row: usize, mi_col: usize) -> [([u16; 8], [u16; 8]); 3] {
        let i = (mi_row & 7) * 8 + (mi_col & 7);
        if self.summed & 1 << i == 0 {
            let src = self.source;
            let (px, py) = (mi_col * 8, mi_row * 8);
            self.line_sums[i] = std::array::from_fn(|plane| {
                if px + 8 <= src.width && py + 8 <= src.height {
                    return pixels::line_sums(&src.planes[plane][py * src.stride + px..], src.stride);
                }
                let mut sums = ([0u16; 8], [0u16; 8]);
                for r in 0..8 {
                    for (c, v) in eight(src.row(plane, py + r), px).into_iter().enumerate() {
                        sums.0[r] += u16::from(v);
                        sums.1[c] += u16::from(v);
                    }
                }
                sums
            });
            self.summed |= 1 << i;
        }
        self.line_sums[i]
    }

    /// The partition of the square of `1 << n` 8×8s at `(mi_row, mi_col)`:
    /// split where its second half in either direction lies past the frame's
    /// edge, or where `choose` finds no way to code it whole.
    fn partition(&mut self, mi_row: usize, mi_col: usize, n: u32) {
        let g = self.geometry;
        if mi_row >= g.mi_rows || mi_col >= g.mi_cols {
            return;
        }
        let half = (1 << n) >> 1;
        let has_rows = mi_row + half < g.mi_rows;
        let has_cols = mi_col + half < g.mi_cols;
        let mode = if has_rows && has_cols { self.choose(mi_row, mi_col, n) } else { None };
        let split = mode.is_none();
        let ctx = self.partition_context(mi_row, mi_col, n);
        let probs = if self.previous.is_some() { PARTITION_PROBS[ctx] } else { KF_PARTITION_PROBS[ctx] };
        match (has_rows, has_cols) {
            // PARTITION_NONE is the tree's first leaf; PARTITION_SPLIT its last.
            (true, true) => {
                self.contexts.tape.write(split, probs[0]);
                if split {
                    self.contexts.tape.write(true, probs[1]);
                    self.contexts.tape.write(true, probs[2]);
                }
            }
            // Split, or a horizontal half the frame cuts.
            (false, true) => self.contexts.tape.write(split, probs[1]),
            // Split, or a vertical half the frame cuts.
            (true, false) => self.contexts.tape.write(split, probs[2]),
            // Split, and the decoder knows it.
            (false, false) => {}
        }
        match mode {
            None => {
                for (r, c) in [(0, 0), (0, half), (half, 0), (half, half)] {
                    self.partition(mi_row + r, mi_col + c, n - 1);
                }
            }
            Some((mode, moments)) => {
                self.block(mi_row, mi_col, n, mode, &moments);
                let bw = 1 << n;
                let ctx = partition_context_of(n);
                let x = mi_col - self.mi_col_start;
                self.above_partition[x..x + bw].fill(ctx);
                self.left_partition[mi_row & 7..(mi_row & 7) + bw].fill(ctx);
            }
        }
    }

    /// libvpx's `partition_plane_context`.
    fn partition_context(&self, mi_row: usize, mi_col: usize, n: u32) -> usize {
        let above = (self.above_partition[mi_col - self.mi_col_start] >> n) & 1;
        let left = (self.left_partition[mi_row & 7] >> n) & 1;
        usize::from(left * 2 + above) + n as usize * 4
    }

    /// Of the first `count` neighbours a square block of `1 << n` 8×8s at
    /// `(mi_row, mi_col)` predicts its vectors from, nearest first, those
    /// that lie in the frame and the tile (libvpx's `mv_ref_blocks` and
    /// `is_inside`).
    fn neighbours(&self, mi_row: usize, mi_col: usize, n: u32, count: usize) -> impl Iterator<Item = ModeInfo> + '_ {
        let g = self.geometry;
        // BLOCK_8X8 is the fourth size, and each square size the third after.
        MV_REF_BLOCKS[3 + 3 * n as usize][..count].iter().filter_map(move |&[dr, dc]| {
            let (r, c) = (mi_row.checked_add_signed(dr.into())?, mi_col.checked_add_signed(dc.into())?);
            (r < g.mi_rows && (self.mi_col_start..self.mi_col_end).contains(&c)).then(|| self.mode_info(r, c))
        })
    }

    /// The mode context of an inter block (libvpx's `get_mode_context`):
    /// what the first two of its neighbours were coded as.
    fn mode_context(&self, mi_row: usize, mi_col: usize, n: u32) -> usize {
        let counter: usize = self
            .neighbours(mi_row, mi_col, n, 2)
            .map(|m| m.inter.map_or(INTRA_COUNT, |(mode, _)| mode.counter()))
            .sum();
        COUNTER_TO_CONTEXT[counter]
    }

    /// The two vectors a decoder predicts for a square block of `1 << n`
    /// 8×8s at `(mi_row, mi_col)`, nearest first, coding against the
    /// previous frame alone (libvpx's `dec_find_mv_refs`): the first two
    /// different vectors among its inter neighbours and then the block's own
    /// place in the frame before, if that was an inter frame; zero for each
    /// not found; both clamped near the frame.
    fn candidates(&self, mi_row: usize, mi_col: usize, n: u32) -> [Mv; 2] {
        let g = self.geometry;
        let last = self.contexts.use_last.then(|| self.contexts.last_mode_info[self.at(mi_row, mi_col)]);
        let mut found = self.neighbours(mi_row, mi_col, n, MV_REF_BLOCKS[0].len()).chain(last).filter_map(|m| m.inter.map(|(_, mv)| mv));
        let nearest = found.next().unwrap_or(Mv::ZERO);
        let near = found.find(|&mv| mv != nearest).unwrap_or(Mv::ZERO);
        [nearest, near].map(|mv| mv.clamp((mi_row, mi_col), 1 << n, (g.mi_rows, g.mi_cols)))
    }

    /// The transform size, as log2 less 2, of a block of `1 << n` 8×8s
    /// coded intra, whose source has `moments` over its `count` pixels: 4×4
    /// in an 8×8 or 16×16 whose pixels deviate by more than two steps, where
    /// DC predicted from nearer pixels does better, and otherwise the largest
    /// that fits up to 16×16 (libvpx's keyframe `ALLOW_16X16`). Measured on
    /// the plan's captures against a half, an eighth and a sixty-fourth of
    /// that variance; the 4×4s bought desktop 2 dB and 5% of its bytes.
    fn intra_transform_size(&self, n: u32, moments: &Moments, count: i64) -> usize {
        let ac = i64::from(self.quantizer.ac_step());
        // The variance per pixel and plane against two steps squared, a step
        // in pixels being an eighth of the AC step.
        if n <= 1 && moments.variance(count) * 16 > ac * ac * 3 * count * count {
            0
        } else {
            (n as usize + 1).min(2)
        }
    }

    /// Ends of block of zero in `eobs` for the transform blocks of size `tx`
    /// of a block of `1 << n` 8×8s at `(mi_row, mi_col)`, which codes no
    /// coefficient.
    fn nothing(&mut self, mi_row: usize, mi_col: usize, n: u32, tx: usize) {
        let count = self.transform_blocks(mi_row, mi_col, n, tx).1.count();
        for eobs in &mut self.eobs {
            eobs.clear();
            eobs.resize(count, 0);
        }
    }

    /// The transform size, as log2 less 2, of an inter block of `1 << n`
    /// 8×8s at `(mi_row, mi_col)` whose prediction is in `recon`: of every
    /// size that fits it, the one whose luma levels cost least in their
    /// squared error and their bits at `RATE_WEIGHT`, the error measured on
    /// the coefficients and the bits at the frame's starting probabilities.
    /// Its luma levels and ends of block are left in `levels` and `eobs`.
    /// Chroma takes the size luma chose: deciding each plane's part of the
    /// cost too saved 0.4–3% of the bytes for a fifth more time.
    fn trial(&mut self, mi_row: usize, mi_col: usize, n: u32) -> usize {
        let ac = i64::from(self.quantizer.ac_step());
        let mut best = (i64::MAX, 0);
        for tx in (0..=(n as usize + 1).min(3)).rev() {
            let side = 4 << tx;
            let area = side * side;
            let mut cost = 0;
            self.eobs[0].clear();
            for (i, (x, y)) in self.transform_blocks(mi_row, mi_col, n, tx).1.enumerate() {
                self.transform(0, mi_col * 8 + x, mi_row * 8 + y, side, TxType::DctDct);
                let levels = &mut self.levels[0][i * area..][..area];
                let (eob, error) = self.quantizer.quantize_with_error(side, tokens::scan(tx, TxType::DctDct).inverse, &self.coeffs[..area], levels);
                let kind = tokens::Kind { tx, tx_type: TxType::DctDct, chroma: false, inter: true };
                let bits = tokens::estimate(self.costs, kind, 0, levels, eob, &mut self.energy);
                // In 256000ths of the squared error on the coefficients'
                // scale, which is 64 times that in pixels.
                cost += error * 256_000 + RATE_WEIGHT * ac * ac * i64::from(bits);
                self.eobs[0].push(eob);
            }
            if cost < best.0 {
                best = (cost, tx);
                std::mem::swap(&mut self.levels[0], &mut self.best_levels);
                std::mem::swap(&mut self.eobs[0], &mut self.best_eobs);
            } else {
                // Below a size that costs more than the one above it, the
                // sizes cost more again.
                break;
            }
        }
        std::mem::swap(&mut self.levels[0], &mut self.best_levels);
        std::mem::swap(&mut self.eobs[0], &mut self.best_eobs);
        best.1
    }

    /// Code the transform size `tx` of the square block of `1 << n` 8×8s
    /// with `above` and `left` it (libvpx's `write_selected_tx_size`): the
    /// tree's nodes down to it, at the probabilities of its largest size
    /// and of whether the sizes above and to the left add up to more than
    /// that (`get_tx_size_context`), a skipped block's counting as the
    /// largest.
    fn write_tx(&mut self, n: u32, tx: usize, above: Option<ModeInfo>, left: Option<ModeInfo>) {
        let largest = (n as usize + 1).min(3);
        let size = |m: Option<ModeInfo>| m.map(|m| if m.skip { largest } else { m.tx });
        let (a, l) = match (size(above), size(left)) {
            (Some(a), Some(l)) => (a, l),
            (Some(a), None) => (a, a),
            (None, Some(l)) => (l, l),
            (None, None) => (largest, largest),
        };
        let slot = tx_slot(largest, usize::from(a + l > largest));
        for node in 0..largest.min(tx + 1) {
            self.contexts.tape.put(tx > node, slot + node);
        }
    }

    /// Code the square block of `1 << n` 8×8s at `(mi_row, mi_col)` in
    /// `mode`, whose source less its prediction has `moments`: at the
    /// transform size `trial` finds for an inter block and
    /// `intra_transform_size` gives an intra one, and skipped when no
    /// transform block keeps a coefficient.
    fn block(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode, moments: &Moments) {
        let g = self.geometry;
        // A skipped inter block's transform is the largest that fits it,
        // which it does not code.
        let largest = (n as usize + 1).min(3);
        let count = 64 * self.count_inside(mi_row, mi_col, n) as i64;
        let mode = match mode {
            Mode::Inter(mv) => Mode::Inter(self.refine(mi_row, mi_col, n, mv)),
            mode => mode,
        };
        let tx = match mode {
            Mode::Intra(_) => self.intra_transform_size(n, moments, count),
            Mode::Inter(mv) => {
                self.predict_inter(mi_row, mi_col, n, mv);
                self.trial(mi_row, mi_col, n)
            }
            Mode::Unchanged => largest,
        };
        self.reconstruct(mi_row, mi_col, n, mode, tx);
        let skip = self.eobs.iter().flatten().all(|&eob| eob == 0);
        let tx = if skip && !matches!(mode, Mode::Intra(_)) { largest } else { tx };
        let intra = if let Mode::Intra(intra) = mode { intra } else { IntraMode::Dc };
        let inter = match mode {
            Mode::Intra(_) => None,
            Mode::Inter(mv) => Some(self.inter_mode(mi_row, mi_col, n, mv)),
            // A decoder gives an inactive block ZEROMV, whatever it would
            // have cost to code.
            Mode::Unchanged => Some((InterMode::Zero, Mv::ZERO)),
        };
        let above = (mi_row > 0).then(|| self.mode_info(mi_row - 1, mi_col));
        let left = (mi_col > self.mi_col_start).then(|| self.mode_info(mi_row, mi_col - 1));
        let skips = usize::from(above.is_some_and(|m| m.skip)) + usize::from(left.is_some_and(|m| m.skip));
        let bw = 1 << n;
        let inactive = mode == Mode::Unchanged;
        // An inter frame's block says its segment first. The last map
        // predicts the active segment for it where any of its 8×8s was
        // active (libvpx's `dec_get_segment_id`, the least of them). An
        // inactive block says nothing more, a decoder taking the rest from
        // the segment.
        let mut predicted = false;
        if self.previous.is_some() {
            let (w, col) = (self.contexts.width, mi_col - self.mi_col_start);
            let last = &self.contexts.last_mode_info;
            let was_inactive = (mi_row..(mi_row + bw).min(g.mi_rows)).all(|row| last[row * w + col..][..bw.min(w - col)].iter().all(|m| m.inactive));
            predicted = inactive == was_inactive;
            let ctx = usize::from(above.is_some_and(|m| m.predicted)) + usize::from(left.is_some_and(|m| m.predicted));
            self.contexts.tape.segment(inactive, predicted, ctx);
        }
        if !inactive {
            self.contexts.tape.put(skip, skip_slot(skips));
            self.contexts.coded[(mi_col - self.mi_col_start) >> SB_LOG2] = true;
            if self.previous.is_none() {
                self.write_tx(n, tx, above, left);
                // The luma mode's probabilities are chosen by the modes above and
                // to the left, DC by rule where there is no block; the chroma
                // mode's, which is the luma's, by the luma's.
                let (a, l) = (above.map_or(IntraMode::Dc, |m| m.mode), left.map_or(IntraMode::Dc, |m| m.mode));
                self.write_intra(intra, &KF_Y_MODE_PROBS[a as usize][l as usize], &KF_UV_MODE_PROBS[intra as usize]);
            } else {
                // libvpx's `get_intra_inter_context`.
                let is_intra = |m: &ModeInfo| m.inter.is_none();
                let ctx = match (above, left) {
                    (Some(a), Some(l)) if is_intra(&a) && is_intra(&l) => 3,
                    (Some(a), Some(l)) => usize::from(is_intra(&a) || is_intra(&l)),
                    (Some(m), None) | (None, Some(m)) => 2 * usize::from(is_intra(&m)),
                    (None, None) => 0,
                };
                self.contexts.tape.write(inter.is_some(), INTRA_INTER_PROBS[ctx]);
                if !skip || inter.is_none() {
                    self.write_tx(n, tx, above, left);
                }
                if let Some((mode, mv)) = inter {
                    // LAST, the reference tree's first leaf, at libvpx's
                    // `vp9_get_pred_context_single_ref_p1` when LAST is the only
                    // reference: 4 beside an inter block, 2 otherwise.
                    let ctx = if above.is_some_and(|m| !is_intra(&m)) || left.is_some_and(|m| !is_intra(&m)) { 4 } else { 2 };
                    self.contexts.tape.write(false, SINGLE_REF_PROBS[ctx][0]);
                    let probs = INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)];
                    for (b, node) in mode.bits() {
                        self.contexts.tape.write(b, probs[node]);
                    }
                    if mode == InterMode::New {
                        let nearest = self.candidates(mi_row, mi_col, n)[0];
                        let tape = &mut self.contexts.tape;
                        mv::code(mv - nearest, |b, p| tape.write(b, p));
                    }
                } else {
                    // At the probabilities of the block's size group.
                    self.write_intra(intra, &Y_MODE_PROBS[(n as usize + 1).min(3)], &UV_MODE_PROBS[intra as usize]);
                }
            }
        }
        let (w, col) = (self.contexts.width, mi_col - self.mi_col_start);
        for row in mi_row..(mi_row + bw).min(g.mi_rows) {
            self.contexts.mode_info[(row - self.top) * w + col..][..bw.min(w - col)].fill(ModeInfo { skip, mode: intra, inter, tx, n, inactive, predicted });
        }
        if skip {
            // A skipped block leaves no coefficient above or left of the
            // blocks after it, over its whole size.
            let (x4, y4) = ((mi_col - self.mi_col_start) * 2, (mi_row * 2) & 15);
            for plane in 0..3 {
                self.above_nonzero[plane][x4..x4 + 2 * bw].fill(0);
                self.left_nonzero[plane][y4..y4 + 2 * bw].fill(0);
            }
        } else {
            self.tokens(mi_row, mi_col, n, mode, tx);
        }
    }

    /// Code the intra mode `mode` at the luma mode probabilities `y`, and as
    /// the chroma mode too at `uv`.
    fn write_intra(&mut self, mode: IntraMode, y: &[u8; 9], uv: &[u8; 9]) {
        for probs in [y, uv] {
            for &(b, node) in mode.bits() {
                self.contexts.tape.write(b, probs[node]);
            }
        }
    }

    /// The type of the transforms of plane `plane` of a block coded in
    /// `mode` at size `tx`: an intra block's luma below 32×32 its mode's,
    /// all else a DCT.
    fn tx_type(mode: Mode, plane: usize, tx: usize) -> TxType {
        match mode {
            Mode::Intra(intra) if plane == 0 && tx < 3 => intra.tx_type(),
            _ => TxType::DctDct,
        }
    }

    /// The cheapest way to code `mv` for the square block of `1 << n` 8×8s
    /// at `(mi_row, mi_col)`.
    fn inter_mode(&self, mi_row: usize, mi_col: usize, n: u32, mv: Mv) -> (InterMode, Mv) {
        (Prices::new(self.candidates(mi_row, mi_col, n), &INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)]).of(mv).1, mv)
    }

    /// The vector within three quarters of a pixel of `mv` that the square
    /// block of `1 << n` 8×8s at `(mi_row, mi_col)` is best predicted at, by
    /// its luma's variance from the prediction and the bits of the vector at
    /// `RATE_WEIGHT`, the variance being an error to come in the residual,
    /// and then kept only where it pays as `laplacian` models it: libvpx's
    /// pruned sub-pixel search
    /// (`vp9_find_best_sub_pixel_tree_pruned_more`), a half pixel where the
    /// whole pixels around `mv` say, or each way, and then each way a quarter
    /// pixel from the best. The patch around `mv` is filtered across once
    /// per sixteenth the vectors tried ask for, so each of them is at most a
    /// pass down. A block the picture does not hold whole keeps `mv`, as
    /// does one whose luma `mv` predicts exactly.
    fn refine(&mut self, mi_row: usize, mi_col: usize, n: u32, mv: Mv) -> Mv {
        let src = self.source;
        let size = 8 << n;
        let (px, py) = (mi_col * 8, mi_row * 8);
        if px + size > src.width || py + size > src.height {
            return mv;
        }
        let previous = self.previous.as_ref().expect("an inter frame");
        let reference = previous.reference;
        self.searches += 1;
        if !self.searches.is_multiple_of(previous.search_every) {
            return mv;
        }
        let prices = Prices::new(self.candidates(mi_row, mi_col, n), &INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)]);
        let ac = i64::from(self.quantizer.ac_step());
        let count = (size * size) as i64;
        let ((dx, dy), _) = mv.split();
        // Every vector tried is less than a pixel from `mv`, so a pixel more
        // each way of the filter's reach holds what any of them reads; and
        // filtered across, a row of whole eights from a pixel before `mv`'s
        // holds any of them filtered across.
        let before = convolve::BEFORE;
        let (wide, high) = (size + 8, size + 2 + TAPS - 1);
        let margin = 1 + before as isize;
        let (block, stride) = reference.block(0, (px as isize + dx - margin, py as isize + dy - margin), (wide + TAPS - 1, high), (src.width, src.height), &mut self.patch);
        // Per sixteenth across, the patch filtered across at it, once a
        // vector tried has asked for it.
        let (planes, prediction) = (&mut self.planes, &mut self.predicted[..]);
        let mut filtered = 0u16;
        let source = &src.planes[0][py * src.stride + px..];
        let bits = |v: Mv| prices.of(v).0;
        let rate = |v: Mv| RATE_WEIGHT * ac * ac * i64::from(bits(v));
        // A vector's luma residual: its squared deviation from its mean, and
        // that weighed with the vector's bits.
        let mut cost = |v: Mv| {
            let ((vx, vy), (fx, fy)) = v.split();
            // Where the vector's whole pixels are from the patch's first.
            let (x, y) = ((vx - dx + 1) as usize, (vy - dy + 1) as usize);
            // What to filter down: the patch, or the patch filtered across,
            // from `before` rows above the vector's.
            let (across, from, x) = if fx == 0 {
                (block, stride, x + before)
            } else {
                if filtered & 1 << fx == 0 {
                    planes[fx].resize(wide * high, 0);
                    convolve::across(block, stride, fx, (wide, high), &mut planes[fx], wide);
                    filtered |= 1 << fx;
                }
                (&planes[fx][..], wide, x)
            };
            let (predicted, predicted_stride) = if fy == 0 {
                (&across[(y + before) * from + x..], from)
            } else {
                convolve::down(&across[y * from + x..], from, fy, (size, size), prediction, size);
                (&prediction[..], size)
            };
            let mut m = Moments::default();
            for r in (0..size).step_by(8) {
                for c in (0..size).step_by(8) {
                    let (sum, squares) = pixels::difference_moments(&source[r * src.stride + c..], src.stride, &predicted[r * predicted_stride + c..], predicted_stride);
                    m.sums[0] += i64::from(sum);
                    m.squares += i64::from(squares);
                }
            }
            let deviation = m.variance(count) / count;
            // As `trial` weighs an error, on the coefficients' scale.
            (deviation * 64 * 256_000 + rate(v), deviation)
        };
        let first = cost(mv);
        // Nothing to take from a prediction that is the source.
        if first.1 == 0 {
            return mv;
        }
        let mut best = (first.0, mv, first.1);
        // From a vector of whole pixels that predicts better than the whole
        // pixels around it, a half pixel only where those say the least
        // lies, fitting a parabola through them (libvpx's
        // `get_cost_surf_min`); otherwise each way.
        let mut whole = |row: i32, col: i32| cost(Mv { row: mv.row + row, col: mv.col + col }).0;
        match (mv.split().1 == (0, 0)).then(|| [whole(0, -8), whole(0, 8), whole(-8, 0), whole(8, 0)]) {
            Some([left, right, up, down]) if [left, right, up, down].iter().all(|&c| c > first.0) => {
                let towards = |less: i64, more: i64| rounded(less - more, less - 2 * first.0 + more) as i32;
                let (col, row) = (towards(left, right), towards(up, down));
                if (row, col) != (0, 0) {
                    try_at(&mut cost, &mut best, row * 4, col * 4);
                }
            }
            _ => around(&mut cost, &mut best, 4),
        }
        around(&mut cost, &mut best, 2);
        // What the residual costs coded is no straight line in its
        // deviation: the vector found is taken only where its residual
        // modelled as coded, and its bits, do better.
        let (step, weight) = (ac as f64 / 8.0, (RATE_WEIGHT * ac * ac) as f64 / 64_000.0);
        let coded = |deviation: i64, v: Mv| {
            let (distortion, residual) = laplacian(deviation as f64, count as f64, step);
            distortion + weight * (residual + f64::from(bits(v)) / 256.0)
        };
        let came = coded(first.1, mv);
        let saved = if best.1 == mv { 0.0 } else { (came - coded(best.2, best.1)).max(0.0) };
        self.contexts.searched[0] += came;
        self.contexts.searched[1] += saved;
        if saved > 0.0 { best.1 } else { mv }
    }

    /// The transform blocks of size `tx` (log2 less 2) of a block at
    /// `(mi_row, mi_col)`, in the order they are predicted and coded: per
    /// plane, rows of them, each starting inside the frame's edge rounded up
    /// to 8. With their side, the offset of each in pixels from the block's
    /// corner.
    fn transform_blocks(&self, mi_row: usize, mi_col: usize, n: u32, tx: usize) -> (usize, impl Iterator<Item = (usize, usize)> + use<>) {
        let g = self.geometry;
        let size = 8 << n;
        let tx = 4 << tx;
        let wide = size.min((g.mi_cols - mi_col) * 8);
        let high = size.min((g.mi_rows - mi_row) * 8);
        (tx, (0..high).step_by(tx).flat_map(move |y| (0..wide).step_by(tx).map(move |x| (x, y))))
    }

    /// Predict the block from the previous frame at `mv` from where it is,
    /// over the part of it inside the frame's edge rounded up to 8: as a
    /// decoder does, past the picture's edges the previous frame's edge
    /// pixels repeat.
    fn predict_inter(&mut self, mi_row: usize, mi_col: usize, n: u32, mv: Mv) {
        let g = self.geometry;
        let reference = self.previous.as_ref().expect("an inter frame").reference;
        let picture = (self.source.width, self.source.height);
        let ((dx, dy), fraction) = mv.split();
        let size = 8 << n;
        let (px, py) = (mi_col * 8, mi_row * 8);
        let wide = size.min(g.mi_cols * 8 - px);
        let high = size.min(g.mi_rows * 8 - py);
        let (stride, x, y) = (self.contexts.stride, px - self.contexts.x, py - self.top * 8);
        let (rx, ry) = (px as isize + dx, py as isize + dy);
        for (plane, recon) in self.contexts.planes.iter_mut().enumerate() {
            let out = &mut recon[y * stride + x..];
            if fraction == (0, 0) {
                let (block, from) = reference.block(plane, (rx, ry), (wide, high), picture, &mut self.patch);
                for r in 0..high {
                    out[r * stride..][..wide].copy_from_slice(&block[r * from..][..wide]);
                }
            } else {
                let before = convolve::BEFORE as isize;
                let (block, from) = reference.block(plane, (rx - before, ry - before), (wide + TAPS - 1, high + TAPS - 1), picture, &mut self.patch);
                convolve::predict(block, from, fraction, wide, high, out, stride);
            }
        }
    }

    /// The residual of the transform block of side `tx` at `(px, py)` of
    /// `plane`, the source less the prediction in `recon` there, transformed
    /// into `coeffs`. Past the picture's edge the source repeats its last
    /// pixel.
    fn transform(&mut self, plane: usize, px: usize, py: usize, tx: usize, tx_type: TxType) {
        let (stride, lx, ly) = (self.contexts.stride, px - self.contexts.x, py - self.top * 8);
        let (src, recon) = (self.source, &self.contexts.planes[plane]);
        let (residual, area) = (&mut self.residual, tx * tx);
        if px + tx <= src.width && py + tx <= src.height {
            pixels::subtract(tx, &src.planes[plane][py * src.stride + px..], src.stride, &recon[ly * stride + lx..], stride, &mut residual[..area]);
        } else {
            for r in 0..tx {
                let srow = src.row(plane, py + r);
                let prow = &recon[(ly + r) * stride + lx..][..tx];
                for c in 0..tx {
                    residual[r * tx + c] = i32::from(srow[(px + c).min(src.width - 1)]) - i32::from(prow[c]);
                }
            }
        }
        transform::forward(tx, tx_type, &residual[..area], &mut self.coeffs[..area]);
    }

    /// Code and reconstruct the block's transform blocks of size `tx`: an
    /// intra block's each predicted from the ones before it, its levels kept
    /// to be written; an inter block's levels as `trial` left them, added to
    /// its prediction; an unchanged block's prediction as it is.
    fn reconstruct(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode, tx: usize) {
        let g = self.geometry;
        let (tx_log2, tx) = (tx, 4 << tx);
        let (stride, x0, y0) = (self.contexts.stride, self.contexts.x, self.top * 8);
        let area = tx * tx;
        if mode == Mode::Unchanged {
            // A superblock the frame does not touch is the reference's where
            // it is, and nothing of this frame reads it here.
            let previous = self.previous.as_ref().expect("an inter frame");
            if previous.touched[(mi_row >> SB_LOG2) * g.sb_cols + (mi_col >> SB_LOG2)] {
                self.predict_inter(mi_row, mi_col, n, Mv::ZERO);
            }
            self.nothing(mi_row, mi_col, n, tx_log2);
            return;
        }
        for plane in 0..3 {
            // All but an inter block's luma, which `trial` left.
            let fresh = matches!(mode, Mode::Intra(_)) || plane > 0;
            if fresh {
                self.eobs[plane].clear();
            }
            let tx_type = Self::tx_type(mode, plane, tx_log2);
            let scan = tokens::scan(tx_log2, tx_type);
            for (i, (x, y)) in self.transform_blocks(mi_row, mi_col, n, tx_log2).1.enumerate() {
                let (px, py) = (mi_col * 8 + x, mi_row * 8 + y);
                // Where it is in the row's pixels.
                let (lx, ly) = (px - x0, py - y0);
                let eob = if fresh {
                    if let Mode::Intra(intra) = mode {
                        let edges = Edges { width: g.mi_cols * 8 - x0, height: g.mi_rows * 8 - py, left: x > 0 || mi_col > self.mi_col_start };
                        let (before, rows) = self.contexts.planes[plane].split_at_mut(ly * stride);
                        let above = match ly {
                            0 => (mi_row > 0).then_some(&self.above_pixels[plane][..]),
                            ly => Some(&before[(ly - 1) * stride..]),
                        };
                        predict::intra(intra, above, rows, stride, lx, tx, &edges);
                    }
                    self.transform(plane, px, py, tx, tx_type);
                    let levels = &mut self.levels[plane][i * area..][..area];
                    let eob = self.quantizer.quantize(tx, scan.inverse, &self.coeffs[..area], levels);
                    self.eobs[plane].push(eob);
                    eob
                } else {
                    self.eobs[plane][i]
                };
                if eob > 0 {
                    let levels = &self.levels[plane][i * area..][..area];
                    self.quantizer.dequantize(tx, levels, scan.order, eob, &mut self.coeffs[..area]);
                    transform::inverse_add(tx, tx_type, &self.coeffs[..area], eob, &mut self.contexts.planes[plane][ly * stride + lx..], stride);
                }
            }
        }
    }

    /// Write the tokens of every transform block of a block that is not
    /// skipped, keeping the contexts a decoder keeps (libvpx's
    /// `vp9_decode_block_tokens`): a transform block's context is whether
    /// any 4×4 column above it or row left of it had a coefficient, and it
    /// leaves its own answer behind over the columns and rows inside the
    /// frame, and none past it.
    fn tokens(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode, tx: usize) {
        let g = self.geometry;
        let (tx_log2, tx) = (tx, 4 << tx);
        let n4 = tx / 4;
        let area = tx * tx;
        // In 4×4 units, from the block's corner, how far the frame reaches.
        let wide4 = (2 << n).min((g.mi_cols - mi_col) * 2);
        let high4 = (2 << n).min((g.mi_rows - mi_row) * 2);
        let inter = !matches!(mode, Mode::Intra(_));
        for plane in 0..3 {
            let tx_type = Self::tx_type(mode, plane, tx_log2);
            for (i, (x, y)) in self.transform_blocks(mi_row, mi_col, n, tx_log2).1.enumerate() {
                let (col4, row4) = (x / 4, y / 4);
                let ax = (mi_col - self.mi_col_start) * 2 + col4;
                let ly = ((mi_row * 2) & 15) + row4;
                let above = self.above_nonzero[plane][ax..ax + n4].iter().any(|&a| a != 0);
                let left = self.left_nonzero[plane][ly..ly + n4].iter().any(|&l| l != 0);
                let eob = self.eobs[plane][i];
                let kind = tokens::Kind { tx: tx_log2, tx_type, chroma: plane > 0, inter };
                let ctx = usize::from(above) + usize::from(left);
                tokens::write(self.contexts.tape, kind, ctx, &self.levels[plane][i * area..][..area], eob, &mut self.energy);
                let coded = u8::from(eob > 0);
                for k in 0..n4 {
                    self.above_nonzero[plane][ax + k] = if col4 + k < wide4 { coded } else { 0 };
                    self.left_nonzero[plane][ly + k] = if row4 + k < high4 { coded } else { 0 };
                }
            }
        }
    }
}
