# What the encoder was measured at on an Apple M2 Max

The same three measurements as [benchmarks.md](benchmarks.md), on the same
captures (its *Test material*), on the Mac the NEON kernels are written on.
The two machines' tables are not to be read against each other: a ratio here
is this encoder's NEON kernels against libvpx's, there its AVX2 kernels
against libvpx's.

They were run on 2026-10-10 on an Apple M2 Max (eight performance cores and
four efficiency ones), macOS 26, on AC power in its high power mode, four
threads, on the encoder of commit `03de3d1`, one measurement at a time and
nothing built beside them, but on a desktop in use: a time on this machine
moves by up to three fifths from one run to the next. macOS pins no thread
to a core and turns no boost off, and a container is no help, being a
virtual machine whose processors are the host's threads. So each
measurement was run three times over, the three one after another, and
each line below is the run whose time ratio is the median of the three
(`tools/median_of_runs.py`, then `tools/tables.py`). What holds the ratios
steady is the harness: a frame is coded by both encoders turn about, the
first of them alternating, and the ratio is taken frame by frame; a keyframe
or a settle is the least time of five passes.

How far the three runs were apart, a cell's largest time ratio less its
least:

| Measurement | Cells | Typical | Nine in ten under | Most | Cells the runs put on both sides of 1.00 |
|---|---:|---:|---:|---:|---:|
| `captures_against_libvpx` | 236 | 0.013 | 0.061 | 0.167 | 0 |
| `keyframes_against_libvpx` | 210 | 0.012 | 0.029 | 0.071 | 0 |
| `settles_against_libvpx` | 280 | 0.028 | 0.070 | 0.148 | 10, each within 0.96–1.08 |

The times themselves moved far more, libvpx's by 5% typically and up to
62%, this encoder's by 3% and up to 28%: read the ratios here, not the
milliseconds. This encoder's bytes were the same in all three runs;
libvpx's differed in nine cells of 236, by up to 3% (`drag` at 1920×1080,
2560×1600 and 3456×2168, `terminal` at 1440×900), and not at all in a
keyframe or a settle.

Throughout, a *ratio* is this encoder over libvpx and a *PSNR difference*
this encoder's luma PSNR less libvpx's at the same dial, as in
[benchmarks.md](benchmarks.md).

## What the tables say

- **Every capture at every size takes less time than libvpx**, frame for
  frame and summed: a line of text 0.19–0.34, `uncover` 0.22–0.30, `flood`
  0.36–0.63, `browser` 0.48–0.54, `drag` 0.52–0.72, `video` 0.61–0.74,
  `walk` 0.63–0.74. From 2048 wide `flood` and `drag` are nearer the line
  than below it (0.36–0.49 to 0.62–0.63, 0.52–0.56 to 0.61–0.72).
- **Over a whole capture the picture is libvpx's or better in 68 cells of
  70**, the two under being `uncover` at 1920×1080 and `drag` at 3200×2000
  (−0.1 dB each): `browser` +0.8 to +1.5 dB at 0.15–0.28× the bytes,
  `drag` −0.1 to +1.5 at 0.12–0.38×, `flood` +0.3 to +0.7 at 0.74–0.97×,
  `terminal` +0.1 to +0.5 at 0.74–0.86×, `video` +0.4 to +0.7 at
  0.95–1.05×, `walk` +0.2 to +1.1 at 0.99–1.10×, `uncover` −0.1 to +0.3 at
  0.94–1.04×.
- **A keyframe inside a busy capture is 0.1 to 0.3 dB over libvpx's** at
  0.97–1.01× the bytes (`video`'s 0.89–0.93×, `walk`'s 0.76–0.84×) in 0.48
  to 0.65 of its time; inside `terminal` and `browser`, 0.0 to 0.3 dB at
  0.22–0.83× in 0.33 to 0.58.
- **A first picture coded alone as a keyframe takes less time than libvpx's
  in all 210 cells**: 0.49 to 0.82 of it up to 1920 wide and 0.51 to 0.92
  from 2048 wide, the farthest under it at dial 90 and the nearest at 40.
  Of a busy picture it is +0.3 to +0.5 dB at dial 90, +0.4 to +0.6 at 65
  and +0.2 to +0.6 at 40 at 0.98–1.03× the bytes; `walk`'s is −0.15 to +1.0
  at 0.67–0.84× the bytes, the −0.15 at dial 40 at 3456×2168 the one cell
  under. `terminal`'s and `browser`'s first pictures are flat, held
  exactly where libvpx's is 48 dB at dial 40, in 0.22–0.38× its bytes.
- **A settle of a busy picture takes more time than libvpx's** (`uncover`,
  `flood`, `drag`, `walk`): from 65, 0.96 to 1.28 of it up to 1920 wide and
  1.11 to 1.42 from 2048 wide, for +0.4 to +1.4 dB at 0.96–1.00× the
  bytes; from 80, 1.00 to 1.42 for −0.1 to +0.8 dB at 0.65–0.98×; from 89,
  1.01 to 1.35 for +0.3 to +0.7 dB at 0.44–1.11×; from 20, 0.53 to 1.01
  for +0.1 to +0.7 dB at 0.84–1.02×. Of the others (`terminal`,
  `browser`, `video`) a settle takes 0.33 to 1.31 of libvpx's time, over
  1.00 only for `video`.
- **What is over the line in bytes**: a flat desktop settled from dial 20
  at 4 to 9.6× (one to six kilobytes against a few hundred bytes), and from
  65 and up at up to 2.1× (tens of bytes); `walk`'s captures at up to 1.10×, `video`'s at up to 1.05×,
  `uncover`'s at up to 1.04×.

Each encoder's own time, per frame of a whole capture, from the same runs:

| Capture | libvpx at 1920×1080 | this encoder | libvpx at 3840×2160@2 | this encoder |
|---|---:|---:|---:|---:|
| `terminal` | 1.07 ms | 0.29 ms | 4.19 ms | 0.81 ms |
| `uncover` | 2.61 ms | 0.83 ms | 8.78 ms | 2.43 ms |
| `drag` | 4.20 ms | 2.28 ms | 10.45 ms | 7.14 ms |
| `walk` | 3.92 ms | 2.83 ms | 22.31 ms | 14.65 ms |
| `video` | 5.28 ms | 3.97 ms | 15.13 ms | 10.45 ms |
| `browser` | 10.59 ms | 5.72 ms | 32.47 ms | 16.23 ms |
| `flood` | 96.35 ms | 42.58 ms | 232.46 ms | 144.89 ms |

## Against libvpx, every capture at every size

### Bytes over the whole capture, this encoder over libvpx

