//! Inter prediction between whole pixels: VP9's eight-tap filter, EIGHTTAP,
//! the frame's one filter, across, down or both, the pass across rounded to
//! pixels before the pass down, as a decoder filters (libvpx's
//! `vpx_convolve8`). NEON on arm64, and held by its tests to the scalar
//! reference beside it, which amd64 runs until it has AVX2 of its own.

use super::tables::SUBPEL_FILTERS;

/// The pixels a filter reads before the one it is centred on; it reads
/// `TAPS - BEFORE - 1` after.
pub(super) const BEFORE: usize = 3;
/// The pixels a filter reads.
pub(super) const TAPS: usize = 8;

/// Fill the `w`×`h` at the start of `out`, rows `out_stride` apart, with the
/// prediction `fx` sixteenths of a pixel right and `fy` down of the `w`×`h`
/// whose corner is `BEFORE` rows and columns into `src`, rows `stride` apart.
/// `src` holds the `BEFORE` rows and columns before the block and the
/// `TAPS - BEFORE - 1` after it; `w` is a multiple of 8, up to 64.
pub(super) fn predict(src: &[u8], stride: usize, (fx, fy): (usize, usize), w: usize, h: usize, out: &mut [u8], out_stride: usize) {
    debug_assert!(w.is_multiple_of(8) && w <= 64 && h <= 64, "{w}×{h}");
    if (fx, fy) == (0, 0) {
        let corner = BEFORE * stride + BEFORE;
        for r in 0..h {
            out[r * out_stride..][..w].copy_from_slice(&src[corner + r * stride..][..w]);
        }
        return;
    }
    #[cfg(target_arch = "aarch64")]
    return neon::predict(src, stride, (fx, fy), (w, h), out, out_stride);
    #[cfg(target_arch = "x86_64")]
    return scalar::predict(src, stride, (fx, fy), (w, h), out, out_stride);
}

