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
of commit `3291a41`. A time on this machine moves by a tenth from one run to
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
  frame and summed: a line of text 0.13–0.18 (0.4 ms at 1280×800 and 1.5
  at 3840×2160, where libvpx takes 2.4 and 10.2), `uncover` 0.27–0.33,
  `flood` 0.40–0.58, `browser` 0.51–0.57, `drag` 0.54–0.73, `video`
  0.63–0.78, `walk` 0.66–0.82. `video` and `walk` are the nearest the line,
  and nearer it from 1920 wide up.
- **A keyframe inside a capture takes 0.36 to 0.96 of libvpx's time** at
  0.97–1.01× the bytes and 0.0 to −0.1 dB (`video`'s 0.88–0.93× the bytes);
  no cell of forty is over 1.00.
- **A first picture coded alone as a keyframe is level with libvpx or
  ahead.** Up to 1920 wide it takes 0.52–0.95 of libvpx's time; from 2048
  wide, in four tile columns and without the loop filter, 0.70–1.02, two
  cells of 108 over 1.00 and both by 0.02. Bytes are 0.96–1.02× (`walk`
  0.64–0.82×, the flat desktop of `browser` 0.22–0.38×) at −0.3 to +0.2 dB
  at dial 90 and mostly above zero at 65. `browser`'s +30 dB at dial 40 is
  a flat desktop this encoder holds exactly. The search's anchors are no
  longer made on a keyframe but on the frame after it, so that frame is
  where 4 to 9 ms of this went.
- **A settle from 65 to 90 is 0.93 to 1.25 of libvpx's time on a busy
  picture** (`uncover`, `flood`, `drag`) at seven sizes, for 1.2 to 1.5 dB
  more picture at 1.00–1.05× the bytes: this encoder's settle lands nearer
  the source. From 20 it is 0.58–0.94 there, from 89 0.35–0.95.
- **At 1280×800, 1440×900 and 2048×1536 the settles read 1.2 to 1.8, and
  that is libvpx's time, not this encoder's.** This encoder's settle took
  the same milliseconds in this run as in the last (`uncover` at 2048×1536
  from 65: 31.2 and 30.8 ms); libvpx's took 23.3 ms in the last run and
  17.3 in this one, and 25.9 when that capture was run again alone straight
  after. libvpx's settle comes out at one of two levels a process, a factor
  of 1.3 to 1.4 apart; which a run gets is not understood, and a settle's
  ratio is good to no better than that.
- **What is over the line in bytes**: a settle of one dial step, 89 to 90,
  at 0.8–1.7× for −0.1 to −0.4 dB (a few kilobytes), and `walk`'s there at
  1.2–1.6× for +0.3 to +0.6; a flat desktop settled from dial 20 at 4 to
  10× (one or two kilobytes against a few hundred bytes); `walk`'s captures
  at up to 1.09×.

## What changed since release 0.0.1-alpha.3

Measured on the same machine before and after, each alone. Every stream is
the release's, byte for byte.

| Change | What it bought |
|---|---|
| A phase of a frame is a queue the calling thread works too and waits on no thread that has not woken; a worker spins a tenth of a millisecond before it parks; a frame naming 1024 8×8s or fewer is coded on one thread | waking a parked thread costs 0.2 ms on this machine and a frame did it seven times: a line of text 2.1 → 0.41 ms at 1280×800, 2.06 → 0.57 at 1920×1080, 3.23 → 1.48 at 3840×2160 |
| A superblock a frame does not touch is recorded without being looked at, and the mode info of an 8×8 is sixteen bytes | with the row above; `uncover`'s frames 2.34 → 1.15 ms at 1280×800, 7.20 → 5.98 at 3840×2160 |
| A keyframe leaves the search's anchors to the frame after it | a first picture's keyframe 5 to 25% less: `walk` at 2560×1600, dial 40, 25.6 → 20.5 ms, `drag` at 3840×2160 75 → 64; the frame after pays it |
| A frame's symbols are counted a thread, not a superblock row | part of the rows above and below |
| The reference is made by whichever is fewer to copy: the superblocks a frame touched into it, or the ones it did not into the reconstruction | `video` 26.4 → 23.4 ms at 2560×1600, `walk` 23.3 → 19.9, `drag` 18.3 → 16.4 |

Against libvpx, the release's cells and this commit's over all ten sizes:

| | 0.0.1-alpha.3 | Now |
|---|---:|---:|
| A line of text, time frame for frame | 0.30–0.79 | 0.13–0.18 |
| `uncover`, time frame for frame | 0.39–0.53 | 0.27–0.33 |
| `drag`, time frame for frame | 0.58–0.80 | 0.54–0.73 |
| `video`, time frame for frame | 0.68–0.87 | 0.63–0.78 |
| `walk`, time frame for frame | 0.72–0.93 | 0.66–0.82 |
| `flood`, time frame for frame | 0.41–0.60 | 0.40–0.58 |
| A first picture's keyframe up to 1920 wide, time | 0.66–1.24 | 0.52–0.95 |
| A first picture's keyframe from 2048 wide, time | 0.87–1.27 | 0.70–1.02 |
| A keyframe inside a capture, time | 0.46–1.13 | 0.36–0.96 |

Looked at and left: the tile phase of a busy keyframe runs four threads at
75 to 80% of four, with no thread found waiting; a busy settle and a frame
of `video` or `walk` have no one hot spot (the token estimate, the block
walk, the intra mode and the transforms 5 to 11% each, the search 11 to 14%
of a `video` or `walk` frame).

## What changed between releases 0.0.1-alpha.2 and 0.0.1-alpha.3

Measured on the same machine before and after, each alone.

| Change | What it bought | Bytes |
|---|---|---|
| A transform block whose prediction is the source is not transformed or quantized, and the loop filter passes an edge that is one value across | a flat 1920×1080 keyframe 12.9 → 7.9 ms, a flat 3840×2160 one 37 → 25 ms, a busy keyframe 5–10% | the same |
| The token loops walk one table of position, band and neighbours; the luma a 4×4 trial coded is not coded again | with the row above, a busy 1920×1080 keyframe on one thread 165 → 113 ms | the same to the printed digit |
| A busy frame is in tile rows, two or four by what its heaviest tile column recorded, so its bools are not coded by one thread a column | coding the bools of a busy keyframe 17.6 → 7.7 ms at 1920×1080 (the frame 47 → 36), 33.5 → 19.7 at 3840×2160; `flood` frames 0.56 → 0.47 of libvpx's time at 1920×1080 | +0.02% |
| A 4×4 block is tried in TM beside DC, V and H | +0.16 to +0.18 dB on the captures' keyframes at 1920×1080 for 3% of their time; over the whole captures +0.1 dB in the same time | −0.1% |

Against libvpx, alpha.2's cells and alpha.3's at the four sizes both
were run at (1280×800, 1920×1080, 2560×1600, 3840×2160):

| | 0.0.1-alpha.2 | 0.0.1-alpha.3 |
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
| `terminal` | $\color{green}{0.17}$ | $\color{green}{0.16}$ | $\color{green}{0.15}$ | $\color{green}{0.13}$ | $\color{green}{0.18}$ | $\color{green}{0.17}$ | $\color{green}{0.16}$ | $\color{green}{0.16}$ | $\color{green}{0.14}$ | $\color{green}{0.14}$ |
| `uncover` | $\color{green}{0.27}$ | $\color{green}{0.30}$ | $\color{green}{0.30}$ | $\color{green}{0.28}$ | $\color{green}{0.31}$ | $\color{green}{0.32}$ | $\color{green}{0.32}$ | $\color{green}{0.32}$ | $\color{green}{0.32}$ | $\color{green}{0.33}$ |
| `flood` | $\color{green}{0.40}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ | $\color{green}{0.58}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ |
| `browser` | $\color{green}{0.51}$ | $\color{green}{0.54}$ | $\color{green}{0.53}$ | $\color{green}{0.54}$ | $\color{green}{0.57}$ | $\color{green}{0.56}$ | $\color{green}{0.51}$ | $\color{green}{0.52}$ | $\color{green}{0.51}$ | $\color{green}{0.54}$ |
| `drag` | $\color{green}{0.55}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.54}$ | $\color{green}{0.73}$ | $\color{green}{0.73}$ | $\color{green}{0.68}$ | $\color{green}{0.66}$ | $\color{green}{0.63}$ | $\color{green}{0.66}$ |
| `video` | $\color{green}{0.63}$ | $\color{green}{0.64}$ | $\color{green}{0.67}$ | $\color{green}{0.76}$ | $\color{green}{0.78}$ | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ |
| `walk` | $\color{green}{0.66}$ | $\color{green}{0.69}$ | $\color{green}{0.68}$ | $\color{green}{0.76}$ | $\color{green}{0.82}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ |

