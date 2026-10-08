//! Two-dimensional DCTs: forward, to code a residual, and inverse, to
//! reconstruct it as a decoder does.
//!
//! A forward transform is the encoder's own business; these follow libvpx's
//! `vpx_fdct*_c`, whose scale the inverse expects. The inverse is libvpx's
//! `vpx_idct*_add_c` and exact: a decoder adds its result to the prediction,
//! and the encoder predicts later blocks from the same pixels.

use super::dct::{fdct32, idct4, idct8, idct16, idct32};
#[cfg(target_arch = "x86_64")]
use super::lanes::{avx2::I32x8, has_avx2};
use super::lanes::{Lanes, Vector};

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

fn round14(x: i32) -> i32 {
    (x + (1 << 13)) >> 14
}

/// The 4-point forward DCT inside `vpx_fdct4x4_c`.
#[inline(always)]
fn fdct4<V: Lanes>(i: &[V; 4], o: &mut [V; 4]) {
    let s = [i[0] + i[3], i[1] + i[2], i[1] - i[2], i[0] - i[3]];
    *o = [
        ((s[0] + s[1]) * COSPI_16_64).round14(),
        (s[2] * COSPI_24_64 + s[3] * COSPI_8_64).round14(),
        ((s[0] - s[1]) * COSPI_16_64).round14(),
        (-s[2] * COSPI_8_64 + s[3] * COSPI_24_64).round14(),
    ];
}

/// The 8-point forward DCT inside `vpx_fdct8x8_c`.
#[inline(always)]
fn fdct8<V: Lanes>(i: &[V; 8], o: &mut [V; 8]) {
    let s = [i[0] + i[7], i[1] + i[6], i[2] + i[5], i[3] + i[4], i[3] - i[4], i[2] - i[5], i[1] - i[6], i[0] - i[7]];
    let (x0, x1, x2, x3) = (s[0] + s[3], s[1] + s[2], s[1] - s[2], s[0] - s[3]);
    let even = [((x0 + x1) * COSPI_16_64).round14(), (x2 * COSPI_24_64 + x3 * COSPI_8_64).round14(), ((x0 - x1) * COSPI_16_64).round14(), (-x2 * COSPI_8_64 + x3 * COSPI_24_64).round14()];
    let t2 = ((s[6] - s[5]) * COSPI_16_64).round14();
    let t3 = ((s[6] + s[5]) * COSPI_16_64).round14();
    let (x0, x1, x2, x3) = (s[4] + t2, s[4] - t2, s[7] - t3, s[7] + t3);
    *o = [
        even[0],
        (x0 * COSPI_28_64 + x3 * COSPI_4_64).round14(),
        even[1],
        (x2 * COSPI_12_64 + x1 * -COSPI_20_64).round14(),
        even[2],
        (x1 * COSPI_12_64 + x2 * COSPI_20_64).round14(),
        even[3],
        (x3 * COSPI_28_64 + x0 * -COSPI_4_64).round14(),
    ];
}

