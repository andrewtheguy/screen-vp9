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
use super::probs::{Counts, inter_mode_slot, intra_inter_slot, mv_slot, partition_slot, single_ref_slot, skip_slot, tx_slot, y_mode_slot};
use super::quantize::{self, Quantizer};
use super::tape::Tape;
use super::transform::TxType;
use super::tables::{
    INTER_MODE_PROBS, KF_PARTITION_PROBS, KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, MV_REF_BLOCKS, UV_MODE_PROBS, Y_MODE_PROBS,
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
const SPLIT_NUM: i64 = 1;

/// The variance per pixel, summed over the planes, above which a square
/// predicted from the previous frame is split, in eighths of the AC step.
const INTER_SPLIT: i64 = 1;

/// What an intra block's modes cost beyond a vector a decoder predicts, in
/// bits: a block is coded intra where its residual's variance beats the
/// inter residual's by more than these bits are worth at `RATE_WEIGHT`.
/// By variance alone, `walk`'s frames at dials 20 to 30 coded some seven
/// hundred intra 8×8s each for 1.5–1.9× libvpx's bytes at the same picture.
const INTRA_BITS: i64 = 24;

/// What a bit is worth in squared error wherever a block's coding is chosen
/// by cost (its intra modes, its transform size, its vector), in
/// thousandths of the AC step squared over 64, a step in pixels being an
/// eighth of one in coefficients. libvpx weighs a bit at about 0.137 of
/// the DC step squared (`vp9_compute_rd_mult`, 4.35 × dc² over `RDDIV`
/// and the error's shift), 0.07 to 0.10 of the AC step squared across the
/// dials, and 140 here, fitted once to bytes alone, was heavier. Measured
/// on the captures at 1920×1080 at 140, 50, 35 and 25: each halving buys
/// about 0.3 dB on a keyframe and 0.2 to 0.5 dB on the inter frames for 1
/// to 3% of the bytes, and at 35 every capture's picture is libvpx's or
/// better but `terminal`'s lines of text at −0.1 dB, at its bytes or
/// fewer but `uncover`'s at 1.01×.
const RATE_WEIGHT: i64 = 35;

/// How many times the cost of a whole vector's prediction the whole
/// vectors around it each cost more for nothing to be looked for between
/// pixels (`refine`). The search found nothing on seven tenths of `drag`'s
/// and `browser`'s area at 2048×1536, nearly all of it where the four
/// cost more than four times as much; at 7, turn about over every cut
/// capture, `drag`, `browser` and `uncover` take 0.84 to 0.93 of the time
/// at their bytes and picture, and the rest within 0.3% of their bytes and
/// 0.03 dB. At 3 `browser` at 2048×1536 lost 0.14 dB.
const SHARP: i64 = 7;

/// A vector the search offers is taken over the vectors a decoder predicts
/// only where its residual's squares are `1 / NEW_VECTOR_GAIN` less than
/// theirs.
const NEW_VECTOR_GAIN: i64 = 4;

/// An intra mode other than DC is taken only where its residual varies
/// `1 / INTRA_MODE_GAIN` less than DC's, for the bits it costs over DC's.
const INTRA_MODE_GAIN: i64 = 16;

/// How far past an intra square's variance with its bits the best vector's
/// residual may lie for the square to be coded both ways and take the
/// cheaper (`decide`), in multiples of it: text arriving on a blank is
/// intra by any measure, and coding it inter as well was a fifth of a
/// flood of text's time. libvpx tries intra in an inter frame only where
/// the inter cost passes a penalty scaled by the quantizer
/// (`vp9_get_intra_cost_penalty`); this is that gate the other way about.
const INTRA_OVER: i64 = 2;

/// An 8×8 intra block is coded as four 4×4 blocks, each predicted in a mode
/// of its own, where the means of its luma 4×4s, each less 128, vary by
/// more than this many AC steps over 256: libvpx's keyframe threshold for
/// the split (`set_vbp_thresholds`, `thresholds[3]`, over
/// `fill_variance_4x4avg`), which is a few grey levels, and where text is.
/// At 10 the captures' keyframes at 1920×1080 are 0.1 to 0.2 dB better for
/// 1% fewer bytes in the same time, but a hatch is then in 4×4s, which
/// are not tried in the diagonal modes, and a picture that churns costs
/// more at a low dial than the quality walk can bring under a link.
const SPLIT_4X4_VARIANCE: i64 = 80;

/// The partition context a 4×4 block leaves (libvpx's
/// `partition_context_lookup[BLOCK_4X4]`).
const PARTITION_CONTEXT_4X4: u8 = 15;

/// The intra modes an 8×8 block is coded each way in, DC first, as
/// libvpx's realtime encoder tries them on a keyframe
/// (`rd_pick_intra_sby_mode`): all ten, less the steep diagonals whose
/// way the best so far does not point (`IntraMode::pointless_after`), and
/// none after a mode that codes no coefficient.
const TRIED_MODES: [IntraMode; 10] = IntraMode::ALL;

/// The intra modes a 4×4 block is coded each way in, pruned as an 8×8's
/// are: the three libvpx's realtime encoder masks a 4×4's to
/// (`intra_y_mode_mask[TX_4X4]`, `INTRA_DC_H_V`) and TM, which bought
/// 0.16 to 0.18 dB on the captures' keyframes at 1920×1080 for 3% of their
/// time. All ten, pruned, bought 0.4 to 0.5 dB for 20 to 25%.
const TRIED_MODES_4X4: [IntraMode; 4] = [IntraMode::Dc, IntraMode::V, IntraMode::H, IntraMode::Tm];

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
/// frame's and whether it is to be skipped, and per superblock the vectors
/// the motion search offers it.
#[derive(Clone, Copy)]
pub(super) struct Previous<'a> {
    pub(super) reference: &'a Reconstruction,
    /// Per 8×8 of the frame, whether its source is the previous frame's:
    /// nothing moved there, whatever the dial did.
    pub(super) still: &'a [bool],
    /// Per 8×8 of the frame, whether it is still and at the dial's
    /// quantizer already: the frame skips it.
    pub(super) unchanged: &'a [bool],
    /// Per superblock of the frame, whether the frame touches it: it or a
    /// neighbour of it may be coded. One it does not touch is left as it
    /// is, in the reference, and not written to the reconstruction.
    pub(super) touched: &'a [bool],
    pub(super) vectors: &'a [[Mv; motion::VECTORS]],
}

/// How a block is coded.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Predicted from the pixels around it, and the residual: the luma in
    /// one mode and the chroma in another.
    Intra { luma: IntraMode, chroma: IntraMode },
    /// An 8×8 as four 4×4 blocks, each predicted from the pixels around it
    /// in a mode of its own, in raster order, and the chroma in one.
    Intra4 { sub: [IntraMode; 4], chroma: IntraMode },
    /// The previous frame at a vector from where the block is, and the
    /// residual against it.
    Inter(Mv),
    /// The previous frame where the block is, as it is: the source has not
    /// changed there, or the previous frame holds it exactly, so nothing
    /// is coded but its segment.
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
    /// Per 4×4 of an 8×8, its mode: the block's for a block of one, and
    /// DC for an inter block, as a decoder takes it. A block's neighbours
    /// read the ones along their edge, which a keyframe codes their modes
    /// by (libvpx's `vp9_above_block_mode`, `vp9_left_block_mode`).
    sub: [IntraMode; 4],
    /// How an inter block's vector was coded, and the vector's row and
    /// column in eighths of a pixel; none for an intra block. A frame
    /// writes one of these for every 8×8 it holds, coded or not, so it is
    /// kept small: sixteen bytes.
    inter: Option<(InterMode, [i16; 2])>,
    /// The transform size, as log2 less 2.
    tx: u8,
    /// The side of the block it is in, in 8×8s as log2.
    n: u8,
    /// Whether the block is in the inactive segment: the previous frame as
    /// it is, which a decoder reads nothing else of and does not filter.
    inactive: bool,
    /// Whether its segment is what the last map predicted (libvpx's
    /// `seg_id_predicted`), which the blocks after it read as the context
    /// of their own prediction flag.
    predicted: bool,
}

const _: () = assert!(size_of::<ModeInfo>() == 16);

impl ModeInfo {
    /// A superblock the frame leaves as the previous frame has it, one
    /// block in the inactive segment, as the map before predicted.
    const QUIET: Self = Self { skip: true, sub: [IntraMode::Dc; 4], inter: Some((InterMode::Zero, [0, 0])), tx: 3, n: SB_LOG2 as u8, inactive: true, predicted: true };
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
    /// Per superblock row, its bools, to be coded once the frame's
    /// probabilities are chosen.
    pub(super) tapes: Vec<Tape>,
    /// Per superblock, what it leaves the superblock below it, once its row
    /// has coded it.
    edges: Vec<OnceLock<Edge>>,
    /// Per superblock, whether any block of it was coded in the active
    /// segment, so that the loop filter has an edge in it.
    coded: Vec<bool>,
    /// Per superblock, in how many of the last frames, two at most, the
    /// frame left it untouched and as the map before it predicted: its
    /// mode info in that many of the two frames kept is `ModeInfo::QUIET`.
    quiet: Vec<u8>,
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
            tapes: (0..geometry.sb_rows).map(|_| Tape::new()).collect(),
            edges: (0..geometry.sb_rows * sb_cols).map(|_| OnceLock::new()).collect(),
            coded: vec![false; geometry.sb_rows * sb_cols],
            quiet: vec![0; geometry.sb_rows * sb_cols],
        }
    }

    /// The vectors of the frame coded last's inter blocks, one per 8×8.
    #[cfg(test)]
    pub(super) fn vectors(&self) -> impl Iterator<Item = Mv> + '_ {
        self.mode_info.iter().filter_map(|m| m.inter.map(|(_, [row, col])| Mv { row: i32::from(row), col: i32::from(col) }))
    }

    /// The intra modes of the frame coded last's intra blocks, one per 8×8:
    /// of a block of four 4×4s, the last's.
    #[cfg(test)]
    pub(super) fn intra_modes(&self) -> impl Iterator<Item = IntraMode> + '_ {
        self.mode_info.iter().filter(|m| m.inter.is_none()).map(|m| m.sub[3])
    }

    /// What the loop filter reads of the block coded last at the 8×8 in row
    /// `mi_row` and the column's 8×8 column `col`.
    pub(super) fn filtered(&self, mi_row: usize, col: usize) -> loopfilter::Block {
        let m = self.mode_info[mi_row * self.width + col];
        loopfilter::Block { n: u32::from(m.n), tx: usize::from(m.tx), intra: m.inter.is_none(), skip: m.skip, inactive: m.inactive }
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
        let Contexts { mode_info, last_mode_info, width, sb_cols, use_last, tapes, edges, coded, quiet, .. } = self;
        let (width, sb_cols, use_last) = (*width, *sb_cols, *use_last);
        let (last_mode_info, edges) = (&last_mode_info[..], &edges[..]);
        let (x, stride) = (column.x, column.stride);
        let [p0, p1, p2] = &mut column.planes;
        let rows = 64 * stride;
        let planes = p0.chunks_mut(rows).zip(p1.chunks_mut(rows)).zip(p2.chunks_mut(rows));
        let rows = mode_info.chunks_mut(8 * width).zip(planes).zip(tapes.iter_mut()).zip(coded.chunks_mut(sb_cols)).zip(quiet.chunks_mut(sb_cols));
        rows.enumerate().map(move |(sb_row, ((((mode_info, ((p0, p1), p2)), tape), coded), quiet))| Job {
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
            coded,
            quiet,
        })
    }
}

