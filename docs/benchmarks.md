# What the encoder was measured at

The numbers the encoder's decisions were taken on. How the encoder works is
in [encoder.md](encoder.md); what was built in which order, and why, in
[plan.md](plan.md).

The releases named here, 0.0.1-alpha.2 to alpha.5, are those of
`screen-vp9-native`, the crate this encoder was until it came home to
`screen-vp9` as `Coder::Native`. Everything here is against libvpx as
`screen-vp9` 0.0.14 set it up, which is the crate's other coder now
(`src/libvpx.rs`), by the three
measurements in `src/native/mod.rs`: `captures_against_libvpx` plays each
capture into both encoders, frame by frame, turn about;
`keyframes_against_libvpx` codes each capture's first picture as a keyframe
at dials 40, 65 and 90; `settles_against_libvpx` codes it at one dial and
settles it at 90, the picture unchanged. The last two keep the least time of
five passes, each on one encoder kept across them, as a session's is. `tools/tables.py` makes the tables below from their logs.

They were run on 2026-10-10 on a six-core Intel i5-8500T (AVX2) under WSL2,
four threads, the machine idle and one measurement at a time, on the encoder
of commit `29a7849`. A time on this machine moves by a tenth from one run to
the next, so a ratio within a tenth of 1.00 is on the line. The same
measurements on an Apple M2 Max, the NEON kernels against libvpx there, are
in [benchmarks-m2-max.md](benchmarks-m2-max.md): the two are not to be read
against each other.

Throughout, a *ratio* is this encoder over libvpx, so under 1.00 is fewer
bytes or less time; a *PSNR difference* is this encoder's luma PSNR less
libvpx's at the same dial, so above zero is a better picture. `@2` marks a
desktop drawn at scale 2. Green is this encoder's side of the line and red
the other; a number on the line is left plain.

## What the tables say

- **Over a whole capture the picture is libvpx's or better in 69 cells of
  70**, the one under being `uncover` at 1920×1080 (−0.1 dB): `browser`
  +0.8 to +1.6 dB at 0.15–0.28× the bytes, `drag` +0.1 to +1.6 at
  0.12–0.39×, `flood` +0.3 to +0.7 at 0.74–0.97×, `terminal` +0.1 to +0.5
  at 0.76–0.86×, `video` +0.5 to +0.9 at 0.95–1.04×, `walk` +0.1 to +1.1
  at 0.99–1.10×, `uncover` at 0.94–1.04×.
- **Every capture at every size takes less time than libvpx**, frame for
  frame and summed: a line of text 0.14–0.19, `uncover` 0.23–0.31,
  `flood` 0.39–0.58, `browser` 0.48–0.54, `drag` 0.53–0.72, `video`
  0.63–0.79, `walk` 0.65–0.82. `video` and `walk` are the nearest the line,
  and nearer it from 1920 wide up.