Under 1.00 is fewer bytes than libvpx.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.85}$ | $\color{green}{0.79}$ | $\color{green}{0.82}$ | $\color{green}{0.82}$ | $\color{green}{0.74}$ | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ | $\color{green}{0.83}$ |
| `uncover` | $\color{green}{0.97}$ | $\color{green}{0.99}$ | $\color{green}{0.94}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{green}{0.96}$ | $\color{red}{1.03}$ |
| `flood` | $\color{green}{0.83}$ | $\color{green}{0.80}$ | $\color{green}{0.74}$ | $\color{green}{0.76}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ |
| `browser` | $\color{green}{0.20}$ | $\color{green}{0.18}$ | $\color{green}{0.28}$ | $\color{green}{0.27}$ | $\color{green}{0.24}$ | $\color{green}{0.15}$ | $\color{green}{0.17}$ | $\color{green}{0.19}$ | $\color{green}{0.20}$ | $\color{green}{0.25}$ |
| `drag` | $\color{green}{0.14}$ | $\color{green}{0.15}$ | $\color{green}{0.17}$ | $\color{green}{0.12}$ | $\color{green}{0.23}$ | $\color{green}{0.34}$ | $\color{green}{0.38}$ | $\color{green}{0.31}$ | $\color{green}{0.32}$ | $\color{green}{0.12}$ |
| `video` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | 1.00 | $\color{red}{1.04}$ | 1.00 | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.05}$ | $\color{green}{0.95}$ |
| `walk` | $\color{red}{1.09}$ | $\color{red}{1.10}$ | $\color{red}{1.10}$ | $\color{red}{1.07}$ | $\color{red}{1.09}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{green}{0.99}$ | $\color{red}{1.10}$ | $\color{red}{1.06}$ |

### Encode time, the median of the per-frame ratios

Under 1.00 is faster than libvpx, frame for frame.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.34}$ | $\color{green}{0.34}$ | $\color{green}{0.29}$ | $\color{green}{0.27}$ | $\color{green}{0.29}$ | $\color{green}{0.23}$ | $\color{green}{0.23}$ | $\color{green}{0.21}$ | $\color{green}{0.19}$ | $\color{green}{0.19}$ |
| `uncover` | $\color{green}{0.22}$ | $\color{green}{0.27}$ | $\color{green}{0.28}$ | $\color{green}{0.30}$ | $\color{green}{0.30}$ | $\color{green}{0.30}$ | $\color{green}{0.27}$ | $\color{green}{0.26}$ | $\color{green}{0.27}$ | $\color{green}{0.28}$ |
| `flood` | $\color{green}{0.36}$ | $\color{green}{0.49}$ | $\color{green}{0.45}$ | $\color{green}{0.46}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.62}$ | $\color{green}{0.62}$ |
| `browser` | $\color{green}{0.48}$ | $\color{green}{0.53}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.51}$ | $\color{green}{0.51}$ | $\color{green}{0.49}$ | $\color{green}{0.49}$ | $\color{green}{0.51}$ |
| `drag` | $\color{green}{0.52}$ | $\color{green}{0.55}$ | $\color{green}{0.56}$ | $\color{green}{0.54}$ | $\color{green}{0.72}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ | $\color{green}{0.62}$ | $\color{green}{0.61}$ | $\color{green}{0.68}$ |
| `video` | $\color{green}{0.61}$ | $\color{green}{0.62}$ | $\color{green}{0.66}$ | $\color{green}{0.74}$ | $\color{green}{0.71}$ | $\color{green}{0.68}$ | $\color{green}{0.69}$ | $\color{green}{0.68}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ |
| `walk` | $\color{green}{0.65}$ | $\color{green}{0.66}$ | $\color{green}{0.63}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.68}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.68}$ | $\color{green}{0.67}$ |

### Encode time, the whole capture's

Every frame's time summed, this encoder's over libvpx's: where the long frames weigh.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.38}$ | $\color{green}{0.38}$ | $\color{green}{0.34}$ | $\color{green}{0.32}$ | $\color{green}{0.35}$ | $\color{green}{0.30}$ | $\color{green}{0.30}$ | $\color{green}{0.27}$ | $\color{green}{0.25}$ | $\color{green}{0.25}$ |
| `uncover` | $\color{green}{0.33}$ | $\color{green}{0.38}$ | $\color{green}{0.40}$ | $\color{green}{0.39}$ | $\color{green}{0.46}$ | $\color{green}{0.42}$ | $\color{green}{0.40}$ | $\color{green}{0.41}$ | $\color{green}{0.40}$ | $\color{green}{0.40}$ |
| `flood` | $\color{green}{0.37}$ | $\color{green}{0.49}$ | $\color{green}{0.45}$ | $\color{green}{0.46}$ | $\color{green}{0.64}$ | $\color{green}{0.64}$ | $\color{green}{0.64}$ | $\color{green}{0.64}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ |
| `browser` | $\color{green}{0.47}$ | $\color{green}{0.53}$ | $\color{green}{0.53}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.50}$ | $\color{green}{0.51}$ | $\color{green}{0.49}$ | $\color{green}{0.48}$ | $\color{green}{0.51}$ |
| `drag` | $\color{green}{0.52}$ | $\color{green}{0.55}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ | $\color{green}{0.71}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ | $\color{green}{0.61}$ | $\color{green}{0.61}$ | $\color{green}{0.64}$ |
| `video` | $\color{green}{0.62}$ | $\color{green}{0.62}$ | $\color{green}{0.67}$ | $\color{green}{0.74}$ | $\color{green}{0.71}$ | $\color{green}{0.67}$ | $\color{green}{0.70}$ | $\color{green}{0.67}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ |
| `walk` | $\color{green}{0.66}$ | $\color{green}{0.66}$ | $\color{green}{0.64}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.68}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.68}$ | $\color{green}{0.67}$ |

### Luma PSNR, this encoder less libvpx, in dB

Above zero is a better picture than libvpx at the same dial.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ |
| `browser` | $\color{green}{+1.0}$ | $\color{green}{+1.0}$ | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ | $\color{green}{+1.2}$ | $\color{green}{+1.1}$ | $\color{green}{+1.2}$ | $\color{green}{+1.3}$ |
| `drag` | $\color{green}{+1.3}$ | $\color{green}{+1.2}$ | $\color{green}{+1.0}$ | $\color{green}{+1.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.5}$ | $\color{green}{+1.5}$ |
| `video` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.5}$ |
| `walk` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ | $\color{green}{+1.1}$ |

## Against libvpx, the frames apart

The time columns are each encoder's median time for a frame of that kind, the
time ratio the median of the per-frame ratios, and the bytes the mean per frame.

### `terminal`, a line of text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 230 | 0.53 | $\color{green}{0.18}$ | $\color{green}{0.34}$ | 120 B | 122 B | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1440×900 | 269 | 0.73 | $\color{green}{0.25}$ | $\color{green}{0.34}$ | 120 B | 145 B | $\color{red}{1.21}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 270 | 0.81 | $\color{green}{0.23}$ | $\color{green}{0.29}$ | 122 B | 122 B | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 271 | 1.07 | $\color{green}{0.28}$ | $\color{green}{0.26}$ | 135 B | 132 B | $\color{green}{0.98}$ | $\color{green}{+0.1}$ |
| 2048×1536@2 | 268 | 1.53 | $\color{green}{0.45}$ | $\color{green}{0.29}$ | 204 B | 185 B | $\color{green}{0.91}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 270 | 2.01 | $\color{green}{0.47}$ | $\color{green}{0.23}$ | 196 B | 189 B | $\color{green}{0.96}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 270 | 2.42 | $\color{green}{0.56}$ | $\color{green}{0.23}$ | 191 B | 227 B | $\color{red}{1.19}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 232 | 3.36 | $\color{green}{0.72}$ | $\color{green}{0.21}$ | 213 B | 217 B | $\color{red}{1.02}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 265 | 3.83 | $\color{green}{0.74}$ | $\color{green}{0.19}$ | 216 B | 210 B | $\color{green}{0.97}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 264 | 4.18 | $\color{green}{0.80}$ | $\color{green}{0.19}$ | 222 B | 220 B | $\color{green}{0.99}$ | $\color{green}{+0.4}$ |

