//! A fixed quantizer: one q index for every coefficient of the frame, at
//! libvpx's realtime rounding (`vp9_quantize_fp`). Quantizing is NEON on
//! arm64 and AVX2 on amd64, held by its tests to the scalar reference
//! beside it.
use super::tables::{AC_QLOOKUP, DC_QLOOKUP};

/// The steps and roundings at one q index, `[DC, AC]` each.
pub(super) struct Quantizer {
    /// The step a decoder multiplies a level by.
    dequant: [i32; 2],
    round: [i32; 2],
    /// 65536 over the step.
    quant: [i32; 2],
}

impl Quantizer {
    pub(super) fn new(q_index: u8) -> Self {
        let dequant = [i32::from(DC_QLOOKUP[usize::from(q_index)]), i32::from(AC_QLOOKUP[usize::from(q_index)])];
        // libvpx rounds the DC up at 48/128 of a step and the rest at 42/128.
        let round = [(48 * dequant[0]) >> 7, (42 * dequant[1]) >> 7];
        Self { dequant, round, quant: dequant.map(|d| (1 << 16) / d) }
    }

    /// The step of every coefficient but the DC.
    pub(super) fn ac_step(&self) -> i32 {
        self.dequant[1]
    }

    /// Quantize the `n`×`n` `coeffs` into `levels`, both raster order, and
    /// return the end of block: one past the last nonzero level in the scan
    /// whose inverse is `iscan`.
    pub(super) fn quantize(&self, n: usize, iscan: &[u16], coeffs: &[i32], levels: &mut [i32]) -> usize {
        self.quantize_in::<false>(n, iscan, coeffs, levels).0
    }

    /// `quantize`, and the squared error of the levels as a decoder
    /// dequantizes them against `coeffs`, in the scale of a transform below
    /// 32×32: 64 times the squared error in pixels. A 32×32 transform's
    /// coefficients are half that scale.
    pub(super) fn quantize_with_error(&self, n: usize, iscan: &[u16], coeffs: &[i32], levels: &mut [i32]) -> (usize, i64) {
        let (eob, error) = self.quantize_in::<true>(n, iscan, coeffs, levels);
        (eob, error << (2 * u32::from(n == 32)))
    }

    /// The squared error of coding nothing of the `n`×`n` `coeffs`, on the
    /// scale `quantize_with_error` gives an error.
    pub(super) fn energy(n: usize, coeffs: &[i32]) -> i64 {
        let sum: i64 = coeffs[..n * n].iter().map(|&c| i64::from(c) * i64::from(c)).sum();
        sum << (2 * u32::from(n == 32))
    }

    /// `quantize`, with the squared error on the coefficients' own scale
    /// where `ERROR`, otherwise 0.
    fn quantize_in<const ERROR: bool>(&self, n: usize, iscan: &[u16], coeffs: &[i32], levels: &mut [i32]) -> (usize, i64) {
        let terms = self.terms(n);
        #[cfg(target_arch = "aarch64")]
        return neon::quantize::<ERROR>(&terms, &coeffs[..n * n], iscan, &mut levels[..n * n]);
        #[cfg(target_arch = "x86_64")]
        return avx2::quantize::<ERROR>(&terms, &coeffs[..n * n], iscan, &mut levels[..n * n]);
    }

    /// How a coefficient of an `n`×`n` transform is quantized. At 32×32 a
    /// decoder halves the dequantized level, so a level there counts half
    /// steps, and below a quarter step is nothing.
    fn terms(&self, n: usize) -> Terms {
        let dequant = self.dequant;
        if n == 32 {
            Terms { least: dequant.map(|d| d >> 2), round: self.round.map(|r| (r + 1) >> 1), quant: self.quant, shift: 15, dequant, half: 1 }
        } else {
            Terms { least: [0; 2], round: self.round, quant: self.quant, shift: 16, dequant, half: 0 }
        }
    }