/// The pass across of `predict` alone: fill the `w`×`h` at the start of
/// `out` with the pixels `f` sixteenths right of those `BEFORE` columns into
/// `src`, each row's; `w` is a multiple of 8.
pub(super) fn across(src: &[u8], stride: usize, f: usize, (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
    debug_assert!(w.is_multiple_of(8), "{w}×{h}");
    #[cfg(target_arch = "aarch64")]
    return neon::across(src, stride, f, (w, h), out, out_stride);
    #[cfg(target_arch = "x86_64")]
    return scalar::across(src, stride, f, w, h, out, out_stride);
}

/// The pass down of `predict` alone: fill the `w`×`h` at the start of `out`
/// with the pixels `f` sixteenths below those `BEFORE` rows into `src`; `w`
/// is a multiple of 8.
pub(super) fn down(src: &[u8], stride: usize, f: usize, (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
    debug_assert!(w.is_multiple_of(8), "{w}×{h}");
    #[cfg(target_arch = "aarch64")]
    return neon::down(src, stride, f, (w, h), out, out_stride);
    #[cfg(target_arch = "x86_64")]
    return scalar::down(src, stride, f, w, h, out, out_stride);
}

#[cfg(any(test, target_arch = "x86_64"))]
mod scalar {
    use super::{BEFORE, SUBPEL_FILTERS, TAPS};

    pub(super) fn predict(src: &[u8], stride: usize, (fx, fy): (usize, usize), (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
        match (fx, fy) {
            (fx, 0) => across(&src[BEFORE * stride..], stride, fx, w, h, out, out_stride),
            (0, fy) => down(&src[BEFORE..], stride, fy, w, h, out, out_stride),
            (fx, fy) => {
                let mut temp = [0; 64 * (64 + TAPS - 1)];
                across(src, stride, fx, w, h + TAPS - 1, &mut temp, w);
                down(&temp, w, fy, w, h, out, out_stride);
            }
        }
    }

    /// `taps` of eight pixels, rounded to a pixel.
    fn filter(taps: &[i16; TAPS], pixel: impl Fn(usize) -> u8) -> u8 {
        let sum: i32 = (0..TAPS).map(|k| i32::from(taps[k]) * i32::from(pixel(k))).sum();
        ((sum + 64) >> 7).clamp(0, 255) as u8
    }

    pub(super) fn across(src: &[u8], stride: usize, f: usize, w: usize, h: usize, out: &mut [u8], out_stride: usize) {
        for r in 0..h {
            for c in 0..w {
                out[r * out_stride + c] = filter(&SUBPEL_FILTERS[f], |k| src[r * stride + c + k]);
            }
        }
    }

    pub(super) fn down(src: &[u8], stride: usize, f: usize, w: usize, h: usize, out: &mut [u8], out_stride: usize) {
        for r in 0..h {
            for c in 0..w {
                out[r * out_stride + c] = filter(&SUBPEL_FILTERS[f], |k| src[(r + k) * stride + c]);
            }
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod neon {
    use std::arch::aarch64::*;

    use super::{BEFORE, SUBPEL_FILTERS};

    // NEON is in every arm64 target's baseline; each `unsafe` here is for
    // that and for loads and stores inside the slices bounds-checked just
    // before them.

    /// Eight pixels of `a` from `at`, widened.
    #[inline(always)]
    fn eight(a: &[u8], at: usize) -> int16x8_t {
        let a = &a[at..][..8];
        unsafe { vreinterpretq_s16_u16(vmovl_u8(vld1_u8(a.as_ptr()))) }
    }

    /// The taps `f` of eight sets of eight pixels, `s[k]` the `k`th of each,
    /// rounded to pixels. As libvpx's NEON sums them, in 16 bits with the
    /// two centre taps added last and saturating: the other six cannot take
    /// the sum out of 16 bits, and a sum the centre taps saturate is a pixel
    /// of 255 either way.
    #[inline(always)]
    fn taps(s: [int16x8_t; 8], f: int16x8_t) -> uint8x8_t {
        unsafe {
            let mut sum = vmulq_laneq_s16::<0>(s[0], f);
            sum = vmlaq_laneq_s16::<1>(sum, s[1], f);
            sum = vmlaq_laneq_s16::<2>(sum, s[2], f);
            sum = vmlaq_laneq_s16::<5>(sum, s[5], f);
            sum = vmlaq_laneq_s16::<6>(sum, s[6], f);
            sum = vmlaq_laneq_s16::<7>(sum, s[7], f);
            sum = vqaddq_s16(sum, vmulq_laneq_s16::<3>(s[3], f));
            sum = vqaddq_s16(sum, vmulq_laneq_s16::<4>(s[4], f));
            vqrshrun_n_s16::<7>(sum)
        }
    }

    /// The taps `f` across the eight pixels from `at` and the seven after.
    #[inline(always)]
    fn across8(src: &[u8], at: usize, f: int16x8_t) -> uint8x8_t {
        unsafe {
            // The last seven read from `at + 7`, so as not to read past them.
            let a = eight(src, at);
            let b = vreinterpretq_s16_u16(vmovl_u8(vext_u8::<1>(vld1_u8(src[at + 7..][..8].as_ptr()), vdup_n_u8(0))));
            taps([a, vextq_s16::<1>(a, b), vextq_s16::<2>(a, b), vextq_s16::<3>(a, b), vextq_s16::<4>(a, b), vextq_s16::<5>(a, b), vextq_s16::<6>(a, b), vextq_s16::<7>(a, b)], f)
        }
    }

    /// Widened.
    #[inline(always)]
    fn wide(v: uint8x8_t) -> int16x8_t {
        unsafe { vreinterpretq_s16_u16(vmovl_u8(v)) }
    }

    /// Eight pixels into `out` from `at`.
    #[inline(always)]
    fn store(out: &mut [u8], at: usize, v: uint8x8_t) {
        let out = &mut out[at..][..8];
        unsafe { vst1_u8(out.as_mut_ptr(), v) }
    }

    pub(super) fn across(src: &[u8], stride: usize, f: usize, (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
        let taps = unsafe { vld1q_s16(SUBPEL_FILTERS[f].as_ptr()) };
        for r in 0..h {
            for c in (0..w).step_by(8) {
                store(out, r * out_stride + c, across8(src, r * stride + c, taps));
            }
        }
    }

    pub(super) fn down(src: &[u8], stride: usize, f: usize, (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
        let f = unsafe { vld1q_s16(SUBPEL_FILTERS[f].as_ptr()) };
        for c in (0..w).step_by(8) {
            let row = |r: usize| eight(src, r * stride + c);
            let (mut s0, mut s1, mut s2, mut s3, mut s4, mut s5, mut s6) = (row(0), row(1), row(2), row(3), row(4), row(5), row(6));
            for r in 0..h {
                let s7 = row(r + 7);
                store(out, r * out_stride + c, taps([s0, s1, s2, s3, s4, s5, s6, s7], f));
                (s0, s1, s2, s3, s4, s5, s6) = (s1, s2, s3, s4, s5, s6, s7);
            }
        }
    }

    pub(super) fn predict(src: &[u8], stride: usize, (fx, fy): (usize, usize), (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
        let (x_taps, y_taps) = unsafe { (vld1q_s16(SUBPEL_FILTERS[fx].as_ptr()), vld1q_s16(SUBPEL_FILTERS[fy].as_ptr())) };
        for c in (0..w).step_by(8) {
            if fy == 0 {
                for r in 0..h {
                    store(out, r * out_stride + c, across8(src, (r + BEFORE) * stride + c, x_taps));
                }
                continue;
            }
            // Row `r` of the eight the next pixel down is taken from, filtered
            // across first where there is a fraction across, and rounded to
            // pixels as a decoder rounds them.
            let row = |r: usize| if fx == 0 { eight(src, r * stride + c + BEFORE) } else { wide(across8(src, r * stride + c, x_taps)) };
            let (mut s0, mut s1, mut s2, mut s3, mut s4, mut s5, mut s6) = (row(0), row(1), row(2), row(3), row(4), row(5), row(6));
            for r in 0..h {
                let s7 = row(r + 7);
                store(out, r * out_stride + c, taps([s0, s1, s2, s3, s4, s5, s6, s7], y_taps));
                (s0, s1, s2, s3, s4, s5, s6) = (s1, s2, s3, s4, s5, s6, s7);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(len: usize, seed: u32) -> Vec<u8> {
        let mut state = seed;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect()
    }

    /// `predict` as the scalar reference makes it.
    fn reference(src: &[u8], stride: usize, (fx, fy): (usize, usize), w: usize, h: usize) -> Vec<u8> {
        let mut out = vec![0; w * h];
        if (fx, fy) == (0, 0) {
            for r in 0..h {
                out[r * w..][..w].copy_from_slice(&src[(BEFORE + r) * stride + BEFORE..][..w]);
            }
        } else {
            scalar::predict(src, stride, (fx, fy), (w, h), &mut out, w);
        }
        out
    }

    #[test]
    fn the_kernels_are_their_scalar_reference() {
        for seed in 1..12 {
            for (w, h) in [(8, 8), (16, 8), (16, 16), (32, 32), (64, 64)] {
                // Exactly the pixels the filter reads, so a read past them
                // is out of bounds; extremes as well as noise, which is
                // where a 16-bit sum can saturate.
                let stride = w + TAPS - 1;
                let src = match seed % 4 {
                    0 => (0..stride * (h + TAPS - 1)).map(|i| if (i / 2) % 2 == 0 { 255 } else { 0 }).collect(),
                    1 => vec![255; stride * (h + TAPS - 1)],
                    _ => noise(stride * (h + TAPS - 1), seed),
                };
                for fx in 0..16 {
                    for fy in 0..16 {
                        let mut out = vec![0; w * h];
                        predict(&src, stride, (fx, fy), w, h, &mut out, w);
                        assert_eq!(out, reference(&src, stride, (fx, fy), w, h), "{w}×{h} at ({fx}, {fy}), seed {seed}");
                    }
                }
            }
        }
    }

    #[test]
    fn each_pass_is_its_scalar_reference_and_both_are_predict() {
        for seed in 1..6 {
            for (w, h) in [(8, 8), (16, 9), (24, 17), (64, 64)] {
                let stride = w + TAPS - 1;
                let src = noise(stride * (h + TAPS - 1), seed);
                for f in 0..16 {
                    let (mut got, mut want) = (vec![0; w * h], vec![0; w * h]);
                    across(&src, stride, f, (w, h), &mut got, w);
                    scalar::across(&src, stride, f, w, h, &mut want, w);
                    assert_eq!(got, want, "across {w}×{h} at {f}, seed {seed}");
                    down(&src, stride, f, (w, h), &mut got, w);
                    scalar::down(&src, stride, f, w, h, &mut want, w);
                    assert_eq!(got, want, "down {w}×{h} at {f}, seed {seed}");
                }
                for (fx, fy) in [(4, 12), (8, 8), (12, 2)] {
                    let mut temp = vec![0; w * (h + TAPS - 1)];
                    across(&src, stride, fx, (w, h + TAPS - 1), &mut temp, w);
                    let mut both = vec![0; w * h];
                    down(&temp, w, fy, (w, h), &mut both, w);
                    assert_eq!(both, reference(&src, stride, (fx, fy), w, h), "{w}×{h} at ({fx}, {fy}), seed {seed}");
                }
            }
        }
    }

    #[test]
    fn the_scalar_reference_is_vp9s_filter() {
        // A half pixel across a step from 0 to 160 under the taps -1, 6,
        // -19, 78, 78, -19, 6, -1: halfway is 80, and either side the
        // filter rings, rounded and held to pixels.
        let stride = 8 + TAPS - 1;
        let src: Vec<u8> = (0..stride * TAPS).map(|i| if i % stride >= BEFORE + 4 { 160 } else { 0 }).collect();
        let mut out = [0; 8];
        scalar::across(&src[BEFORE * stride..], stride, 8, 8, 1, &mut out, 8);
        // From the left: -160, 5 × 160, -14 × 160, 64 × 160, 142 × 160 ...
        assert_eq!(out, [0, 6, 0, 80, 178, 154, 161, 160]);
    }
}
