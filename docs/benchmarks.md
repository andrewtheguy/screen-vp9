# What the encoder was measured at

The numbers the encoder's decisions were taken on. How the encoder works is
in [encoder.md](encoder.md); what was built in which order, and why, in
[plan.md](plan.md).

Everything here is against libvpx as `screen-vp9` 0.0.14 sets it up
(`src/reference.rs`, which only the tests compile), by the three
measurements in `src/encoder/mod.rs`: `captures_against_libvpx` plays each
capture into both encoders, frame by frame, turn about;
`keyframes_against_libvpx` codes each capture's first picture as a keyframe
at dials 40, 65 and 90; `settles_against_libvpx` codes it at one dial and
settles it at 90, the picture unchanged. The last two keep the least time of
five passes. `tools/tables.py` makes the tables below from their logs.

They were run on 2026-10-10 on a six-core Intel i5-8500T (AVX2) under WSL2,
four threads, the machine idle and one measurement at a time, on the encoder
of commit `c3c3616`. A time on this machine moves by a tenth from one run to
the next, so a ratio within a tenth of 1.00 is on the line. The tables this
file held before were made on an Apple M2 Max, where this encoder stood
better against libvpx than it does here: the two are not to be read against
each other.

Throughout, a *ratio* is this encoder over libvpx, so under 1.00 is fewer
bytes or less time; a *PSNR difference* is this encoder's luma PSNR less
libvpx's at the same dial, so above zero is a better picture. `@2` marks a
desktop drawn at scale 2. Green is this encoder's side of the line and red
the other; a number on the line is left plain.

## What the tables say

- **Over a whole capture the picture is libvpx's or better in 68 cells of
  70**, the two under being `terminal` and `uncover` at 1920×1080 (−0.1,
  −0.3 dB): `browser` +0.7 to +1.6 dB at 0.15–0.28× the bytes, `drag` 0.0
  to +1.4 at 0.12–0.39×, `flood` +0.2 to +0.5 at 0.74–0.97×, `terminal`
  0.0 to +0.3 at 0.76–0.86×, `video` +0.4 to +0.7 at 0.94–1.03×, `walk`
  0.0 to +1.0 at 0.98–1.09×, `uncover` at 0.94–1.04×.
- **Every capture at every size takes less time than libvpx**, frame for
  frame and summed: a line of text 0.30–0.79 (about 2 ms at 1280×800 and 3
  at 3840×2160, where libvpx takes 2.7 and 10.6), `uncover` 0.39–0.53,
  `flood` 0.41–0.60, `browser` 0.54–0.60, `drag` 0.58–0.80, `video`
  0.68–0.87, `walk` 0.72–0.93. `video` and `walk` are the nearest the line,
  and nearer it from 1920 wide up.
- **A keyframe inside a capture takes 0.46 to 1.13 of libvpx's time** at
  0.97–1.01× the bytes and 0.0 to −0.1 dB (`video`'s 0.88–0.93× the bytes);
  four cells of forty are over 1.00, all from 2048 wide up.
- **A first picture coded alone is where this encoder is not ahead.** Up to
  1920 wide a keyframe takes 0.66–1.04 of libvpx's time (two cells of 72
  are 1.2); from 2048 wide, in four tile columns and without the loop
  filter, 0.87–1.27, the worst at dial 40 and on `walk`'s small picture.
  Bytes are 0.96–1.02× (`walk` 0.64–0.82×, the flat desktop of `browser`
  0.22–0.38×) at −0.3 to +0.2 dB at dial 90 and mostly above zero at 65.
  `browser`'s +30 dB at dial 40 is a flat desktop this encoder holds exactly.
- **A settle from 65 to 90 is 0.93 to 1.34 of libvpx's time on a busy
  picture** (`uncover`, `flood`, `drag`), for 1.2 to 1.5 dB more picture at
  1.00–1.05× the bytes: this encoder's settle lands nearer the source. From
  20 it is 0.57–0.92, from 89 0.26–0.93 past 1280×800.
- **What is over the line in bytes**: a settle of one dial step, 89 to 90,
  at 0.8–1.7× for −0.1 to −0.4 dB (a few kilobytes), and `walk`'s there at
  1.2–1.6× for +0.3 to +0.6; a flat desktop settled from dial 20 at 4 to
  10× (one or two kilobytes against a few hundred bytes); `walk`'s captures
  at up to 1.09×.
- **1280×800 settles and lines of text are slower against libvpx than the
  sizes beside them** (a settle from 80 or 89 up to 1.45, a line 0.79):
  why has not been looked into.

## What changed since release 0.0.1-alpha.2

Measured on the same machine before and after, each alone.

| Change | What it bought | Bytes |
|---|---|---|
| A transform block whose prediction is the source is not transformed or quantized, and the loop filter passes an edge that is one value across | a flat 1920×1080 keyframe 12.9 → 7.9 ms, a flat 3840×2160 one 37 → 25 ms, a busy keyframe 5–10% | the same |
| The token loops walk one table of position, band and neighbours; the luma a 4×4 trial coded is not coded again | with the row above, a busy 1920×1080 keyframe on one thread 165 → 113 ms | the same to the printed digit |
| A busy frame is in tile rows, two or four by what its heaviest tile column recorded, so its bools are not coded by one thread a column | coding the bools of a busy keyframe 17.6 → 7.7 ms at 1920×1080 (the frame 47 → 36), 33.5 → 19.7 at 3840×2160; `flood` frames 0.56 → 0.47 of libvpx's time at 1920×1080 | +0.02% |
| A 4×4 block is tried in TM beside DC, V and H | +0.16 to +0.18 dB on the captures' keyframes at 1920×1080 for 3% of their time; over the whole captures +0.1 dB in the same time | −0.1% |

Against libvpx, the release's cells and this commit's at the four sizes both
were run at (1280×800, 1920×1080, 2560×1600, 3840×2160):

| | Release | Now |
|---|---:|---:|
| A keyframe of a busy first picture, time | 0.9–1.5 | 0.66–1.2 |
| `flood`, time frame for frame | 0.55–0.61 | 0.41–0.58 |
| `uncover`, time frame for frame | 0.39–0.49 | 0.40–0.53 |
| `drag`, time frame for frame | 0.63–0.85 | 0.58–0.80 |
| `video`, time frame for frame | 0.73–0.89 | 0.68–0.85 |
| A keyframe of a busy first picture, luma PSNR | −0.05 to −0.37 dB | −0.3 to +0.2 dB |

### Measured again and left as they were

Each over the seven captures at 1920×1080, this encoder alone, 184 MB and
50.69 dB summed and averaged at the constants kept:

| Constant | Tried | Bytes | PSNR, dB | Time |
|---|---|---:|---:|---:|
| Rate weight, 35 | 25 | +1.0% | +0.07 | the same |
| Intra bits, 24 | 12, 48 | 0.0% | −0.02, −0.01 | +2% |
| New vector's gain, 4 | 2 | −0.7% | −0.10 | +5% |
| | 8 | +0.4% | +0.14 | +8% |
| Inter split, 1 | 2 | −1.1% | −0.03 | the same |
| | 4 | −1.7% | −0.05 | −3% |
| A 64×64 inter block | allowed | −0.1% | −0.34 | −7% |

On the keyframes of the five busy captures at 1920×1080, dials 40 and 90:

| Constant | Tried | What it did |
|---|---|---|
| Rate weight, 35 | 25, 50 | bytes for decibels along one curve |
| Split variance, 8 | 4, 16 | nothing |
| 4×4 modes, DC V H TM | all ten, pruned | +0.3 dB more for 20–25% more time |
| 4×4 split, 80 steps | 10 | +0.1 to +0.2 dB for 1% fewer bytes in the same time; a hatch is then coded without a diagonal mode and the quality walk does not bring a churning picture under its link (two tests), so it is open, not taken |
| Intra modes ranked by prediction error, the best few coded | 8×8 and 4×4 | slower (132 ms against 125 summed) and −0.3 dB |

