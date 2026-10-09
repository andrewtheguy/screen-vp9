//! Intra prediction as a decoder makes it, which the encoder has to repeat
//! exactly: a block is predicted from pixels already reconstructed, and any
//! difference compounds down and right through the frame.

use super::transform::TxType;

/// VP9's ten intra modes, by their index.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum IntraMode {
    /// The mean of the pixels above and to the left.
    #[default]
    Dc = 0,
    /// The row above, down every row.
    V = 1,
    /// The column to the left, along every column.
    H = 2,
    /// The row above and the one to its right, down and to the left.
    D45 = 3,
    /// The row above and the column to the left, down and to the right.
    D135 = 4,
    /// Steeper than D135: two rows down for one column across.
    D117 = 5,
    /// Flatter than D135: two columns across for one row down.
    D153 = 6,
    /// The column to the left, across and down.
    D207 = 7,
    /// The row above, down and to the left, two rows down for one across.
    D63 = 8,
    /// Each pixel the one above it and the one left of it less the one
    /// above and left of the block ("true motion").
    Tm = 9,
}

impl IntraMode {
    /// Every mode, in the order they are tried.
    pub(super) const ALL: [IntraMode; 10] = [IntraMode::Dc, IntraMode::V, IntraMode::H, IntraMode::D45, IntraMode::D135, IntraMode::D117, IntraMode::D153, IntraMode::D207, IntraMode::D63, IntraMode::Tm];