### `terminal`, a scroll

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 8 | 4.56 | $\color{green}{3.05}$ | $\color{green}{0.34}$ | 40 kB | 33 kB | $\color{green}{0.84}$ | $\color{green}{+0.2}$ |
| 1440×900 | 8 | 5.85 | $\color{green}{3.03}$ | $\color{green}{0.47}$ | 46 kB | 36 kB | $\color{green}{0.77}$ | $\color{green}{+0.1}$ |
| 1600×1000 | 7 | 5.71 | $\color{green}{3.43}$ | $\color{green}{0.44}$ | 62 kB | 50 kB | $\color{green}{0.81}$ | $\color{red}{-0.1}$ |
| 1920×1080 | 7 | 6.50 | $\color{green}{3.56}$ | $\color{green}{0.44}$ | 68 kB | 56 kB | $\color{green}{0.81}$ | 0.0 |
| 2048×1536@2 | 8 | 11.4 | $\color{green}{7.44}$ | $\color{green}{0.44}$ | 103 kB | 74 kB | $\color{green}{0.72}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 8 | 12.4 | $\color{green}{8.16}$ | $\color{green}{0.49}$ | 117 kB | 86 kB | $\color{green}{0.73}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 8 | 17.0 | $\color{green}{7.39}$ | $\color{green}{0.49}$ | 122 kB | 90 kB | $\color{green}{0.74}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 7 | 14.1 | $\color{green}{6.04}$ | $\color{green}{0.40}$ | 122 kB | 104 kB | $\color{green}{0.85}$ | $\color{green}{+0.3}$ |
| 3456×2168@2 | 7 | 15.5 | $\color{green}{8.46}$ | $\color{green}{0.37}$ | 169 kB | 142 kB | $\color{green}{0.84}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 6 | 22.6 | $\color{green}{12.5}$ | $\color{green}{0.48}$ | 201 kB | 165 kB | $\color{green}{0.82}$ | $\color{green}{+0.4}$ |

### `drag`, a quarter of the picture

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 394 | 4.11 | $\color{green}{2.17}$ | $\color{green}{0.52}$ | 5750 B | 593 B | $\color{green}{0.10}$ | $\color{green}{+1.3}$ |
| 1440×900 | 394 | 3.74 | $\color{green}{2.05}$ | $\color{green}{0.55}$ | 6438 B | 666 B | $\color{green}{0.10}$ | $\color{green}{+1.2}$ |
| 1600×1000 | 398 | 3.88 | $\color{green}{2.17}$ | $\color{green}{0.56}$ | 6212 B | 685 B | $\color{green}{0.11}$ | $\color{green}{+1.0}$ |
| 1920×1080 | 391 | 4.20 | $\color{green}{2.28}$ | $\color{green}{0.54}$ | 8533 B | 723 B | $\color{green}{0.09}$ | $\color{green}{+1.2}$ |
| 2048×1536@2 | 344 | 8.38 | $\color{green}{6.06}$ | $\color{green}{0.72}$ | 9496 B | 1821 B | $\color{green}{0.19}$ | $\color{green}{+0.1}$ |
| 2560×1600@2 | 371 | 9.17 | $\color{green}{6.31}$ | $\color{green}{0.69}$ | 7145 B | 2039 B | $\color{green}{0.28}$ | $\color{green}{+0.1}$ |
| 2880×1800@2 | 371 | 9.45 | $\color{green}{6.63}$ | $\color{green}{0.70}$ | 7149 B | 2314 B | $\color{green}{0.32}$ | $\color{green}{+0.1}$ |
| 3200×2000@2 | 331 | 10.6 | $\color{green}{6.70}$ | $\color{green}{0.62}$ | 12 kB | 2839 B | $\color{green}{0.24}$ | $\color{red}{-0.1}$ |
| 3456×2168@2 | 355 | 11.1 | $\color{green}{6.77}$ | $\color{green}{0.61}$ | 10 kB | 2727 B | $\color{green}{0.26}$ | $\color{green}{+0.5}$ |
| 3840×2160@2 | 340 | 10.4 | $\color{green}{7.14}$ | $\color{green}{0.68}$ | 31 kB | 2832 B | $\color{green}{0.09}$ | $\color{green}{+1.5}$ |

### `browser`, a page scrolled

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 206 | 7.54 | $\color{green}{3.60}$ | $\color{green}{0.48}$ | 11 kB | 2276 B | $\color{green}{0.20}$ | $\color{green}{+1.0}$ |
| 1440×900 | 219 | 7.57 | $\color{green}{3.96}$ | $\color{green}{0.53}$ | 16 kB | 2797 B | $\color{green}{0.18}$ | $\color{green}{+1.0}$ |
| 1600×1000 | 223 | 9.09 | $\color{green}{4.82}$ | $\color{green}{0.54}$ | 15 kB | 4177 B | $\color{green}{0.28}$ | $\color{green}{+0.9}$ |
| 1920×1080 | 171 | 10.6 | $\color{green}{5.72}$ | $\color{green}{0.54}$ | 18 kB | 4760 B | $\color{green}{0.27}$ | $\color{green}{+0.9}$ |
| 2048×1536@2 | 186 | 14.9 | $\color{green}{8.07}$ | $\color{green}{0.55}$ | 54 kB | 13 kB | $\color{green}{0.24}$ | $\color{green}{+1.4}$ |
| 2560×1600@2 | 137 | 21.2 | $\color{green}{10.9}$ | $\color{green}{0.51}$ | 74 kB | 11 kB | $\color{green}{0.15}$ | $\color{green}{+1.6}$ |
| 2880×1800@2 | 128 | 25.2 | $\color{green}{12.8}$ | $\color{green}{0.52}$ | 76 kB | 13 kB | $\color{green}{0.17}$ | $\color{green}{+1.2}$ |
| 3200×2000@2 | 85 | 29.8 | $\color{green}{15.1}$ | $\color{green}{0.49}$ | 95 kB | 18 kB | $\color{green}{0.19}$ | $\color{green}{+1.1}$ |
| 3456×2168@2 | 88 | 33.2 | $\color{green}{16.3}$ | $\color{green}{0.49}$ | 97 kB | 20 kB | $\color{green}{0.20}$ | $\color{green}{+1.2}$ |
| 3840×2160@2 | 94 | 33.6 | $\color{green}{17.0}$ | $\color{green}{0.51}$ | 90 kB | 23 kB | $\color{green}{0.25}$ | $\color{green}{+1.3}$ |

