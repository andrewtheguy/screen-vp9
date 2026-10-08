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

use super::bool_writer::{self, BoolWriter};
use super::motion;
use super::mv::{self, Mv};
use super::predict::{self, Edges};
use super::quantize::Quantizer;
use super::tables::{
    INTER_MODE_PROBS, INTRA_INTER_PROBS, KF_PARTITION_PROBS, KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, MV_REF_BLOCKS, PARTITION_PROBS, SINGLE_REF_PROBS,
    SKIP_PROBS, UV_MODE_PROBS, Y_MODE_PROBS,
};
use super::{Geometry, Reconstruction, tokens, transform};

/// `DC_PRED`'s index among the intra modes.
const DC_PRED: usize = 0;

/// A superblock's side in mode info units of 8×8, as log2.
const SB_LOG2: u32 = 3;

/// The variance per pixel, summed over the planes, above which a square is
/// split, as a fraction of the AC step: `1 / SPLIT_VARIANCE`. Measured on the
/// plan's captures, a keyframe's bytes and PSNR barely move between splitting
/// everything with any variance and keeping squares whole up to half a step,
/// and around an eighth was best; keeping every square whole cost up to a
/// third more bytes and 1–3 dB.
const SPLIT_VARIANCE: i64 = 8;

/// A vector the search offers is taken over the vectors a decoder predicts
/// only where it leaves `1 / NEW_VECTOR_GAIN` less variance than they do.
const NEW_VECTOR_GAIN: i64 = 4;

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

/// The picture being coded: three planes at its own size.
pub(super) struct Source<'a> {
    pub(super) planes: [&'a [u8]; 3],
    pub(super) stride: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

/// What an inter frame is coded against: the previous frame as a decoder
/// has it, per 8×8 of the frame whether the source there is the previous
/// frame's, and per superblock the vectors the motion search offers it.
pub(super) struct Previous<'a> {
    pub(super) reference: &'a Reconstruction,
    pub(super) unchanged: &'a [bool],
    pub(super) vectors: &'a [[Mv; motion::VECTORS]],
}

/// How a block is coded.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// DC prediction.
    Intra,
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
    /// How an inter block's vector was coded, and the vector; none for an
    /// intra block.
    inter: Option<(InterMode, Mv)>,
}

/// What a frame's tiles share: the contexts read across a tile's top edge,
/// and the mode info of every 8×8 coded so far, and of the frame before.
pub(super) struct FrameContexts {
    above_partition: Vec<u8>,
    /// Per plane, per column of 4×4: whether the transform block above had
    /// a coefficient.
    above_nonzero: [Vec<u8>; 3],
    mode_info: Vec<ModeInfo>,
    last_mode_info: Vec<ModeInfo>,
    /// Whether a decoder predicts vectors from the frame before's too:
    /// when that was an inter frame (libvpx's `use_prev_frame_mvs`).
    use_last: bool,
}

impl FrameContexts {
    pub(super) fn new(geometry: &Geometry) -> Self {
        Self {
            above_partition: vec![0; geometry.sb_cols * 8],
            above_nonzero: std::array::from_fn(|_| vec![0; geometry.sb_cols * 16]),
            mode_info: vec![ModeInfo::default(); geometry.mi_rows * geometry.mi_cols],
            last_mode_info: vec![ModeInfo::default(); geometry.mi_rows * geometry.mi_cols],
            use_last: false,
        }
    }