/// What a superblock leaves the superblock below it once it is coded: the
/// partition contexts and whether the transform blocks along its bottom
/// edge had coefficients, the mode info of its last three rows of 8×8s, as
/// far up as a block below predicts its vectors from, and its last row of
/// pixels, which DC below it predicts from.
// Each superblock leaves one a frame, in room the column keeps: a box
// would be made and dropped for every one.
#[allow(clippy::large_enum_variant)]
enum Edge {
    Coded { partition: [u8; 8], nonzero: [[u8; 16]; 3], mode_info: [[ModeInfo; 8]; 3], pixels: [[u8; 64]; 3] },
    /// A superblock the frame does not touch, which is one block of
    /// `ModeInfo::QUIET`, its segment as the map before predicted or not:
    /// nothing under it reads its pixels, being untouched too or coding
    /// nothing.
    Quiet { predicted: bool },
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
    /// Whether the frame is coded at `Speed::Fastest`: an intra block's
    /// mode by its line sums rather than coded each way.
    pub(super) fast: bool,
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
    /// Per superblock of the row, whether a block of it is active.
    coded: &'a mut [bool],
    /// Per superblock of the row, `Contexts::quiet`.
    quiet: &'a mut [u8],
}

/// Code every superblock row of every tile column of a frame, on `threads`
/// of `pool`'s threads: each takes the next row in the order rows are
/// across the frame, and codes a superblock once the row above it in its
/// column has coded the one above it. A row reads of the rows above only
/// what they leave it, so the frame is the same on any number of threads.
/// Gives back what the frame's tapes count, which each thread keeps for
/// the rows it codes: a tape's own counts are left empty.
pub(super) fn code(pool: &Pool, threads: usize, frame: &Frame, contexts: &mut [Contexts], columns: &mut [Column]) -> Counts {
    let mut columns: Vec<_> = contexts.iter_mut().zip(columns).map(|(contexts, column)| contexts.jobs(column)).collect();
    let mut queue = Vec::new();
    for _ in 0..frame.geometry.sb_rows {
        queue.extend(columns.iter_mut().map(|jobs| Mutex::new(jobs.next())));
    }
    drop(columns);
    let (next, failed) = (AtomicUsize::new(0), AtomicBool::new(false));
    let counts = Mutex::new(Counts::new());
    let work = || {
        let _failed = Failed(&failed);
        let mut scratch = Scratch::new();
        while let Some(job) = queue.get(next.fetch_add(1, Ordering::Relaxed)) {
            let job = job.lock().expect("a job taken whole").take().expect("a row for every job");
            scratch = Tile::new(frame, job, scratch, &failed).code();
        }
        counts.lock().unwrap_or_else(std::sync::PoisonError::into_inner).add(&scratch.counts);
    };
    pool.run(threads.min(queue.len()), |_| work());
    counts.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner)
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

/// The `levels` of a transform block of side `side` and `kind`, which
/// quantizing left at `error` from its `coeffs`, kept where they are worth
/// their bits and cleared where coding nothing of the block costs less:
/// the levels' bits at the frame's starting probabilities, and the end of
/// block of the levels kept, and the error they leave, a bit weighed at
/// `RATE_WEIGHT`. libvpx's realtime encoder skips a block whose residual
/// costs more than it saves (`vp9_pick_inter_mode`); a settle's residual,
/// the previous frame's quantization error, leaves a lone level deep in
/// the scan of many a block, a hundred zero tokens for one coefficient.
#[allow(clippy::too_many_arguments)]
fn worth_coding(costs: &tokens::Costs, map: &mut [u8; 1024], quantizer: &Quantizer, side: usize, kind: tokens::Kind, coeffs: &[i16], levels: &mut [i16], eob: usize, error: i64) -> (usize, u32, i64) {
    let bits = tokens::estimate(costs, kind, 0, levels, eob, map);
    if eob == 0 {
        return (0, bits, error);
    }
    let ac = i64::from(quantizer.ac_step());
    let weight = RATE_WEIGHT * ac * ac;
    let nothing = tokens::estimate(costs, kind, 0, &[], 0, map);
    let left = Quantizer::energy(side, coeffs);
    if left * 256_000 + weight * i64::from(nothing) <= error * 256_000 + weight * i64::from(bits) {
        levels.fill(0);
        (0, nothing, left)
    } else {
        (eob, bits, error)
    }
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
    levels: [Vec<i16>; 3],
    eobs: [Vec<usize>; 3],
    best_levels: Vec<i16>,
    best_eobs: Vec<usize>,
    kept_levels: Vec<i16>,
    kept_eobs: Vec<usize>,
    patch: Vec<u8>,
    planes: [Vec<u8>; 16],
    predicted: Box<[u8; 64 * 64]>,
    residual: Box<[i16; 1024]>,
    coeffs: Box<[i16; 1024]>,
    dequantized: Box<[i32; 1024]>,
    map: Box<[u8; 1024]>,
    /// What the rows coded on the thread count, which each row's tape
    /// holds while the row is coded.
    counts: Counts,
}