    /// The transforms of a luma block predicted this way, below 32×32
    /// (libvpx's `intra_mode_to_tx_type_lookup`): an ADST across the
    /// direction the prediction runs from its edge.
    pub(super) fn tx_type(self) -> TxType {
        match self {
            IntraMode::Dc | IntraMode::D45 => TxType::DctDct,
            IntraMode::V | IntraMode::D117 | IntraMode::D63 => TxType::AdstDct,
            IntraMode::H | IntraMode::D153 | IntraMode::D207 => TxType::DctAdst,
            IntraMode::Tm | IntraMode::D135 => TxType::AdstAdst,
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
            IntraMode::D135 => &[(true, 0), (true, 1), (true, 2), (false, 3), (true, 4), (false, 5)],
            IntraMode::D117 => &[(true, 0), (true, 1), (true, 2), (false, 3), (true, 4), (true, 5)],
            IntraMode::D45 => &[(true, 0), (true, 1), (true, 2), (true, 3), (false, 6)],
            IntraMode::D63 => &[(true, 0), (true, 1), (true, 2), (true, 3), (true, 6), (false, 7)],
            IntraMode::D153 => &[(true, 0), (true, 1), (true, 2), (true, 3), (true, 6), (true, 7), (false, 8)],
            IntraMode::D207 => &[(true, 0), (true, 1), (true, 2), (true, 3), (true, 6), (true, 7), (true, 8)],
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
    /// Whether the row above goes on to the right of the block: a transform
    /// block short of its block's right edge (libvpx's `have_right`). Only
    /// a 4×4 reads it; a larger block repeats its last pixel above.
    pub(super) right: bool,
}

fn avg2(a: u8, b: u8) -> u8 {
    ((u32::from(a) + u32::from(b) + 1) >> 1) as u8
}

fn avg3(a: u8, b: u8, c: u8) -> u8 {
    ((u32::from(a) + 2 * u32::from(b) + u32::from(c) + 2) >> 2) as u8
}

/// Fill the `size`×`size` block at column `x` of the rows of `plane` from
/// its first with its prediction in `mode` from `above`, the row above the
/// block when there is one, and the column to the left when `edges` says
/// it is there (libvpx's `build_intra_predictors` and its predictors). The
/// row above is read as 127s where there is none, with the corner above and
/// left 129 where there is no column and 127 where there is no row, and the
/// column to the left as 129s where there is none; DC takes the mean of
/// whichever of them there are, or 128.
pub(super) fn intra(mode: IntraMode, above: Option<&[u8]>, plane: &mut [u8], stride: usize, x: usize, size: usize, edges: &Edges) {
    // The row above from the corner, twice the block's width.
    let mut top = [127u8; 1 + 64];
    let mut left = [129u8; 32];
    if let Some(row) = above {
        let at = |i: usize| row[(x + i).min(edges.width - 1)];
        for i in 0..size {
            top[1 + i] = at(i);
        }
        for i in size..2 * size {
            top[1 + i] = if size == 4 && edges.right { at(i) } else { top[size] };
        }
        top[0] = if edges.left { row[x - 1] } else { 129 };
    }
    if edges.left {
        for (i, l) in left[..size].iter_mut().enumerate() {
            *l = plane[i.min(edges.height - 1) * stride + x - 1];
        }
    }
    let (corner, a, l) = (top[0], &top[1..], &left[..size]);
    let mut out = [0u8; 32 * 32];
    let at = |r: usize, c: usize| r * size + c;
    match mode {
        IntraMode::Dc => {
            let log2 = size.ilog2();
            let sum = |px: &[u8]| px[..size].iter().map(|&v| u32::from(v)).sum::<u32>();
            let dc = match (above.is_some(), edges.left) {
                (true, true) => (sum(a) + sum(l) + size as u32) >> (log2 + 1),
                (true, false) => (sum(a) + size as u32 / 2) >> log2,
                (false, true) => (sum(l) + size as u32 / 2) >> log2,
                (false, false) => 128,
            };
            out[..size * size].fill(dc as u8);
        }
        IntraMode::V => {
            for r in 0..size {
                out[at(r, 0)..at(r, size)].copy_from_slice(&a[..size]);
            }
        }
        IntraMode::H => {
            for r in 0..size {
                out[at(r, 0)..at(r, size)].fill(l[r]);
            }
        }
        IntraMode::Tm => {
            for r in 0..size {
                for c in 0..size {
                    out[at(r, c)] = (i32::from(l[r]) + i32::from(a[c]) - i32::from(corner)).clamp(0, 255) as u8;
                }
            }
        }
        IntraMode::D45 if size == 4 => {
            let v = |i: usize| a[i];
            for (r, c, px) in [
                (0, 0, avg3(v(0), v(1), v(2))),
                (0, 1, avg3(v(1), v(2), v(3))),
                (0, 2, avg3(v(2), v(3), v(4))),
                (0, 3, avg3(v(3), v(4), v(5))),
                (1, 3, avg3(v(4), v(5), v(6))),
                (2, 3, avg3(v(5), v(6), v(7))),
                (3, 3, v(7)),
            ] {
                out[at(r, c)] = px;
            }
            for r in 1..4 {
                for c in 0..3 {
                    out[at(r, c)] = out[at(r - 1, c + 1)];
                }
            }
        }
        IntraMode::D45 => {
            let right = a[size - 1];
            for c in 0..size - 1 {
                out[c] = avg3(a[c], a[c + 1], a[c + 2]);
            }
            out[size - 1] = right;
            for r in 1..size {
                let n = size - 1 - r;
                let (done, rest) = out.split_at_mut(at(r, 0));
                rest[..n].copy_from_slice(&done[r..r + n]);
                rest[n..size].fill(right);
            }
        }
        IntraMode::D63 if size == 4 => {
            let v = |i: usize| a[i];
            for (r, c, px) in [
                (0, 0, avg2(v(0), v(1))),
                (0, 1, avg2(v(1), v(2))),
                (0, 2, avg2(v(2), v(3))),
                (0, 3, avg2(v(3), v(4))),
                (2, 3, avg2(v(4), v(5))),
                (1, 0, avg3(v(0), v(1), v(2))),
                (1, 1, avg3(v(1), v(2), v(3))),
                (1, 2, avg3(v(2), v(3), v(4))),
                (1, 3, avg3(v(3), v(4), v(5))),
                (3, 3, avg3(v(4), v(5), v(6))),
            ] {
                out[at(r, c)] = px;
            }
            for r in 2..4 {
                for c in 0..3 {
                    out[at(r, c)] = out[at(r - 2, c + 1)];
                }
            }
        }
        IntraMode::D63 => {
            for c in 0..size {
                out[c] = avg2(a[c], a[c + 1]);
                out[at(1, c)] = avg3(a[c], a[c + 1], a[c + 2]);
            }
            let last = a[size - 1];
            for r in (2..size).step_by(2) {
                let n = size - r / 2;
                let (done, rest) = out.split_at_mut(at(r, 0));
                rest[..n].copy_from_slice(&done[r / 2..r / 2 + n]);
                rest[n..size].fill(last);
                rest[size..size + n].copy_from_slice(&done[size + r / 2..size + r / 2 + n]);
                rest[size + n..2 * size].fill(last);
            }
        }
        IntraMode::D117 => {
            out[0] = avg2(corner, a[0]);
            for c in 1..size {
                out[c] = avg2(a[c - 1], a[c]);
            }
            out[at(1, 0)] = avg3(l[0], corner, a[0]);
            out[at(1, 1)] = avg3(corner, a[0], a[1]);
            for c in 2..size {
                out[at(1, c)] = avg3(a[c - 2], a[c - 1], a[c]);
            }
            out[at(2, 0)] = avg3(corner, l[0], l[1]);
            for r in 3..size {
                out[at(r, 0)] = avg3(l[r - 3], l[r - 2], l[r - 1]);
            }
            for r in 2..size {
                for c in 1..size {
                    out[at(r, c)] = out[at(r - 2, c - 1)];
                }
            }
        }
        IntraMode::D135 => {
            // The border from the bottom left, up the column, round the
            // corner and along the row; row `r` is the border from `size -
            // 1 - r`.
            let mut border = [0u8; 63];
            for i in 0..size - 2 {
                border[i] = avg3(l[size - 3 - i], l[size - 2 - i], l[size - 1 - i]);
            }
            border[size - 2] = avg3(corner, l[0], l[1]);
            border[size - 1] = avg3(l[0], corner, a[0]);
            border[size] = avg3(corner, a[0], a[1]);
            for i in 0..size - 2 {
                border[size + 1 + i] = avg3(a[i], a[i + 1], a[i + 2]);
            }
            for r in 0..size {
                out[at(r, 0)..at(r, size)].copy_from_slice(&border[size - 1 - r..2 * size - 1 - r]);
            }
        }
        IntraMode::D153 => {
            out[0] = avg2(corner, l[0]);
            for r in 1..size {
                out[at(r, 0)] = avg2(l[r - 1], l[r]);
            }
            out[1] = avg3(l[0], corner, a[0]);
            out[at(1, 1)] = avg3(corner, l[0], l[1]);
            for r in 2..size {
                out[at(r, 1)] = avg3(l[r - 2], l[r - 1], l[r]);
            }
            out[2] = avg3(corner, a[0], a[1]);
            for c in 1..size - 2 {
                out[2 + c] = avg3(a[c - 1], a[c], a[c + 1]);
            }
            for r in 1..size {
                for c in 2..size {
                    out[at(r, c)] = out[at(r - 1, c - 2)];
                }
            }
        }
        IntraMode::D207 => {
            for r in 0..size - 1 {
                out[at(r, 0)] = avg2(l[r], l[r + 1]);
            }
            out[at(size - 1, 0)] = l[size - 1];
            for r in 0..size - 2 {
                out[at(r, 1)] = avg3(l[r], l[r + 1], l[r + 2]);
            }
            out[at(size - 2, 1)] = avg3(l[size - 2], l[size - 1], l[size - 1]);
            out[at(size - 1, 1)] = l[size - 1];
            out[at(size - 1, 2)..at(size - 1, size)].fill(l[size - 1]);
            for r in (0..size - 1).rev() {
                for c in 2..size {
                    out[at(r, c)] = out[at(r + 1, c - 2)];
                }
            }
        }
    }
    for r in 0..size {
        plane[r * stride + x..][..size].copy_from_slice(&out[at(r, 0)..at(r, size)]);
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
        let edges = |y: usize, left| Edges { width: 10, height: 10 - y, left, right: false };
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