## Against libvpx, every capture at every size

### Bytes over the whole capture, this encoder over libvpx

Under 1.00 is fewer bytes than libvpx.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.85}$ | $\color{green}{0.83}$ | $\color{green}{0.83}$ | $\color{green}{0.84}$ | $\color{green}{0.76}$ | $\color{green}{0.77}$ | $\color{green}{0.78}$ | $\color{green}{0.86}$ | $\color{green}{0.86}$ | $\color{green}{0.85}$ |
| `uncover` | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.94}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{green}{0.96}$ | $\color{red}{1.03}$ |
| `flood` | $\color{green}{0.84}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.76}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ |
| `browser` | $\color{green}{0.20}$ | $\color{green}{0.17}$ | $\color{green}{0.25}$ | $\color{green}{0.28}$ | $\color{green}{0.24}$ | $\color{green}{0.15}$ | $\color{green}{0.17}$ | $\color{green}{0.20}$ | $\color{green}{0.22}$ | $\color{green}{0.26}$ |
| `drag` | $\color{green}{0.14}$ | $\color{green}{0.15}$ | $\color{green}{0.17}$ | $\color{green}{0.12}$ | $\color{green}{0.23}$ | $\color{green}{0.31}$ | $\color{green}{0.39}$ | $\color{green}{0.30}$ | $\color{green}{0.32}$ | $\color{green}{0.12}$ |
| `video` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{green}{0.99}$ | $\color{red}{1.02}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.99}$ | $\color{red}{1.03}$ | $\color{green}{0.94}$ |
| `walk` | $\color{red}{1.07}$ | $\color{red}{1.09}$ | $\color{red}{1.08}$ | $\color{red}{1.05}$ | $\color{red}{1.07}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{red}{1.08}$ | $\color{red}{1.05}$ |

### Encode time, the median of the per-frame ratios

Under 1.00 is faster than libvpx, frame for frame.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.79}$ | $\color{green}{0.57}$ | $\color{green}{0.54}$ | $\color{green}{0.46}$ | $\color{green}{0.51}$ | $\color{green}{0.45}$ | $\color{green}{0.38}$ | $\color{green}{0.34}$ | $\color{green}{0.32}$ | $\color{green}{0.30}$ |
| `uncover` | $\color{green}{0.53}$ | $\color{green}{0.46}$ | $\color{green}{0.46}$ | $\color{green}{0.40}$ | $\color{green}{0.46}$ | $\color{green}{0.42}$ | $\color{green}{0.41}$ | $\color{green}{0.39}$ | $\color{green}{0.41}$ | $\color{green}{0.40}$ |
| `flood` | $\color{green}{0.41}$ | $\color{green}{0.51}$ | $\color{green}{0.48}$ | $\color{green}{0.47}$ | $\color{green}{0.60}$ | $\color{green}{0.58}$ | $\color{green}{0.57}$ | $\color{green}{0.57}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ |
| `browser` | $\color{green}{0.54}$ | $\color{green}{0.58}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.60}$ | $\color{green}{0.57}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.57}$ |
| `drag` | $\color{green}{0.61}$ | $\color{green}{0.62}$ | $\color{green}{0.59}$ | $\color{green}{0.58}$ | $\color{green}{0.79}$ | $\color{green}{0.80}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ |
| `video` | $\color{green}{0.68}$ | $\color{green}{0.71}$ | $\color{green}{0.74}$ | $\color{green}{0.83}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ | $\color{green}{0.84}$ | $\color{green}{0.85}$ | $\color{green}{0.85}$ | $\color{green}{0.84}$ |
| `walk` | $\color{green}{0.72}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.84}$ | $\color{green}{0.93}$ | $\color{green}{0.87}$ | $\color{green}{0.86}$ | $\color{green}{0.85}$ | $\color{green}{0.85}$ | $\color{green}{0.83}$ |

### Encode time, the whole capture's

Every frame's time summed, this encoder's over libvpx's: where the long frames weigh.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.73}$ | $\color{green}{0.58}$ | $\color{green}{0.54}$ | $\color{green}{0.50}$ | $\color{green}{0.53}$ | $\color{green}{0.49}$ | $\color{green}{0.48}$ | $\color{green}{0.38}$ | $\color{green}{0.36}$ | $\color{green}{0.35}$ |
| `uncover` | $\color{green}{0.55}$ | $\color{green}{0.54}$ | $\color{green}{0.52}$ | $\color{green}{0.48}$ | $\color{green}{0.60}$ | $\color{green}{0.54}$ | $\color{green}{0.51}$ | $\color{green}{0.54}$ | $\color{green}{0.64}$ | $\color{green}{0.51}$ |
| `flood` | $\color{green}{0.42}$ | $\color{green}{0.51}$ | $\color{green}{0.48}$ | $\color{green}{0.48}$ | $\color{green}{0.60}$ | $\color{green}{0.58}$ | $\color{green}{0.58}$ | $\color{green}{0.57}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ |
| `browser` | $\color{green}{0.54}$ | $\color{green}{0.57}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.59}$ | $\color{green}{0.57}$ | $\color{green}{0.55}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ | $\color{green}{0.56}$ |
| `drag` | $\color{green}{0.62}$ | $\color{green}{0.61}$ | $\color{green}{0.59}$ | $\color{green}{0.58}$ | $\color{green}{0.76}$ | $\color{green}{0.80}$ | $\color{green}{0.71}$ | $\color{green}{0.69}$ | $\color{green}{0.68}$ | $\color{green}{0.69}$ |
| `video` | $\color{green}{0.67}$ | $\color{green}{0.71}$ | $\color{green}{0.74}$ | $\color{green}{0.83}$ | $\color{green}{0.87}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.85}$ |
| `walk` | $\color{green}{0.71}$ | $\color{green}{0.77}$ | $\color{green}{0.73}$ | $\color{green}{0.85}$ | $\color{green}{0.92}$ | $\color{green}{0.87}$ | $\color{green}{0.86}$ | $\color{green}{0.85}$ | $\color{green}{0.83}$ | $\color{green}{0.82}$ |

### Luma PSNR, this encoder less libvpx, in dB

Above zero is a better picture than libvpx at the same dial.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.1}$ | 0.0 | $\color{green}{+0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `uncover` | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | 0.0 | $\color{red}{-0.3}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `flood` | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `browser` | $\color{green}{+1.0}$ | $\color{green}{+1.0}$ | $\color{green}{+1.0}$ | $\color{green}{+0.7}$ | $\color{green}{+1.3}$ | $\color{green}{+1.6}$ | $\color{green}{+1.2}$ | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ |
| `drag` | $\color{green}{+1.1}$ | $\color{green}{+1.2}$ | $\color{green}{+1.1}$ | $\color{green}{+1.4}$ | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+1.3}$ |
| `video` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.5}$ |
| `walk` | 0.0 | 0.0 | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.8}$ | $\color{green}{+1.0}$ |

## Against libvpx, the frames apart

The time columns are each encoder's median time for a frame of that kind, the
time ratio the median of the per-frame ratios, and the bytes the mean per frame.