### Encode time, the whole capture's

Every frame's time summed, this encoder's over libvpx's: where the long frames weigh.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.24}$ | $\color{green}{0.22}$ | $\color{green}{0.21}$ | $\color{green}{0.18}$ | $\color{green}{0.26}$ | $\color{green}{0.24}$ | $\color{green}{0.23}$ | $\color{green}{0.22}$ | $\color{green}{0.20}$ | $\color{green}{0.21}$ |
| `uncover` | $\color{green}{0.38}$ | $\color{green}{0.41}$ | $\color{green}{0.41}$ | $\color{green}{0.37}$ | $\color{green}{0.49}$ | $\color{green}{0.45}$ | $\color{green}{0.46}$ | $\color{green}{0.46}$ | $\color{green}{0.44}$ | $\color{green}{0.45}$ |
| `flood` | $\color{green}{0.41}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ | $\color{green}{0.58}$ | $\color{green}{0.57}$ | $\color{green}{0.57}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ |
| `browser` | $\color{green}{0.51}$ | $\color{green}{0.53}$ | $\color{green}{0.52}$ | $\color{green}{0.53}$ | $\color{green}{0.57}$ | $\color{green}{0.54}$ | $\color{green}{0.52}$ | $\color{green}{0.51}$ | $\color{green}{0.51}$ | $\color{green}{0.53}$ |
| `drag` | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.54}$ | $\color{green}{0.72}$ | $\color{green}{0.72}$ | $\color{green}{0.67}$ | $\color{green}{0.64}$ | $\color{green}{0.64}$ | $\color{green}{0.63}$ |
| `video` | $\color{green}{0.63}$ | $\color{green}{0.65}$ | $\color{green}{0.68}$ | $\color{green}{0.76}$ | $\color{green}{0.78}$ | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ |
| `walk` | $\color{green}{0.66}$ | $\color{green}{0.68}$ | $\color{green}{0.72}$ | $\color{green}{0.76}$ | $\color{green}{0.82}$ | $\color{green}{0.75}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{green}{0.73}$ | $\color{green}{0.73}$ |

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
| 1280×800 | 230 | 2.37 | $\color{green}{0.41}$ | $\color{green}{0.17}$ | 123 B | 124 B | $\color{red}{1.01}$ | $\color{green}{+0.1}$ |
| 1440×900 | 269 | 3.32 | $\color{green}{0.53}$ | $\color{green}{0.16}$ | 119 B | 142 B | $\color{red}{1.20}$ | 0.0 |
| 1600×1000 | 270 | 3.45 | $\color{green}{0.51}$ | $\color{green}{0.15}$ | 121 B | 121 B | 1.00 | $\color{green}{+0.1}$ |
| 1920×1080 | 271 | 4.51 | $\color{green}{0.57}$ | $\color{green}{0.13}$ | 134 B | 132 B | $\color{green}{0.98}$ | $\color{red}{-0.1}$ |
| 2048×1536@2 | 268 | 4.54 | $\color{green}{0.83}$ | $\color{green}{0.18}$ | 205 B | 184 B | $\color{green}{0.90}$ | $\color{green}{+0.3}$ |
| 2560×1600@2 | 270 | 5.61 | $\color{green}{0.92}$ | $\color{green}{0.16}$ | 197 B | 187 B | $\color{green}{0.95}$ | $\color{green}{+0.3}$ |
| 2880×1800@2 | 270 | 7.00 | $\color{green}{1.10}$ | $\color{green}{0.16}$ | 190 B | 224 B | $\color{red}{1.18}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 232 | 8.25 | $\color{green}{1.32}$ | $\color{green}{0.16}$ | 209 B | 217 B | $\color{red}{1.04}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 265 | 9.35 | $\color{green}{1.35}$ | $\color{green}{0.14}$ | 218 B | 208 B | $\color{green}{0.95}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 264 | 10.2 | $\color{green}{1.48}$ | $\color{green}{0.14}$ | 221 B | 219 B | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |

### `terminal`, a scroll

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 8 | 11.4 | $\color{green}{7.19}$ | $\color{green}{0.42}$ | 40 kB | 34 kB | $\color{green}{0.84}$ | $\color{green}{+0.1}$ |
| 1440×900 | 8 | 13.3 | $\color{green}{7.46}$ | $\color{green}{0.54}$ | 46 kB | 36 kB | $\color{green}{0.80}$ | 0.0 |
| 1600×1000 | 7 | 14.1 | $\color{green}{8.88}$ | $\color{green}{0.54}$ | 61 kB | 50 kB | $\color{green}{0.82}$ | $\color{red}{-0.2}$ |
| 1920×1080 | 7 | 14.6 | $\color{green}{8.34}$ | $\color{green}{0.47}$ | 67 kB | 56 kB | $\color{green}{0.83}$ | $\color{red}{-0.1}$ |
| 2048×1536@2 | 8 | 24.8 | $\color{green}{18.0}$ | $\color{green}{0.48}$ | 102 kB | 76 kB | $\color{green}{0.74}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 8 | 30.3 | $\color{green}{20.9}$ | $\color{green}{0.55}$ | 114 kB | 87 kB | $\color{green}{0.76}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 8 | 34.8 | $\color{green}{19.2}$ | $\color{green}{0.49}$ | 122 kB | 93 kB | $\color{green}{0.76}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 7 | 31.4 | $\color{green}{15.4}$ | $\color{green}{0.49}$ | 122 kB | 105 kB | $\color{green}{0.86}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 7 | 34.4 | $\color{green}{23.4}$ | $\color{green}{0.49}$ | 169 kB | 144 kB | $\color{green}{0.86}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 6 | 48.3 | $\color{green}{34.1}$ | $\color{green}{0.48}$ | 201 kB | 169 kB | $\color{green}{0.84}$ | $\color{green}{+0.2}$ |

### `drag`, a quarter of the picture

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 394 | 11.5 | $\color{green}{6.21}$ | $\color{green}{0.55}$ | 6090 B | 591 B | $\color{green}{0.10}$ | $\color{green}{+1.1}$ |
| 1440×900 | 394 | 10.6 | $\color{green}{5.91}$ | $\color{green}{0.56}$ | 6112 B | 671 B | $\color{green}{0.11}$ | $\color{green}{+1.2}$ |
| 1600×1000 | 398 | 10.9 | $\color{green}{6.13}$ | $\color{green}{0.56}$ | 6037 B | 684 B | $\color{green}{0.11}$ | $\color{green}{+1.1}$ |
| 1920×1080 | 391 | 11.9 | $\color{green}{6.42}$ | $\color{green}{0.54}$ | 8512 B | 729 B | $\color{green}{0.09}$ | $\color{green}{+1.4}$ |
| 2048×1536@2 | 344 | 21.5 | $\color{green}{15.7}$ | $\color{green}{0.73}$ | 9859 B | 1836 B | $\color{green}{0.19}$ | $\color{green}{+0.1}$ |
| 2560×1600@2 | 371 | 22.6 | $\color{green}{16.4}$ | $\color{green}{0.73}$ | 7199 B | 1773 B | $\color{green}{0.25}$ | $\color{green}{+0.1}$ |
| 2880×1800@2 | 371 | 24.2 | $\color{green}{16.2}$ | $\color{green}{0.68}$ | 6983 B | 2313 B | $\color{green}{0.33}$ | $\color{green}{+0.1}$ |
| 3200×2000@2 | 331 | 26.6 | $\color{green}{17.7}$ | $\color{green}{0.66}$ | 12 kB | 2665 B | $\color{green}{0.23}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 355 | 27.7 | $\color{green}{17.3}$ | $\color{green}{0.63}$ | 10 kB | 2742 B | $\color{green}{0.26}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 340 | 26.2 | $\color{green}{17.3}$ | $\color{green}{0.66}$ | 31 kB | 2856 B | $\color{green}{0.09}$ | $\color{green}{+1.3}$ |

### `browser`, a page scrolled

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 206 | 20.5 | $\color{green}{10.3}$ | $\color{green}{0.51}$ | 11 kB | 2290 B | $\color{green}{0.20}$ | $\color{green}{+1.0}$ |
| 1440×900 | 219 | 20.5 | $\color{green}{11.1}$ | $\color{green}{0.55}$ | 16 kB | 2783 B | $\color{green}{0.17}$ | $\color{green}{+1.0}$ |
| 1600×1000 | 223 | 23.2 | $\color{green}{12.3}$ | $\color{green}{0.53}$ | 15 kB | 3794 B | $\color{green}{0.25}$ | $\color{green}{+1.0}$ |
| 1920×1080 | 171 | 28.1 | $\color{green}{15.3}$ | $\color{green}{0.54}$ | 17 kB | 4854 B | $\color{green}{0.28}$ | $\color{green}{+0.7}$ |
| 2048×1536@2 | 186 | 38.2 | $\color{green}{21.8}$ | $\color{green}{0.58}$ | 54 kB | 13 kB | $\color{green}{0.24}$ | $\color{green}{+1.3}$ |
| 2560×1600@2 | 137 | 54.4 | $\color{green}{29.0}$ | $\color{green}{0.56}$ | 75 kB | 11 kB | $\color{green}{0.15}$ | $\color{green}{+1.6}$ |
| 2880×1800@2 | 128 | 62.5 | $\color{green}{32.5}$ | $\color{green}{0.51}$ | 77 kB | 13 kB | $\color{green}{0.17}$ | $\color{green}{+1.2}$ |
| 3200×2000@2 | 85 | 76.9 | $\color{green}{39.8}$ | $\color{green}{0.53}$ | 94 kB | 19 kB | $\color{green}{0.20}$ | $\color{green}{+1.1}$ |
| 3456×2168@2 | 88 | 80.0 | $\color{green}{41.7}$ | $\color{green}{0.51}$ | 94 kB | 21 kB | $\color{green}{0.22}$ | $\color{green}{+1.1}$ |
| 3840×2160@2 | 94 | 81.2 | $\color{green}{43.1}$ | $\color{green}{0.55}$ | 90 kB | 23 kB | $\color{green}{0.26}$ | $\color{green}{+1.2}$ |

