//! Two-dimensional DCTs: forward, to code a residual, and inverse, to
//! reconstruct it as a decoder does.
//!
//! A forward transform is the encoder's own business; these follow libvpx's
//! `vpx_fdct*_c`, whose scale the inverse expects. The inverse is libvpx's
//! `vpx_idct*_add_c` and exact: a decoder adds its result to the prediction,
//! and the encoder predicts later blocks from the same pixels.

use super::dct::{fdct32, idct4, idct8, idct16, idct32};

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
fn fdct4(i: [i32; 4]) -> [i32; 4] {
    let s = [i[0] + i[3], i[1] + i[2], i[1] - i[2], i[0] - i[3]];
    [
        round14((s[0] + s[1]) * COSPI_16_64),
        round14(s[2] * COSPI_24_64 + s[3] * COSPI_8_64),
        round14((s[0] - s[1]) * COSPI_16_64),
        round14(-s[2] * COSPI_8_64 + s[3] * COSPI_24_64),
    ]
}

/// The 8-point forward DCT inside `vpx_fdct8x8_c`.
fn fdct8(i: [i32; 8]) -> [i32; 8] {
    let s = [i[0] + i[7], i[1] + i[6], i[2] + i[5], i[3] + i[4], i[3] - i[4], i[2] - i[5], i[1] - i[6], i[0] - i[7]];
    let (x0, x1, x2, x3) = (s[0] + s[3], s[1] + s[2], s[1] - s[2], s[0] - s[3]);
    let even = [round14((x0 + x1) * COSPI_16_64), round14(x2 * COSPI_24_64 + x3 * COSPI_8_64), round14((x0 - x1) * COSPI_16_64), round14(-x2 * COSPI_8_64 + x3 * COSPI_24_64)];
    let t2 = round14((s[6] - s[5]) * COSPI_16_64);
    let t3 = round14((s[6] + s[5]) * COSPI_16_64);
    let (x0, x1, x2, x3) = (s[4] + t2, s[4] - t2, s[7] - t3, s[7] + t3);
    [
        even[0],
        round14(x0 * COSPI_28_64 + x3 * COSPI_4_64),
        even[1],
        round14(x2 * COSPI_12_64 + x1 * -COSPI_20_64),
        even[2],
        round14(x1 * COSPI_12_64 + x2 * COSPI_20_64),
        even[3],
        round14(x3 * COSPI_28_64 + x0 * -COSPI_4_64),
    ]
}

/// The 16-point forward DCT inside `vpx_fdct16x16_c`: the even half is the
/// 8-point one of the folded input.
fn fdct16(i: [i32; 16]) -> [i32; 16] {
    let even = fdct8(std::array::from_fn(|k| i[k] + i[15 - k]));
    let step1: [i32; 8] = std::array::from_fn(|k| i[7 - k] - i[8 + k]);
    let mut step2 = [0; 8];
    step2[2] = round14((step1[5] - step1[2]) * COSPI_16_64);
    step2[3] = round14((step1[4] - step1[3]) * COSPI_16_64);
    step2[4] = round14((step1[4] + step1[3]) * COSPI_16_64);
    step2[5] = round14((step1[5] + step1[2]) * COSPI_16_64);
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
    step2[1] = round14(step3[1] * -COSPI_8_64 + step3[6] * COSPI_24_64);
    step2[2] = round14(step3[2] * COSPI_24_64 + step3[5] * COSPI_8_64);
    step2[5] = round14(step3[2] * COSPI_8_64 - step3[5] * COSPI_24_64);
    step2[6] = round14(step3[1] * COSPI_24_64 + step3[6] * COSPI_8_64);
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
    let mut out = [0; 16];
    for k in 0..8 {
        out[2 * k] = even[k];
    }
    out[1] = round14(s[0] * COSPI_30_64 + s[7] * COSPI_2_64);
    out[9] = round14(s[1] * COSPI_14_64 + s[6] * COSPI_18_64);
    out[5] = round14(s[2] * COSPI_22_64 + s[5] * COSPI_10_64);
    out[13] = round14(s[3] * COSPI_6_64 + s[4] * COSPI_26_64);
    out[3] = round14(s[3] * -COSPI_26_64 + s[4] * COSPI_6_64);
    out[11] = round14(s[2] * -COSPI_10_64 + s[5] * COSPI_22_64);
    out[7] = round14(s[1] * -COSPI_18_64 + s[6] * COSPI_14_64);
    out[15] = round14(s[0] * -COSPI_2_64 + s[7] * COSPI_30_64);
    out
}