/// The 16-point forward DCT inside `vpx_fdct16x16_c`: the even half is the
/// 8-point one of the folded input.
#[inline(always)]
fn fdct16<V: Lanes>(i: &[V; 16], o: &mut [V; 16]) {
    let (mut folded, mut step1) = ([V::splat(0); 8], [V::splat(0); 8]);
    for k in 0..8 {
        folded[k] = i[k] + i[15 - k];
        step1[k] = i[7 - k] - i[8 + k];
    }
    let mut even = [V::splat(0); 8];
    fdct8(&folded, &mut even);
    let mut step2 = [V::splat(0); 8];
    step2[2] = ((step1[5] - step1[2]) * COSPI_16_64).round14();
    step2[3] = ((step1[4] - step1[3]) * COSPI_16_64).round14();
    step2[4] = ((step1[4] + step1[3]) * COSPI_16_64).round14();
    step2[5] = ((step1[5] + step1[2]) * COSPI_16_64).round14();
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
    step2[1] = (step3[1] * -COSPI_8_64 + step3[6] * COSPI_24_64).round14();
    step2[2] = (step3[2] * COSPI_24_64 + step3[5] * COSPI_8_64).round14();
    step2[5] = (step3[2] * COSPI_8_64 - step3[5] * COSPI_24_64).round14();
    step2[6] = (step3[1] * COSPI_24_64 + step3[6] * COSPI_8_64).round14();
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
    o[1] = (s[0] * COSPI_30_64 + s[7] * COSPI_2_64).round14();
    o[9] = (s[1] * COSPI_14_64 + s[6] * COSPI_18_64).round14();
    o[5] = (s[2] * COSPI_22_64 + s[5] * COSPI_10_64).round14();
    o[13] = (s[3] * COSPI_6_64 + s[4] * COSPI_26_64).round14();
    o[3] = (s[3] * -COSPI_26_64 + s[4] * COSPI_6_64).round14();
    o[11] = (s[2] * -COSPI_10_64 + s[5] * COSPI_22_64).round14();
    o[7] = (s[1] * -COSPI_18_64 + s[6] * COSPI_14_64).round14();
    o[15] = (s[0] * -COSPI_2_64 + s[7] * COSPI_30_64).round14();
}

// Everything from a kernel's entry down to the `Lanes` it runs on is called
// directly and inlined, with no closure or function value between: AVX2's
// intrinsics are inlined only into code compiled for AVX2, which is the
// entry alone, so any function left out of line would call each of them.

/// A one-dimensional transform of `N` points.
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
    Fdct4 = fdct4(4);
    Fdct8 = fdct8(8);
    Fdct16 = fdct16(16);
    Fdct32 = fdct32(32);
    Idct4 = idct4(4);
    Idct8 = idct8(8);
    Idct16 = idct16(16);
    Idct32 = idct32(32);
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

/// How a pass of the forward DCT scales a block: each value times `times`,
/// plus `plus`, plus one more where it is positive if `up` or negative if
/// `down`, shifted right `shift`.
#[derive(Clone, Copy)]
struct Scale {
    times: i32,
    plus: i32,
    up: bool,
    down: bool,
    shift: u32,
}

impl Scale {
    const KEEP: Self = Self::times(1);

    const fn times(times: i32) -> Self {
        Self { times, plus: 0, up: false, down: false, shift: 0 }
    }

    const fn shifted(plus: i32, up: bool, down: bool, shift: u32) -> Self {
        Self { times: 1, plus, up, down, shift }
    }

