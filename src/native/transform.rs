//! Two-dimensional transforms, DCTs and below 32×32 the hybrids of a DCT and
//! an ADST: forward, to code a residual, and inverse, to reconstruct it as a
//! decoder does.
//!
//! A forward transform is the encoder's own business; these follow libvpx's
//! `vpx_fdct*_c` and `vp9_fht*_c`, whose scale the inverse expects, and
//! give their coefficients to the bit. Under 32×32 they are made in 16 bits
//! a value, as libvpx's own kernels make them: a product of two values is
//! summed with another in 32 and rounded back (`Lanes16::dot`), and nothing
//! else of a residual of 8-bit pixels leaves 16. The
//! inverse is libvpx's `vpx_idct*_add_c` and `vp9_iht*_add_c` and exact: a
//! decoder adds its result to the prediction, and the encoder predicts later
//! blocks from the same pixels.

use super::dct::{fdct32, iadst4, iadst8, iadst16, idct4, idct8, idct16, idct32};
use super::lanes::{Lanes, Lanes16, Narrow4, Narrow8, Narrow16, Vector, Vector4};

/// cos(π/4) in 14 bits.
const COSPI_16_64: i32 = 11585;
const COSPI_24_64: i32 = 6270;
const COSPI_8_64: i32 = 15137;
const COSPI_28_64: i32 = 3196;
const COSPI_4_64: i32 = 16069;
const COSPI_12_64: i32 = 13623;
const COSPI_20_64: i32 = 9102;
const COSPI_30_64: i32 = 1606;
const COSPI_2_64: i32 = 16305;
const COSPI_14_64: i32 = 12665;
const COSPI_18_64: i32 = 10394;
const COSPI_22_64: i32 = 7723;
const COSPI_10_64: i32 = 14449;
const COSPI_6_64: i32 = 15679;
const COSPI_26_64: i32 = 4756;
/// cos(kπ/64) beside cos((32 − k)π/64) for `k` 1, 5, 9 and on to 29: the
/// turns of the 16-point ADST's first stage.
const ODD_TURNS: [(i32, i32); 8] = [(16364, 804), (15893, 3981), (14811, 7005), (13160, 9760), (11003, 12140), (8423, 14053), (5520, 15426), (2404, 16207)];
/// The 4-point ADST's sines, sin(kπ/9) scaled.
const SINPI_1_9: i32 = 5283;
const SINPI_2_9: i32 = 9929;
const SINPI_3_9: i32 = 13377;
const SINPI_4_9: i32 = 15212;

fn round14(x: i32) -> i32 {
    (x + (1 << 13)) >> 14
}

/// Which passes of a transform are ADSTs rather than DCTs, the first named
/// the one down the columns (VP9's `TX_TYPE`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum TxType {
    DctDct,
    AdstDct,
    DctAdst,
    AdstAdst,
}

/// The 4-point forward DCT inside `vpx_fdct4x4_c`.
#[inline(always)]
fn fdct4<V: Lanes16>(i: &[V; 4], o: &mut [V; 4]) {
    let s = [i[0] + i[3], i[1] + i[2], i[1] - i[2], i[0] - i[3]];
    *o = [
        V::round14(s[0].dot(COSPI_16_64, s[1], COSPI_16_64)),
        V::round14(s[2].dot(COSPI_24_64, s[3], COSPI_8_64)),
        V::round14(s[0].dot(COSPI_16_64, s[1], -COSPI_16_64)),
        V::round14(s[2].dot(-COSPI_8_64, s[3], COSPI_24_64)),
    ];
}

/// The 8-point forward DCT inside `vpx_fdct8x8_c`.
#[inline(always)]
fn fdct8<V: Lanes16>(i: &[V; 8], o: &mut [V; 8]) {
    let s = [i[0] + i[7], i[1] + i[6], i[2] + i[5], i[3] + i[4], i[3] - i[4], i[2] - i[5], i[1] - i[6], i[0] - i[7]];
    let mut even = [V::splat(0); 4];
    fdct4(&[s[0], s[1], s[2], s[3]], &mut even);
    let t2 = V::round14(s[6].dot(COSPI_16_64, s[5], -COSPI_16_64));
    let t3 = V::round14(s[6].dot(COSPI_16_64, s[5], COSPI_16_64));
    let (x0, x1, x2, x3) = (s[4] + t2, s[4] - t2, s[7] - t3, s[7] + t3);
    *o = [
        even[0],
        V::round14(x0.dot(COSPI_28_64, x3, COSPI_4_64)),
        even[1],
        V::round14(x2.dot(COSPI_12_64, x1, -COSPI_20_64)),
        even[2],
        V::round14(x1.dot(COSPI_12_64, x2, COSPI_20_64)),
        even[3],
        V::round14(x3.dot(COSPI_28_64, x0, -COSPI_4_64)),
    ];
}