    /// Back to how a decoder starts a frame, the mode info coded last kept
    /// as the frame before's. `last_inter` says whether that frame was an
    /// inter frame.
    pub(super) fn start(&mut self, last_inter: bool) {
        self.above_partition.fill(0);
        for plane in &mut self.above_nonzero {
            plane.fill(0);
        }
        std::mem::swap(&mut self.mode_info, &mut self.last_mode_info);
        self.mode_info.fill(ModeInfo::default());
        self.use_last = last_inter;
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

/// Fill `out` from `row` at column `x` on, as a decoder reads a reference
/// frame: past either end of the row, the pixel at that end.
fn read_row(out: &mut [u8], row: &[u8], x: isize) {
    let w = row.len() as isize;
    if x >= 0 && x + out.len() as isize <= w {
        out.copy_from_slice(&row[x as usize..][..out.len()]);
    } else {
        for (i, o) in out.iter_mut().enumerate() {
            *o = row[(x + i as isize).clamp(0, w - 1) as usize];
        }
    }
}

/// The eight pixels of `row` from `x`, read as `read_row` reads them.
fn eight(row: &[u8], x: isize) -> [u8; 8] {
    let mut out = [0; 8];
    read_row(&mut out, row, x);
    out
}

/// The coder of one tile of a frame.
pub(super) struct Tile<'a> {
    geometry: &'a Geometry,
    mi_col_start: usize,
    mi_col_end: usize,
    contexts: &'a mut FrameContexts,
    left_partition: [u8; 8],
    /// Per plane, per row of 4×4 in the superblock row.
    left_nonzero: [[u8; 16]; 3],
    recon: &'a mut Reconstruction,
    source: &'a Source<'a>,
    /// What an inter frame is coded against; none in a keyframe.
    previous: Option<Previous<'a>>,
    quantizer: &'a Quantizer,
    /// The variance a square is split above, `SPLIT_VARIANCE` of the step.
    split_variance: i64,
    /// The superblock being coded, per 8×8 in raster order.
    measures: [Measure; 64],
    /// The superblock being coded, per vector other than zero measured at
    /// so far: per 8×8, the moments of its residual at that vector, and
    /// which of them are measured yet.
    moved: Vec<(Mv, [Moments; 64], u64)>,
    w: BoolWriter,
    /// The block being coded: per plane, the levels of its transform blocks
    /// one after another, each in raster order, and their ends of block.
    levels: [Vec<i32>; 3],
    eobs: [Vec<usize>; 3],
}

impl<'a> Tile<'a> {
    pub(super) fn new(
        geometry: &'a Geometry,
        cols: std::ops::Range<usize>,
        contexts: &'a mut FrameContexts,
        recon: &'a mut Reconstruction,
        source: &'a Source<'a>,
        previous: Option<Previous<'a>>,
        quantizer: &'a Quantizer,
    ) -> Self {
        Self {
            geometry,
            mi_col_start: cols.start,
            mi_col_end: cols.end,
            contexts,
            left_partition: [0; 8],
            left_nonzero: [[0; 16]; 3],
            recon,
            source,
            previous,
            quantizer,
            split_variance: i64::from(quantizer.ac_step()) / SPLIT_VARIANCE,
            measures: [Measure::default(); 64],
            moved: Vec::new(),
            w: BoolWriter::new(),
            levels: std::array::from_fn(|_| vec![0; 64 * 64]),
            eobs: std::array::from_fn(|_| Vec::with_capacity(16)),
        }
    }

    /// Code every superblock of the tile and give back its bytes.
    pub(super) fn code(mut self) -> Vec<u8> {
        for mi_row in (0..self.geometry.mi_rows).step_by(8) {
            self.left_partition = [0; 8];
            self.left_nonzero = [[0; 16]; 3];
            for mi_col in (self.mi_col_start..self.mi_col_end).step_by(8) {
                if !self.unchanged(mi_row, mi_col, SB_LOG2) {
                    self.measure(mi_row, mi_col);
                    self.moved.clear();
                }
                self.partition(mi_row, mi_col, SB_LOG2);
            }
        }
        self.w.finish()
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

    /// The row `y` of the source's `plane` as it is coded: past the
    /// picture's bottom, its last row.
    fn source_row(&self, plane: usize, y: usize) -> &'a [u8] {
        let src = self.source;
        &src.planes[plane][y.min(src.height - 1) * src.stride..][..src.width]
    }

    /// The moments of the source's 8×8 at `(px, py)` as it is coded: past
    /// the picture's edges, its last pixels repeated.
    fn source_moments(&self, px: usize, py: usize) -> Moments {
        let mut m = Moments::default();
        for plane in 0..3 {
            for y in py..py + 8 {
                for v in eight(self.source_row(plane, y), px as isize) {
                    m.sums[plane] += i64::from(v);
                    m.squares += i64::from(v) * i64::from(v);
                }
            }
        }
        m
    }

