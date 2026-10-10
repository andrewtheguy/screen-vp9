# What the encoder was measured at

The numbers the encoder's decisions were taken on, each in a column of its
own. How the encoder works is in [encoder.md](encoder.md); what was built in
which order, and why, in [plan.md](plan.md).

Two kinds of measurement are here, and they are not on the same machine:

- **Against libvpx**, made before the fork of 2026-10-09 by a harness that
  played each capture into both encoders, on four threads on an Apple M2 Max.
  That harness left this repository with the libvpx encoder; a comparison
  with libvpx is made from `screen-vp9` now, on the same captures.
- **This encoder alone**, by `captures_measured`, `keyframes_measured` and
  `settles_measured` in `src/encoder/mod.rs`, with libvpx decoding. Those
  below were run on a six-core Intel i5-8500T (AVX2), four threads, where a
  time varies by up to a third from one run to the next: only the large
  changes in a time column mean anything.

Throughout, a *ratio* is this encoder over libvpx (or after over before), so
under 1.00 is fewer bytes or less time; a *PSNR difference* is this encoder's
luma PSNR less libvpx's at the same dial, so above zero is a better picture.
`@2` marks a desktop drawn at scale 2.

## Against libvpx, every capture at every size

The `frame-captures` set of 2026-10-09, after phase 8. Phases 9 and 11
changed the encoder since and were measured at 1920×1080 alone, so these are
phase 8's numbers; the run over every size is open in [plan.md](plan.md).

### Bytes over the whole capture, this encoder over libvpx

Under 1.00 is fewer bytes than libvpx.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | 0.86 | 0.80 | 0.83 | 0.83 | 0.75 | 0.77 | 0.79 | 0.87 | 0.86 | 0.84 |
| `uncover` | 0.97 | 0.99 | 0.94 | 1.03 | 1.01 | 1.01 | 1.05 | 1.04 | 0.97 | 1.04 |
| `flood` | 0.84 | 0.80 | 0.74 | 0.76 | 0.96 | 0.97 | 0.97 | 0.97 | 0.97 | 0.97 |
| `browser` | 0.21 | 0.18 | 0.29 | 0.29 | 0.24 | 0.15 | 0.18 | 0.20 | 0.22 | 0.25 |
| `drag` | 0.15 | 0.15 | 0.17 | 0.12 | 0.24 | 0.35 | 0.38 | 0.32 | 0.32 | 0.12 |
| `video` | 1.02 | 1.02 | 1.01 | 1.00 | 0.99 | 0.97 | 0.99 | 1.01 | 1.04 | 0.93 |
| `walk` | 1.11 | 1.12 | 1.12 | 1.09 | 1.06 | 1.01 | 1.01 | 0.98 | 1.09 | 1.06 |

### Encode time, the median of the per-frame ratios, this encoder over libvpx

Under 1.00 is faster than libvpx.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | 0.52 | 0.47 | 0.42 | 0.38 | 0.36 | 0.30 | 0.29 | 0.30 | 0.25 | 0.28 |
| `uncover` | 0.29 | 0.35 | 0.34 | 0.35 | 0.35 | 0.36 | 0.41 | 0.37 | 0.37 | 0.40 |
| `flood` | 0.57 | 0.57 | 0.53 | 0.54 | 0.61 | 0.60 | 0.60 | 0.60 | 0.60 | 0.59 |
| `browser` | 0.58 | 0.65 | 0.66 | 0.67 | 0.65 | 0.59 | 0.63 | 0.61 | 0.59 | 0.64 |
| `drag` | 0.63 | 0.67 | 0.67 | 0.66 | 0.86 | 0.84 | 0.86 | 0.76 | 0.74 | 0.77 |
| `video` | 0.72 | 0.74 | 0.77 | 0.89 | 0.87 | 0.81 | 0.85 | 0.79 | 0.78 | 0.83 |
| `walk` | 0.75 | 0.78 | 0.76 | 0.87 | 0.89 | 0.83 | 0.87 | 0.83 | 0.81 | 0.79 |