impl Scratch {
    fn new() -> Self {
        Self {
            moved: Vec::new(),
            levels: std::array::from_fn(|_| vec![0; 64 * 64]),
            eobs: std::array::from_fn(|_| Vec::with_capacity(256)),
            best_levels: vec![0; 64 * 64],
            best_eobs: Vec::with_capacity(256),
            kept_levels: vec![0; 64 * 64],
            kept_eobs: Vec::with_capacity(256),
            patch: Vec::new(),
            planes: std::array::from_fn(|_| Vec::new()),
            predicted: Box::new([0; 64 * 64]),
            residual: Box::new([0; 1024]),
            coeffs: Box::new([0; 1024]),
            dequantized: Box::new([0; 1024]),
            map: Box::new([0; 1024]),
            counts: Counts::new(),
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
    fast: bool,
    /// Whether a thread coding a row above has failed.
    failed: &'a AtomicBool,
    /// The variance a square is split above, `SPLIT_VARIANCE` of the step.
    split_variance: i64,
    /// The variance an inter square is split above, `INTER_SPLIT` eighths
    /// of the step.
    inter_split_variance: i64,
    /// The superblock being coded, per 8×8 in raster order.
    measures: [Measure; 64],
    /// Per 8×8 of the superblock and per plane, the sums of its source's
    /// rows and of its columns, once an intra mode's residual has asked for
    /// them, and which are there yet.
    line_sums: [[([u16; 8], [u16; 8]); 3]; 64],
    summed: u64,
    /// The superblock being coded, per vector other than zero measured at
    /// so far: per 8×8, the moments of its residual at that vector, and
    /// which of them are measured yet.
    moved: Vec<(Mv, [Moments; 64], u64)>,
    /// The block being coded: per plane, the levels of its transform blocks
    /// one after another, each in raster order, and their ends of block.
    levels: [Vec<i16>; 3],
    eobs: [Vec<usize>; 3],
    /// The luma levels and ends of block of the best transform size tried so
    /// far.
    best_levels: Vec<i16>,
    best_eobs: Vec<usize>,
    /// The luma levels and ends of block of the vector a block came with,
    /// kept while it is tried at the one the search between pixels found.
    kept_levels: Vec<i16>,
    kept_eobs: Vec<usize>,
    /// Scratch for a block of the previous frame copied out past its edges,
    /// for it filtered across at each sixteenth, and for a prediction
    /// filtered from it.
    patch: Vec<u8>,
    planes: [Vec<u8>; 16],
    predicted: Box<[u8; 64 * 64]>,
    /// Scratch for a transform block's residual and coefficients, for the
    /// coefficients a decoder makes of its levels, and for the token coder.
    residual: Box<[i16; 1024]>,
    coeffs: Box<[i16; 1024]>,
    dequantized: Box<[i32; 1024]>,
    map: Box<[u8; 1024]>,
    /// The tape's own counts, empty, while the tape holds the thread's.
    spare: Counts,
}

impl<'a> Tile<'a> {
    /// The coder of the superblock row `job`, working in `scratch`.
    fn new(frame: &Frame<'a>, job: Job<'a>, scratch: Scratch, failed: &'a AtomicBool) -> Self {
        let Scratch { mut moved, levels, eobs, best_levels, best_eobs, kept_levels, kept_eobs, patch, planes, predicted, residual, coeffs, dequantized, map, counts: mut spare } = scratch;
        moved.clear();
        std::mem::swap(&mut job.tape.counts, &mut spare);
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
            fast: frame.fast,
            failed,
            split_variance: i64::from(frame.quantizer.ac_step()) * SPLIT_NUM / SPLIT_VARIANCE,
            inter_split_variance: i64::from(frame.quantizer.ac_step()) * INTER_SPLIT / 8,
            measures: [Measure::default(); 64],
            line_sums: [[([0; 8], [0; 8]); 3]; 64],
            summed: 0,
            moved,
            levels,
            eobs,
            best_levels,
            best_eobs,
            kept_levels,
            kept_eobs,
            patch,
            planes,
            predicted,
            residual,
            coeffs,
            dequantized,
            map,
            spare,
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
            if self.untouched(mi_row, mi_col) {
                self.quiet(mi_row, mi_col, sb_col);
                continue;
            }
            self.contexts.quiet[sb_col] = 0;
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
        std::mem::swap(&mut self.contexts.tape.counts, &mut self.spare);
        let Tile { moved, levels, eobs, best_levels, best_eobs, kept_levels, kept_eobs, patch, planes, predicted, residual, coeffs, dequantized, map, spare: counts, .. } = self;
        Scratch { moved, levels, eobs, best_levels, best_eobs, kept_levels, kept_eobs, patch, planes, predicted, residual, coeffs, dequantized, map, counts }
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
        match edge {
            Edge::Coded { partition, nonzero, mode_info, pixels } => {
                self.above_partition[x..x + 8].copy_from_slice(partition);
                for plane in 0..3 {
                    self.above_nonzero[plane][2 * x..2 * x + 16].copy_from_slice(&nonzero[plane]);
                    self.above_pixels[plane][8 * x..8 * x + 64].copy_from_slice(&pixels[plane]);
                }
                for (r, row) in mode_info.iter().enumerate() {
                    self.above_mode_info[r * w + x..][..cols].copy_from_slice(&row[..cols]);
                }
            }
            &Edge::Quiet { predicted } => {
                self.above_partition[x..x + 8].fill(partition_context_of(SB_LOG2));
                for plane in 0..3 {
                    self.above_nonzero[plane][2 * x..2 * x + 16].fill(0);
                }
                for r in 0..3 {
                    self.above_mode_info[r * w + x..][..cols].fill(ModeInfo { predicted, ..ModeInfo::QUIET });
                }
            }
        }
    }

    /// Whether the frame leaves the superblock at `(mi_row, mi_col)`
    /// untouched and it is one block: it is not split by the frame's edge.
    fn untouched(&self, mi_row: usize, mi_col: usize) -> bool {
        let g = self.geometry;
        let half = 1 << (SB_LOG2 - 1);
        self.previous.as_ref().is_some_and(|previous| !previous.touched[(mi_row >> SB_LOG2) * g.sb_cols + (mi_col >> SB_LOG2)]) && mi_row + half < g.mi_rows && mi_col + half < g.mi_cols
    }

    /// Code superblock `sb_col` of the row, at `(mi_row, mi_col)`, which
    /// the frame leaves untouched: what `partition` and `block` record and
    /// leave of one whole block that is `Mode::Unchanged`, without the
    /// work of finding that out. Its mode info is `ModeInfo::QUIET` frame
    /// after frame, and is not written again where both of the frames
    /// kept hold it already.
    fn quiet(&mut self, mi_row: usize, mi_col: usize, sb_col: usize) {
        let g = self.geometry;
        let (w, col) = (self.contexts.width, mi_col - self.mi_col_start);
        let (rows, cols) = (mi_row..(mi_row + 8).min(g.mi_rows), 8.min(w - col));
        let ctx = self.partition_context(mi_row, mi_col, SB_LOG2);
        self.contexts.tape.put(false, partition_slot(ctx));
        let above = (mi_row > 0).then(|| self.mode_info(mi_row - 1, mi_col));
        let left = (mi_col > self.mi_col_start).then(|| self.mode_info(mi_row, mi_col - 1));
        let before = self.contexts.quiet[sb_col];
        let last = self.contexts.last_mode_info;
        let predicted = before > 0 || rows.clone().all(|row| last[row * w + col..][..cols].iter().all(|m| m.inactive));
        let ctx = usize::from(above.is_some_and(|m| m.predicted)) + usize::from(left.is_some_and(|m| m.predicted));
        self.contexts.tape.segment(true, predicted, ctx);
        if !(predicted && before == 2) {
            for row in rows {
                self.contexts.mode_info[(row - self.top) * w + col..][..cols].fill(ModeInfo { predicted, ..ModeInfo::QUIET });
            }
        }
        self.contexts.quiet[sb_col] = if predicted { (before + 1).min(2) } else { 0 };
        let x = col;
        self.above_partition[x..x + 8].fill(partition_context_of(SB_LOG2));
        self.left_partition.fill(partition_context_of(SB_LOG2));
        for plane in 0..3 {
            self.above_nonzero[plane][2 * x..2 * x + 16].fill(0);
            self.left_nonzero[plane].fill(0);
        }
        if mi_row + 8 < g.mi_rows {
            let left = self.contexts.edges[self.top / 8 * self.contexts.sb_cols + sb_col].set(Edge::Quiet { predicted });
            assert!(left.is_ok(), "a superblock's edge left once");
        }
    }

    /// Leave the row below what superblock `sb_col` of the row, coded,
    /// leaves it.
    fn leave_edge(&self, sb_col: usize) {
        let (w, x, stride) = (self.contexts.width, sb_col * 8, self.contexts.stride);
        let cols = (w - x).min(8);
        let mut mode_info = [[ModeInfo::default(); 8]; 3];
        for (r, row) in mode_info.iter_mut().enumerate() {
            row[..cols].copy_from_slice(&self.contexts.mode_info[(5 + r) * w + x..][..cols]);
        }
        let edge = Edge::Coded {
            partition: self.above_partition[x..x + 8].try_into().expect("a superblock's"),
            nonzero: std::array::from_fn(|plane| self.above_nonzero[plane][2 * x..2 * x + 16].try_into().expect("a superblock's")),
            mode_info,
            pixels: std::array::from_fn(|plane| self.contexts.planes[plane][63 * stride + 8 * x..][..64].try_into().expect("a superblock's")),
        };
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
    /// `(mi_row, mi_col)` that the frame holds are the previous frame's at
    /// the dial's quantizer, which the frame skips.
    fn count_unchanged(&self, mi_row: usize, mi_col: usize, n: u32) -> Option<usize> {
        self.count_marked(mi_row, mi_col, n, |previous| previous.unchanged)
    }

    /// Whether, in an inter frame, the source of the square of `1 << n`
    /// 8×8s at `(mi_row, mi_col)` is the previous frame's wherever the frame
    /// holds it, at whatever dial: nothing moved there.
    fn still(&self, mi_row: usize, mi_col: usize, n: u32) -> bool {
        self.count_marked(mi_row, mi_col, n, |previous| previous.still) == Some(self.count_inside(mi_row, mi_col, n))
    }

    /// In an inter frame, how many of the 8×8s of the square of `1 << n` at
    /// `(mi_row, mi_col)` that the frame holds are marked in the mask
    /// `of` picks from the previous frame.
    fn count_marked(&self, mi_row: usize, mi_col: usize, n: u32, of: impl Fn(&Previous<'a>) -> &'a [bool]) -> Option<usize> {
        let mask = of(self.previous.as_ref()?);
        let mi_cols = self.geometry.mi_cols;
        let (rows, cols) = self.inside(mi_row, mi_col, n);
        let (row0, col0) = (mi_row - (mi_row & 7), mi_col - (mi_col & 7));
        Some(rows.into_iter().map(|r| mask[(row0 + r) * mi_cols + col0..][cols.clone()].iter().filter(|&&u| u).count()).sum())
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
        // What the previous frame holds exactly has nothing to measure:
        // its residual is nothing, and that is all `choose` reads of it.
        if self.exact(mi_row, mi_col) {
            self.measures = [Measure::default(); 64];
            return;
        }
        let (rows, cols) = self.inside(mi_row, mi_col, SB_LOG2);
        for r in rows {
            let py = (mi_row + r) * 8;
            // The residuals four 8×8s at a time, as far as the row goes.
            let mut residuals = [Moments::default(); 8];
            if self.previous.is_some() {
                let mut c = cols.start;
                while c < cols.end {
                    if c + 4 <= cols.end {
                        residuals[c..c + 4].copy_from_slice(&self.residual_moments4((mi_col + c) * 8, py, Mv::ZERO));
                        c += 4;
                    } else {
                        residuals[c] = self.residual_moments((mi_col + c) * 8, py, Mv::ZERO);
                        c += 1;
                    }
                }
            }
            for c in cols.clone() {
                let source = self.source_moments((mi_col + c) * 8, py);
                self.measures[r * 8 + c] = Measure { source, residual: residuals[c] };
            }
        }
    }

    /// Whether the previous frame holds the superblock at `(mi_row,
    /// mi_col)` to the pixel, as far as the picture holds it: past the
    /// picture's edge the source and a decoder's reference each repeat
    /// their last pixel, so what is the same inside is the same there.
    fn exact(&self, mi_row: usize, mi_col: usize) -> bool {
        let Some(previous) = &self.previous else { return false };
        let src = self.source;
        let (px, py) = (mi_col * 8, mi_row * 8);
        let (w, h) = (64.min(src.width - px), 64.min(src.height - py));
        let Some(column) = previous.reference.holding(px, w) else { return false };
        (0..3).all(|plane| (py..py + h).all(|y| src.planes[plane][y * src.stride + px..][..w] == *column.row(plane, y, px, w)))
    }

    /// Whether the means of the four luma 4×4s of the 8×8 at `(mi_row,
    /// mi_col)` vary by more than `SPLIT_4X4_VARIANCE` (libvpx's
    /// `get_variance` over `fill_variance_4x4avg`, which takes each 4×4 as
    /// its rounded mean less 128).
    fn quarters_vary(&self, mi_row: usize, mi_col: usize) -> bool {
        let src = self.source;
        let (px, py) = (mi_col * 8, mi_row * 8);
        let mut sums = [0i64; 4];
        for r in 0..8 {
            let row = eight(src.row(0, py + r), px);
            for (c, &v) in row.iter().enumerate() {
                sums[(r / 4) * 2 + c / 4] += i64::from(v);
            }
        }
        let (mut se, mut sse) = (0i64, 0i64);
        for s in sums {
            let e = ((s + 8) >> 4) - 128;
            se += e;
            sse += e * e;
        }
        (256 * (sse - ((se * se) >> 2))) >> 2 > SPLIT_4X4_VARIANCE * i64::from(self.quantizer.ac_step())
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

    /// `residual_moments` of the four 8×8s side by side from `(px, py)`:
    /// the previous frame read once for the four, and their differences
    /// taken a row of the four at a time, where the picture holds them all.
    fn residual_moments4(&mut self, px: usize, py: usize, mv: Mv) -> [Moments; 4] {
        let src = self.source;
        if px + 32 > src.width || py + 8 > src.height {
            return std::array::from_fn(|i| self.residual_moments(px + 8 * i, py, mv));
        }
        let reference = self.previous.as_ref().expect("an inter frame").reference;
        let ((dx, dy), _) = mv.split();
        let mut four = [Moments::default(); 4];
        for plane in 0..3 {
            let (prediction, stride) = reference.block(plane, (px as isize + dx, py as isize + dy), (32, 8), (src.width, src.height), &mut self.patch);
            let moments = pixels::difference_moments4(&src.planes[plane][py * src.stride + px..], src.stride, prediction, stride);
            for (m, (sum, squares)) in four.iter_mut().zip(moments) {
                m.sums[plane] = i64::from(sum);
                m.squares += i64::from(squares);
            }
        }
        four
    }

    /// The moments of the square of `1 << n` 8×8s at `(mi_row, mi_col)`, as
    /// far as the frame holds it, less the previous frame at `mv`, a vector
    /// other than zero, from it: at its nearest whole pixels, which measure
    /// a vector between them for nothing and nearly as well as filtering
    /// would (filtering found 0.3–0.6% fewer bytes on the first plan's
    /// recordings of video for a sixth more time). Each 8×8 is measured at a vector once per
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
            let mut c = cols.start;
            while c < cols.end {
                let i = r * 8 + c;
                // Four 8×8s at a time where the row has four not yet
                // measured.
                let four = c + 4 <= cols.end && self.moved[at].2 >> i & 0xf == 0;
                if four {
                    let m = self.residual_moments4((col0 + c) * 8, (row0 + r) * 8, mv);
                    let (_, moments, measured) = &mut self.moved[at];
                    moments[i..i + 4].copy_from_slice(&m);
                    *measured |= 0xf << i;
                } else if self.moved[at].2 & 1 << i == 0 {
                    let m = self.residual_moments((col0 + c) * 8, (row0 + r) * 8, mv);
                    let (_, moments, measured) = &mut self.moved[at];
                    moments[i] = m;
                    *measured |= 1 << i;
                }
                let n = if four { 4 } else { 1 };
                for m in &self.moved[at].1[i..i + n] {
                    total.add(m);
                }
                c += n;
            }
        }
        total
    }

    /// How the square of `1 << n` 8×8s at `(mi_row, mi_col)` is coded
    /// whole, and the moments of its source less that prediction (of the
    /// source alone for DC), or none when it varies more from its best
    /// prediction than a square is coded whole at.
    fn choose(&mut self, mi_row: usize, mi_col: usize, n: u32) -> Option<(Mode, Moments, Option<Mv>)> {
        let partly = match self.count_unchanged(mi_row, mi_col, n) {
            Some(u) if u == self.count_inside(mi_row, mi_col, n) => return Some((Mode::Unchanged, Moments::default(), None)),
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
        // A square the previous frame holds exactly is the previous frame's
        // at any dial: there is no finer to code it, so it is left as the
        // blocks the frame skips are, in the inactive segment and
        // unfiltered. A settle of a quiet desktop is mostly these.
        if self.previous.is_some() && total.residual.squares == 0 {
            return Some((Mode::Unchanged, Moments::default(), None));
        }
        let dc = total.source.variance(count);
        // A square of one value a plane, which DC predicts to the pixel from
        // what the frame has coded around it, has nothing to code that way,
        // where the previous frame would have a coefficient a transform
        // block to make up what it is off by: a flat desktop a keyframe at a
        // coarse dial left a value off in one plane settles in these, block
        // after block, each exact for the next.
        if self.previous.is_some() && !partly && dc == 0 && self.still(mi_row, mi_col, n) && self.dc_exact(mi_row, mi_col, n) {
            return Some((Mode::Intra { luma: IntraMode::Dc, chroma: IntraMode::Dc }, total.source, None));
        }
        // An intra block's modes, weighed as the variances are: per pixel
        // squared over the count, times the count again.
        let ac = i64::from(self.quantizer.ac_step());
        let intra_bits = RATE_WEIGHT * ac * ac * INTRA_BITS * count / 64_000;
        // With the vector an intra square was chosen over, where there is
        // one, for `block` to code it both ways.
        let (mode, variance, moments, over) = match &self.previous {
            // A square whose source is the previous frame's throughout, at
            // another dial, settles: the previous frame where it is and the
            // residual, which is that frame's quantization error, with
            // nothing searched; an intra mode where DC's residual is
            // smaller, which from a coarse dial it is where the neighbours
            // have settled already.
            Some(_) if self.still(mi_row, mi_col, n) => {
                let residual = total.residual.variance(count);
                let (intra, variance, moments) = if dc < residual { self.intra_mode(mi_row, mi_col, n, &total.source, count) } else { (IntraMode::Dc, dc, total.source) };
                if residual <= variance + intra_bits { (Mode::Inter(Mv::ZERO), residual, total.residual, None) } else { (Mode::Intra { luma: intra, chroma: intra }, variance, moments, None) }
            }
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
                if best.1 <= variance + intra_bits {
                    (Mode::Inter(best.0), best.1, best.2, None)
                } else {
                    // A 64×64 is never coded inter, below; a square the
                    // vector leaves far from intra's residual is intra.
                    let near = best.1 <= INTRA_OVER * (variance + intra_bits);
                    (Mode::Intra { luma: intra, chroma: intra }, variance, moments, (n < 3 && near).then_some(best.0))
                }
            }
            None => {
                let (intra, variance, moments) = self.intra_mode(mi_row, mi_col, n, &total.source, count);
                (Mode::Intra { luma: intra, chroma: intra }, variance, moments, None)
            }
        };
        // A square that is unchanged in part is split unless what moved
        // into it is coded as moving, so the unchanged part stays the
        // previous frame's rather than coding again what the previous frame
        // got wrong there. What moved takes the unchanged part with it: it
        // is as likely to be where the motion left it as where it was.
        if partly && matches!(mode, Mode::Intra { .. } | Mode::Inter(Mv::ZERO)) {
            return None;
        }
        // A 64×64 is predicted from the previous frame in its four 32×32s,
        // each with a vector of its own: on the shader animations of the
        // quiet samples, whose light moves over a pattern that stays, one
        // vector for the whole cost 1–2% of their bytes.
        if n == 3 && matches!(mode, Mode::Inter(_)) {
            return None;
        }
        // An 8×8 intra block of text, coded each way, is four 4×4s.
        let mode = match mode {
            Mode::Intra { .. } if n == 0 && !self.fast && self.quarters_vary(mi_row, mi_col) => Mode::Intra4 { sub: [IntraMode::Dc; 4], chroma: IntraMode::Dc },
            mode => mode,
        };
        let split_variance = if matches!(mode, Mode::Inter(_)) { self.inter_split_variance } else { self.split_variance };
        (n == 0 || variance <= split_variance * count * count).then_some((mode, moments, over))
    }

    /// Whether DC predicts the square of `1 << n` 8×8s at `(mi_row,
    /// mi_col)`, whose source is one value a plane, to the pixel: the pixels
    /// coded above it and to its left, as far as the frame holds the square,
    /// are that value in every plane. A transform block inside the square
    /// is then predicted from the same, and past the frame's edge a decoder
    /// repeats the last pixel inside it. A square with neither is predicted
    /// 128 and left to the other modes.
    fn dc_exact(&self, mi_row: usize, mi_col: usize, n: u32) -> bool {
        let (up, left) = (mi_row > 0, mi_col > self.mi_col_start);
        if !up && !left {
            return false;
        }
        let (rows, cols) = self.inside(mi_row, mi_col, n);
        let (w, h) = (cols.len() * 8, rows.len() * 8);
        let (stride, lx, ly) = (self.contexts.stride, mi_col * 8 - self.contexts.x, (mi_row - self.top) * 8);
        (0..3).all(|plane| {
            let value = self.source.row(plane, mi_row * 8)[mi_col * 8];
            let coded = &self.contexts.planes[plane];
            let above = match ly {
                0 => &self.above_pixels[plane][lx..lx + w],
                ly => &coded[(ly - 1) * stride + lx..][..w],
            };
            (!up || above.iter().all(|&p| p == value)) && (!left || (0..h).all(|r| coded[(ly + r) * stride + lx - 1] == value))
        })
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
        // The sums of the square's rows and of its columns, per plane: a
        // line of 64 pixels sums inside 16 bits.
        let (mut colsum, mut rowsum) = ([[0u16; 64]; 3], [[0u16; 64]; 3]);
        for r in rows.clone() {
            for c in cols.clone() {
                let sums = self.line_sums(row0 + r, col0 + c);
                let (y, x) = ((r - rows.start) * 8, (c - cols.start) * 8);
                for (plane, (row_sums, col_sums)) in sums.iter().enumerate() {
                    for k in 0..8 {
                        rowsum[plane][y + k] += row_sums[k];
                        colsum[plane][x + k] += col_sums[k];
                    }
                }
            }
        }
        // Per plane the residual's sum, and the sum of its squares over the
        // planes, each per mode: V, H and TM.
        let mut sums = [[0i64; 3]; 3];
        let mut squares = [source.squares; 3];
        // A line's pixels with its sums: the pixels' sum, their squares'
        // and their products with the sums, none of them past 32 bits.
        let products = |line: &[u8], of: &[u16]| {
            let sums = line.iter().zip(of).fold((0u32, 0u32, 0u32), |(sum, squares, products), (&v, &of)| {
                let v = u32::from(v);
                (sum + v, squares + v * v, products + v * u32::from(of))
            });
            (i64::from(sums.0), i64::from(sums.1), i64::from(sums.2))
        };
        for (plane, sums) in sums.iter_mut().enumerate() {
            // The row above the square, past the picture's edge its last
            // pixel, and the column left of it.
            let (mut above, mut beside) = ([127u8; 64], [129u8; 64]);
            if up {
                let row = src.row(plane, py - 1);
                let inside = w.min(src.width - px);
                above[..inside].copy_from_slice(&row[px..px + inside]);
                above[inside..w].fill(row[src.width - 1]);
            }
            if left {
                for (r, beside) in beside[..h].iter_mut().enumerate() {
                    *beside = src.row(plane, py + r)[px - 1];
                }
            }
            let corner = match (up, left) {
                (true, true) => i64::from(src.row(plane, py - 1)[px - 1]),
                (true, false) => 129,
                (false, _) => 127,
            };
            let (a, a2, ac) = products(&above[..w], &colsum[plane][..w]);
            let (l, l2, lr) = products(&beside[..h], &rowsum[plane][..h]);
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
    fn line_sums(&mut self, mi_row: usize, mi_col: usize) -> &[([u16; 8], [u16; 8]); 3] {
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
        &self.line_sums[i]
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
        let chosen = if has_rows && has_cols { self.choose(mi_row, mi_col, n) } else { None };
        // Decided before the partition is written: four 4×4s are a split.
        let mode = chosen.map(|(mode, moments, over)| self.decide(mi_row, mi_col, n, mode, &moments, over));
        let split = mode.is_none_or(|(mode, ..)| matches!(mode, Mode::Intra4 { .. }));
        let ctx = self.partition_context(mi_row, mi_col, n);
        // An inter frame's at the probabilities it updates, a keyframe's at
        // its own.
        let inter = self.previous.is_some();
        let tape = &mut self.contexts.tape;
        let mut put = |bit: bool, node: usize| if inter { tape.put(bit, partition_slot(ctx) + node) } else { tape.write(bit, KF_PARTITION_PROBS[ctx][node]) };
        match (has_rows, has_cols) {
            // PARTITION_NONE is the tree's first leaf; PARTITION_SPLIT its last.
            (true, true) => {
                put(split, 0);
                if split {
                    put(true, 1);
                    put(true, 2);
                }
            }
            // Split, or a horizontal half the frame cuts.
            (false, true) => put(split, 1),
            // Split, or a vertical half the frame cuts.
            (true, false) => put(split, 2),
            // Split, and the decoder knows it.
            (false, false) => {}
        }
        match mode {
            None => {
                for (r, c) in [(0, 0), (0, half), (half, 0), (half, half)] {
                    self.partition(mi_row + r, mi_col + c, n - 1);
                }
            }
            Some((mode, tx, decided)) => {
                self.block(mi_row, mi_col, n, mode, tx, decided);
                let bw = 1 << n;
                let ctx = if matches!(mode, Mode::Intra4 { .. }) { PARTITION_CONTEXT_4X4 } else { partition_context_of(n) };
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
        let mut found = self.neighbours(mi_row, mi_col, n, MV_REF_BLOCKS[0].len()).chain(last).filter_map(|m| m.inter.map(|(_, [row, col])| Mv { row: i32::from(row), col: i32::from(col) }));
        let nearest = found.next().unwrap_or(Mv::ZERO);
        let near = found.find(|&mv| mv != nearest).unwrap_or(Mv::ZERO);
        [nearest, near].map(|mv| mv.clamp((mi_row, mi_col), 1 << n, (g.mi_rows, g.mi_cols)))
    }

    /// The transform size, as log2 less 2, of a block of `1 << n` 8×8s
    /// coded intra, whose source has `moments` over its `count` pixels: 4×4
    /// in an 8×8 whose pixels deviate by more than two steps, where a mode
    /// predicted from nearer pixels does better, and otherwise the largest
    /// that fits. Measured on the captures' keyframes at 1920×1080: at dial
    /// 40 the 4×4s are never taken, at dial 90 they save `walk`'s keyframe
    /// 3% of its bytes and 0.15 dB and `browser`'s a little; the same rule
    /// on a 16×16 changed nothing. Held to 16×16, as libvpx's keyframe
    /// (`ALLOW_16X16`), a flat ground a level off stays off: at dial 40 a
    /// 16×16's DC moves its mean two grey levels and a 32×32's, at half
    /// the step, one, and `terminal`'s keyframe, its ground 47 coded 48
    /// from the first block on, was 6 dB of luma under libvpx's.
    fn intra_transform_size(&self, n: u32, moments: &Moments, count: i64) -> usize {
        let ac = i64::from(self.quantizer.ac_step());
        // The variance per pixel and plane against two steps squared, a step
        // in pixels being an eighth of the AC step.
        if n == 0 && moments.variance(count) * 16 > ac * ac * 3 * count * count {
            0
        } else {
            (n as usize + 1).min(3)
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
    /// A transform block whose levels save less than their bits are worth
    /// codes nothing, as libvpx's realtime encoder skips a block whose
    /// residual costs more than it saves (`vp9_pick_inter_mode`): a
    /// settle's residual, the previous frame's quantization error, leaves
    /// a lone level deep in the scan of many a block, a hundred zero
    /// tokens for one coefficient. Its luma levels and ends of block are
    /// left in `levels` and `eobs`, and its cost is given with it. Chroma
    /// takes the size luma chose: deciding each plane's part of the cost
    /// too saved 0.4–3% of the bytes for a fifth more time. A size's trial
    /// stops at the transform block that takes its cost past the size
    /// above's, which ends the trial as it would have at the end.
    fn trial(&mut self, mi_row: usize, mi_col: usize, n: u32) -> (usize, i64) {
        let ac = i64::from(self.quantizer.ac_step());
        let weight = RATE_WEIGHT * ac * ac;
        let mut best = (i64::MAX, 0);
        for tx in (0..=(n as usize + 1).min(3)).rev() {
            let side = 4 << tx;
            let area = side * side;
            let kind = tokens::Kind { tx, tx_type: TxType::DctDct, chroma: false, inter: true };
            let mut cost = 0;
            self.eobs[0].clear();
            for (i, (x, y)) in self.transform_blocks(mi_row, mi_col, n, tx).1.enumerate() {
                self.transform(0, mi_col * 8 + x, mi_row * 8 + y, side, TxType::DctDct);
                let levels = &mut self.levels[0][i * area..][..area];
                let (eob, error) = self.quantizer.quantize_with_error(side, false, tokens::scan(tx, TxType::DctDct).inverse, &self.coeffs[..area], levels);
                let (eob, bits, error) = worth_coding(self.costs, &mut self.map, self.quantizer, side, kind, &self.coeffs[..area], levels, eob, error);
                // In 256000ths of the squared error on the coefficients'
                // scale, which is 64 times that in pixels.
                cost += error * 256_000 + weight * i64::from(bits);
                self.eobs[0].push(eob);
                // A size that costs more than the one above it before its
                // last transform block is coded costs more at the end.
                if cost >= best.0 {
                    break;
                }
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
        (best.1, best.0)
    }

    /// Code the transform size `tx` of the square block of `1 << n` 8×8s
    /// with `above` and `left` it (libvpx's `write_selected_tx_size`): the
    /// tree's nodes down to it, at the probabilities of its largest size
    /// and of whether the sizes above and to the left add up to more than
    /// that (`get_tx_size_context`), a skipped block's counting as the
    /// largest.
    fn write_tx(&mut self, n: u32, tx: usize, above: Option<ModeInfo>, left: Option<ModeInfo>) {
        let largest = (n as usize + 1).min(3);
        let size = |m: Option<ModeInfo>| m.map(|m| if m.skip { largest } else { usize::from(m.tx) });
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

    /// The vector and the transform size of the square inter block of
    /// `1 << n` 8×8s at `(mi_row, mi_col)` that came with `mv`, its
    /// prediction in `recon` and its luma levels in `levels`: where the
    /// search between pixels found another vector (`refine`), the block's
    /// luma is coded at both by `trial`, each with its mode's and its
    /// vector's bits at `RATE_WEIGHT`, and takes the one that costs less,
    /// whose cost it gives with it:
    /// how good a vector between pixels is shows only coded, a page moved by
    /// a fraction of a pixel saving most of a frame where text barely moves
    /// at all. A block that settles has nothing to find between the pixels.
    /// `known` is the transform size and cost of the luma coded at `mv`
    /// already, by `decide`, its levels in `levels`.
    /// The luma at the vector found is what `refine` predicted to find it;
    /// the chroma is predicted once, at the vector taken.
    fn inter(&mut self, mi_row: usize, mi_col: usize, n: u32, mv: Mv, known: Option<(usize, i64)>) -> (Mode, usize, i64) {
        let found = if self.still(mi_row, mi_col, n) { None } else { self.refine(mi_row, mi_col, n, mv) };
        let ac = i64::from(self.quantizer.ac_step());
        let prices = Prices::new(self.candidates(mi_row, mi_col, n), &INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)]);
        let with = |cost: i64, v: Mv| cost + RATE_WEIGHT * ac * ac * i64::from(prices.of(v).0);
        // The luma coded at `mv`: `trial` reads the luma alone, so the
        // chroma is predicted once, at the vector taken. With `known` the
        // levels are in `levels` but the prediction is not in `recon`.
        let (tx, cost) = match known {
            Some(known) => known,
            None => {
                self.predict_inter(mi_row, mi_col, n, mv, 0..1);
                self.trial(mi_row, mi_col, n)
            }
        };
        let Some(other) = found else {
            self.predict_inter(mi_row, mi_col, n, mv, if known.is_some() { 0..3 } else { 1..3 });
            return (Mode::Inter(mv), tx, with(cost, mv));
        };
        std::mem::swap(&mut self.levels[0], &mut self.kept_levels);
        std::mem::swap(&mut self.eobs[0], &mut self.kept_eobs);
        self.take_refined(mi_row, mi_col, n);
        let (other_tx, other_cost) = self.trial(mi_row, mi_col, n);
        if with(other_cost, other) < with(cost, mv) {
            self.predict_inter(mi_row, mi_col, n, other, 1..3);
            return (Mode::Inter(other), other_tx, with(other_cost, other));
        }
        std::mem::swap(&mut self.levels[0], &mut self.kept_levels);
        std::mem::swap(&mut self.eobs[0], &mut self.kept_eobs);
        self.predict_inter(mi_row, mi_col, n, mv, 0..3);
        (Mode::Inter(mv), tx, with(cost, mv))
    }

    /// What the square block of `1 << n` 8×8s at `(mi_row, mi_col)` chosen
    /// as `mode`, whose source less its prediction has `moments`, is coded
    /// as, before its partition is written: its mode, at the transform
    /// size `trial` finds for an inter block and `intra_transform_size`
    /// gives an intra one, and whether an intra block's luma modes are
    /// decided already. An inter block's luma is coded by this, its levels
    /// left for `block`.
    fn decide(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode, moments: &Moments, over: Option<Mv>) -> (Mode, usize, bool) {
        // A skipped inter block's transform is the largest that fits it,
        // which it does not code.
        let largest = (n as usize + 1).min(3);
        let count = 64 * self.count_inside(mi_row, mi_col, n) as i64;
        let above = (mi_row > 0).then(|| self.mode_info(mi_row - 1, mi_col));
        let left = (mi_col > self.mi_col_start).then(|| self.mode_info(mi_row, mi_col - 1));
        let (mode, tx) = match mode {
            Mode::Intra { .. } => (mode, self.intra_transform_size(n, moments, count)),
            Mode::Intra4 { .. } => (mode, 0),
            Mode::Inter(mv) => {
                let (mode, tx, _) = self.inter(mi_row, mi_col, n, mv, None);
                (mode, tx)
            }
            Mode::Unchanged => (mode, largest),
        };
        // An intra square of an inter frame chosen over a vector is coded
        // inter at that vector too, and takes it where that codes no
        // coefficient; else its luma is coded intra and it takes the
        // cheaper of the two in error and bits, each with its mode's bits.
        // The moments the choice was made on see the residual and not what
        // it costs coded: an intra block of a video is dearer coded than
        // they say, its coefficients and its 4×4s.
        match (mode, over) {
            (Mode::Intra { .. } | Mode::Intra4 { .. }, Some(mv)) => {
                // At the whole vector, without the search between pixels,
                // which is most of what an inter block of a video costs:
                // the square that takes the vector is searched then.
                let ac = i64::from(self.quantizer.ac_step());
                let prices = Prices::new(self.candidates(mi_row, mi_col, n), &INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)]);
                // `trial` reads the luma alone.
                self.predict_inter(mi_row, mi_col, n, mv, 0..1);
                let (inter_tx, trial_cost) = self.trial(mi_row, mi_col, n);
                let inter_cost = trial_cost + RATE_WEIGHT * ac * ac * i64::from(prices.of(mv).0);
                if self.eobs[0].iter().all(|&eob| eob == 0) {
                    self.predict_inter(mi_row, mi_col, n, mv, 1..3);
                    (Mode::Inter(mv), inter_tx, false)
                } else {
                    std::mem::swap(&mut self.levels[0], &mut self.kept_levels);
                    std::mem::swap(&mut self.eobs[0], &mut self.kept_eobs);
                    let (intra, cost) = match mode {
                        Mode::Intra { luma, chroma } => {
                            let one = [luma];
                            let modes: &[IntraMode] = if n == 0 && !self.fast { &TRIED_MODES } else { &one };
                            let (luma, cost) = self.intra_trial(mi_row, mi_col, n, tx, above, left, modes);
                            (Mode::Intra { luma, chroma }, cost)
                        }
                        _ => {
                            let (sub, cost) = self.intra4_trial(mi_row, mi_col, above, left);
                            (Mode::Intra4 { sub, chroma: IntraMode::Dc }, cost)
                        }
                    };
                    if inter_cost <= cost {
                        // Coded at the vector already: its levels back, and
                        // only the search between pixels to do.
                        std::mem::swap(&mut self.levels[0], &mut self.kept_levels);
                        std::mem::swap(&mut self.eobs[0], &mut self.kept_eobs);
                        let (inter, inter_tx, _) = self.inter(mi_row, mi_col, n, mv, Some((inter_tx, trial_cost)));
                        (inter, inter_tx, false)
                    } else {
                        (intra, tx, true)
                    }
                }
            }
            (mode, _) => (mode, tx, false),
        }
    }

    /// Code the square block of `1 << n` 8×8s at `(mi_row, mi_col)` as
    /// `decide` left it: in `mode` at transform size `tx`, an inter block's
    /// luma levels in `levels` and an intra block's luma modes decided when
    /// `decided`; skipped when no transform block keeps a coefficient.
    fn block(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode, tx: usize, decided: bool) {
        let g = self.geometry;
        let largest = (n as usize + 1).min(3);
        let above = (mi_row > 0).then(|| self.mode_info(mi_row - 1, mi_col));
        let left = (mi_col > self.mi_col_start).then(|| self.mode_info(mi_row, mi_col - 1));
        // An 8×8 intra block's mode is chosen by coding it each way, and
        // each of four 4×4s' in turn, unless that was decided already; then
        // the chroma's, by coding the chroma in DC and in the luma's mode.
        let mode = match mode {
            Mode::Intra { luma, .. } if n == 0 && !self.fast => {
                let luma = if decided { luma } else { self.intra_trial(mi_row, mi_col, n, tx, above, left, &TRIED_MODES).0 };
                Mode::Intra { luma, chroma: self.chroma_trial(mi_row, mi_col, n, tx, luma) }
            }
            Mode::Intra4 { sub, .. } => {
                let sub = if decided { sub } else { self.intra4_trial(mi_row, mi_col, above, left).0 };
                Mode::Intra4 { sub, chroma: self.chroma_trial(mi_row, mi_col, n, tx, sub[3]) }
            }
            mode => mode,
        };
        self.reconstruct(mi_row, mi_col, n, mode, tx);
        let skip = self.eobs.iter().flatten().all(|&eob| eob == 0);
        let tx = if skip && !matches!(mode, Mode::Intra { .. } | Mode::Intra4 { .. }) { largest } else { tx };
        let sub = match mode {
            Mode::Intra { luma, .. } => [luma; 4],
            Mode::Intra4 { sub, .. } => sub,
            _ => [IntraMode::Dc; 4],
        };
        let inter = match mode {
            Mode::Intra { .. } | Mode::Intra4 { .. } => None,
            Mode::Inter(mv) => Some(self.inter_mode(mi_row, mi_col, n, mv)),
            // A decoder gives an inactive block ZEROMV, whatever it would
            // have cost to code.
            Mode::Unchanged => Some((InterMode::Zero, Mv::ZERO)),
        };
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
                // Four 4×4s take the one transform size without saying so.
                if !matches!(mode, Mode::Intra4 { .. }) {
                    self.write_tx(n, tx, above, left);
                }
                // The luma mode's probabilities are chosen by the modes above and
                // to the left, DC by rule where there is no block; the chroma
                // mode's by the luma's.
                self.write_modes(mode, sub, n, above, left);
            } else {
                // libvpx's `get_intra_inter_context`.
                let is_intra = |m: &ModeInfo| m.inter.is_none();
                let ctx = match (above, left) {
                    (Some(a), Some(l)) if is_intra(&a) && is_intra(&l) => 3,
                    (Some(a), Some(l)) => usize::from(is_intra(&a) || is_intra(&l)),
                    (Some(m), None) | (None, Some(m)) => 2 * usize::from(is_intra(&m)),
                    (None, None) => 0,
                };
                self.contexts.tape.put(inter.is_some(), intra_inter_slot(ctx));
                if (!skip || inter.is_none()) && !matches!(mode, Mode::Intra4 { .. }) {
                    self.write_tx(n, tx, above, left);
                }
                if let Some((mode, mv)) = inter {
                    // LAST, the reference tree's first leaf, at libvpx's
                    // `vp9_get_pred_context_single_ref_p1` when LAST is the only
                    // reference: 4 beside an inter block, 2 otherwise.
                    let ctx = if above.is_some_and(|m| !is_intra(&m)) || left.is_some_and(|m| !is_intra(&m)) { 4 } else { 2 };
                    self.contexts.tape.put(false, single_ref_slot(ctx));
                    let slot = inter_mode_slot(self.mode_context(mi_row, mi_col, n));
                    for (b, node) in mode.bits() {
                        self.contexts.tape.put(b, slot + node);
                    }
                    if mode == InterMode::New {
                        let nearest = self.candidates(mi_row, mi_col, n)[0];
                        let tape = &mut self.contexts.tape;
                        mv::code(mv - nearest, |b, i| tape.put(b, mv_slot(i)));
                    }
                } else {
                    self.write_modes(mode, sub, n, above, left);
                }
            }
        }
        let (w, col) = (self.contexts.width, mi_col - self.mi_col_start);
        for row in mi_row..(mi_row + bw).min(g.mi_rows) {
            self.contexts.mode_info[(row - self.top) * w + col..][..bw.min(w - col)].fill(ModeInfo { skip, sub, inter: inter.map(|(mode, mv)| (mode, [mv.row as i16, mv.col as i16])), tx: tx as u8, n: n as u8, inactive, predicted });
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

    /// Of the intra `modes`, the one that codes the luma of the block of
    /// `1 << n` 8×8s at `(mi_row, mi_col)` cheapest at transform size `tx`
    /// in its squared error and its bits at `RATE_WEIGHT`, and that cost: each is predicted
    /// and coded as `reconstruct` codes it, the error measured on the
    /// coefficients and the bits at the frame's starting probabilities, with
    /// the mode's own bits, `above` and `left` being the blocks a keyframe
    /// chooses the mode's probabilities by; the chroma's mode is
    /// `chroma_trial`'s. libvpx's
    /// realtime encoder codes a keyframe's blocks under 16×16 this way.
    #[allow(clippy::too_many_arguments)]
    fn intra_trial(&mut self, mi_row: usize, mi_col: usize, n: u32, tx: usize, above: Option<ModeInfo>, left: Option<ModeInfo>, modes: &[IntraMode]) -> (IntraMode, i64) {
        let g = self.geometry;
        let ac = i64::from(self.quantizer.ac_step());
        let y_probs = *self.y_mode_probs(&[IntraMode::Dc; 4], 0, false, n, above, left);
        let (tx_log2, side) = (tx, 4 << tx);
        let area = side * side;
        let n4 = side / 4;
        let wide4 = (2 << n).min((g.mi_cols - mi_col) * 2);
        let high4 = (2 << n).min((g.mi_rows - mi_row) * 2);
        let (stride, x0, y0) = (self.contexts.stride, self.contexts.x, self.top * 8);
        let (ax, ly0) = ((mi_col - self.mi_col_start) * 2, (mi_row * 2) & 15);
        let mut best = (i64::MAX, IntraMode::Dc, true);
        let (up, beside) = (mi_row > 0, mi_col > self.mi_col_start);
        let blocks = self.transform_blocks(mi_row, mi_col, n, tx_log2).1.count();
        for &mode in modes {
            // A mode that codes no coefficient is beaten only on its bits,
            // and the modes before it cost fewer.
            if best.0 < i64::MAX && best.2 {
                break;
            }
            // A mode predicted without an edge it reads is DC's picture
            // or near it, at more bits; a steep diagonal is tried only
            // where the best so far points its way. A mode tried alone is
            // tried.
            let (reads_above, reads_left) = mode.reads();
            if modes.len() > 1 && (mode != IntraMode::Dc && (reads_above && !up || reads_left && !beside) || mode.pointless_after(best.1)) {
                continue;
            }
            let mut bits: i64 = mode.bits().iter().map(|&(b, node)| i64::from(bool_writer::cost(b, y_probs[node]))).sum();
            let tx_type = Self::tx_type(Mode::Intra { luma: mode, chroma: mode }, 0, tx_log2, 0);
            let scan = tokens::scan(tx_log2, tx_type);
            // The nonzero contexts as the block finds them, moved along by
            // its own transform blocks without being left behind.
            let mut above_nz = [0u8; 16];
            let mut left_nz = [0u8; 16];
            above_nz[..2 << n].copy_from_slice(&self.above_nonzero[0][ax..ax + (2 << n)]);
            left_nz[..2 << n].copy_from_slice(&self.left_nonzero[0][ly0..ly0 + (2 << n)]);
            let mut error = 0i64;
            let mut coded_any = false;
            for (i, (x, y)) in self.transform_blocks(mi_row, mi_col, n, tx_log2).1.enumerate() {
                let (px, py) = (mi_col * 8 + x, mi_row * 8 + y);
                let (lx, ly) = (px - x0, py - y0);
                {
                    let edges = Edges { width: g.mi_cols * 8 - x0, height: g.mi_rows * 8 - py, left: x > 0 || mi_col > self.mi_col_start, right: x + side < (8 << n) };
                    let (before, rows) = self.contexts.planes[0].split_at_mut(ly * stride);
                    let above = match ly {
                        0 => (mi_row > 0).then_some(&self.above_pixels[0][..]),
                        ly => Some(&before[(ly - 1) * stride..]),
                    };
                    predict::intra(mode, above, rows, stride, lx, side, &edges);
                }
                self.transform(0, px, py, side, tx_type);
                let levels = &mut self.levels[0][i * area..][..area];
                let (eob, e) = self.quantizer.quantize_with_error(side, true, scan.inverse, &self.coeffs[..area], levels);
                error += e;
                let (col4, row4) = (x / 4, y / 4);
                let ctx = usize::from(above_nz[col4..col4 + n4].iter().any(|&a| a != 0)) + usize::from(left_nz[row4..row4 + n4].iter().any(|&l| l != 0));
                let kind = tokens::Kind { tx: tx_log2, tx_type, chroma: false, inter: false };
                bits += i64::from(tokens::estimate(self.costs, kind, ctx, levels, eob, &mut self.map));
                let coded = u8::from(eob > 0);
                coded_any |= eob > 0;
                for k in 0..n4 {
                    above_nz[col4 + k] = if col4 + k < wide4 { coded } else { 0 };
                    left_nz[row4 + k] = if row4 + k < high4 { coded } else { 0 };
                }
                // The transform blocks after this one predict from it.
                if eob > 0 && i + 1 < blocks {
                    let levels = &self.levels[0][i * area..][..area];
                    self.quantizer.dequantize(side, levels, scan.order, eob, &mut self.dequantized[..area]);
                    transform::inverse_add(side, tx_type, &self.dequantized[..area], eob, &mut self.contexts.planes[0][ly * stride + lx..], stride);
                }
            }
            let cost = error * 256_000 + RATE_WEIGHT * ac * ac * bits;
            if cost < best.0 {
                best = (cost, mode, !coded_any);
            }
        }
        (best.1, best.0)
    }

    /// The modes of the four 4×4s of the 8×8 at `(mi_row, mi_col)`, each in
    /// turn the one of the ten that codes its luma cheapest in squared
    /// error and bits at `RATE_WEIGHT`, as `intra_trial` weighs them, the
    /// 4×4s before it reconstructed in theirs, and each mode's bits at the
    /// probabilities the modes beside it give (libvpx's
    /// `rd_pick_intra_sub_8x8_y_mode`); and what the four cost together.
    fn intra4_trial(&mut self, mi_row: usize, mi_col: usize, above: Option<ModeInfo>, left: Option<ModeInfo>) -> ([IntraMode; 4], i64) {
        let g = self.geometry;
        let ac = i64::from(self.quantizer.ac_step());
        let (stride, x0, y0) = (self.contexts.stride, self.contexts.x, self.top * 8);
        let (ax, ly0) = ((mi_col - self.mi_col_start) * 2, (mi_row * 2) & 15);
        let wide4 = 2.min((g.mi_cols - mi_col) * 2);
        let high4 = 2.min((g.mi_rows - mi_row) * 2);
        let mut above_nz = [0u8; 2];
        let mut left_nz = [0u8; 2];
        above_nz.copy_from_slice(&self.above_nonzero[0][ax..ax + 2]);
        left_nz.copy_from_slice(&self.left_nonzero[0][ly0..ly0 + 2]);
        let mut sub = [IntraMode::Dc; 4];
        let mut total = 0;
        self.eobs[0].clear();
        for b in 0..4 {
            let (x, y) = ((b & 1) * 4, (b >> 1) * 4);
            let (px, py) = (mi_col * 8 + x, mi_row * 8 + y);
            let (lx, ly) = (px - x0, py - y0);
            let y_probs = *self.y_mode_probs(&sub, b, true, 0, above, left);
            let ctx = usize::from(above_nz[x / 4] != 0) + usize::from(left_nz[y / 4] != 0);
            let (up, beside) = (y > 0 || mi_row > 0, x > 0 || mi_col > self.mi_col_start);
            let mut best = (i64::MAX, IntraMode::Dc, 0usize);
            for mode in TRIED_MODES_4X4 {
                if best.0 < i64::MAX && best.2 == 0 {
                    break;
                }
                let (reads_above, reads_left) = mode.reads();
                if mode != IntraMode::Dc && (reads_above && !up || reads_left && !beside) || mode.pointless_after(best.1) {
                    continue;
                }
                let bits: i64 = mode.bits().iter().map(|&(bit, node)| i64::from(bool_writer::cost(bit, y_probs[node]))).sum();
                let tx_type = mode.tx_type();
                let scan = tokens::scan(0, tx_type);
                {
                    let edges = Edges { width: g.mi_cols * 8 - x0, height: g.mi_rows * 8 - py, left: x > 0 || mi_col > self.mi_col_start, right: x + 4 < 8 };
                    let (before, rows) = self.contexts.planes[0].split_at_mut(ly * stride);
                    let above = match ly {
                        0 => (mi_row > 0).then_some(&self.above_pixels[0][..]),
                        ly => Some(&before[(ly - 1) * stride..]),
                    };
                    predict::intra(mode, above, rows, stride, lx, 4, &edges);
                }
                self.transform(0, px, py, 4, tx_type);
                let levels = &mut self.levels[0][b * 16..][..16];
                let (eob, error) = self.quantizer.quantize_with_error(4, true, scan.inverse, &self.coeffs[..16], levels);
                let kind = tokens::Kind { tx: 0, tx_type, chroma: false, inter: false };
                let bits = bits + i64::from(tokens::estimate(self.costs, kind, ctx, levels, eob, &mut self.map));
                let cost = error * 256_000 + RATE_WEIGHT * ac * ac * bits;
                if cost < best.0 {
                    best = (cost, mode, eob);
                }
            }
            sub[b] = best.1;
            total += best.0;
            // Coded in its mode as the block is, for the 4×4s after it to
            // predict from and for `reconstruct` to leave as it is.
            let mode = best.1;
            let tx_type = mode.tx_type();
            let scan = tokens::scan(0, tx_type);
            {
                let edges = Edges { width: g.mi_cols * 8 - x0, height: g.mi_rows * 8 - py, left: x > 0 || mi_col > self.mi_col_start, right: x + 4 < 8 };
                let (before, rows) = self.contexts.planes[0].split_at_mut(ly * stride);
                let above = match ly {
                    0 => (mi_row > 0).then_some(&self.above_pixels[0][..]),
                    ly => Some(&before[(ly - 1) * stride..]),
                };
                predict::intra(mode, above, rows, stride, lx, 4, &edges);
            }
            self.transform(0, px, py, 4, tx_type);
            let levels = &mut self.levels[0][b * 16..][..16];
            let (eob, error) = self.quantizer.quantize_with_error(4, true, scan.inverse, &self.coeffs[..16], levels);
            let kind = tokens::Kind { tx: 0, tx_type, chroma: false, inter: false };
            let (eob, _, _) = worth_coding(self.costs, &mut self.map, self.quantizer, 4, kind, &self.coeffs[..16], levels, eob, error);
            if eob > 0 {
                self.quantizer.dequantize(4, levels, scan.order, eob, &mut self.dequantized[..16]);
                transform::inverse_add(4, tx_type, &self.dequantized[..16], eob, &mut self.contexts.planes[0][ly * stride + lx..], stride);
            }
            self.eobs[0].push(eob);
            let coded = u8::from(eob > 0);
            above_nz[x / 4] = if x / 4 < wide4 { coded } else { 0 };
            left_nz[y / 4] = if y / 4 < high4 { coded } else { 0 };
        }
        (sub, total)
    }

    /// The chroma mode of the intra block of `1 << n` 8×8s at
    /// `(mi_row, mi_col)` whose luma mode is `luma`: DC or the luma's,
    /// whichever codes the two chroma planes cheaper at transform size `tx`
    /// in squared error and bits at `RATE_WEIGHT`, as `intra_trial` weighs
    /// a luma mode, with the mode's own bits at the probabilities the luma
    /// mode gives it. libvpx chooses a chroma mode by cost
    /// (`rd_pick_intra_sbuv_mode`) from the modes its speed allows, DC
    /// alone in its realtime encoder; in 4:4:4 the chroma is two planes of
    /// three, and which of the two is cheaper differs by the picture.
    fn chroma_trial(&mut self, mi_row: usize, mi_col: usize, n: u32, tx: usize, luma: IntraMode) -> IntraMode {
        if luma == IntraMode::Dc {
            return IntraMode::Dc;
        }
        let g = self.geometry;
        let ac = i64::from(self.quantizer.ac_step());
        let uv_probs = Self::uv_mode_probs(self.previous.is_none(), luma);
        let (tx_log2, side) = (tx, 4 << tx);
        let area = side * side;
        let n4 = side / 4;
        let wide4 = (2 << n).min((g.mi_cols - mi_col) * 2);
        let high4 = (2 << n).min((g.mi_rows - mi_row) * 2);
        let (stride, x0, y0) = (self.contexts.stride, self.contexts.x, self.top * 8);
        let (ax, ly0) = ((mi_col - self.mi_col_start) * 2, (mi_row * 2) & 15);
        let blocks = self.transform_blocks(mi_row, mi_col, n, tx_log2).1.count();
        let scan = tokens::scan(tx_log2, TxType::DctDct);
        let kind = tokens::Kind { tx: tx_log2, tx_type: TxType::DctDct, chroma: true, inter: false };
        let mut best = (i64::MAX, IntraMode::Dc, true);
        for mode in [IntraMode::Dc, luma] {
            // DC, tried first, costs the fewer bits: a mode after it pays
            // only where DC left coefficients.
            if best.0 < i64::MAX && best.2 {
                break;
            }
            let mut bits: i64 = mode.bits().iter().map(|&(b, node)| i64::from(bool_writer::cost(b, uv_probs[node]))).sum();
            let mut error = 0i64;
            let mut coded_any = false;
            for plane in 1..3 {
                let mut above_nz = [0u8; 16];
                let mut left_nz = [0u8; 16];
                above_nz[..2 << n].copy_from_slice(&self.above_nonzero[plane][ax..ax + (2 << n)]);
                left_nz[..2 << n].copy_from_slice(&self.left_nonzero[plane][ly0..ly0 + (2 << n)]);
                for (i, (x, y)) in self.transform_blocks(mi_row, mi_col, n, tx_log2).1.enumerate() {
                    let (px, py) = (mi_col * 8 + x, mi_row * 8 + y);
                    let (lx, ly) = (px - x0, py - y0);
                    {
                        let edges = Edges { width: g.mi_cols * 8 - x0, height: g.mi_rows * 8 - py, left: x > 0 || mi_col > self.mi_col_start, right: x + side < (8 << n) };
                        let (before, rows) = self.contexts.planes[plane].split_at_mut(ly * stride);
                        let above = match ly {
                            0 => (mi_row > 0).then_some(&self.above_pixels[plane][..]),
                            ly => Some(&before[(ly - 1) * stride..]),
                        };
                        predict::intra(mode, above, rows, stride, lx, side, &edges);
                    }
                    self.transform(plane, px, py, side, TxType::DctDct);
                    let levels = &mut self.levels[plane][i * area..][..area];
                    let (eob, e) = self.quantizer.quantize_with_error(side, true, scan.inverse, &self.coeffs[..area], levels);
                    error += e;
                    let (col4, row4) = (x / 4, y / 4);
                    let ctx = usize::from(above_nz[col4..col4 + n4].iter().any(|&a| a != 0)) + usize::from(left_nz[row4..row4 + n4].iter().any(|&l| l != 0));
                    bits += i64::from(tokens::estimate(self.costs, kind, ctx, levels, eob, &mut self.map));
                    let coded = u8::from(eob > 0);
                    coded_any |= eob > 0;
                    for k in 0..n4 {
                        above_nz[col4 + k] = if col4 + k < wide4 { coded } else { 0 };
                        left_nz[row4 + k] = if row4 + k < high4 { coded } else { 0 };
                    }
                    if eob > 0 && i + 1 < blocks {
                        let levels = &self.levels[plane][i * area..][..area];
                        self.quantizer.dequantize(side, levels, scan.order, eob, &mut self.dequantized[..area]);
                        transform::inverse_add(side, TxType::DctDct, &self.dequantized[..area], eob, &mut self.contexts.planes[plane][ly * stride + lx..], stride);
                    }
                }
            }
            let cost = error * 256_000 + RATE_WEIGHT * ac * ac * bits;
            if cost < best.0 {
                best = (cost, mode, !coded_any);
            }
        }
        best.1
    }

    /// The luma mode probabilities of the 4×4 `b` (in raster order; 0 for a
    /// block of one) of an intra block whose modes so far are `sub`, with
    /// `above` and `left` the blocks beside it: in a keyframe, by the modes
    /// of the 4×4s above and to the left (libvpx's `vp9_above_block_mode`
    /// and `vp9_left_block_mode`), DC where there is no block or an inter
    /// one; in an inter frame, by the block's size group.
    fn y_mode_probs(&self, sub: &[IntraMode; 4], b: usize, split4: bool, n: u32, above: Option<ModeInfo>, left: Option<ModeInfo>) -> &'static [u8; 9] {
        if self.previous.is_some() {
            return &Y_MODE_PROBS[if split4 { 0 } else { (n as usize + 1).min(3) }];
        }
        let a = if b < 2 { above.map_or(IntraMode::Dc, |m| m.sub[b + 2]) } else { sub[b - 2] };
        let l = if b & 1 == 0 { left.map_or(IntraMode::Dc, |m| m.sub[b + 1]) } else { sub[b - 1] };
        &KF_Y_MODE_PROBS[a as usize][l as usize]
    }

    /// Code the modes of an intra block: one for a block of one, four for
    /// four 4×4s, and then the chroma mode, at the probabilities the luma
    /// mode (the last 4×4's) gives it.
    fn write_modes(&mut self, mode: Mode, sub: [IntraMode; 4], n: u32, above: Option<ModeInfo>, left: Option<ModeInfo>) {
        let (keyframe, uv) = (self.previous.is_none(), Self::intra_of(mode, 1, 0).expect("an intra block"));
        let split4 = matches!(mode, Mode::Intra4 { .. });
        for b in 0..if split4 { 4 } else { 1 } {
            // An inter frame's by size group, at the probabilities it
            // updates.
            if !keyframe {
                let slot = y_mode_slot(if split4 { 0 } else { (n as usize + 1).min(3) });
                for &(bit, node) in sub[b].bits() {
                    self.contexts.tape.put(bit, slot + node);
                }
                continue;
            }
            let probs = self.y_mode_probs(&sub, b, split4, n, above, left);
            for &(bit, node) in sub[b].bits() {
                self.contexts.tape.write(bit, probs[node]);
            }
        }
        let probs = Self::uv_mode_probs(keyframe, sub[3]);
        for &(bit, node) in uv.bits() {
            self.contexts.tape.write(bit, probs[node]);
        }
    }

    /// The chroma mode probabilities of an intra block whose luma mode is
    /// `luma`, in a keyframe or not.
    fn uv_mode_probs(keyframe: bool, luma: IntraMode) -> &'static [u8; 9] {
        if keyframe { &KF_UV_MODE_PROBS[luma as usize] } else { &UV_MODE_PROBS[luma as usize] }
    }

    /// The intra mode transform block `i` of `plane` of a block coded in
    /// `mode` is predicted in: the luma's, or its own 4×4's, and the
    /// chroma's; none for an inter block.
    fn intra_of(mode: Mode, plane: usize, i: usize) -> Option<IntraMode> {
        match mode {
            Mode::Intra { luma, chroma } => Some(if plane == 0 { luma } else { chroma }),
            Mode::Intra4 { sub, chroma } => Some(if plane == 0 { sub[i] } else { chroma }),
            _ => None,
        }
    }

    /// The type of transform block `i` of plane `plane` of a block coded in
    /// `mode` at size `tx`: an intra block's luma below 32×32 its mode's,
    /// all else a DCT.
    fn tx_type(mode: Mode, plane: usize, tx: usize, i: usize) -> TxType {
        match Self::intra_of(mode, plane, i) {
            Some(intra) if plane == 0 && tx < 3 => intra.tx_type(),
            _ => TxType::DctDct,
        }
    }

    /// The cheapest way to code `mv` for the square block of `1 << n` 8×8s
    /// at `(mi_row, mi_col)`.
    fn inter_mode(&self, mi_row: usize, mi_col: usize, n: u32, mv: Mv) -> (InterMode, Mv) {
        (Prices::new(self.candidates(mi_row, mi_col, n), &INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)]).of(mv).1, mv)
    }

    /// The vector within three quarters of a pixel of `mv`, other than
    /// `mv`, that the square block of `1 << n` 8×8s at `(mi_row, mi_col)`
    /// is best predicted at, if one is, by its luma's variance from the prediction and the
    /// bits of the vector at `RATE_WEIGHT`, the variance being an error to
    /// come in the residual: libvpx's pruned sub-pixel search
    /// (`vp9_find_best_sub_pixel_tree_pruned_more`), a half pixel where the
    /// whole pixels around `mv` say, or each way, and then each way a quarter
    /// pixel from the best. The patch around `mv` is filtered across once
    /// per sixteenth the vectors tried ask for, so each of them is at most a
    /// pass down. A block the picture does not hold whole is not looked at;
    /// one whose luma `mv` predicts exactly has nothing better.
    fn refine(&mut self, mi_row: usize, mi_col: usize, n: u32, mv: Mv) -> Option<Mv> {
        let src = self.source;
        let size = 8 << n;
        let (px, py) = (mi_col * 8, mi_row * 8);
        if px + size > src.width || py + size > src.height {
            return None;
        }
        let reference = self.previous.as_ref().expect("an inter frame").reference;
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
            return None;
        }
        let mut best = (first.0, mv, first.1);
        // From a vector of whole pixels that predicts better than the whole
        // pixels around it, a half pixel only where those say the least
        // lies, fitting a parabola through them (libvpx's
        // `get_cost_surf_min`); otherwise each way. Where each of them costs
        // more than `SHARP` times as much again, the picture sits on whole
        // pixels, as text and a page moved by whole ones do, and a filtered
        // prediction only blurs it: nothing is looked for between them.
        let mut whole = |row: i32, col: i32| cost(Mv { row: mv.row + row, col: mv.col + col }).0;
        match (mv.split().1 == (0, 0)).then(|| [whole(0, -8), whole(0, 8), whole(-8, 0), whole(8, 0)]) {
            Some(around) if around.iter().all(|&c| c - first.0 > SHARP * first.0) => return None,
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
        if best.1 == mv {
            return None;
        }
        // The luma predicted at the vector found, kept in `predicted` for
        // `inter`, from the patch filtered across already: the pass down
        // alone, or a copy where the vector sits on a row.
        let ((vx, vy), (fx, fy)) = best.1.split();
        let (x, y) = ((vx - dx + 1) as usize, (vy - dy + 1) as usize);
        let (across, from, x) = if fx == 0 { (block, stride, x + before) } else { (&planes[fx][..], wide, x) };
        if fy == 0 {
            for r in 0..size {
                prediction[r * size..][..size].copy_from_slice(&across[(y + before + r) * from + x..][..size]);
            }
        } else {
            convolve::down(&across[y * from + x..], from, fy, (size, size), prediction, size);
        }
        Some(best.1)
    }

    /// The luma `refine` predicted at the vector it found, from `predicted`
    /// into `recon`, where `predict_inter` would put it.
    fn take_refined(&mut self, mi_row: usize, mi_col: usize, n: u32) {
        let size = 8 << n;
        let (stride, x, y) = (self.contexts.stride, mi_col * 8 - self.contexts.x, mi_row * 8 - self.top * 8);
        let out = &mut self.contexts.planes[0][y * stride + x..];
        for r in 0..size {
            out[r * stride..][..size].copy_from_slice(&self.predicted[r * size..][..size]);
        }
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
    /// whole: as a decoder does, past the picture's edges the previous
    /// frame's edge pixels repeat. The part past the frame's edge rounded
    /// up to 8 is read by the transform blocks that overhang it, in a
    /// picture whose 8×8 rows are not a multiple of the transform's (1080
    /// has 135); left stale, their residual there was garbage, coded at a
    /// thousand tokens a block in every inter frame that coded the bottom
    /// row, unseen by a decoder and paid for.
    fn predict_inter(&mut self, mi_row: usize, mi_col: usize, n: u32, mv: Mv, planes: std::ops::Range<usize>) {
        let reference = self.previous.as_ref().expect("an inter frame").reference;
        let picture = (self.source.width, self.source.height);
        let ((dx, dy), fraction) = mv.split();
        let size = 8 << n;
        let (px, py) = (mi_col * 8, mi_row * 8);
        let (wide, high) = (size, size);
        let (stride, x, y) = (self.contexts.stride, px - self.contexts.x, py - self.top * 8);
        let (rx, ry) = (px as isize + dx, py as isize + dy);
        for (plane, recon) in self.contexts.planes.iter_mut().enumerate().take(planes.end).skip(planes.start) {
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
                    residual[r * tx + c] = i16::from(srow[(px + c).min(src.width - 1)]) - i16::from(prow[c]);
                }
            }
        }
        // A prediction that is the source leaves nothing to transform.
        if quantize::nothing(&residual[..area]) {
            self.coeffs[..area].fill(0);
            return;
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
                self.predict_inter(mi_row, mi_col, n, Mv::ZERO, 0..3);
            }
            self.nothing(mi_row, mi_col, n, tx_log2);
            return;
        }
        for plane in 0..3 {
            // Four 4×4s' luma `intra4_trial` left coded and reconstructed.
            if plane == 0 && matches!(mode, Mode::Intra4 { .. }) {
                continue;
            }
            // All but an inter block's luma, which `trial` left.
            let fresh = matches!(mode, Mode::Intra { .. }) || plane > 0;
            if fresh {
                self.eobs[plane].clear();
            }
            for (i, (x, y)) in self.transform_blocks(mi_row, mi_col, n, tx_log2).1.enumerate() {
                let tx_type = Self::tx_type(mode, plane, tx_log2, i);
                let scan = tokens::scan(tx_log2, tx_type);
                let (px, py) = (mi_col * 8 + x, mi_row * 8 + y);
                // Where it is in the row's pixels.
                let (lx, ly) = (px - x0, py - y0);
                let eob = if fresh {
                    if let Some(intra) = Self::intra_of(mode, plane, i) {
                        let edges = Edges { width: g.mi_cols * 8 - x0, height: g.mi_rows * 8 - py, left: x > 0 || mi_col > self.mi_col_start, right: x + tx < (8 << n) };
                        let (before, rows) = self.contexts.planes[plane].split_at_mut(ly * stride);
                        let above = match ly {
                            0 => (mi_row > 0).then_some(&self.above_pixels[plane][..]),
                            ly => Some(&before[(ly - 1) * stride..]),
                        };
                        predict::intra(intra, above, rows, stride, lx, tx, &edges);
                    }
                    self.transform(plane, px, py, tx, tx_type);
                    let levels = &mut self.levels[plane][i * area..][..area];
                    let kind = tokens::Kind { tx: tx_log2, tx_type, chroma: plane > 0, inter: !matches!(mode, Mode::Intra { .. } | Mode::Intra4 { .. }) };
                    let (eob, error) = self.quantizer.quantize_with_error(tx, !kind.inter, scan.inverse, &self.coeffs[..area], levels);
                    let (eob, _, _) = worth_coding(self.costs, &mut self.map, self.quantizer, tx, kind, &self.coeffs[..area], levels, eob, error);
                    self.eobs[plane].push(eob);
                    eob
                } else {
                    self.eobs[plane][i]
                };
                if eob > 0 {
                    let levels = &self.levels[plane][i * area..][..area];
                    self.quantizer.dequantize(tx, levels, scan.order, eob, &mut self.dequantized[..area]);
                    transform::inverse_add(tx, tx_type, &self.dequantized[..area], eob, &mut self.contexts.planes[plane][ly * stride + lx..], stride);
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
        let inter = !matches!(mode, Mode::Intra { .. } | Mode::Intra4 { .. });
        for plane in 0..3 {
            for (i, (x, y)) in self.transform_blocks(mi_row, mi_col, n, tx_log2).1.enumerate() {
                let tx_type = Self::tx_type(mode, plane, tx_log2, i);
                let (col4, row4) = (x / 4, y / 4);
                let ax = (mi_col - self.mi_col_start) * 2 + col4;
                let ly = ((mi_row * 2) & 15) + row4;
                let above = self.above_nonzero[plane][ax..ax + n4].iter().any(|&a| a != 0);
                let left = self.left_nonzero[plane][ly..ly + n4].iter().any(|&l| l != 0);
                let eob = self.eobs[plane][i];
                let kind = tokens::Kind { tx: tx_log2, tx_type, chroma: plane > 0, inter };
                let ctx = usize::from(above) + usize::from(left);
                tokens::write(self.contexts.tape, kind, ctx, &self.levels[plane][i * area..][..area], eob, &mut self.map);
                let coded = u8::from(eob > 0);
                for k in 0..n4 {
                    self.above_nonzero[plane][ax + k] = if col4 + k < wide4 { coded } else { 0 };
                    self.left_nonzero[plane][ly + k] = if row4 + k < high4 { coded } else { 0 };
                }
            }
        }
    }
}
