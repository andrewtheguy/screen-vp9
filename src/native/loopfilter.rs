//! The loop filter a decoder runs over a frame once it has reconstructed
//! it, and that the encoder runs over its own reconstruction so that the
//! next frame is coded against what a decoder holds. Ported from libvpx's
//! `vp9_loopfilter.c` and `vpx_dsp/loopfilter.c` at its 4:4:4 path, where
//! each plane is filtered with luma's edges: per superblock in raster
//! order, every vertical edge in it, then every horizontal one, so that
//! where two filters touch the same pixels they do so in libvpx's order:
//! a superblock's vertical edges move pixels the horizontal edges of the one
//! left of it have moved already.
//!
//! An edge is filtered eight lines at a time, a line to each lane of a
//! vector of 16-bit values, and on amd64 two edges that lie one after the
//! other and take the same filter are sixteen lines of one AVX2 register,
//! each eight with its own level. Both are held by their tests to libvpx's C
//! written out a line at a time. A line that is one value across an edge is
//! what the filter leaves it, and a desktop is mostly those: a row of a
//! superblock's edges is passed at once where all its lines are, and an edge
//! alone after that.

use super::Geometry;

/// The largest filter level.
const MAX_LEVEL: i32 = 63;

/// Of the 8×8 columns of a superblock's mask, the first of each 32×32,
/// whose edges get the 8-wide filter at least where a transform is 4×4
/// (libvpx's `left_border`), and of its rows the same (`above_border`).
const LEFT_BORDER: u64 = 0x1111_1111_1111_1111;
const ABOVE_BORDER: u64 = 0x0000_00ff_0000_00ff;

/// Per transform size, the 8×8s of a superblock whose left edge is a
/// transform's edge (libvpx's `left_64x64_txform_mask`), and whose top edge
/// is (`above_64x64_txform_mask`).
const LEFT_TX: [u64; 4] = [u64::MAX, u64::MAX, 0x5555_5555_5555_5555, 0x1111_1111_1111_1111];
const ABOVE_TX: [u64; 4] = [u64::MAX, u64::MAX, 0x00ff_00ff_00ff_00ff, 0x0000_00ff_0000_00ff];

/// What the filter reads of a coded block, at its first 8×8.
#[derive(Clone, Copy)]
pub(super) struct Block {
    /// Its side in 8×8s, as log2: blocks are square.
    pub(super) n: u32,
    /// Its transform size, as log2 less 2.
    pub(super) tx: usize,
    pub(super) intra: bool,
    /// Whether it coded no coefficient.
    pub(super) skip: bool,
    /// Whether it is in the inactive segment, whose level is 0: none of its
    /// edges are filtered, though a neighbour's edge against it may be.
    pub(super) inactive: bool,
}

/// The frame's filter level for a quantizer index, as libvpx's real-time
/// encoder picks it (`vp9_pick_filter_level`, `LPF_PICK_FROM_Q`): a line
/// fitted to the levels a search found, four less on a keyframe.
pub(super) fn level(ac_step: i32, keyframe: bool) -> u8 {
    let guess = (ac_step * 20723 + 1_015_158 + (1 << 17)) >> 18;
    (guess - if keyframe { 4 } else { 0 }).clamp(0, MAX_LEVEL) as u8
}

/// The thresholds of a filter level at sharpness 0 (libvpx's
/// `update_sharpness` and `hev_thr`).
#[derive(Clone, Copy)]
struct Limits {
    /// The most the sum of the steps across the edge may be.
    blimit: i32,
    /// The most each step beside the edge may be.
    limit: i32,
    /// The step beside the edge above which only the edge's own two pixels
    /// move.
    hev: i32,
}

impl Limits {
    fn of(level: u8) -> Self {
        let level = i32::from(level);
        let limit = level.max(1);
        Self { blimit: 2 * (level + 2) + limit, limit, hev: level >> 4 }
    }
}

/// One superblock's edges and levels (libvpx's `LOOP_FILTER_MASK`, luma's
/// part): per 8×8 in raster order a bit, per filter size, of whether its
/// left and its top edge are filtered; whether the 4×4 edges inside it are;
/// and its level.
#[derive(Clone, Copy)]
pub(super) struct Masks {
    /// Indexed by transform size: 4×4 edges, 8×8 and 16×16 (which take
    /// 32×32's).
    left: [u64; 4],
    above: [u64; 4],
    inner: u64,
    levels: [u8; 64],
}

impl Masks {
    /// The masks of the superblock at `(mi_row, mi_col)`, each block's level
    /// `level` and one more for an intra block (libvpx's default reference
    /// deltas, at a scale of 2 from level 32), the block whose first 8×8 is
    /// `(mi_row, mi_col)` given by `block` (libvpx's `vp9_setup_mask` and
    /// `vp9_adjust_mask`).
    pub(super) fn new(g: &Geometry, mi_row: usize, mi_col: usize, level: u8, block: impl Fn(usize, usize) -> Block) -> Self {
        let mut m = Masks { left: [0; 4], above: [0; 4], inner: 0, levels: [0; 64] };
        let rows = (g.mi_rows - mi_row).min(8);
        let cols = (g.mi_cols - mi_col).min(8);
        let scale = 1 << (level >> 5);
        let intra_level = (i32::from(level) + scale).min(MAX_LEVEL) as u8;
        for r in 0..rows {
            for c in 0..cols {
                let b = block(mi_row + r, mi_col + c);
                let bw = 1usize << b.n;
                if r % bw != 0 || c % bw != 0 {
                    continue;
                }
                let level = if b.inactive {
                    0
                } else if b.intra {
                    intra_level
                } else {
                    level
                };
                if level == 0 {
                    continue;
                }
                let shift = r * 8 + c;
                for row in m.levels[shift..].chunks_mut(8).take(bw) {
                    row[..bw].fill(level);
                }
                // The block's own edges, and inside it its transforms'.
                let above = (1u64 << bw) - 1;
                let left = (0..bw).fold(0, |l, i| l | 1 << (8 * i));
                let size = above * left;
                m.above[b.tx] |= above << shift;
                m.left[b.tx] |= left << shift;
                if b.skip && !b.intra {
                    continue;
                }
                m.above[b.tx] |= (size & ABOVE_TX[b.tx]) << shift;
                m.left[b.tx] |= (size & LEFT_TX[b.tx]) << shift;
                if b.tx == 0 {
                    m.inner |= size << shift;
                }
            }
        }
        // The widest filter is 16, and every 32×32's edges get at least 8.
        m.left[2] |= m.left[3];
        m.above[2] |= m.above[3];
        m.left[1] |= m.left[0] & LEFT_BORDER;
        m.left[0] &= !LEFT_BORDER;
        m.above[1] |= m.above[0] & ABOVE_BORDER;
        m.above[0] &= !ABOVE_BORDER;
        // Nothing past the frame's last 8×8s, nor down its left edge.
        let rows_inside = if rows < 8 { (1 << (rows * 8)) - 1 } else { u64::MAX };
        let inside = rows_inside & (((1 << cols) - 1) * 0x0101_0101_0101_0101);
        for i in 0..3 {
            m.left[i] &= inside;
            m.above[i] &= inside;
            if mi_col == 0 {
                m.left[i] &= 0xfefe_fefe_fefe_fefe;
            }
        }
        m.inner &= inside;
        // The top edge of the frame is not filtered either.
        if mi_row == 0 {
            for i in 0..3 {
                m.above[i] &= !0xff;
            }
        }
        m
    }
}