### Luma PSNR, this encoder less libvpx, in dB

Above zero is a better picture than libvpx at the same dial.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | 0.0 | 0.0 | 0.0 | −0.1 | +0.3 | +0.3 | +0.2 | +0.3 | +0.2 | +0.2 |
| `uncover` | +0.1 | +0.1 | 0.0 | −0.3 | +0.2 | +0.1 | +0.1 | +0.2 | +0.1 | 0.0 |
| `flood` | +0.2 | +0.3 | +0.3 | +0.2 | +0.4 | +0.4 | +0.4 | +0.4 | +0.4 | +0.4 |
| `browser` | +1.0 | +1.1 | +0.8 | +0.8 | +1.4 | +1.4 | +1.2 | +0.9 | +1.0 | +1.2 |
| `drag` | +1.3 | +1.3 | +0.9 | +1.1 | +0.1 | +0.1 | +0.1 | −0.3 | +0.4 | +1.2 |
| `video` | +0.4 | +0.3 | +0.4 | +0.4 | +0.5 | +0.5 | +0.6 | +0.5 | +0.6 | +0.6 |
| `walk` | +0.1 | +0.2 | +0.4 | +0.3 | +0.6 | +0.8 | +0.8 | +0.9 | +0.8 | +1.0 |


## Against libvpx, the frames apart

The time columns are each encoder's median time for a frame of that kind, and
the time ratio is of those two medians; the bytes are the mean per frame.

### `terminal`, a line of text (≈ 260 a capture)

| Size | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 0.54 | 0.29 | 0.54 | 120 B | 127 B | 1.06 | 0.0 |
| 1440×900 | 0.70 | 0.34 | 0.49 | 120 B | 149 B | 1.24 | 0.0 |
| 1600×1000 | 0.86 | 0.36 | 0.42 | 122 B | 126 B | 1.03 | 0.0 |
| 1920×1080 | 1.09 | 0.42 | 0.39 | 135 B | 134 B | 0.99 | −0.1 |
| 2048×1536@2 | 1.52 | 0.55 | 0.36 | 204 B | 187 B | 0.92 | +0.3 |
| 2560×1600@2 | 2.16 | 0.65 | 0.30 | 196 B | 190 B | 0.97 | +0.3 |
| 2880×1800@2 | 2.55 | 0.73 | 0.29 | 191 B | 229 B | 1.20 | +0.2 |
| 3200×2000@2 | 3.25 | 0.97 | 0.30 | 213 B | 219 B | 1.03 | +0.3 |
| 3456×2168@2 | 3.74 | 0.92 | 0.25 | 216 B | 212 B | 0.98 | +0.2 |
| 3840×2160@2 | 4.05 | 1.12 | 0.28 | 222 B | 222 B | 1.00 | +0.2 |

### `terminal`, a scroll (7 or 8)

| Size | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 4.97 | 4.11 | 0.83 | 40 kB | 34 kB | 0.85 | +0.1 |
| 1440×900 | 5.90 | 4.21 | 0.71 | 48 kB | 36 kB | 0.75 | 0.0 |
| 1600×1000 | 5.60 | 4.20 | 0.75 | 62 kB | 51 kB | 0.82 | −0.2 |
| 1920×1080 | 5.89 | 4.32 | 0.73 | 68 kB | 56 kB | 0.82 | −0.1 |
| 2048×1536@2 | 11.1 | 8.4 | 0.76 | 103 kB | 76 kB | 0.74 | +0.3 |
| 2560×1600@2 | 26.5 | 11.2 | 0.42 | 117 kB | 88 kB | 0.75 | +0.5 |
| 2880×1800@2 | 17.2 | 9.3 | 0.54 | 122 kB | 94 kB | 0.77 | +0.2 |
| 3200×2000@2 | 14.4 | 7.7 | 0.53 | 122 kB | 105 kB | 0.86 | +0.2 |
| 3456×2168@2 | 15.9 | 10.5 | 0.66 | 169 kB | 145 kB | 0.86 | +0.2 |
| 3840×2160@2 | 22.7 | 17.7 | 0.78 | 201 kB | 169 kB | 0.84 | +0.2 |