    /// The moments of the source's 8×8 at `(px, py)` less the previous frame
    /// at `mv` from it, read as a decoder reads it.
    fn residual_moments(&self, px: usize, py: usize, mv: Mv) -> Moments {
        let reference = self.previous.as_ref().expect("an inter frame").reference;
        let (width, height) = (self.source.width, self.source.height);
        let (dy, dx) = mv.whole();
        let mut m = Moments::default();
        let (rx, ry) = (px as isize + dx, py as isize + dy);
        if px + 8 <= width && py + 8 <= height && rx >= 0 && ry >= 0 && rx as usize + 8 <= width && ry as usize + 8 <= height {
            // Both wholly inside the picture: read as they are.
            let (rx, ry) = (rx as usize, ry as usize);
            let src = self.source;
            for plane in 0..3 {
                let (mut sum, mut squares) = (0i32, 0i32);
                for k in 0..8 {
                    let a = &src.planes[plane][(py + k) * src.stride + px..][..8];
                    let b = &reference.planes[plane][(ry + k) * reference.stride + rx..][..8];
                    for (&a, &b) in a.iter().zip(b) {
                        let d = i32::from(a) - i32::from(b);
                        sum += d;
                        squares += d * d;
                    }
                }
                m.sums[plane] = i64::from(sum);
                m.squares += i64::from(squares);
            }
            return m;
        }
        for plane in 0..3 {
            for y in py..py + 8 {
                let ry = (y as isize + dy).clamp(0, height as isize - 1) as usize;
                let p = eight(&reference.planes[plane][ry * reference.stride..][..width], px as isize + dx);
                for (a, b) in eight(self.source_row(plane, y), px as isize).into_iter().zip(p) {
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
    /// other than zero, from it. Each 8×8 is measured at a vector once per
    /// superblock, however many squares it is in.
    fn square_residual(&mut self, mi_row: usize, mi_col: usize, n: u32, mv: Mv) -> Moments {
        let (rows, cols) = self.inside(mi_row, mi_col, n);
        let (row0, col0) = (mi_row - (mi_row & 7), mi_col - (mi_col & 7));
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
    /// whole, or none when it varies more from its best prediction than a
    /// square is coded whole at.
    fn choose(&mut self, mi_row: usize, mi_col: usize, n: u32) -> Option<Mode> {
        let partly = match self.count_unchanged(mi_row, mi_col, n) {
            Some(u) if u == self.count_inside(mi_row, mi_col, n) => return Some(Mode::Unchanged),
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
        let intra = total.source.variance(count);
        let (mode, variance) = match &self.previous {
            Some(previous) => {
                // Zero first, then the vectors a decoder predicts, then the
                // search's, which are new vectors to code and so have to do
                // clearly better than the others to be taken.
                let searched = previous.vectors[(mi_row >> SB_LOG2) * self.geometry.sb_cols + (mi_col >> SB_LOG2)];
                let predicted = self.candidates(mi_row, mi_col, n);
                let mut best = (Mv::ZERO, total.residual.variance(count));
                let mut tried = [Mv::ZERO; 1 + 2 + motion::VECTORS];
                for (i, mv) in predicted.into_iter().chain(searched).enumerate() {
                    if best.1 == 0 || tried.contains(&mv) {
                        continue;
                    }
                    tried[1 + i] = mv;
                    let variance = self.square_residual(mi_row, mi_col, n, mv).variance(count);
                    let better = if i < predicted.len() { variance < best.1 } else { variance < best.1 - best.1 / NEW_VECTOR_GAIN };
                    if better {
                        best = (mv, variance);
                    }
                }
                if best.1 <= intra { (Mode::Inter(best.0), best.1) } else { (Mode::Intra, intra) }
            }
            None => (Mode::Intra, intra),
        };
        // A square that is unchanged in part is split unless what moved
        // into it is coded as moving, so the unchanged part stays the
        // previous frame's rather than coding again what the previous frame
        // got wrong there. What moved takes the unchanged part with it: it
        // is as likely to be where the motion left it as where it was.
        if partly && matches!(mode, Mode::Intra | Mode::Inter(Mv::ZERO)) {
            return None;
        }
        (n == 0 || variance <= self.split_variance * count * count).then_some(mode)
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
                self.w.write(split, probs[0]);
                if split {
                    self.w.write(true, probs[1]);
                    self.w.write(true, probs[2]);
                }
            }
            // Split, or a horizontal half the frame cuts.
            (false, true) => self.w.write(split, probs[1]),
            // Split, or a vertical half the frame cuts.
            (true, false) => self.w.write(split, probs[2]),
            // Split, and the decoder knows it.
            (false, false) => {}
        }
        match mode {
            None => {
                for (r, c) in [(0, 0), (0, half), (half, 0), (half, half)] {
                    self.partition(mi_row + r, mi_col + c, n - 1);
                }
            }
            Some(mode) => {
                self.block(mi_row, mi_col, n, mode);
                let bw = 1 << n;
                let ctx = partition_context_of(n);
                self.contexts.above_partition[mi_col..mi_col + bw].fill(ctx);
                self.left_partition[mi_row & 7..(mi_row & 7) + bw].fill(ctx);
            }
        }
    }

    /// libvpx's `partition_plane_context`.
    fn partition_context(&self, mi_row: usize, mi_col: usize, n: u32) -> usize {
        let above = (self.contexts.above_partition[mi_col] >> n) & 1;
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
            (r < g.mi_rows && (self.mi_col_start..self.mi_col_end).contains(&c)).then(|| self.contexts.mode_info[r * g.mi_cols + c])
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
        let last = self.contexts.use_last.then(|| self.contexts.last_mode_info[mi_row * g.mi_cols + mi_col]);
        let mut found = self.neighbours(mi_row, mi_col, n, MV_REF_BLOCKS[0].len()).chain(last).filter_map(|m| m.inter.map(|(_, mv)| mv));
        let nearest = found.next().unwrap_or(Mv::ZERO);
        let near = found.find(|&mv| mv != nearest).unwrap_or(Mv::ZERO);
        [nearest, near].map(|mv| mv.clamp((mi_row, mi_col), 1 << n, (g.mi_rows, g.mi_cols)))
    }

    /// Code the square block of `1 << n` 8×8s at `(mi_row, mi_col)` in
    /// `mode`, at the transform size `ALLOW_32X32` gives it, and skipped
    /// when no transform block keeps a coefficient.
    fn block(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode) {
        let g = self.geometry;
        self.reconstruct(mi_row, mi_col, n, mode);
        let skip = self.eobs.iter().flatten().all(|&eob| eob == 0);
        let inter = match mode {
            Mode::Intra => None,
            Mode::Inter(mv) => Some(self.inter_mode(mi_row, mi_col, n, mv)),
            Mode::Unchanged => Some(self.inter_mode(mi_row, mi_col, n, Mv::ZERO)),
        };
        let above = (mi_row > 0).then(|| self.contexts.mode_info[(mi_row - 1) * g.mi_cols + mi_col]);
        let left = (mi_col > self.mi_col_start).then(|| self.contexts.mode_info[mi_row * g.mi_cols + mi_col - 1]);
        let skips = usize::from(above.is_some_and(|m| m.skip)) + usize::from(left.is_some_and(|m| m.skip));
        let bw = 1 << n;
        self.w.write(skip, SKIP_PROBS[skips]);
        if self.previous.is_none() {
            // The luma mode's probabilities are chosen by the modes above and
            // to the left, which are DC in every block of a keyframe this
            // encoder codes and DC by rule where there is no block.
            self.w.write(false, KF_Y_MODE_PROBS[DC_PRED][DC_PRED][0]);
            self.w.write(false, KF_UV_MODE_PROBS[DC_PRED][0]);
        } else {
            // libvpx's `get_intra_inter_context`.
            let is_intra = |m: &ModeInfo| m.inter.is_none();
            let ctx = match (above, left) {
                (Some(a), Some(l)) if is_intra(&a) && is_intra(&l) => 3,
                (Some(a), Some(l)) => usize::from(is_intra(&a) || is_intra(&l)),
                (Some(m), None) | (None, Some(m)) => 2 * usize::from(is_intra(&m)),
                (None, None) => 0,
            };
            self.w.write(inter.is_some(), INTRA_INTER_PROBS[ctx]);
            if let Some((mode, mv)) = inter {
                // LAST, the reference tree's first leaf, at libvpx's
                // `vp9_get_pred_context_single_ref_p1` when LAST is the only
                // reference: 4 beside an inter block, 2 otherwise.
                let ctx = if above.is_some_and(|m| !is_intra(&m)) || left.is_some_and(|m| !is_intra(&m)) { 4 } else { 2 };
                self.w.write(false, SINGLE_REF_PROBS[ctx][0]);
                let probs = INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)];
                for (b, node) in mode.bits() {
                    self.w.write(b, probs[node]);
                }
                if mode == InterMode::New {
                    let nearest = self.candidates(mi_row, mi_col, n)[0];
                    let w = &mut self.w;
                    mv::code(mv - nearest, |b, p| w.write(b, p));
                }
            } else {
                // DC, at the probabilities of the block's size group.
                self.w.write(false, Y_MODE_PROBS[(n as usize + 1).min(3)][0]);
                self.w.write(false, UV_MODE_PROBS[DC_PRED][0]);
            }
        }
        for row in mi_row..(mi_row + bw).min(g.mi_rows) {
            self.contexts.mode_info[row * g.mi_cols + mi_col..][..bw.min(g.mi_cols - mi_col)].fill(ModeInfo { skip, inter });
        }
        if skip {
            // A skipped block leaves no coefficient above or left of the
            // blocks after it, over its whole size.
            let (x4, y4) = (mi_col * 2, (mi_row * 2) & 15);
            for plane in 0..3 {
                self.contexts.above_nonzero[plane][x4..x4 + 2 * bw].fill(0);
                self.left_nonzero[plane][y4..y4 + 2 * bw].fill(0);
            }
        } else {
            self.tokens(mi_row, mi_col, n, inter.is_some());
        }
    }

    /// The cheapest way to code `mv` for the square block of `1 << n` 8×8s
    /// at `(mi_row, mi_col)`: as zero, as a vector a decoder predicts for
    /// the block, or as a new one against the nearest of those.
    fn inter_mode(&self, mi_row: usize, mi_col: usize, n: u32, mv: Mv) -> (InterMode, Mv) {
        let [nearest, near] = self.candidates(mi_row, mi_col, n);
        let probs = &INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)];
        let new = (mv != nearest).then(|| (InterMode::New.cost(probs) + mv::cost(mv - nearest), InterMode::New));
        let mode = [(InterMode::Zero, Mv::ZERO), (InterMode::Nearest, nearest), (InterMode::Near, near)]
            .into_iter()
            .filter(|&(_, predicted)| predicted == mv)
            .map(|(mode, _)| (mode.cost(probs), mode))
            .chain(new)
            .min_by_key(|&(cost, _)| cost)
            .expect("a new vector when no other way fits")
            .1;
        (mode, mv)
    }