/// The 16-point forward DCT inside `vpx_fdct16x16_c`: the even half is the
/// 8-point one of the folded input.
#[inline(always)]
fn fdct16<V: Lanes16>(i: &[V; 16], o: &mut [V; 16]) {
    let (mut folded, mut step1) = ([V::splat(0); 8], [V::splat(0); 8]);
    for k in 0..8 {
        folded[k] = i[k] + i[15 - k];
        step1[k] = i[7 - k] - i[8 + k];
    }
    let mut even = [V::splat(0); 8];
    fdct8(&folded, &mut even);
    let mut step2 = [V::splat(0); 8];
    step2[2] = V::round14(step1[5].dot(COSPI_16_64, step1[2], -COSPI_16_64));
    step2[3] = V::round14(step1[4].dot(COSPI_16_64, step1[3], -COSPI_16_64));
    step2[4] = V::round14(step1[4].dot(COSPI_16_64, step1[3], COSPI_16_64));
    step2[5] = V::round14(step1[5].dot(COSPI_16_64, step1[2], COSPI_16_64));
    let step3 = [
        step1[0] + step2[3],
        step1[1] + step2[2],
        step1[1] - step2[2],
        step1[0] - step2[3],
        step1[7] - step2[4],
        step1[6] - step2[5],
        step1[6] + step2[5],
        step1[7] + step2[4],
    ];
    step2[1] = V::round14(step3[1].dot(-COSPI_8_64, step3[6], COSPI_24_64));
    step2[2] = V::round14(step3[2].dot(COSPI_24_64, step3[5], COSPI_8_64));
    step2[5] = V::round14(step3[2].dot(COSPI_8_64, step3[5], -COSPI_24_64));
    step2[6] = V::round14(step3[1].dot(COSPI_24_64, step3[6], COSPI_8_64));
    let s = [
        step3[0] + step2[1],
        step3[0] - step2[1],
        step3[3] + step2[2],
        step3[3] - step2[2],
        step3[4] - step2[5],
        step3[4] + step2[5],
        step3[7] - step2[6],
        step3[7] + step2[6],
    ];
    for k in 0..8 {
        o[2 * k] = even[k];
    }
    o[1] = V::round14(s[0].dot(COSPI_30_64, s[7], COSPI_2_64));
    o[9] = V::round14(s[1].dot(COSPI_14_64, s[6], COSPI_18_64));
    o[5] = V::round14(s[2].dot(COSPI_22_64, s[5], COSPI_10_64));
    o[13] = V::round14(s[3].dot(COSPI_6_64, s[4], COSPI_26_64));
    o[3] = V::round14(s[3].dot(-COSPI_26_64, s[4], COSPI_6_64));
    o[11] = V::round14(s[2].dot(-COSPI_10_64, s[5], COSPI_22_64));
    o[7] = V::round14(s[1].dot(-COSPI_18_64, s[6], COSPI_14_64));
    o[15] = V::round14(s[0].dot(-COSPI_2_64, s[7], COSPI_30_64));
}

/// libvpx's `fadst4` (`vp9_dct.c`), the sums of its products made in 32
/// bits as its own kernels make them.
#[inline(always)]
fn fadst4<V: Lanes16>(i: &[V; 4], o: &mut [V; 4]) {
    let zero = V::splat(0);
    let x0 = i[0].dot(SINPI_1_9, i[1], SINPI_2_9) + i[3].dot(SINPI_4_9, zero, 0);
    let x1 = i[0].dot(SINPI_3_9, i[1], SINPI_3_9) - i[3].dot(SINPI_3_9, zero, 0);
    let x2 = i[0].dot(SINPI_4_9, i[1], -SINPI_1_9) + i[3].dot(SINPI_2_9, zero, 0);
    let x3 = i[2].dot(SINPI_3_9, zero, 0);
    *o = [V::round14(x0 + x3), V::round14(x1), V::round14(x2 - x3), V::round14(x2 - x0 + x3)];
}

