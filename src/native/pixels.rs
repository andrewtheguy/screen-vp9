//! Kernels over 8-bit pixels: the moments and the sums of rows and columns
//! the partition decision reads, and a residual. Each is NEON on arm64 and AVX2 on amd64, and held by its
//! tests to the scalar reference beside it.

/// The sum and the sum of squares of the 8×8 at the start of `a`, rows
/// `stride` apart.
pub(super) fn moments(a: &[u8], stride: usize) -> (i32, i32) {
    #[cfg(target_arch = "aarch64")]
    return neon::moments(a, stride);
    #[cfg(target_arch = "x86_64")]
    return avx2::moments(a, stride);
}

/// The sums of each row and of each column of the 8×8 at the start of `a`,
/// rows `stride` apart.
pub(super) fn line_sums(a: &[u8], stride: usize) -> ([u16; 8], [u16; 8]) {
    #[cfg(target_arch = "aarch64")]
    return neon::line_sums(a, stride);
    #[cfg(target_arch = "x86_64")]
    return avx2::line_sums(a, stride);
}

/// The sum and the sum of squares of the 8×8 at the start of `a` less the
/// one at the start of `b`.
pub(super) fn difference_moments(a: &[u8], a_stride: usize, b: &[u8], b_stride: usize) -> (i32, i32) {
    #[cfg(target_arch = "aarch64")]
    return neon::difference_moments(a, a_stride, b, b_stride);
    #[cfg(target_arch = "x86_64")]
    return avx2::difference_moments(a, a_stride, b, b_stride);
}

/// `difference_moments` of each of the four 8×8s side by side at the start
/// of `a`, less the four at the start of `b`.
pub(super) fn difference_moments4(a: &[u8], a_stride: usize, b: &[u8], b_stride: usize) -> [(i32, i32); 4] {
    #[cfg(target_arch = "aarch64")]
    return neon::difference_moments4(a, a_stride, b, b_stride);
    #[cfg(target_arch = "x86_64")]
    return avx2::difference_moments4(a, a_stride, b, b_stride);
}

/// The `n`×`n` at the start of `a` less the one at the start of `b`, into
/// `out` in raster order; `n` is 4 to 32.
pub(super) fn subtract(n: usize, a: &[u8], a_stride: usize, b: &[u8], b_stride: usize, out: &mut [i16]) {
    #[cfg(target_arch = "aarch64")]
    return neon::subtract(n, a, a_stride, b, b_stride, out);
    #[cfg(target_arch = "x86_64")]
    return avx2::subtract(n, a, a_stride, b, b_stride, out);
}

#[cfg(test)]
mod scalar {
    pub(super) fn moments(a: &[u8], stride: usize) -> (i32, i32) {
        let (mut sum, mut squares) = (0, 0);
        for r in 0..8 {
            for &v in &a[r * stride..][..8] {
                sum += i32::from(v);
                squares += i32::from(v) * i32::from(v);
            }
        }
        (sum, squares)
    }

    pub(super) fn line_sums(a: &[u8], stride: usize) -> ([u16; 8], [u16; 8]) {
        let (mut rows, mut cols) = ([0; 8], [0; 8]);
        for r in 0..8 {
            for (c, &v) in a[r * stride..][..8].iter().enumerate() {
                rows[r] += u16::from(v);
                cols[c] += u16::from(v);
            }
        }
        (rows, cols)
    }

    pub(super) fn difference_moments(a: &[u8], a_stride: usize, b: &[u8], b_stride: usize) -> (i32, i32) {
        let (mut sum, mut squares) = (0, 0);
        for r in 0..8 {
            for (&x, &y) in a[r * a_stride..][..8].iter().zip(&b[r * b_stride..][..8]) {
                let d = i32::from(x) - i32::from(y);
                sum += d;
                squares += d * d;
            }
        }
        (sum, squares)
    }