/// How far an edge's filter reaches to either side, at most.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Width {
    Four,
    Eight,
    Sixteen,
}

impl Width {
    /// How many pixels each side of the edge it reads.
    fn reach(self) -> usize {
        if self == Width::Sixteen { 8 } else { 4 }
    }
}

/// A value per line across an edge, each operation the scalar one in each
/// lane, so that a filter is written once for however many lines a register
/// holds.
trait Line: Copy {
    fn splat(v: i16) -> Self;
    fn add(self, o: Self) -> Self;
    fn sub(self, o: Self) -> Self;
    fn and(self, o: Self) -> Self;
    fn times(self, k: i16) -> Self;
    /// `|self - o|`.
    fn diff(self, o: Self) -> Self;
    /// -1 where at most `limit`, 0 elsewhere.
    fn at_most(self, limit: Self) -> Self;
    /// Held to a signed byte.
    fn clamp8(self) -> Self;
    /// Shifted right `N`, the sign kept.
    fn shr<const N: i32>(self) -> Self;
    /// Of `a` and `b`, `a` where `self` is -1 and `b` where it is 0.
    fn select(self, a: Self, b: Self) -> Self;
    fn any(self) -> bool;
}

/// A value per line of the eight across an edge: one NEON register on
/// arm64, one SSE register on amd64, so that a filter's taps stay in
/// registers. Every operation is the scalar one in each lane.
#[derive(Clone, Copy)]
struct V(#[cfg(target_arch = "aarch64")] std::arch::aarch64::int16x8_t, #[cfg(target_arch = "x86_64")] std::arch::x86_64::__m128i);

// The crate is built for NEON; each `unsafe` here is for that.
#[cfg(target_arch = "aarch64")]
impl Line for V {
    #[inline(always)]
    fn splat(v: i16) -> V {
        V(unsafe { std::arch::aarch64::vdupq_n_s16(v) })
    }

    #[inline(always)]
    fn add(self, o: V) -> V {
        V(unsafe { std::arch::aarch64::vaddq_s16(self.0, o.0) })
    }

    #[inline(always)]
    fn sub(self, o: V) -> V {
        V(unsafe { std::arch::aarch64::vsubq_s16(self.0, o.0) })
    }

    #[inline(always)]
    fn and(self, o: V) -> V {
        V(unsafe { std::arch::aarch64::vandq_s16(self.0, o.0) })
    }

    #[inline(always)]
    fn times(self, k: i16) -> V {
        V(unsafe { std::arch::aarch64::vmulq_n_s16(self.0, k) })
    }

    #[inline(always)]
    fn diff(self, o: V) -> V {
        V(unsafe { std::arch::aarch64::vabdq_s16(self.0, o.0) })
    }

    #[inline(always)]
    fn at_most(self, limit: V) -> V {
        use std::arch::aarch64::*;
        V(unsafe { vreinterpretq_s16_u16(vcleq_s16(self.0, limit.0)) })
    }

    #[inline(always)]
    fn clamp8(self) -> V {
        use std::arch::aarch64::*;
        V(unsafe { vmaxq_s16(vminq_s16(self.0, vdupq_n_s16(127)), vdupq_n_s16(-128)) })
    }

    #[inline(always)]
    fn shr<const N: i32>(self) -> V {
        V(unsafe { std::arch::aarch64::vshrq_n_s16::<N>(self.0) })
    }

    #[inline(always)]
    fn select(self, a: V, b: V) -> V {
        use std::arch::aarch64::*;
        V(unsafe { vbslq_s16(vreinterpretq_u16_s16(self.0), a.0, b.0) })
    }

    #[inline(always)]
    fn any(self) -> bool {
        use std::arch::aarch64::*;
        unsafe { vmaxvq_u16(vreinterpretq_u16_s16(self.0)) != 0 }
    }
}

#[cfg(target_arch = "aarch64")]
impl V {
    /// The eight bytes of `v`, lowest first.
    #[inline(always)]
    fn from_u64(v: u64) -> V {
        use std::arch::aarch64::*;
        V(unsafe { vreinterpretq_s16_u16(vmovl_u8(vcreate_u8(v))) })
    }

    /// Each lane, a pixel, as a byte of the `u64`, lowest first.
    #[inline(always)]
    fn to_u64(self) -> u64 {
        use std::arch::aarch64::*;
        unsafe { vget_lane_u64::<0>(vreinterpret_u64_u8(vmovn_u16(vreinterpretq_u16_s16(self.0)))) }
    }

    /// The 8×8 of `px` from `at`, rows `stride` apart, turned about its
    /// diagonal: a lane per row, a value per column.
    #[inline(always)]
    fn columns(px: &[u8], at: usize, stride: usize) -> [V; 8] {
        use std::arch::aarch64::*;
        let px = &px[at..at + 7 * stride + 8];
        let rows: [uint8x8_t; 8] = std::array::from_fn(|i| unsafe { vld1_u8(px[i * stride..].as_ptr()) });
        transpose8(rows).map(|c| V(unsafe { vreinterpretq_s16_u16(vmovl_u8(c)) }))
    }

    /// `columns` back into `px`: each value a pixel.
    #[inline(always)]
    fn put_columns(px: &mut [u8], at: usize, stride: usize, columns: [V; 8]) {
        use std::arch::aarch64::*;
        let px = &mut px[at..at + 7 * stride + 8];
        let rows = transpose8(columns.map(|c| unsafe { vmovn_u16(vreinterpretq_u16_s16(c.0)) }));
        for (i, row) in rows.into_iter().enumerate() {
            unsafe { vst1_u8(px[i * stride..].as_mut_ptr(), row) };
        }
    }
}

/// Eight rows of eight bytes turned about the diagonal (libvpx's
/// `transpose_u8_8x8`): bytes, then pairs, then fours swapped between rows
/// 1, 2 and 4 apart.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn transpose8(a: [std::arch::aarch64::uint8x8_t; 8]) -> [std::arch::aarch64::uint8x8_t; 8] {
    use std::arch::aarch64::*;
    unsafe {
        let b = [vtrn_u8(a[0], a[1]), vtrn_u8(a[2], a[3]), vtrn_u8(a[4], a[5]), vtrn_u8(a[6], a[7])];
        let h = |v: uint8x8_t| vreinterpret_u16_u8(v);
        let c = [vtrn_u16(h(b[0].0), h(b[1].0)), vtrn_u16(h(b[0].1), h(b[1].1)), vtrn_u16(h(b[2].0), h(b[3].0)), vtrn_u16(h(b[2].1), h(b[3].1))];
        let w = |v: uint16x4_t| vreinterpret_u32_u16(v);
        let d = [vtrn_u32(w(c[0].0), w(c[2].0)), vtrn_u32(w(c[1].0), w(c[3].0)), vtrn_u32(w(c[0].1), w(c[2].1)), vtrn_u32(w(c[1].1), w(c[3].1))];
        let u = |v: uint32x2_t| vreinterpret_u8_u32(v);
        [u(d[0].0), u(d[1].0), u(d[2].0), u(d[3].0), u(d[0].1), u(d[1].1), u(d[2].1), u(d[3].1)]
    }
}