### `terminal`, a line of text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 230 | 2.66 | $\color{green}{2.11}$ | $\color{green}{0.79}$ | 123 B | 124 B | $\color{red}{1.01}$ | $\color{green}{+0.1}$ |
| 1440×900 | 269 | 3.24 | $\color{green}{1.87}$ | $\color{green}{0.58}$ | 119 B | 142 B | $\color{red}{1.20}$ | 0.0 |
| 1600×1000 | 270 | 3.63 | $\color{green}{1.96}$ | $\color{green}{0.54}$ | 121 B | 121 B | 1.00 | $\color{green}{+0.1}$ |
| 1920×1080 | 271 | 4.50 | $\color{green}{2.06}$ | $\color{green}{0.46}$ | 134 B | 132 B | $\color{green}{0.98}$ | $\color{red}{-0.1}$ |
| 2048×1536@2 | 268 | 4.50 | $\color{green}{2.27}$ | $\color{green}{0.51}$ | 205 B | 184 B | $\color{green}{0.90}$ | $\color{green}{+0.3}$ |
| 2560×1600@2 | 270 | 5.45 | $\color{green}{2.45}$ | $\color{green}{0.45}$ | 197 B | 187 B | $\color{green}{0.95}$ | $\color{green}{+0.3}$ |
| 2880×1800@2 | 270 | 7.22 | $\color{green}{2.72}$ | $\color{green}{0.37}$ | 190 B | 224 B | $\color{red}{1.18}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 232 | 8.40 | $\color{green}{2.86}$ | $\color{green}{0.34}$ | 209 B | 217 B | $\color{red}{1.04}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 265 | 9.76 | $\color{green}{3.15}$ | $\color{green}{0.32}$ | 218 B | 208 B | $\color{green}{0.95}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 264 | 10.6 | $\color{green}{3.23}$ | $\color{green}{0.30}$ | 221 B | 219 B | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |

### `terminal`, a scroll

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 8 | 12.2 | $\color{green}{8.47}$ | $\color{green}{0.69}$ | 40 kB | 34 kB | $\color{green}{0.84}$ | $\color{green}{+0.1}$ |
| 1440×900 | 8 | 13.4 | $\color{green}{7.81}$ | $\color{green}{0.58}$ | 46 kB | 36 kB | $\color{green}{0.80}$ | 0.0 |
| 1600×1000 | 7 | 13.1 | $\color{green}{8.13}$ | $\color{green}{0.55}$ | 61 kB | 50 kB | $\color{green}{0.82}$ | $\color{red}{-0.2}$ |
| 1920×1080 | 7 | 15.2 | $\color{green}{9.38}$ | $\color{green}{0.54}$ | 67 kB | 56 kB | $\color{green}{0.83}$ | $\color{red}{-0.1}$ |
| 2048×1536@2 | 8 | 25.9 | $\color{green}{17.4}$ | $\color{green}{0.51}$ | 102 kB | 76 kB | $\color{green}{0.74}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 8 | 27.6 | $\color{green}{22.2}$ | $\color{green}{0.62}$ | 114 kB | 87 kB | $\color{green}{0.76}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 8 | 43.6 | $\color{green}{21.7}$ | $\color{green}{0.57}$ | 122 kB | 93 kB | $\color{green}{0.76}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 7 | 30.5 | $\color{green}{16.2}$ | $\color{green}{0.55}$ | 122 kB | 105 kB | $\color{green}{0.86}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 7 | 38.7 | $\color{green}{22.1}$ | $\color{green}{0.47}$ | 169 kB | 144 kB | $\color{green}{0.86}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 6 | 48.2 | $\color{green}{34.5}$ | $\color{green}{0.55}$ | 201 kB | 169 kB | $\color{green}{0.84}$ | $\color{green}{+0.2}$ |

### `drag`, a quarter of the picture

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 394 | 10.7 | $\color{green}{6.44}$ | $\color{green}{0.61}$ | 6090 B | 591 B | $\color{green}{0.10}$ | $\color{green}{+1.1}$ |
| 1440×900 | 394 | 10.6 | $\color{green}{6.59}$ | $\color{green}{0.62}$ | 6112 B | 671 B | $\color{green}{0.11}$ | $\color{green}{+1.2}$ |
| 1600×1000 | 398 | 11.0 | $\color{green}{6.55}$ | $\color{green}{0.59}$ | 6037 B | 684 B | $\color{green}{0.11}$ | $\color{green}{+1.1}$ |
| 1920×1080 | 391 | 11.8 | $\color{green}{6.77}$ | $\color{green}{0.58}$ | 8512 B | 729 B | $\color{green}{0.09}$ | $\color{green}{+1.4}$ |
| 2048×1536@2 | 344 | 21.9 | $\color{green}{17.4}$ | $\color{green}{0.80}$ | 9859 B | 1836 B | $\color{green}{0.19}$ | $\color{green}{+0.1}$ |
| 2560×1600@2 | 371 | 22.8 | $\color{green}{18.3}$ | $\color{green}{0.80}$ | 7199 B | 1773 B | $\color{green}{0.25}$ | $\color{green}{+0.1}$ |
| 2880×1800@2 | 371 | 24.7 | $\color{green}{17.8}$ | $\color{green}{0.72}$ | 6983 B | 2313 B | $\color{green}{0.33}$ | $\color{green}{+0.1}$ |
| 3200×2000@2 | 331 | 26.9 | $\color{green}{19.1}$ | $\color{green}{0.71}$ | 12 kB | 2665 B | $\color{green}{0.23}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 355 | 27.4 | $\color{green}{18.7}$ | $\color{green}{0.69}$ | 10 kB | 2742 B | $\color{green}{0.26}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 340 | 26.4 | $\color{green}{19.1}$ | $\color{green}{0.72}$ | 31 kB | 2856 B | $\color{green}{0.09}$ | $\color{green}{+1.3}$ |

### `browser`, a page scrolled

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 206 | 20.3 | $\color{green}{11.0}$ | $\color{green}{0.54}$ | 11 kB | 2290 B | $\color{green}{0.20}$ | $\color{green}{+1.0}$ |
| 1440×900 | 219 | 20.6 | $\color{green}{11.9}$ | $\color{green}{0.58}$ | 16 kB | 2783 B | $\color{green}{0.17}$ | $\color{green}{+1.0}$ |
| 1600×1000 | 223 | 23.8 | $\color{green}{13.3}$ | $\color{green}{0.56}$ | 15 kB | 3794 B | $\color{green}{0.25}$ | $\color{green}{+1.0}$ |
| 1920×1080 | 171 | 28.9 | $\color{green}{16.1}$ | $\color{green}{0.56}$ | 17 kB | 4854 B | $\color{green}{0.28}$ | $\color{green}{+0.7}$ |
| 2048×1536@2 | 186 | 39.3 | $\color{green}{23.9}$ | $\color{green}{0.60}$ | 54 kB | 13 kB | $\color{green}{0.24}$ | $\color{green}{+1.3}$ |
| 2560×1600@2 | 137 | 55.3 | $\color{green}{30.5}$ | $\color{green}{0.57}$ | 75 kB | 11 kB | $\color{green}{0.15}$ | $\color{green}{+1.6}$ |
| 2880×1800@2 | 128 | 64.8 | $\color{green}{35.1}$ | $\color{green}{0.55}$ | 77 kB | 13 kB | $\color{green}{0.17}$ | $\color{green}{+1.2}$ |
| 3200×2000@2 | 85 | 77.9 | $\color{green}{42.0}$ | $\color{green}{0.56}$ | 94 kB | 19 kB | $\color{green}{0.20}$ | $\color{green}{+1.1}$ |
| 3456×2168@2 | 88 | 83.9 | $\color{green}{45.5}$ | $\color{green}{0.54}$ | 94 kB | 21 kB | $\color{green}{0.22}$ | $\color{green}{+1.1}$ |
| 3840×2160@2 | 94 | 81.6 | $\color{green}{46.1}$ | $\color{green}{0.57}$ | 90 kB | 23 kB | $\color{green}{0.26}$ | $\color{green}{+1.2}$ |