    /// Each value of `block` scaled, a vector at a time.
    #[inline(always)]
    fn apply<V: Lanes>(self, block: &mut [i32]) {
        for at in (0..block.len()).step_by(V::WIDTH) {
            let mut v = V::load(&block[at..]);
            let mut sum = v * self.times + V::splat(self.plus);
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

/// The forward DCT of an `n`×`n` residual, `n` 4 to 32, both raster order:
/// `out[v * n + h]` holds vertical frequency `v` and horizontal `h`.
pub(super) fn forward(n: usize, residual: &[i32], out: &mut [i32]) {
    #[cfg(target_arch = "x86_64")]
    if n >= 8 && has_avx2() {
        // SAFETY: the machine has AVX2.
        return unsafe { forward_avx2(n, residual, out) };
    }
    forward_in::<Vector>(n, residual, out);
}

/// `forward` in AVX2's eight lanes, which are wider than a 4×4's rows; only
/// the tests transform a 4×4, no block being smaller than 8×8.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn forward_avx2(n: usize, residual: &[i32], out: &mut [i32]) {
    forward_in::<I32x8>(n, residual, out);
}

/// `forward` in lanes of `V`: columns, then rows, each pass scaled as
/// libvpx's `vpx_fdct*_c` scales it.
#[inline(always)]
fn forward_in<V: Lanes>(n: usize, residual: &[i32], out: &mut [i32]) {
    let quarter = Scale::shifted(1, false, false, 2);
    match n {
        4 => forward_n::<V, Fdct4, 4, 16>(residual, out, [Scale::times(16), Scale::KEEP, quarter]),
        // Halved toward zero.
        8 => forward_n::<V, Fdct8, 8, 64>(residual, out, [Scale::times(4), Scale::KEEP, Scale::shifted(0, false, true, 1)]),
        16 => forward_n::<V, Fdct16, 16, 256>(residual, out, [Scale::times(4), quarter, Scale::KEEP]),
        32 => forward_n::<V, Fdct32, 32, 1024>(residual, out, [Scale::times(4), Scale::shifted(1, true, false, 2), Scale::shifted(1, false, true, 2)]),
        _ => unreachable!("a {n}-point transform"),
    }
}

/// The `N`×`N` forward DCT of `AREA` values: the residual scaled by
/// `before`, `T` down its columns, the result scaled by `between`, `T` down
/// its rows and that scaled by `after`.
#[inline(always)]
fn forward_n<V: Lanes, T: OneD<N>, const N: usize, const AREA: usize>(residual: &[i32], out: &mut [i32], [before, between, after]: [Scale; 3]) {
    let (mut a, mut b) = ([0; AREA], [0; AREA]);
    b.copy_from_slice(&residual[..AREA]);
    before.apply::<V>(&mut b);
    // A 4×4's first DC is nudged off zero.
    if N == 4 && b[0] != 0 {
        b[0] += 1;
    }
    down::<V, T, N>(false, &b, &mut a);
    between.apply::<V>(&mut a);
    V::transpose(N, &a, &mut b);
    down::<V, T, N>(false, &b, &mut a);
    after.apply::<V>(&mut a);
    // Each pass left its result transposed.
    V::transpose(N, &a, &mut out[..AREA]);
}

/// Add the inverse DCT of `n`×`n` dequantized `coeffs` to the `n`×`n`
/// pixels at the start of `dst`, rows `stride` apart. `eob` is how many
/// coefficients the scan reached; with one, only the DC is there, which
/// libvpx's decoder takes its own path for, to the same result.
pub(super) fn inverse_add(n: usize, coeffs: &[i32], eob: usize, dst: &mut [u8], stride: usize) {
    #[cfg(target_arch = "x86_64")]
    if n >= 8 && has_avx2() {
        // SAFETY: the machine has AVX2.
        return unsafe { inverse_add_avx2(n, coeffs, eob, dst, stride) };
    }
    inverse_add_in::<Vector>(n, coeffs, eob, dst, stride);
}

/// `inverse_add` in AVX2's eight lanes, from 8×8 up as `forward_avx2`.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn inverse_add_avx2(n: usize, coeffs: &[i32], eob: usize, dst: &mut [u8], stride: usize) {
    inverse_add_in::<I32x8>(n, coeffs, eob, dst, stride);
}

/// `inverse_add` in lanes of `V`: rows, then columns, then the rounding
/// shift and the add, as `vpx_idct*_add_c`.
#[inline(always)]
fn inverse_add_in<V: Lanes>(n: usize, coeffs: &[i32], eob: usize, dst: &mut [u8], stride: usize) {
    let shift = match n {
        4 => 4,
        8 => 5,
        _ => 6,
    };
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
        4 => inverse_n::<V, Idct4, 4, 16>(shift, coeffs, dst, stride),
        8 => inverse_n::<V, Idct8, 8, 64>(shift, coeffs, dst, stride),
        16 => inverse_n::<V, Idct16, 16, 256>(shift, coeffs, dst, stride),
        32 => inverse_n::<V, Idct32, 32, 1024>(shift, coeffs, dst, stride),
        _ => unreachable!("a {n}-point transform"),
    }
}

