//! A fixed quantizer: one q index for every coefficient of the frame, at
//! libvpx's realtime rounding (`vp9_quantize_fp`).

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

    /// Quantize the `n`×`n` `coeffs` into `levels`, both raster order, and
    /// return the end of block: one past the last nonzero level in `scan`.
    pub(super) fn quantize(&self, n: usize, coeffs: &[i32], scan: &[u16], levels: &mut [i32]) -> usize {
        let mut eob = 0;
        for (i, &rc) in scan.iter().enumerate() {
            let rc = usize::from(rc);
            let k = usize::from(rc != 0);
            let coeff = coeffs[rc];
            let abs = coeff.abs();
            // At 32×32 a decoder halves the dequantized level, so a level
            // there counts half steps, and below a quarter step is nothing.
            let level = if n == 32 {
                if abs < self.dequant[k] >> 2 {
                    0
                } else {
                    ((abs + ((self.round[k] + 1) >> 1)).min(i32::from(i16::MAX)) * self.quant[k]) >> 15
                }
            } else {
                ((abs + self.round[k]).min(i32::from(i16::MAX)) * self.quant[k]) >> 16
            };
            levels[rc] = if coeff < 0 { -level } else { level };
            if level != 0 {
                eob = i + 1;
            }
        }
        eob
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::tables::SCAN_4X4;

    #[test]
    fn a_coefficient_comes_back_within_its_rounding() {
        let q = Quantizer::new(120);
        let coeffs: Vec<i32> = (0..16).map(|i| (i * 37 - 300) * if i % 3 == 0 { -1 } else { 1 }).collect();
        let mut levels = vec![0; 16];
        let eob = q.quantize(4, &coeffs, &SCAN_4X4, &mut levels);
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
}
