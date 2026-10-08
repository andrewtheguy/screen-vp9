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
//! where it is and DC, and takes the one it varies less from. A square whose
//! source is what it was in the previous frame is the previous frame's,
//! skipped, without being measured; one that is so only in part is split.

use super::bool_writer::BoolWriter;
use super::predict::{self, Edges};
use super::quantize::Quantizer;
use super::tables::{
    INTER_MODE_PROBS, INTRA_INTER_PROBS, KF_PARTITION_PROBS, KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, PARTITION_PROBS, SINGLE_REF_PROBS, SKIP_PROBS,
    UV_MODE_PROBS, Y_MODE_PROBS,
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

/// Per square size, 8×8 to 64×64, the two neighbours whose modes give an
/// inter block's mode context, as (row, column) offsets in 8×8s: the first
/// two of libvpx's `mv_ref_blocks`.
const MODE_NEIGHBOURS: [[(isize, isize); 2]; 4] = [[(-1, 0), (0, -1)], [(-1, 0), (0, -1)], [(-1, 1), (1, -1)], [(-1, 3), (3, -1)]];

/// What a neighbour adds to the mode context's counter (libvpx's
/// `mode_2_counter`): an intra block, a `NEARESTMV` one and a `ZEROMV` one.
const INTRA_COUNT: usize = 9;
const NEARESTMV_COUNT: usize = 0;
const ZEROMV_COUNT: usize = 3;

/// How far, in 8×8s, a block may reach past the frame's bottom or right edge
/// before a decoder clamps the vectors it predicts for it away from zero
/// (libvpx's `clamp_mv_ref`, which allows 16 pixels).
const MV_BORDER: usize = 2;

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
/// has it, and per 8×8 of the frame, whether the source there is the
/// previous frame's.
pub(super) struct Previous<'a> {
    pub(super) reference: &'a Reconstruction,
    pub(super) unchanged: &'a [bool],
}

/// How a block is coded.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// DC prediction.
    Intra,
    /// The previous frame where the block is, and the residual against it.
    Zero,
    /// The previous frame where the block is, as it is: the source has not
    /// changed there, so nothing is coded but the skip.
    Unchanged,
}

/// What a decoder keeps of a coded 8×8 that the blocks after it read.
#[derive(Clone, Copy, Default)]
struct ModeInfo {
    skip: bool,
    inter: bool,
    /// An inter block coded as `ZEROMV` rather than `NEARESTMV`.
    zeromv: bool,
}

/// What a frame's tiles share: the contexts read across a tile's top edge,
/// and the mode info of every 8×8 coded so far.
pub(super) struct FrameContexts {
    above_partition: Vec<u8>,
    /// Per plane, per column of 4×4: whether the transform block above had
    /// a coefficient.
    above_nonzero: [Vec<u8>; 3],
    mode_info: Vec<ModeInfo>,
}

impl FrameContexts {
    pub(super) fn new(geometry: &Geometry) -> Self {
        Self {
            above_partition: vec![0; geometry.sb_cols * 8],
            above_nonzero: std::array::from_fn(|_| vec![0; geometry.sb_cols * 16]),
            mode_info: vec![ModeInfo::default(); geometry.mi_rows * geometry.mi_cols],
        }
    }