### `drag`, a quarter of the picture (≈ 390)

| Size | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 4.08 | 2.59 | 0.63 | 5750 B | 615 B | 0.11 | +1.3 |
| 1440×900 | 3.85 | 2.54 | 0.66 | 6438 B | 690 B | 0.11 | +1.3 |
| 1600×1000 | 3.98 | 2.66 | 0.67 | 6212 B | 712 B | 0.11 | +0.9 |
| 1920×1080 | 4.19 | 2.76 | 0.66 | 8533 B | 759 B | 0.09 | +1.2 |
| 2048×1536@2 | 8.54 | 7.33 | 0.86 | 9368 B | 1831 B | 0.20 | +0.1 |
| 2560×1600@2 | 9.11 | 7.68 | 0.84 | 7145 B | 2071 B | 0.29 | +0.1 |
| 2880×1800@2 | 9.57 | 8.22 | 0.86 | 7149 B | 2337 B | 0.33 | +0.1 |
| 3200×2000@2 | 10.7 | 8.2 | 0.77 | 12 kB | 3 kB | 0.25 | −0.3 |
| 3456×2168@2 | 11.0 | 8.2 | 0.75 | 11 kB | 3 kB | 0.27 | +0.4 |
| 3840×2160@2 | 11.0 | 8.4 | 0.76 | 31 kB | 3 kB | 0.10 | +1.3 |


## Against libvpx, the keyframes inside the captures

The first picture of a capture and the one its scenario asks for. Times are
each encoder's median; the bytes ratio is over the capture's keyframes
together.

### Keyframes of `uncover`, a page under a terminal

| Size | libvpx, ms | This encoder, ms | Time ratio | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|
| 1280×800 | 24 | 19 | 0.79 | 1.01 | −0.2 |
| 1440×900 | 19 | 16 | 0.84 | 1.01 | −0.2 |
| 1600×1000 | 25 | 20 | 0.80 | 1.01 | −0.2 |
| 1920×1080 | 34 | 24 | 0.71 | 1.01 | −0.2 |
| 2048×1536@2 | 36 | 26 | 0.72 | 1.01 | −0.1 |
| 2560×1600@2 | 45 | 33 | 0.73 | 1.02 | −0.1 |
| 2880×1800@2 | 52 | 39 | 0.75 | 1.01 | −0.1 |
| 3200×2000@2 | 56 | 43 | 0.77 | 1.02 | −0.1 |
| 3456×2168@2 | 74 | 55 | 0.74 | 1.01 | −0.1 |
| 3840×2160@2 | 77 | 57 | 0.74 | 1.01 | −0.1 |

### Keyframes of `drag`, a terminal of text

| Size | libvpx, ms | This encoder, ms | Time ratio | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|
| 1280×800 | 15 | 12 | 0.80 | 1.00 | −0.2 |
| 1440×900 | 14 | 11 | 0.79 | 0.99 | −0.2 |
| 1600×1000 | 17 | 14 | 0.82 | 0.99 | −0.1 |
| 1920×1080 | 19 | 13 | 0.68 | 0.98 | −0.1 |
| 2048×1536@2 | 23 | 15 | 0.65 | 0.99 | −0.1 |
| 2560×1600@2 | 30 | 20 | 0.67 | 0.99 | −0.1 |
| 2880×1800@2 | 33 | 22 | 0.67 | 0.98 | −0.1 |
| 3200×2000@2 | 45 | 32 | 0.71 | 1.00 | −0.1 |
| 3456×2168@2 | 51 | 31 | 0.61 | 0.98 | −0.1 |
| 3840×2160@2 | 64 | 35 | 0.55 | 0.99 | −0.1 |