- **A keyframe inside a capture is 0.2 to 0.3 dB over libvpx's** at
  0.98–1.01× the bytes (`video`'s 0.91–0.93×) in less than its time; no
  cell of forty is over 1.00.
- **A first picture coded alone as a keyframe is 0.2 to 0.6 dB over
  libvpx's at its bytes**: +0.3 to +0.5 at dial 90, +0.4 to +0.8 at 65
  and +0.2 to +0.7 at 40 (`walk` −0.1 to +0.9 there), at 0.98–1.03× the
  bytes (`walk` 0.67–0.84×, the flat desktop of `browser` 0.22–0.38×).
  Up to 1920 wide it takes 0.53–0.92 of libvpx's time; from 2048 wide, in
  four tile columns and without the loop filter, 0.67–1.06, ten cells of
  108 over 1.00, seven of them at dial 40, by 0.06 at most. `browser`'s
  +30 dB at dial 40 is a flat desktop this encoder holds exactly. The
  search's anchors are not made on a keyframe but on the first frame
  after it that changes the picture, so that frame is where 4 to 9 ms of
  this went.
- **A settle of a busy picture takes more time than libvpx's**
  (`uncover`, `flood`, `drag`): from 65, 1.12 to 1.77 of it, for 1.0 to
  1.4 dB more picture at 0.96–1.00× the bytes; from 80, 1.12 to 1.70 for
  0.0 to +0.2 dB at 0.68–0.84×; from 89, 0.93 to 1.30 for +0.3 to +0.4 dB
  at 0.44–1.03×; from 20, 0.61 to 1.10 for +0.1 to +0.4 dB at 0.95–1.02×.
  It is nearer the line up to 1920 wide (1.00–1.44 from 65) than from
  2048 wide (up to 1.77). This encoder's settle lands nearer the source,
  and the time is what that costs: coded in the largest transform alone,
  as an experiment before the rounding below, the settle of `uncover` at
  1920×1080 took 16 ms in place of 22 to 26 and came to 43.80 dB in place
  of 45.70, libvpx's picture, where libvpx takes 17. It is left as it
  is, a settle being there to reach the ceiling in one frame.
- **libvpx's settle time used to come out at one of two levels, and that
  was the measurement.** It made each encoder afresh for every pass, and
  a fresh libvpx touches its second frame's memory for the first time:
  the settle paid the page faults, or did not where the allocator handed
  back memory an earlier capture in the process had touched. `uncover`
  from 65 at 1600×1000, alone in a process, took libvpx 19.2 to 22.2 ms
  in ten runs; 14.8 to 15.8 with the allocator told to keep its heap
  (`glibc.malloc.mmap_threshold`, `trim_threshold`, `top_pad`) and 19.6
  to 20.7 told to map every block. This encoder's was the same 18.5 to
  20 either way. One encoder of each is kept across the passes now, as a
  session keeps its, and libvpx's settle is 0.5 to 0.65 of what the
  tables before held for it, at every size: the ratios above are that,
  and not a slower settle here.
- **An intra block's coefficients are rounded as libvpx rounds them.**
  libvpx quantizes an intra block with `vpx_quantize_b`, every coefficient
  rounded up at 48/128 of a step, and an inter block with
  `vp9_quantize_fp`, at 42/128 past the DC; this encoder took the second
  for both. At 48 a keyframe at 1920×1080 is 0.42 to 0.62 dB better at
  dials 89 and 90 for 0.1% more bytes, 0.44 to 0.49 at 65 for 0.1 to
  0.4%, and 0.35 to 0.47 at 40 for 1.3 to 2.1% (`walk`'s 0.10 to 0.44 dB
  for 2.1 to 4.3%); a whole capture is 0.03 to 0.17 dB better for −0.7
  to +1.1% of the bytes in the same time. It was the 0.05 to 0.23 dB a
  keyframe at the top of the dial was under libvpx's, and what a settle
  of one dial step was short of. Tried beside it at 1920×1080: libvpx's
  dead zone (nothing under 84/128 of a step) changes nothing at dial 90
  and trades 0.1 dB for 0.5% of the bytes at 40; 56/128 buys a keyframe
  0.4 dB more for 0.5% more bytes but `walk` 3% more over its capture
  for 0.11 dB, and `drag`'s capture 0.06 dB less; inter blocks at 48 too
  cost `video` 2.6% and `walk` 3.6% of their captures.
- **What is over the line in bytes**: a flat desktop settled from dial 65
  and up at 1.0 to 2.1× (tens of bytes); `walk`'s captures at up to
  1.10×, `uncover`'s and `video`'s at up to 1.04×. Settled from dial 20 it
  was at 4 to 10× (one or two kilobytes against a few hundred bytes) and
  is at 0.47 to 0.92×, `browser`'s row below, since 2026-10-11: the
  keyframe at 20 leaves the whole of one chroma plane a value off, its
  first block's rounding carried along by DC, and the settle had a
  coefficient in every 32×32 to make it up, where each square is now
  predicted from the ones settled before it with nothing to code.

## What changed between releases 0.0.1-alpha.3 and 0.0.1-alpha.4

Measured on the same machine before and after, each alone. Every stream is
alpha.3's, byte for byte.

| Change | What it bought |
|---|---|
| A phase of a frame is a queue the calling thread works too and waits on no thread that has not woken; a worker spins a tenth of a millisecond before it parks; a frame naming 1024 8×8s or fewer is coded on one thread | waking a parked thread costs 0.2 ms on this machine and a frame did it seven times: a line of text 2.1 → 0.41 ms at 1280×800, 2.06 → 0.57 at 1920×1080, 3.23 → 1.48 at 3840×2160 |
| A superblock a frame does not touch is recorded without being looked at, and the mode info of an 8×8 is sixteen bytes | with the row above; `uncover`'s frames 2.34 → 1.15 ms at 1280×800, 7.20 → 5.98 at 3840×2160 |
| A keyframe leaves the search's anchors to the frame after it | a first picture's keyframe 5 to 25% less: `walk` at 2560×1600, dial 40, 25.6 → 20.5 ms, `drag` at 3840×2160 75 → 64; the frame after pays it |
| A frame's symbols are counted a thread, not a superblock row | part of the rows above and below |
| The reference is made by whichever is fewer to copy: the superblocks a frame touched into it, or the ones it did not into the reconstruction | `video` 26.4 → 23.4 ms at 2560×1600, `walk` 23.3 → 19.9, `drag` 18.3 → 16.4 |
| The anchors a keyframe left wait for the first frame that changes the picture | a settle straight after a keyframe 3 ms less at 2048×1536: `drag` from 65 20 → 17.7 ms, `video` from 89 11.5 → 8.5 |

Against libvpx, alpha.3's cells and alpha.4's over all ten sizes:

| | 0.0.1-alpha.3 | 0.0.1-alpha.4 |
|---|---:|---:|
| A line of text, time frame for frame | 0.30–0.79 | 0.13–0.19 |
| `uncover`, time frame for frame | 0.39–0.53 | 0.25–0.31 |
| `drag`, time frame for frame | 0.58–0.80 | 0.54–0.73 |
| `video`, time frame for frame | 0.68–0.87 | 0.64–0.80 |
| `walk`, time frame for frame | 0.72–0.93 | 0.66–0.83 |
| `flood`, time frame for frame | 0.41–0.60 | 0.40–0.58 |
| A first picture's keyframe up to 1920 wide, time | 0.66–1.24 | 0.55–0.93 |
| A first picture's keyframe from 2048 wide, time | 0.87–1.27 | 0.67–1.04 |
| A keyframe inside a capture, time | 0.46–1.13 | 0.25–0.88 |
| A settle from 20, time | 0.57–0.92 | 0.46–0.88 |

Release 0.0.1-alpha.5, which the tables below are of: the search was gone
over, with every stream as it was (the 8×8s whose anchors are taken again
are marked a row at a time, what they held is kept only when the tables
are brought up to date by it, the tables are filled in one phase, a hash
is spread over the bits its table reads, and a table is picked without a
division: the search takes 13% less on `video` and `flood` at 1920×1080,
a frame of `browser` 5% less, of `drag` 2 to 4%, of `walk` at 2560×1600
4%, and of `video` nothing to 1%); an intra block is rounded at 48/128,
which changes every stream; and a settle is measured on warm encoders,
which moves libvpx's settle times and no stream.

Looked at and left: the tile phase of a busy keyframe runs four threads at
75 to 80% of four, with no thread found waiting; a frame of `video` or
`walk` has no one hot spot (the token estimate, the block walk, the intra
mode and the transforms 5 to 11% each, the search 11 to 14%); a busy
settle's time is in the transform sizes under the largest, which are its
1.5 to 2 dB (above). An inter block whose largest transform codes nothing
tried no smaller, as an experiment: a settle of one dial step took 20 to
35% fewer bytes for 0.07 to 0.24 dB less and one from 65 2 to 4% fewer for
0.15 to 0.4 dB less, in the same time; not taken.

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
| `terminal` | $\color{green}{0.85}$ | $\color{green}{0.83}$ | $\color{green}{0.83}$ | $\color{green}{0.84}$ | $\color{green}{0.76}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ | $\color{green}{0.86}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ |
| `uncover` | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.94}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{green}{0.96}$ | $\color{red}{1.04}$ |
| `flood` | $\color{green}{0.84}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.76}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ |
| `browser` | $\color{green}{0.20}$ | $\color{green}{0.17}$ | $\color{green}{0.25}$ | $\color{green}{0.28}$ | $\color{green}{0.24}$ | $\color{green}{0.15}$ | $\color{green}{0.17}$ | $\color{green}{0.20}$ | $\color{green}{0.22}$ | $\color{green}{0.26}$ |
| `drag` | $\color{green}{0.14}$ | $\color{green}{0.15}$ | $\color{green}{0.17}$ | $\color{green}{0.12}$ | $\color{green}{0.22}$ | $\color{green}{0.31}$ | $\color{green}{0.39}$ | $\color{green}{0.30}$ | $\color{green}{0.32}$ | $\color{green}{0.12}$ |
| `video` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | 1.00 | $\color{red}{1.03}$ | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.04}$ | $\color{green}{0.95}$ |
| `walk` | $\color{red}{1.08}$ | $\color{red}{1.10}$ | $\color{red}{1.10}$ | $\color{red}{1.06}$ | $\color{red}{1.09}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{green}{0.99}$ | $\color{red}{1.10}$ | $\color{red}{1.06}$ |

### Encode time, the median of the per-frame ratios

Under 1.00 is faster than libvpx, frame for frame.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.18}$ | $\color{green}{0.17}$ | $\color{green}{0.15}$ | $\color{green}{0.14}$ | $\color{green}{0.19}$ | $\color{green}{0.17}$ | $\color{green}{0.17}$ | $\color{green}{0.17}$ | $\color{green}{0.15}$ | $\color{green}{0.15}$ |
| `uncover` | $\color{green}{0.23}$ | $\color{green}{0.25}$ | $\color{green}{0.27}$ | $\color{green}{0.28}$ | $\color{green}{0.29}$ | $\color{green}{0.30}$ | $\color{green}{0.29}$ | $\color{green}{0.29}$ | $\color{green}{0.31}$ | $\color{green}{0.30}$ |
| `flood` | $\color{green}{0.39}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ | $\color{green}{0.58}$ | $\color{green}{0.57}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ |
| `browser` | $\color{green}{0.48}$ | $\color{green}{0.53}$ | $\color{green}{0.52}$ | $\color{green}{0.53}$ | $\color{green}{0.54}$ | $\color{green}{0.52}$ | $\color{green}{0.49}$ | $\color{green}{0.48}$ | $\color{green}{0.49}$ | $\color{green}{0.53}$ |
| `drag` | $\color{green}{0.54}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.53}$ | $\color{green}{0.70}$ | $\color{green}{0.72}$ | $\color{green}{0.67}$ | $\color{green}{0.64}$ | $\color{green}{0.65}$ | $\color{green}{0.63}$ |
| `video` | $\color{green}{0.63}$ | $\color{green}{0.64}$ | $\color{green}{0.68}$ | $\color{green}{0.77}$ | $\color{green}{0.79}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ |
| `walk` | $\color{green}{0.65}$ | $\color{green}{0.67}$ | $\color{green}{0.67}$ | $\color{green}{0.77}$ | $\color{green}{0.82}$ | $\color{green}{0.77}$ | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.75}$ | $\color{green}{0.72}$ |

### Encode time, the whole capture's