### `flood`, a window of new text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 149 | 133.7 | $\color{green}{51.9}$ | $\color{green}{0.40}$ | 675 kB | 564 kB | $\color{green}{0.84}$ | $\color{green}{+0.2}$ |
| 1440×900 | 149 | 111.3 | $\color{green}{54.1}$ | $\color{green}{0.49}$ | 673 kB | 519 kB | $\color{green}{0.77}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 149 | 146.8 | $\color{green}{67.4}$ | $\color{green}{0.47}$ | 936 kB | 691 kB | $\color{green}{0.74}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 149 | 213.8 | $\color{green}{95.6}$ | $\color{green}{0.46}$ | 1430 kB | 1083 kB | $\color{green}{0.76}$ | $\color{green}{+0.3}$ |
| 2048×1536@2 | 149 | 215.6 | $\color{green}{123.1}$ | $\color{green}{0.58}$ | 1372 kB | 1304 kB | $\color{green}{0.95}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 149 | 283.3 | $\color{green}{159.8}$ | $\color{green}{0.56}$ | 1803 kB | 1735 kB | $\color{green}{0.96}$ | $\color{green}{+0.4}$ |
| 2880×1800@2 | 149 | 369.8 | $\color{green}{207.3}$ | $\color{green}{0.56}$ | 2346 kB | 2262 kB | $\color{green}{0.96}$ | $\color{green}{+0.5}$ |
| 3200×2000@2 | 149 | 455.6 | $\color{green}{250.6}$ | $\color{green}{0.56}$ | 2897 kB | 2795 kB | $\color{green}{0.96}$ | $\color{green}{+0.5}$ |
| 3456×2168@2 | 149 | 538.2 | $\color{green}{295.1}$ | $\color{green}{0.55}$ | 3398 kB | 3275 kB | $\color{green}{0.96}$ | $\color{green}{+0.5}$ |
| 3840×2160@2 | 149 | 605.6 | $\color{green}{327.2}$ | $\color{green}{0.54}$ | 3784 kB | 3666 kB | $\color{green}{0.97}$ | $\color{green}{+0.5}$ |

### `uncover`, a window moved off a page

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 53 | 4.16 | $\color{green}{1.15}$ | $\color{green}{0.27}$ | 3563 B | 3270 B | $\color{green}{0.92}$ | $\color{green}{+0.1}$ |
| 1440×900 | 53 | 4.99 | $\color{green}{1.48}$ | $\color{green}{0.29}$ | 3978 B | 3776 B | $\color{green}{0.95}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 54 | 5.60 | $\color{green}{1.73}$ | $\color{green}{0.30}$ | 5069 B | 4433 B | $\color{green}{0.87}$ | 0.0 |
| 1920×1080 | 53 | 7.02 | $\color{green}{1.99}$ | $\color{green}{0.28}$ | 3796 B | 3944 B | $\color{red}{1.04}$ | $\color{red}{-0.3}$ |
| 2048×1536@2 | 54 | 8.06 | $\color{green}{2.48}$ | $\color{green}{0.31}$ | 13 kB | 13 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 54 | 10.0 | $\color{green}{3.13}$ | $\color{green}{0.31}$ | 8388 B | 8353 B | 1.00 | $\color{green}{+0.2}$ |
| 2880×1800@2 | 54 | 12.5 | $\color{green}{3.91}$ | $\color{green}{0.32}$ | 11 kB | 12 kB | $\color{red}{1.07}$ | $\color{green}{+0.2}$ |
| 3200×2000@2 | 40 | 15.0 | $\color{green}{4.70}$ | $\color{green}{0.32}$ | 20 kB | 21 kB | $\color{red}{1.05}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 54 | 17.1 | $\color{green}{5.43}$ | $\color{green}{0.31}$ | 12 kB | 11 kB | $\color{green}{0.90}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 53 | 18.5 | $\color{green}{5.98}$ | $\color{green}{0.32}$ | 15 kB | 16 kB | $\color{red}{1.07}$ | $\color{green}{+0.1}$ |

### `video`, a frame of the pattern

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 483 | 16.3 | $\color{green}{10.2}$ | $\color{green}{0.63}$ | 34 kB | 35 kB | $\color{red}{1.01}$ | $\color{green}{+0.4}$ |
| 1440×900 | 489 | 16.2 | $\color{green}{10.3}$ | $\color{green}{0.64}$ | 35 kB | 36 kB | $\color{red}{1.01}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 487 | 16.3 | $\color{green}{10.7}$ | $\color{green}{0.67}$ | 35 kB | 36 kB | $\color{red}{1.01}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 474 | 13.3 | $\color{green}{10.2}$ | $\color{green}{0.76}$ | 36 kB | 36 kB | $\color{green}{0.99}$ | $\color{green}{+0.4}$ |
| 2048×1536@2 | 397 | 22.6 | $\color{green}{18.1}$ | $\color{green}{0.78}$ | 65 kB | 66 kB | $\color{red}{1.02}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 309 | 29.9 | $\color{green}{23.4}$ | $\color{green}{0.75}$ | 80 kB | 79 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 2880×1800@2 | 255 | 38.4 | $\color{green}{29.9}$ | $\color{green}{0.76}$ | 97 kB | 98 kB | 1.00 | $\color{green}{+0.6}$ |
| 3200×2000@2 | 190 | 49.6 | $\color{green}{36.2}$ | $\color{green}{0.76}$ | 112 kB | 112 kB | 1.00 | $\color{green}{+0.7}$ |
| 3456×2168@2 | 172 | 57.2 | $\color{green}{40.7}$ | $\color{green}{0.75}$ | 122 kB | 126 kB | $\color{red}{1.03}$ | $\color{green}{+0.8}$ |
| 3840×2160@2 | 231 | 36.3 | $\color{green}{27.8}$ | $\color{green}{0.77}$ | 89 kB | 84 kB | $\color{green}{0.94}$ | $\color{green}{+0.5}$ |