### `flood`, a window of new text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 149 | 133.4 | $\color{green}{53.6}$ | $\color{green}{0.41}$ | 675 kB | 564 kB | $\color{green}{0.84}$ | $\color{green}{+0.2}$ |
| 1440×900 | 149 | 113.9 | $\color{green}{56.4}$ | $\color{green}{0.51}$ | 673 kB | 519 kB | $\color{green}{0.77}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 149 | 148.8 | $\color{green}{71.0}$ | $\color{green}{0.48}$ | 936 kB | 691 kB | $\color{green}{0.74}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 149 | 220.0 | $\color{green}{102.4}$ | $\color{green}{0.47}$ | 1430 kB | 1083 kB | $\color{green}{0.76}$ | $\color{green}{+0.3}$ |
| 2048×1536@2 | 149 | 218.4 | $\color{green}{130.1}$ | $\color{green}{0.60}$ | 1372 kB | 1304 kB | $\color{green}{0.95}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 149 | 288.8 | $\color{green}{165.8}$ | $\color{green}{0.58}$ | 1803 kB | 1735 kB | $\color{green}{0.96}$ | $\color{green}{+0.4}$ |
| 2880×1800@2 | 149 | 381.0 | $\color{green}{218.3}$ | $\color{green}{0.57}$ | 2346 kB | 2262 kB | $\color{green}{0.96}$ | $\color{green}{+0.5}$ |
| 3200×2000@2 | 149 | 464.7 | $\color{green}{261.0}$ | $\color{green}{0.57}$ | 2897 kB | 2795 kB | $\color{green}{0.96}$ | $\color{green}{+0.5}$ |
| 3456×2168@2 | 149 | 551.3 | $\color{green}{310.1}$ | $\color{green}{0.56}$ | 3398 kB | 3275 kB | $\color{green}{0.96}$ | $\color{green}{+0.5}$ |
| 3840×2160@2 | 149 | 616.6 | $\color{green}{340.8}$ | $\color{green}{0.55}$ | 3784 kB | 3666 kB | $\color{green}{0.97}$ | $\color{green}{+0.5}$ |

### `uncover`, a window moved off a page

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 53 | 4.38 | $\color{green}{2.34}$ | $\color{green}{0.52}$ | 3563 B | 3270 B | $\color{green}{0.92}$ | $\color{green}{+0.1}$ |
| 1440×900 | 53 | 4.98 | $\color{green}{2.26}$ | $\color{green}{0.46}$ | 3978 B | 3776 B | $\color{green}{0.95}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 54 | 5.71 | $\color{green}{2.57}$ | $\color{green}{0.46}$ | 5069 B | 4433 B | $\color{green}{0.87}$ | 0.0 |
| 1920×1080 | 53 | 6.92 | $\color{green}{2.83}$ | $\color{green}{0.40}$ | 3796 B | 3944 B | $\color{red}{1.04}$ | $\color{red}{-0.3}$ |
| 2048×1536@2 | 54 | 7.90 | $\color{green}{3.54}$ | $\color{green}{0.45}$ | 13 kB | 13 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 54 | 10.2 | $\color{green}{4.16}$ | $\color{green}{0.42}$ | 8388 B | 8353 B | 1.00 | $\color{green}{+0.2}$ |
| 2880×1800@2 | 54 | 12.6 | $\color{green}{5.21}$ | $\color{green}{0.41}$ | 11 kB | 12 kB | $\color{red}{1.07}$ | $\color{green}{+0.2}$ |
| 3200×2000@2 | 40 | 15.5 | $\color{green}{5.83}$ | $\color{green}{0.39}$ | 20 kB | 21 kB | $\color{red}{1.05}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 54 | 17.1 | $\color{green}{6.79}$ | $\color{green}{0.40}$ | 12 kB | 11 kB | $\color{green}{0.90}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 53 | 18.4 | $\color{green}{7.20}$ | $\color{green}{0.40}$ | 15 kB | 16 kB | $\color{red}{1.07}$ | $\color{green}{+0.1}$ |

### `video`, a frame of the pattern

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 483 | 16.2 | $\color{green}{10.9}$ | $\color{green}{0.68}$ | 34 kB | 35 kB | $\color{red}{1.01}$ | $\color{green}{+0.4}$ |
| 1440×900 | 489 | 16.2 | $\color{green}{11.4}$ | $\color{green}{0.71}$ | 35 kB | 36 kB | $\color{red}{1.01}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 487 | 16.1 | $\color{green}{11.6}$ | $\color{green}{0.74}$ | 35 kB | 36 kB | $\color{red}{1.01}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 474 | 13.5 | $\color{green}{11.3}$ | $\color{green}{0.83}$ | 36 kB | 36 kB | $\color{green}{0.99}$ | $\color{green}{+0.4}$ |
| 2048×1536@2 | 397 | 22.4 | $\color{green}{19.9}$ | $\color{green}{0.87}$ | 65 kB | 66 kB | $\color{red}{1.02}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 309 | 30.2 | $\color{green}{26.4}$ | $\color{green}{0.85}$ | 80 kB | 79 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 2880×1800@2 | 255 | 39.8 | $\color{green}{33.8}$ | $\color{green}{0.84}$ | 97 kB | 98 kB | 1.00 | $\color{green}{+0.6}$ |
| 3200×2000@2 | 190 | 45.0 | $\color{green}{39.4}$ | $\color{green}{0.85}$ | 112 kB | 112 kB | 1.00 | $\color{green}{+0.7}$ |
| 3456×2168@2 | 172 | 57.6 | $\color{green}{46.2}$ | $\color{green}{0.85}$ | 122 kB | 126 kB | $\color{red}{1.03}$ | $\color{green}{+0.8}$ |
| 3840×2160@2 | 231 | 36.5 | $\color{green}{30.5}$ | $\color{green}{0.84}$ | 89 kB | 84 kB | $\color{green}{0.94}$ | $\color{green}{+0.5}$ |

### `walk`, a frame as the dial walks

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 239 | 11.2 | $\color{green}{8.19}$ | $\color{green}{0.72}$ | 12 kB | 13 kB | $\color{red}{1.08}$ | 0.0 |
| 1440×900 | 234 | 10.9 | $\color{green}{8.70}$ | $\color{green}{0.76}$ | 13 kB | 14 kB | $\color{red}{1.09}$ | 0.0 |
| 1600×1000 | 232 | 13.9 | $\color{green}{10.7}$ | $\color{green}{0.74}$ | 17 kB | 18 kB | $\color{red}{1.09}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 227 | 10.8 | $\color{green}{9.06}$ | $\color{green}{0.84}$ | 14 kB | 14 kB | $\color{red}{1.06}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 230 | 17.4 | $\color{green}{15.6}$ | $\color{green}{0.93}$ | 27 kB | 29 kB | $\color{red}{1.08}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 196 | 27.1 | $\color{green}{23.3}$ | $\color{green}{0.86}$ | 43 kB | 44 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 195 | 27.0 | $\color{green}{22.5}$ | $\color{green}{0.87}$ | 35 kB | 35 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 3200×2000@2 | 174 | 30.6 | $\color{green}{26.3}$ | $\color{green}{0.85}$ | 47 kB | 46 kB | $\color{green}{0.98}$ | $\color{green}{+0.6}$ |
| 3456×2168@2 | 128 | 44.5 | $\color{green}{37.3}$ | $\color{green}{0.85}$ | 55 kB | 60 kB | $\color{red}{1.09}$ | $\color{green}{+0.8}$ |
| 3840×2160@2 | 102 | 52.5 | $\color{green}{42.5}$ | $\color{green}{0.83}$ | 68 kB | 72 kB | $\color{red}{1.05}$ | $\color{green}{+1.0}$ |

### `walk`, the settle a session asked for

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 4.68 | $\color{green}{3.18}$ | $\color{green}{0.68}$ | 157 B | 114 B | $\color{green}{0.73}$ | 0.0 |
| 1440×900 | 1 | 5.46 | $\color{green}{3.64}$ | $\color{green}{0.67}$ | 201 B | 157 B | $\color{green}{0.78}$ | 0.0 |
| 1600×1000 | 1 | 5.46 | $\color{green}{4.32}$ | $\color{green}{0.79}$ | 179 B | 148 B | $\color{green}{0.83}$ | 0.0 |
| 1920×1080 | 1 | 6.52 | $\color{green}{4.65}$ | $\color{green}{0.71}$ | 209 B | 137 B | $\color{green}{0.66}$ | 0.0 |
| 2048×1536@2 | 1 | 6.21 | $\color{green}{4.89}$ | $\color{green}{0.79}$ | 211 B | 108 B | $\color{green}{0.51}$ | 0.0 |
| 2880×1800@2 | 1 | 13.0 | $\color{green}{7.60}$ | $\color{green}{0.58}$ | 404 B | 186 B | $\color{green}{0.46}$ | 0.0 |
| 3456×2168@2 | 1 | 15.4 | $\color{green}{8.95}$ | $\color{green}{0.58}$ | 658 B | 186 B | $\color{green}{0.28}$ | 0.0 |
| 3840×2160@2 | 1 | 17.1 | $\color{green}{9.15}$ | $\color{green}{0.54}$ | 729 B | 183 B | $\color{green}{0.25}$ | 0.0 |

