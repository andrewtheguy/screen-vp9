//! A fixed quantizer: one q index for every coefficient of the frame, at
//! libvpx's realtime rounding (`vp9_quantize_fp`). Quantizing is NEON on
//! arm64, held by its tests to the scalar reference beside it.

use super::tables::{AC_QLOOKUP, DC_QLOOKUP, ISCAN_4X4, ISCAN_8X8, ISCAN_16X16, ISCAN_32X32};

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
    /// return the end of block: one past the last nonzero level in the
    /// scan.
    pub(super) fn quantize(&self, n: usize, coeffs: &[i32], levels: &mut [i32]) -> usize {
        let iscan: &[u16] = match n {
            4 => &ISCAN_4X4,
            8 => &ISCAN_8X8,
            16 => &ISCAN_16X16,
            _ => &ISCAN_32X32,
        };
        let terms = self.terms(n);
        #[cfg(target_arch = "aarch64")]
        return neon::quantize(&terms, &coeffs[..n * n], iscan, &mut levels[..n * n]);
        #[cfg(not(target_arch = "aarch64"))]
        return quantize_scalar(&terms, &coeffs[..n * n], iscan, &mut levels[..n * n]);
    }

    /// How a coefficient of an `n`×`n` transform is quantized. At 32×32 a
    /// decoder halves the dequantized level, so a level there counts half
    /// steps, and below a quarter step is nothing.
    fn terms(&self, n: usize) -> Terms {
        if n == 32 {
            Terms { least: self.dequant.map(|d| d >> 2), round: self.round.map(|r| (r + 1) >> 1), quant: self.quant, shift: 15 }
        } else {
            Terms { least: [0; 2], round: self.round, quant: self.quant, shift: 16 }
        }
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
/// `quant`, shifted right `shift`.
struct Terms {
    least: [i32; 2],
    round: [i32; 2],
    quant: [i32; 2],
    shift: u32,
}

/// `Quantizer::quantize` one coefficient at a time, in raster order.
#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
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

    /// `Quantizer::quantize` four coefficients at a time.
    pub(super) fn quantize(terms: &Terms, coeffs: &[i32], iscan: &[u16], levels: &mut [i32]) -> usize {
        // NEON is in every arm64 target's baseline, and every load and store
        // is of four values inside slices of whole fours, `coeffs` and
        // `levels` as long as `iscan`.
        assert!(coeffs.len() == iscan.len() && levels.len() == iscan.len() && iscan.len().is_multiple_of(4));
        unsafe {
            // The DC's terms in the first lane of the first four, the rest
            // the AC's.
            let lanes = |t: [i32; 2], first: bool| if first { vsetq_lane_s32::<0>(t[0], vdupq_n_s32(t[1])) } else { vdupq_n_s32(t[1]) };
            let shift = vdupq_n_s32(-(terms.shift as i32));
            let most = vdupq_n_s32(i32::from(i16::MAX));
            let mut eob = vdupq_n_u32(0);
            for at in (0..iscan.len()).step_by(4) {
                let first = at == 0;
                let c = vld1q_s32(coeffs.as_ptr().add(at));
                let abs = vabsq_s32(c);
                let level = vshlq_s32(vmulq_s32(vminq_s32(vaddq_s32(abs, lanes(terms.round, first)), most), lanes(terms.quant, first)), shift);
                let level = vbicq_s32(level, vreinterpretq_s32_u32(vcltq_s32(abs, lanes(terms.least, first))));
                vst1q_s32(levels.as_mut_ptr().add(at), vbslq_s32(vcltzq_s32(c), vnegq_s32(level), level));
                let ends = vmovl_u16(vld1_u16(iscan.as_ptr().add(at)));
                eob = vmaxq_u32(eob, vandq_u32(vtstq_s32(level, level), ends));
            }
            vmaxvq_u32(eob) as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::tables::SCAN_4X4;

    #[test]
    fn a_coefficient_comes_back_within_its_rounding() {
        let q = Quantizer::new(120);
        let coeffs: Vec<i32> = (0..16).map(|i| (i * 37 - 300) * if i % 3 == 0 { -1 } else { 1 }).collect();
        let mut levels = vec![0; 16];
        let eob = q.quantize(4, &coeffs, &mut levels);
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
                    let eob = q.quantize(n, &coeffs, &mut got);
                    assert_eq!(eob, quantize_scalar(&q.terms(n), &coeffs, iscan, &mut want), "{n}x{n} at {q_index}, spread {spread}");
                    assert_eq!(got, want, "{n}x{n} at {q_index}, spread {spread}");
                }
            }
        }
    }
}