### `walk`, a frame as the dial walks

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 239 | 10.9 | $\color{green}{7.46}$ | $\color{green}{0.66}$ | 12 kB | 13 kB | $\color{red}{1.08}$ | 0.0 |
| 1440×900 | 234 | 10.9 | $\color{green}{7.67}$ | $\color{green}{0.69}$ | 13 kB | 14 kB | $\color{red}{1.09}$ | 0.0 |
| 1600×1000 | 232 | 17.5 | $\color{green}{11.2}$ | $\color{green}{0.68}$ | 17 kB | 18 kB | $\color{red}{1.09}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 227 | 10.8 | $\color{green}{8.30}$ | $\color{green}{0.76}$ | 14 kB | 14 kB | $\color{red}{1.06}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 230 | 18.2 | $\color{green}{14.3}$ | $\color{green}{0.82}$ | 27 kB | 29 kB | $\color{red}{1.08}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 196 | 27.0 | $\color{green}{19.9}$ | $\color{green}{0.76}$ | 43 kB | 44 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 195 | 25.4 | $\color{green}{19.4}$ | $\color{green}{0.76}$ | 35 kB | 35 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 3200×2000@2 | 174 | 32.2 | $\color{green}{24.8}$ | $\color{green}{0.76}$ | 47 kB | 46 kB | $\color{green}{0.98}$ | $\color{green}{+0.6}$ |
| 3456×2168@2 | 128 | 45.1 | $\color{green}{32.2}$ | $\color{green}{0.74}$ | 55 kB | 60 kB | $\color{red}{1.09}$ | $\color{green}{+0.8}$ |
| 3840×2160@2 | 102 | 52.6 | $\color{green}{37.5}$ | $\color{green}{0.74}$ | 68 kB | 72 kB | $\color{red}{1.05}$ | $\color{green}{+1.0}$ |

### `walk`, the settle a session asked for

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 4.81 | $\color{green}{2.53}$ | $\color{green}{0.53}$ | 157 B | 114 B | $\color{green}{0.73}$ | 0.0 |
| 1440×900 | 1 | 5.12 | $\color{green}{2.38}$ | $\color{green}{0.47}$ | 201 B | 157 B | $\color{green}{0.78}$ | 0.0 |
| 1600×1000 | 1 | 5.89 | $\color{green}{2.80}$ | $\color{green}{0.47}$ | 179 B | 148 B | $\color{green}{0.83}$ | 0.0 |
| 1920×1080 | 1 | 6.66 | $\color{green}{2.99}$ | $\color{green}{0.45}$ | 209 B | 137 B | $\color{green}{0.66}$ | 0.0 |
| 2048×1536@2 | 1 | 6.00 | $\color{green}{2.86}$ | $\color{green}{0.48}$ | 211 B | 108 B | $\color{green}{0.51}$ | 0.0 |
| 2880×1800@2 | 1 | 10.8 | $\color{green}{5.69}$ | $\color{green}{0.53}$ | 404 B | 186 B | $\color{green}{0.46}$ | 0.0 |
| 3456×2168@2 | 1 | 15.7 | $\color{green}{7.32}$ | $\color{green}{0.47}$ | 658 B | 186 B | $\color{green}{0.28}$ | 0.0 |
| 3840×2160@2 | 1 | 16.8 | $\color{green}{7.39}$ | $\color{green}{0.44}$ | 729 B | 183 B | $\color{green}{0.25}$ | 0.0 |

### Keyframes inside `uncover`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 53.3 | $\color{green}{36.6}$ | $\color{green}{0.69}$ | 104 kB | 105 kB | $\color{red}{1.01}$ | $\color{red}{-0.1}$ |
| 1440×900 | 2 | 40.2 | $\color{green}{26.0}$ | $\color{green}{0.65}$ | 112 kB | 114 kB | $\color{red}{1.01}$ | $\color{red}{-0.1}$ |
| 1600×1000 | 2 | 51.1 | $\color{green}{33.4}$ | $\color{green}{0.66}$ | 144 kB | 146 kB | $\color{red}{1.01}$ | $\color{red}{-0.1}$ |
| 1920×1080 | 2 | 66.2 | $\color{green}{37.0}$ | $\color{green}{0.57}$ | 161 kB | 162 kB | $\color{red}{1.01}$ | $\color{red}{-0.1}$ |
| 2048×1536@2 | 2 | 58.8 | $\color{green}{49.1}$ | $\color{green}{0.83}$ | 223 kB | 224 kB | 1.00 | $\color{red}{-0.1}$ |
| 2560×1600@2 | 2 | 88.5 | $\color{green}{68.0}$ | $\color{green}{0.77}$ | 273 kB | 274 kB | 1.00 | 0.0 |
| 2880×1800@2 | 2 | 86.9 | $\color{green}{65.4}$ | $\color{green}{0.75}$ | 295 kB | 296 kB | 1.00 | 0.0 |
| 3200×2000@2 | 2 | 102.6 | $\color{green}{76.9}$ | $\color{green}{0.75}$ | 309 kB | 310 kB | 1.00 | 0.0 |
| 3456×2168@2 | 2 | 137.1 | $\color{green}{97.2}$ | $\color{green}{0.71}$ | 421 kB | 423 kB | 1.00 | 0.0 |
| 3840×2160@2 | 2 | 128.4 | $\color{green}{102.4}$ | $\color{green}{0.80}$ | 420 kB | 422 kB | 1.00 | 0.0 |