### Keyframes inside `uncover`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 51.5 | $\color{green}{31.4}$ | $\color{green}{0.77}$ | 104 kB | 105 kB | $\color{red}{1.01}$ | $\color{red}{-0.1}$ |
| 1440×900 | 2 | 40.1 | $\color{green}{29.5}$ | $\color{green}{0.81}$ | 112 kB | 114 kB | $\color{red}{1.01}$ | $\color{red}{-0.1}$ |
| 1600×1000 | 2 | 49.2 | $\color{green}{33.0}$ | $\color{green}{0.76}$ | 144 kB | 146 kB | $\color{red}{1.01}$ | $\color{red}{-0.1}$ |
| 1920×1080 | 2 | 62.3 | $\color{green}{41.1}$ | $\color{green}{0.82}$ | 161 kB | 162 kB | $\color{red}{1.01}$ | $\color{red}{-0.1}$ |
| 2048×1536@2 | 2 | 58.6 | $\color{green}{55.0}$ | $\color{green}{0.97}$ | 223 kB | 224 kB | 1.00 | $\color{red}{-0.1}$ |
| 2560×1600@2 | 2 | 75.3 | $\color{green}{66.4}$ | $\color{green}{0.90}$ | 273 kB | 274 kB | 1.00 | 0.0 |
| 2880×1800@2 | 2 | 121.4 | $\color{green}{75.6}$ | $\color{green}{0.86}$ | 295 kB | 296 kB | 1.00 | 0.0 |
| 3200×2000@2 | 2 | 94.5 | $\color{green}{90.2}$ | $\color{green}{0.96}$ | 309 kB | 310 kB | 1.00 | 0.0 |
| 3456×2168@2 | 2 | 133.7 | $\color{green}{111.8}$ | $\color{green}{0.98}$ | 421 kB | 423 kB | 1.00 | 0.0 |
| 3840×2160@2 | 2 | 153.0 | $\color{green}{107.0}$ | $\color{green}{0.70}$ | 420 kB | 422 kB | 1.00 | 0.0 |

### Keyframes inside `drag`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 73.3 | $\color{green}{28.8}$ | $\color{green}{0.46}$ | 54 kB | 54 kB | $\color{green}{0.99}$ | $\color{red}{-0.1}$ |
| 1440×900 | 2 | 68.5 | $\color{green}{24.6}$ | $\color{green}{0.58}$ | 65 kB | 64 kB | $\color{green}{0.99}$ | 0.0 |
| 1600×1000 | 2 | 84.1 | $\color{green}{29.0}$ | $\color{green}{0.59}$ | 83 kB | 82 kB | $\color{green}{0.99}$ | $\color{red}{-0.1}$ |
| 1920×1080 | 2 | 81.2 | $\color{green}{26.2}$ | $\color{green}{0.53}$ | 66 kB | 65 kB | $\color{green}{0.97}$ | 0.0 |
| 2048×1536@2 | 2 | 169.1 | $\color{green}{40.0}$ | $\color{green}{0.49}$ | 89 kB | 88 kB | $\color{green}{0.98}$ | 0.0 |
| 2560×1600@2 | 2 | 180.4 | $\color{green}{51.0}$ | $\color{green}{0.52}$ | 125 kB | 122 kB | $\color{green}{0.98}$ | 0.0 |
| 2880×1800@2 | 2 | 170.2 | $\color{green}{58.4}$ | $\color{green}{0.68}$ | 125 kB | 121 kB | $\color{green}{0.97}$ | 0.0 |
| 3200×2000@2 | 2 | 185.3 | $\color{green}{74.9}$ | $\color{green}{0.49}$ | 203 kB | 200 kB | $\color{green}{0.99}$ | 0.0 |
| 3456×2168@2 | 2 | 191.3 | $\color{green}{73.2}$ | $\color{green}{0.48}$ | 166 kB | 161 kB | $\color{green}{0.97}$ | 0.0 |
| 3840×2160@2 | 2 | 217.5 | $\color{green}{95.0}$ | $\color{green}{0.59}$ | 192 kB | 188 kB | $\color{green}{0.98}$ | 0.0 |

### Keyframes inside `flood`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 32.4 | $\color{green}{16.6}$ | $\color{green}{0.72}$ | 52 kB | 51 kB | $\color{green}{0.98}$ | $\color{red}{-0.1}$ |
| 1440×900 | 2 | 36.4 | $\color{green}{18.9}$ | $\color{green}{0.57}$ | 58 kB | 57 kB | $\color{green}{0.98}$ | $\color{red}{-0.1}$ |
| 1600×1000 | 2 | 35.0 | $\color{green}{24.2}$ | $\color{green}{0.78}$ | 80 kB | 79 kB | $\color{green}{0.99}$ | 0.0 |
| 1920×1080 | 2 | 36.3 | $\color{green}{29.0}$ | $\color{green}{0.80}$ | 91 kB | 90 kB | $\color{green}{0.99}$ | 0.0 |
| 2048×1536@2 | 2 | 53.7 | $\color{green}{35.9}$ | $\color{green}{0.99}$ | 107 kB | 107 kB | 1.00 | 0.0 |
| 2560×1600@2 | 2 | 77.9 | $\color{green}{44.4}$ | $\color{green}{0.66}$ | 118 kB | 115 kB | $\color{green}{0.98}$ | 0.0 |
| 2880×1800@2 | 2 | 109.2 | $\color{green}{51.1}$ | $\color{red}{1.04}$ | 131 kB | 128 kB | $\color{green}{0.98}$ | 0.0 |
| 3200×2000@2 | 2 | 108.8 | $\color{green}{69.4}$ | $\color{green}{0.92}$ | 183 kB | 180 kB | $\color{green}{0.99}$ | 0.0 |
| 3456×2168@2 | 2 | 77.2 | $\color{green}{73.9}$ | $\color{green}{0.97}$ | 207 kB | 205 kB | $\color{green}{0.99}$ | 0.0 |
| 3840×2160@2 | 2 | 80.3 | $\color{green}{74.3}$ | $\color{red}{1.04}$ | 206 kB | 203 kB | $\color{green}{0.99}$ | 0.0 |

### Keyframes inside `video`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 3 | 17.6 | $\color{green}{13.5}$ | $\color{green}{0.72}$ | 49 kB | 45 kB | $\color{green}{0.92}$ | 0.0 |
| 1440×900 | 3 | 16.8 | $\color{green}{14.3}$ | $\color{green}{0.87}$ | 51 kB | 46 kB | $\color{green}{0.91}$ | 0.0 |
| 1600×1000 | 3 | 17.6 | $\color{green}{15.5}$ | $\color{green}{0.78}$ | 51 kB | 47 kB | $\color{green}{0.91}$ | 0.0 |
| 1920×1080 | 3 | 17.4 | $\color{green}{16.9}$ | $\color{green}{0.80}$ | 51 kB | 46 kB | $\color{green}{0.91}$ | 0.0 |
| 2048×1536@2 | 3 | 21.5 | $\color{red}{27.1}$ | $\color{red}{1.13}$ | 104 kB | 97 kB | $\color{green}{0.93}$ | $\color{red}{-0.1}$ |
| 2560×1600@2 | 3 | 30.7 | $\color{red}{34.1}$ | $\color{green}{0.95}$ | 116 kB | 104 kB | $\color{green}{0.90}$ | 0.0 |
| 2880×1800@2 | 3 | 36.1 | $\color{red}{36.4}$ | $\color{green}{0.95}$ | 120 kB | 108 kB | $\color{green}{0.90}$ | $\color{green}{+0.1}$ |
| 3200×2000@2 | 3 | 39.1 | $\color{red}{41.3}$ | $\color{green}{0.97}$ | 126 kB | 113 kB | $\color{green}{0.90}$ | 0.0 |
| 3456×2168@2 | 3 | 47.6 | $\color{green}{44.2}$ | $\color{red}{1.04}$ | 132 kB | 116 kB | $\color{green}{0.88}$ | $\color{green}{+0.1}$ |
| 3840×2160@2 | 3 | 40.1 | $\color{red}{47.1}$ | $\color{green}{0.93}$ | 117 kB | 104 kB | $\color{green}{0.89}$ | 0.0 |

