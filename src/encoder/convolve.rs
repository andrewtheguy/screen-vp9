//! Inter prediction between whole pixels: VP9's smooth eight-tap filter,
//! EIGHTTAP_SMOOTH, the frame's one filter, across, down or both, the pass across rounded to
//! pixels before the pass down, as a decoder filters (libvpx's
//! `vpx_convolve8`). NEON on arm64 and AVX2 on amd64, and held by its tests
//! to the scalar reference beside it.

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
    return avx2::predict(src, stride, (fx, fy), (w, h), out, out_stride);
}

/// The pass across of `predict` alone: fill the `w`×`h` at the start of
/// `out` with the pixels `f` sixteenths right of those `BEFORE` columns into
/// `src`, each row's; `f` is 1 to 15 and `w` a multiple of 8.
pub(super) fn across(src: &[u8], stride: usize, f: usize, (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
    debug_assert!((1..16).contains(&f) && w.is_multiple_of(8), "{w}×{h} at {f}");
    #[cfg(target_arch = "aarch64")]
    return neon::across(src, stride, f, (w, h), out, out_stride);
    #[cfg(target_arch = "x86_64")]
    return avx2::across(src, stride, f, (w, h), out, out_stride);
}

/// The pass down of `predict` alone: fill the `w`×`h` at the start of `out`
/// with the pixels `f` sixteenths below those `BEFORE` rows into `src`; `f`
/// is 1 to 15 and `w` a multiple of 8.
pub(super) fn down(src: &[u8], stride: usize, f: usize, (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
    debug_assert!((1..16).contains(&f) && w.is_multiple_of(8), "{w}×{h} at {f}");
    #[cfg(target_arch = "aarch64")]
    return neon::down(src, stride, f, (w, h), out, out_stride);
    #[cfg(target_arch = "x86_64")]
    return avx2::down(src, stride, f, (w, h), out, out_stride);
}

#[cfg(test)]
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

    // The crate is built for NEON; each `unsafe` here is for
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

#[cfg(target_arch = "x86_64")]
mod avx2 {
    use std::arch::x86_64::*;

    use super::{BEFORE, SUBPEL_FILTERS};

    // The crate is built for AVX2, but its intrinsics are still `unsafe` to
    // call from a function not marked for it; each `unsafe` here is for
    // that and for loads and stores inside the slices bounds-checked just
    // before them.

    /// Eight pixels of `a` from `at`, in the low half.
    #[inline(always)]
    fn eight(a: &[u8], at: usize) -> __m128i {
        let a = &a[at..][..8];
        unsafe { _mm_loadl_epi64(a.as_ptr().cast()) }
    }

    /// The low eight pixels of `v` into `out` from `at`.
    #[inline(always)]
    fn store(out: &mut [u8], at: usize, v: __m128i) {
        let out = &mut out[at..][..8];
        unsafe { _mm_storel_epi64(out.as_mut_ptr().cast(), v) }
    }

    /// `lo` and `hi` as the lanes of one vector.
    #[inline(always)]
    fn lanes(lo: __m128i, hi: __m128i) -> __m256i {
        unsafe { _mm256_set_m128i(hi, lo) }
    }

    /// The taps `f` as `maddubs` takes them: the pair `2k` and `2k + 1`
    /// throughout the `k`th. Each is a byte at every fraction but none,
    /// whose 128 the passes are not asked for.
    #[inline(always)]
    fn pairs(f: usize) -> [__m256i; 4] {
        let t = SUBPEL_FILTERS[f];
        std::array::from_fn(|k| unsafe { _mm256_set1_epi16(i16::from_le_bytes([t[2 * k] as i8 as u8, t[2 * k + 1] as i8 as u8])) })
    }

    /// The taps `f` of sixteen sets of eight pixels, `p[k]` the pixels the
    /// `k`th pair of taps takes, each pixel beside the one after it, rounded
    /// to pixels, the low eight of each lane. As libvpx's AVX2 sums them, in
    /// 16 bits: no pair of taps, nor the first and third pairs or the second
    /// and fourth with the rounding, can take a sum of pixels out of 16 bits
    /// (the taps' signs bound each), and a sum the last addition saturates is
    /// a pixel of 0 or 255 either way.
    #[inline(always)]
    fn taps(p: [__m256i; 4], f: &[__m256i; 4]) -> __m256i {
        unsafe {
            let x: [__m256i; 4] = std::array::from_fn(|k| _mm256_maddubs_epi16(p[k], f[k]));
            let first = _mm256_add_epi16(_mm256_add_epi16(x[0], x[2]), _mm256_set1_epi16(64));
            let sum = _mm256_adds_epi16(first, _mm256_add_epi16(x[1], x[3]));
            let sum = _mm256_srai_epi16::<7>(sum);
            _mm256_packus_epi16(sum, sum)
        }
    }

    /// Per pair of taps, where in the sixteen bytes `chunk` gathers each
    /// pixel the eight across take, each beside the one after it.
    const ACROSS: [[i8; 16]; 4] = {
        let mut masks = [[0; 16]; 4];
        let mut k = 0;
        while k < 4 {
            let mut i = 0;
            while i < 16 {
                // Pixel `m` from the first is byte `m` up to 7, and `m + 1`
                // after: the second eight are loaded from the seventh.
                let m = i / 2 + 2 * k + i % 2;
                masks[k][i] = (if m < 8 { m } else { m + 1 }) as i8;
                i += 1;
            }
            k += 1;
        }
        masks
    };

    /// The fifteen pixels from `at` that eight across take: the eight from
    /// `at` and the eight from `at + 7`, so as not to read past them.
    #[inline(always)]
    fn chunk(src: &[u8], at: usize) -> __m128i {
        unsafe { _mm_unpacklo_epi64(eight(src, at), eight(src, at + 7)) }
    }

    /// The taps `f` across the eight pixels from `at` and the seven after,
    /// in the low lane, and from `at2` in the high.
    #[inline(always)]
    fn across16(src: &[u8], at: usize, at2: usize, f: &[__m256i; 4]) -> __m256i {
        unsafe {
            let v = lanes(chunk(src, at), chunk(src, at2));
            taps(std::array::from_fn(|k| _mm256_shuffle_epi8(v, _mm256_set_m128i(_mm_loadu_si128(ACROSS[k].as_ptr().cast()), _mm_loadu_si128(ACROSS[k].as_ptr().cast())))), f)
        }
    }

    pub(super) fn across(src: &[u8], stride: usize, f: usize, (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
        let f = pairs(f);
        // The eights of every row in turn, two at a time.
        let eights = w / 8;
        let at = |i: usize| (i / eights, i % eights * 8);
        let count = eights * h;
        for i in (0..count).step_by(2) {
            let ((r, c), (r2, c2)) = (at(i), at((i + 1).min(count - 1)));
            let v = across16(src, r * stride + c, r2 * stride + c2, &f);
            unsafe {
                store(out, r * out_stride + c, _mm256_castsi256_si128(v));
                store(out, r2 * out_stride + c2, _mm256_extracti128_si256::<1>(v));
            }
        }
    }

    /// The taps `f` down the eight columns `rows` gives two rows at a time,
    /// as `rows(r)` gives rows `r` and `r + 1` of the `h + 7`, into the `h`
    /// rows of eight at the start of `out`; `rows` gives nothing it is asked
    /// for past them.
    #[inline(always)]
    fn down8(mut rows: impl FnMut(usize) -> [__m128i; 2], f: &[__m256i; 4], h: usize, out: &mut [u8], out_stride: usize) {
        unsafe {
            let u = |a, b| _mm_unpacklo_epi8(a, b);
            let ([s0, s1], [s2, s3], [s4, s5], [mut a, mut b]) = (rows(0), rows(2), rows(4), rows(6));
            // Per pair of taps, the rows it takes for row `r` in the low lane
            // and `r + 1` in the high.
            let mut p = [lanes(u(s0, s1), u(s1, s2)), lanes(u(s2, s3), u(s3, s4)), lanes(u(s4, s5), u(s5, a)), _mm256_setzero_si256()];
            for r in (0..h).step_by(2) {
                let [c, d] = rows(r + 8);
                p[3] = lanes(u(a, b), u(b, c));
                let v = taps(p, f);
                store(out, r * out_stride, _mm256_castsi256_si128(v));
                if r + 1 < h {
                    store(out, (r + 1) * out_stride, _mm256_extracti128_si256::<1>(v));
                }
                p = [p[1], p[2], p[3], p[3]];
                (a, b) = (c, d);
            }
        }
    }

    pub(super) fn down(src: &[u8], stride: usize, f: usize, (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
        let f = pairs(f);
        let rows = h + 7;
        for c in (0..w).step_by(8) {
            let row = |r: usize| if r < rows { eight(src, r * stride + c) } else { unsafe { _mm_setzero_si128() } };
            down8(|r| [row(r), row(r + 1)], &f, h, &mut out[c..], out_stride);
        }
    }

    pub(super) fn predict(src: &[u8], stride: usize, (fx, fy): (usize, usize), (w, h): (usize, usize), out: &mut [u8], out_stride: usize) {
        match (fx, fy) {
            (fx, 0) => across(&src[BEFORE * stride..], stride, fx, (w, h), out, out_stride),
            (0, fy) => down(&src[BEFORE..], stride, fy, (w, h), out, out_stride),
            (fx, fy) => {
                let (x_taps, y_taps) = (pairs(fx), pairs(fy));
                let rows = h + 7;
                for c in (0..w).step_by(8) {
                    // Two rows filtered across at a time, and rounded to
                    // pixels as a decoder rounds them.
                    let filtered = |r: usize| {
                        if r >= rows {
                            return unsafe { [_mm_setzero_si128(); 2] };
                        }
                        let v = across16(src, r * stride + c, (r + 1).min(rows - 1) * stride + c, &x_taps);
                        unsafe { [_mm256_castsi256_si128(v), _mm256_extracti128_si256::<1>(v)] }
                    };
                    down8(filtered, &y_taps, h, &mut out[c..], out_stride);
                }
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
                for f in 1..16 {
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
        // A half pixel across a step from 0 to 160 under the taps -1, -4,
        // 14, 55, 55, 14, -4, -1: halfway is 80, and either side the
        // filter rings, rounded and held to pixels.
        let stride = 8 + TAPS - 1;
        let src: Vec<u8> = (0..stride * TAPS).map(|i| if i % stride >= BEFORE + 4 { 160 } else { 0 }).collect();
        let mut out = [0; 8];
        scalar::across(&src[BEFORE * stride..], stride, 8, 8, 1, &mut out, 8);
        // From the left: -160, -5 × 160, 9 × 160, 64 × 160, 119 × 160 ...
        assert_eq!(out, [0, 0, 11, 80, 149, 166, 161, 160]);
    }
}