### `flood`, a window of new text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 149 | 65.6 | $\color{green}{23.2}$ | $\color{green}{0.36}$ | 676 kB | 563 kB | $\color{green}{0.83}$ | $\color{green}{+0.3}$ |
| 1440×900 | 149 | 48.0 | $\color{green}{23.7}$ | $\color{green}{0.49}$ | 647 kB | 516 kB | $\color{green}{0.80}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 149 | 68.1 | $\color{green}{29.4}$ | $\color{green}{0.45}$ | 934 kB | 688 kB | $\color{green}{0.74}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 149 | 96.7 | $\color{green}{42.6}$ | $\color{green}{0.46}$ | 1423 kB | 1081 kB | $\color{green}{0.76}$ | $\color{green}{+0.4}$ |
| 2048×1536@2 | 149 | 85.8 | $\color{green}{54.7}$ | $\color{green}{0.63}$ | 1368 kB | 1309 kB | $\color{green}{0.96}$ | $\color{green}{+0.6}$ |
| 2560×1600@2 | 149 | 111.8 | $\color{green}{70.3}$ | $\color{green}{0.63}$ | 1799 kB | 1742 kB | $\color{green}{0.97}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 149 | 142.6 | $\color{green}{90.6}$ | $\color{green}{0.63}$ | 2340 kB | 2272 kB | $\color{green}{0.97}$ | $\color{green}{+0.6}$ |
| 3200×2000@2 | 149 | 177.0 | $\color{green}{111.1}$ | $\color{green}{0.63}$ | 2890 kB | 2806 kB | $\color{green}{0.97}$ | $\color{green}{+0.7}$ |
| 3456×2168@2 | 149 | 209.2 | $\color{green}{130.7}$ | $\color{green}{0.62}$ | 3391 kB | 3290 kB | $\color{green}{0.97}$ | $\color{green}{+0.6}$ |
| 3840×2160@2 | 149 | 232.5 | $\color{green}{144.9}$ | $\color{green}{0.62}$ | 3776 kB | 3682 kB | $\color{green}{0.97}$ | $\color{green}{+0.7}$ |

### `uncover`, a window moved off a page

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 53 | 1.49 | $\color{green}{0.34}$ | $\color{green}{0.21}$ | 3557 B | 3292 B | $\color{green}{0.93}$ | $\color{green}{+0.3}$ |
| 1440×900 | 53 | 1.90 | $\color{green}{0.49}$ | $\color{green}{0.26}$ | 3970 B | 3810 B | $\color{green}{0.96}$ | $\color{green}{+0.3}$ |
| 1600×1000 | 54 | 2.09 | $\color{green}{0.60}$ | $\color{green}{0.28}$ | 5212 B | 4506 B | $\color{green}{0.86}$ | $\color{green}{+0.1}$ |
| 1920×1080 | 53 | 2.60 | $\color{green}{0.80}$ | $\color{green}{0.30}$ | 3780 B | 3948 B | $\color{red}{1.04}$ | $\color{red}{-0.1}$ |
| 2048×1536@2 | 54 | 3.40 | $\color{green}{1.03}$ | $\color{green}{0.30}$ | 13 kB | 13 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 54 | 4.58 | $\color{green}{1.39}$ | $\color{green}{0.30}$ | 8406 B | 8390 B | 1.00 | $\color{green}{+0.2}$ |
| 2880×1800@2 | 54 | 5.46 | $\color{green}{1.47}$ | $\color{green}{0.27}$ | 11 kB | 12 kB | $\color{red}{1.07}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 40 | 7.23 | $\color{green}{1.87}$ | $\color{green}{0.26}$ | 20 kB | 21 kB | $\color{red}{1.05}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 54 | 7.84 | $\color{green}{2.12}$ | $\color{green}{0.27}$ | 12 kB | 11 kB | $\color{green}{0.89}$ | $\color{green}{+0.3}$ |
| 3840×2160@2 | 53 | 8.77 | $\color{green}{2.42}$ | $\color{green}{0.28}$ | 15 kB | 16 kB | $\color{red}{1.06}$ | $\color{green}{+0.2}$ |

### `video`, a frame of the pattern

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 483 | 6.73 | $\color{green}{4.10}$ | $\color{green}{0.61}$ | 34 kB | 35 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 1440×900 | 489 | 6.55 | $\color{green}{4.05}$ | $\color{green}{0.62}$ | 35 kB | 36 kB | $\color{red}{1.02}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 487 | 6.49 | $\color{green}{4.21}$ | $\color{green}{0.66}$ | 35 kB | 36 kB | $\color{red}{1.02}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 474 | 5.28 | $\color{green}{3.97}$ | $\color{green}{0.74}$ | 36 kB | 36 kB | 1.00 | $\color{green}{+0.5}$ |
| 2048×1536@2 | 397 | 9.66 | $\color{green}{7.03}$ | $\color{green}{0.71}$ | 65 kB | 67 kB | $\color{red}{1.04}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 309 | 13.0 | $\color{green}{9.16}$ | $\color{green}{0.68}$ | 80 kB | 80 kB | 1.00 | $\color{green}{+0.5}$ |
| 2880×1800@2 | 255 | 16.1 | $\color{green}{11.7}$ | $\color{green}{0.69}$ | 97 kB | 99 kB | $\color{red}{1.02}$ | $\color{green}{+0.7}$ |
| 3200×2000@2 | 190 | 19.2 | $\color{green}{13.5}$ | $\color{green}{0.68}$ | 112 kB | 113 kB | $\color{red}{1.01}$ | $\color{green}{+0.6}$ |
| 3456×2168@2 | 172 | 24.0 | $\color{green}{15.6}$ | $\color{green}{0.68}$ | 122 kB | 128 kB | $\color{red}{1.05}$ | $\color{green}{+0.7}$ |
| 3840×2160@2 | 231 | 15.1 | $\color{green}{10.4}$ | $\color{green}{0.69}$ | 89 kB | 85 kB | $\color{green}{0.95}$ | $\color{green}{+0.5}$ |

### `walk`, a frame as the dial walks

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 239 | 3.99 | $\color{green}{2.69}$ | $\color{green}{0.65}$ | 12 kB | 14 kB | $\color{red}{1.09}$ | $\color{green}{+0.2}$ |
| 1440×900 | 234 | 3.90 | $\color{green}{2.61}$ | $\color{green}{0.66}$ | 13 kB | 14 kB | $\color{red}{1.11}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 232 | 5.38 | $\color{green}{3.62}$ | $\color{green}{0.63}$ | 17 kB | 18 kB | $\color{red}{1.10}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 227 | 3.92 | $\color{green}{2.83}$ | $\color{green}{0.74}$ | 14 kB | 15 kB | $\color{red}{1.08}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 230 | 6.99 | $\color{green}{5.11}$ | $\color{green}{0.73}$ | 27 kB | 30 kB | $\color{red}{1.10}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 196 | 11.5 | $\color{green}{7.75}$ | $\color{green}{0.68}$ | 43 kB | 44 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 195 | 10.5 | $\color{green}{7.22}$ | $\color{green}{0.70}$ | 34 kB | 36 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 3200×2000@2 | 174 | 12.9 | $\color{green}{9.33}$ | $\color{green}{0.70}$ | 46 kB | 46 kB | 1.00 | $\color{green}{+0.7}$ |
| 3456×2168@2 | 128 | 18.5 | $\color{green}{12.6}$ | $\color{green}{0.68}$ | 55 kB | 61 kB | $\color{red}{1.11}$ | $\color{green}{+0.8}$ |
| 3840×2160@2 | 102 | 22.4 | $\color{green}{14.8}$ | $\color{green}{0.67}$ | 68 kB | 72 kB | $\color{red}{1.07}$ | $\color{green}{+1.1}$ |