### Keyframes of `flood`

| Size | libvpx, ms | This encoder, ms | Time ratio | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|
| 1280×800 | 14 | 11 | 0.79 | 0.99 | −0.1 |
| 1440×900 | 12 | 10 | 0.83 | 0.99 | −0.1 |
| 1600×1000 | 15 | 15 | 1.00 | 0.99 | −0.1 |
| 1920×1080 | 19 | 15 | 0.79 | 0.99 | −0.1 |
| 2048×1536@2 | 24 | 17 | 0.71 | 1.01 | −0.1 |
| 2560×1600@2 | 30 | 20 | 0.67 | 0.99 | −0.1 |
| 2880×1800@2 | 62 | 23 | 0.37 | 0.99 | −0.1 |
| 3200×2000@2 | 50 | 31 | 0.62 | 1.00 | −0.1 |
| 3456×2168@2 | 86 | 35 | 0.41 | 1.00 | −0.1 |
| 3840×2160@2 | 95 | 36 | 0.38 | 1.00 | −0.1 |

### Keyframes of `video`

| Size | libvpx, ms | This encoder, ms | Time ratio | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|
| 1280×800 | 8 | 8 | 1.00 | 0.92 | −0.0 |
| 1440×900 | 8 | 7 | 0.88 | 0.91 | −0.1 |
| 1600×1000 | 8 | 8 | 1.00 | 0.91 | −0.1 |
| 1920×1080 | 8 | 9 | 1.12 | 0.91 | −0.0 |
| 2048×1536@2 | 11 | 12 | 1.09 | 0.94 | −0.1 |
| 2560×1600@2 | 14 | 15 | 1.07 | 0.91 | −0.1 |
| 2880×1800@2 | 17 | 18 | 1.06 | 0.91 | +0.0 |
| 3200×2000@2 | 19 | 21 | 1.11 | 0.91 | −0.1 |
| 3456×2168@2 | 21 | 23 | 1.10 | 0.89 | +0.0 |
| 3840×2160@2 | 19 | 21 | 1.11 | 0.90 | −0.0 |


## What the tables say

- **The picture is libvpx's or better at the same dial on every capture at
  every size**, over all frames: `browser` +0.8 to +1.4 dB at a sixth to a
  third of its bytes, `drag` +0.1 to +1.3 at 0.12–0.38×, `flood` +0.2 to
  +0.4 at 0.74–0.97×, `video` +0.3 to +0.6 at 0.93–1.04×, `walk` +0.1 to
  +1.0 at 0.98–1.12×, `terminal` 0.0 to +0.3 at 0.75–0.87×, `uncover` 0.0
  to +0.2 at 0.94–1.05×; the three cells under are `terminal` and `uncover`
  at 1920×1080 (−0.1, −0.3) and `drag` at 3200×2000 (−0.3). Where the
  picture arrives exact and the compositor damages the whole surface it
  still makes a sixth to a third of libvpx's bytes, now at a better
  picture.
- **A frame of a line of text, the common frame, takes 0.25 to 0.52 of
  libvpx's time at every size** (0.29 ms at 1280×800, 1.12 ms at
  3840×2160), at level bytes (0.95–1.24×, the worst 1440×900 and
  2880×1800, which are two tile columns) and level or better picture. The
  rectangle half of the gate is met.
- **Whole frames take 0.3 to 0.9 of libvpx's time**: `flood` 0.53–0.61,
  `uncover` 0.29–0.41, `browser` 0.58–0.67, `drag` 0.63–0.86, `video`
  0.72–0.89, `walk` 0.75–0.89.
