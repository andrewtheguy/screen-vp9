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
    /// Whether this mode is not worth trying after `best`, the best so
    /// far (libvpx's `conditional_skipintra`): a steep diagonal pays only
    /// where a mode on its side of the quadrant has.
    pub(super) fn pointless_after(self, best: IntraMode) -> bool {
        use IntraMode::*;
        match self {
            D117 => best != V && best != D135,
            D63 => best != V && best != D45,
            D207 => best != H && best != D45,
            D153 => best != H && best != D135,
            _ => false,
        }
    }

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
    // Each size its own code, so that its loops unroll.
    match size {
        4 => fill::<4>(mode, above, plane, stride, x, edges),
        8 => fill::<8>(mode, above, plane, stride, x, edges),
        16 => fill::<16>(mode, above, plane, stride, x, edges),
        _ => fill::<32>(mode, above, plane, stride, x, edges),
    }
}

/// `intra` at one size.
fn fill<const N: usize>(mode: IntraMode, above: Option<&[u8]>, plane: &mut [u8], stride: usize, x: usize, edges: &Edges) {
    let size = N;
    // The row above from the corner, twice the block's width.
    let mut top = [127u8; 1 + 64];
    let mut left = [129u8; 32];
    if let Some(row) = above {
        // As far as the row is read, and its last pixel after that; a row
        // short of the picture's edge is copied a pixel at a time.
        let read = if size == 4 && edges.right { 2 * size } else { size };
        if x + read <= edges.width {
            top[1..1 + N].copy_from_slice(&row[x..x + N]);
            if read > size {
                top[1 + N..1 + 2 * N].copy_from_slice(&row[x + N..x + 2 * N]);
            }
        } else {
            for i in 0..read {
                top[1 + i] = row[(x + i).min(edges.width - 1)];
            }
        }
        if read == size {
            let last = top[size];
            top[1 + N..1 + 2 * N].fill(last);
        }
        top[0] = if edges.left { row[x - 1] } else { 129 };
    }
    if edges.left {
        let column = &plane[x - 1..];
        if size <= edges.height {
            let column = &column[..(N - 1) * stride + 1];
            for (i, l) in left[..N].iter_mut().enumerate() {
                *l = column[i * stride];
            }
        } else {
            for (i, l) in left[..N].iter_mut().enumerate() {
                *l = column[i.min(edges.height - 1) * stride];
            }
        }
    }
    let (corner, a, l) = (top[0], &top[1..], &left[..size]);
    // The block is written in place a row at a time, row `r` from `at(r)`:
    // the residual after it reads a row in one, and waits on a row written
    // in pieces.
    let at = |r: usize| r * stride + x;
    let out = plane;
    // The diagonal modes are each a line or two of means along the edge,
    // every row a stretch of them: `line[i]` is the mean of three from `i`
    // of the edge, `pairs` the means of two with them, turn about.
    let mut line = [0u8; 96];
    let mut pairs = [0u8; 96];
    // The edge from the bottom of the column up it, round the corner and
    // along the row above, for the modes that read both.
    let border = || {
        let mut border = [0u8; 32 + 65];
        for i in 0..N {
            border[i] = l[N - 1 - i];
        }
        border[N..2 * N + 1].copy_from_slice(&top[..N + 1]);
        border
    };
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
            for r in 0..size {
                out[at(r)..at(r) + N].fill(dc as u8);
            }
        }
        IntraMode::V => {
            for r in 0..size {
                out[at(r)..at(r) + N].copy_from_slice(&a[..N]);
            }
        }
        IntraMode::H => {
            for r in 0..size {
                out[at(r)..at(r) + N].fill(l[r]);
            }
        }
        IntraMode::Tm => {
            // A row at a time, for the compiler to take sixteen pixels of
            // it a register.
            for r in 0..size {
                let rise = i16::from(l[r]) - i16::from(corner);
                for (px, &a) in out[at(r)..at(r) + N].iter_mut().zip(&a[..N]) {
                    *px = (rise + i16::from(a)).clamp(0, 255) as u8;
                }
            }
        }
        IntraMode::D45 => {
            // Row `r` is the means from `r`, and past the row above its
            // last pixel: a 4×4 reads eight of it, a larger block its own
            // width.
            let means = if size == 4 { 6 } else { size - 1 };
            for i in 0..means {
                line[i] = avg3(a[i], a[i + 1], a[i + 2]);
            }
            line[means..2 * N].fill(a[if size == 4 { 7 } else { size - 1 }]);
            for r in 0..size {
                out[at(r)..at(r) + N].copy_from_slice(&line[r..r + N]);
            }
        }
        IntraMode::D63 => {
            // Two rows down for one across: the even rows the means of
            // two, the odd of three, each pair of rows from one further
            // along, a block over 4×4 with the last pixel above it past
            // its width.
            let means = if size == 4 { 5 } else { size };
            for i in 0..means {
                pairs[i] = avg2(a[i], a[i + 1]);
                line[i] = avg3(a[i], a[i + 1], a[i + 2]);
            }
            pairs[means..N + N / 2].fill(a[size - 1]);
            line[means..N + N / 2].fill(a[size - 1]);
            for r in (0..size).step_by(2) {
                out[at(r)..at(r) + N].copy_from_slice(&pairs[r / 2..r / 2 + N]);
                out[at(r + 1)..at(r + 1) + N].copy_from_slice(&line[r / 2..r / 2 + N]);
            }
        }
        IntraMode::D117 => {
            // Two rows down for one across the other way: the even rows
            // the means of two along the row above, the odd of three, and
            // left of them as a pair of rows moves along, the means of
            // three down the column, every other one to each.
            let border = border();
            let mut means = [0u8; 64];
            for i in 0..2 * N - 1 {
                means[i] = avg3(border[i], border[i + 1], border[i + 2]);
            }
            for c in 0..N {
                pairs[N / 2 + c] = avg2(top[c], top[c + 1]);
                line[N / 2 + c] = means[N - 1 + c];
            }
            for m in 1..N / 2 {
                pairs[N / 2 - m] = means[N - 2 * m];
                line[N / 2 - m] = means[N - 1 - 2 * m];
            }
            for r in (0..size).step_by(2) {
                let from = N / 2 - r / 2;
                out[at(r)..at(r) + N].copy_from_slice(&pairs[from..from + N]);
                out[at(r + 1)..at(r + 1) + N].copy_from_slice(&line[from..from + N]);
            }
        }
        IntraMode::D135 => {
            // Row `r` is the means of three along the border from `size -
            // 1 - r`.
            let border = border();
            for i in 0..2 * N - 1 {
                line[i] = avg3(border[i], border[i + 1], border[i + 2]);
            }
            for r in 0..size {
                out[at(r)..at(r) + N].copy_from_slice(&line[N - 1 - r..2 * N - 1 - r]);
            }
        }
        IntraMode::D153 => {
            // Two columns across for one row down: up the column a mean
            // of two and one of three turn about, and the means of three
            // along the row above after them; row `r` from two short of
            // the row before it.
            let border = border();
            for i in 0..N {
                line[2 * i] = avg2(border[i], border[i + 1]);
                line[2 * i + 1] = avg3(border[i], border[i + 1], border[i + 2]);
            }
            for i in 0..N - 2 {
                line[2 * N + i] = avg3(border[N + i], border[N + i + 1], border[N + i + 2]);
            }
            for r in 0..size {
                let from = 2 * (N - 1 - r);
                out[at(r)..at(r) + N].copy_from_slice(&line[from..from + N]);
            }
        }
        IntraMode::D207 => {
            // The same down the column, its last pixel after it: row `r`
            // from two further along.
            let mut column = [l[size - 1]; 34];
            column[..N].copy_from_slice(l);
            for i in 0..N {
                line[2 * i] = avg2(column[i], column[i + 1]);
                line[2 * i + 1] = avg3(column[i], column[i + 1], column[i + 2]);
            }
            line[2 * N..3 * N].fill(l[size - 1]);
            for r in 0..size {
                out[at(r)..at(r) + N].copy_from_slice(&line[2 * r..2 * r + N]);
            }
        }
    }
}

