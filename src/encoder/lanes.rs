//! Arithmetic on one `i32` or on a vector of them, so that a kernel is
//! written once and runs both as its scalar reference and in SIMD, each
//! lane of a vector a column of a block: the two can only differ where a
//! vector's own operations differ from the scalar ones, which the tests of
//! each kernel hold them to.
//!
//! NEON is always there on arm64, so it is used without asking.

use std::ops::{Add, Mul, Neg, Sub};

/// One `i32`, or `WIDTH` of them operated on at once. Every operation is
/// the scalar one in each lane, wrapping as release builds wrap.
pub(super) trait Lanes: Copy + Add<Output = Self> + Sub<Output = Self> + Neg<Output = Self> + Mul<i32, Output = Self> {
    /// How many values.
    const WIDTH: usize;
    /// `v` in every lane.
    fn splat(v: i32) -> Self;
    /// `WIDTH` values from the start of `src`.
    fn load(src: &[i32]) -> Self;
    /// Its values into the start of `dst`.
    fn store(self, dst: &mut [i32]);
    /// libvpx's `dct_const_round_shift`: `(x + (1 << 13)) >> 14`.
    fn round14(self) -> Self;
    /// What a store into 16 bits leaves.
    fn wrap16(self) -> Self;
    /// Shifted right `n`, the sign kept.
    fn shr(self, n: u32) -> Self;
    /// 1 where positive, 0 elsewhere.
    fn positive(self) -> Self;
    /// 1 where negative, 0 elsewhere.
    fn negative(self) -> Self;
    /// Whether every value is zero.
    fn is_zero(self) -> bool;

    /// The `n`×`n` `src` transposed into `dst`, both rows of `n`.
    fn transpose(n: usize, src: &[i32], dst: &mut [i32]) {
        for r in 0..n {
            for c in 0..n {
                dst[c * n + r] = src[r * n + c];
            }
        }
    }

    /// Add each of `values`, shifted right `shift` with rounding, to the
    /// pixel of `dst` it lies over, clamped to a pixel.
    fn add_shifted(values: &[i32], shift: u32, dst: &mut [u8]) {
        for (p, v) in dst.iter_mut().zip(values) {
            *p = (i32::from(*p) + ((v + (1 << (shift - 1))) >> shift)).clamp(0, 255) as u8;
        }
    }
}

/// The widest lanes this machine has.
#[cfg(target_arch = "aarch64")]
pub(super) type Vector = neon::I32x4;
#[cfg(not(target_arch = "aarch64"))]
pub(super) type Vector = i32;

impl Lanes for i32 {
    const WIDTH: usize = 1;

    fn splat(v: i32) -> Self {
        v
    }

    fn load(src: &[i32]) -> Self {
        src[0]
    }

    fn store(self, dst: &mut [i32]) {
        dst[0] = self;
    }

    fn round14(self) -> Self {
        (self + (1 << 13)) >> 14
    }

    fn wrap16(self) -> Self {
        self as i16 as i32
    }

    fn shr(self, n: u32) -> Self {
        self >> n
    }

    fn positive(self) -> Self {
        i32::from(self > 0)
    }

    fn negative(self) -> Self {
        i32::from(self < 0)
    }

    fn is_zero(self) -> bool {
        self == 0
    }
}

#[cfg(target_arch = "aarch64")]
pub(super) mod neon {
    use std::arch::aarch64::*;
    use std::ops::{Add, Mul, Neg, Sub};

    use super::Lanes;

    // NEON is in every arm64 target's baseline, so the `unsafe` around an
    // intrinsic is only for that, unless a comment says what else it is for.

    /// Four `i32` in a NEON register.
    #[derive(Clone, Copy)]
    pub(in super::super) struct I32x4(int32x4_t);

    impl Add for I32x4 {
        type Output = Self;
        #[inline(always)]
        fn add(self, other: Self) -> Self {
            unsafe { Self(vaddq_s32(self.0, other.0)) }
        }
    }

    impl Sub for I32x4 {
        type Output = Self;
        #[inline(always)]
        fn sub(self, other: Self) -> Self {
            unsafe { Self(vsubq_s32(self.0, other.0)) }
        }
    }

    impl Neg for I32x4 {
        type Output = Self;
        #[inline(always)]
        fn neg(self) -> Self {
            unsafe { Self(vnegq_s32(self.0)) }
        }
    }