    /// Back to how a decoder starts a frame.
    pub(super) fn clear(&mut self) {
        self.above_partition.fill(0);
        for plane in &mut self.above_nonzero {
            plane.fill(0);
        }
        self.mode_info.fill(ModeInfo::default());
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

/// The eight pixels of `row` from `x`, the last pixel repeated past its end.
fn eight(row: &[u8], x: usize) -> [u8; 8] {
    std::array::from_fn(|i| row[(x + i).min(row.len() - 1)])
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
    /// that the frame holds, of the source as it is coded: past the
    /// picture's edge, its last pixel repeated, and the previous frame's the
    /// same way, as a decoder reads it.
    fn measure(&mut self, mi_row: usize, mi_col: usize) {
        let src = self.source;
        let (rows, cols) = self.inside(mi_row, mi_col, SB_LOG2);
        for r in rows {
            for c in cols.clone() {
                let (px, py) = ((mi_col + c) * 8, (mi_row + r) * 8);
                let mut m = Measure::default();
                for plane in 0..3 {
                    for y in py..py + 8 {
                        let y = y.min(src.height - 1);
                        let s = eight(&src.planes[plane][y * src.stride..][..src.width], px);
                        for v in s {
                            m.source.sums[plane] += i64::from(v);
                            m.source.squares += i64::from(v) * i64::from(v);
                        }
                        if let Some(previous) = &self.previous {
                            let reference = previous.reference;
                            let p = eight(&reference.planes[plane][y * reference.stride..][..src.width], px);
                            for (a, b) in s.into_iter().zip(p) {
                                let d = i64::from(a) - i64::from(b);
                                m.residual.sums[plane] += d;
                                m.residual.squares += d * d;
                            }
                        }
                    }
                }
                self.measures[r * 8 + c] = m;
            }
        }
    }

    /// How the square of `1 << n` 8×8s at `(mi_row, mi_col)` is coded
    /// whole, or none when it varies more from its best prediction than a
    /// square is coded whole at.
    fn choose(&self, mi_row: usize, mi_col: usize, n: u32) -> Option<Mode> {
        match self.count_unchanged(mi_row, mi_col, n) {
            Some(u) if u == self.count_inside(mi_row, mi_col, n) => return Some(Mode::Unchanged),
            // Partly unchanged: split, so the unchanged part stays the
            // previous frame's rather than coding again what the previous
            // frame got wrong there.
            Some(u) if u > 0 && n > 0 => return None,
            _ => {}
        }
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
            Some(_) if total.residual.variance(count) <= intra => (Mode::Zero, total.residual.variance(count)),
            _ => (Mode::Intra, intra),
        };
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

    /// The mode context of an inter block (libvpx's `get_mode_context`):
    /// what its two nearest neighbours in the tile were coded as.
    fn mode_context(&self, mi_row: usize, mi_col: usize, n: u32) -> usize {
        let g = self.geometry;
        let counter: usize = MODE_NEIGHBOURS[n as usize]
            .iter()
            .filter_map(|&(dr, dc)| {
                let (r, c) = (mi_row.checked_add_signed(dr)?, mi_col.checked_add_signed(dc)?);
                (r < g.mi_rows && (self.mi_col_start..self.mi_col_end).contains(&c)).then(|| self.contexts.mode_info[r * g.mi_cols + c])
            })
            .map(|m| match (m.inter, m.zeromv) {
                (false, _) => INTRA_COUNT,
                (true, false) => NEARESTMV_COUNT,
                (true, true) => ZEROMV_COUNT,
            })
            .sum();
        COUNTER_TO_CONTEXT[counter]
    }

    /// Code the square block of `1 << n` 8×8s at `(mi_row, mi_col)` in
    /// `mode`, at the transform size `ALLOW_32X32` gives it, and skipped
    /// when no transform block keeps a coefficient.
    fn block(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode) {
        let g = self.geometry;
        self.reconstruct(mi_row, mi_col, n, mode);
        let skip = self.eobs.iter().flatten().all(|&eob| eob == 0);
        let inter = mode != Mode::Intra;
        let above = (mi_row > 0).then(|| self.contexts.mode_info[(mi_row - 1) * g.mi_cols + mi_col]);
        let left = (mi_col > self.mi_col_start).then(|| self.contexts.mode_info[mi_row * g.mi_cols + mi_col - 1]);
        let skips = usize::from(above.is_some_and(|m| m.skip)) + usize::from(left.is_some_and(|m| m.skip));
        // The zero vector, as NEARESTMV where a decoder finds it to be the
        // nearest: while every vector in the stream is zero, so is every
        // candidate, but a block reaching far past the frame's edge has
        // its candidates clamped away from zero and has to say ZEROMV. The
        // default probabilities expect NEARESTMV: beside two zero vectors
        // ZEROMV costs 7 bits, NEARESTMV a little over half of one, and
        // NEARESTMV is the cheaper in every context.
        let bw = 1 << n;
        let zeromv = inter && (mi_row + bw > g.mi_rows + MV_BORDER || mi_col + bw > g.mi_cols + MV_BORDER);
        self.w.write(skip, SKIP_PROBS[skips]);
        if self.previous.is_none() {
            // The luma mode's probabilities are chosen by the modes above and
            // to the left, which are DC in every block of a keyframe this
            // encoder codes and DC by rule where there is no block.
            self.w.write(false, KF_Y_MODE_PROBS[DC_PRED][DC_PRED][0]);
            self.w.write(false, KF_UV_MODE_PROBS[DC_PRED][0]);
        } else {
            // libvpx's `get_intra_inter_context`.
            let ctx = match (above, left) {
                (Some(a), Some(l)) if !a.inter && !l.inter => 3,
                (Some(a), Some(l)) => usize::from(!a.inter || !l.inter),
                (Some(m), None) | (None, Some(m)) => 2 * usize::from(!m.inter),
                (None, None) => 0,
            };
            self.w.write(inter, INTRA_INTER_PROBS[ctx]);
            if inter {
                // LAST, the reference tree's first leaf, at libvpx's
                // `vp9_get_pred_context_single_ref_p1` when LAST is the only
                // reference: 4 beside an inter block, 2 otherwise.
                let ctx = if above.is_some_and(|m| m.inter) || left.is_some_and(|m| m.inter) { 4 } else { 2 };
                self.w.write(false, SINGLE_REF_PROBS[ctx][0]);
                // ZEROMV is the inter mode tree's first leaf, NEARESTMV its
                // second.
                let probs = INTER_MODE_PROBS[self.mode_context(mi_row, mi_col, n)];
                self.w.write(!zeromv, probs[0]);
                if !zeromv {
                    self.w.write(false, probs[1]);
                }
            } else {
                // DC, at the probabilities of the block's size group.
                self.w.write(false, Y_MODE_PROBS[(n as usize + 1).min(3)][0]);
                self.w.write(false, UV_MODE_PROBS[DC_PRED][0]);
            }
        }
        for row in mi_row..(mi_row + bw).min(g.mi_rows) {
            self.contexts.mode_info[row * g.mi_cols + mi_col..][..bw.min(g.mi_cols - mi_col)].fill(ModeInfo { skip, inter, zeromv });
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
            self.tokens(mi_row, mi_col, n, inter);
        }
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

    /// Predict the block from the previous frame where it is, over the part
    /// of it inside the frame's edge rounded up to 8: as a decoder does, past
    /// the picture's edge the previous frame's last pixel repeats.
    fn predict_inter(&mut self, mi_row: usize, mi_col: usize, n: u32) {
        let g = self.geometry;
        let reference = self.previous.as_ref().expect("an inter frame").reference;
        let (width, height) = (self.source.width, self.source.height);
        let size = 8 << n;
        let (px, py) = (mi_col * 8, mi_row * 8);
        let wide = size.min(g.mi_cols * 8 - px);
        let high = size.min(g.mi_rows * 8 - py);
        // The picture's edge is never left of a block's corner.
        let copied = wide.min(width - px);
        let stride = self.recon.stride;
        for (plane, recon) in self.recon.planes.iter_mut().enumerate() {
            for y in py..py + high {
                let from = &reference.planes[plane][y.min(height - 1) * reference.stride..][..width];
                let to = &mut recon[y * stride + px..][..wide];
                to[..copied].copy_from_slice(&from[px..px + copied]);
                to[copied..].fill(from[width - 1]);
            }
        }
    }

    /// Predict, code and reconstruct the block's transform blocks, each from
    /// the ones before it, keeping their levels to be written.
    fn reconstruct(&mut self, mi_row: usize, mi_col: usize, n: u32, mode: Mode) {
        let g = self.geometry;
        if mode != Mode::Intra {
            self.predict_inter(mi_row, mi_col, n);
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