// The crate is built for AVX2, which brings the SSE4.1 these need, but its
// intrinsics are still `unsafe` to call from a function not marked for it;
// each `unsafe` here is for that.
#[cfg(target_arch = "x86_64")]
impl Line for V {
    #[inline(always)]
    fn splat(v: i16) -> V {
        V(unsafe { std::arch::x86_64::_mm_set1_epi16(v) })
    }

    #[inline(always)]
    fn add(self, o: V) -> V {
        V(unsafe { std::arch::x86_64::_mm_add_epi16(self.0, o.0) })
    }

    #[inline(always)]
    fn sub(self, o: V) -> V {
        V(unsafe { std::arch::x86_64::_mm_sub_epi16(self.0, o.0) })
    }

    #[inline(always)]
    fn and(self, o: V) -> V {
        V(unsafe { std::arch::x86_64::_mm_and_si128(self.0, o.0) })
    }

    #[inline(always)]
    fn times(self, k: i16) -> V {
        use std::arch::x86_64::*;
        V(unsafe { _mm_mullo_epi16(self.0, _mm_set1_epi16(k)) })
    }

    #[inline(always)]
    fn diff(self, o: V) -> V {
        use std::arch::x86_64::*;
        V(unsafe { _mm_abs_epi16(_mm_sub_epi16(self.0, o.0)) })
    }

    #[inline(always)]
    fn at_most(self, limit: V) -> V {
        use std::arch::x86_64::*;
        V(unsafe { _mm_xor_si128(_mm_cmpgt_epi16(self.0, limit.0), _mm_set1_epi16(-1)) })
    }

    #[inline(always)]
    fn clamp8(self) -> V {
        use std::arch::x86_64::*;
        V(unsafe { _mm_max_epi16(_mm_min_epi16(self.0, _mm_set1_epi16(127)), _mm_set1_epi16(-128)) })
    }

    #[inline(always)]
    fn shr<const N: i32>(self) -> V {
        V(unsafe { std::arch::x86_64::_mm_srai_epi16::<N>(self.0) })
    }

    #[inline(always)]
    fn select(self, a: V, b: V) -> V {
        V(unsafe { std::arch::x86_64::_mm_blendv_epi8(b.0, a.0, self.0) })
    }

    #[inline(always)]
    fn any(self) -> bool {
        unsafe { std::arch::x86_64::_mm_movemask_epi8(self.0) != 0 }
    }
}

#[cfg(target_arch = "x86_64")]
impl V {
    /// The eight bytes of `v`, lowest first.
    #[inline(always)]
    fn from_u64(v: u64) -> V {
        use std::arch::x86_64::*;
        V(unsafe { _mm_cvtepu8_epi16(_mm_cvtsi64_si128(v as i64)) })
    }

    /// Each lane, a pixel, as a byte of the `u64`, lowest first.
    #[inline(always)]
    fn to_u64(self) -> u64 {
        use std::arch::x86_64::*;
        unsafe { _mm_cvtsi128_si64(_mm_packus_epi16(self.0, self.0)) as u64 }
    }

    /// The 8×8 of `px` from `at`, rows `stride` apart, turned about its
    /// diagonal: a lane per row, a value per column.
    #[inline(always)]
    fn columns(px: &[u8], at: usize, stride: usize) -> [V; 8] {
        use std::arch::x86_64::*;
        let pairs = turned(px, at, stride);
        std::array::from_fn(|c| V(unsafe { _mm_cvtepu8_epi16(if c % 2 == 0 { pairs[c / 2] } else { _mm_srli_si128::<8>(pairs[c / 2]) }) }))
    }

    /// `columns` back into `px`: each value a pixel.
    #[inline(always)]
    fn put_columns(px: &mut [u8], at: usize, stride: usize, columns: [V; 8]) {
        use std::arch::x86_64::*;
        put_turned(px, at, stride, columns.map(|c| unsafe { _mm_packus_epi16(c.0, c.0) }));
    }
}

/// Sixteen lines across an edge, two eights with a level each, in one AVX2
/// register: the first eight in its low half.
#[cfg(target_arch = "x86_64")]
#[derive(Clone, Copy)]
struct Pair(std::arch::x86_64::__m256i);

// As for `V`: the crate is built for AVX2.
#[cfg(target_arch = "x86_64")]
impl Line for Pair {
    #[inline(always)]
    fn splat(v: i16) -> Pair {
        Pair(unsafe { std::arch::x86_64::_mm256_set1_epi16(v) })
    }