/// The `N`×`N` inverse DCT of `AREA` coefficients, `T` along the rows and
/// then down the columns, added to `dst`.
#[inline(always)]
fn inverse_n<V: Lanes, T: OneD<N>, const N: usize, const AREA: usize>(shift: u32, coeffs: &[i32], dst: &mut [u8], stride: usize) {
    let (mut a, mut b) = ([0; AREA], [0; AREA]);
    // The rows, as columns; a row of zeros transforms to zeros.
    V::transpose(N, &coeffs[..AREA], &mut a);
    down::<V, T, N>(true, &a, &mut b);
    V::transpose(N, &b, &mut a);
    down::<V, T, N>(false, &a, &mut b);
    for (r, values) in b.as_chunks::<N>().0.iter().enumerate() {
        V::add_shifted(values, shift, &mut dst[r * stride..][..N]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The orthonormal 2D DCT, in floating point, of an `n`×`n` block.
    fn reference(n: usize, x: &[i32]) -> Vec<f64> {
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
    fn residual(n: usize, seed: u32, amplitude: u32) -> Vec<i32> {
        let mut state = seed;
        (0..n * n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state % (2 * amplitude + 1)) as i32 - amplitude as i32
            })
            .collect()
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
                forward(n, &x, &mut got);
                for (i, (g, r)) in got.iter().zip(reference(n, &x)).enumerate() {
                    assert!((f64::from(*g) - scale * r).abs() <= tolerance, "{n}x{n} coefficient {i}: {g} against {}", scale * r);
                }
            }
        }
    }

    #[test]
    fn the_inverse_undoes_the_forward_transform() {
        for n in [4, 8, 16, 32] {
            for seed in 1..4 {
                // On mid grey, and small enough that neither end clips.
                let x = residual(n, seed * 104_729, 100);
                let mut coeffs = vec![0; n * n];
                forward(n, &x, &mut coeffs);
                let mut dst = vec![128u8; n * n];
                inverse_add(n, &coeffs, n * n, &mut dst, n);
                for (i, (d, r)) in dst.iter().zip(&x).enumerate() {
                    let back = i32::from(*d) - 128;
                    assert!((back - r).abs() <= 1, "{n}x{n} pixel {i}: {back} for {r}");
                }
            }
        }
    }

    #[test]
    fn a_dc_alone_adds_what_the_whole_transform_adds() {
        for n in [4, 8, 16, 32] {
            for dc in [-4000, -77, 1, 9, 1234] {
                let mut coeffs = vec![0; n * n];
                coeffs[0] = dc;
                let (mut fast, mut full) = (vec![100u8; n * n], vec![100u8; n * n]);
                inverse_add(n, &coeffs, 1, &mut fast, n);
                inverse_add(n, &coeffs, 2, &mut full, n);
                assert_eq!(fast, full, "{n}x{n}, DC {dc}");
            }
        }
    }

    #[test]
    fn the_widest_lanes_transform_as_one_lane_does() {
        for n in [4, 8, 16, 32] {
            for seed in 1..40 {
                // Residuals as large as a pixel's, and smaller.
                let source = residual(n, seed, [255, 40, 3][seed as usize % 3]);
                let (mut scalar, mut vector) = (vec![0; n * n], vec![0; n * n]);
                forward_in::<i32>(n, &source, &mut scalar);
                forward(n, &source, &mut vector);
                assert_eq!(scalar, vector, "forward {n}x{n}, seed {seed}");
                // Coefficients anywhere in 16 bits, which wrap inside the
                // inverse, some rows left empty.
                let mut coeffs = residual(n, seed + 1000, 32767);
                for row in coeffs.chunks_exact_mut(n).skip(seed as usize % n) {
                    row.fill(0);
                }
                let pixels: Vec<u8> = source.iter().map(|&v| (v + 128).clamp(0, 255) as u8).collect();
                let (mut scalar, mut vector) = (pixels.clone(), pixels);
                inverse_add_in::<i32>(n, &coeffs, n * n, &mut scalar, n);
                inverse_add(n, &coeffs, n * n, &mut vector, n);
                assert_eq!(scalar, vector, "inverse {n}x{n}, seed {seed}");
            }
        }
    }
}
