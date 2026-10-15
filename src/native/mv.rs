//! Motion vectors, to a quarter of a pixel: the vectors a decoder predicts
//! for a block are clamped as libvpx clamps them (`clamp_mv_ref`), and a new
//! vector is coded against the nearest of them as libvpx reads it
//! (`read_mv`), at the frame's probabilities and, high precision being off
//! in the frame header, without the eighth-pixel bit. With every vector in
//! quarters, so is every vector a decoder predicts, and libvpx's rounding of
//! a predicted vector to quarters (`lower_mv_precision`) leaves it as it is.

use super::tables::{MV_BITS_PROBS, MV_CLASS_PROBS, MV_CLASS0_FP_PROBS, MV_CLASS0_PROBS, MV_FP_PROBS, MV_JOINT_PROBS, MV_SIGN_PROBS};

/// The most whole pixels either part of a vector this encoder codes may
/// have. A vector less one predicted from vectors this size stays within
/// what a component can code, 2048 pixels.
pub(super) const MAX_PIXELS: i32 = 1023;

/// How far past a frame's edges a predicted vector may reach, in eighths:
/// 16 pixels (libvpx's `MV_BORDER`).
const BORDER: i32 = 16 << 3;

/// libvpx's trees for the joint, the class and the fraction: an entry above
/// zero is the index of a node's two branches, any other is a leaf, negated.
const JOINT_TREE: [i8; 6] = [0, 2, -1, 4, -2, -3];
const CLASS_TREE: [i8; 20] = [0, 2, -1, 4, 6, 8, -2, -3, 10, 12, -4, -5, -6, 14, 16, 18, -7, -8, -9, -10];
const FP_TREE: [i8; 6] = [0, 2, -1, 4, -2, -3];

/// A vector's probabilities, one after another in the order a compressed
/// header updates them: the joints (3); per component the sign, the classes
/// (10), class 0 and the bits (10); per component class 0's fractions (2 of
/// 3) and the fractions (3).
pub(super) const PROBS: usize = 3 + 2 * COMPONENT + 2 * FRACTIONS;
const COMPONENT: usize = 1 + 10 + 1 + 10;
const FRACTIONS: usize = 2 * 3 + 3;
/// Where each is in `PROBS`, a component's from `3 + comp * COMPONENT` and
/// its fractions' from `FRACTIONS_AT + comp * FRACTIONS`.
const SIGN: usize = 0;
const CLASSES: usize = 1;
const CLASS0: usize = 11;
const BITS: usize = 12;
const FRACTIONS_AT: usize = 3 + 2 * COMPONENT;
const FP: usize = 6;