    #[inline(always)]
    fn add(self, o: Pair) -> Pair {
        Pair(unsafe { std::arch::x86_64::_mm256_add_epi16(self.0, o.0) })
    }

    #[inline(always)]
    fn sub(self, o: Pair) -> Pair {
        Pair(unsafe { std::arch::x86_64::_mm256_sub_epi16(self.0, o.0) })
    }

    #[inline(always)]
    fn and(self, o: Pair) -> Pair {
        Pair(unsafe { std::arch::x86_64::_mm256_and_si256(self.0, o.0) })
    }

    #[inline(always)]
    fn times(self, k: i16) -> Pair {
        use std::arch::x86_64::*;
        Pair(unsafe { _mm256_mullo_epi16(self.0, _mm256_set1_epi16(k)) })
    }

    #[inline(always)]
    fn diff(self, o: Pair) -> Pair {
        use std::arch::x86_64::*;
        Pair(unsafe { _mm256_abs_epi16(_mm256_sub_epi16(self.0, o.0)) })
    }

    #[inline(always)]
    fn at_most(self, limit: Pair) -> Pair {
        use std::arch::x86_64::*;
        Pair(unsafe { _mm256_xor_si256(_mm256_cmpgt_epi16(self.0, limit.0), _mm256_set1_epi16(-1)) })
    }

    #[inline(always)]
    fn clamp8(self) -> Pair {
        use std::arch::x86_64::*;
        Pair(unsafe { _mm256_max_epi16(_mm256_min_epi16(self.0, _mm256_set1_epi16(127)), _mm256_set1_epi16(-128)) })
    }

    #[inline(always)]
    fn shr<const N: i32>(self) -> Pair {
        Pair(unsafe { std::arch::x86_64::_mm256_srai_epi16::<N>(self.0) })
    }

    #[inline(always)]
    fn select(self, a: Pair, b: Pair) -> Pair {
        Pair(unsafe { std::arch::x86_64::_mm256_blendv_epi8(b.0, a.0, self.0) })
    }

    #[inline(always)]
    fn any(self) -> bool {
        unsafe { std::arch::x86_64::_mm256_movemask_epi8(self.0) != 0 }
    }
}

#[cfg(target_arch = "x86_64")]
impl Pair {
    /// `a` for the first eight lines and `b` for the second.
    #[inline(always)]
    fn of(a: V, b: V) -> Pair {
        Pair(unsafe { std::arch::x86_64::_mm256_set_m128i(b.0, a.0) })
    }

    /// The sixteen bytes of `v`, a line each.
    #[inline(always)]
    fn from_bytes(v: std::arch::x86_64::__m128i) -> Pair {
        Pair(unsafe { std::arch::x86_64::_mm256_cvtepu8_epi16(v) })
    }

    /// Each lane, a pixel, as a byte.
    #[inline(always)]
    fn to_bytes(self) -> std::arch::x86_64::__m128i {
        use std::arch::x86_64::*;
        unsafe { _mm256_castsi256_si128(_mm256_permute4x64_epi64::<0b1000>(_mm256_packus_epi16(self.0, self.0))) }
    }
}

/// The 8×8 of `px` from `at`, rows `stride` apart, turned about its
/// diagonal, as `transpose8` leaves it.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn turned(px: &[u8], at: usize, stride: usize) -> [std::arch::x86_64::__m128i; 4] {
    use std::arch::x86_64::*;
    let px = &px[at..at + 7 * stride + 8];
    transpose8(std::array::from_fn(|i| unsafe { _mm_loadl_epi64(px[i * stride..].as_ptr().cast()) }))
}

/// Eight columns, each the low half of a register, back into the 8×8 of
/// `px` that `turned` reads.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn put_turned(px: &mut [u8], at: usize, stride: usize, columns: [std::arch::x86_64::__m128i; 8]) {
    use std::arch::x86_64::*;
    let px = &mut px[at..at + 7 * stride + 8];
    let pairs = transpose8(columns);
    for (i, row) in px.chunks_mut(stride).take(8).enumerate() {
        unsafe { _mm_storel_epi64(row.as_mut_ptr().cast(), if i % 2 == 0 { pairs[i / 2] } else { _mm_srli_si128::<8>(pairs[i / 2]) }) };
    }
}

/// Eight rows of eight bytes, each the low half of a register, turned about
/// the diagonal: interleaved by bytes, pairs and fours, which leaves the
/// rows of the turned 8×8 two to a register, the first in the low half.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn transpose8(a: [std::arch::x86_64::__m128i; 8]) -> [std::arch::x86_64::__m128i; 4] {
    use std::arch::x86_64::*;
    unsafe {
        let b = [_mm_unpacklo_epi8(a[0], a[1]), _mm_unpacklo_epi8(a[2], a[3]), _mm_unpacklo_epi8(a[4], a[5]), _mm_unpacklo_epi8(a[6], a[7])];
        let c = [_mm_unpacklo_epi16(b[0], b[1]), _mm_unpackhi_epi16(b[0], b[1]), _mm_unpacklo_epi16(b[2], b[3]), _mm_unpackhi_epi16(b[2], b[3])];
        [_mm_unpacklo_epi32(c[0], c[2]), _mm_unpackhi_epi32(c[0], c[2]), _mm_unpacklo_epi32(c[1], c[3]), _mm_unpackhi_epi32(c[1], c[3])]
    }
}

/// The lines across an edge, a lane each: from the eighth pixel before the
/// edge, `[0]`, to the eighth after, `[15]`.
type Taps<L> = [L; 16];

impl Limits {
    /// A line's limit, its blimit and its hev limit, the same for every
    /// line.
    #[inline(always)]
    fn lanes(self) -> [V; 3] {
        [self.limit, self.blimit, self.hev].map(|v| V::splat(v as i16))
    }
}

