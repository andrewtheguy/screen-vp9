//! Motion vectors, all of them whole pixels: the vectors a decoder predicts
//! for a block are clamped as libvpx clamps them (`clamp_mv_ref`), and a new
//! vector is coded against the nearest of them as libvpx reads it
//! (`read_mv`), at the default probabilities and, high precision being off
//! in the frame header, without the eighth-pixel bit.

use super::tables::{MV_BITS_PROBS, MV_CLASS_PROBS, MV_CLASS0_FP_PROBS, MV_CLASS0_PROBS, MV_FP_PROBS, MV_JOINT_PROBS, MV_SIGN_PROBS};

/// How far past a frame's edges a predicted vector may reach, in eighths:
/// 16 pixels (libvpx's `MV_BORDER`).
const BORDER: i32 = 16 << 3;

/// libvpx's trees for the joint, the class and the fraction: an entry above
/// zero is the index of a node's two branches, any other is a leaf, negated.
const JOINT_TREE: [i8; 6] = [0, 2, -1, 4, -2, -3];
const CLASS_TREE: [i8; 20] = [0, 2, -1, 4, 6, 8, -2, -3, 10, 12, -4, -5, -6, 14, 16, 18, -7, -8, -9, -10];
const FP_TREE: [i8; 6] = [0, 2, -1, 4, -2, -3];

/// A motion vector in eighths of a pixel, as the bitstream counts it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub(super) struct Mv {
    pub(super) row: i32,
    pub(super) col: i32,
}

impl Mv {
    pub(super) const ZERO: Mv = Mv { row: 0, col: 0 };

    /// `rows` and `cols` whole pixels.
    pub(super) fn pixels(rows: i32, cols: i32) -> Self {
        Self { row: rows * 8, col: cols * 8 }
    }

    /// The whole pixels down and across.
    pub(super) fn whole(self) -> (isize, isize) {
        ((self.row >> 3) as isize, (self.col >> 3) as isize)
    }

    /// The vector a decoder predicts from this one for the square of `bw`
    /// 8×8s at `(mi_row, mi_col)` in a frame of `mi_rows`×`mi_cols`: held to
    /// at most 16 pixels past each edge of the frame, the block's far edge
    /// for the bottom and right. As libvpx's `clamp` does, a value under the
    /// lower bound takes it and any other value over the upper takes that,
    /// so that where the bounds cross, which no block this encoder codes
    /// makes them do, the upper wins.
    pub(super) fn clamp(self, (mi_row, mi_col): (usize, usize), bw: usize, (mi_rows, mi_cols): (usize, usize)) -> Self {
        let held = |v: i32, at: usize, count: usize| {
            let low = -(at as i32 * 64) - BORDER;
            let high = (count as i32 - bw as i32 - at as i32) * 64 + BORDER;
            if v < low {
                low
            } else if v > high {
                high
            } else {
                v
            }
        };
        Self { row: held(self.row, mi_row, mi_rows), col: held(self.col, mi_col, mi_cols) }
    }
}

impl std::ops::Sub for Mv {
    type Output = Mv;
    fn sub(self, other: Mv) -> Mv {
        Mv { row: self.row - other.row, col: self.col - other.col }
    }
}

/// Code `leaf` of `tree` at `probs` through `bit`, root first.
fn tree(tree: &[i8], probs: &[u8], leaf: i8, bit: &mut impl FnMut(bool, u8)) {
    // The path from the leaf back up: each entry's pair of branches starts
    // at an even index, which its parent's entry holds.
    let mut path = [(false, 0); 10];
    let mut depth = 0;
    let mut at = tree.iter().position(|&t| t == -leaf).expect("a leaf of the tree");
    loop {
        let node = at & !1;
        path[depth] = (at & 1 == 1, probs[node >> 1]);
        depth += 1;
        if node == 0 {
            break;
        }
        at = tree.iter().position(|&t| t == node as i8).expect("a node of the tree");
    }
    for &(b, p) in path[..depth].iter().rev() {
        bit(b, p);
    }
}

/// Code one non-zero component of a difference, `comp` 0 down and 1
/// across: its sign, its class, the whole pixels within the class and the
/// fraction, which is always the last quarter with the eighth bit implied.
fn component(v: i32, comp: usize, bit: &mut impl FnMut(bool, u8)) {
    debug_assert!(v != 0 && v % 8 == 0, "a whole-pixel component, {v}");
    bit(v < 0, MV_SIGN_PROBS[comp]);
    let z = v.unsigned_abs() - 1;
    let class = if z >= 8192 { 10 } else { (z >> 3).checked_ilog2().unwrap_or(0) };
    let offset = z - if class == 0 { 0 } else { 2 << (class + 2) };
    let (d, fr) = (offset >> 3, (offset >> 1) & 3);
    tree(&CLASS_TREE, &MV_CLASS_PROBS[comp], class as i8, bit);
    if class == 0 {
        bit(d == 1, MV_CLASS0_PROBS[comp]);
        tree(&FP_TREE, &MV_CLASS0_FP_PROBS[comp][d as usize], fr as i8, bit);
    } else {
        for i in 0..class {
            bit((d >> i) & 1 == 1, MV_BITS_PROBS[comp][i as usize]);
        }
        tree(&FP_TREE, &MV_FP_PROBS[comp], fr as i8, bit);
    }
}