### `walk`, the settle a session asked for

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 1.71 | $\color{green}{0.75}$ | $\color{green}{0.44}$ | 129 B | 115 B | $\color{green}{0.89}$ | 0.0 |
| 1440×900 | 1 | 1.80 | $\color{green}{0.86}$ | $\color{green}{0.48}$ | 226 B | 178 B | $\color{green}{0.79}$ | 0.0 |
| 1600×1000 | 1 | 1.94 | $\color{green}{1.05}$ | $\color{green}{0.54}$ | 213 B | 137 B | $\color{green}{0.64}$ | 0.0 |
| 1920×1080 | 1 | 2.41 | $\color{green}{1.09}$ | $\color{green}{0.45}$ | 167 B | 142 B | $\color{green}{0.85}$ | 0.0 |
| 2048×1536@2 | 1 | 2.53 | $\color{green}{1.42}$ | $\color{green}{0.56}$ | 216 B | 126 B | $\color{green}{0.58}$ | 0.0 |
| 2880×1800@2 | 1 | 4.46 | $\color{green}{2.39}$ | $\color{green}{0.54}$ | 402 B | 209 B | $\color{green}{0.52}$ | 0.0 |
| 3456×2168@2 | 1 | 6.45 | $\color{green}{3.89}$ | $\color{green}{0.60}$ | 653 B | 203 B | $\color{green}{0.31}$ | 0.0 |
| 3840×2160@2 | 1 | 7.38 | $\color{green}{3.97}$ | $\color{green}{0.54}$ | 741 B | 184 B | $\color{green}{0.25}$ | 0.0 |

### Keyframes inside `uncover`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 23.9 | $\color{green}{11.7}$ | $\color{green}{0.49}$ | 104 kB | 105 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1440×900 | 2 | 19.1 | $\color{green}{10.6}$ | $\color{green}{0.56}$ | 112 kB | 113 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 2 | 25.3 | $\color{green}{13.5}$ | $\color{green}{0.56}$ | 144 kB | 145 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 2 | 31.1 | $\color{green}{15.8}$ | $\color{green}{0.52}$ | 160 kB | 162 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 2 | 31.8 | $\color{green}{20.4}$ | $\color{green}{0.64}$ | 223 kB | 224 kB | 1.00 | $\color{green}{+0.2}$ |
| 2560×1600@2 | 2 | 39.9 | $\color{green}{25.8}$ | $\color{green}{0.65}$ | 273 kB | 275 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2880×1800@2 | 2 | 50.1 | $\color{green}{29.0}$ | $\color{green}{0.58}$ | 295 kB | 296 kB | 1.00 | $\color{green}{+0.2}$ |
| 3200×2000@2 | 2 | 53.2 | $\color{green}{31.5}$ | $\color{green}{0.59}$ | 308 kB | 311 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 2 | 69.2 | $\color{green}{40.7}$ | $\color{green}{0.59}$ | 421 kB | 423 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 2 | 75.2 | $\color{green}{42.2}$ | $\color{green}{0.56}$ | 420 kB | 423 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |

### Keyframes inside `drag`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 14.8 | $\color{green}{7.18}$ | $\color{green}{0.48}$ | 54 kB | 54 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 1440×900 | 2 | 13.7 | $\color{green}{7.56}$ | $\color{green}{0.55}$ | 65 kB | 64 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 2 | 15.8 | $\color{green}{9.39}$ | $\color{green}{0.59}$ | 83 kB | 82 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 2 | 15.8 | $\color{green}{8.71}$ | $\color{green}{0.55}$ | 66 kB | 65 kB | $\color{green}{0.97}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 2 | 18.5 | $\color{green}{10.5}$ | $\color{green}{0.56}$ | 89 kB | 88 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 2 | 26.2 | $\color{green}{14.6}$ | $\color{green}{0.56}$ | 125 kB | 122 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 2880×1800@2 | 2 | 31.1 | $\color{green}{15.2}$ | $\color{green}{0.52}$ | 125 kB | 122 kB | $\color{green}{0.97}$ | $\color{green}{+0.2}$ |
| 3200×2000@2 | 2 | 46.0 | $\color{green}{23.5}$ | $\color{green}{0.51}$ | 203 kB | 200 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 2 | 41.6 | $\color{green}{21.0}$ | $\color{green}{0.52}$ | 166 kB | 161 kB | $\color{green}{0.97}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 2 | 45.5 | $\color{green}{24.1}$ | $\color{green}{0.53}$ | 192 kB | 188 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |

### Keyframes inside `flood`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 14.0 | $\color{green}{6.38}$ | $\color{green}{0.49}$ | 52 kB | 51 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 1440×900 | 2 | 11.9 | $\color{green}{7.10}$ | $\color{green}{0.60}$ | 58 kB | 57 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 2 | 14.6 | $\color{green}{8.98}$ | $\color{green}{0.62}$ | 80 kB | 79 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 2 | 16.9 | $\color{green}{9.74}$ | $\color{green}{0.57}$ | 91 kB | 90 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 2 | 23.4 | $\color{green}{12.6}$ | $\color{green}{0.54}$ | 107 kB | 107 kB | 1.00 | $\color{green}{+0.2}$ |
| 2560×1600@2 | 2 | 27.8 | $\color{green}{13.8}$ | $\color{green}{0.50}$ | 118 kB | 115 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 2880×1800@2 | 2 | 31.5 | $\color{green}{16.0}$ | $\color{green}{0.52}$ | 131 kB | 128 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 3200×2000@2 | 2 | 41.6 | $\color{green}{21.9}$ | $\color{green}{0.53}$ | 182 kB | 180 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 2 | 48.5 | $\color{green}{25.5}$ | $\color{green}{0.53}$ | 207 kB | 205 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 2 | 47.0 | $\color{green}{25.4}$ | $\color{green}{0.54}$ | 205 kB | 203 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |

### Keyframes inside `video`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 3 | 7.81 | $\color{green}{4.97}$ | $\color{green}{0.50}$ | 49 kB | 45 kB | $\color{green}{0.93}$ | $\color{green}{+0.3}$ |
| 1440×900 | 3 | 7.43 | $\color{green}{5.08}$ | $\color{green}{0.53}$ | 51 kB | 46 kB | $\color{green}{0.91}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 3 | 7.47 | $\color{green}{5.44}$ | $\color{green}{0.54}$ | 51 kB | 47 kB | $\color{green}{0.91}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 3 | 7.95 | $\color{green}{5.71}$ | $\color{green}{0.53}$ | 51 kB | 47 kB | $\color{green}{0.91}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 3 | 10.9 | $\color{green}{9.14}$ | $\color{green}{0.51}$ | 104 kB | 97 kB | $\color{green}{0.93}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 3 | 13.7 | $\color{green}{11.6}$ | $\color{green}{0.49}$ | 115 kB | 104 kB | $\color{green}{0.90}$ | $\color{green}{+0.2}$ |
| 2880×1800@2 | 3 | 16.0 | $\color{green}{13.0}$ | $\color{green}{0.50}$ | 120 kB | 109 kB | $\color{green}{0.91}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 3 | 18.6 | $\color{green}{14.0}$ | $\color{green}{0.50}$ | 125 kB | 113 kB | $\color{green}{0.91}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 3 | 21.2 | $\color{green}{14.5}$ | $\color{green}{0.49}$ | 132 kB | 117 kB | $\color{green}{0.89}$ | $\color{green}{+0.3}$ |
| 3840×2160@2 | 3 | 18.6 | $\color{green}{14.1}$ | $\color{green}{0.51}$ | 116 kB | 105 kB | $\color{green}{0.90}$ | $\color{green}{+0.2}$ |