/// Filter the lines across an edge in `t` as libvpx's `vpx_lpf_*_c` filter
/// of `width` filters each (`filter_mask`, `flat_mask4`, `flat_mask5`,
/// `filter4`, `filter8` and `filter16`), each lane a line with its own
/// limits, without a branch inside a lane so that they are one vector. Only
/// the taps `width` reads are looked at.
#[inline(always)]
fn filter_lines<L: Line>(t: &mut Taps<L>, width: Width, [limit, blimit, hev_limit]: [L; 3]) {
    let s = *t;
    let one = L::splat(1);
    let mask = s[4]
        .diff(s[5])
        .at_most(limit)
        .and(s[5].diff(s[6]).at_most(limit))
        .and(s[6].diff(s[7]).at_most(limit))
        .and(s[9].diff(s[8]).at_most(limit))
        .and(s[10].diff(s[9]).at_most(limit))
        .and(s[11].diff(s[10]).at_most(limit))
        .and(s[7].diff(s[8]).add(s[7].diff(s[8])).add(s[6].diff(s[9]).shr::<1>()).at_most(blimit));
    if !mask.any() {
        return;
    }
    let not_hev = s[6].diff(s[7]).at_most(hev_limit).and(s[9].diff(s[8]).at_most(hev_limit));
    let bias = L::splat(128);
    let [ps1, ps0, qs0, qs1] = [s[6].sub(bias), s[7].sub(bias), s[8].sub(bias), s[9].sub(bias)];
    let inner = qs0.sub(ps0);
    let outer = not_hev.select(L::splat(0), ps1.sub(qs1).clamp8());
    let f = outer.add(inner).add(inner).add(inner).clamp8().and(mask);
    let f1 = f.add(L::splat(4)).clamp8().shr::<3>();
    let f2 = f.add(L::splat(3)).clamp8().shr::<3>();
    let o = f1.add(one).shr::<1>().and(not_hev);
    t[6] = ps1.add(o).clamp8().add(bias);
    t[7] = ps0.add(f2).clamp8().add(bias);
    t[8] = qs0.sub(f1).clamp8().add(bias);
    t[9] = qs1.sub(o).clamp8().add(bias);
    if width == Width::Four {
        return;
    }
    let flat = mask
        .and(s[6].diff(s[7]).at_most(one))
        .and(s[5].diff(s[7]).at_most(one))
        .and(s[4].diff(s[7]).at_most(one))
        .and(s[9].diff(s[8]).at_most(one))
        .and(s[10].diff(s[8]).at_most(one))
        .and(s[11].diff(s[8]).at_most(one));
    if !flat.any() {
        return;
    }
    // 7 taps, [1, 1, 1, 2, 1, 1, 1], over p3 to q3, each end repeated: a
    // running sum, one tap out and one in at each step.
    let mut sum = s[4].times(3).add(s[5].times(2)).add(s[6]).add(s[7]).add(s[8]);
    for k in 5..11 {
        t[k] = flat.select(sum.add(L::splat(4)).shr::<3>(), t[k]);
        sum = sum.sub(s[(k - 3).max(4)]).sub(s[k]).add(s[k + 1]).add(s[(k + 4).min(11)]);
    }
    if width == Width::Eight {
        return;
    }
    let flat2 = flat
        .and(s[3].diff(s[7]).at_most(one))
        .and(s[2].diff(s[7]).at_most(one))
        .and(s[1].diff(s[7]).at_most(one))
        .and(s[0].diff(s[7]).at_most(one))
        .and(s[12].diff(s[8]).at_most(one))
        .and(s[13].diff(s[8]).at_most(one))
        .and(s[14].diff(s[8]).at_most(one))
        .and(s[15].diff(s[8]).at_most(one));
    if !flat2.any() {
        return;
    }
    // 15 taps, [1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1], over p7 to q7.
    let mut sum = s[0].times(7).add(s[1].times(2));
    for tap in &s[2..9] {
        sum = sum.add(*tap);
    }
    for k in 1..15 {
        t[k] = flat2.select(sum.add(L::splat(8)).shr::<4>(), t[k]);
        sum = sum.sub(s[k.saturating_sub(7)]).sub(s[k]).add(s[k + 1]).add(s[(k + 8).min(15)]);
    }
}

/// Whether `line` is one value from end to end.
#[inline(always)]
fn even(line: &[u8]) -> bool {
    line[1..] == line[..line.len() - 1]
}

/// Whether each of the eight rows from `at` is one value from `reach`
/// before the vertical edge there to `reach` after it. Such lines are what
/// the filter leaves them, and a desktop is mostly those: looked for before
/// any is turned.
#[inline(always)]
fn even_rows(px: &[u8], at: usize, stride: usize, reach: usize) -> bool {
    (0..8).all(|r| {
        let line = &px[at + r * stride - reach..][..2 * reach];
        line.as_chunks::<8>().0.iter().all(|eight| u64::from_le_bytes(*eight) == u64::from(line[0]) * 0x0101_0101_0101_0101)
    })
}

/// As `even_rows`, of the eight columns from `at` across the horizontal
/// edge above it.
#[inline(always)]
fn even_columns(px: &[u8], at: usize, stride: usize, reach: usize) -> bool {
    let row = |j: usize| u64::from_le_bytes(px[at + j * stride - 8 * stride..][..8].try_into().expect("eight pixels"));
    let first = row(8 - reach);
    (9 - reach..8 + reach).all(|j| row(j) == first)
}

/// Filter the vertical edge before pixel `at` of `px`, on its eight rows
/// `stride` apart.
fn vertical_edge(px: &mut [u8], at: usize, stride: usize, width: Width, l: Limits) {
    if !even_rows(px, at, stride, width.reach()) {
        vertical_lines(px, at, stride, width, l);
    }
}

/// `vertical_edge` without its look for even lines: each 8×8 across the
/// edge turned so that its columns are lanes.
fn vertical_lines(px: &mut [u8], at: usize, stride: usize, width: Width, l: Limits) {
    let reach = width.reach();
    let mut t: Taps<V> = [V::splat(0); 16];
    // The 8×8s from `reach` before the edge.
    for x in (8 - reach..8 + reach).step_by(8) {
        t[x..x + 8].copy_from_slice(&V::columns(px, at + x - 8, stride));
    }
    filter_lines(&mut t, width, l.lanes());
    for x in (8 - reach..8 + reach).step_by(8) {
        V::put_columns(px, at + x - 8, stride, t[x..x + 8].try_into().expect("eight columns"));
    }
}

/// Filter the horizontal edge above pixel `at` of `px`, on the eight
/// columns from it, its rows `stride` apart.
fn horizontal_edge(px: &mut [u8], at: usize, stride: usize, width: Width, l: Limits) {
    if !even_columns(px, at, stride, width.reach()) {
        horizontal_lines(px, at, stride, width, l);
    }
}