Every frame's time summed, this encoder's over libvpx's: where the long frames weigh.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.26}$ | $\color{green}{0.22}$ | $\color{green}{0.21}$ | $\color{green}{0.19}$ | $\color{green}{0.27}$ | $\color{green}{0.24}$ | $\color{green}{0.23}$ | $\color{green}{0.22}$ | $\color{green}{0.21}$ | $\color{green}{0.21}$ |
| `uncover` | $\color{green}{0.34}$ | $\color{green}{0.37}$ | $\color{green}{0.39}$ | $\color{green}{0.37}$ | $\color{green}{0.48}$ | $\color{green}{0.43}$ | $\color{green}{0.40}$ | $\color{green}{0.45}$ | $\color{green}{0.43}$ | $\color{green}{0.44}$ |
| `flood` | $\color{green}{0.41}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ | $\color{green}{0.57}$ | $\color{green}{0.58}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.56}$ |
| `browser` | $\color{green}{0.48}$ | $\color{green}{0.52}$ | $\color{green}{0.51}$ | $\color{green}{0.53}$ | $\color{green}{0.54}$ | $\color{green}{0.52}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ | $\color{green}{0.49}$ | $\color{green}{0.54}$ |
| `drag` | $\color{green}{0.55}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.53}$ | $\color{green}{0.69}$ | $\color{green}{0.71}$ | $\color{green}{0.66}$ | $\color{green}{0.62}$ | $\color{green}{0.64}$ | $\color{green}{0.60}$ |
| `video` | $\color{green}{0.63}$ | $\color{green}{0.65}$ | $\color{green}{0.68}$ | $\color{green}{0.78}$ | $\color{green}{0.79}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ |
| `walk` | $\color{green}{0.65}$ | $\color{green}{0.68}$ | $\color{green}{0.67}$ | $\color{green}{0.77}$ | $\color{green}{0.83}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.75}$ | $\color{green}{0.72}$ |

### Luma PSNR, this encoder less libvpx, in dB

Above zero is a better picture than libvpx at the same dial.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.1}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ |
| `browser` | $\color{green}{+1.0}$ | $\color{green}{+1.0}$ | $\color{green}{+1.1}$ | $\color{green}{+0.8}$ | $\color{green}{+1.4}$ | $\color{green}{+1.6}$ | $\color{green}{+1.3}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ |
| `drag` | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ | $\color{green}{+1.0}$ | $\color{green}{+1.6}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+1.4}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.9}$ | $\color{green}{+0.6}$ |
| `walk` | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.9}$ | $\color{green}{+1.1}$ |

## Against libvpx, the frames apart

The time columns are each encoder's median time for a frame of that kind, the
time ratio the median of the per-frame ratios, and the bytes the mean per frame.

### `terminal`, a line of text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 230 | 2.26 | $\color{green}{0.42}$ | $\color{green}{0.18}$ | 123 B | 124 B | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1440×900 | 269 | 3.19 | $\color{green}{0.53}$ | $\color{green}{0.16}$ | 119 B | 142 B | $\color{red}{1.20}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 270 | 3.52 | $\color{green}{0.53}$ | $\color{green}{0.15}$ | 121 B | 122 B | 1.00 | $\color{green}{+0.3}$ |
| 1920×1080 | 271 | 4.40 | $\color{green}{0.60}$ | $\color{green}{0.14}$ | 134 B | 132 B | $\color{green}{0.98}$ | $\color{green}{+0.1}$ |
| 2048×1536@2 | 268 | 4.40 | $\color{green}{0.85}$ | $\color{green}{0.19}$ | 205 B | 184 B | $\color{green}{0.90}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 270 | 5.40 | $\color{green}{0.92}$ | $\color{green}{0.17}$ | 197 B | 187 B | $\color{green}{0.95}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 270 | 6.82 | $\color{green}{1.13}$ | $\color{green}{0.17}$ | 190 B | 224 B | $\color{red}{1.18}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 232 | 8.18 | $\color{green}{1.36}$ | $\color{green}{0.16}$ | 209 B | 217 B | $\color{red}{1.04}$ | $\color{green}{+0.3}$ |
| 3456×2168@2 | 265 | 9.66 | $\color{green}{1.42}$ | $\color{green}{0.14}$ | 218 B | 208 B | $\color{green}{0.95}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 264 | 10.5 | $\color{green}{1.56}$ | $\color{green}{0.15}$ | 221 B | 219 B | $\color{green}{0.99}$ | $\color{green}{+0.4}$ |

### `terminal`, a scroll

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 8 | 11.2 | $\color{green}{7.21}$ | $\color{green}{0.39}$ | 40 kB | 34 kB | $\color{green}{0.84}$ | $\color{green}{+0.1}$ |
| 1440×900 | 8 | 13.6 | $\color{green}{7.43}$ | $\color{green}{0.55}$ | 46 kB | 36 kB | $\color{green}{0.79}$ | $\color{green}{+0.1}$ |
| 1600×1000 | 7 | 12.4 | $\color{green}{8.36}$ | $\color{green}{0.52}$ | 61 kB | 50 kB | $\color{green}{0.82}$ | $\color{red}{-0.1}$ |
| 1920×1080 | 7 | 14.3 | $\color{green}{8.64}$ | $\color{green}{0.42}$ | 67 kB | 56 kB | $\color{green}{0.83}$ | 0.0 |
| 2048×1536@2 | 8 | 25.3 | $\color{green}{18.4}$ | $\color{green}{0.45}$ | 102 kB | 76 kB | $\color{green}{0.74}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 8 | 27.3 | $\color{green}{20.2}$ | $\color{green}{0.54}$ | 114 kB | 86 kB | $\color{green}{0.76}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 8 | 37.1 | $\color{green}{19.1}$ | $\color{green}{0.49}$ | 122 kB | 91 kB | $\color{green}{0.75}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 7 | 34.5 | $\color{green}{16.2}$ | $\color{green}{0.47}$ | 122 kB | 104 kB | $\color{green}{0.85}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 7 | 34.8 | $\color{green}{33.8}$ | $\color{green}{0.48}$ | 169 kB | 144 kB | $\color{green}{0.86}$ | $\color{green}{+0.3}$ |
| 3840×2160@2 | 6 | 49.7 | $\color{green}{33.3}$ | $\color{green}{0.48}$ | 201 kB | 169 kB | $\color{green}{0.84}$ | $\color{green}{+0.3}$ |

### `drag`, a quarter of the picture

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 394 | 10.9 | $\color{green}{5.86}$ | $\color{green}{0.54}$ | 6090 B | 594 B | $\color{green}{0.10}$ | $\color{green}{+1.4}$ |
| 1440×900 | 394 | 10.5 | $\color{green}{5.89}$ | $\color{green}{0.56}$ | 6112 B | 668 B | $\color{green}{0.11}$ | $\color{green}{+1.3}$ |
| 1600×1000 | 398 | 10.9 | $\color{green}{6.07}$ | $\color{green}{0.56}$ | 6037 B | 682 B | $\color{green}{0.11}$ | $\color{green}{+1.0}$ |
| 1920×1080 | 391 | 11.6 | $\color{green}{6.15}$ | $\color{green}{0.53}$ | 8512 B | 729 B | $\color{green}{0.09}$ | $\color{green}{+1.6}$ |
| 2048×1536@2 | 344 | 21.5 | $\color{green}{15.2}$ | $\color{green}{0.70}$ | 9859 B | 1807 B | $\color{green}{0.18}$ | $\color{green}{+0.1}$ |
| 2560×1600@2 | 371 | 22.2 | $\color{green}{15.8}$ | $\color{green}{0.72}$ | 7199 B | 1755 B | $\color{green}{0.24}$ | $\color{green}{+0.1}$ |
| 2880×1800@2 | 371 | 24.0 | $\color{green}{16.0}$ | $\color{green}{0.67}$ | 6983 B | 2311 B | $\color{green}{0.33}$ | $\color{green}{+0.1}$ |
| 3200×2000@2 | 331 | 26.3 | $\color{green}{16.9}$ | $\color{green}{0.64}$ | 12 kB | 2647 B | $\color{green}{0.23}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 355 | 27.2 | $\color{green}{17.4}$ | $\color{green}{0.65}$ | 10 kB | 2714 B | $\color{green}{0.26}$ | $\color{green}{+0.5}$ |
| 3840×2160@2 | 340 | 26.8 | $\color{green}{16.8}$ | $\color{green}{0.63}$ | 31 kB | 2836 B | $\color{green}{0.09}$ | $\color{green}{+1.4}$ |