    /// The error `quantize_with_error` gives, one coefficient at a time
    /// from the levels it left, the reference its kernels are held to.
    #[cfg(test)]
    fn error(&self, n: usize, coeffs: &[i32], levels: &[i32]) -> i64 {
        let shift = u32::from(n == 32);
        // A coefficient of 8-bit pixels is within 32640 of zero, so an error
        // squares within 32 bits.
        let error = |c: i32, level: i32, step: i32| {
            let v = (level.abs() * step) >> shift;
            let e = c - if level < 0 { -v } else { v };
            i64::from(e * e)
        };
        let dc = error(coeffs[0], levels[0], self.dequant[0]);
        let ac: i64 = coeffs[1..n * n].iter().zip(&levels[1..n * n]).map(|(&c, &l)| error(c, l, self.dequant[1])).sum();
        (dc + ac) << (2 * shift)
    }

    /// What a decoder makes of `levels` (libvpx's `decode_coefs`): each
    /// times its step, halved at 32×32, as the 16 bits a decoder keeps it in.
    pub(super) fn dequantize(&self, n: usize, levels: &[i32], scan: &[u16], eob: usize, coeffs: &mut [i32]) {
        coeffs.fill(0);
        let shift = u32::from(n == 32);
        for &rc in &scan[..eob] {
            let rc = usize::from(rc);
            let level = levels[rc];
            let v = (level.abs() * self.dequant[usize::from(rc != 0)]) >> shift;
            coeffs[rc] = i32::from((if level < 0 { -v } else { v }) as i16);
        }
    }
}

/// How each coefficient is quantized, `[DC, AC]` each: below `least` it is
/// nothing; otherwise its magnitude plus `round`, held to 16 bits, times
/// `quant`, shifted right `shift`. A decoder takes a level back as its
/// magnitude times `dequant`, shifted right `half`.
struct Terms {
    least: [i32; 2],
    round: [i32; 2],
    quant: [i32; 2],
    shift: u32,
    dequant: [i32; 2],
    half: u32,
}

/// `Quantizer::quantize` one coefficient at a time, in raster order.
#[cfg(test)]
fn quantize_scalar(terms: &Terms, coeffs: &[i32], iscan: &[u16], levels: &mut [i32]) -> usize {
    let mut eob = 0;
    for (rc, (&coeff, out)) in coeffs.iter().zip(levels).enumerate() {
        let k = usize::from(rc != 0);
        let abs = coeff.abs();
        let level = if abs < terms.least[k] { 0 } else { ((abs + terms.round[k]).min(i32::from(i16::MAX)) * terms.quant[k]) >> terms.shift };
        *out = if coeff < 0 { -level } else { level };
        if level != 0 {
            eob = eob.max(usize::from(iscan[rc]));
        }
    }
    eob
}

#[cfg(target_arch = "aarch64")]
mod neon {
    use std::arch::aarch64::*;

    use super::Terms;

    /// `Quantizer::quantize_in` four coefficients at a time.
    pub(super) fn quantize<const ERROR: bool>(terms: &Terms, coeffs: &[i32], iscan: &[u16], levels: &mut [i32]) -> (usize, i64) {
        // The crate is built for NEON, and every load and store
        // is of four values inside slices of whole fours, `coeffs` and
        // `levels` as long as `iscan`.
        assert!(coeffs.len() == iscan.len() && levels.len() == iscan.len() && iscan.len().is_multiple_of(4));
        unsafe {
            // The DC's terms in the first lane of the first four, the rest
            // the AC's.
            let lanes = |t: [i32; 2], first: bool| if first { vsetq_lane_s32::<0>(t[0], vdupq_n_s32(t[1])) } else { vdupq_n_s32(t[1]) };
            let shift = vdupq_n_s32(-(terms.shift as i32));
            let half = vdupq_n_s32(-(terms.half as i32));
            let most = vdupq_n_s32(i32::from(i16::MAX));
            let (mut eob, mut error) = (vdupq_n_u32(0), vdupq_n_s64(0));
            for at in (0..iscan.len()).step_by(4) {
                let first = at == 0;
                let c = vld1q_s32(coeffs.as_ptr().add(at));
                let abs = vabsq_s32(c);
                let level = vshlq_s32(vmulq_s32(vminq_s32(vaddq_s32(abs, lanes(terms.round, first)), most), lanes(terms.quant, first)), shift);
                let level = vbicq_s32(level, vreinterpretq_s32_u32(vcltq_s32(abs, lanes(terms.least, first))));
                vst1q_s32(levels.as_mut_ptr().add(at), vbslq_s32(vcltzq_s32(c), vnegq_s32(level), level));
                let ends = vmovl_u16(vld1_u16(iscan.as_ptr().add(at)));
                eob = vmaxq_u32(eob, vandq_u32(vtstq_s32(level, level), ends));
                if ERROR {
                    // A level has its coefficient's sign, so the error is
                    // in the magnitudes; it squares within 32 bits and is
                    // summed in 64.
                    let d = vsubq_s32(abs, vshlq_s32(vmulq_s32(level, lanes(terms.dequant, first)), half));
                    error = vmlal_s32(error, vget_low_s32(d), vget_low_s32(d));
                    error = vmlal_high_s32(error, d, d);
                }
            }
            (vmaxvq_u32(eob) as usize, vaddvq_s64(error))
        }
    }
}