/// Code `diff`, a new vector less the one it is coded against, through
/// `bit`: which of its parts are not zero, then each of those, down first.
pub(super) fn code(diff: Mv, mut bit: impl FnMut(bool, u8)) {
    let joint = 2 * i8::from(diff.row != 0) + i8::from(diff.col != 0);
    tree(&JOINT_TREE, &MV_JOINT_PROBS, joint, &mut bit);
    if diff.row != 0 {
        component(diff.row, 0, &mut bit);
    }
    if diff.col != 0 {
        component(diff.col, 1, &mut bit);
    }
}

/// What coding `diff` costs, in 256ths of a bit.
pub(super) fn cost(diff: Mv) -> u32 {
    let mut total = 0;
    code(diff, |b, p| total += super::bool_writer::cost(b, p));
    total
}

#[cfg(test)]
mod tests {
    use super::super::bool_writer::BoolWriter;
    use super::super::bool_writer::tests::BoolReader;
    use super::*;

    /// A tree read as the VP9 specification reads one (§9.3.1).
    fn read_tree(r: &mut BoolReader, tree: &[i8], probs: &[u8]) -> i32 {
        let mut n = 0;
        loop {
            n = tree[n as usize + usize::from(r.read(probs[n as usize >> 1]))];
            if n <= 0 {
                return -i32::from(n);
            }
        }
    }

    /// A component read as the specification reads one (§6.4.20, §9.2.6).
    fn read_component(r: &mut BoolReader, comp: usize) -> i32 {
        let sign = r.read(MV_SIGN_PROBS[comp]);
        let class = read_tree(r, &CLASS_TREE, &MV_CLASS_PROBS[comp]);
        let mag = if class == 0 {
            let d = i32::from(r.read(MV_CLASS0_PROBS[comp]));
            let fr = read_tree(r, &FP_TREE, &MV_CLASS0_FP_PROBS[comp][d as usize]);
            ((d << 3) | (fr << 1) | 1) + 1
        } else {
            let d = (0..class).fold(0, |d, i| d | i32::from(r.read(MV_BITS_PROBS[comp][i as usize])) << i);
            let fr = read_tree(r, &FP_TREE, &MV_FP_PROBS[comp]);
            (2 << (class + 2)) + ((d << 3) | (fr << 1) | 1) + 1
        };
        if sign { -mag } else { mag }
    }

    #[test]
    fn differences_of_every_class_read_back() {
        let mut diffs = Vec::new();
        for v in [1, 2, 3, 7, 8, 15, 16, 100, 511, 1023, 1024, 2046] {
            for (row, col) in [(v, 0), (0, -v), (-v, v), (v, 1)] {
                diffs.push(Mv::pixels(row, col));
            }
        }
        let mut w = BoolWriter::new();
        for &d in &diffs {
            code(d, |b, p| w.write(b, p));
        }
        let data = w.finish();
        let mut r = BoolReader::new(&data);
        for &d in &diffs {
            let joint = read_tree(&mut r, &JOINT_TREE, &MV_JOINT_PROBS);
            let row = if joint >= 2 { read_component(&mut r, 0) } else { 0 };
            let col = if joint & 1 == 1 { read_component(&mut r, 1) } else { 0 };
            assert_eq!(Mv { row, col }, d);
        }
    }

    #[test]
    fn a_longer_difference_costs_more() {
        assert!(cost(Mv::pixels(0, 1)) < cost(Mv::pixels(0, 40)));
        assert!(cost(Mv::pixels(0, 40)) < cost(Mv::pixels(30, 40)));
    }

    #[test]
    fn predicted_vectors_are_held_near_the_frame() {
        let frame = (10, 20);
        // Inside: untouched.
        assert_eq!(Mv::pixels(-3, 5).clamp((4, 4), 2, frame), Mv::pixels(-3, 5));
        // At most 16 pixels past the left and top.
        assert_eq!(Mv::pixels(-100, -100).clamp((1, 2), 1, frame), Mv::pixels(-24, -32));
        // A 64×64 three 8×8s past the bottom right: zero is held to -8.
        assert_eq!(Mv::ZERO.clamp((5, 15), 8, frame), Mv::pixels(-8, -8));
        // A 64×64 in a frame one 8×8 high: the bounds cross and the upper
        // wins.
        assert_eq!(Mv::pixels(3, 0).clamp((0, 0), 8, (1, 20)), Mv::pixels(-40, 0));
    }
}
