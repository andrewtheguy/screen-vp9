//! One tile column of a frame: its superblocks in raster order, each a
//! partition tree down to the blocks that are coded, and each block's
//! reconstruction made as a decoder makes it.

use super::bool_writer::BoolWriter;
use super::predict::{self, Edges};
use super::quantize::Quantizer;
use super::tables::{KF_PARTITION_PROBS, KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, SKIP_PROBS};
use super::{Geometry, Reconstruction, tokens, transform};

/// `DC_PRED`'s index among the intra modes.
const DC_PRED: usize = 0;

/// A superblock's side in mode info units of 8×8, as log2.
const SB_LOG2: u32 = 3;

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

/// What a frame's tiles share: the contexts read across a tile's top edge,
/// and the skip flag of every 8×8 coded so far.
pub(super) struct FrameContexts {
    above_partition: Vec<u8>,
    /// Per plane, per column of 4×4: whether the transform block above had
    /// a coefficient.
    above_nonzero: [Vec<u8>; 3],
    skips: Vec<bool>,
}

impl FrameContexts {
    pub(super) fn new(geometry: &Geometry) -> Self {
        Self {
            above_partition: vec![0; geometry.sb_cols * 8],
            above_nonzero: std::array::from_fn(|_| vec![0; geometry.sb_cols * 16]),
            skips: vec![false; geometry.mi_rows * geometry.mi_cols],
        }
    }

    /// Back to how a decoder starts a frame.
    pub(super) fn clear(&mut self) {
        self.above_partition.fill(0);
        for plane in &mut self.above_nonzero {
            plane.fill(0);
        }
        self.skips.fill(false);
    }
}

/// The coder of one tile of a keyframe.
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
    quantizer: &'a Quantizer,
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
            quantizer,
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
                self.partition(mi_row, mi_col, SB_LOG2);
            }
        }
        self.w.finish()
    }

    /// The partition of the square of `1 << n` 8×8s at `(mi_row, mi_col)`.
    /// The block is whole where the frame holds all of it and split where it
    /// reaches past the edge.
    fn partition(&mut self, mi_row: usize, mi_col: usize, n: u32) {
        let g = self.geometry;
        if mi_row >= g.mi_rows || mi_col >= g.mi_cols {
            return;
        }
        let half = (1 << n) >> 1;
        let has_rows = mi_row + half < g.mi_rows;
        let has_cols = mi_col + half < g.mi_cols;
        let split = !(has_rows && has_cols);
        let probs = KF_PARTITION_PROBS[self.partition_context(mi_row, mi_col, n)];
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
        if split {
            for (r, c) in [(0, 0), (0, half), (half, 0), (half, half)] {
                self.partition(mi_row + r, mi_col + c, n - 1);
            }
        } else {
            self.block(mi_row, mi_col, n);
            let bw = 1 << n;
            let ctx = partition_context_of(n);
            self.contexts.above_partition[mi_col..mi_col + bw].fill(ctx);
            self.left_partition[mi_row & 7..(mi_row & 7) + bw].fill(ctx);
        }
    }

    /// libvpx's `partition_plane_context`.
    fn partition_context(&self, mi_row: usize, mi_col: usize, n: u32) -> usize {
        let above = (self.contexts.above_partition[mi_col] >> n) & 1;
        let left = (self.left_partition[mi_row & 7] >> n) & 1;
        usize::from(left * 2 + above) + n as usize * 4
    }

    /// Code the square block of `1 << n` 8×8s at `(mi_row, mi_col)`: DC
    /// predicted, at the transform size `ALLOW_32X32` gives it, and skipped
    /// when no transform block keeps a coefficient.
    fn block(&mut self, mi_row: usize, mi_col: usize, n: u32) {
        let g = self.geometry;
        self.reconstruct(mi_row, mi_col, n);
        let skip = self.eobs.iter().flatten().all(|&eob| eob == 0);
        let above = mi_row > 0 && self.contexts.skips[(mi_row - 1) * g.mi_cols + mi_col];
        let left = mi_col > self.mi_col_start && self.contexts.skips[mi_row * g.mi_cols + mi_col - 1];
        self.w.write(skip, SKIP_PROBS[usize::from(above) + usize::from(left)]);
        // The luma mode's probabilities are chosen by the modes above and to
        // the left, which are DC in every block this encoder codes and DC by
        // rule where there is no block.
        self.w.write(false, KF_Y_MODE_PROBS[DC_PRED][DC_PRED][0]);
        self.w.write(false, KF_UV_MODE_PROBS[DC_PRED][0]);
        let bw = 1 << n;
        for row in mi_row..(mi_row + bw).min(g.mi_rows) {
            self.contexts.skips[row * g.mi_cols + mi_col..][..bw.min(g.mi_cols - mi_col)].fill(skip);
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
            self.tokens(mi_row, mi_col, n);
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

    /// Predict, code and reconstruct the block's transform blocks, each from
    /// the ones before it, keeping their levels to be written.
    fn reconstruct(&mut self, mi_row: usize, mi_col: usize, n: u32) {
        let g = self.geometry;
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
            for (i, &(x, y)) in blocks.iter().enumerate() {
                let (px, py) = (mi_col * 8 + x, mi_row * 8 + y);
                let recon = &mut self.recon.planes[plane];
                let edges = Edges { width: g.mi_cols * 8, height: g.mi_rows * 8, above: y > 0 || mi_row > 0, left: x > 0 || mi_col > self.mi_col_start };
                predict::dc(recon, stride, px, py, tx, &edges);
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
    fn tokens(&mut self, mi_row: usize, mi_col: usize, n: u32) {
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
                tokens::write(&mut self.w, tx_log2, plane > 0, false, usize::from(above) + usize::from(left), &self.levels[plane][i * area..][..area], eob);
                let coded = u8::from(eob > 0);
                for k in 0..n4 {
                    self.contexts.above_nonzero[plane][ax + k] = if col4 + k < wide4 { coded } else { 0 };
                    self.left_nonzero[plane][ly + k] = if row4 + k < high4 { coded } else { 0 };
                }
            }
        }
    }
}