- **Keyframes take 0.9 to 1.0 of libvpx's time** inside the captures at
  −0.0 to −0.2 dB and 0.98–1.02× the bytes, `video`'s 0.9–1.1×. On the
  captures' first pictures coded alone at dials 40 and 90
  (`keyframes_against_libvpx`), `drag`, `flood` and `uncover` take
  0.79–1.06× libvpx's time at −0.1 to −0.4 dB and 0.97–1.02× the bytes,
  `video` 0.89–1.10× at −0.4 to +0.1, `walk` 0.84–1.21× at 0.65–0.82× the
  bytes; `terminal`'s and `browser`'s first pictures in this set are the
  desktop before the resize, a few hundred bytes, and say nothing.
- **What is over the line**: `walk`'s bytes at 0.98–1.12× (the dial's walk
  down to 20 and the settle, phase 9's), `uncover`'s at 0.94–1.05×, and
  the smallest keyframes' time (`video`'s, `walk`'s) at up to 1.2×.

## Settles, before and after phase 11

`settles_measured` at 1920×1080: each capture's first picture coded as a
keyframe at one dial and then settled at 90, the picture unchanged, the least
time of five passes. Before and after a square the previous frame holds
exactly was left in the inactive segment (phase 11). `browser`'s first
picture is a flat desktop and `terminal`'s the 1600×1000 desktop before its
resize, both exact from dial 65 up; the rest are busy pictures, of which
only the flat parts are exact. The luma PSNR of every row is the same before
and after to a hundredth of a decibel.

### A settle from dial 20 to 90

| First picture of | Bytes before | Bytes after | Bytes ratio | Time before, ms | Time after, ms | Time ratio |
|---|---:|---:|---:|---:|---:|---:|
| `browser` | 1594 | 1594 | 1.00 | 24.6 | 18.5 | 0.75 |
| `drag` | 133002 | 132982 | 1.00 | 37.0 | 32.8 | 0.89 |
| `flood` | 169607 | 169613 | 1.00 | 37.6 | 37.6 | 1.00 |
| `terminal` | 1311 | 1311 | 1.00 | 13.6 | 14.0 | 1.03 |
| `uncover` | 299594 | 299605 | 1.00 | 60.4 | 57.8 | 0.96 |
| `video` | 85857 | 85857 | 1.00 | 35.1 | 27.8 | 0.79 |
| `walk` | 67160 | 67160 | 1.00 | 26.3 | 25.3 | 0.96 |

### A settle from dial 65 to 90

| First picture of | Bytes before | Bytes after | Bytes ratio | Time before, ms | Time after, ms | Time ratio |
|---|---:|---:|---:|---:|---:|---:|
| `browser` | 418 | 61 | 0.15 | 22.6 | 4.7 | 0.21 |
| `drag` | 41155 | 41031 | 1.00 | 30.8 | 20.0 | 0.65 |
| `flood` | 53971 | 53905 | 1.00 | 36.9 | 32.5 | 0.88 |
| `terminal` | 373 | 57 | 0.15 | 11.7 | 4.0 | 0.34 |
| `uncover` | 91070 | 91049 | 1.00 | 39.4 | 39.1 | 0.99 |
| `video` | 26583 | 26293 | 0.99 | 26.2 | 16.9 | 0.64 |
| `walk` | 29777 | 29575 | 0.99 | 23.3 | 15.7 | 0.67 |

### A settle from dial 80 to 90

| First picture of | Bytes before | Bytes after | Bytes ratio | Time before, ms | Time after, ms | Time ratio |
|---|---:|---:|---:|---:|---:|---:|
| `browser` | 400 | 45 | 0.11 | 17.2 | 4.6 | 0.27 |
| `drag` | 15285 | 15119 | 0.99 | 26.5 | 22.4 | 0.85 |
| `flood` | 18476 | 18342 | 0.99 | 29.8 | 32.2 | 1.08 |
| `terminal` | 356 | 40 | 0.11 | 12.2 | 4.1 | 0.34 |
| `uncover` | 33612 | 33552 | 1.00 | 36.9 | 29.2 | 0.79 |
| `video` | 8740 | 8450 | 0.97 | 20.1 | 12.2 | 0.61 |
| `walk` | 15309 | 15111 | 0.99 | 23.0 | 14.6 | 0.64 |