### `browser`, a page scrolled

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 206 | 19.3 | $\color{green}{9.30}$ | $\color{green}{0.48}$ | 11 kB | 2265 B | $\color{green}{0.20}$ | $\color{green}{+1.0}$ |
| 1440×900 | 219 | 20.6 | $\color{green}{10.8}$ | $\color{green}{0.53}$ | 16 kB | 2767 B | $\color{green}{0.17}$ | $\color{green}{+1.0}$ |
| 1600×1000 | 223 | 23.5 | $\color{green}{12.1}$ | $\color{green}{0.52}$ | 15 kB | 3791 B | $\color{green}{0.25}$ | $\color{green}{+1.1}$ |
| 1920×1080 | 171 | 28.0 | $\color{green}{14.8}$ | $\color{green}{0.53}$ | 17 kB | 4821 B | $\color{green}{0.28}$ | $\color{green}{+0.8}$ |
| 2048×1536@2 | 186 | 37.9 | $\color{green}{20.9}$ | $\color{green}{0.55}$ | 54 kB | 13 kB | $\color{green}{0.24}$ | $\color{green}{+1.4}$ |
| 2560×1600@2 | 137 | 52.8 | $\color{green}{27.8}$ | $\color{green}{0.52}$ | 75 kB | 11 kB | $\color{green}{0.15}$ | $\color{green}{+1.6}$ |
| 2880×1800@2 | 128 | 63.0 | $\color{green}{31.1}$ | $\color{green}{0.50}$ | 77 kB | 13 kB | $\color{green}{0.17}$ | $\color{green}{+1.3}$ |
| 3200×2000@2 | 85 | 80.4 | $\color{green}{38.5}$ | $\color{green}{0.48}$ | 94 kB | 19 kB | $\color{green}{0.20}$ | $\color{green}{+1.2}$ |
| 3456×2168@2 | 88 | 84.1 | $\color{green}{40.5}$ | $\color{green}{0.49}$ | 94 kB | 21 kB | $\color{green}{0.22}$ | $\color{green}{+1.3}$ |
| 3840×2160@2 | 94 | 83.9 | $\color{green}{43.2}$ | $\color{green}{0.53}$ | 90 kB | 23 kB | $\color{green}{0.26}$ | $\color{green}{+1.2}$ |

### `flood`, a window of new text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 149 | 134.9 | $\color{green}{51.5}$ | $\color{green}{0.39}$ | 675 kB | 565 kB | $\color{green}{0.84}$ | $\color{green}{+0.3}$ |
| 1440×900 | 149 | 112.0 | $\color{green}{53.9}$ | $\color{green}{0.49}$ | 673 kB | 517 kB | $\color{green}{0.77}$ | $\color{green}{+0.5}$ |
| 1600×1000 | 149 | 149.8 | $\color{green}{67.7}$ | $\color{green}{0.47}$ | 936 kB | 689 kB | $\color{green}{0.74}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 149 | 215.1 | $\color{green}{97.7}$ | $\color{green}{0.46}$ | 1430 kB | 1083 kB | $\color{green}{0.76}$ | $\color{green}{+0.4}$ |
| 2048×1536@2 | 149 | 215.4 | $\color{green}{122.6}$ | $\color{green}{0.58}$ | 1372 kB | 1312 kB | $\color{green}{0.96}$ | $\color{green}{+0.6}$ |
| 2560×1600@2 | 149 | 283.2 | $\color{green}{161.7}$ | $\color{green}{0.57}$ | 1803 kB | 1746 kB | $\color{green}{0.97}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 149 | 374.4 | $\color{green}{208.4}$ | $\color{green}{0.56}$ | 2346 kB | 2276 kB | $\color{green}{0.97}$ | $\color{green}{+0.6}$ |
| 3200×2000@2 | 149 | 459.2 | $\color{green}{253.8}$ | $\color{green}{0.56}$ | 2897 kB | 2813 kB | $\color{green}{0.97}$ | $\color{green}{+0.7}$ |
| 3456×2168@2 | 149 | 547.4 | $\color{green}{299.4}$ | $\color{green}{0.55}$ | 3398 kB | 3297 kB | $\color{green}{0.97}$ | $\color{green}{+0.7}$ |
| 3840×2160@2 | 149 | 642.3 | $\color{green}{355.5}$ | $\color{green}{0.54}$ | 3784 kB | 3690 kB | $\color{green}{0.97}$ | $\color{green}{+0.7}$ |

### `uncover`, a window moved off a page

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 53 | 4.18 | $\color{green}{0.98}$ | $\color{green}{0.23}$ | 3563 B | 3266 B | $\color{green}{0.92}$ | $\color{green}{+0.2}$ |
| 1440×900 | 53 | 4.95 | $\color{green}{1.28}$ | $\color{green}{0.25}$ | 3978 B | 3774 B | $\color{green}{0.95}$ | $\color{green}{+0.3}$ |
| 1600×1000 | 54 | 5.82 | $\color{green}{1.55}$ | $\color{green}{0.27}$ | 5069 B | 4429 B | $\color{green}{0.87}$ | $\color{green}{+0.1}$ |
| 1920×1080 | 53 | 6.80 | $\color{green}{1.85}$ | $\color{green}{0.28}$ | 3796 B | 3946 B | $\color{red}{1.04}$ | $\color{red}{-0.1}$ |
| 2048×1536@2 | 54 | 7.95 | $\color{green}{2.32}$ | $\color{green}{0.29}$ | 13 kB | 13 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 54 | 9.82 | $\color{green}{2.86}$ | $\color{green}{0.30}$ | 8388 B | 8373 B | 1.00 | $\color{green}{+0.3}$ |
| 2880×1800@2 | 54 | 12.5 | $\color{green}{3.68}$ | $\color{green}{0.29}$ | 11 kB | 12 kB | $\color{red}{1.07}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 40 | 15.4 | $\color{green}{4.34}$ | $\color{green}{0.29}$ | 20 kB | 21 kB | $\color{red}{1.05}$ | $\color{green}{+0.3}$ |
| 3456×2168@2 | 54 | 18.0 | $\color{green}{5.28}$ | $\color{green}{0.31}$ | 12 kB | 11 kB | $\color{green}{0.90}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 53 | 19.3 | $\color{green}{5.68}$ | $\color{green}{0.30}$ | 15 kB | 16 kB | $\color{red}{1.07}$ | $\color{green}{+0.2}$ |

### `video`, a frame of the pattern

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 483 | 16.2 | $\color{green}{10.0}$ | $\color{green}{0.63}$ | 34 kB | 35 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 1440×900 | 489 | 16.1 | $\color{green}{10.2}$ | $\color{green}{0.64}$ | 35 kB | 36 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 1600×1000 | 487 | 15.8 | $\color{green}{10.7}$ | $\color{green}{0.68}$ | 35 kB | 36 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 1920×1080 | 474 | 13.0 | $\color{green}{10.1}$ | $\color{green}{0.78}$ | 36 kB | 36 kB | 1.00 | $\color{green}{+0.6}$ |
| 2048×1536@2 | 397 | 22.4 | $\color{green}{18.2}$ | $\color{green}{0.79}$ | 65 kB | 67 kB | $\color{red}{1.04}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 309 | 30.9 | $\color{green}{23.7}$ | $\color{green}{0.77}$ | 80 kB | 80 kB | 1.00 | $\color{green}{+0.5}$ |
| 2880×1800@2 | 255 | 39.6 | $\color{green}{30.0}$ | $\color{green}{0.77}$ | 97 kB | 99 kB | $\color{red}{1.02}$ | $\color{green}{+0.8}$ |
| 3200×2000@2 | 190 | 46.1 | $\color{green}{35.5}$ | $\color{green}{0.76}$ | 112 kB | 114 kB | $\color{red}{1.01}$ | $\color{green}{+0.8}$ |
| 3456×2168@2 | 172 | 55.3 | $\color{green}{40.1}$ | $\color{green}{0.77}$ | 122 kB | 128 kB | $\color{red}{1.05}$ | $\color{green}{+0.9}$ |
| 3840×2160@2 | 231 | 36.9 | $\color{green}{28.1}$ | $\color{green}{0.76}$ | 89 kB | 85 kB | $\color{green}{0.95}$ | $\color{green}{+0.6}$ |