/// `horizontal_edge` without its look for even lines.
fn horizontal_lines(px: &mut [u8], at: usize, stride: usize, width: Width, l: Limits) {
    let reach = width.reach();
    let mut t: Taps<V> = [V::splat(0); 16];
    for j in 8 - reach..8 + reach {
        t[j] = V::from_u64(u64::from_le_bytes(px[at + j * stride - 8 * stride..][..8].try_into().expect("eight pixels")));
    }
    filter_lines(&mut t, width, l.lanes());
    for j in 8 - reach..8 + reach {
        px[at + j * stride - 8 * stride..][..8].copy_from_slice(&t[j].to_u64().to_le_bytes());
    }
}

/// Two vertical edges one under the other, before pixel `at` of `px` and
/// eight rows down, each with its limits: sixteen lines a register on
/// amd64.
#[cfg(target_arch = "x86_64")]
fn vertical_pair(px: &mut [u8], at: usize, stride: usize, width: Width, l: [Limits; 2]) {
    use std::arch::x86_64::*;
    let reach = width.reach();
    let below = at + 8 * stride;
    match [even_rows(px, at, stride, reach), even_rows(px, below, stride, reach)] {
        [true, true] => return,
        [false, true] => return vertical_lines(px, at, stride, width, l[0]),
        [true, false] => return vertical_lines(px, below, stride, width, l[1]),
        [false, false] => {}
    }
    let mut t: Taps<Pair> = [Pair::splat(0); 16];
    for x in (8 - reach..8 + reach).step_by(8) {
        let (a, b) = (turned(px, at + x - 8, stride), turned(px, below + x - 8, stride));
        for c in 0..8 {
            t[x + c] = Pair::from_bytes(unsafe { if c % 2 == 0 { _mm_unpacklo_epi64(a[c / 2], b[c / 2]) } else { _mm_unpackhi_epi64(a[c / 2], b[c / 2]) } });
        }
    }
    filter_lines(&mut t, width, pair_lanes(l));
    for x in (8 - reach..8 + reach).step_by(8) {
        let columns: [__m128i; 8] = std::array::from_fn(|c| t[x + c].to_bytes());
        put_turned(px, at + x - 8, stride, columns);
        put_turned(px, below + x - 8, stride, columns.map(|c| unsafe { _mm_srli_si128::<8>(c) }));
    }
}

/// Two horizontal edges side by side, above pixel `at` of `px` and eight
/// columns on, each with its limits: sixteen lines a register on amd64.
#[cfg(target_arch = "x86_64")]
fn horizontal_pair(px: &mut [u8], at: usize, stride: usize, width: Width, l: [Limits; 2]) {
    use std::arch::x86_64::*;
    let reach = width.reach();
    match [even_columns(px, at, stride, reach), even_columns(px, at + 8, stride, reach)] {
        [true, true] => return,
        [false, true] => return horizontal_lines(px, at, stride, width, l[0]),
        [true, false] => return horizontal_lines(px, at + 8, stride, width, l[1]),
        [false, false] => {}
    }
    let mut t: Taps<Pair> = [Pair::splat(0); 16];
    for j in 8 - reach..8 + reach {
        t[j] = Pair::from_bytes(unsafe { _mm_loadu_si128(px[at + j * stride - 8 * stride..][..16].as_ptr().cast()) });
    }
    filter_lines(&mut t, width, pair_lanes(l));
    for j in 8 - reach..8 + reach {
        unsafe { _mm_storeu_si128(px[at + j * stride - 8 * stride..][..16].as_mut_ptr().cast(), t[j].to_bytes()) };
    }
}

/// The limits of two eights of lines, each in its half.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn pair_lanes(l: [Limits; 2]) -> [Pair; 3] {
    let (a, b) = (l[0].lanes(), l[1].lanes());
    std::array::from_fn(|i| Pair::of(a[i], b[i]))
}

/// Two vertical edges one under the other, as on amd64, where NEON's
/// register holds the lines of one.
#[cfg(target_arch = "aarch64")]
fn vertical_pair(px: &mut [u8], at: usize, stride: usize, width: Width, l: [Limits; 2]) {
    vertical_edge(px, at, stride, width, l[0]);
    vertical_edge(px, at + 8 * stride, stride, width, l[1]);
}

/// Two horizontal edges side by side, as on amd64, where NEON's register
/// holds the lines of one.
#[cfg(target_arch = "aarch64")]
fn horizontal_pair(px: &mut [u8], at: usize, stride: usize, width: Width, l: [Limits; 2]) {
    horizontal_edge(px, at, stride, width, l[0]);
    horizontal_edge(px, at + 8, stride, width, l[1]);
}

/// The edges `kinds` names of two 8×8s, the first's at `at` and the second's
/// `step` on: as one where they are the same filter.
#[inline(always)]
fn edges(px: &mut [u8], at: usize, stride: usize, vertical: bool, kinds: [Option<Width>; 2], l: [Limits; 2]) {
    let step = if vertical { 8 * stride } else { 8 };
    match kinds {
        [Some(a), Some(b)] if a == b && vertical => vertical_pair(px, at, stride, a, l),
        [Some(a), Some(b)] if a == b => horizontal_pair(px, at, stride, a, l),
        _ => {
            for (i, kind) in kinds.into_iter().enumerate() {
                let Some(width) = kind else { continue };
                if vertical {
                    vertical_edge(px, at + i * step, stride, width, l[i]);
                } else {
                    horizontal_edge(px, at + i * step, stride, width, l[i]);
                }
            }
        }
    }
}