### A settle from dial 89 to 90

| First picture of | Bytes before | Bytes after | Bytes ratio | Time before, ms | Time after, ms | Time ratio |
|---|---:|---:|---:|---:|---:|---:|
| `browser` | 400 | 45 | 0.11 | 19.0 | 4.8 | 0.25 |
| `drag` | 2869 | 2716 | 0.95 | 17.5 | 12.8 | 0.73 |
| `flood` | 2821 | 2706 | 0.96 | 19.6 | 20.1 | 1.02 |
| `terminal` | 356 | 40 | 0.11 | 12.3 | 3.6 | 0.29 |
| `uncover` | 6753 | 6707 | 0.99 | 26.3 | 19.1 | 0.73 |
| `video` | 1661 | 1371 | 0.83 | 16.4 | 9.0 | 0.55 |
| `walk` | 5546 | 5348 | 0.96 | 18.9 | 11.9 | 0.63 |

Over the 1920×1080 captures played whole, the same change moved nothing but
`walk`'s one quiet settle, from 387 bytes to 122 (libvpx's was 167), and
`walk`'s other frames by 0.09% of their bytes and −0.03 dB.

## Test material

The captures are what a wlshare session handed `Stream`, written by its own
`--capture-frames` flag in the capture container, lossless, with the
rectangles each frame came with, `zstd` compressed: `<scenario>-<WxH>[@2].wlcap.zst`,
`@2` a desktop drawn at scale 2, as `docker/capture/README.md` beside them
says. Where a machine keeps them is in its `CLAUDE.local.md`, not here;
the harness reads `tmp/captures/` or the directory `CAPTURES` names, and
`ONLY` keeps the captures whose name holds it.

| Scenario | Kind | What a frame names |
|---|---|---|
| `terminal` | quiet | a resize into the size, then a terminal typed into one key at a time and scrolled: a line or two a frame |
| `uncover` | quiet | a terminal over a page moved aside, then closed: the whole surface, by the compositor's damage |
| `flood` | busy | a terminal flooded with coloured random lines, every frame the whole window new |
| `browser` | busy | a long page in Chromium scrolled by wheel, by key and back: the whole surface |
| `drag` | busy | a floating terminal dragged around the desktop: a quarter of the picture |
| `video` | busy | a test pattern playing in a 1280×720 window, a keyframe asked at 10 s |
| `walk` | busy | the video while the client reads late, so the dial walks down to 20 and back, then stopped so the desktop settles |

Sizes: the ten `capture-frames.sh` runs by default, which vp9-wasm's
benchmark has samples of: 1280×800, 1440×900, 1600×1000 and 1920×1080 at
scale 1, and the same four desktops at scale 2, 2560×1600, 2880×1800,
3200×2000 and 3840×2160, with 2048×1536 and 3456×2168. The set measured
from phase 8.7 on is the `frame-captures` of 2026-10-09 in the artifacts'
backup (`/Volumes/dasdata/backup/vp9-artifacts/frame-captures/`, 70
files), copied to `tmp/captures/`; the 63 files of nine sizes the phases
before were measured on are archived beside it. Each is measured at quality 90, the
dial the captures were made at, and at the dials the `walk` reaches, on four
threads and on five; `terminal` and `flood` once more at 100, the dial the
first plan measured wlshare at.

The eighteen decoded samples the first gate read, and the libvpx-coded
samples `vp9-samples.sh` makes for vp9-wasm, are not encoder material: the
first are decodes of decodes, the second are coded by the encoder under
test's rival.