impl IntraMode {
    /// Whether the mode reads the row above, and whether the column to the
    /// left (libvpx's `extend_modes`); predicted without an edge it reads,
    /// a mode says little that DC does not.
    pub(super) fn reads(self) -> (bool, bool) {
        match self {
            IntraMode::Dc | IntraMode::Tm | IntraMode::D135 | IntraMode::D117 | IntraMode::D153 => (true, true),
            IntraMode::V | IntraMode::D45 | IntraMode::D63 => (true, false),
            IntraMode::H | IntraMode::D207 => (false, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `intra` a pixel at a time, as libvpx's predictors are written.
    fn a_pixel_at_a_time(mode: IntraMode, above: Option<&[u8]>, plane: &mut [u8], stride: usize, x: usize, size: usize, edges: &Edges) {
        // The row above from the corner, twice the block's width.
        let mut top = [127u8; 1 + 64];
        let mut left = [129u8; 32];
        if let Some(row) = above {
            // As far as the row is read, and its last pixel after that; a row
            // short of the picture's edge is copied whole.
            let read = if size == 4 && edges.right { 2 * size } else { size };
            if x + read <= edges.width {
                top[1..1 + read].copy_from_slice(&row[x..x + read]);
            } else {
                for i in 0..read {
                    top[1 + i] = row[(x + i).min(edges.width - 1)];
                }
            }
            let last = top[size];
            top[1 + read..1 + 2 * size].fill(last);
            top[0] = if edges.left { row[x - 1] } else { 129 };
        }
        if edges.left {
            for (i, l) in left[..size].iter_mut().enumerate() {
                *l = plane[i.min(edges.height - 1) * stride + x - 1];
            }
        }
        let (corner, a, l) = (top[0], &top[1..], &left[..size]);
        // The block is written in place, row `r` column `c` at `at(r, c)`; the
        // diagonal modes read back rows they wrote.
        let at = |r: usize, c: usize| r * stride + x + c;
        let out = plane;
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
                for r in 0..size {
                    out[at(r, 0)..at(r, size)].fill(dc as u8);
                }
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
                    let rise = i16::from(l[r]) - i16::from(corner);
                    for (px, &a) in out[at(r, 0)..at(r, size)].iter_mut().zip(&a[..size]) {
                        *px = (rise + i16::from(a)).clamp(0, 255) as u8;
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
                    out[at(0, c)] = avg3(a[c], a[c + 1], a[c + 2]);
                }
                out[at(0, size - 1)] = right;
                for r in 1..size {
                    let n = size - 1 - r;
                    let (done, rest) = out.split_at_mut(at(r, 0));
                    rest[..n].copy_from_slice(&done[at(0, r)..at(0, r + n)]);
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
                    out[at(0, c)] = avg2(a[c], a[c + 1]);
                    out[at(1, c)] = avg3(a[c], a[c + 1], a[c + 2]);
                }
                let last = a[size - 1];
                for r in (2..size).step_by(2) {
                    let n = size - r / 2;
                    let (done, rest) = out.split_at_mut(at(r, 0));
                    rest[..n].copy_from_slice(&done[at(0, r / 2)..at(0, r / 2 + n)]);
                    rest[n..size].fill(last);
                    rest[stride..stride + n].copy_from_slice(&done[at(1, r / 2)..at(1, r / 2 + n)]);
                    rest[stride + n..stride + size].fill(last);
                }
            }
            IntraMode::D117 => {
                out[at(0, 0)] = avg2(corner, a[0]);
                for c in 1..size {
                    out[at(0, c)] = avg2(a[c - 1], a[c]);
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
                out[at(0, 0)] = avg2(corner, l[0]);
                for r in 1..size {
                    out[at(r, 0)] = avg2(l[r - 1], l[r]);
                }
                out[at(0, 1)] = avg3(l[0], corner, a[0]);
                out[at(1, 1)] = avg3(corner, l[0], l[1]);
                for r in 2..size {
                    out[at(r, 1)] = avg3(l[r - 2], l[r - 1], l[r]);
                }
                out[at(0, 2)] = avg3(corner, a[0], a[1]);
                for c in 1..size - 2 {
                    out[at(0, 2 + c)] = avg3(a[c - 1], a[c], a[c + 1]);
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
    }

    #[test]
    fn every_mode_is_what_a_pixel_at_a_time_gives() {
        let stride = 80;
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut noise = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 32) as u8
        };
        let picture: Vec<u8> = (0..stride * 72).map(|_| noise()).collect();
        let row: Vec<u8> = (0..stride).map(|_| noise()).collect();
        for size in [4, 8, 16, 32] {
            for mode in IntraMode::ALL {
                // The block inside the plane, at its left edge, and with
                // the picture ending part of the way across and down it.
                for (x, width, height) in [(16, 72, 64), (0, 72, 64), (16, 16 + size, size), (16, 16 + size / 2 + 3, size / 2 + 1), (16, 16 + size + 5, 64)] {
                    for (above, left, right) in [(true, true, true), (true, true, false), (true, false, true), (true, false, false), (false, true, false), (false, false, false)] {
                        if x == 0 && left {
                            continue;
                        }
                        let edges = Edges { width, height, left, right };
                        let above = above.then_some(&row[..]);
                        let (mut made, mut held) = (picture.clone(), picture.clone());
                        intra(mode, above, &mut made[4 * stride..], stride, x, size, &edges);
                        a_pixel_at_a_time(mode, above, &mut held[4 * stride..], stride, x, size, &edges);
                        assert!(made == held, "{mode:?} at {size} from {x} in {width}x{height}, above {}, left {left}, right {right}", above.is_some());
                    }
                }
            }
        }
    }

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