/// libvpx's `fadst8` (`vp9_dct.c`).
#[inline(always)]
fn fadst8<V: Lanes16>(i: &[V; 8], o: &mut [V; 8]) {
    let x = [i[7], i[0], i[5], i[2], i[3], i[4], i[1], i[6]];
    // Stage 1.
    let s0 = x[0].dot(COSPI_2_64, x[1], COSPI_30_64);
    let s1 = x[0].dot(COSPI_30_64, x[1], -COSPI_2_64);
    let s2 = x[2].dot(COSPI_10_64, x[3], COSPI_22_64);
    let s3 = x[2].dot(COSPI_22_64, x[3], -COSPI_10_64);
    let s4 = x[4].dot(COSPI_18_64, x[5], COSPI_14_64);
    let s5 = x[4].dot(COSPI_14_64, x[5], -COSPI_18_64);
    let s6 = x[6].dot(COSPI_26_64, x[7], COSPI_6_64);
    let s7 = x[6].dot(COSPI_6_64, x[7], -COSPI_26_64);
    let x = [s0 + s4, s1 + s5, s2 + s6, s3 + s7, s0 - s4, s1 - s5, s2 - s6, s3 - s7].map(V::round14);
    // Stage 2.
    let s4 = x[4].dot(COSPI_8_64, x[5], COSPI_24_64);
    let s5 = x[4].dot(COSPI_24_64, x[5], -COSPI_8_64);
    let s6 = x[6].dot(-COSPI_24_64, x[7], COSPI_8_64);
    let s7 = x[6].dot(COSPI_8_64, x[7], COSPI_24_64);
    let (x0, x1, x2, x3) = (x[0] + x[2], x[1] + x[3], x[0] - x[2], x[1] - x[3]);
    let [x4, x5, x6, x7] = [s4 + s6, s5 + s7, s4 - s6, s5 - s7].map(V::round14);
    // Stage 3.
    let half = |a: V, b: V, sign: i32| V::round14(a.dot(COSPI_16_64, b, sign * COSPI_16_64));
    *o = [x0, -x4, half(x6, x7, 1), -half(x2, x3, 1), half(x2, x3, -1), -half(x6, x7, -1), x5, -x1];
}

/// libvpx's `fadst16` (`vp9_dct.c`).
#[inline(always)]
fn fadst16<V: Lanes16>(i: &[V; 16], o: &mut [V; 16]) {
    let x = [i[15], i[0], i[13], i[2], i[11], i[4], i[9], i[6], i[7], i[8], i[5], i[10], i[3], i[12], i[1], i[14]];
    // Stage 1: each pair turned by an odd sixty-fourth, and the halves
    // summed and differenced.
    let mut s = [x[0].dot(0, x[0], 0); 16];
    for (k, (a, b)) in ODD_TURNS.into_iter().enumerate() {
        s[2 * k] = x[2 * k].dot(a, x[2 * k + 1], b);
        s[2 * k + 1] = x[2 * k].dot(b, x[2 * k + 1], -a);
    }
    let mut x = [V::splat(0); 16];
    for k in 0..8 {
        x[k] = V::round14(s[k] + s[k + 8]);
        x[k + 8] = V::round14(s[k] - s[k + 8]);
    }
    // Stage 2.
    let s8 = x[8].dot(COSPI_4_64, x[9], COSPI_28_64);
    let s9 = x[8].dot(COSPI_28_64, x[9], -COSPI_4_64);
    let s10 = x[10].dot(COSPI_20_64, x[11], COSPI_12_64);
    let s11 = x[10].dot(COSPI_12_64, x[11], -COSPI_20_64);
    let s12 = x[12].dot(-COSPI_28_64, x[13], COSPI_4_64);
    let s13 = x[12].dot(COSPI_4_64, x[13], COSPI_28_64);
    let s14 = x[14].dot(-COSPI_12_64, x[15], COSPI_20_64);
    let s15 = x[14].dot(COSPI_20_64, x[15], COSPI_12_64);
    let x = [
        x[0] + x[4],
        x[1] + x[5],
        x[2] + x[6],
        x[3] + x[7],
        x[0] - x[4],
        x[1] - x[5],
        x[2] - x[6],
        x[3] - x[7],
        V::round14(s8 + s12),
        V::round14(s9 + s13),
        V::round14(s10 + s14),
        V::round14(s11 + s15),
        V::round14(s8 - s12),
        V::round14(s9 - s13),
        V::round14(s10 - s14),
        V::round14(s11 - s15),
    ];
    // Stage 3.
    let s4 = x[4].dot(COSPI_8_64, x[5], COSPI_24_64);
    let s5 = x[4].dot(COSPI_24_64, x[5], -COSPI_8_64);
    let s6 = x[6].dot(-COSPI_24_64, x[7], COSPI_8_64);
    let s7 = x[6].dot(COSPI_8_64, x[7], COSPI_24_64);
    let s12 = x[12].dot(COSPI_8_64, x[13], COSPI_24_64);
    let s13 = x[12].dot(COSPI_24_64, x[13], -COSPI_8_64);
    let s14 = x[14].dot(-COSPI_24_64, x[15], COSPI_8_64);
    let s15 = x[14].dot(COSPI_8_64, x[15], COSPI_24_64);
    let x = [
        x[0] + x[2],
        x[1] + x[3],
        x[0] - x[2],
        x[1] - x[3],
        V::round14(s4 + s6),
        V::round14(s5 + s7),
        V::round14(s4 - s6),
        V::round14(s5 - s7),
        x[8] + x[10],
        x[9] + x[11],
        x[8] - x[10],
        x[9] - x[11],
        V::round14(s12 + s14),
        V::round14(s13 + s15),
        V::round14(s12 - s14),
        V::round14(s13 - s15),
    ];
    // Stage 4.
    let half = |a: V, j: i32, b: V, k: i32| V::round14(a.dot(j * COSPI_16_64, b, k * COSPI_16_64));
    *o = [
        x[0],
        -x[8],
        x[12],
        -x[4],
        half(x[6], 1, x[7], 1),
        half(x[14], -1, x[15], -1),
        half(x[10], 1, x[11], 1),
        half(x[2], -1, x[3], -1),
        half(x[2], 1, x[3], -1),
        half(x[10], -1, x[11], 1),
        half(x[14], 1, x[15], -1),
        half(x[6], -1, x[7], 1),
        x[5],
        -x[13],
        x[9],
        -x[1],
    ];
}