/// VP9's default vector probabilities, as `PROBS` orders them.
pub(super) const DEFAULT_PROBS: [u8; PROBS] = {
    let mut p = [0; PROBS];
    let mut i = 0;
    while i < 3 {
        p[i] = MV_JOINT_PROBS[i];
        i += 1;
    }
    let mut comp = 0;
    while comp < 2 {
        let at = 3 + comp * COMPONENT;
        p[at + SIGN] = MV_SIGN_PROBS[comp];
        p[at + CLASS0] = MV_CLASS0_PROBS[comp];
        let mut i = 0;
        while i < 10 {
            p[at + CLASSES + i] = MV_CLASS_PROBS[comp][i];
            p[at + BITS + i] = MV_BITS_PROBS[comp][i];
            i += 1;
        }
        let at = FRACTIONS_AT + comp * FRACTIONS;
        let mut i = 0;
        while i < 3 {
            p[at + i] = MV_CLASS0_FP_PROBS[comp][0][i];
            p[at + 3 + i] = MV_CLASS0_FP_PROBS[comp][1][i];
            p[at + FP + i] = MV_FP_PROBS[comp][i];
            i += 1;
        }
        comp += 1;
    }
    p
};

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

    /// The nearest vector of whole pixels, halves rounded up.
    pub(super) fn round(self) -> Self {
        Self { row: (self.row + 4) & !7, col: (self.col + 4) & !7 }
    }

    /// The whole pixels across and down, rounded down, and the sixteenths
    /// past them across and down that a 4:4:4 plane is filtered at.
    pub(super) fn split(self) -> ((isize, isize), (usize, usize)) {
        (((self.col >> 3) as isize, (self.row >> 3) as isize), (((self.col & 7) * 2) as usize, ((self.row & 7) * 2) as usize))
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

/// Code `leaf` of `tree`, whose probabilities are from `base` in `PROBS`,
/// through `bit`, root first.
fn tree(tree: &[i8], base: usize, leaf: i8, bit: &mut impl FnMut(bool, usize)) {
    // The path from the leaf back up: each entry's pair of branches starts
    // at an even index, which its parent's entry holds.
    let mut path = [(false, 0); 10];
    let mut depth = 0;
    let mut at = tree.iter().position(|&t| t == -leaf).expect("a leaf of the tree");
    loop {
        let node = at & !1;
        path[depth] = (at & 1 == 1, base + (node >> 1));
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
/// quarters, the eighth bit implied.
fn component(v: i32, comp: usize, bit: &mut impl FnMut(bool, usize)) {
    debug_assert!(v != 0 && v % 2 == 0, "a component in quarters, {v}");
    let (at, fractions) = (3 + comp * COMPONENT, FRACTIONS_AT + comp * FRACTIONS);
    bit(v < 0, at + SIGN);
    let z = v.unsigned_abs() - 1;
    let class = if z >= 8192 { 10 } else { (z >> 3).checked_ilog2().unwrap_or(0) };
    let offset = z - if class == 0 { 0 } else { 2 << (class + 2) };
    let (d, fr) = (offset >> 3, (offset >> 1) & 3);
    tree(&CLASS_TREE, at + CLASSES, class as i8, bit);
    if class == 0 {
        bit(d == 1, at + CLASS0);
        tree(&FP_TREE, fractions + 3 * d as usize, fr as i8, bit);
    } else {
        for i in 0..class {
            bit((d >> i) & 1 == 1, at + BITS + i as usize);
        }
        tree(&FP_TREE, fractions + FP, fr as i8, bit);
    }
}

/// Code `diff`, a new vector less the one it is coded against, through
/// `bit`, each bool with where its probability is in `PROBS`: which of its
/// parts are not zero, then each of those, down first.
pub(super) fn code(diff: Mv, mut bit: impl FnMut(bool, usize)) {
    let joint = 2 * i8::from(diff.row != 0) + i8::from(diff.col != 0);
    tree(&JOINT_TREE, 0, joint, &mut bit);
    if diff.row != 0 {
        component(diff.row, 0, &mut bit);
    }
    if diff.col != 0 {
        component(diff.col, 1, &mut bit);
    }
}

/// What coding `diff` costs at the default probabilities, in 256ths of a
/// bit.
pub(super) fn cost(diff: Mv) -> u32 {
    /// What each joint costs, and per component what each value it can take
    /// costs, from `-LIMIT`, in quarters.
    struct Costs {
        joints: [u32; 4],
        components: [Vec<u32>; 2],
    }
    /// The longest a component of a difference can be, in eighths: 2048
    /// pixels.
    const LIMIT: i32 = 2048 * 8;
    static COSTS: std::sync::LazyLock<Costs> = std::sync::LazyLock::new(|| {
        /// What the bools `code` codes cost.
        fn priced(code: impl FnOnce(&mut dyn FnMut(bool, usize))) -> u32 {
            let mut total = 0;
            code(&mut |b, i| total += super::bool_writer::cost(b, DEFAULT_PROBS[i]));
            total
        }
        Costs {
            joints: std::array::from_fn(|joint| priced(|mut bit| tree(&JOINT_TREE, 0, joint as i8, &mut bit))),
            components: std::array::from_fn(|comp| (-LIMIT..=LIMIT).step_by(2).map(|v| if v == 0 { 0 } else { priced(|mut bit| component(v, comp, &mut bit)) }).collect()),
        }
    });
    let joint = 2 * usize::from(diff.row != 0) + usize::from(diff.col != 0);
    let at = |v: i32| ((v + LIMIT) / 2) as usize;
    COSTS.joints[joint] + COSTS.components[0][at(diff.row)] + COSTS.components[1][at(diff.col)]
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
            // A quarter, a half and three quarters past, and short of.
            for q in [2, 4, 6] {
                diffs.push(Mv { row: v * 8 + q, col: -(v * 8 - q) });
            }
        }
        diffs.extend([Mv { row: 2, col: 0 }, Mv { row: 0, col: -6 }]);
        let mut w = BoolWriter::new();
        for &d in &diffs {
            code(d, |b, i| w.write(b, DEFAULT_PROBS[i]));
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
    fn a_difference_costs_its_bools() {
        for d in [Mv::ZERO, Mv::pixels(0, 1), Mv { row: -6, col: 0 }, Mv { row: 2046 * 8 + 6, col: -16384 }, Mv { row: 16384, col: 30 }] {
            let mut total = 0;
            code(d, |b, i| total += super::super::bool_writer::cost(b, DEFAULT_PROBS[i]));
            assert_eq!(cost(d), total, "{d:?}");
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