### `walk`, a frame as the dial walks

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 239 | 11.1 | $\color{green}{7.18}$ | $\color{green}{0.65}$ | 12 kB | 14 kB | $\color{red}{1.09}$ | $\color{green}{+0.1}$ |
| 1440×900 | 234 | 10.7 | $\color{green}{7.46}$ | $\color{green}{0.67}$ | 13 kB | 14 kB | $\color{red}{1.10}$ | $\color{green}{+0.1}$ |
| 1600×1000 | 232 | 13.7 | $\color{green}{9.71}$ | $\color{green}{0.68}$ | 17 kB | 18 kB | $\color{red}{1.10}$ | $\color{green}{+0.3}$ |
| 1920×1080 | 227 | 10.6 | $\color{green}{8.24}$ | $\color{green}{0.77}$ | 14 kB | 15 kB | $\color{red}{1.07}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 230 | 16.7 | $\color{green}{13.9}$ | $\color{green}{0.82}$ | 27 kB | 30 kB | $\color{red}{1.10}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 196 | 26.6 | $\color{green}{20.6}$ | $\color{green}{0.77}$ | 43 kB | 45 kB | $\color{red}{1.03}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 195 | 25.2 | $\color{green}{18.9}$ | $\color{green}{0.75}$ | 35 kB | 36 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 3200×2000@2 | 174 | 31.1 | $\color{green}{23.6}$ | $\color{green}{0.76}$ | 47 kB | 46 kB | 1.00 | $\color{green}{+0.7}$ |
| 3456×2168@2 | 128 | 43.9 | $\color{green}{32.6}$ | $\color{green}{0.75}$ | 55 kB | 61 kB | $\color{red}{1.11}$ | $\color{green}{+0.9}$ |
| 3840×2160@2 | 102 | 53.7 | $\color{green}{37.0}$ | $\color{green}{0.72}$ | 68 kB | 73 kB | $\color{red}{1.07}$ | $\color{green}{+1.2}$ |

### `walk`, the settle a session asked for

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 4.95 | $\color{green}{2.69}$ | $\color{green}{0.55}$ | 157 B | 116 B | $\color{green}{0.74}$ | 0.0 |
| 1440×900 | 1 | 5.28 | $\color{green}{2.61}$ | $\color{green}{0.49}$ | 201 B | 182 B | $\color{green}{0.91}$ | 0.0 |
| 1600×1000 | 1 | 5.96 | $\color{green}{2.52}$ | $\color{green}{0.42}$ | 179 B | 147 B | $\color{green}{0.82}$ | 0.0 |
| 1920×1080 | 1 | 6.86 | $\color{green}{3.03}$ | $\color{green}{0.44}$ | 209 B | 127 B | $\color{green}{0.61}$ | 0.0 |
| 2048×1536@2 | 1 | 6.74 | $\color{green}{3.11}$ | $\color{green}{0.46}$ | 211 B | 126 B | $\color{green}{0.60}$ | 0.0 |
| 2880×1800@2 | 1 | 10.2 | $\color{green}{4.30}$ | $\color{green}{0.42}$ | 404 B | 148 B | $\color{green}{0.37}$ | 0.0 |
| 3456×2168@2 | 1 | 15.9 | $\color{green}{7.02}$ | $\color{green}{0.44}$ | 658 B | 179 B | $\color{green}{0.27}$ | 0.0 |
| 3840×2160@2 | 1 | 16.4 | $\color{green}{8.18}$ | $\color{green}{0.50}$ | 729 B | 173 B | $\color{green}{0.24}$ | 0.0 |

### Keyframes inside `uncover`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 46.0 | $\color{green}{25.6}$ | $\color{green}{0.56}$ | 104 kB | 105 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1440×900 | 2 | 38.9 | $\color{green}{24.2}$ | $\color{green}{0.62}$ | 112 kB | 114 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 2 | 49.0 | $\color{green}{31.6}$ | $\color{green}{0.68}$ | 144 kB | 146 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 2 | 61.6 | $\color{green}{36.2}$ | $\color{green}{0.67}$ | 161 kB | 162 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 2 | 58.6 | $\color{green}{49.1}$ | $\color{green}{0.84}$ | 223 kB | 224 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 2 | 75.8 | $\color{green}{60.9}$ | $\color{green}{0.80}$ | 273 kB | 275 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 2880×1800@2 | 2 | 91.1 | $\color{green}{68.3}$ | $\color{green}{0.75}$ | 295 kB | 296 kB | 1.00 | $\color{green}{+0.2}$ |
| 3200×2000@2 | 2 | 96.1 | $\color{green}{82.5}$ | $\color{green}{0.86}$ | 309 kB | 311 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 2 | 120.0 | $\color{green}{94.5}$ | $\color{green}{0.79}$ | 421 kB | 423 kB | 1.00 | $\color{green}{+0.2}$ |
| 3840×2160@2 | 2 | 129.6 | $\color{green}{99.3}$ | $\color{green}{0.82}$ | 420 kB | 423 kB | $\color{red}{1.01}$ | $\color{green}{+0.2}$ |

### Keyframes inside `drag`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 59.9 | $\color{green}{19.8}$ | $\color{green}{0.43}$ | 54 kB | 54 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 1440×900 | 2 | 65.6 | $\color{green}{21.6}$ | $\color{green}{0.47}$ | 65 kB | 64 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 2 | 88.2 | $\color{green}{28.5}$ | $\color{green}{0.44}$ | 83 kB | 82 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 2 | 77.9 | $\color{green}{23.4}$ | $\color{green}{0.45}$ | 66 kB | 65 kB | $\color{green}{0.97}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 2 | 98.0 | $\color{green}{32.0}$ | $\color{green}{0.49}$ | 89 kB | 88 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 2 | 120.7 | $\color{green}{42.4}$ | $\color{green}{0.49}$ | 125 kB | 122 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 2880×1800@2 | 2 | 142.3 | $\color{green}{45.8}$ | $\color{green}{0.48}$ | 125 kB | 122 kB | $\color{green}{0.97}$ | $\color{green}{+0.2}$ |
| 3200×2000@2 | 2 | 303.7 | $\color{green}{93.0}$ | $\color{green}{0.70}$ | 203 kB | 200 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 2 | 274.5 | $\color{green}{74.5}$ | $\color{green}{0.49}$ | 166 kB | 161 kB | $\color{green}{0.97}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 2 | 249.3 | $\color{green}{70.9}$ | $\color{green}{0.46}$ | 192 kB | 188 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |

### Keyframes inside `flood`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 31.2 | $\color{green}{15.9}$ | $\color{green}{0.56}$ | 52 kB | 51 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 1440×900 | 2 | 33.6 | $\color{green}{17.4}$ | $\color{green}{0.61}$ | 58 kB | 57 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 1600×1000 | 2 | 33.7 | $\color{green}{22.1}$ | $\color{green}{0.65}$ | 80 kB | 79 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 2 | 36.3 | $\color{green}{24.2}$ | $\color{green}{0.67}$ | 91 kB | 90 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 2 | 40.3 | $\color{green}{32.5}$ | $\color{green}{0.81}$ | 107 kB | 107 kB | 1.00 | $\color{green}{+0.2}$ |
| 2560×1600@2 | 2 | 65.3 | $\color{green}{36.2}$ | $\color{green}{0.55}$ | 118 kB | 115 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 2880×1800@2 | 2 | 103.7 | $\color{green}{43.3}$ | $\color{green}{0.76}$ | 131 kB | 128 kB | $\color{green}{0.98}$ | $\color{green}{+0.2}$ |
| 3200×2000@2 | 2 | 104.6 | $\color{green}{55.3}$ | $\color{green}{0.53}$ | 183 kB | 180 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 3456×2168@2 | 2 | 79.0 | $\color{green}{72.7}$ | $\color{green}{0.92}$ | 207 kB | 205 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 2 | 83.0 | $\color{green}{64.9}$ | $\color{green}{0.78}$ | 206 kB | 203 kB | $\color{green}{0.99}$ | $\color{green}{+0.2}$ |