#[cfg(target_arch = "x86_64")]
mod avx2 {
    use std::arch::x86_64::*;

    use super::Terms;

    /// `Quantizer::quantize_in` eight coefficients at a time.
    pub(super) fn quantize<const ERROR: bool>(terms: &Terms, coeffs: &[i32], iscan: &[u16], levels: &mut [i32]) -> (usize, i64) {
        // The crate is built for AVX2, but its intrinsics are still `unsafe`
        // to call from a function not marked for it; and every load and store
        // is of eight values inside slices of whole eights, `coeffs` and
        // `levels` as long as `iscan`.
        assert!(coeffs.len() == iscan.len() && levels.len() == iscan.len() && iscan.len().is_multiple_of(8));
        unsafe {
            // The DC's terms in the first lane of the first eight, the rest the
            // AC's.
            let lanes = |t: [i32; 2], first: bool| if first { _mm256_blend_epi32::<1>(_mm256_set1_epi32(t[1]), _mm256_set1_epi32(t[0])) } else { _mm256_set1_epi32(t[1]) };
            let shift = _mm_cvtsi32_si128(terms.shift as i32);
            let half = _mm_cvtsi32_si128(terms.half as i32);
            let most = _mm256_set1_epi32(i32::from(i16::MAX));
            let (mut eob, mut error) = (_mm256_setzero_si256(), _mm256_setzero_si256());
            for at in (0..iscan.len()).step_by(8) {
                let first = at == 0;
                let c = _mm256_loadu_si256(coeffs.as_ptr().add(at).cast());
                let abs = _mm256_abs_epi32(c);
                let level = _mm256_sra_epi32(_mm256_mullo_epi32(_mm256_min_epi32(_mm256_add_epi32(abs, lanes(terms.round, first)), most), lanes(terms.quant, first)), shift);
                let level = _mm256_andnot_si256(_mm256_cmpgt_epi32(lanes(terms.least, first), abs), level);
                // The sign put back: negated where `c` is negative.
                let sign = _mm256_srai_epi32::<31>(c);
                _mm256_storeu_si256(levels.as_mut_ptr().add(at).cast(), _mm256_sub_epi32(_mm256_xor_si256(level, sign), sign));
                let ends = _mm256_cvtepu16_epi32(_mm_loadu_si128(iscan.as_ptr().add(at).cast()));
                eob = _mm256_max_epu32(eob, _mm256_andnot_si256(_mm256_cmpeq_epi32(level, _mm256_setzero_si256()), ends));
                if ERROR {
                    // A level has its coefficient's sign, so the error is
                    // in the magnitudes; it squares within 32 bits and is
                    // summed in 64, the even lanes and then the odd.
                    let d = _mm256_sub_epi32(abs, _mm256_srl_epi32(_mm256_mullo_epi32(level, lanes(terms.dequant, first)), half));
                    let odd = _mm256_srli_epi64::<32>(d);
                    error = _mm256_add_epi64(error, _mm256_add_epi64(_mm256_mul_epi32(d, d), _mm256_mul_epi32(odd, odd)));
                }
            }
            let error = _mm_add_epi64(_mm256_castsi256_si128(error), _mm256_extracti128_si256::<1>(error));
            let error = _mm_cvtsi128_si64(_mm_add_epi64(error, _mm_unpackhi_epi64(error, error)));
            let eob = _mm_max_epu32(_mm256_castsi256_si128(eob), _mm256_extracti128_si256::<1>(eob));
            let eob = _mm_max_epu32(eob, _mm_shuffle_epi32::<0b01_00_11_10>(eob));
            (_mm_cvtsi128_si32(_mm_max_epu32(eob, _mm_shuffle_epi32::<0b10_11_00_01>(eob))) as usize, error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::tables::{ISCAN_4X4, ISCAN_8X8, ISCAN_16X16, ISCAN_32X32, SCAN_4X4};

    #[test]
    fn the_error_is_what_dequantizing_leaves() {
        let q = Quantizer::new(60);
        for n in [4, 8, 16, 32] {
            let coeffs: Vec<i32> = (0..n * n).map(|i| ((i * 7919) % 2001) as i32 - 1000).collect();
            let mut levels = vec![0; n * n];
            let iscan = match n {
                4 => &ISCAN_4X4[..],
                8 => &ISCAN_8X8[..],
                16 => &ISCAN_16X16[..],
                _ => &ISCAN_32X32[..],
            };
            q.quantize(n, iscan, &coeffs, &mut levels);
            // The scan is the inverse scan's inverse: every level is in it.
            let mut scan = vec![0u16; n * n];
            for (rc, &at) in iscan.iter().enumerate() {
                scan[usize::from(at) - 1] = rc as u16;
            }
            let mut back = vec![0; n * n];
            q.dequantize(n, &levels, &scan, n * n, &mut back);
            let squared: i64 = coeffs.iter().zip(&back).map(|(&c, &b)| i64::from(c - b).pow(2)).sum();
            assert_eq!(q.error(n, &coeffs, &levels), if n == 32 { squared * 4 } else { squared }, "{n}x{n}");
            assert_eq!(q.quantize_with_error(n, iscan, &coeffs, &mut levels), (q.quantize(n, iscan, &coeffs, &mut levels), q.error(n, &coeffs, &levels)), "{n}x{n}");
        }
    }

    #[test]
    fn a_coefficient_comes_back_within_its_rounding() {
        let q = Quantizer::new(120);
        let coeffs: Vec<i32> = (0..16).map(|i| (i * 37 - 300) * if i % 3 == 0 { -1 } else { 1 }).collect();
        let mut levels = vec![0; 16];
        let eob = q.quantize(4, &ISCAN_4X4, &coeffs, &mut levels);
        let mut back = vec![0; 16];
        q.dequantize(4, &levels, &SCAN_4X4, eob, &mut back);
        for (rc, (c, b)) in coeffs.iter().zip(&back).enumerate() {
            // Within the rounding, give or take the one level the step's
            // reciprocal can lose.
            let k = usize::from(rc != 0);
            assert!((c - b).abs() <= q.dequant[k] + q.round[k], "coefficient {rc}: {c} came back {b}");
        }
        // The end of block is one past the last level in scan order.
        let last = SCAN_4X4.iter().rposition(|&rc| levels[usize::from(rc)] != 0).map_or(0, |i| i + 1);
        assert_eq!(eob, last);
    }

    #[test]
    fn quantizing_is_its_scalar_reference() {
        let mut state = 0x2545_f491_u32;
        for q_index in [0, 1, 40, 120, 255] {
            let q = Quantizer::new(q_index);
            for (n, iscan) in [(4, &ISCAN_4X4[..]), (8, &ISCAN_8X8), (16, &ISCAN_16X16), (32, &ISCAN_32X32)] {
                for spread in [4, 64, 1024, 65536] {
                    // Mostly small coefficients, a few up to 16 bits.
                    let coeffs: Vec<i32> = (0..n * n)
                        .map(|_| {
                            state ^= state << 13;
                            state ^= state >> 17;
                            state ^= state << 5;
                            (state % spread) as i32 - spread as i32 / 2
                        })
                        .collect();
                    let (mut got, mut want) = (vec![0; n * n], vec![0; n * n]);
                    let (eob, error) = q.quantize_with_error(n, iscan, &coeffs, &mut got);
                    assert_eq!(eob, quantize_scalar(&q.terms(n), &coeffs, iscan, &mut want), "{n}x{n} at {q_index}, spread {spread}");
                    assert_eq!(got, want, "{n}x{n} at {q_index}, spread {spread}");
                    assert_eq!(error, q.error(n, &coeffs, &want), "{n}x{n} at {q_index}, spread {spread}");
                    assert_eq!(q.quantize(n, iscan, &coeffs, &mut got), eob, "{n}x{n} at {q_index}, spread {spread}");
                }
            }
        }
    }
}