    impl Mul<i32> for I32x4 {
        type Output = Self;
        #[inline(always)]
        fn mul(self, k: i32) -> Self {
            unsafe { Self(vmulq_n_s32(self.0, k)) }
        }
    }

    impl Lanes for I32x4 {
        const WIDTH: usize = 4;

        #[inline(always)]
        fn splat(v: i32) -> Self {
            unsafe { Self(vdupq_n_s32(v)) }
        }

        #[inline(always)]
        fn load(src: &[i32]) -> Self {
            let src = &src[..4];
            // SAFETY: `src` holds the four values read.
            Self(unsafe { vld1q_s32(src.as_ptr()) })
        }

        #[inline(always)]
        fn store(self, dst: &mut [i32]) {
            let dst = &mut dst[..4];
            // SAFETY: `dst` holds the four values written.
            unsafe { vst1q_s32(dst.as_mut_ptr(), self.0) }
        }

        #[inline(always)]
        fn round14(self) -> Self {
            unsafe { Self(vrshrq_n_s32::<14>(self.0)) }
        }

        #[inline(always)]
        fn wrap16(self) -> Self {
            unsafe { Self(vmovl_s16(vmovn_s32(self.0))) }
        }

        #[inline(always)]
        fn shr(self, n: u32) -> Self {
            unsafe { Self(vshlq_s32(self.0, vdupq_n_s32(-(n as i32)))) }
        }

        #[inline(always)]
        fn positive(self) -> Self {
            unsafe { Self(vreinterpretq_s32_u32(vshrq_n_u32::<31>(vcgtzq_s32(self.0)))) }
        }

        #[inline(always)]
        fn negative(self) -> Self {
            unsafe { Self(vreinterpretq_s32_u32(vshrq_n_u32::<31>(vreinterpretq_u32_s32(self.0)))) }
        }

        #[inline(always)]
        fn is_zero(self) -> bool {
            unsafe { vmaxvq_u32(vreinterpretq_u32_s32(self.0)) == 0 }
        }

        fn transpose(n: usize, src: &[i32], dst: &mut [i32]) {
            // A 4×4 at a time, its rows swapped pairwise across 32 and then
            // 64 bits.
            unsafe {
            for r in (0..n).step_by(4) {
                for c in (0..n).step_by(4) {
                    let row = |k: usize| Self::load(&src[(r + k) * n + c..]).0;
                    let (t0, t1) = (vtrn1q_s32(row(0), row(1)), vtrn2q_s32(row(0), row(1)));
                    let (t2, t3) = (vtrn1q_s32(row(2), row(3)), vtrn2q_s32(row(2), row(3)));
                    let wide = |t: int32x4_t| vreinterpretq_s64_s32(t);
                    let narrow = |t: int64x2_t| Self(vreinterpretq_s32_s64(t));
                    narrow(vtrn1q_s64(wide(t0), wide(t2))).store(&mut dst[c * n + r..]);
                    narrow(vtrn1q_s64(wide(t1), wide(t3))).store(&mut dst[(c + 1) * n + r..]);
                    narrow(vtrn2q_s64(wide(t0), wide(t2))).store(&mut dst[(c + 2) * n + r..]);
                    narrow(vtrn2q_s64(wide(t1), wide(t3))).store(&mut dst[(c + 3) * n + r..]);
                }
            }
            }
        }

        fn add_shifted(values: &[i32], shift: u32, dst: &mut [u8]) {
            let right = unsafe { vdupq_n_s32(-(shift as i32)) };
            let (eights, rest) = dst.as_chunks_mut::<8>();
            for (i, dst) in eights.iter_mut().enumerate() {
                // A rounding shift, then the pixels widened, added and
                // narrowed back with saturation.
                let (lo, hi) = (Self::load(&values[i * 8..]).0, Self::load(&values[i * 8 + 4..]).0);
                // SAFETY: `dst` is eight pixels.
                unsafe {
                    let (lo, hi) = (vrshlq_s32(lo, right), vrshlq_s32(hi, right));
                    let pixels = vreinterpretq_s16_u16(vmovl_u8(vld1_u8(dst.as_ptr())));
                    let sum = vaddq_s16(pixels, vcombine_s16(vmovn_s32(lo), vmovn_s32(hi)));
                    vst1_u8(dst.as_mut_ptr(), vqmovun_s16(sum));
                }
            }
            let done = eights.len() * 8;
            <i32 as Lanes>::add_shifted(&values[done..], shift, rest);
        }
    }
}