### Keyframes inside `drag`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 60.5 | $\color{green}{17.7}$ | $\color{green}{0.36}$ | 54 kB | 54 kB | $\color{green}{0.99}$ | $\color{red}{-0.1}$ |
| 1440×900 | 2 | 63.1 | $\color{green}{19.7}$ | $\color{green}{0.42}$ | 65 kB | 64 kB | $\color{green}{0.99}$ | 0.0 |
| 1600×1000 | 2 | 73.6 | $\color{green}{25.3}$ | $\color{green}{0.45}$ | 83 kB | 82 kB | $\color{green}{0.99}$ | $\color{red}{-0.1}$ |
| 1920×1080 | 2 | 79.3 | $\color{green}{23.1}$ | $\color{green}{0.45}$ | 66 kB | 65 kB | $\color{green}{0.97}$ | 0.0 |
| 2048×1536@2 | 2 | 146.7 | $\color{green}{44.1}$ | $\color{green}{0.45}$ | 89 kB | 88 kB | $\color{green}{0.98}$ | 0.0 |
| 2560×1600@2 | 2 | 158.3 | $\color{green}{42.8}$ | $\color{green}{0.49}$ | 125 kB | 122 kB | $\color{green}{0.98}$ | 0.0 |
| 2880×1800@2 | 2 | 150.5 | $\color{green}{44.6}$ | $\color{green}{0.45}$ | 125 kB | 121 kB | $\color{green}{0.97}$ | 0.0 |
| 3200×2000@2 | 2 | 210.6 | $\color{green}{71.0}$ | $\color{green}{0.51}$ | 203 kB | 200 kB | $\color{green}{0.99}$ | 0.0 |
| 3456×2168@2 | 2 | 186.6 | $\color{green}{62.6}$ | $\color{green}{0.48}$ | 166 kB | 161 kB | $\color{green}{0.97}$ | 0.0 |
| 3840×2160@2 | 2 | 204.6 | $\color{green}{66.8}$ | $\color{green}{0.51}$ | 192 kB | 188 kB | $\color{green}{0.98}$ | 0.0 |

### Keyframes inside `flood`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 32.5 | $\color{green}{17.2}$ | $\color{green}{0.57}$ | 52 kB | 51 kB | $\color{green}{0.98}$ | $\color{red}{-0.1}$ |
| 1440×900 | 2 | 33.1 | $\color{green}{16.9}$ | $\color{green}{0.68}$ | 58 kB | 57 kB | $\color{green}{0.98}$ | $\color{red}{-0.1}$ |
| 1600×1000 | 2 | 60.0 | $\color{green}{24.4}$ | $\color{green}{0.41}$ | 80 kB | 79 kB | $\color{green}{0.99}$ | 0.0 |
| 1920×1080 | 2 | 53.0 | $\color{green}{24.7}$ | $\color{green}{0.59}$ | 91 kB | 90 kB | $\color{green}{0.99}$ | 0.0 |
| 2048×1536@2 | 2 | 56.1 | $\color{green}{31.9}$ | $\color{green}{0.57}$ | 107 kB | 107 kB | 1.00 | 0.0 |
| 2560×1600@2 | 2 | 75.3 | $\color{green}{34.8}$ | $\color{green}{0.71}$ | 118 kB | 115 kB | $\color{green}{0.98}$ | 0.0 |
| 2880×1800@2 | 2 | 140.3 | $\color{green}{52.0}$ | $\color{green}{0.64}$ | 131 kB | 128 kB | $\color{green}{0.98}$ | 0.0 |
| 3200×2000@2 | 2 | 194.2 | $\color{green}{56.3}$ | $\color{green}{0.96}$ | 183 kB | 180 kB | $\color{green}{0.99}$ | 0.0 |
| 3456×2168@2 | 2 | 100.7 | $\color{green}{73.2}$ | $\color{green}{0.73}$ | 207 kB | 205 kB | $\color{green}{0.99}$ | 0.0 |
| 3840×2160@2 | 2 | 92.5 | $\color{green}{64.6}$ | $\color{green}{0.70}$ | 206 kB | 203 kB | $\color{green}{0.99}$ | 0.0 |

### Keyframes inside `video`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 3 | 18.3 | $\color{green}{12.4}$ | $\color{green}{0.55}$ | 49 kB | 45 kB | $\color{green}{0.92}$ | 0.0 |
| 1440×900 | 3 | 17.0 | $\color{green}{12.4}$ | $\color{green}{0.55}$ | 51 kB | 46 kB | $\color{green}{0.91}$ | 0.0 |
| 1600×1000 | 3 | 17.3 | $\color{green}{12.9}$ | $\color{green}{0.65}$ | 51 kB | 47 kB | $\color{green}{0.91}$ | 0.0 |
| 1920×1080 | 3 | 16.7 | $\color{green}{14.5}$ | $\color{green}{0.64}$ | 51 kB | 46 kB | $\color{green}{0.91}$ | 0.0 |
| 2048×1536@2 | 3 | 22.3 | $\color{green}{22.2}$ | $\color{green}{0.77}$ | 104 kB | 97 kB | $\color{green}{0.93}$ | $\color{red}{-0.1}$ |
| 2560×1600@2 | 3 | 28.4 | $\color{red}{30.1}$ | $\color{green}{0.73}$ | 116 kB | 104 kB | $\color{green}{0.90}$ | 0.0 |
| 2880×1800@2 | 3 | 34.1 | $\color{green}{30.8}$ | $\color{green}{0.65}$ | 120 kB | 108 kB | $\color{green}{0.90}$ | $\color{green}{+0.1}$ |
| 3200×2000@2 | 3 | 37.6 | $\color{green}{33.2}$ | $\color{green}{0.69}$ | 126 kB | 113 kB | $\color{green}{0.90}$ | 0.0 |
| 3456×2168@2 | 3 | 44.1 | $\color{green}{38.6}$ | $\color{green}{0.54}$ | 132 kB | 116 kB | $\color{green}{0.88}$ | $\color{green}{+0.1}$ |
| 3840×2160@2 | 3 | 39.9 | $\color{green}{37.0}$ | $\color{green}{0.67}$ | 117 kB | 104 kB | $\color{green}{0.89}$ | 0.0 |

## Against libvpx, keyframes of each capture's first picture