## Against libvpx, keyframes of each capture's first picture

### A keyframe at dial 40: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.85}$ | $\color{green}{0.92}$ | $\color{green}{0.96}$ | $\color{green}{0.82}$ | $\color{red}{1.20}$ | $\color{red}{1.05}$ | $\color{red}{1.08}$ | $\color{red}{1.02}$ | $\color{red}{1.09}$ | $\color{red}{1.02}$ |
| `flood` | $\color{green}{0.78}$ | $\color{green}{0.99}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.17}$ | $\color{red}{1.11}$ | $\color{red}{1.05}$ | $\color{red}{1.11}$ | $\color{red}{1.15}$ | $\color{red}{1.14}$ |
| `browser` | $\color{green}{0.75}$ | $\color{green}{0.78}$ | $\color{green}{0.86}$ | $\color{green}{0.85}$ | $\color{green}{0.99}$ | $\color{green}{0.94}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ |
| `drag` | $\color{green}{0.84}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{red}{1.10}$ | $\color{red}{1.09}$ | $\color{red}{1.14}$ | $\color{red}{1.09}$ | $\color{red}{1.11}$ | $\color{red}{1.17}$ |
| `video` | $\color{green}{0.90}$ | $\color{green}{0.95}$ | $\color{red}{1.20}$ | $\color{green}{0.87}$ | $\color{red}{1.17}$ | $\color{red}{1.14}$ | $\color{red}{1.12}$ | $\color{red}{1.04}$ | $\color{red}{1.09}$ | $\color{red}{1.06}$ |
| `walk` | $\color{green}{0.78}$ | $\color{green}{0.87}$ | $\color{green}{0.83}$ | $\color{green}{0.90}$ | $\color{red}{1.14}$ | $\color{red}{1.22}$ | $\color{red}{1.27}$ | $\color{red}{1.20}$ | $\color{red}{1.21}$ | $\color{red}{1.20}$ |

### A keyframe at dial 40: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ |
| `browser` | $\color{green}{0.32}$ | $\color{green}{0.38}$ | $\color{green}{0.25}$ | $\color{green}{0.23}$ | $\color{green}{0.26}$ | $\color{green}{0.25}$ | $\color{green}{0.29}$ | $\color{green}{0.27}$ | $\color{green}{0.22}$ | $\color{green}{0.24}$ |
| `drag` | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.99}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ |
| `video` | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ |
| `walk` | $\color{green}{0.70}$ | $\color{green}{0.65}$ | $\color{green}{0.64}$ | $\color{green}{0.64}$ | $\color{green}{0.66}$ | $\color{green}{0.67}$ | $\color{green}{0.67}$ | $\color{green}{0.70}$ | $\color{green}{0.65}$ | $\color{green}{0.65}$ |

### A keyframe at dial 40: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ |
| `flood` | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.1}$ | 0.0 | 0.0 | 0.0 | 0.0 | $\color{red}{-0.1}$ |
| `browser` | $\color{green}{+30.0}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ |
| `drag` | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `video` | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ |
| `walk` | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.3}$ | $\color{red}{-0.1}$ | $\color{green}{+0.4}$ | $\color{red}{-0.2}$ | $\color{green}{+1.0}$ |

### A keyframe at dial 65: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.76}$ | $\color{green}{0.84}$ | $\color{red}{1.24}$ | $\color{green}{0.74}$ | $\color{red}{1.07}$ | 1.00 | 1.00 | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{green}{0.96}$ |
| `flood` | $\color{green}{0.80}$ | $\color{green}{0.88}$ | $\color{green}{0.96}$ | $\color{green}{0.91}$ | $\color{red}{1.15}$ | $\color{red}{1.09}$ | $\color{red}{1.06}$ | $\color{red}{1.06}$ | $\color{red}{1.11}$ | $\color{red}{1.17}$ |
| `browser` | $\color{green}{0.73}$ | $\color{green}{0.85}$ | $\color{green}{0.85}$ | $\color{green}{0.86}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.94}$ | $\color{green}{0.95}$ | 1.00 | $\color{red}{1.08}$ |
| `drag` | $\color{green}{0.77}$ | $\color{green}{0.90}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ | $\color{red}{1.07}$ | $\color{red}{1.14}$ | $\color{red}{1.03}$ | $\color{red}{1.05}$ | $\color{red}{1.02}$ | $\color{red}{1.13}$ |
| `video` | $\color{green}{0.81}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ | $\color{green}{0.81}$ | $\color{red}{1.07}$ | $\color{red}{1.07}$ | $\color{red}{1.01}$ | $\color{green}{0.97}$ | 1.00 | $\color{red}{1.02}$ |
| `walk` | $\color{green}{0.80}$ | $\color{green}{0.88}$ | $\color{green}{0.92}$ | $\color{green}{0.96}$ | $\color{red}{1.15}$ | $\color{red}{1.20}$ | $\color{red}{1.20}$ | $\color{red}{1.18}$ | $\color{red}{1.14}$ | $\color{red}{1.09}$ |

### A keyframe at dial 65: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 |
| `browser` | $\color{green}{0.31}$ | $\color{green}{0.36}$ | $\color{green}{0.25}$ | $\color{green}{0.23}$ | $\color{green}{0.25}$ | $\color{green}{0.24}$ | $\color{green}{0.29}$ | $\color{green}{0.26}$ | $\color{green}{0.22}$ | $\color{green}{0.23}$ |
| `drag` | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.99}$ |
| `video` | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `walk` | $\color{green}{0.80}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.75}$ | $\color{green}{0.72}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ | $\color{green}{0.72}$ | $\color{green}{0.72}$ |

### A keyframe at dial 65: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `flood` | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.7}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | 0.0 | $\color{green}{+0.1}$ | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `video` | $\color{green}{+0.1}$ | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `walk` | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.3}$ |

### A keyframe at dial 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.66}$ | $\color{green}{0.81}$ | $\color{red}{1.02}$ | $\color{green}{0.68}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.91}$ | $\color{green}{0.95}$ | $\color{green}{0.91}$ | $\color{green}{0.87}$ |
| `flood` | $\color{green}{0.67}$ | $\color{green}{0.82}$ | $\color{green}{0.91}$ | $\color{green}{0.83}$ | $\color{red}{1.07}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | 1.00 | 1.00 | $\color{red}{1.06}$ |
| `browser` | $\color{green}{0.75}$ | $\color{green}{0.85}$ | $\color{green}{0.85}$ | $\color{green}{0.84}$ | $\color{red}{1.02}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | 1.00 | $\color{red}{1.05}$ |
| `drag` | $\color{green}{0.70}$ | $\color{green}{0.80}$ | $\color{green}{0.86}$ | $\color{green}{0.83}$ | $\color{green}{0.98}$ | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{red}{1.06}$ |
| `video` | $\color{green}{0.69}$ | $\color{green}{0.76}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.90}$ | $\color{green}{0.93}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ |
| `walk` | $\color{green}{0.82}$ | $\color{green}{0.91}$ | $\color{red}{1.02}$ | $\color{green}{0.91}$ | $\color{red}{1.17}$ | $\color{red}{1.10}$ | $\color{red}{1.09}$ | $\color{red}{1.12}$ | $\color{red}{1.17}$ | $\color{red}{1.09}$ |