    pub(super) fn subtract(n: usize, a: &[u8], a_stride: usize, b: &[u8], b_stride: usize, out: &mut [i16]) {
        for r in 0..n {
            for ((o, &x), &y) in out[r * n..][..n].iter_mut().zip(&a[r * a_stride..][..n]).zip(&b[r * b_stride..][..n]) {
                *o = i16::from(x) - i16::from(y);
            }
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod neon {
    use std::arch::aarch64::*;

    // The crate is built for NEON; each `unsafe` here is for
    // that and for loads and stores inside the slices bounds-checked just
    // before them.

    /// Eight pixels of `a` from `at`.
    #[inline(always)]
    fn eight(a: &[u8], at: usize) -> uint8x8_t {
        let a = &a[at..][..8];
        unsafe { vld1_u8(a.as_ptr()) }
    }

    pub(super) fn moments(a: &[u8], stride: usize) -> (i32, i32) {
        unsafe {
            let (mut sum, mut squares) = (vdupq_n_u16(0), vdupq_n_u32(0));
            for r in 0..8 {
                let v = eight(a, r * stride);
                sum = vaddw_u8(sum, v);
                squares = vpadalq_u16(squares, vmull_u8(v, v));
            }
            (vaddlvq_u16(sum) as i32, vaddvq_u32(squares) as i32)
        }
    }

    pub(super) fn line_sums(a: &[u8], stride: usize) -> ([u16; 8], [u16; 8]) {
        let (mut rows, mut cols) = ([0; 8], [0; 8]);
        unsafe {
            let mut sum = vdupq_n_u16(0);
            for (r, row) in rows.iter_mut().enumerate() {
                let v = eight(a, r * stride);
                sum = vaddw_u8(sum, v);
                *row = vaddlv_u8(v);
            }
            // SAFETY: `cols` is eight values.
            vst1q_u16(cols.as_mut_ptr(), sum);
        }
        (rows, cols)
    }

    pub(super) fn difference_moments(a: &[u8], a_stride: usize, b: &[u8], b_stride: usize) -> (i32, i32) {
        unsafe {
            let (mut sum, mut squares) = (vdupq_n_s16(0), vdupq_n_s32(0));
            for r in 0..8 {
                let d = vreinterpretq_s16_u16(vsubl_u8(eight(a, r * a_stride), eight(b, r * b_stride)));
                sum = vaddq_s16(sum, d);
                squares = vmlal_s16(squares, vget_low_s16(d), vget_low_s16(d));
                squares = vmlal_high_s16(squares, d, d);
            }
            (vaddlvq_s16(sum), vaddvq_s32(squares))
        }
    }

    /// An 8×8 at a time: eight pixels are a register's worth here.
    pub(super) fn difference_moments4(a: &[u8], a_stride: usize, b: &[u8], b_stride: usize) -> [(i32, i32); 4] {
        std::array::from_fn(|i| difference_moments(&a[8 * i..], a_stride, &b[8 * i..], b_stride))
    }

    pub(super) fn subtract(n: usize, a: &[u8], a_stride: usize, b: &[u8], b_stride: usize, out: &mut [i16]) {
        if n == 4 {
            // A row's four pixels each as the low half of a vector of eight.
            let four = |p: &[u8], at: usize| u64::from(u32::from_le_bytes(p[at..at + 4].try_into().expect("four pixels")));
            for r in 0..4 {
                let o = &mut out[r * 4..][..4];
                // SAFETY: `o` is four values.
                unsafe {
                    let d = vreinterpretq_s16_u16(vsubl_u8(vcreate_u8(four(a, r * a_stride)), vcreate_u8(four(b, r * b_stride))));
                    vst1_s16(o.as_mut_ptr(), vget_low_s16(d));
                }
            }
            return;
        }
        for r in 0..n {
            for c in (0..n).step_by(8) {
                let o = &mut out[r * n + c..][..8];
                // SAFETY: `o` is eight values.
                unsafe { vst1q_s16(o.as_mut_ptr(), vreinterpretq_s16_u16(vsubl_u8(eight(a, r * a_stride + c), eight(b, r * b_stride + c)))) };
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
mod avx2 {
    use std::arch::x86_64::*;

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

    /// Rows `r` and `r + 1` of the 8×8 at the start of `a`, widened to 16 bits.
    #[inline(always)]
    fn two_rows(a: &[u8], stride: usize, r: usize) -> __m256i {
        unsafe { _mm256_cvtepu8_epi16(_mm_unpacklo_epi64(eight(a, r * stride), eight(a, (r + 1) * stride))) }
    }

    /// The sum of the eight `i32` of `v`.
    #[inline(always)]
    fn total(v: __m256i) -> i32 {
        unsafe {
            let v = _mm_add_epi32(_mm256_castsi256_si128(v), _mm256_extracti128_si256::<1>(v));
            let v = _mm_add_epi32(v, _mm_shuffle_epi32::<0b01_00_11_10>(v));
            _mm_cvtsi128_si32(_mm_add_epi32(v, _mm_shuffle_epi32::<0b10_11_00_01>(v)))
        }
    }

    pub(super) fn moments(a: &[u8], stride: usize) -> (i32, i32) {
        unsafe {
            let (mut sum, mut squares) = (_mm256_setzero_si256(), _mm256_setzero_si256());
            for r in (0..8).step_by(2) {
                let v = two_rows(a, stride, r);
                sum = _mm256_add_epi16(sum, v);
                squares = _mm256_add_epi32(squares, _mm256_madd_epi16(v, v));
            }
            (total(_mm256_madd_epi16(sum, _mm256_set1_epi16(1))), total(squares))
        }
    }

    pub(super) fn line_sums(a: &[u8], stride: usize) -> ([u16; 8], [u16; 8]) {
        let (mut rows, mut cols) = ([0; 8], [0; 8]);
        unsafe {
            let mut sum = _mm256_setzero_si256();
            for r in (0..8).step_by(2) {
                let pair = _mm_unpacklo_epi64(eight(a, r * stride), eight(a, (r + 1) * stride));
                sum = _mm256_add_epi16(sum, _mm256_cvtepu8_epi16(pair));
                // Each row's eight pixels summed into its own half.
                let rows_of = _mm_sad_epu8(pair, _mm_setzero_si128());
                rows[r] = _mm_extract_epi16::<0>(rows_of) as u16;
                rows[r + 1] = _mm_extract_epi16::<4>(rows_of) as u16;
            }
            let sum = _mm_add_epi16(_mm256_castsi256_si128(sum), _mm256_extracti128_si256::<1>(sum));
            // SAFETY: `cols` is eight values.
            _mm_storeu_si128(cols.as_mut_ptr().cast(), sum);
        }
        (rows, cols)
    }

    pub(super) fn difference_moments(a: &[u8], a_stride: usize, b: &[u8], b_stride: usize) -> (i32, i32) {
        unsafe {
            let (mut sum, mut squares) = (_mm256_setzero_si256(), _mm256_setzero_si256());
            for r in (0..8).step_by(2) {
                let d = _mm256_sub_epi16(two_rows(a, a_stride, r), two_rows(b, b_stride, r));
                sum = _mm256_add_epi16(sum, d);
                squares = _mm256_add_epi32(squares, _mm256_madd_epi16(d, d));
            }
            (total(_mm256_madd_epi16(sum, _mm256_set1_epi16(1))), total(squares))
        }
    }

    /// A row of the four at a time, thirty-two pixels a register: each
    /// 8×8's sum is its row's in `a` less its row's in `b`, which `vpsadbw`
    /// against nothing gives eight pixels at a time, and its squares are
    /// those of the differences' sizes, widened to 16 bits a half at a time.
    pub(super) fn difference_moments4(a: &[u8], a_stride: usize, b: &[u8], b_stride: usize) -> [(i32, i32); 4] {
        let row = |p: &[u8], at: usize| {
            let p = &p[at..][..32];
            unsafe { _mm256_loadu_si256(p.as_ptr().cast()) }
        };
        unsafe {
            let zero = _mm256_setzero_si256();
            // The low half of each lane is the first and the third 8×8,
            // the high half the second and the fourth.
            let (mut sums, mut low, mut high) = (zero, zero, zero);
            for r in 0..8 {
                let (x, y) = (row(a, r * a_stride), row(b, r * b_stride));
                sums = _mm256_add_epi64(sums, _mm256_sub_epi64(_mm256_sad_epu8(x, zero), _mm256_sad_epu8(y, zero)));
                let d = _mm256_or_si256(_mm256_subs_epu8(x, y), _mm256_subs_epu8(y, x));
                let (l, h) = (_mm256_unpacklo_epi8(d, zero), _mm256_unpackhi_epi8(d, zero));
                low = _mm256_add_epi32(low, _mm256_madd_epi16(l, l));
                high = _mm256_add_epi32(high, _mm256_madd_epi16(h, h));
            }
            // Each 8×8's four sums of squares into one.
            let pairs = _mm256_hadd_epi32(low, high);
            let squares = _mm256_hadd_epi32(pairs, pairs);
            let (mut sum, mut square) = ([0i64; 4], [0i32; 8]);
            // SAFETY: each is a register's worth.
            _mm256_storeu_si256(sum.as_mut_ptr().cast(), sums);
            _mm256_storeu_si256(square.as_mut_ptr().cast(), squares);
            [(sum[0] as i32, square[0]), (sum[1] as i32, square[1]), (sum[2] as i32, square[4]), (sum[3] as i32, square[5])]
        }
    }

    pub(super) fn subtract(n: usize, a: &[u8], a_stride: usize, b: &[u8], b_stride: usize, out: &mut [i16]) {
        match n {
            4 => {
                // Two rows' four pixels at a time.
                let four = |p: &[u8], at: usize| unsafe { _mm_cvtsi32_si128(i32::from_le_bytes(p[at..at + 4].try_into().expect("four pixels"))) };
                let two = |p: &[u8], stride: usize, r: usize| unsafe { _mm_cvtepu8_epi16(_mm_unpacklo_epi32(four(p, r * stride), four(p, (r + 1) * stride))) };
                for r in (0..4).step_by(2) {
                    let o = &mut out[r * 4..][..8];
                    unsafe { _mm_storeu_si128(o.as_mut_ptr().cast(), _mm_sub_epi16(two(a, a_stride, r), two(b, b_stride, r))) };
                }
            }
            8 => {
                for r in 0..8 {
                    let o = &mut out[r * 8..][..8];
                    unsafe { _mm_storeu_si128(o.as_mut_ptr().cast(), _mm_sub_epi16(_mm_cvtepu8_epi16(eight(a, r * a_stride)), _mm_cvtepu8_epi16(eight(b, r * b_stride)))) };
                }
            }
            _ => {
                // Sixteen pixels a register.
                let sixteen = |p: &[u8], at: usize| {
                    let p = &p[at..][..16];
                    unsafe { _mm256_cvtepu8_epi16(_mm_loadu_si128(p.as_ptr().cast())) }
                };
                for r in 0..n {
                    for c in (0..n).step_by(16) {
                        let o = &mut out[r * n + c..][..16];
                        unsafe { _mm256_storeu_si256(o.as_mut_ptr().cast(), _mm256_sub_epi16(sixteen(a, r * a_stride + c), sixteen(b, r * b_stride + c))) };
                    }
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

    #[test]
    fn the_kernels_are_their_scalar_references() {
        for seed in 1..50 {
            let (a, b) = (noise(40 * 40, seed), noise(40 * 40, seed + 99));
            // Flat and extreme blocks as well as noise.
            let a = match seed % 5 {
                0 => vec![255; a.len()],
                1 => vec![0; a.len()],
                _ => a,
            };
            let (sa, sb) = (33 + seed as usize % 7, 35);
            let at = seed as usize % 5;
            assert_eq!(moments(&a[at..], sa), scalar::moments(&a[at..], sa), "seed {seed}");
            assert_eq!(line_sums(&a[at..], sa), scalar::line_sums(&a[at..], sa), "seed {seed}");
            assert_eq!(difference_moments(&a[at..], sa, &b, sb), scalar::difference_moments(&a[at..], sa, &b, sb), "seed {seed}");
            assert_eq!(difference_moments(&b, sb, &a[at..], sa), scalar::difference_moments(&b, sb, &a[at..], sa), "seed {seed}");
            for (x, xs, y, ys) in [(&a[at..], sa, &b[..], sb), (&b[..], sb, &a[at..], sa)] {
                let each: [(i32, i32); 4] = std::array::from_fn(|i| scalar::difference_moments(&x[8 * i..], xs, &y[8 * i..], ys));
                assert_eq!(difference_moments4(x, xs, y, ys), each, "seed {seed}");
            }
            for n in [4, 8, 16, 32] {
                let (mut got, mut want) = (vec![0; n * n], vec![0; n * n]);
                let (a, b) = (noise(n * 40, seed), noise(n * 40, seed + 7));
                subtract(n, &a, 40, &b, 39, &mut got);
                scalar::subtract(n, &a, 40, &b, 39, &mut want);
                assert_eq!(got, want, "{n}x{n}, seed {seed}");
            }
        }
    }
}