## Against libvpx, keyframes of each capture's first picture

### A keyframe at dial 40: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.67}$ | $\color{green}{0.75}$ | $\color{green}{0.72}$ | $\color{green}{0.67}$ | $\color{green}{0.81}$ | $\color{green}{0.80}$ | $\color{green}{0.78}$ | $\color{green}{0.78}$ | $\color{green}{0.78}$ | $\color{green}{0.77}$ |
| `flood` | $\color{green}{0.62}$ | $\color{green}{0.78}$ | $\color{green}{0.82}$ | $\color{green}{0.75}$ | $\color{green}{0.82}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.78}$ | $\color{green}{0.81}$ | $\color{green}{0.82}$ |
| `browser` | $\color{green}{0.53}$ | $\color{green}{0.62}$ | $\color{green}{0.60}$ | $\color{green}{0.60}$ | $\color{green}{0.63}$ | $\color{green}{0.61}$ | $\color{green}{0.62}$ | $\color{green}{0.59}$ | $\color{green}{0.60}$ | $\color{green}{0.59}$ |
| `drag` | $\color{green}{0.67}$ | $\color{green}{0.78}$ | $\color{green}{0.80}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ | $\color{green}{0.80}$ | $\color{green}{0.78}$ | $\color{green}{0.78}$ | $\color{green}{0.78}$ | $\color{green}{0.83}$ |
| `video` | $\color{green}{0.71}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.66}$ | $\color{green}{0.83}$ | $\color{green}{0.82}$ | $\color{green}{0.80}$ | $\color{green}{0.74}$ | $\color{green}{0.78}$ | $\color{green}{0.75}$ |
| `walk` | $\color{green}{0.66}$ | $\color{green}{0.72}$ | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.90}$ | $\color{green}{0.86}$ | $\color{green}{0.91}$ | $\color{green}{0.89}$ | $\color{green}{0.92}$ | $\color{green}{0.83}$ |

### A keyframe at dial 40: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ |
| `flood` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | 1.00 |
| `browser` | $\color{green}{0.32}$ | $\color{green}{0.38}$ | $\color{green}{0.25}$ | $\color{green}{0.23}$ | $\color{green}{0.26}$ | $\color{green}{0.25}$ | $\color{green}{0.29}$ | $\color{green}{0.27}$ | $\color{green}{0.22}$ | $\color{green}{0.24}$ |
| `drag` | $\color{red}{1.01}$ | 1.00 | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.98}$ |
| `video` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 |
| `walk` | $\color{green}{0.73}$ | $\color{green}{0.68}$ | $\color{green}{0.68}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ | $\color{green}{0.67}$ | $\color{green}{0.67}$ |

### A keyframe at dial 40: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `browser` | $\color{green}{+30.0}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ |
| `drag` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.8}$ | $\color{green}{+0.5}$ | $\color{green}{+0.3}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+1.0}$ |

### A keyframe at dial 65: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.57}$ | $\color{green}{0.66}$ | $\color{green}{0.63}$ | $\color{green}{0.58}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.72}$ | $\color{green}{0.73}$ | $\color{green}{0.72}$ | $\color{green}{0.70}$ |
| `flood` | $\color{green}{0.56}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ | $\color{green}{0.67}$ | $\color{green}{0.79}$ | $\color{green}{0.72}$ | $\color{green}{0.73}$ | $\color{green}{0.72}$ | $\color{green}{0.75}$ | $\color{green}{0.75}$ |
| `browser` | $\color{green}{0.53}$ | $\color{green}{0.62}$ | $\color{green}{0.60}$ | $\color{green}{0.61}$ | $\color{green}{0.60}$ | $\color{green}{0.56}$ | $\color{green}{0.58}$ | $\color{green}{0.57}$ | $\color{green}{0.58}$ | $\color{green}{0.59}$ |
| `drag` | $\color{green}{0.57}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ | $\color{green}{0.66}$ | $\color{green}{0.72}$ | $\color{green}{0.74}$ | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.70}$ | $\color{green}{0.74}$ |
| `video` | $\color{green}{0.58}$ | $\color{green}{0.63}$ | $\color{green}{0.66}$ | $\color{green}{0.56}$ | $\color{green}{0.78}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.65}$ | $\color{green}{0.66}$ | $\color{green}{0.64}$ |
| `walk` | $\color{green}{0.67}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ | $\color{green}{0.74}$ | $\color{green}{0.82}$ | $\color{green}{0.83}$ | $\color{green}{0.83}$ | $\color{green}{0.81}$ | $\color{green}{0.81}$ | $\color{green}{0.75}$ |

### A keyframe at dial 65: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | 1.00 |
| `browser` | $\color{green}{0.31}$ | $\color{green}{0.36}$ | $\color{green}{0.25}$ | $\color{green}{0.23}$ | $\color{green}{0.25}$ | $\color{green}{0.24}$ | $\color{green}{0.29}$ | $\color{green}{0.26}$ | $\color{green}{0.22}$ | $\color{green}{0.23}$ |
| `drag` | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `video` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 |
| `walk` | $\color{green}{0.83}$ | $\color{green}{0.79}$ | $\color{green}{0.78}$ | $\color{green}{0.78}$ | $\color{green}{0.77}$ | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.79}$ | $\color{green}{0.74}$ | $\color{green}{0.74}$ |

### A keyframe at dial 65: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `flood` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.7}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |

### A keyframe at dial 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.50}$ | $\color{green}{0.58}$ | $\color{green}{0.57}$ | $\color{green}{0.53}$ | $\color{green}{0.71}$ | $\color{green}{0.69}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.67}$ | $\color{green}{0.66}$ |
| `flood` | $\color{green}{0.50}$ | $\color{green}{0.66}$ | $\color{green}{0.66}$ | $\color{green}{0.62}$ | $\color{green}{0.75}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ | $\color{green}{0.72}$ |
| `browser` | $\color{green}{0.53}$ | $\color{green}{0.63}$ | $\color{green}{0.59}$ | $\color{green}{0.61}$ | $\color{green}{0.60}$ | $\color{green}{0.57}$ | $\color{green}{0.57}$ | $\color{green}{0.57}$ | $\color{green}{0.58}$ | $\color{green}{0.59}$ |
| `drag` | $\color{green}{0.53}$ | $\color{green}{0.64}$ | $\color{green}{0.66}$ | $\color{green}{0.60}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.67}$ | $\color{green}{0.71}$ |
| `video` | $\color{green}{0.50}$ | $\color{green}{0.55}$ | $\color{green}{0.59}$ | $\color{green}{0.50}$ | $\color{green}{0.71}$ | $\color{green}{0.65}$ | $\color{green}{0.66}$ | $\color{green}{0.60}$ | $\color{green}{0.63}$ | $\color{green}{0.61}$ |
| `walk` | $\color{green}{0.66}$ | $\color{green}{0.70}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.83}$ | $\color{green}{0.81}$ | $\color{green}{0.82}$ | $\color{green}{0.77}$ |