    /// The transform blocks of a block at `(mi_row, mi_col)`, in the order
    /// they are predicted and coded: per plane, rows of them, each starting
    /// inside the frame's edge rounded up to 8. With the transform size, the
    /// offset of each in pixels from the block's corner.
    fn transform_blocks(&self, mi_row: usize, mi_col: usize, n: u32) -> (usize, impl Iterator<Item = (usize, usize)>) {
        let g = self.geometry;
        let size = 8 << n;
        let tx = size.min(32);
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
        let (width, height) = (self.source.width, self.source.height);
        let (dy, dx) = mv.whole();
        let size = 8 << n;
        let (px, py) = (mi_col * 8, mi_row * 8);
        let wide = size.min(g.mi_cols * 8 - px);
        let high = size.min(g.mi_rows * 8 - py);
        let stride = self.recon.stride;
        for (plane, recon) in self.recon.planes.iter_mut().enumerate() {
            for y in py..py + high {
                let ry = (y as isize + dy).clamp(0, height as isize - 1) as usize;
                read_row(&mut recon[y * stride + px..][..wide], &reference.planes[plane][ry * reference.stride..][..width], px as isize + dx);
            }
        }
    }

    /// Predict, code and reconstruct the block's transform blocks, each from
    /// the ones before it, keeping their levels to be written.
    fn reconstruct(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode) {
        let g = self.geometry;
        match mode {
            Mode::Intra => {}
            Mode::Inter(mv) => self.predict_inter(mi_row, mi_col, n, mv),
            Mode::Unchanged => self.predict_inter(mi_row, mi_col, n, Mv::ZERO),
        }
        let (tx, blocks) = self.transform_blocks(mi_row, mi_col, n);
        let blocks: Vec<_> = blocks.collect();
        let (scan, _) = tokens::scan(tx.ilog2() as usize - 2);
        let stride = self.recon.stride;
        let src = self.source;
        let mut residual = [0i32; 1024];
        let mut coeffs = [0i32; 1024];
        let area = tx * tx;
        for plane in 0..3 {
            self.eobs[plane].clear();
            if mode == Mode::Unchanged {
                self.eobs[plane].resize(blocks.len(), 0);
                continue;
            }
            for (i, &(x, y)) in blocks.iter().enumerate() {
                let (px, py) = (mi_col * 8 + x, mi_row * 8 + y);
                let recon = &mut self.recon.planes[plane];
                if mode == Mode::Intra {
                    let edges = Edges { width: g.mi_cols * 8, height: g.mi_rows * 8, above: y > 0 || mi_row > 0, left: x > 0 || mi_col > self.mi_col_start };
                    predict::dc(recon, stride, px, py, tx, &edges);
                }
                // Past the picture's edge the source repeats its last pixel.
                for r in 0..tx {
                    let srow = &src.planes[plane][(py + r).min(src.height - 1) * src.stride..][..src.width];
                    let prow = &recon[(py + r) * stride + px..][..tx];
                    for c in 0..tx {
                        residual[r * tx + c] = i32::from(srow[(px + c).min(src.width - 1)]) - i32::from(prow[c]);
                    }
                }
                transform::forward(tx, &residual[..area], &mut coeffs[..area]);
                let levels = &mut self.levels[plane][i * area..][..area];
                let eob = self.quantizer.quantize(tx, &coeffs[..area], scan, levels);
                if eob > 0 {
                    self.quantizer.dequantize(tx, levels, scan, eob, &mut coeffs[..area]);
                    transform::inverse_add(tx, &coeffs[..area], eob, &mut recon[py * stride + px..], stride);
                }
                self.eobs[plane].push(eob);
            }
        }
    }