/// Every vertical edge of one plane of a superblock, two 8×8 rows at a
/// time, the superblock's corner at pixel `at` of `px`, its rows `stride`
/// apart, as far down as `rows` 8×8 rows (the first half of libvpx's
/// `vp9_filter_block_plane_ss00`). It reads and moves up to eight pixels
/// left of the superblock.
pub(super) fn vertical(px: &mut [u8], at: usize, stride: usize, rows: usize, m: &Masks) {
    for r in (0..rows).step_by(2) {
        let shift = r * 8;
        let bits = |mask: u64| (mask >> shift) as u32 & 0xffff;
        let (m16, m8, m4, inner) = (bits(m.left[2]), bits(m.left[1]), bits(m.left[0]), bits(m.inner));
        let (left, all) = (m16 | m8 | m4, m16 | m8 | m4 | inner);
        if all == 0 {
            continue;
        }
        // The two rows' edges at once where every line across them is even.
        let columns = all | all >> 8;
        let (first, last) = (columns.trailing_zeros() as usize, 32 - (columns & 0xff).leading_zeros() as usize);
        let before = if (left | left >> 8) >> first & 1 != 0 { 8 } else { 0 };
        let lines = if all & 0xff == 0 { 8..16 } else if all >> 8 == 0 { 0..8 } else { 0..16 };
        let from = at + r * 8 * stride + first * 8 - before;
        if lines.into_iter().all(|y| even(&px[from + y * stride..][..(last - first) * 8 + before])) {
            continue;
        }
        for c in 0..8 {
            let both = 1 << c | 1 << (c + 8);
            if (m16 | m8 | m4 | inner) & both == 0 {
                continue;
            }
            let limits = [Limits::of(m.levels[shift + c]), Limits::of(m.levels[shift + 8 + c])];
            // The 8×8 column's edge in the two rows.
            let x = at + r * 8 * stride + c * 8;
            // libvpx filters the two rows at 16 with the first's limits.
            let paired = m16 & both == both;
            if paired {
                vertical_pair(px, x, stride, Width::Sixteen, [limits[0]; 2]);
            }
            for (mask, width, offset) in [(m16, Width::Sixteen, 0), (m8, Width::Eight, 0), (m4, Width::Four, 0), (inner, Width::Four, 4)] {
                if paired && width == Width::Sixteen {
                    continue;
                }
                let kinds = [0, 1].map(|i| (mask >> (c + 8 * i) & 1 != 0).then_some(width));
                edges(px, x + offset, stride, true, kinds, limits);
            }
        }
    }
}