### A keyframe at dial 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `browser` | $\color{green}{0.31}$ | $\color{green}{0.36}$ | $\color{green}{0.24}$ | $\color{green}{0.23}$ | $\color{green}{0.24}$ | $\color{green}{0.24}$ | $\color{green}{0.29}$ | $\color{green}{0.26}$ | $\color{green}{0.22}$ | $\color{green}{0.23}$ |
| `drag` | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ |
| `video` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `walk` | $\color{green}{0.82}$ | $\color{green}{0.79}$ | $\color{green}{0.79}$ | $\color{green}{0.79}$ | $\color{green}{0.78}$ | $\color{green}{0.75}$ | $\color{green}{0.77}$ | $\color{green}{0.78}$ | $\color{green}{0.75}$ | $\color{green}{0.75}$ |

### A keyframe at dial 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ |
| `flood` | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | 0.0 | $\color{red}{-0.1}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `video` | $\color{red}{-0.2}$ | $\color{red}{-0.3}$ | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ |
| `walk` | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | 0.0 | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |

## Against libvpx, settles of each capture's first picture

### A settle from dial 20 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.66}$ | $\color{green}{0.72}$ | $\color{green}{0.67}$ | $\color{green}{0.67}$ | $\color{green}{0.90}$ | $\color{green}{0.87}$ | $\color{green}{0.81}$ | $\color{green}{0.86}$ | $\color{green}{0.88}$ | $\color{green}{0.82}$ |
| `flood` | $\color{green}{0.73}$ | $\color{green}{0.76}$ | $\color{green}{0.81}$ | $\color{green}{0.80}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ | $\color{green}{0.90}$ | $\color{green}{0.89}$ | $\color{green}{0.90}$ | $\color{green}{0.91}$ |
| `browser` | $\color{green}{0.78}$ | $\color{green}{0.60}$ | $\color{green}{0.58}$ | $\color{green}{0.59}$ | $\color{green}{0.71}$ | $\color{green}{0.57}$ | $\color{green}{0.67}$ | $\color{green}{0.65}$ | $\color{green}{0.67}$ | $\color{green}{0.68}$ |
| `drag` | $\color{green}{0.75}$ | $\color{green}{0.77}$ | $\color{green}{0.81}$ | $\color{green}{0.78}$ | $\color{green}{0.85}$ | $\color{green}{0.90}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ |
| `video` | $\color{green}{0.61}$ | $\color{green}{0.71}$ | $\color{green}{0.70}$ | $\color{green}{0.63}$ | $\color{green}{0.88}$ | $\color{green}{0.80}$ | $\color{green}{0.78}$ | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.77}$ |
| `walk` | $\color{green}{0.70}$ | $\color{green}{0.76}$ | $\color{green}{0.75}$ | $\color{green}{0.81}$ | $\color{green}{0.87}$ | $\color{green}{0.92}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.87}$ | $\color{green}{0.81}$ |

### A settle from dial 20 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `browser` | $\color{red}{4.64}$ | $\color{red}{4.04}$ | $\color{red}{4.62}$ | $\color{red}{5.38}$ | $\color{red}{5.48}$ | $\color{red}{6.55}$ | $\color{red}{7.61}$ | $\color{red}{8.60}$ | $\color{red}{9.04}$ | $\color{red}{9.62}$ |
| `drag` | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ |
| `video` | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.05}$ |
| `walk` | $\color{green}{0.89}$ | $\color{green}{0.86}$ | $\color{green}{0.87}$ | $\color{green}{0.87}$ | $\color{green}{0.88}$ | $\color{green}{0.85}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.89}$ |

### A settle from dial 20 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `flood` | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.1}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `walk` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ | $\color{green}{+0.7}$ |

### A settle from dial 65 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.28}$ | $\color{red}{1.13}$ | $\color{red}{1.08}$ | $\color{red}{1.09}$ | $\color{red}{1.34}$ | $\color{red}{1.30}$ | $\color{red}{1.18}$ | $\color{red}{1.17}$ | $\color{red}{1.16}$ | $\color{red}{1.19}$ |
| `flood` | $\color{red}{1.26}$ | $\color{red}{1.05}$ | $\color{red}{1.11}$ | $\color{red}{1.05}$ | $\color{red}{1.15}$ | $\color{red}{1.07}$ | $\color{red}{1.03}$ | $\color{red}{1.04}$ | $\color{red}{1.06}$ | $\color{red}{1.06}$ |
| `browser` | $\color{green}{0.67}$ | $\color{green}{0.30}$ | $\color{green}{0.30}$ | $\color{green}{0.30}$ | $\color{green}{0.31}$ | $\color{green}{0.28}$ | $\color{green}{0.27}$ | $\color{green}{0.28}$ | $\color{green}{0.29}$ | $\color{green}{0.29}$ |
| `drag` | $\color{red}{1.27}$ | $\color{red}{1.07}$ | $\color{red}{1.06}$ | $\color{green}{0.93}$ | $\color{red}{1.08}$ | $\color{red}{1.17}$ | $\color{green}{0.99}$ | $\color{red}{1.09}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ |
| `video` | $\color{green}{0.87}$ | $\color{green}{0.82}$ | $\color{green}{0.77}$ | $\color{green}{0.69}$ | $\color{red}{1.05}$ | $\color{green}{0.90}$ | $\color{green}{0.79}$ | $\color{green}{0.70}$ | $\color{green}{0.68}$ | $\color{green}{0.62}$ |
| `walk` | $\color{green}{0.89}$ | $\color{green}{0.85}$ | $\color{green}{0.84}$ | $\color{green}{0.80}$ | $\color{green}{0.98}$ | $\color{red}{1.03}$ | 1.00 | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.74}$ |

### A settle from dial 65 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.02}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ |
| `flood` | $\color{red}{1.05}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.02}$ |
| `browser` | $\color{red}{1.22}$ | $\color{red}{1.37}$ | $\color{red}{1.04}$ | $\color{red}{1.15}$ | $\color{red}{1.12}$ | $\color{red}{1.22}$ | $\color{red}{1.61}$ | $\color{red}{1.45}$ | $\color{red}{1.40}$ | $\color{red}{1.42}$ |
| `drag` | $\color{red}{1.04}$ | $\color{red}{1.05}$ | $\color{red}{1.05}$ | $\color{red}{1.04}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ |
| `video` | 1.00 | $\color{red}{1.03}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.04}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ |
| `walk` | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{green}{0.98}$ | 1.00 | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.99}$ |

### A settle from dial 65 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+1.5}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ | $\color{green}{+1.5}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ | $\color{green}{+1.5}$ |
| `flood` | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.2}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ | $\color{green}{+1.5}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.7}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `video` | $\color{green}{+1.2}$ | $\color{green}{+0.9}$ | $\color{green}{+1.1}$ | $\color{green}{+1.0}$ | $\color{green}{+1.5}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ | $\color{green}{+1.5}$ | $\color{green}{+1.3}$ |
| `walk` | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.9}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |

### A settle from dial 80 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.37}$ | $\color{red}{1.08}$ | $\color{red}{1.01}$ | $\color{green}{0.99}$ | $\color{red}{1.19}$ | $\color{red}{1.19}$ | $\color{red}{1.06}$ | $\color{red}{1.05}$ | $\color{red}{1.14}$ | $\color{red}{1.03}$ |
| `flood` | $\color{red}{1.22}$ | $\color{green}{0.95}$ | $\color{green}{0.99}$ | $\color{green}{0.93}$ | $\color{green}{0.95}$ | $\color{green}{0.93}$ | $\color{green}{0.89}$ | $\color{green}{0.92}$ | $\color{green}{0.89}$ | $\color{green}{0.86}$ |
| `browser` | $\color{green}{0.61}$ | $\color{green}{0.31}$ | $\color{green}{0.30}$ | $\color{green}{0.31}$ | $\color{green}{0.30}$ | $\color{green}{0.27}$ | $\color{green}{0.27}$ | $\color{green}{0.28}$ | $\color{green}{0.28}$ | $\color{green}{0.27}$ |
| `drag` | $\color{red}{1.38}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.87}$ | $\color{green}{0.92}$ | 1.00 | $\color{green}{0.82}$ | $\color{green}{0.94}$ | $\color{green}{0.83}$ | $\color{green}{0.85}$ |
| `video` | $\color{green}{0.85}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.60}$ | $\color{green}{0.93}$ | $\color{green}{0.80}$ | $\color{green}{0.68}$ | $\color{green}{0.62}$ | $\color{green}{0.57}$ | $\color{green}{0.54}$ |
| `walk` | $\color{green}{0.89}$ | $\color{green}{0.88}$ | $\color{green}{0.82}$ | $\color{green}{0.80}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.91}$ | $\color{green}{0.72}$ |

### A settle from dial 80 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.16}$ | $\color{red}{1.19}$ | $\color{red}{1.13}$ | $\color{red}{1.09}$ | 1.00 | 1.00 | $\color{green}{0.96}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ |
| `flood` | $\color{green}{0.96}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{red}{1.05}$ | $\color{red}{1.06}$ | $\color{green}{0.99}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ |
| `browser` | $\color{red}{1.14}$ | $\color{red}{1.49}$ | $\color{red}{1.08}$ | $\color{red}{1.29}$ | $\color{red}{1.30}$ | $\color{red}{1.47}$ | $\color{red}{2.05}$ | $\color{red}{1.81}$ | $\color{red}{1.73}$ | $\color{red}{1.76}$ |
| `drag` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{green}{0.94}$ | $\color{red}{1.03}$ | $\color{red}{1.04}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{red}{1.01}$ | $\color{green}{0.96}$ | $\color{red}{1.03}$ |
| `video` | $\color{red}{1.14}$ | $\color{red}{1.10}$ | $\color{red}{1.22}$ | $\color{green}{0.91}$ | $\color{green}{0.99}$ | $\color{red}{1.06}$ | $\color{red}{1.05}$ | $\color{red}{1.10}$ | $\color{green}{0.98}$ | $\color{red}{1.04}$ |
| `walk` | $\color{red}{1.16}$ | $\color{red}{1.13}$ | $\color{red}{1.14}$ | $\color{red}{1.19}$ | $\color{red}{1.10}$ | $\color{red}{1.08}$ | $\color{red}{1.01}$ | $\color{red}{1.08}$ | $\color{red}{1.06}$ | $\color{red}{1.07}$ |

### A settle from dial 80 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `flood` | $\color{red}{-0.1}$ | 0.0 | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | 0.0 | $\color{green}{+0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ | $\color{red}{-0.1}$ | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | 0.0 | $\color{green}{+0.1}$ |
| `walk` | $\color{green}{+0.9}$ | $\color{green}{+0.9}$ | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |

### A settle from dial 89 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.45}$ | $\color{green}{0.86}$ | $\color{green}{0.93}$ | $\color{green}{0.91}$ | $\color{green}{0.92}$ | $\color{green}{0.88}$ | $\color{green}{0.81}$ | $\color{green}{0.76}$ | $\color{green}{0.79}$ | $\color{green}{0.76}$ |
| `flood` | $\color{red}{1.23}$ | $\color{green}{0.76}$ | $\color{green}{0.78}$ | $\color{green}{0.73}$ | $\color{green}{0.77}$ | $\color{green}{0.75}$ | $\color{green}{0.70}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.65}$ |
| `browser` | $\color{green}{0.61}$ | $\color{green}{0.30}$ | $\color{green}{0.29}$ | $\color{green}{0.31}$ | $\color{green}{0.28}$ | $\color{green}{0.27}$ | $\color{green}{0.26}$ | $\color{green}{0.27}$ | $\color{green}{0.28}$ | $\color{green}{0.28}$ |
| `drag` | $\color{red}{1.28}$ | $\color{green}{0.80}$ | $\color{green}{0.79}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ | $\color{green}{0.74}$ | $\color{green}{0.68}$ | $\color{green}{0.69}$ | $\color{green}{0.62}$ | $\color{green}{0.63}$ |
| `video` | $\color{green}{0.72}$ | $\color{green}{0.62}$ | $\color{green}{0.60}$ | $\color{green}{0.48}$ | $\color{green}{0.66}$ | $\color{green}{0.62}$ | $\color{green}{0.54}$ | $\color{green}{0.49}$ | $\color{green}{0.46}$ | $\color{green}{0.44}$ |
| `walk` | $\color{green}{0.80}$ | $\color{green}{0.81}$ | $\color{green}{0.76}$ | $\color{green}{0.69}$ | $\color{green}{0.87}$ | $\color{green}{0.90}$ | $\color{green}{0.87}$ | $\color{green}{0.89}$ | $\color{green}{0.89}$ | $\color{green}{0.65}$ |

### A settle from dial 89 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.31}$ | $\color{red}{1.67}$ | $\color{red}{1.33}$ | $\color{red}{1.53}$ | $\color{green}{0.90}$ | $\color{green}{0.83}$ | $\color{green}{0.89}$ | $\color{green}{0.83}$ | $\color{green}{0.79}$ | $\color{green}{0.82}$ |
| `flood` | $\color{red}{1.20}$ | $\color{red}{1.03}$ | $\color{green}{0.88}$ | $\color{red}{1.18}$ | $\color{red}{1.05}$ | $\color{red}{1.42}$ | $\color{red}{1.36}$ | $\color{red}{1.14}$ | $\color{red}{1.19}$ | $\color{red}{1.21}$ |
| `browser` | $\color{red}{1.14}$ | $\color{red}{1.49}$ | $\color{red}{1.08}$ | $\color{red}{1.29}$ | $\color{red}{1.30}$ | $\color{red}{1.47}$ | $\color{red}{2.05}$ | $\color{red}{1.81}$ | $\color{red}{1.73}$ | $\color{red}{1.76}$ |
| `drag` | $\color{red}{1.18}$ | $\color{red}{1.28}$ | $\color{red}{1.18}$ | $\color{red}{1.40}$ | $\color{red}{1.22}$ | $\color{red}{1.04}$ | $\color{red}{1.30}$ | $\color{red}{1.17}$ | $\color{red}{1.09}$ | $\color{red}{1.08}$ |
| `video` | $\color{red}{1.14}$ | $\color{red}{1.30}$ | $\color{red}{1.29}$ | $\color{green}{0.89}$ | $\color{green}{0.81}$ | $\color{green}{0.93}$ | $\color{green}{0.96}$ | $\color{red}{1.13}$ | $\color{green}{0.79}$ | $\color{green}{0.96}$ |
| `walk` | $\color{red}{1.57}$ | $\color{red}{1.60}$ | $\color{red}{1.57}$ | $\color{red}{1.55}$ | $\color{red}{1.39}$ | $\color{red}{1.31}$ | $\color{red}{1.22}$ | $\color{red}{1.38}$ | $\color{red}{1.36}$ | $\color{red}{1.30}$ |

### A settle from dial 89 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ |
| `flood` | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | 0.0 | 0.0 | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | 0.0 | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ |
| `video` | $\color{red}{-0.2}$ | $\color{red}{-0.4}$ | $\color{red}{-0.3}$ | $\color{red}{-0.2}$ | $\color{red}{-0.3}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ |
| `walk` | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.3}$ |


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
is the `frame-captures` of 2026-10-09 in the artifacts'
backup (`/Volumes/dasdata/backup/vp9-artifacts/frame-captures/`, 70
files), copied to `tmp/captures/`. Each is measured at quality 90, the
dial the captures were made at, on four threads.

The eighteen decoded samples the first gate read, and the libvpx-coded
samples `vp9-samples.sh` makes for vp9-wasm, are not encoder material: the
first are decodes of decodes, the second are coded by the encoder under
test's rival.