// Everything from a kernel's entry down to the `Lanes` it runs on is called
// directly and inlined, with no closure or function value between: out of
// line, a 4×4 takes twice the time. Except in a debug build, where each
// transform of a size and type, `inverse_n` or `forward_passes`, is a
// function of its own: unoptimized, each inlined copy keeps stack of its
// own, and `inverse_add` took 2.2 MiB in AVX2, more than a thread has.

/// A one-dimensional inverse transform of `N` points, or the 32-point
/// forward one, which is in the inverses' lanes.
trait OneD<const N: usize> {
    fn run<V: Lanes>(i: &[V; N], o: &mut [V; N]);
}

macro_rules! one_d {
    ($($name:ident = $f:ident($n:literal);)*) => {
        $(
            struct $name;

            impl OneD<$n> for $name {
                #[inline(always)]
                fn run<V: Lanes>(i: &[V; $n], o: &mut [V; $n]) {
                    $f(i, o);
                }
            }
        )*
    };
}

one_d! {
    Fdct32 = fdct32(32);
    Idct4 = idct4(4);
    Idct8 = idct8(8);
    Idct16 = idct16(16);
    Idct32 = idct32(32);
    Iadst4 = iadst4(4);
    Iadst8 = iadst8(8);
    Iadst16 = iadst16(16);
}

/// A one-dimensional forward transform of `N` points, under 32.
trait Forward<const N: usize> {
    fn run<V: Lanes16>(i: &[V; N], o: &mut [V; N]);
}

macro_rules! forward {
    ($($name:ident = $f:ident($n:literal);)*) => {
        $(
            struct $name;

            impl Forward<$n> for $name {
                #[inline(always)]
                fn run<V: Lanes16>(i: &[V; $n], o: &mut [V; $n]) {
                    $f(i, o);
                }
            }
        )*
    };
}

forward! {
    Fdct4 = fdct4(4);
    Fdct8 = fdct8(8);
    Fdct16 = fdct16(16);
    Fadst4 = fadst4(4);
    Fadst8 = fadst8(8);
    Fadst16 = fadst16(16);
}

/// The forward and inverse transforms of `N` points of one kind, DCT or ADST.
trait Kind<const N: usize> {
    type Forward: Forward<N>;
    type Inverse: OneD<N>;
}

struct Dct;
struct Adst;

macro_rules! kinds {
    ($($kind:ident($n:literal) = $forward:ident, $inverse:ident;)*) => {
        $(
            impl Kind<$n> for $kind {
                type Forward = $forward;
                type Inverse = $inverse;
            }
        )*
    };
}

kinds! {
    Dct(4) = Fdct4, Idct4;
    Dct(8) = Fdct8, Idct8;
    Dct(16) = Fdct16, Idct16;
    Adst(4) = Fadst4, Iadst4;
    Adst(8) = Fadst8, Iadst8;
    Adst(16) = Fadst16, Iadst16;
}

/// Run `T` down every column of the `N`×`N` `src`, a vector of columns at a
/// time: `dst[k * N + c]` is output `k` of column `c`.
#[inline(always)]
fn forward_down<V: Lanes16, T: Forward<N>, const N: usize>(src: &[i16], dst: &mut [i16]) {
    let (mut x, mut out) = ([V::splat(0); N], [V::splat(0); N]);
    for c in (0..N).step_by(V::WIDTH) {
        for (r, x) in x.iter_mut().enumerate() {
            *x = V::load(&src[r * N + c..]);
        }
        T::run(&x, &mut out);
        for (k, v) in out.iter().enumerate() {
            v.store(&mut dst[k * N + c..]);
        }
    }
}

/// Run `T` down every column of the `N`×`N` `src`, a vector of columns at a
/// time: `dst[k * N + c]` is output `k` of column `c`. With `skip_zero`,
/// columns that are all zero are taken to transform to zeros.
#[inline(always)]
fn down<V: Lanes, T: OneD<N>, const N: usize>(skip_zero: bool, src: &[i32], dst: &mut [i32]) {
    let (mut x, mut out) = ([V::splat(0); N], [V::splat(0); N]);
    for c in (0..N).step_by(V::WIDTH) {
        for (r, x) in x.iter_mut().enumerate() {
            *x = V::load(&src[r * N + c..]);
        }
        if skip_zero && all_zero(&x) {
            for r in 0..N {
                V::splat(0).store(&mut dst[r * N + c..]);
            }
            continue;
        }
        T::run(&x, &mut out);
        for (k, v) in out.iter().enumerate() {
            v.store(&mut dst[k * N + c..]);
        }
    }
}