### Keyframes inside `video`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 3 | 17.5 | $\color{green}{12.9}$ | $\color{green}{0.59}$ | 49 kB | 45 kB | $\color{green}{0.93}$ | $\color{green}{+0.3}$ |
| 1440×900 | 3 | 20.1 | $\color{green}{13.0}$ | $\color{green}{0.61}$ | 51 kB | 46 kB | $\color{green}{0.91}$ | $\color{green}{+0.3}$ |
| 1600×1000 | 3 | 17.3 | $\color{green}{13.5}$ | $\color{green}{0.62}$ | 51 kB | 47 kB | $\color{green}{0.91}$ | $\color{green}{+0.3}$ |
| 1920×1080 | 3 | 20.6 | $\color{green}{14.4}$ | $\color{green}{0.60}$ | 51 kB | 47 kB | $\color{green}{0.91}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 3 | 24.3 | $\color{red}{25.4}$ | $\color{green}{0.77}$ | 104 kB | 97 kB | $\color{green}{0.93}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 3 | 29.3 | $\color{green}{29.1}$ | $\color{green}{0.77}$ | 116 kB | 105 kB | $\color{green}{0.90}$ | $\color{green}{+0.3}$ |
| 2880×1800@2 | 3 | 38.0 | $\color{green}{31.6}$ | $\color{green}{0.69}$ | 120 kB | 109 kB | $\color{green}{0.91}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 3 | 39.7 | $\color{green}{33.9}$ | $\color{green}{0.72}$ | 126 kB | 114 kB | $\color{green}{0.91}$ | $\color{green}{+0.3}$ |
| 3456×2168@2 | 3 | 47.5 | $\color{red}{48.0}$ | $\color{red}{1.01}$ | 132 kB | 117 kB | $\color{green}{0.89}$ | $\color{green}{+0.3}$ |
| 3840×2160@2 | 3 | 43.9 | $\color{green}{36.8}$ | $\color{green}{0.66}$ | 117 kB | 105 kB | $\color{green}{0.90}$ | $\color{green}{+0.2}$ |

## Against libvpx, keyframes of each capture's first picture

### A keyframe at dial 40: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.81}$ | $\color{green}{0.92}$ | $\color{green}{0.87}$ | $\color{green}{0.69}$ | $\color{red}{1.03}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.99}$ |
| `flood` | $\color{green}{0.72}$ | $\color{green}{0.88}$ | $\color{green}{0.92}$ | $\color{green}{0.90}$ | 1.00 | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{red}{1.01}$ | $\color{green}{0.97}$ |
| `browser` | $\color{green}{0.55}$ | $\color{green}{0.64}$ | $\color{green}{0.65}$ | $\color{green}{0.64}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.68}$ |
| `drag` | $\color{green}{0.75}$ | $\color{green}{0.86}$ | $\color{green}{0.91}$ | $\color{green}{0.88}$ | $\color{green}{0.88}$ | $\color{red}{1.03}$ | $\color{green}{0.85}$ | $\color{green}{0.87}$ | $\color{green}{0.97}$ | $\color{red}{1.06}$ |
| `video` | $\color{green}{0.78}$ | $\color{green}{0.87}$ | $\color{green}{0.88}$ | $\color{green}{0.78}$ | $\color{red}{1.05}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.90}$ | $\color{green}{0.98}$ | $\color{green}{0.92}$ |
| `walk` | $\color{green}{0.71}$ | $\color{green}{0.71}$ | $\color{green}{0.78}$ | $\color{green}{0.71}$ | $\color{green}{0.92}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | $\color{green}{0.88}$ | $\color{green}{0.94}$ |

### A keyframe at dial 40: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | 1.00 |
| `browser` | $\color{green}{0.32}$ | $\color{green}{0.38}$ | $\color{green}{0.25}$ | $\color{green}{0.23}$ | $\color{green}{0.26}$ | $\color{green}{0.25}$ | $\color{green}{0.29}$ | $\color{green}{0.27}$ | $\color{green}{0.22}$ | $\color{green}{0.24}$ |
| `drag` | 1.00 | 1.00 | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ |
| `video` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 |
| `walk` | $\color{green}{0.73}$ | $\color{green}{0.68}$ | $\color{green}{0.68}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ | $\color{green}{0.67}$ | $\color{green}{0.67}$ |

### A keyframe at dial 40: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `browser` | $\color{green}{+30.0}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ | $\color{green}{+29.9}$ |
| `drag` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.7}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ |
| `walk` | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.1}$ | $\color{green}{+0.5}$ | $\color{red}{-0.1}$ | $\color{green}{+0.9}$ |

### A keyframe at dial 65: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.71}$ | $\color{green}{0.76}$ | $\color{green}{0.78}$ | $\color{green}{0.69}$ | $\color{red}{1.03}$ | $\color{green}{0.93}$ | $\color{green}{0.90}$ | $\color{green}{0.97}$ | $\color{green}{0.90}$ | $\color{green}{0.89}$ |
| `flood` | $\color{green}{0.67}$ | $\color{green}{0.82}$ | $\color{green}{0.85}$ | $\color{green}{0.84}$ | $\color{green}{0.96}$ | $\color{green}{0.98}$ | $\color{green}{0.91}$ | $\color{green}{0.93}$ | 1.00 | 1.00 |
| `browser` | $\color{green}{0.56}$ | $\color{green}{0.61}$ | $\color{green}{0.65}$ | $\color{green}{0.65}$ | $\color{green}{0.75}$ | $\color{green}{0.70}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.73}$ | $\color{green}{0.71}$ |
| `drag` | $\color{green}{0.71}$ | $\color{green}{0.77}$ | $\color{green}{0.84}$ | $\color{green}{0.81}$ | $\color{green}{0.90}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.88}$ | $\color{green}{0.99}$ |
| `video` | $\color{green}{0.72}$ | $\color{green}{0.75}$ | $\color{green}{0.78}$ | $\color{green}{0.70}$ | $\color{red}{1.01}$ | $\color{green}{0.94}$ | $\color{green}{0.90}$ | $\color{green}{0.88}$ | $\color{green}{0.89}$ | $\color{green}{0.87}$ |
| `walk` | $\color{green}{0.71}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ | $\color{green}{0.79}$ | $\color{green}{0.94}$ | $\color{green}{0.98}$ | $\color{green}{0.92}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.91}$ |

### A keyframe at dial 65: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ |
| `flood` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | 1.00 |
| `browser` | $\color{green}{0.31}$ | $\color{green}{0.36}$ | $\color{green}{0.25}$ | $\color{green}{0.23}$ | $\color{green}{0.25}$ | $\color{green}{0.24}$ | $\color{green}{0.29}$ | $\color{green}{0.26}$ | $\color{green}{0.22}$ | $\color{green}{0.23}$ |
| `drag` | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `video` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 |
| `walk` | $\color{green}{0.83}$ | $\color{green}{0.79}$ | $\color{green}{0.78}$ | $\color{green}{0.79}$ | $\color{green}{0.77}$ | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.79}$ | $\color{green}{0.74}$ | $\color{green}{0.75}$ |

### A keyframe at dial 65: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `flood` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.7}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ | $\color{green}{+0.5}$ |

### A keyframe at dial 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.59}$ | $\color{green}{0.71}$ | $\color{green}{0.69}$ | $\color{green}{0.63}$ | $\color{green}{0.93}$ | $\color{green}{0.91}$ | $\color{green}{0.84}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ | $\color{green}{0.82}$ |
| `flood` | $\color{green}{0.60}$ | $\color{green}{0.80}$ | $\color{green}{0.81}$ | $\color{green}{0.77}$ | $\color{green}{0.93}$ | $\color{green}{0.88}$ | $\color{green}{0.93}$ | $\color{green}{0.88}$ | $\color{green}{0.94}$ | $\color{green}{0.93}$ |
| `browser` | $\color{green}{0.53}$ | $\color{green}{0.59}$ | $\color{green}{0.62}$ | $\color{green}{0.66}$ | $\color{green}{0.75}$ | $\color{green}{0.69}$ | $\color{green}{0.74}$ | $\color{green}{0.70}$ | $\color{green}{0.67}$ | $\color{green}{0.73}$ |
| `drag` | $\color{green}{0.65}$ | $\color{green}{0.73}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.84}$ | $\color{green}{0.89}$ | $\color{green}{0.83}$ | $\color{green}{0.90}$ | $\color{green}{0.87}$ | $\color{green}{0.92}$ |
| `video` | $\color{green}{0.62}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.63}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ | $\color{green}{0.81}$ | $\color{green}{0.78}$ | $\color{green}{0.80}$ | $\color{green}{0.75}$ |
| `walk` | $\color{green}{0.73}$ | $\color{green}{0.76}$ | $\color{green}{0.79}$ | $\color{green}{0.78}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.97}$ | $\color{red}{1.01}$ | $\color{green}{0.90}$ |

