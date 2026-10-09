//! Intra prediction as a decoder makes it, which the encoder has to repeat
//! exactly: a block is predicted from pixels already reconstructed, and any
//! difference compounds down and right through the frame.

use super::transform::TxType;

/// The intra modes this encoder codes, by their index among VP9's ten.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum IntraMode {
    /// The mean of the pixels above and to the left.
    #[default]
    Dc = 0,
    /// The row above, down every row.
    V = 1,
    /// The column to the left, along every column.
    H = 2,
    /// Each pixel the one above it and the one left of it less the one
    /// above and left of the block ("true motion").
    Tm = 9,
}

impl IntraMode {
    pub(super) const ALL: [IntraMode; 4] = [IntraMode::Dc, IntraMode::V, IntraMode::H, IntraMode::Tm];

    /// The transforms of a luma block predicted this way, below 32×32
    /// (libvpx's `intra_mode_to_tx_type_lookup`): an ADST across the
    /// direction the prediction runs from its edge.
    pub(super) fn tx_type(self) -> TxType {
        match self {
            IntraMode::Dc => TxType::DctDct,
            IntraMode::V => TxType::AdstDct,
            IntraMode::H => TxType::DctAdst,
            IntraMode::Tm => TxType::AdstAdst,
        }
    }

    /// The bools of the intra mode tree (libvpx's `vp9_intra_mode_tree`)
    /// down to this leaf, each with its node.
    pub(super) fn bits(self) -> &'static [(bool, usize)] {
        match self {
            IntraMode::Dc => &[(false, 0)],
            IntraMode::Tm => &[(true, 0), (false, 1)],
            IntraMode::V => &[(true, 0), (true, 1), (false, 2)],
            IntraMode::H => &[(true, 0), (true, 1), (true, 2), (false, 3), (false, 4)],
        }
    }
}

/// What a transform block may read around it.
pub(super) struct Edges {
    /// The plane's width rounded up to 8, and its height so rounded less the
    /// block's first row, which is how far a decoder reads; it repeats the
    /// last pixel inside them beyond.
    pub(super) width: usize,
    pub(super) height: usize,
    /// Whether the column to the left is there: inside the block, or a block
    /// to the left in the same tile.
    pub(super) left: bool,
}

/// Fill the `size`×`size` block at column `x` of the rows of `plane` from
/// its first with its prediction in `mode` from `above`, the row above the
/// block when there is one, and the column to the left when `edges` says
/// it is there (libvpx's `build_intra_predictors` and its predictors):
///
/// - DC: the rounded mean of the row above and the column to the left, of
///   whichever of them there is, or 128 with neither;
/// - V, H and TM: a missing row above as 127s and a missing column to the
///   left as 129s, and TM's corner from the row above, 129 there without
///   the column and 127 without the row.
pub(super) fn intra(mode: IntraMode, above: Option<&[u8]>, plane: &mut [u8], stride: usize, x: usize, size: usize, edges: &Edges) {
    let top = |i: usize| above.map_or(127, |row| i32::from(row[(x + i).min(edges.width - 1)]));
    let left = |plane: &[u8], i: usize| if edges.left { i32::from(plane[i.min(edges.height - 1) * stride + x - 1]) } else { 129 };
    match mode {
        IntraMode::Dc => {
            let log2 = size.ilog2();
            let above = above.map(|_| (0..size).map(top).sum::<i32>());
            let left = || -> i32 { (0..size).map(|i| left(plane, i)).sum() };
            let dc = match (above, edges.left) {
                (Some(above), true) => (above + left() + size as i32) >> (log2 + 1),
                (Some(above), false) => (above + size as i32 / 2) >> log2,
                (None, true) => (left() + size as i32 / 2) >> log2,
                (None, false) => 128,
            };
            for row in 0..size {
                plane[row * stride + x..][..size].fill(dc as u8);
            }
        }
        IntraMode::V => {
            let row: [u8; 32] = std::array::from_fn(|i| if i < size { top(i) as u8 } else { 0 });
            for r in 0..size {
                plane[r * stride + x..][..size].copy_from_slice(&row[..size]);
            }
        }
        IntraMode::H => {
            for r in 0..size {
                let v = left(plane, r) as u8;
                plane[r * stride + x..][..size].fill(v);
            }
        }
        IntraMode::Tm => {
            let corner = match above {
                Some(row) if edges.left => i32::from(row[x - 1]),
                Some(_) => 129,
                None => 127,
            };
            let row: [i32; 32] = std::array::from_fn(|i| if i < size { top(i) - corner } else { 0 });
            for r in 0..size {
                let l = left(plane, r);
                for (p, &t) in plane[r * stride + x..][..size].iter_mut().zip(&row) {
                    *p = (l + t).clamp(0, 255) as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_averages_what_is_there_and_repeats_the_last_pixel_past_the_edge() {
        // A 12×12 plane whose frame is 10×10: row 3 above a 4×4 block at
        // (8, 4), column 7 to its left.
        let stride = 12;
        let mut plane = vec![0u8; stride * 12];
        plane[3 * stride + 8] = 40;
        plane[3 * stride + 9] = 80;
        for row in 4..12 {
            plane[row * stride + 7] = row as u8 * 10;
        }
        let edges = |y: usize, left| Edges { width: 10, height: 10 - y, left };
        // The row above row `y`, and the rows from it.
        fn at(plane: &mut [u8], stride: usize, y: usize) -> (Vec<u8>, &mut [u8]) {
            (plane[(y - 1) * stride..][..stride].to_vec(), &mut plane[y * stride..])
        }
        // Above: 40, 80, then 80 twice past the edge.
        let (above, rows) = at(&mut plane, stride, 4);
        intra(IntraMode::Dc, Some(&above), rows, stride, 8, 4, &edges(4, false));
        assert_eq!(plane[4 * stride + 8], ((40 + 80 * 3 + 2) / 4) as u8);
        // Left: rows 4 to 7, inside the frame.
        intra(IntraMode::Dc, None, &mut plane[4 * stride..], stride, 8, 4, &edges(4, true));
        assert_eq!(plane[7 * stride + 11], ((40 + 50 + 60 + 70 + 2) / 4) as u8);
        // Both at row 8: above, column 9 repeated; left, row 9 repeated.
        plane[7 * stride + 8..][..4].copy_from_slice(&[1, 2, 3, 4]);
        let (above, rows) = at(&mut plane, stride, 8);
        intra(IntraMode::Dc, Some(&above), rows, stride, 8, 4, &edges(8, true));
        assert_eq!(plane[8 * stride + 8], ((1 + 2 + 2 + 2 + 80 + 90 + 90 + 90 + 4) / 8) as u8);
        intra(IntraMode::Dc, None, &mut plane[8 * stride..], stride, 8, 4, &edges(8, false));
        assert!(plane[8 * stride + 8..][..4].iter().all(|&p| p == 128));
    }
}