#[inline(always)]
fn all_zero<V: Lanes, const N: usize>(x: &[V; N]) -> bool {
    for v in x {
        if !v.is_zero() {
            return false;
        }
    }
    true
}

/// How a pass of the forward DCT scales a block: each value shifted left
/// `left`, plus `plus`, plus one more where it is positive if `up` or
/// negative if `down`, shifted right `shift`.
#[derive(Clone, Copy)]
struct Scale {
    left: u32,
    plus: i16,
    up: bool,
    down: bool,
    shift: u32,
}

impl Scale {
    const KEEP: Self = Self::left(0);

    const fn left(left: u32) -> Self {
        Self { left, plus: 0, up: false, down: false, shift: 0 }
    }

    const fn shifted(plus: i16, up: bool, down: bool, shift: u32) -> Self {
        Self { left: 0, plus, up, down, shift }
    }

    /// Each value of `block` scaled, a vector at a time.
    #[inline(always)]
    fn apply<V: Lanes16>(self, block: &mut [i16]) {
        for at in (0..block.len()).step_by(V::WIDTH) {
            let mut v = V::load(&block[at..]);
            let mut sum = v.shl(self.left) + V::splat(self.plus);
            if self.up {
                sum = sum + v.positive();
            }
            if self.down {
                sum = sum + v.negative();
            }
            v = sum.shr(self.shift);
            v.store(&mut block[at..]);
        }
    }
}

/// The forward transform of type `tx_type` of an `n`×`n` residual, `n` 4
/// to 32 (32 a DCT only), both raster order: `out[v * n + h]` holds
/// vertical frequency `v` and horizontal `h`.
pub(super) fn forward(n: usize, tx_type: TxType, residual: &[i16], out: &mut [i16]) {
    match n {
        4 => forward_in::<Narrow4>(n, tx_type, residual, out),
        8 => forward_in::<Narrow8>(n, tx_type, residual, out),
        16 => forward_in::<Narrow16>(n, tx_type, residual, out),
        _ => forward_32::<Vector>(residual, out),
    }
}

/// `forward` under 32×32 in lanes of `V`: columns, then rows, each pass
/// scaled as libvpx's `vpx_fdct*_c` scales it, or for a hybrid `vp9_fht*_c`.
#[inline(always)]
fn forward_in<V: Lanes16>(n: usize, tx_type: TxType, residual: &[i16], out: &mut [i16]) {
    let quarter = Scale::shifted(1, false, false, 2);
    // Halved toward zero.
    let half = Scale::shifted(0, false, true, 1);
    let scales = |n| match n {
        4 => [Scale::left(4), Scale::KEEP, quarter],
        8 => [Scale::left(2), Scale::KEEP, half],
        // A hybrid's quarter rounds a negative half away from zero.
        _ => [Scale::left(2), if tx_type == TxType::DctDct { quarter } else { Scale::shifted(1, false, true, 2) }, Scale::KEEP],
    };
    macro_rules! hybrid {
        ($n:literal, $area:literal) => {
            match tx_type {
                TxType::DctDct => forward_n::<V, Dct, Dct, $n, $area>(residual, out, scales($n)),
                TxType::AdstDct => forward_n::<V, Adst, Dct, $n, $area>(residual, out, scales($n)),
                TxType::DctAdst => forward_n::<V, Dct, Adst, $n, $area>(residual, out, scales($n)),
                TxType::AdstAdst => forward_n::<V, Adst, Adst, $n, $area>(residual, out, scales($n)),
            }
        };
    }
    match n {
        4 => hybrid!(4, 16),
        8 => hybrid!(8, 64),
        16 => hybrid!(16, 256),
        _ => unreachable!("a {n}-point transform in 16 bits"),
    }
}

/// The `N`×`N` forward transform of `AREA` values: the residual scaled by
/// `before`, `C` down its columns, the result scaled by `between`, `R` down
/// its rows and that scaled by `after`.
#[inline(always)]
fn forward_n<V: Lanes16, C: Kind<N>, R: Kind<N>, const N: usize, const AREA: usize>(residual: &[i16], out: &mut [i16], scales: [Scale; 3]) {
    forward_passes::<V, C::Forward, R::Forward, N, AREA>(residual, out, scales);
}