### A keyframe at dial 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `browser` | $\color{green}{0.31}$ | $\color{green}{0.36}$ | $\color{green}{0.24}$ | $\color{green}{0.23}$ | $\color{green}{0.24}$ | $\color{green}{0.24}$ | $\color{green}{0.29}$ | $\color{green}{0.26}$ | $\color{green}{0.22}$ | $\color{green}{0.23}$ |
| `drag` | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ |
| `video` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{green}{0.99}$ | 1.00 |
| `walk` | $\color{green}{0.84}$ | $\color{green}{0.81}$ | $\color{green}{0.80}$ | $\color{green}{0.81}$ | $\color{green}{0.80}$ | $\color{green}{0.77}$ | $\color{green}{0.78}$ | $\color{green}{0.79}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ |

### A keyframe at dial 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ |

## Against libvpx, settles of each capture's first picture

### A settle from dial 20 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.61}$ | $\color{green}{0.77}$ | $\color{green}{0.69}$ | $\color{green}{0.68}$ | $\color{red}{1.04}$ | $\color{green}{0.96}$ | $\color{green}{0.92}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.97}$ |
| `flood` | $\color{green}{0.65}$ | $\color{green}{0.84}$ | $\color{green}{0.88}$ | $\color{green}{0.87}$ | $\color{red}{1.03}$ | $\color{red}{1.05}$ | $\color{red}{1.09}$ | $\color{red}{1.08}$ | $\color{red}{1.09}$ | $\color{red}{1.10}$ |
| `browser` | $\color{green}{0.59}$ | $\color{green}{0.73}$ | $\color{green}{0.72}$ | $\color{green}{0.77}$ | $\color{green}{0.97}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.94}$ |
| `drag` | $\color{green}{0.71}$ | $\color{green}{0.85}$ | $\color{green}{0.88}$ | $\color{green}{0.87}$ | $\color{red}{1.05}$ | $\color{red}{1.07}$ | $\color{red}{1.07}$ | 1.00 | $\color{red}{1.02}$ | $\color{red}{1.05}$ |
| `video` | $\color{green}{0.68}$ | $\color{green}{0.74}$ | $\color{green}{0.78}$ | $\color{green}{0.71}$ | $\color{red}{1.01}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.91}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ |
| `walk` | $\color{green}{0.75}$ | $\color{green}{0.78}$ | $\color{green}{0.85}$ | $\color{green}{0.96}$ | $\color{red}{1.01}$ | $\color{red}{1.10}$ | $\color{red}{1.02}$ | $\color{red}{1.05}$ | $\color{red}{1.09}$ | $\color{red}{1.01}$ |

### A settle from dial 20 to 90: bytes

This encoder's bytes over libvpx's. `browser`'s row is of 2026-10-11, with
the flat squares of a settle predicted from each other; the others are the
run's, and at 1920×1080 are a hundredth or two under it since.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | 1.00 | $\color{red}{1.01}$ | 1.00 |
| `browser` | $\color{green}{0.54}$ | $\color{green}{0.71}$ | $\color{green}{0.47}$ | $\color{green}{0.53}$ | $\color{green}{0.55}$ | $\color{green}{0.62}$ | $\color{green}{0.92}$ | $\color{green}{0.84}$ | $\color{green}{0.76}$ | $\color{green}{0.78}$ |
| `drag` | $\color{green}{0.97}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ |
| `video` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.04}$ |
| `walk` | $\color{green}{0.89}$ | $\color{green}{0.86}$ | $\color{green}{0.87}$ | $\color{green}{0.86}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.89}$ |

### A settle from dial 20 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `flood` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.1}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `walk` | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.8}$ | $\color{green}{+0.7}$ |

### A settle from dial 65 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.24}$ | $\color{red}{1.31}$ | $\color{red}{1.24}$ | $\color{red}{1.28}$ | $\color{red}{1.77}$ | $\color{red}{1.62}$ | $\color{red}{1.60}$ | $\color{red}{1.71}$ | $\color{red}{1.66}$ | $\color{red}{1.60}$ |
| `flood` | $\color{red}{1.12}$ | $\color{red}{1.30}$ | $\color{red}{1.41}$ | $\color{red}{1.40}$ | $\color{red}{1.65}$ | $\color{red}{1.55}$ | $\color{red}{1.47}$ | $\color{red}{1.58}$ | $\color{red}{1.58}$ | $\color{red}{1.56}$ |
| `browser` | $\color{green}{0.36}$ | $\color{green}{0.43}$ | $\color{green}{0.42}$ | $\color{green}{0.44}$ | $\color{green}{0.53}$ | $\color{green}{0.54}$ | $\color{green}{0.56}$ | $\color{green}{0.51}$ | $\color{green}{0.52}$ | $\color{green}{0.52}$ |
| `drag` | $\color{red}{1.22}$ | $\color{red}{1.35}$ | $\color{red}{1.44}$ | $\color{red}{1.23}$ | $\color{red}{1.56}$ | $\color{red}{1.67}$ | $\color{red}{1.47}$ | $\color{red}{1.58}$ | $\color{red}{1.45}$ | $\color{red}{1.50}$ |
| `video` | $\color{red}{1.09}$ | $\color{red}{1.08}$ | $\color{red}{1.05}$ | 1.00 | $\color{red}{1.49}$ | $\color{red}{1.33}$ | $\color{red}{1.21}$ | $\color{red}{1.15}$ | $\color{red}{1.04}$ | $\color{green}{0.93}$ |
| `walk` | $\color{red}{1.12}$ | $\color{red}{1.05}$ | $\color{red}{1.13}$ | $\color{red}{1.08}$ | $\color{red}{1.41}$ | $\color{red}{1.48}$ | $\color{red}{1.38}$ | $\color{red}{1.38}$ | $\color{red}{1.41}$ | $\color{red}{1.15}$ |

### A settle from dial 65 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.97}$ | 1.00 | $\color{green}{0.99}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ |
| `browser` | $\color{red}{1.22}$ | $\color{red}{1.37}$ | $\color{red}{1.04}$ | $\color{red}{1.15}$ | $\color{red}{1.12}$ | $\color{red}{1.22}$ | $\color{red}{1.61}$ | $\color{red}{1.45}$ | $\color{red}{1.40}$ | $\color{red}{1.42}$ |
| `drag` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ |
| `video` | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | 1.00 | $\color{green}{0.93}$ | $\color{green}{0.93}$ |
| `walk` | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ |

### A settle from dial 65 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ | $\color{green}{+1.2}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `flood` | $\color{green}{+1.2}$ | $\color{green}{+1.1}$ | $\color{green}{+1.2}$ | $\color{green}{+1.0}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `browser` | 0.0 | $\color{green}{+1.1}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.7}$ | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.1}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ |
| `video` | $\color{green}{+1.1}$ | $\color{green}{+0.8}$ | $\color{green}{+1.0}$ | $\color{green}{+0.8}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.5}$ | $\color{green}{+1.3}$ |
| `walk` | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.9}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |

### A settle from dial 80 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.19}$ | $\color{red}{1.23}$ | $\color{red}{1.15}$ | $\color{red}{1.16}$ | $\color{red}{1.70}$ | $\color{red}{1.59}$ | $\color{red}{1.48}$ | $\color{red}{1.43}$ | $\color{red}{1.51}$ | $\color{red}{1.46}$ |
| `flood` | $\color{red}{1.13}$ | $\color{red}{1.12}$ | $\color{red}{1.27}$ | $\color{red}{1.24}$ | $\color{red}{1.34}$ | $\color{red}{1.33}$ | $\color{red}{1.27}$ | $\color{red}{1.32}$ | $\color{red}{1.33}$ | $\color{red}{1.29}$ |
| `browser` | $\color{green}{0.35}$ | $\color{green}{0.34}$ | $\color{green}{0.38}$ | $\color{green}{0.40}$ | $\color{green}{0.45}$ | $\color{green}{0.48}$ | $\color{green}{0.58}$ | $\color{green}{0.48}$ | $\color{green}{0.52}$ | $\color{green}{0.50}$ |
| `drag` | $\color{red}{1.17}$ | $\color{red}{1.16}$ | $\color{red}{1.20}$ | $\color{red}{1.15}$ | $\color{red}{1.34}$ | $\color{red}{1.35}$ | $\color{red}{1.26}$ | $\color{red}{1.37}$ | $\color{red}{1.18}$ | $\color{red}{1.25}$ |
| `video` | $\color{red}{1.04}$ | $\color{red}{1.06}$ | $\color{red}{1.01}$ | $\color{green}{0.82}$ | $\color{red}{1.20}$ | $\color{red}{1.22}$ | $\color{red}{1.03}$ | $\color{green}{0.96}$ | $\color{green}{0.88}$ | $\color{green}{0.85}$ |
| `walk` | $\color{red}{1.10}$ | $\color{red}{1.06}$ | $\color{red}{1.05}$ | $\color{red}{1.09}$ | $\color{red}{1.37}$ | $\color{red}{1.42}$ | $\color{red}{1.38}$ | $\color{red}{1.42}$ | $\color{red}{1.37}$ | $\color{red}{1.17}$ |

### A settle from dial 80 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.79}$ | $\color{green}{0.84}$ | $\color{green}{0.81}$ | $\color{green}{0.76}$ | $\color{green}{0.75}$ | $\color{green}{0.73}$ | $\color{green}{0.69}$ | $\color{green}{0.71}$ | $\color{green}{0.73}$ | $\color{green}{0.72}$ |
| `flood` | $\color{green}{0.73}$ | $\color{green}{0.70}$ | $\color{green}{0.68}$ | $\color{green}{0.69}$ | $\color{green}{0.71}$ | $\color{green}{0.79}$ | $\color{green}{0.79}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{green}{0.75}$ |
| `browser` | $\color{red}{1.14}$ | $\color{red}{1.49}$ | $\color{red}{1.08}$ | $\color{red}{1.29}$ | $\color{red}{1.30}$ | $\color{red}{1.47}$ | $\color{red}{2.05}$ | $\color{red}{1.81}$ | $\color{red}{1.73}$ | $\color{red}{1.76}$ |
| `drag` | $\color{green}{0.71}$ | $\color{green}{0.73}$ | $\color{green}{0.69}$ | $\color{green}{0.76}$ | $\color{green}{0.78}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ | $\color{green}{0.73}$ | $\color{green}{0.78}$ |
| `video` | $\color{green}{0.83}$ | $\color{green}{0.88}$ | $\color{green}{0.92}$ | $\color{green}{0.64}$ | $\color{green}{0.70}$ | $\color{green}{0.79}$ | $\color{green}{0.74}$ | $\color{green}{0.81}$ | $\color{green}{0.72}$ | $\color{green}{0.74}$ |
| `walk` | 1.00 | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{red}{1.01}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.89}$ | $\color{green}{0.97}$ | $\color{green}{0.92}$ | $\color{green}{0.94}$ |

### A settle from dial 80 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `flood` | 0.0 | 0.0 | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | 0.0 | $\color{green}{+0.1}$ | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ |
| `video` | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{red}{-0.1}$ | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `walk` | $\color{green}{+0.9}$ | $\color{green}{+0.9}$ | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |

### A settle from dial 89 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{red}{1.13}$ | $\color{red}{1.11}$ | $\color{red}{1.16}$ | $\color{red}{1.15}$ | $\color{red}{1.30}$ | $\color{red}{1.26}$ | $\color{red}{1.21}$ | $\color{red}{1.17}$ | $\color{red}{1.18}$ | $\color{red}{1.12}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | 1.00 | $\color{red}{1.08}$ | $\color{red}{1.10}$ | $\color{red}{1.14}$ | $\color{red}{1.04}$ | $\color{red}{1.05}$ | $\color{red}{1.08}$ | $\color{red}{1.02}$ |
| `browser` | $\color{green}{0.35}$ | $\color{green}{0.33}$ | $\color{green}{0.34}$ | $\color{green}{0.39}$ | $\color{green}{0.45}$ | $\color{green}{0.50}$ | $\color{green}{0.48}$ | $\color{green}{0.48}$ | $\color{green}{0.51}$ | $\color{green}{0.48}$ |
| `drag` | $\color{red}{1.06}$ | $\color{red}{1.07}$ | $\color{red}{1.15}$ | $\color{green}{0.93}$ | $\color{red}{1.03}$ | $\color{red}{1.07}$ | $\color{red}{1.07}$ | $\color{red}{1.08}$ | $\color{red}{1.02}$ | 1.00 |
| `video` | $\color{green}{0.77}$ | $\color{green}{0.78}$ | $\color{green}{0.80}$ | $\color{green}{0.68}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.84}$ | $\color{green}{0.82}$ | $\color{green}{0.76}$ | $\color{green}{0.73}$ |
| `walk` | $\color{red}{1.03}$ | $\color{red}{1.07}$ | $\color{green}{0.98}$ | $\color{green}{0.93}$ | $\color{red}{1.29}$ | $\color{red}{1.34}$ | $\color{red}{1.30}$ | $\color{red}{1.35}$ | $\color{red}{1.37}$ | $\color{red}{1.06}$ |

### A settle from dial 89 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{0.63}$ | $\color{green}{0.70}$ | $\color{green}{0.63}$ | $\color{green}{0.72}$ | $\color{green}{0.48}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ | $\color{green}{0.55}$ | $\color{green}{0.44}$ | $\color{green}{0.44}$ |
| `flood` | $\color{green}{0.85}$ | $\color{green}{0.79}$ | $\color{green}{0.70}$ | $\color{green}{0.79}$ | $\color{green}{0.74}$ | $\color{green}{0.92}$ | 1.00 | $\color{green}{0.74}$ | $\color{green}{0.79}$ | $\color{green}{0.76}$ |
| `browser` | $\color{red}{1.14}$ | $\color{red}{1.49}$ | $\color{red}{1.08}$ | $\color{red}{1.29}$ | $\color{red}{1.30}$ | $\color{red}{1.47}$ | $\color{red}{2.05}$ | $\color{red}{1.81}$ | $\color{red}{1.73}$ | $\color{red}{1.76}$ |
| `drag` | $\color{green}{0.93}$ | $\color{green}{0.86}$ | $\color{green}{0.80}$ | $\color{green}{0.95}$ | $\color{green}{0.89}$ | $\color{green}{0.69}$ | $\color{red}{1.03}$ | $\color{green}{0.82}$ | $\color{green}{0.79}$ | $\color{green}{0.72}$ |
| `video` | $\color{green}{0.48}$ | $\color{green}{0.55}$ | $\color{green}{0.57}$ | $\color{green}{0.62}$ | $\color{green}{0.47}$ | $\color{green}{0.56}$ | $\color{green}{0.59}$ | $\color{green}{0.67}$ | $\color{green}{0.44}$ | $\color{green}{0.56}$ |
| `walk` | $\color{red}{1.03}$ | $\color{red}{1.04}$ | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.93}$ | $\color{green}{0.89}$ | $\color{green}{0.91}$ | $\color{green}{0.94}$ | $\color{red}{1.01}$ | $\color{green}{0.86}$ |

### A settle from dial 89 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `drag` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `walk` | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.5}$ |

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
