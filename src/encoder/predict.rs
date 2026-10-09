//! Intra prediction as a decoder makes it, which the encoder has to repeat
//! exactly: a block is predicted from pixels already reconstructed, and any
//! difference compounds down and right through the frame.

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
/// its first with the DC prediction: the rounded mean of `above`, the row
/// above the block when there is one, and the column to the left, of
/// whichever of them there is, or 128 with neither (libvpx's
/// `build_intra_predictors` and `vpx_dc_*predictor`).
pub(super) fn dc(above: Option<&[u8]>, plane: &mut [u8], stride: usize, x: usize, size: usize, edges: &Edges) {
    let log2 = size.ilog2();
    let above = above.map(|row| (0..size).map(|i| usize::from(row[(x + i).min(edges.width - 1)])).sum::<usize>());
    let left = || -> usize { (0..size).map(|i| usize::from(plane[i.min(edges.height - 1) * stride + x - 1])).sum() };
    let dc = match (above, edges.left) {
        (Some(above), true) => (above + left() + size) >> (log2 + 1),
        (Some(above), false) => (above + size / 2) >> log2,
        (None, true) => (left() + size / 2) >> log2,
        (None, false) => 128,
    };
    for row in 0..size {
        plane[row * stride + x..][..size].fill(dc as u8);
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
        dc(Some(&above), rows, stride, 8, 4, &edges(4, false));
        assert_eq!(plane[4 * stride + 8], ((40 + 80 * 3 + 2) / 4) as u8);
        // Left: rows 4 to 7, inside the frame.
        dc(None, &mut plane[4 * stride..], stride, 8, 4, &edges(4, true));
        assert_eq!(plane[7 * stride + 11], ((40 + 50 + 60 + 70 + 2) / 4) as u8);
        // Both at row 8: above, column 9 repeated; left, row 9 repeated.
        plane[7 * stride + 8..][..4].copy_from_slice(&[1, 2, 3, 4]);
        let (above, rows) = at(&mut plane, stride, 8);
        dc(Some(&above), rows, stride, 8, 4, &edges(8, true));
        assert_eq!(plane[8 * stride + 8], ((1 + 2 + 2 + 2 + 80 + 90 + 90 + 90 + 4) / 8) as u8);
        dc(None, &mut plane[8 * stride..], stride, 8, 4, &edges(8, false));
        assert!(plane[8 * stride + 8..][..4].iter().all(|&p| p == 128));
    }
}