/// `forward_n` of the one-dimensional transforms themselves.
#[cfg_attr(not(debug_assertions), inline(always))]
fn forward_passes<V: Lanes16, C: Forward<N>, R: Forward<N>, const N: usize, const AREA: usize>(residual: &[i16], out: &mut [i16], [before, between, after]: [Scale; 3]) {
    let (mut a, mut b) = ([0; AREA], [0; AREA]);
    b.copy_from_slice(&residual[..AREA]);
    before.apply::<V>(&mut b);
    // A 4×4's first DC is nudged off zero.
    if N == 4 && b[0] != 0 {
        b[0] += 1;
    }
    forward_down::<V, C, N>(&b, &mut a);
    between.apply::<V>(&mut a);
    V::transpose(N, &a, &mut b);
    forward_down::<V, R, N>(&b, &mut a);
    after.apply::<V>(&mut a);
    // Each pass left its result transposed.
    V::transpose(N, &a, &mut out[..AREA]);
}

/// The 32×32 forward DCT, as `vpx_fdct32x32_c`: its second pass leaves 16
/// bits, so it is made in 32, in the lanes of the inverses, the residual
/// widened to them and the coefficients, which are back inside 16, narrowed.
#[cfg_attr(not(debug_assertions), inline(always))]
fn forward_32<V: Lanes>(residual: &[i16], out: &mut [i16]) {
    let (mut a, mut b) = ([0; 1024], [0; 1024]);
    for (wide, &v) in b.iter_mut().zip(residual) {
        *wide = i32::from(v) * 4;
    }
    down::<V, Fdct32, 32>(false, &b, &mut a);
    scale_32::<V>(&mut a, true);
    V::transpose(32, &a, &mut b);
    down::<V, Fdct32, 32>(false, &b, &mut a);
    scale_32::<V>(&mut a, false);
    V::transpose(32, &a, &mut b);
    for (narrow, &v) in out[..1024].iter_mut().zip(&b) {
        debug_assert!(i16::try_from(v).is_ok(), "{v} is past 16 bits");
        *narrow = v as i16;
    }
}

/// A pass of the 32×32 forward DCT quartered: a half rounded up where
/// positive if `up`, down where negative otherwise.
#[inline(always)]
fn scale_32<V: Lanes>(block: &mut [i32], up: bool) {
    for at in (0..block.len()).step_by(V::WIDTH) {
        let v = V::load(&block[at..]);
        (v + V::splat(1) + if up { v.positive() } else { v.negative() }).shr(2).store(&mut block[at..]);
    }
}

/// Add the inverse transform of type `tx_type` of `n`×`n` dequantized
/// `coeffs` to the `n`×`n` pixels at the start of `dst`, rows `stride`
/// apart. `eob` is how many coefficients the scan reached; with one, only a
/// DCT's DC is there, which libvpx's decoder takes its own path for, to the
/// same result.
pub(super) fn inverse_add(n: usize, tx_type: TxType, coeffs: &[i32], eob: usize, dst: &mut [u8], stride: usize) {
    if n == 4 {
        return inverse_add_in::<Vector4>(n, tx_type, coeffs, eob, dst, stride);
    }
    inverse_add_in::<Vector>(n, tx_type, coeffs, eob, dst, stride);
}

/// `inverse_add` in lanes of `V`: rows, then columns, then the rounding
/// shift and the add, as `vpx_idct*_add_c` and `vp9_iht*_add_c`.
#[inline(always)]
fn inverse_add_in<V: Lanes>(n: usize, tx_type: TxType, coeffs: &[i32], eob: usize, dst: &mut [u8], stride: usize) {
    let shift = match n {
        4 => 4,
        8 => 5,
        _ => 6,
    };
    macro_rules! hybrid {
        ($n:literal, $area:literal) => {
            match tx_type {
                TxType::DctDct => unreachable!("a DCT taken above"),
                TxType::AdstDct => inverse_n::<V, <Adst as Kind<$n>>::Inverse, <Dct as Kind<$n>>::Inverse, $n, $area>(shift, coeffs, dst, stride),
                TxType::DctAdst => inverse_n::<V, <Dct as Kind<$n>>::Inverse, <Adst as Kind<$n>>::Inverse, $n, $area>(shift, coeffs, dst, stride),
                TxType::AdstAdst => inverse_n::<V, <Adst as Kind<$n>>::Inverse, <Adst as Kind<$n>>::Inverse, $n, $area>(shift, coeffs, dst, stride),
            }
        };
    }
    if tx_type != TxType::DctDct {
        return match n {
            4 => hybrid!(4, 16),
            8 => hybrid!(8, 64),
            16 => hybrid!(16, 256),
            _ => unreachable!("a {n}-point hybrid"),
        };
    }
    if eob == 1 {
        let dc = round14(round14(coeffs[0] * COSPI_16_64) as i16 as i32 * COSPI_16_64) as i16 as i32;
        let add = (dc + (1 << (shift - 1))) >> shift;
        for r in 0..n {
            for p in &mut dst[r * stride..][..n] {
                *p = (i32::from(*p) + add).clamp(0, 255) as u8;
            }
        }
        return;
    }
    match n {
        4 => inverse_n::<V, Idct4, Idct4, 4, 16>(shift, coeffs, dst, stride),
        8 => inverse_n::<V, Idct8, Idct8, 8, 64>(shift, coeffs, dst, stride),
        16 => inverse_n::<V, Idct16, Idct16, 16, 256>(shift, coeffs, dst, stride),
        32 => inverse_n::<V, Idct32, Idct32, 32, 1024>(shift, coeffs, dst, stride),
        _ => unreachable!("a {n}-point transform"),
    }
}