/// Every horizontal edge of one plane of a superblock, laid out as for
/// `vertical`, a row at a time (the second half of libvpx's
/// `vp9_filter_block_plane_ss00`). It reads and moves up to eight rows
/// above the superblock.
pub(super) fn horizontal(px: &mut [u8], at: usize, stride: usize, rows: usize, m: &Masks) {
    for r in 0..rows {
        let shift = r * 8;
        let bits = |mask: u64| (mask >> shift) as u32 & 0xff;
        let (m16, m8, m4, inner) = (bits(m.above[2]), bits(m.above[1]), bits(m.above[0]), bits(m.inner));
        let (top, all) = (m16 | m8 | m4, m16 | m8 | m4 | inner);
        if all == 0 {
            continue;
        }
        // The row's edges at once where every column across them is even.
        let (first, last) = (all.trailing_zeros() as usize, 32 - all.leading_zeros() as usize);
        let from = at + r * 8 * stride + first * 8;
        let line = |y: usize| &px[from + y * stride - 8 * stride..][..(last - first) * 8];
        if (if top == 0 { 9..16 } else { 1..16 }).all(|y| line(y) == line(y - 1)) {
            continue;
        }
        let mut c = 0;
        while c < 8 {
            let x = at + r * 8 * stride + c * 8;
            // libvpx filters two 8×8s at 16 with the first's limits.
            let double = |c: usize| m16 >> c & 3 == 3;
            if double(c) {
                horizontal_pair(px, x, stride, Width::Sixteen, [Limits::of(m.levels[shift + c]); 2]);
                c += 2;
                continue;
            }
            // The 8×8 after this one with it, unless it is the first of two
            // at 16: columns apart share no pixel, so the order between them
            // is free, and each still has its top edge before its inner one.
            let n = if c + 1 < 8 && !double(c + 1) { 2 } else { 1 };
            let on = |mask: u32, i: usize| i < n && mask >> (c + i) & 1 != 0;
            let top = [0, 1].map(|i| {
                if on(m16, i) {
                    Some(Width::Sixteen)
                } else if on(m8, i) {
                    Some(Width::Eight)
                } else {
                    on(m4, i).then_some(Width::Four)
                }
            });
            let limits = [0, 1].map(|i| Limits::of(m.levels[shift + (c + i).min(7)]));
            edges(px, x, stride, false, top, limits);
            edges(px, x + 4 * stride, stride, false, [0, 1].map(|i| on(inner, i).then_some(Width::Four)), limits);
            c += n;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// libvpx's `filter_mask`, `flat_mask4`, `flat_mask5`, `filter4`,
    /// `filter8` and `filter16`, written as its C is, on the one line across
    /// the edge before `at` of `px`, `step` apart: the reference the eight
    /// lanes are held to.
    fn filter(px: &mut [u8], at: usize, step: usize, width: Width, l: Limits) {
        let reach = width.reach();
        let (mut p, mut q) = ([0i32; 8], [0i32; 8]);
        for k in 0..reach {
            p[k] = i32::from(px[at - (k + 1) * step]);
            q[k] = i32::from(px[at + k * step]);
        }
        let mut set = |k: isize, v: i32| {
            let i = if k < 0 { at - (-k) as usize * step } else { at + k as usize * step };
            px[i] = v as u8;
        };
        let mask = (p[3] - p[2]).abs() <= l.limit
            && (p[2] - p[1]).abs() <= l.limit
            && (p[1] - p[0]).abs() <= l.limit
            && (q[1] - q[0]).abs() <= l.limit
            && (q[2] - q[1]).abs() <= l.limit
            && (q[3] - q[2]).abs() <= l.limit
            && (p[0] - q[0]).abs() * 2 + (p[1] - q[1]).abs() / 2 <= l.blimit;
        let flat = (1..4).all(|k| (p[k] - p[0]).abs() <= 1 && (q[k] - q[0]).abs() <= 1);
        let flat2 = (4..8).all(|k| (p[k] - p[0]).abs() <= 1 && (q[k] - q[0]).abs() <= 1);
        if width == Width::Sixteen && flat2 && flat && mask {
            let [p7, p6, p5, p4, p3, p2, p1, p0] = [p[7], p[6], p[5], p[4], p[3], p[2], p[1], p[0]];
            let [q0, q1, q2, q3, q4, q5, q6, q7] = [q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7]];
            let r = |v: i32| (v + 8) >> 4;
            set(-7, r(p7 * 7 + p6 * 2 + p5 + p4 + p3 + p2 + p1 + p0 + q0));
            set(-6, r(p7 * 6 + p6 + p5 * 2 + p4 + p3 + p2 + p1 + p0 + q0 + q1));
            set(-5, r(p7 * 5 + p6 + p5 + p4 * 2 + p3 + p2 + p1 + p0 + q0 + q1 + q2));
            set(-4, r(p7 * 4 + p6 + p5 + p4 + p3 * 2 + p2 + p1 + p0 + q0 + q1 + q2 + q3));
            set(-3, r(p7 * 3 + p6 + p5 + p4 + p3 + p2 * 2 + p1 + p0 + q0 + q1 + q2 + q3 + q4));
            set(-2, r(p7 * 2 + p6 + p5 + p4 + p3 + p2 + p1 * 2 + p0 + q0 + q1 + q2 + q3 + q4 + q5));
            set(-1, r(p7 + p6 + p5 + p4 + p3 + p2 + p1 + p0 * 2 + q0 + q1 + q2 + q3 + q4 + q5 + q6));
            set(0, r(p6 + p5 + p4 + p3 + p2 + p1 + p0 + q0 * 2 + q1 + q2 + q3 + q4 + q5 + q6 + q7));
            set(1, r(p5 + p4 + p3 + p2 + p1 + p0 + q0 + q1 * 2 + q2 + q3 + q4 + q5 + q6 + q7 * 2));
            set(2, r(p4 + p3 + p2 + p1 + p0 + q0 + q1 + q2 * 2 + q3 + q4 + q5 + q6 + q7 * 3));
            set(3, r(p3 + p2 + p1 + p0 + q0 + q1 + q2 + q3 * 2 + q4 + q5 + q6 + q7 * 4));
            set(4, r(p2 + p1 + p0 + q0 + q1 + q2 + q3 + q4 * 2 + q5 + q6 + q7 * 5));
            set(5, r(p1 + p0 + q0 + q1 + q2 + q3 + q4 + q5 * 2 + q6 + q7 * 6));
            set(6, r(p0 + q0 + q1 + q2 + q3 + q4 + q5 + q6 * 2 + q7 * 7));
        } else if width != Width::Four && flat && mask {
            let [p3, p2, p1, p0, q0, q1, q2, q3] = [p[3], p[2], p[1], p[0], q[0], q[1], q[2], q[3]];
            let r = |v: i32| (v + 4) >> 3;
            set(-3, r(p3 + p3 + p3 + 2 * p2 + p1 + p0 + q0));
            set(-2, r(p3 + p3 + p2 + 2 * p1 + p0 + q0 + q1));
            set(-1, r(p3 + p2 + p1 + 2 * p0 + q0 + q1 + q2));
            set(0, r(p2 + p1 + p0 + 2 * q0 + q1 + q2 + q3));
            set(1, r(p1 + p0 + q0 + 2 * q1 + q2 + q3 + q3));
            set(2, r(p0 + q0 + q1 + 2 * q2 + q3 + q3 + q3));
        } else {
            let c = |v: i32| v.clamp(-128, 127);
            let (ps1, ps0, qs0, qs1) = (p[1] - 128, p[0] - 128, q[0] - 128, q[1] - 128);
            let hev = (p[1] - p[0]).abs() > l.hev || (q[1] - q[0]).abs() > l.hev;
            let filter = if hev { c(ps1 - qs1) } else { 0 };
            let filter = if mask { c(filter + 3 * (qs0 - ps0)) } else { 0 };
            let (f1, f2) = (c(filter + 4) >> 3, c(filter + 3) >> 3);
            set(0, c(qs0 - f1) + 128);
            set(-1, c(ps0 + f2) + 128);
            let filter = if hev { 0 } else { (f1 + 1) >> 1 };
            set(1, c(qs1 - filter) + 128);
            set(-2, c(ps1 + filter) + 128);
        }
    }

    #[test]
    fn lines_at_once_are_the_scalar_reference() {
        let mut state = 0x2545_f491_u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        let stride = 24;
        for round in 0..20_000 {
            // Steps small enough that every filter and its flat cases are
            // reached, from a base anywhere in the range, and at times wild.
            let base = (next() % 256) as i32;
            let spread = [1, 2, 3, 6, 40, 255][round % 6];
            let px: Vec<u8> = (0..stride * 24).map(|_| (base + (next() % (2 * spread + 1)) as i32 - spread as i32).clamp(0, 255) as u8).collect();
            let width = [Width::Four, Width::Eight, Width::Sixteen][round / 6 % 3];
            let limits = Limits::of((next() % 64) as u8);
            let at = 8 * stride + 8;
            let (mut lanes, mut reference) = (px.clone(), px.clone());
            if round % 2 == 0 {
                vertical_edge(&mut lanes, at, stride, width, limits);
                for i in 0..8 {
                    filter(&mut reference, at + i * stride, 1, width, limits);
                }
            } else {
                horizontal_edge(&mut lanes, at, stride, width, limits);
                for i in 0..8 {
                    filter(&mut reference, at + i, stride, width, limits);
                }
            }
            assert_eq!(lanes, reference, "round {round}, {width:?}");
            // Sixteen lines, two eights with a level each.
            let limits = [limits, Limits::of((next() % 64) as u8)];
            let (mut lanes, mut reference) = (px.clone(), px.clone());
            if round % 2 == 0 {
                vertical_pair(&mut lanes, at, stride, width, limits);
                for i in 0..16 {
                    filter(&mut reference, at + i * stride, 1, width, limits[i / 8]);
                }
            } else {
                horizontal_pair(&mut lanes, at, stride, width, limits);
                for i in 0..16 {
                    filter(&mut reference, at + i, stride, width, limits[i / 8]);
                }
            }
            assert_eq!(lanes, reference, "round {round}, {width:?}, sixteen lines");
        }
    }

    /// Each filter's time on eight lines across a vertical and a horizontal
    /// edge, the lines flat enough that every one is filtered in full. A
    /// measurement, not a check; run it in release:
    ///
    /// ```text
    /// cargo test --release edge_time -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "a measurement that wants a release build"]
    fn edge_time() {
        let stride = 64;
        let mut state = 7u32;
        let mut px: Vec<u8> = (0..stride * 64)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (100 + state % 3) as u8
            })
            .collect();
        for width in [Width::Four, Width::Eight, Width::Sixteen] {
            for vertical in [true, false] {
                let n = 2_000_000;
                let started = std::time::Instant::now();
                for i in 0..n {
                    let at = 8 * stride + 8 + (i % 4) * 8;
                    if vertical {
                        vertical_edge(&mut px, at, stride, width, Limits::of(9));
                    } else {
                        horizontal_edge(&mut px, at, stride, width, Limits::of(9));
                    }
                }
                let edge = if vertical { "vertical" } else { "horizontal" };
                println!("{width:?}, {edge}: {:.1} ns", started.elapsed().as_nanos() as f64 / n as f64);
            }
        }
    }
}