    /// Write the tokens of every transform block of a block that is not
    /// skipped, keeping the contexts a decoder keeps (libvpx's
    /// `vp9_decode_block_tokens`): a transform block's context is whether
    /// any 4×4 column above it or row left of it had a coefficient, and it
    /// leaves its own answer behind over the columns and rows inside the
    /// frame, and none past it.
    fn tokens(&mut self, mi_row: usize, mi_col: usize, n: u32, inter: bool) {
        let g = self.geometry;
        let (tx, blocks) = self.transform_blocks(mi_row, mi_col, n);
        let blocks: Vec<_> = blocks.collect();
        let tx_log2 = tx.ilog2() as usize - 2;
        let n4 = tx / 4;
        let area = tx * tx;
        // In 4×4 units, from the block's corner, how far the frame reaches.
        let wide4 = (2 << n).min((g.mi_cols - mi_col) * 2);
        let high4 = (2 << n).min((g.mi_rows - mi_row) * 2);
        for plane in 0..3 {
            for (i, &(x, y)) in blocks.iter().enumerate() {
                let (col4, row4) = (x / 4, y / 4);
                let ax = mi_col * 2 + col4;
                let ly = ((mi_row * 2) & 15) + row4;
                let above = self.contexts.above_nonzero[plane][ax..ax + n4].iter().any(|&a| a != 0);
                let left = self.left_nonzero[plane][ly..ly + n4].iter().any(|&l| l != 0);
                let eob = self.eobs[plane][i];
                tokens::write(&mut self.w, tx_log2, plane > 0, inter, usize::from(above) + usize::from(left), &self.levels[plane][i * area..][..area], eob);
                let coded = u8::from(eob > 0);
                for k in 0..n4 {
                    self.contexts.above_nonzero[plane][ax + k] = if col4 + k < wide4 { coded } else { 0 };
                    self.left_nonzero[plane][ly + k] = if row4 + k < high4 { coded } else { 0 };
                }
            }
        }
    }
}