/// The `N`×`N` inverse transform of `AREA` coefficients, `R` along the
/// rows and then `C` down the columns, added to `dst`.
#[cfg_attr(not(debug_assertions), inline(always))]
fn inverse_n<V: Lanes, C: OneD<N>, R: OneD<N>, const N: usize, const AREA: usize>(shift: u32, coeffs: &[i32], dst: &mut [u8], stride: usize) {
    let (mut a, mut b) = ([0; AREA], [0; AREA]);
    // The rows, as columns; a row of zeros transforms to zeros.
    V::transpose(N, &coeffs[..AREA], &mut a);
    down::<V, R, N>(true, &a, &mut b);
    V::transpose(N, &b, &mut a);
    down::<V, C, N>(false, &a, &mut b);
    for (r, values) in b.as_chunks::<N>().0.iter().enumerate() {
        V::add_shifted(values, shift, &mut dst[r * stride..][..N]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The orthonormal 2D DCT, in floating point, of an `n`×`n` block.
    fn reference(n: usize, x: &[i16]) -> Vec<f64> {
        let basis = |k: usize, i: usize| {
            let scale = if k == 0 { (1.0 / n as f64).sqrt() } else { (2.0 / n as f64).sqrt() };
            scale * (std::f64::consts::PI * (2 * i + 1) as f64 * k as f64 / (2 * n) as f64).cos()
        };
        let mut out = vec![0.0; n * n];
        for v in 0..n {
            for h in 0..n {
                out[v * n + h] = (0..n).flat_map(|r| (0..n).map(move |c| (r, c))).map(|(r, c)| basis(v, r) * basis(h, c) * f64::from(x[r * n + c])).sum();
            }
        }
        out
    }

    /// A residual of `-amplitude..=amplitude`.
    fn residual(n: usize, seed: u32, amplitude: u32) -> Vec<i16> {
        let mut state = seed;
        (0..n * n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                ((state % (2 * amplitude + 1)) as i32 - amplitude as i32) as i16
            })
            .collect()
    }

    /// `forward` a value at a time, the reference: a sum or a rounded value
    /// past 16 bits panics in it.
    fn forward_scalar(n: usize, tx_type: TxType, residual: &[i16], out: &mut [i16]) {
        if n == 32 {
            return forward_32::<i32>(residual, out);
        }
        forward_in::<i16>(n, tx_type, residual, out);
    }

    /// The coefficients a decoder is handed, in the 32 bits the inverse
    /// takes them in.
    fn wide(coeffs: &[i16]) -> Vec<i32> {
        coeffs.iter().map(|&c| i32::from(c)).collect()
    }

    #[test]
    fn the_forward_transform_is_the_dct_at_vp9s_scale() {
        // 8 times orthonormal up to 16×16 and 4 times at 32×32, the scales
        // the inverse undoes, to within the rounding of libvpx's fixed
        // point: worst at 16×16, which drops two bits between its passes.
        for (n, scale, tolerance) in [(4, 8.0, 1.0), (8, 8.0, 2.0), (16, 8.0, 6.0), (32, 4.0, 3.0)] {
            for seed in 1..4 {
                let x = residual(n, seed * 7919, 255);
                let mut got = vec![0; n * n];
                forward(n, TxType::DctDct, &x, &mut got);
                for (i, (g, r)) in got.iter().zip(reference(n, &x)).enumerate() {
                    assert!((f64::from(*g) - scale * r).abs() <= tolerance, "{n}x{n} coefficient {i}: {g} against {}", scale * r);
                }
            }
        }
    }

    /// Every size and type of transform there is: the hybrids below 32×32.
    fn transforms() -> impl Iterator<Item = (usize, TxType)> {
        let types = [TxType::DctDct, TxType::AdstDct, TxType::DctAdst, TxType::AdstAdst];
        [4, 8, 16].into_iter().flat_map(move |n| types.map(|t| (n, t))).chain([(32, TxType::DctDct)])
    }

    #[test]
    fn the_inverse_undoes_the_forward_transform() {
        for (n, tx_type) in transforms() {
            for seed in 1..4 {
                // On mid grey, and small enough that neither end clips.
                let x = residual(n, seed * 104_729, 100);
                let mut coeffs = vec![0; n * n];
                forward(n, tx_type, &x, &mut coeffs);
                let mut dst = vec![128u8; n * n];
                inverse_add(n, tx_type, &wide(&coeffs), n * n, &mut dst, n);
                for (i, (d, r)) in dst.iter().zip(&x).enumerate() {
                    let back = i32::from(*d) - 128;
                    assert!((back - i32::from(*r)).abs() <= 1, "{n}x{n} {tx_type:?} pixel {i}: {back} for {r}");
                }
            }
        }
    }

    #[test]
    fn an_adst_codes_a_ramp_from_its_edge_in_less_than_the_dct() {
        // A residual that grows away from the edge it is predicted from,
        // down the columns, as V leaves: an ADST down them takes less to
        // code it, in the sum of the coefficients' magnitudes.
        for n in [4, 8, 16] {
            let x: Vec<i16> = (0..n * n).map(|i| (i / n + 1) as i16 * 4).collect();
            let size = |tx_type| {
                let mut coeffs = vec![0; n * n];
                forward(n, tx_type, &x, &mut coeffs);
                coeffs.iter().map(|&c| i32::from(c).abs()).sum::<i32>()
            };
            assert!(size(TxType::AdstDct) < size(TxType::DctDct), "{n}x{n}: {} against {}", size(TxType::AdstDct), size(TxType::DctDct));
        }
    }

    #[test]
    fn a_dc_alone_adds_what_the_whole_transform_adds() {
        for n in [4, 8, 16, 32] {
            for dc in [-4000, -77, 1, 9, 1234] {
                let mut coeffs = vec![0; n * n];
                coeffs[0] = dc;
                let (mut fast, mut full) = (vec![100u8; n * n], vec![100u8; n * n]);
                inverse_add(n, TxType::DctDct, &coeffs, 1, &mut fast, n);
                inverse_add(n, TxType::DctDct, &coeffs, 2, &mut full, n);
                assert_eq!(fast, full, "{n}x{n}, DC {dc}");
            }
        }
    }

    #[test]
    fn a_residual_at_its_largest_stays_in_16_bits() {
        // The residuals that reach furthest are of the largest difference
        // two pixels have, signed as one of the transform's own patterns is
        // or as nothing is. The reference panics past 16 bits, and the
        // vectors, which would wrap or saturate there, are held to it.
        for (n, tx_type) in transforms() {
            let basis = |k: usize, i: usize| (std::f64::consts::PI * (2 * i + 1) as f64 * k as f64 / (2 * n) as f64).cos();
            let patterns = (0..n * n).map(|p| (0..n * n).map(|i| if basis(p / n, i / n) * basis(p % n, i % n) < 0.0 { -255 } else { 255 }).collect::<Vec<i16>>());
            let noise = (1..200).map(|seed| residual(n, seed, 255).iter().map(|&v| if v < 0 { -255 } else { 255 }).collect());
            for (i, source) in patterns.chain(noise).enumerate() {
                for sign in [1, -1] {
                    let source: Vec<i16> = source.iter().map(|&v| v * sign).collect();
                    let (mut scalar, mut vector) = (vec![0; n * n], vec![0; n * n]);
                    forward_scalar(n, tx_type, &source, &mut scalar);
                    forward(n, tx_type, &source, &mut vector);
                    assert_eq!(scalar, vector, "forward {n}x{n} {tx_type:?}, pattern {i}");
                }
            }
        }
    }

    #[test]
    fn the_widest_lanes_transform_as_one_lane_does() {
        for (n, tx_type) in transforms() {
            for seed in 1..40 {
                // Residuals as large as a pixel's, and smaller.
                let source = residual(n, seed, [255, 40, 3][seed as usize % 3]);
                let (mut scalar, mut vector) = (vec![0; n * n], vec![0; n * n]);
                forward_scalar(n, tx_type, &source, &mut scalar);
                forward(n, tx_type, &source, &mut vector);
                assert_eq!(scalar, vector, "forward {n}x{n} {tx_type:?}, seed {seed}");
                // A DCT's coefficients anywhere in 16 bits, which wrap inside
                // the inverse; an ADST's as large as a pixel's residual
                // makes, whose products libvpx keeps in 32 bits. Some rows
                // are left empty.
                let mut coeffs = wide(&if tx_type == TxType::DctDct { residual(n, seed + 1000, 32767) } else { scalar });
                for row in coeffs.chunks_exact_mut(n).skip(seed as usize % n) {
                    row.fill(0);
                }
                let pixels: Vec<u8> = source.iter().map(|&v| (v + 128).clamp(0, 255) as u8).collect();
                let (mut scalar, mut vector) = (pixels.clone(), pixels);
                inverse_add_in::<i32>(n, tx_type, &coeffs, n * n, &mut scalar, n);
                inverse_add(n, tx_type, &coeffs, n * n, &mut vector, n);
                assert_eq!(scalar, vector, "inverse {n}x{n} {tx_type:?}, seed {seed}");
            }
        }
    }
}