### A keyframe at dial 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `browser` | $\color{green}{0.31}$ | $\color{green}{0.36}$ | $\color{green}{0.24}$ | $\color{green}{0.23}$ | $\color{green}{0.24}$ | $\color{green}{0.24}$ | $\color{green}{0.29}$ | $\color{green}{0.26}$ | $\color{green}{0.22}$ | $\color{green}{0.23}$ |
| `drag` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ |
| `video` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | 1.00 | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `walk` | $\color{green}{0.84}$ | $\color{green}{0.81}$ | $\color{green}{0.81}$ | $\color{green}{0.81}$ | $\color{green}{0.80}$ | $\color{green}{0.77}$ | $\color{green}{0.79}$ | $\color{green}{0.79}$ | $\color{green}{0.77}$ | $\color{green}{0.78}$ |

### A keyframe at dial 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ |
| `walk` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |

## Against libvpx, settles of each capture's first picture

### A settle from dial 20 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.53}$ | $\color{green}{0.65}$ | $\color{green}{0.62}$ | $\color{green}{0.62}$ | $\color{green}{0.85}$ | $\color{green}{0.81}$ | $\color{green}{0.79}$ | $\color{green}{0.84}$ | $\color{green}{0.80}$ | $\color{green}{0.77}$ |
| `flood` | $\color{green}{0.56}$ | $\color{green}{0.76}$ | $\color{green}{0.78}$ | $\color{green}{0.76}$ | $\color{green}{0.93}$ | $\color{green}{0.90}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.94}$ | 1.00 |
| `browser` | $\color{green}{0.67}$ | $\color{green}{0.81}$ | $\color{green}{0.78}$ | $\color{green}{0.83}$ | $\color{green}{0.93}$ | $\color{green}{0.88}$ | $\color{green}{0.93}$ | $\color{green}{0.92}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ |
| `drag` | $\color{green}{0.58}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.94}$ | $\color{green}{0.93}$ | $\color{green}{0.95}$ | $\color{green}{0.89}$ | $\color{green}{0.89}$ | $\color{green}{0.94}$ |
| `video` | $\color{green}{0.57}$ | $\color{green}{0.66}$ | $\color{green}{0.71}$ | $\color{green}{0.64}$ | $\color{green}{0.93}$ | $\color{green}{0.84}$ | $\color{green}{0.88}$ | $\color{green}{0.81}$ | $\color{green}{0.88}$ | $\color{green}{0.87}$ |
| `walk` | $\color{green}{0.67}$ | $\color{green}{0.76}$ | $\color{green}{0.77}$ | $\color{green}{0.91}$ | $\color{green}{0.96}$ | $\color{red}{1.01}$ | $\color{green}{0.96}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |

### A settle from dial 20 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | 1.00 |
| `browser` | $\color{red}{4.64}$ | $\color{red}{4.04}$ | $\color{red}{4.62}$ | $\color{red}{5.38}$ | $\color{red}{5.48}$ | $\color{red}{6.55}$ | $\color{red}{7.61}$ | $\color{red}{8.60}$ | $\color{red}{9.04}$ | $\color{red}{9.62}$ |
| `drag` | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ |
| `video` | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ |
| `walk` | $\color{green}{0.88}$ | $\color{green}{0.86}$ | $\color{green}{0.87}$ | $\color{green}{0.87}$ | $\color{green}{0.88}$ | $\color{green}{0.85}$ | $\color{green}{0.85}$ | $\color{green}{0.85}$ | $\color{green}{0.84}$ | $\color{green}{0.89}$ |

### A settle from dial 20 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `flood` | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.1}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `walk` | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ |

### A settle from dial 65 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.01}$ | $\color{red}{1.15}$ | $\color{red}{1.11}$ | $\color{red}{1.10}$ | $\color{red}{1.42}$ | $\color{red}{1.31}$ | $\color{red}{1.29}$ | $\color{red}{1.33}$ | $\color{red}{1.25}$ | $\color{red}{1.20}$ |
| `flood` | $\color{red}{1.02}$ | $\color{red}{1.26}$ | $\color{red}{1.28}$ | $\color{red}{1.25}$ | $\color{red}{1.41}$ | $\color{red}{1.32}$ | $\color{red}{1.32}$ | $\color{red}{1.29}$ | $\color{red}{1.33}$ | $\color{red}{1.33}$ |
| `browser` | $\color{green}{0.35}$ | $\color{green}{0.43}$ | $\color{green}{0.48}$ | $\color{green}{0.47}$ | $\color{green}{0.55}$ | $\color{green}{0.53}$ | $\color{green}{0.65}$ | $\color{green}{0.52}$ | $\color{green}{0.61}$ | $\color{green}{0.64}$ |
| `drag` | 1.00 | $\color{red}{1.23}$ | $\color{red}{1.28}$ | $\color{red}{1.11}$ | $\color{red}{1.31}$ | $\color{red}{1.34}$ | $\color{red}{1.33}$ | $\color{red}{1.31}$ | $\color{red}{1.24}$ | $\color{red}{1.28}$ |
| `video` | $\color{green}{0.90}$ | $\color{red}{1.03}$ | $\color{green}{0.99}$ | $\color{green}{0.93}$ | $\color{red}{1.31}$ | $\color{red}{1.07}$ | $\color{red}{1.07}$ | $\color{green}{0.95}$ | $\color{green}{0.91}$ | $\color{green}{0.87}$ |
| `walk` | $\color{green}{0.96}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.09}$ | $\color{red}{1.25}$ | $\color{red}{1.31}$ | $\color{red}{1.31}$ | $\color{red}{1.23}$ | $\color{red}{1.28}$ | $\color{red}{1.11}$ |

### A settle from dial 65 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | 1.00 | 1.00 | 1.00 | $\color{green}{0.97}$ | 1.00 | $\color{green}{0.99}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ |
| `browser` | $\color{red}{1.22}$ | $\color{red}{1.37}$ | $\color{red}{1.04}$ | $\color{red}{1.15}$ | $\color{red}{1.12}$ | $\color{red}{1.22}$ | $\color{red}{1.61}$ | $\color{red}{1.45}$ | $\color{red}{1.40}$ | $\color{red}{1.42}$ |
| `drag` | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ |
| `video` | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.98}$ | $\color{green}{0.95}$ | $\color{green}{0.99}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | 1.00 | $\color{green}{0.93}$ | $\color{green}{0.94}$ |
| `walk` | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ |

### A settle from dial 65 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+1.3}$ | $\color{green}{+1.2}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.2}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `flood` | $\color{green}{+1.1}$ | $\color{green}{+1.2}$ | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.7}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ |
| `video` | $\color{green}{+1.0}$ | $\color{green}{+0.6}$ | $\color{green}{+1.1}$ | $\color{green}{+0.8}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `walk` | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |

### A settle from dial 80 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.10}$ | $\color{red}{1.20}$ | $\color{red}{1.11}$ | $\color{red}{1.06}$ | $\color{red}{1.42}$ | $\color{red}{1.35}$ | $\color{red}{1.30}$ | $\color{red}{1.21}$ | $\color{red}{1.26}$ | $\color{red}{1.22}$ |
| `flood` | $\color{red}{1.06}$ | $\color{red}{1.17}$ | $\color{red}{1.19}$ | $\color{red}{1.16}$ | $\color{red}{1.27}$ | $\color{red}{1.19}$ | $\color{red}{1.23}$ | $\color{red}{1.15}$ | $\color{red}{1.16}$ | $\color{red}{1.16}$ |
| `browser` | $\color{green}{0.35}$ | $\color{green}{0.45}$ | $\color{green}{0.47}$ | $\color{green}{0.48}$ | $\color{green}{0.50}$ | $\color{green}{0.52}$ | $\color{green}{0.60}$ | $\color{green}{0.48}$ | $\color{green}{0.55}$ | $\color{green}{0.63}$ |
| `drag` | $\color{red}{1.08}$ | $\color{red}{1.19}$ | $\color{red}{1.21}$ | $\color{red}{1.09}$ | $\color{red}{1.19}$ | $\color{red}{1.21}$ | $\color{red}{1.16}$ | $\color{red}{1.19}$ | $\color{red}{1.08}$ | $\color{red}{1.15}$ |
| `video` | $\color{green}{0.96}$ | $\color{red}{1.10}$ | $\color{red}{1.04}$ | $\color{green}{0.86}$ | $\color{red}{1.11}$ | $\color{red}{1.08}$ | $\color{red}{1.01}$ | $\color{green}{0.89}$ | $\color{green}{0.86}$ | $\color{green}{0.86}$ |
| `walk` | 1.00 | $\color{red}{1.10}$ | $\color{red}{1.06}$ | $\color{red}{1.08}$ | $\color{red}{1.23}$ | $\color{red}{1.33}$ | $\color{red}{1.35}$ | $\color{red}{1.29}$ | $\color{red}{1.29}$ | $\color{red}{1.10}$ |

### A settle from dial 80 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.78}$ | $\color{green}{0.80}$ | $\color{green}{0.81}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.72}$ | $\color{green}{0.67}$ | $\color{green}{0.73}$ | $\color{green}{0.73}$ |
| `flood` | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.65}$ | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.77}$ | $\color{green}{0.80}$ | $\color{green}{0.74}$ | $\color{green}{0.75}$ | $\color{green}{0.74}$ |
| `browser` | $\color{red}{1.14}$ | $\color{red}{1.49}$ | $\color{red}{1.08}$ | $\color{red}{1.29}$ | $\color{red}{1.30}$ | $\color{red}{1.47}$ | $\color{red}{2.05}$ | $\color{red}{1.81}$ | $\color{red}{1.73}$ | $\color{red}{1.76}$ |
| `drag` | $\color{green}{0.76}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.73}$ | $\color{green}{0.78}$ | $\color{green}{0.76}$ | $\color{green}{0.72}$ | $\color{green}{0.77}$ | $\color{green}{0.73}$ | $\color{green}{0.78}$ |
| `video` | $\color{green}{0.86}$ | $\color{green}{0.86}$ | $\color{green}{0.93}$ | $\color{green}{0.64}$ | $\color{green}{0.72}$ | $\color{green}{0.81}$ | $\color{green}{0.76}$ | $\color{green}{0.81}$ | $\color{green}{0.72}$ | $\color{green}{0.77}$ |
| `walk` | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.94}$ | $\color{green}{0.93}$ | $\color{green}{0.89}$ | $\color{green}{0.93}$ | $\color{green}{0.90}$ | $\color{green}{0.93}$ |

### A settle from dial 80 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `flood` | $\color{red}{-0.1}$ | 0.0 | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.1}$ | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ |
| `video` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{red}{-0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `walk` | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ |

### A settle from dial 89 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.24}$ | $\color{red}{1.28}$ | $\color{red}{1.23}$ | $\color{red}{1.21}$ | $\color{red}{1.28}$ | $\color{red}{1.18}$ | $\color{red}{1.17}$ | $\color{red}{1.15}$ | $\color{red}{1.13}$ | $\color{red}{1.11}$ |
| `flood` | $\color{red}{1.09}$ | $\color{red}{1.10}$ | $\color{red}{1.11}$ | $\color{red}{1.16}$ | $\color{red}{1.14}$ | $\color{red}{1.10}$ | $\color{red}{1.10}$ | $\color{red}{1.06}$ | $\color{red}{1.07}$ | $\color{red}{1.05}$ |
| `browser` | $\color{green}{0.35}$ | $\color{green}{0.44}$ | $\color{green}{0.50}$ | $\color{green}{0.48}$ | $\color{green}{0.48}$ | $\color{green}{0.52}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ | $\color{green}{0.63}$ |
| `drag` | $\color{red}{1.03}$ | $\color{red}{1.18}$ | $\color{red}{1.20}$ | $\color{red}{1.07}$ | $\color{red}{1.06}$ | $\color{red}{1.07}$ | $\color{red}{1.12}$ | $\color{red}{1.09}$ | $\color{red}{1.01}$ | $\color{red}{1.03}$ |
| `video` | $\color{green}{0.86}$ | $\color{green}{0.91}$ | $\color{green}{0.86}$ | $\color{green}{0.81}$ | $\color{green}{0.99}$ | $\color{green}{0.90}$ | $\color{green}{0.91}$ | $\color{green}{0.81}$ | $\color{green}{0.79}$ | $\color{green}{0.78}$ |
| `walk` | $\color{red}{1.10}$ | $\color{red}{1.14}$ | $\color{red}{1.07}$ | $\color{red}{1.05}$ | $\color{red}{1.24}$ | $\color{red}{1.34}$ | $\color{red}{1.35}$ | $\color{red}{1.29}$ | $\color{red}{1.31}$ | $\color{red}{1.07}$ |

### A settle from dial 89 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.67}$ | $\color{green}{0.82}$ | $\color{green}{0.68}$ | $\color{green}{0.75}$ | $\color{green}{0.49}$ | $\color{green}{0.45}$ | $\color{green}{0.44}$ | $\color{green}{0.63}$ | $\color{green}{0.45}$ | $\color{green}{0.50}$ |
| `flood` | $\color{green}{0.85}$ | $\color{green}{0.76}$ | $\color{green}{0.70}$ | $\color{green}{0.86}$ | $\color{green}{0.65}$ | $\color{green}{0.94}$ | $\color{green}{0.99}$ | $\color{green}{0.74}$ | $\color{green}{0.91}$ | $\color{green}{0.83}$ |
| `browser` | $\color{red}{1.14}$ | $\color{red}{1.49}$ | $\color{red}{1.08}$ | $\color{red}{1.29}$ | $\color{red}{1.30}$ | $\color{red}{1.47}$ | $\color{red}{2.05}$ | $\color{red}{1.81}$ | $\color{red}{1.73}$ | $\color{red}{1.76}$ |
| `drag` | $\color{green}{0.93}$ | $\color{green}{0.74}$ | $\color{green}{0.84}$ | $\color{green}{0.96}$ | $\color{green}{0.87}$ | $\color{green}{0.71}$ | $\color{red}{1.09}$ | $\color{green}{0.80}$ | $\color{green}{0.82}$ | $\color{green}{0.81}$ |
| `video` | $\color{green}{0.58}$ | $\color{green}{0.56}$ | $\color{green}{0.51}$ | $\color{green}{0.63}$ | $\color{green}{0.53}$ | $\color{green}{0.54}$ | $\color{green}{0.55}$ | $\color{green}{0.60}$ | $\color{green}{0.56}$ | $\color{green}{0.54}$ |
| `walk` | $\color{red}{1.10}$ | $\color{red}{1.07}$ | $\color{red}{1.06}$ | $\color{red}{1.09}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.89}$ | $\color{green}{0.95}$ | $\color{red}{1.01}$ | $\color{green}{0.86}$ |

### A settle from dial 89 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `walk` | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ |

