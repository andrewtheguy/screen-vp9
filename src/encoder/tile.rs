//! One tile column of a frame: its superblocks in raster order, each a
//! partition tree down to the blocks that are coded, and each block's
//! reconstruction made as a decoder makes it.

use super::bool_writer::BoolWriter;
use super::predict::{self, Edges};
use super::tables::{KF_PARTITION_PROBS, KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, SKIP_PROBS};
use super::{Geometry, Reconstruction};

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

/// What a frame's tiles share: the contexts read across a tile's top edge,
/// one entry per 8×8 column, and the skip flag of every 8×8 coded so far.
pub(super) struct FrameContexts {
    above_partition: Vec<u8>,
    skips: Vec<bool>,
}

impl FrameContexts {
    pub(super) fn new(geometry: &Geometry) -> Self {
        Self { above_partition: vec![0; geometry.sb_cols * 8], skips: vec![false; geometry.mi_rows * geometry.mi_cols] }
    }

    /// Back to how a decoder starts a frame.
    pub(super) fn clear(&mut self) {
        self.above_partition.fill(0);
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
    recon: &'a mut Reconstruction,
    w: BoolWriter,
}

impl<'a> Tile<'a> {
    pub(super) fn new(geometry: &'a Geometry, cols: std::ops::Range<usize>, contexts: &'a mut FrameContexts, recon: &'a mut Reconstruction) -> Self {
        Self { geometry, mi_col_start: cols.start, mi_col_end: cols.end, contexts, left_partition: [0; 8], recon, w: BoolWriter::new() }
    }

    /// Code every superblock of the tile and give back its bytes.
    pub(super) fn code(mut self) -> Vec<u8> {
        for mi_row in (0..self.geometry.mi_rows).step_by(8) {
            self.left_partition = [0; 8];
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

    /// Code the square block of `1 << n` 8×8s at `(mi_row, mi_col)`: skipped,
    /// DC predicted, at the transform size `ALLOW_32X32` gives it.
    fn block(&mut self, mi_row: usize, mi_col: usize, n: u32) {
        let g = self.geometry;
        let skip = true;
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
        self.reconstruct(mi_row, mi_col, n);
    }

    /// Predict the block's transform blocks in raster order, each from the
    /// ones before it. A transform block starting past the frame's edge,
    /// rounded up to 8, is not coded and not predicted.
    fn reconstruct(&mut self, mi_row: usize, mi_col: usize, n: u32) {
        let g = self.geometry;
        let size = 8 << n;
        let tx = size.min(32);
        let wide = size.min((g.mi_cols - mi_col) * 8);
        let high = size.min((g.mi_rows - mi_row) * 8);
        let stride = self.recon.stride;
        for plane in &mut self.recon.planes {
            for y in (0..high).step_by(tx) {
                for x in (0..wide).step_by(tx) {
                    let edges = Edges { width: g.mi_cols * 8, height: g.mi_rows * 8, above: y > 0 || mi_row > 0, left: x > 0 || mi_col > self.mi_col_start };
                    predict::dc(plane, stride, mi_col * 8 + x, mi_row * 8 + y, tx, &edges);
                }
            }
        }
    }
}