/// Column `c` of an `N`×`N` block.
fn column<const N: usize>(block: &[i32], c: usize) -> [i32; N] {
    std::array::from_fn(|r| block[r * N + c])
}

/// The forward DCT of an `n`×`n` residual, `n` 4 to 32, both raster order:
/// `out[v * n + h]` holds vertical frequency `v` and horizontal `h`.
pub(super) fn forward(n: usize, residual: &[i32], out: &mut [i32]) {
    match n {
        4 => {
            // Columns, scaled up and the first one's DC nudged off zero, then
            // rows; each pass leaves its result transposed.
            let mut mid = [0; 16];
            for c in 0..4 {
                let mut x = column::<4>(residual, c).map(|v| v * 16);
                if c == 0 && x[0] != 0 {
                    x[0] += 1;
                }
                mid[c * 4..][..4].copy_from_slice(&fdct4(x));
            }
            for r in 0..4 {
                for (o, v) in out[r * 4..][..4].iter_mut().zip(fdct4(column::<4>(&mid, r))) {
                    *o = (v + 1) >> 2;
                }
            }
        }
        8 => {
            let mut mid = [0; 64];
            for c in 0..8 {
                mid[c * 8..][..8].copy_from_slice(&fdct8(column::<8>(residual, c).map(|v| v * 4)));
            }
            for r in 0..8 {
                for (o, v) in out[r * 8..][..8].iter_mut().zip(fdct8(column::<8>(&mid, r))) {
                    *o = v / 2;
                }
            }
        }
        16 => {
            let mut mid = [0; 256];
            for c in 0..16 {
                mid[c * 16..][..16].copy_from_slice(&fdct16(column::<16>(residual, c).map(|v| v * 4)));
            }
            for r in 0..16 {
                out[r * 16..][..16].copy_from_slice(&fdct16(column::<16>(&mid, r).map(|v| (v + 1) >> 2)));
            }
        }
        32 => {
            // Columns stay columns here, then rows.
            let mut mid = [0; 1024];
            let mut t = [0; 32];
            for c in 0..32 {
                fdct32(&column::<32>(residual, c).map(|v| v * 4), &mut t);
                for (r, v) in t.iter().enumerate() {
                    mid[r * 32 + c] = (v + 1 + i32::from(*v > 0)) >> 2;
                }
            }
            for r in 0..32 {
                fdct32(mid[r * 32..][..32].try_into().expect("a row"), &mut t);
                for (o, v) in out[r * 32..][..32].iter_mut().zip(t) {
                    *o = (v + 1 + i32::from(v < 0)) >> 2;
                }
            }
        }
        _ => unreachable!("a {n}-point transform"),
    }
}

/// Add the inverse DCT of `n`×`n` dequantized `coeffs` to the `n`×`n`
/// pixels at the start of `dst`, rows `stride` apart. `eob` is how many
/// coefficients the scan reached; with one, only the DC is there, which
/// libvpx's decoder takes its own path for, to the same result.
pub(super) fn inverse_add(n: usize, coeffs: &[i32], eob: usize, dst: &mut [u8], stride: usize) {
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
        4 => inverse_2d::<4>(idct4, shift, coeffs, dst, stride),
        8 => inverse_2d::<8>(idct8, shift, coeffs, dst, stride),
        16 => inverse_2d::<16>(idct16, shift, coeffs, dst, stride),
        32 => inverse_2d::<32>(idct32, shift, coeffs, dst, stride),
        _ => unreachable!("a {n}-point transform"),
    }
}

/// Rows, then columns, then the rounding shift and the add.
fn inverse_2d<const N: usize>(idct: fn(&[i32; N], &mut [i32; N]), shift: u32, coeffs: &[i32], dst: &mut [u8], stride: usize) {
    let mut mid = vec![0; N * N];
    for r in 0..N {
        let row: &[i32; N] = coeffs[r * N..][..N].try_into().expect("a row");
        // A row of zeros transforms to zeros.
        if row.iter().any(|&c| c != 0) {
            idct(row, (&mut mid[r * N..][..N]).try_into().expect("a row"));
        }
    }
    let mut out = [0; N];
    for c in 0..N {
        idct(&column::<N>(&mid, c), &mut out);
        for (r, v) in out.iter().enumerate() {
            let p = &mut dst[r * stride + c];
            *p = (i32::from(*p) + ((v + (1 << (shift - 1))) >> shift)).clamp(0, 255) as u8;
        }
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
}