### A keyframe at dial 40: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.79}$ | $\color{green}{0.84}$ | $\color{green}{0.85}$ | $\color{green}{0.77}$ | $\color{green}{0.99}$ | $\color{red}{1.02}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ |
| `flood` | $\color{green}{0.70}$ | $\color{green}{0.85}$ | $\color{green}{0.95}$ | $\color{green}{0.88}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.91}$ | $\color{green}{0.98}$ | 1.00 |
| `browser` | $\color{green}{0.58}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.62}$ | $\color{green}{0.73}$ | $\color{green}{0.73}$ | $\color{green}{0.72}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.73}$ |
| `drag` | $\color{green}{0.75}$ | $\color{green}{0.85}$ | $\color{green}{0.89}$ | $\color{green}{0.83}$ | $\color{green}{0.92}$ | $\color{green}{0.97}$ | $\color{green}{0.91}$ | $\color{green}{0.95}$ | $\color{green}{0.90}$ | $\color{green}{0.98}$ |
| `video` | $\color{green}{0.79}$ | $\color{green}{0.84}$ | $\color{green}{0.81}$ | $\color{green}{0.78}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.92}$ | $\color{green}{0.97}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ |
| `walk` | $\color{green}{0.66}$ | $\color{green}{0.70}$ | $\color{green}{0.79}$ | $\color{green}{0.77}$ | $\color{green}{0.92}$ | $\color{green}{0.96}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.98}$ | $\color{green}{0.94}$ |

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
| `uncover` | $\color{green}{0.66}$ | $\color{green}{0.77}$ | $\color{green}{0.78}$ | $\color{green}{0.67}$ | $\color{green}{0.98}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ | $\color{green}{0.92}$ | $\color{green}{0.91}$ |
| `flood` | $\color{green}{0.68}$ | $\color{green}{0.82}$ | $\color{green}{0.88}$ | $\color{green}{0.82}$ | $\color{green}{0.91}$ | $\color{green}{0.92}$ | $\color{green}{0.92}$ | $\color{green}{0.88}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ |
| `browser` | $\color{green}{0.52}$ | $\color{green}{0.64}$ | $\color{green}{0.63}$ | $\color{green}{0.64}$ | $\color{green}{0.77}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ |
| `drag` | $\color{green}{0.70}$ | $\color{green}{0.78}$ | $\color{green}{0.78}$ | $\color{green}{0.81}$ | $\color{green}{0.95}$ | $\color{green}{0.97}$ | $\color{green}{0.92}$ | $\color{green}{0.90}$ | $\color{green}{0.86}$ | $\color{green}{0.99}$ |
| `video` | $\color{green}{0.71}$ | $\color{green}{0.75}$ | $\color{green}{0.77}$ | $\color{green}{0.67}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.88}$ | $\color{green}{0.81}$ | $\color{green}{0.88}$ | $\color{green}{0.83}$ |
| `walk` | $\color{green}{0.70}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ | $\color{green}{0.81}$ | $\color{green}{0.91}$ | $\color{green}{0.95}$ | $\color{green}{0.92}$ | $\color{green}{0.96}$ | $\color{green}{0.94}$ | $\color{green}{0.90}$ |

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
| `uncover` | $\color{green}{0.61}$ | $\color{green}{0.71}$ | $\color{green}{0.65}$ | $\color{green}{0.62}$ | $\color{green}{0.88}$ | $\color{green}{0.86}$ | $\color{green}{0.80}$ | $\color{green}{0.88}$ | $\color{green}{0.84}$ | $\color{green}{0.80}$ |
| `flood` | $\color{green}{0.60}$ | $\color{green}{0.74}$ | $\color{green}{0.80}$ | $\color{green}{0.76}$ | $\color{green}{0.94}$ | $\color{green}{0.86}$ | $\color{green}{0.88}$ | $\color{green}{0.91}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ |
| `browser` | $\color{green}{0.56}$ | $\color{green}{0.60}$ | $\color{green}{0.56}$ | $\color{green}{0.67}$ | $\color{green}{0.72}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.73}$ | $\color{green}{0.75}$ | $\color{green}{0.73}$ |
| `drag` | $\color{green}{0.64}$ | $\color{green}{0.72}$ | $\color{green}{0.77}$ | $\color{green}{0.75}$ | $\color{green}{0.87}$ | $\color{green}{0.90}$ | $\color{green}{0.84}$ | $\color{green}{0.89}$ | $\color{green}{0.84}$ | $\color{green}{0.87}$ |
| `video` | $\color{green}{0.62}$ | $\color{green}{0.69}$ | $\color{green}{0.74}$ | $\color{green}{0.63}$ | $\color{green}{0.78}$ | $\color{green}{0.87}$ | $\color{green}{0.77}$ | $\color{green}{0.70}$ | $\color{green}{0.81}$ | $\color{green}{0.78}$ |
| `walk` | $\color{green}{0.70}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{green}{0.81}$ | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.92}$ |

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
| `uncover` | $\color{green}{0.65}$ | $\color{green}{0.79}$ | $\color{green}{0.68}$ | $\color{green}{0.67}$ | $\color{red}{1.08}$ | $\color{green}{0.85}$ | $\color{green}{0.82}$ | $\color{green}{0.89}$ | $\color{green}{0.86}$ | $\color{green}{0.83}$ |
| `flood` | $\color{green}{0.69}$ | $\color{green}{0.90}$ | $\color{green}{0.78}$ | $\color{green}{0.79}$ | $\color{red}{1.05}$ | $\color{green}{0.86}$ | $\color{green}{0.89}$ | $\color{green}{0.92}$ | $\color{green}{0.92}$ | $\color{green}{0.94}$ |
| `browser` | $\color{green}{0.71}$ | $\color{green}{0.56}$ | $\color{green}{0.58}$ | $\color{green}{0.60}$ | $\color{green}{0.68}$ | $\color{green}{0.68}$ | $\color{green}{0.68}$ | $\color{green}{0.61}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ |
| `drag` | $\color{green}{0.73}$ | $\color{green}{0.89}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{red}{1.08}$ | $\color{green}{0.89}$ | $\color{green}{0.88}$ | $\color{green}{0.81}$ | $\color{green}{0.88}$ | $\color{green}{0.87}$ |
| `video` | $\color{green}{0.73}$ | $\color{green}{0.82}$ | $\color{green}{0.69}$ | $\color{green}{0.63}$ | $\color{red}{1.01}$ | $\color{green}{0.83}$ | $\color{green}{0.77}$ | $\color{green}{0.79}$ | $\color{green}{0.80}$ | $\color{green}{0.79}$ |
| `walk` | $\color{green}{0.78}$ | $\color{green}{0.88}$ | $\color{green}{0.71}$ | $\color{green}{0.82}$ | $\color{red}{1.10}$ | $\color{green}{0.89}$ | $\color{green}{0.85}$ | $\color{green}{0.86}$ | $\color{green}{0.88}$ | $\color{green}{0.82}$ |

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
| `uncover` | $\color{red}{1.33}$ | $\color{red}{1.45}$ | $\color{red}{1.07}$ | $\color{red}{1.05}$ | $\color{red}{1.78}$ | $\color{red}{1.25}$ | $\color{red}{1.21}$ | $\color{red}{1.20}$ | $\color{red}{1.22}$ | $\color{red}{1.20}$ |
| `flood` | $\color{red}{1.31}$ | $\color{red}{1.40}$ | $\color{red}{1.09}$ | $\color{red}{1.03}$ | $\color{red}{1.73}$ | $\color{red}{1.08}$ | $\color{red}{1.03}$ | $\color{red}{1.05}$ | $\color{red}{1.12}$ | $\color{red}{1.07}$ |
| `browser` | $\color{green}{0.55}$ | $\color{green}{0.64}$ | $\color{green}{0.31}$ | $\color{green}{0.34}$ | $\color{green}{0.39}$ | $\color{green}{0.40}$ | $\color{green}{0.41}$ | $\color{green}{0.38}$ | $\color{green}{0.42}$ | $\color{green}{0.42}$ |
| `drag` | $\color{red}{1.30}$ | $\color{red}{1.44}$ | $\color{red}{1.06}$ | $\color{green}{0.93}$ | $\color{red}{1.61}$ | $\color{red}{1.09}$ | $\color{red}{1.01}$ | $\color{red}{1.10}$ | $\color{red}{1.01}$ | $\color{red}{1.04}$ |
| `video` | $\color{red}{1.24}$ | $\color{red}{1.18}$ | $\color{green}{0.75}$ | $\color{green}{0.72}$ | $\color{red}{1.55}$ | $\color{green}{0.92}$ | $\color{green}{0.82}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.71}$ |
| `walk` | $\color{red}{1.19}$ | $\color{red}{1.20}$ | $\color{green}{0.84}$ | $\color{green}{0.89}$ | $\color{red}{1.53}$ | $\color{red}{1.02}$ | 1.00 | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.84}$ |

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
| `uncover` | $\color{red}{1.41}$ | $\color{red}{1.56}$ | $\color{green}{0.96}$ | $\color{green}{0.98}$ | $\color{red}{1.80}$ | $\color{red}{1.22}$ | $\color{red}{1.12}$ | $\color{red}{1.08}$ | $\color{red}{1.14}$ | $\color{red}{1.11}$ |
| `flood` | $\color{red}{1.19}$ | $\color{red}{1.44}$ | $\color{green}{0.90}$ | $\color{green}{0.96}$ | $\color{red}{1.59}$ | $\color{green}{0.96}$ | $\color{green}{0.91}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.93}$ |
| `browser` | $\color{green}{0.61}$ | $\color{green}{0.55}$ | $\color{green}{0.34}$ | $\color{green}{0.34}$ | $\color{green}{0.36}$ | $\color{green}{0.36}$ | $\color{green}{0.38}$ | $\color{green}{0.38}$ | $\color{green}{0.41}$ | $\color{green}{0.41}$ |
| `drag` | $\color{red}{1.36}$ | $\color{red}{1.46}$ | $\color{green}{0.96}$ | $\color{green}{0.87}$ | $\color{red}{1.53}$ | $\color{green}{0.95}$ | $\color{green}{0.88}$ | 1.00 | $\color{green}{0.88}$ | $\color{green}{0.93}$ |
| `video` | $\color{red}{1.26}$ | $\color{red}{1.21}$ | $\color{green}{0.73}$ | $\color{green}{0.65}$ | $\color{red}{1.40}$ | $\color{green}{0.85}$ | $\color{green}{0.75}$ | $\color{green}{0.70}$ | $\color{green}{0.65}$ | $\color{green}{0.64}$ |
| `walk` | $\color{red}{1.19}$ | $\color{red}{1.24}$ | $\color{green}{0.78}$ | $\color{green}{0.82}$ | $\color{red}{1.46}$ | $\color{red}{1.04}$ | $\color{green}{0.97}$ | $\color{green}{0.94}$ | $\color{green}{0.98}$ | $\color{green}{0.80}$ |

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
| `uncover` | $\color{red}{1.36}$ | $\color{red}{1.44}$ | $\color{green}{0.95}$ | $\color{green}{0.92}$ | $\color{red}{1.58}$ | $\color{green}{0.92}$ | $\color{green}{0.88}$ | $\color{green}{0.83}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ |
| `flood` | $\color{red}{1.22}$ | $\color{red}{1.18}$ | $\color{green}{0.72}$ | $\color{green}{0.74}$ | $\color{red}{1.32}$ | $\color{green}{0.75}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ |
| `browser` | $\color{green}{0.47}$ | $\color{green}{0.56}$ | $\color{green}{0.35}$ | $\color{green}{0.35}$ | $\color{green}{0.37}$ | $\color{green}{0.38}$ | $\color{green}{0.36}$ | $\color{green}{0.39}$ | $\color{green}{0.41}$ | $\color{green}{0.41}$ |
| `drag` | $\color{red}{1.19}$ | $\color{red}{1.24}$ | $\color{green}{0.76}$ | $\color{green}{0.70}$ | $\color{red}{1.25}$ | $\color{green}{0.77}$ | $\color{green}{0.72}$ | $\color{green}{0.84}$ | $\color{green}{0.71}$ | $\color{green}{0.72}$ |
| `video` | $\color{red}{1.17}$ | $\color{red}{1.06}$ | $\color{green}{0.60}$ | $\color{green}{0.55}$ | $\color{red}{1.28}$ | $\color{green}{0.68}$ | $\color{green}{0.62}$ | $\color{green}{0.59}$ | $\color{green}{0.57}$ | $\color{green}{0.55}$ |
| `walk` | $\color{red}{1.24}$ | $\color{red}{1.29}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{red}{1.56}$ | $\color{green}{0.92}$ | $\color{green}{0.87}$ | $\color{green}{0.90}$ | $\color{green}{0.91}$ | $\color{green}{0.75}$ |

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
