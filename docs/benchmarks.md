# What the encoder was measured at

The numbers the encoder's decisions were taken on. How the encoder works is
in [encoder.md](encoder.md); what was built in which order, and why, in
[plan.md](plan.md).

The releases named here, 0.0.1-alpha.2 to alpha.5, are those of
`screen-vp9-native`, the crate this encoder was until it came home to
`screen-vp9` as `Coder::Native`. The tables are against libvpx as
`screen-vp9` 0.0.19 sets it up, the crate's other coder (`src/libvpx.rs`),
by the three measurements in `src/native/mod.rs`:
`captures_against_libvpx` plays each capture into both encoders, frame by
frame, turn about; `keyframes_against_libvpx` codes each capture's first
picture as a keyframe at dials 40, 65 and 90, and `video`'s and `walk`'s
last of more than one colour, since both begin on the clip paused on
black, which for `walk` is the frame its session pauses the clip on and
settles; `settles_against_libvpx`
codes it at one dial and settles it at 90, the picture unchanged. The last
two keep the least time of five passes, each on one encoder kept across
them, as a session's is. `tools/tables.py` makes the tables below from
their logs.

They were run on 2026-10-15 on a six-core Intel i5-8500T (x86-64-v3) under
WSL2, four threads, the machine idle and one measurement at a time, on the
build after release 0.0.19 that updates an inter frame's mode, partition
and vector probabilities, takes the 32×32 intra transform, codes an intra
square of an inter frame inter at its vector too and takes the cheaper,
and cuts a smaller transform size's trial short
([plan.md](plan.md#taken-up-on-2026-10-15)), over the captures of
2026-10-13 cut to 120 frames each, `walk` whole and captured again that
day with its clip paused for the settle ([Test
material](#test-material)): the three took 47 minutes. A time on this
machine moves by a tenth from one run to the next, so a ratio within a
tenth of 1.00 is on the line. The same
measurements on an Apple M2 Max, in
[benchmarks-m2-max.md](benchmarks-m2-max.md), are of the set before and an
earlier encoder, and are not run again here: the two are not to be read
against each other.

Throughout, a *ratio* is this encoder over libvpx, so under 1.00 is fewer
bytes or less time; a *PSNR difference* is this encoder's luma PSNR less
libvpx's at the same dial, so above zero is a better picture. `@2` marks a
desktop drawn at scale 2. Green is this encoder's side of the line and red
the other; a number on the line is left plain.

## What the tables say

- **A video is on the line in bytes from 2880 wide and a little over it
  under, and over libvpx's picture.** `video` takes 1.00 to 1.05 of
  libvpx's bytes over its frames at +0.6 to +0.9 dB, 1.00 from 2880
  wide; `walk`, the same video as the dial walks down to 20 or 30 and
  partway back up, then paused and settled, 1.01 to 1.08 at +0.4 to
  +0.7, the frame its session settles, from 41 to 75, at 0.84 to 0.92 of
  libvpx's bytes at +0.3 to +0.9 dB, in 0.86 to 1.21 of its time up to
  1920 wide and 1.16 to 1.57 from 2048, one frame's time. At 0.0.19
  they were 1.03 to 1.08 and, on the walk before, 1.08 to 1.13, the
  frames' modes, partitions and vectors coded at VP9's default
  probabilities, and without vectors between pixels, at 0.0.18, 1.22 to
  1.27 and 1.16 to 1.25 at up to 0.6 dB under: where a video's bytes go
  is measured in [plan.md](plan.md#where-a-videos-bytes-go-on-2026-10-14).
- **Everything else over a whole capture is libvpx's picture or better, at
  fewer bytes, but `drag` from 2048 to 3456 wide**, 0.2 to 0.4 dB under at
  0.40 to 0.49 of the bytes, which the capture whole is too, at a quarter
  to a tenth of them ([plan.md](plan.md#measured-for-what-comes-next)):
  `browser` +0.8 to +1.6 dB at 0.18–0.30×, `uncover` +0.8 to +1.1 at
  0.42–0.86×, `flood` +0.4 to +0.8 at 0.76–0.95×, `still` +0.3 to +0.4 at
  0.98–1.00×, `terminal` +0.1 to +0.4 at 0.77–0.93×, `drag` elsewhere
  +0.2 to +0.4 at 0.22–0.25×. That is 75 cells of 80 at libvpx's picture
  or over.
- **Every capture at every size takes less time than libvpx but `video`
  at 2048×1536, on the line at 1.01**, frame for frame and summed: a line
  of text 0.14–0.22, `browser` 0.38–0.45, `flood` 0.39–0.56, `drag`
  0.46–0.57, `uncover` 0.51–0.68, `walk` 0.71–0.81, `video` 0.85–1.01,
  `still`'s two keyframes 0.51–0.89.
- **A keyframe inside a capture is 0.1 to 0.6 dB over libvpx's** at
  0.97–1.01× the bytes (`terminal`'s 0.77–0.95×), in 0.38 to 0.80 of its
  time but `flood`'s from 2048 wide, 0.93 to 1.02.
- **A first picture coded alone as a keyframe is over libvpx's picture at
  every dial and size**: the screens 0.2 to 0.8 dB at dial 40 at
  0.98–1.03× the bytes, `terminal` 0.5 to 5.9 dB at 0.65–0.88×; at 65 and
  90 every picture 0.1 to 0.7 dB over at 0.97–1.03×, `terminal`'s at
  0.73–0.95×. `terminal`'s at 40 was 2.4 to 7.7 dB under, its flat ground
  a grey level off ([plan.md](plan.md#taken-up-on-2026-10-15)). **A
  picture of the video** is 0.1 to 0.2 dB over at 40 at 0.86–1.01× the
  bytes, and 0.3 to 0.5 at 65 and 90 at 0.97–1.06×. Up to 1920 wide a
  keyframe takes 0.38 to 0.98 of libvpx's time; from 2048 wide 0.49 to
  1.04, `flood`'s at 0.98 to 1.04 and the video's at 90 up to 1.02.
- **A settle of a busy picture takes more time than libvpx's** from 65:
  `still`, `uncover` and `browser` 0.98 to 1.46 of it, for 1.0 to 1.5 dB
  more picture at 0.90–0.97× the bytes; `flood` 0.77 to 1.72; a picture
  of the video 1.04 to 1.61, at +0.1 to +0.4 dB and 0.93–0.98× the bytes;
  `drag` 0.83 to 1.26. From 80 `flood` is 0.99 to 1.96 of it, at 1.01 to
  1.10 of the bytes, the video 0.93 to 1.52 at 0.79–0.87× the bytes, and
  `still`, `uncover` and `browser` 0.81 to 1.12, on the line; from 89
  `flood` is 1.07 to 1.77, the video 0.87 to 1.44, and the others 0.62 to
  0.92. From 20 a settle is 0.30 to 1.02 of libvpx's time but the video's
  from 2048 wide, 1.04 to 1.18, every settle from 20 at libvpx's picture
  or over at 0.77–1.04× its bytes. `terminal`'s settles are 0.30 to 0.68
  of it.
- **What is over the line in bytes, besides the video over its frames**:
  a settle from 89 of a picture of the video from 2048 wide (1.03 to
  1.32×, at +0.4 to +0.5 dB), of `terminal` up to 1920 wide (up to
  1.55×, a few hundred bytes) and of `uncover` at one size (1.05×); a
  settle of `flood` from 80 (1.01 to 1.10×) and from 65 from 2048 wide
  (1.01 to 1.02×); single cells of `terminal`'s settles from 20, 65 and
  80 (up to 1.13×); and a keyframe of the video at 65 and 90 (1.01 to
  1.06×).

## Found on the captures before

Measured on the set of 2026-10-09, before the one the tables are of. Its
`uncover` never showed its terminal (the page was drawn over it, so from
2.3 s no frame changed a byte) and its `video` was a test pattern in a
1280×720 window; the numbers below are of it, and of the encoder of the
time.

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
| `terminal` | $\color{green}{0.92}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ | $\color{green}{0.91}$ | $\color{green}{0.78}$ | $\color{green}{0.79}$ | $\color{green}{0.80}$ | $\color{green}{0.80}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ |
| `still` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `uncover` | $\color{green}{0.86}$ | $\color{green}{0.84}$ | $\color{green}{0.64}$ | $\color{green}{0.65}$ | $\color{green}{0.50}$ | $\color{green}{0.47}$ | $\color{green}{0.48}$ | $\color{green}{0.52}$ | $\color{green}{0.56}$ | $\color{green}{0.42}$ |
| `flood` | $\color{green}{0.89}$ | $\color{green}{0.76}$ | $\color{green}{0.84}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.93}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ |
| `browser` | $\color{green}{0.25}$ | $\color{green}{0.23}$ | $\color{green}{0.21}$ | $\color{green}{0.30}$ | $\color{green}{0.22}$ | $\color{green}{0.19}$ | $\color{green}{0.19}$ | $\color{green}{0.18}$ | $\color{green}{0.20}$ | $\color{green}{0.18}$ |
| `drag` | $\color{green}{0.23}$ | $\color{green}{0.23}$ | $\color{green}{0.24}$ | $\color{green}{0.22}$ | $\color{green}{0.40}$ | $\color{green}{0.47}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ | $\color{green}{0.44}$ | $\color{green}{0.25}$ |
| `video` | $\color{red}{1.05}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 | 1.00 |
| `walk` | $\color{red}{1.08}$ | $\color{red}{1.07}$ | $\color{red}{1.06}$ | $\color{red}{1.05}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ |

### Encode time, the median of the per-frame ratios

Under 1.00 is faster than libvpx, frame for frame.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.20}$ | $\color{green}{0.17}$ | $\color{green}{0.15}$ | $\color{green}{0.14}$ | $\color{green}{0.22}$ | $\color{green}{0.18}$ | $\color{green}{0.18}$ | $\color{green}{0.16}$ | $\color{green}{0.16}$ | $\color{green}{0.16}$ |
| `still` | $\color{green}{0.51}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.68}$ | $\color{green}{0.75}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.89}$ | $\color{green}{0.70}$ |
| `uncover` | $\color{green}{0.68}$ | $\color{green}{0.68}$ | $\color{green}{0.65}$ | $\color{green}{0.64}$ | $\color{green}{0.58}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.53}$ | $\color{green}{0.53}$ | $\color{green}{0.51}$ |
| `flood` | $\color{green}{0.39}$ | $\color{green}{0.44}$ | $\color{green}{0.48}$ | $\color{green}{0.50}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.53}$ |
| `browser` | $\color{green}{0.39}$ | $\color{green}{0.40}$ | $\color{green}{0.38}$ | $\color{green}{0.40}$ | $\color{green}{0.45}$ | $\color{green}{0.45}$ | $\color{green}{0.43}$ | $\color{green}{0.41}$ | $\color{green}{0.39}$ | $\color{green}{0.42}$ |
| `drag` | $\color{green}{0.46}$ | $\color{green}{0.49}$ | $\color{green}{0.48}$ | $\color{green}{0.49}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.52}$ | $\color{green}{0.52}$ | $\color{green}{0.57}$ |
| `video` | $\color{green}{0.85}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.92}$ | $\color{red}{1.01}$ | $\color{green}{0.94}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.92}$ | $\color{green}{0.88}$ |
| `walk` | $\color{green}{0.79}$ | $\color{green}{0.78}$ | $\color{green}{0.78}$ | $\color{green}{0.74}$ | $\color{green}{0.81}$ | $\color{green}{0.76}$ | $\color{green}{0.72}$ | $\color{green}{0.73}$ | $\color{green}{0.76}$ | $\color{green}{0.71}$ |

### Encode time, the whole capture's

Every frame's time summed, this encoder's over libvpx's: where the long frames weigh.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.22}$ | $\color{green}{0.19}$ | $\color{green}{0.18}$ | $\color{green}{0.17}$ | $\color{green}{0.26}$ | $\color{green}{0.22}$ | $\color{green}{0.21}$ | $\color{green}{0.19}$ | $\color{green}{0.18}$ | $\color{green}{0.18}$ |
| `still` | $\color{green}{0.48}$ | $\color{green}{0.57}$ | $\color{green}{0.59}$ | $\color{green}{0.55}$ | $\color{green}{0.70}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.65}$ | $\color{green}{0.77}$ | $\color{green}{0.68}$ |
| `uncover` | $\color{green}{0.62}$ | $\color{green}{0.63}$ | $\color{green}{0.61}$ | $\color{green}{0.59}$ | $\color{green}{0.56}$ | $\color{green}{0.54}$ | $\color{green}{0.53}$ | $\color{green}{0.51}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ |
| `flood` | $\color{green}{0.39}$ | $\color{green}{0.45}$ | $\color{green}{0.47}$ | $\color{green}{0.51}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ |
| `browser` | $\color{green}{0.39}$ | $\color{green}{0.40}$ | $\color{green}{0.39}$ | $\color{green}{0.40}$ | $\color{green}{0.44}$ | $\color{green}{0.43}$ | $\color{green}{0.42}$ | $\color{green}{0.39}$ | $\color{green}{0.38}$ | $\color{green}{0.40}$ |
| `drag` | $\color{green}{0.46}$ | $\color{green}{0.48}$ | $\color{green}{0.48}$ | $\color{green}{0.48}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.51}$ | $\color{green}{0.50}$ | $\color{green}{0.53}$ |
| `video` | $\color{green}{0.86}$ | $\color{green}{0.94}$ | $\color{green}{0.96}$ | $\color{green}{0.93}$ | $\color{red}{1.01}$ | $\color{green}{0.95}$ | $\color{green}{0.93}$ | $\color{green}{0.92}$ | $\color{green}{0.92}$ | $\color{green}{0.89}$ |
| `walk` | $\color{green}{0.80}$ | $\color{green}{0.81}$ | $\color{green}{0.81}$ | $\color{green}{0.77}$ | $\color{green}{0.84}$ | $\color{green}{0.79}$ | $\color{green}{0.76}$ | $\color{green}{0.76}$ | $\color{green}{0.77}$ | $\color{green}{0.73}$ |

### Luma PSNR, this encoder less libvpx, in dB

Above zero is a better picture than libvpx at the same dial.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `still` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.9}$ | $\color{green}{+0.9}$ | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+1.1}$ | $\color{green}{+1.0}$ | $\color{green}{+1.0}$ | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ |
| `flood` | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ |
| `browser` | $\color{green}{+0.8}$ | $\color{green}{+1.0}$ | $\color{green}{+1.2}$ | $\color{green}{+1.0}$ | $\color{green}{+1.5}$ | $\color{green}{+1.6}$ | $\color{green}{+1.6}$ | $\color{green}{+1.6}$ | $\color{green}{+1.5}$ | $\color{green}{+1.6}$ |
| `drag` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{red}{-0.4}$ | $\color{red}{-0.4}$ | $\color{red}{-0.3}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.9}$ |
| `walk` | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ |

## Against libvpx, the frames apart

The time columns are each encoder's median time for a frame of that kind, the
time ratio the median of the per-frame ratios, and the bytes the mean per frame.

### `terminal`, a line of text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 117 | 2.15 | $\color{green}{0.41}$ | $\color{green}{0.19}$ | 116 B | 94 B | $\color{green}{0.81}$ | $\color{green}{+0.2}$ |
| 1440×900 | 117 | 3.08 | $\color{green}{0.51}$ | $\color{green}{0.17}$ | 118 B | 102 B | $\color{green}{0.87}$ | $\color{green}{+0.1}$ |
| 1600×1000 | 117 | 3.45 | $\color{green}{0.52}$ | $\color{green}{0.15}$ | 124 B | 113 B | $\color{green}{0.91}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 117 | 4.16 | $\color{green}{0.58}$ | $\color{green}{0.14}$ | 145 B | 120 B | $\color{green}{0.83}$ | $\color{green}{+0.1}$ |
| 2048×1536@2 | 117 | 4.17 | $\color{green}{0.93}$ | $\color{green}{0.22}$ | 224 B | 171 B | $\color{green}{0.77}$ | $\color{green}{+0.3}$ |
| 2560×1600@2 | 117 | 5.19 | $\color{green}{0.95}$ | $\color{green}{0.18}$ | 185 B | 142 B | $\color{green}{0.77}$ | $\color{green}{+0.4}$ |
| 2880×1800@2 | 117 | 6.69 | $\color{green}{1.17}$ | $\color{green}{0.17}$ | 191 B | 149 B | $\color{green}{0.78}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 117 | 8.11 | $\color{green}{1.33}$ | $\color{green}{0.16}$ | 192 B | 154 B | $\color{green}{0.80}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 117 | 9.06 | $\color{green}{1.41}$ | $\color{green}{0.16}$ | 217 B | 157 B | $\color{green}{0.72}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 117 | 10.1 | $\color{green}{1.56}$ | $\color{green}{0.15}$ | 244 B | 176 B | $\color{green}{0.72}$ | $\color{green}{+0.4}$ |

### `terminal`, a scroll

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 10.6 | $\color{green}{7.43}$ | $\color{green}{0.70}$ | 51 kB | 49 kB | $\color{green}{0.95}$ | 0.0 |
| 1440×900 | 1 | 12.8 | $\color{green}{7.85}$ | $\color{green}{0.61}$ | 50 kB | 48 kB | $\color{green}{0.95}$ | 0.0 |
| 1600×1000 | 1 | 13.6 | $\color{green}{8.19}$ | $\color{green}{0.60}$ | 50 kB | 48 kB | $\color{green}{0.95}$ | 0.0 |
| 1920×1080 | 1 | 13.3 | $\color{green}{8.63}$ | $\color{green}{0.65}$ | 51 kB | 48 kB | $\color{green}{0.94}$ | $\color{green}{+0.1}$ |
| 2048×1536@2 | 1 | 22.6 | $\color{green}{15.3}$ | $\color{green}{0.67}$ | 135 kB | 104 kB | $\color{green}{0.77}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 1 | 26.2 | $\color{green}{16.3}$ | $\color{green}{0.62}$ | 134 kB | 104 kB | $\color{green}{0.78}$ | $\color{green}{+0.3}$ |
| 2880×1800@2 | 1 | 28.3 | $\color{green}{17.7}$ | $\color{green}{0.63}$ | 133 kB | 106 kB | $\color{green}{0.80}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 1 | 35.0 | $\color{green}{19.9}$ | $\color{green}{0.57}$ | 121 kB | 96 kB | $\color{green}{0.80}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 1 | 34.6 | $\color{green}{20.1}$ | $\color{green}{0.58}$ | 130 kB | 101 kB | $\color{green}{0.78}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 1 | 37.6 | $\color{green}{20.9}$ | $\color{green}{0.56}$ | 133 kB | 104 kB | $\color{green}{0.78}$ | $\color{green}{+0.3}$ |

### `drag`, a quarter of the picture

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 118 | 10.6 | $\color{green}{4.91}$ | $\color{green}{0.46}$ | 5850 B | 210 B | $\color{green}{0.04}$ | $\color{green}{+0.3}$ |
| 1440×900 | 118 | 10.4 | $\color{green}{5.03}$ | $\color{green}{0.49}$ | 5841 B | 209 B | $\color{green}{0.04}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 118 | 10.8 | $\color{green}{5.22}$ | $\color{green}{0.48}$ | 5618 B | 207 B | $\color{green}{0.04}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 118 | 11.5 | $\color{green}{5.42}$ | $\color{green}{0.49}$ | 6086 B | 199 B | $\color{green}{0.03}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 118 | 21.5 | $\color{green}{11.7}$ | $\color{green}{0.54}$ | 7332 B | 833 B | $\color{green}{0.11}$ | $\color{red}{-0.4}$ |
| 2560×1600@2 | 118 | 22.6 | $\color{green}{11.9}$ | $\color{green}{0.53}$ | 5708 B | 820 B | $\color{green}{0.14}$ | $\color{red}{-0.4}$ |
| 2880×1800@2 | 118 | 24.0 | $\color{green}{13.0}$ | $\color{green}{0.54}$ | 5329 B | 812 B | $\color{green}{0.15}$ | $\color{red}{-0.3}$ |
| 3200×2000@2 | 118 | 25.9 | $\color{green}{13.3}$ | $\color{green}{0.52}$ | 5703 B | 822 B | $\color{green}{0.14}$ | $\color{red}{-0.2}$ |
| 3456×2168@2 | 118 | 26.6 | $\color{green}{13.5}$ | $\color{green}{0.52}$ | 6359 B | 831 B | $\color{green}{0.13}$ | $\color{red}{-0.2}$ |
| 3840×2160@2 | 118 | 24.1 | $\color{green}{13.7}$ | $\color{green}{0.57}$ | 14 kB | 833 B | $\color{green}{0.06}$ | $\color{green}{+0.3}$ |

### `browser`, a page scrolled

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 118 | 18.4 | $\color{green}{7.23}$ | $\color{green}{0.39}$ | 9303 B | 1067 B | $\color{green}{0.12}$ | $\color{green}{+0.8}$ |
| 1440×900 | 118 | 20.1 | $\color{green}{7.86}$ | $\color{green}{0.40}$ | 13 kB | 1614 B | $\color{green}{0.12}$ | $\color{green}{+1.0}$ |
| 1600×1000 | 119 | 24.3 | $\color{green}{9.33}$ | $\color{green}{0.38}$ | 15 kB | 1976 B | $\color{green}{0.13}$ | $\color{green}{+1.2}$ |
| 1920×1080 | 118 | 26.7 | $\color{green}{10.6}$ | $\color{green}{0.40}$ | 12 kB | 1666 B | $\color{green}{0.14}$ | $\color{green}{+1.0}$ |
| 2048×1536@2 | 118 | 35.0 | $\color{green}{15.4}$ | $\color{green}{0.45}$ | 40 kB | 5837 B | $\color{green}{0.14}$ | $\color{green}{+1.5}$ |
| 2560×1600@2 | 118 | 46.1 | $\color{green}{20.4}$ | $\color{green}{0.45}$ | 50 kB | 6575 B | $\color{green}{0.13}$ | $\color{green}{+1.7}$ |
| 2880×1800@2 | 118 | 54.5 | $\color{green}{23.2}$ | $\color{green}{0.43}$ | 52 kB | 6262 B | $\color{green}{0.12}$ | $\color{green}{+1.6}$ |
| 3200×2000@2 | 118 | 67.1 | $\color{green}{26.7}$ | $\color{green}{0.41}$ | 65 kB | 6556 B | $\color{green}{0.10}$ | $\color{green}{+1.6}$ |
| 3456×2168@2 | 118 | 75.6 | $\color{green}{29.5}$ | $\color{green}{0.39}$ | 74 kB | 9135 B | $\color{green}{0.12}$ | $\color{green}{+1.6}$ |
| 3840×2160@2 | 118 | 78.0 | $\color{green}{32.0}$ | $\color{green}{0.41}$ | 73 kB | 7262 B | $\color{green}{0.10}$ | $\color{green}{+1.6}$ |

### `flood`, a window of new text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 119 | 136.5 | $\color{green}{53.0}$ | $\color{green}{0.39}$ | 734 kB | 649 kB | $\color{green}{0.88}$ | $\color{green}{+0.4}$ |
| 1440×900 | 119 | 131.1 | $\color{green}{56.0}$ | $\color{green}{0.43}$ | 913 kB | 687 kB | $\color{green}{0.75}$ | $\color{green}{+0.5}$ |
| 1600×1000 | 119 | 164.6 | $\color{green}{81.2}$ | $\color{green}{0.48}$ | 1157 kB | 966 kB | $\color{green}{0.83}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 119 | 217.0 | $\color{green}{109.0}$ | $\color{green}{0.50}$ | 1592 kB | 1444 kB | $\color{green}{0.91}$ | $\color{green}{+0.4}$ |
| 2048×1536@2 | 119 | 215.0 | $\color{green}{120.2}$ | $\color{green}{0.56}$ | 1438 kB | 1310 kB | $\color{green}{0.91}$ | $\color{green}{+0.8}$ |
| 2560×1600@2 | 119 | 283.5 | $\color{green}{156.2}$ | $\color{green}{0.55}$ | 1855 kB | 1722 kB | $\color{green}{0.93}$ | $\color{green}{+0.8}$ |
| 2880×1800@2 | 119 | 372.6 | $\color{green}{202.4}$ | $\color{green}{0.55}$ | 2387 kB | 2269 kB | $\color{green}{0.95}$ | $\color{green}{+0.8}$ |
| 3200×2000@2 | 119 | 458.1 | $\color{green}{246.5}$ | $\color{green}{0.54}$ | 2913 kB | 2761 kB | $\color{green}{0.95}$ | $\color{green}{+0.8}$ |
| 3456×2168@2 | 119 | 547.4 | $\color{green}{292.3}$ | $\color{green}{0.54}$ | 3452 kB | 3292 kB | $\color{green}{0.95}$ | $\color{green}{+0.8}$ |
| 3840×2160@2 | 119 | 611.0 | $\color{green}{324.0}$ | $\color{green}{0.53}$ | 3835 kB | 3654 kB | $\color{green}{0.95}$ | $\color{green}{+0.8}$ |

### `uncover`, a window moved off a page

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 47 | 7.47 | $\color{green}{4.97}$ | $\color{green}{0.68}$ | 3304 B | 2544 B | $\color{green}{0.77}$ | $\color{green}{+1.0}$ |
| 1440×900 | 47 | 8.25 | $\color{green}{5.59}$ | $\color{green}{0.68}$ | 3480 B | 2566 B | $\color{green}{0.74}$ | $\color{green}{+0.9}$ |
| 1600×1000 | 47 | 8.85 | $\color{green}{5.78}$ | $\color{green}{0.65}$ | 3654 B | 1144 B | $\color{green}{0.31}$ | $\color{green}{+0.9}$ |
| 1920×1080 | 47 | 9.75 | $\color{green}{6.17}$ | $\color{green}{0.64}$ | 3993 B | 1152 B | $\color{green}{0.29}$ | $\color{green}{+0.8}$ |
| 2048×1536@2 | 45 | 19.2 | $\color{green}{11.6}$ | $\color{green}{0.58}$ | 15 kB | 4788 B | $\color{green}{0.32}$ | $\color{green}{+1.1}$ |
| 2560×1600@2 | 47 | 24.3 | $\color{green}{13.3}$ | $\color{green}{0.55}$ | 17 kB | 5094 B | $\color{green}{0.30}$ | $\color{green}{+1.1}$ |
| 2880×1800@2 | 47 | 26.0 | $\color{green}{14.8}$ | $\color{green}{0.55}$ | 17 kB | 5171 B | $\color{green}{0.30}$ | $\color{green}{+1.0}$ |
| 3200×2000@2 | 47 | 29.3 | $\color{green}{15.5}$ | $\color{green}{0.53}$ | 16 kB | 4776 B | $\color{green}{0.29}$ | $\color{green}{+0.9}$ |
| 3456×2168@2 | 47 | 31.1 | $\color{green}{16.2}$ | $\color{green}{0.53}$ | 17 kB | 5401 B | $\color{green}{0.31}$ | $\color{green}{+0.8}$ |
| 3840×2160@2 | 47 | 32.9 | $\color{green}{16.3}$ | $\color{green}{0.51}$ | 26 kB | 5121 B | $\color{green}{0.19}$ | $\color{green}{+0.8}$ |

### `video`, a frame of the video

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 118 | 18.8 | $\color{green}{17.1}$ | $\color{green}{0.86}$ | 36 kB | 38 kB | $\color{red}{1.05}$ | $\color{green}{+0.6}$ |
| 1440×900 | 118 | 20.4 | 20.4 | $\color{green}{0.94}$ | 43 kB | 45 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 1600×1000 | 118 | 23.6 | $\color{red}{24.3}$ | $\color{green}{0.95}$ | 50 kB | 52 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 1920×1080 | 118 | 28.5 | $\color{green}{27.3}$ | $\color{green}{0.92}$ | 55 kB | 57 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 2048×1536@2 | 118 | 43.3 | $\color{red}{46.7}$ | $\color{red}{1.03}$ | 91 kB | 93 kB | $\color{red}{1.02}$ | $\color{green}{+0.8}$ |
| 2560×1600@2 | 118 | 50.8 | $\color{red}{51.1}$ | $\color{green}{0.94}$ | 97 kB | 98 kB | $\color{red}{1.01}$ | $\color{green}{+0.8}$ |
| 2880×1800@2 | 118 | 63.1 | $\color{green}{61.5}$ | $\color{green}{0.92}$ | 114 kB | 114 kB | 1.00 | $\color{green}{+0.8}$ |
| 3200×2000@2 | 118 | 75.9 | $\color{green}{72.1}$ | $\color{green}{0.91}$ | 132 kB | 132 kB | 1.00 | $\color{green}{+0.8}$ |
| 3456×2168@2 | 118 | 86.4 | $\color{green}{84.5}$ | $\color{green}{0.93}$ | 146 kB | 147 kB | 1.00 | $\color{green}{+0.8}$ |
| 3840×2160@2 | 118 | 88.3 | $\color{green}{81.8}$ | $\color{green}{0.88}$ | 137 kB | 137 kB | 1.00 | $\color{green}{+0.9}$ |

### `walk`, a frame as the dial walks

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 200 | 15.6 | $\color{green}{12.4}$ | $\color{green}{0.79}$ | 15 kB | 16 kB | $\color{red}{1.09}$ | $\color{green}{+0.7}$ |
| 1440×900 | 204 | 16.9 | $\color{green}{13.2}$ | $\color{green}{0.78}$ | 18 kB | 19 kB | $\color{red}{1.07}$ | $\color{green}{+0.7}$ |
| 1600×1000 | 171 | 19.6 | $\color{green}{14.6}$ | $\color{green}{0.78}$ | 19 kB | 20 kB | $\color{red}{1.06}$ | $\color{green}{+0.6}$ |
| 1920×1080 | 171 | 22.8 | $\color{green}{16.3}$ | $\color{green}{0.74}$ | 21 kB | 22 kB | $\color{red}{1.05}$ | $\color{green}{+0.5}$ |
| 2048×1536@2 | 171 | 35.2 | $\color{green}{27.8}$ | $\color{green}{0.81}$ | 34 kB | 36 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 2560×1600@2 | 171 | 40.5 | $\color{green}{29.8}$ | $\color{green}{0.76}$ | 38 kB | 39 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 171 | 52.2 | $\color{green}{36.1}$ | $\color{green}{0.72}$ | 44 kB | 45 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 3200×2000@2 | 171 | 62.2 | $\color{green}{45.2}$ | $\color{green}{0.73}$ | 51 kB | 52 kB | $\color{red}{1.02}$ | $\color{green}{+0.5}$ |
| 3456×2168@2 | 171 | 69.2 | $\color{green}{51.5}$ | $\color{green}{0.76}$ | 57 kB | 58 kB | $\color{red}{1.01}$ | $\color{green}{+0.5}$ |
| 3840×2160@2 | 171 | 70.9 | $\color{green}{49.6}$ | $\color{green}{0.71}$ | 56 kB | 57 kB | $\color{red}{1.01}$ | $\color{green}{+0.4}$ |

### Keyframes inside `terminal`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 10.1 | $\color{green}{2.97}$ | $\color{green}{0.42}$ | 3460 B | 3285 B | $\color{green}{0.95}$ | $\color{green}{+0.5}$ |
| 1440×900 | 2 | 10.4 | $\color{green}{3.56}$ | $\color{green}{0.48}$ | 3490 B | 3150 B | $\color{green}{0.90}$ | $\color{green}{+0.3}$ |
| 1600×1000 | 2 | 11.9 | $\color{green}{4.58}$ | $\color{green}{0.49}$ | 3281 B | 2801 B | $\color{green}{0.85}$ | $\color{green}{+0.6}$ |
| 1920×1080 | 2 | 14.8 | $\color{green}{4.74}$ | $\color{green}{0.43}$ | 3716 B | 3061 B | $\color{green}{0.82}$ | $\color{green}{+0.1}$ |
| 2048×1536@2 | 2 | 15.0 | $\color{green}{6.70}$ | $\color{green}{0.61}$ | 8042 B | 7125 B | $\color{green}{0.89}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 2 | 18.8 | $\color{green}{7.75}$ | $\color{green}{0.58}$ | 8494 B | 7403 B | $\color{green}{0.87}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 2 | 22.0 | $\color{green}{9.55}$ | $\color{green}{0.54}$ | 8808 B | 7450 B | $\color{green}{0.85}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 2 | 29.1 | $\color{green}{12.7}$ | $\color{green}{0.49}$ | 7907 B | 6207 B | $\color{green}{0.79}$ | $\color{green}{+0.5}$ |
| 3456×2168@2 | 2 | 38.6 | $\color{green}{13.7}$ | $\color{green}{0.42}$ | 9751 B | 7637 B | $\color{green}{0.78}$ | $\color{green}{+0.6}$ |
| 3840×2160@2 | 2 | 44.1 | $\color{green}{17.3}$ | $\color{green}{0.53}$ | 10 kB | 7798 B | $\color{green}{0.77}$ | $\color{green}{+0.5}$ |

### Keyframes inside `uncover`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 30.4 | $\color{green}{13.2}$ | $\color{green}{0.43}$ | 105 kB | 105 kB | 1.00 | $\color{green}{+0.4}$ |
| 1440×900 | 1 | 29.2 | $\color{green}{13.7}$ | $\color{green}{0.47}$ | 120 kB | 117 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 1 | 47.2 | $\color{green}{21.8}$ | $\color{green}{0.46}$ | 160 kB | 158 kB | $\color{green}{0.99}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 1 | 43.6 | $\color{green}{24.5}$ | $\color{green}{0.56}$ | 204 kB | 202 kB | $\color{green}{0.99}$ | $\color{green}{+0.4}$ |
| 2048×1536@2 | 1 | 53.6 | $\color{green}{30.7}$ | $\color{green}{0.57}$ | 246 kB | 246 kB | 1.00 | $\color{green}{+0.3}$ |
| 2560×1600@2 | 1 | 46.7 | $\color{green}{29.7}$ | $\color{green}{0.64}$ | 261 kB | 260 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |
| 2880×1800@2 | 1 | 55.3 | $\color{green}{32.5}$ | $\color{green}{0.59}$ | 293 kB | 285 kB | $\color{green}{0.97}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 1 | 71.3 | $\color{green}{45.1}$ | $\color{green}{0.63}$ | 385 kB | 381 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |
| 3456×2168@2 | 1 | 80.3 | $\color{green}{52.8}$ | $\color{green}{0.66}$ | 456 kB | 451 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |
| 3840×2160@2 | 1 | 84.9 | $\color{green}{56.9}$ | $\color{green}{0.67}$ | 483 kB | 479 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |

### Keyframes inside `drag`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 22.3 | $\color{green}{10.1}$ | $\color{green}{0.50}$ | 87 kB | 87 kB | 1.00 | $\color{green}{+0.4}$ |
| 1440×900 | 2 | 26.1 | $\color{green}{12.5}$ | $\color{green}{0.53}$ | 87 kB | 87 kB | 1.00 | $\color{green}{+0.4}$ |
| 1600×1000 | 2 | 27.1 | $\color{green}{13.5}$ | $\color{green}{0.50}$ | 87 kB | 87 kB | 1.00 | $\color{green}{+0.4}$ |
| 1920×1080 | 2 | 26.7 | $\color{green}{12.1}$ | $\color{green}{0.51}$ | 87 kB | 87 kB | 1.00 | $\color{green}{+0.4}$ |
| 2048×1536@2 | 2 | 36.0 | $\color{green}{25.3}$ | $\color{green}{0.74}$ | 207 kB | 207 kB | 1.00 | $\color{green}{+0.4}$ |
| 2560×1600@2 | 2 | 43.3 | $\color{green}{24.2}$ | $\color{green}{0.65}$ | 208 kB | 208 kB | 1.00 | $\color{green}{+0.3}$ |
| 2880×1800@2 | 2 | 45.4 | $\color{green}{28.4}$ | $\color{green}{0.66}$ | 208 kB | 208 kB | 1.00 | $\color{green}{+0.4}$ |
| 3200×2000@2 | 2 | 64.2 | $\color{green}{28.7}$ | $\color{green}{0.48}$ | 209 kB | 208 kB | 1.00 | $\color{green}{+0.3}$ |
| 3456×2168@2 | 2 | 118.3 | $\color{green}{29.2}$ | $\color{green}{0.38}$ | 209 kB | 208 kB | $\color{green}{0.99}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 2 | 124.4 | $\color{green}{29.9}$ | $\color{green}{0.43}$ | 209 kB | 208 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |

### Keyframes inside `flood`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 115.8 | $\color{green}{50.9}$ | $\color{green}{0.44}$ | 775 kB | 779 kB | $\color{red}{1.01}$ | $\color{green}{+0.5}$ |
| 1440×900 | 1 | 89.1 | $\color{green}{62.4}$ | $\color{green}{0.70}$ | 1000 kB | 1007 kB | $\color{red}{1.01}$ | $\color{green}{+0.5}$ |
| 1600×1000 | 1 | 105.8 | $\color{green}{78.3}$ | $\color{green}{0.74}$ | 1250 kB | 1255 kB | 1.00 | $\color{green}{+0.5}$ |
| 1920×1080 | 1 | 136.9 | $\color{green}{108.8}$ | $\color{green}{0.80}$ | 1645 kB | 1656 kB | $\color{red}{1.01}$ | $\color{green}{+0.5}$ |
| 2048×1536@2 | 1 | 106.8 | $\color{green}{104.0}$ | $\color{green}{0.97}$ | 1433 kB | 1403 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 1 | 132.8 | $\color{green}{131.1}$ | $\color{green}{0.99}$ | 1786 kB | 1752 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 2880×1800@2 | 1 | 185.4 | $\color{green}{172.4}$ | $\color{green}{0.93}$ | 2330 kB | 2279 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 1 | 225.4 | $\color{green}{213.4}$ | $\color{green}{0.95}$ | 2854 kB | 2797 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 1 | 257.9 | $\color{red}{261.1}$ | $\color{red}{1.01}$ | 3559 kB | 3459 kB | $\color{green}{0.97}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 1 | 302.9 | $\color{red}{307.7}$ | $\color{red}{1.02}$ | 3929 kB | 3853 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |

## Against libvpx, keyframes of each capture's first picture, a paused video's last

### A keyframe at dial 40: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.45}$ | $\color{green}{0.46}$ | $\color{green}{0.48}$ | $\color{green}{0.44}$ | $\color{green}{0.57}$ | $\color{green}{0.60}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.49}$ | $\color{green}{0.55}$ |
| `still` | $\color{green}{0.56}$ | $\color{green}{0.68}$ | $\color{green}{0.76}$ | $\color{green}{0.69}$ | $\color{green}{0.78}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ | $\color{green}{0.78}$ |
| `uncover` | $\color{green}{0.62}$ | $\color{green}{0.60}$ | $\color{green}{0.69}$ | $\color{green}{0.75}$ | $\color{green}{0.83}$ | $\color{green}{0.84}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ |
| `flood` | $\color{green}{0.66}$ | $\color{green}{0.81}$ | $\color{green}{0.84}$ | $\color{green}{0.91}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | 1.00 | $\color{red}{1.03}$ | $\color{red}{1.04}$ |
| `browser` | $\color{green}{0.55}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.81}$ | $\color{green}{0.78}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.79}$ | $\color{green}{0.79}$ |
| `drag` | $\color{green}{0.58}$ | $\color{green}{0.64}$ | $\color{green}{0.61}$ | $\color{green}{0.58}$ | $\color{green}{0.87}$ | $\color{green}{0.73}$ | $\color{green}{0.78}$ | $\color{green}{0.73}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ |
| `video` | $\color{green}{0.81}$ | $\color{green}{0.87}$ | $\color{green}{0.89}$ | $\color{green}{0.80}$ | $\color{green}{0.90}$ | $\color{green}{0.77}$ | $\color{green}{0.78}$ | $\color{green}{0.70}$ | $\color{green}{0.74}$ | $\color{green}{0.68}$ |
| `walk` | $\color{green}{0.76}$ | $\color{green}{0.86}$ | $\color{green}{0.88}$ | $\color{green}{0.83}$ | $\color{green}{0.83}$ | $\color{green}{0.81}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{green}{0.75}$ | $\color{green}{0.71}$ |

### A keyframe at dial 40: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.88}$ | $\color{green}{0.85}$ | $\color{green}{0.78}$ | $\color{green}{0.73}$ | $\color{green}{0.81}$ | $\color{green}{0.79}$ | $\color{green}{0.76}$ | $\color{green}{0.69}$ | $\color{green}{0.65}$ | $\color{green}{0.67}$ |
| `still` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | 1.00 |
| `uncover` | $\color{red}{1.02}$ | 1.00 | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.98}$ | 1.00 | 1.00 | 1.00 |
| `flood` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `browser` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | 1.00 |
| `drag` | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | 1.00 |
| `video` | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.91}$ | $\color{green}{0.90}$ | $\color{green}{0.88}$ | $\color{green}{0.88}$ |
| `walk` | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.92}$ | $\color{green}{0.91}$ | $\color{green}{0.90}$ | $\color{green}{0.88}$ | $\color{green}{0.86}$ | $\color{green}{0.86}$ |

### A keyframe at dial 40: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+1.2}$ | $\color{green}{+2.9}$ | $\color{green}{+5.9}$ | $\color{green}{+1.3}$ | $\color{green}{+0.5}$ |
| `still` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `browser` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `drag` | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ |
| `video` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ |
| `walk` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |

### A keyframe at dial 65: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.39}$ | $\color{green}{0.47}$ | $\color{green}{0.45}$ | $\color{green}{0.45}$ | $\color{green}{0.60}$ | $\color{green}{0.59}$ | $\color{green}{0.54}$ | $\color{green}{0.53}$ | $\color{green}{0.55}$ | $\color{green}{0.55}$ |
| `still` | $\color{green}{0.56}$ | $\color{green}{0.63}$ | $\color{green}{0.69}$ | $\color{green}{0.66}$ | $\color{green}{0.81}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ | $\color{green}{0.72}$ | $\color{green}{0.75}$ | $\color{green}{0.75}$ |
| `uncover` | $\color{green}{0.53}$ | $\color{green}{0.55}$ | $\color{green}{0.61}$ | $\color{green}{0.66}$ | $\color{green}{0.77}$ | $\color{green}{0.76}$ | $\color{green}{0.72}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.76}$ |
| `flood` | $\color{green}{0.56}$ | $\color{green}{0.77}$ | $\color{green}{0.80}$ | $\color{green}{0.80}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | 1.00 | $\color{green}{0.98}$ | $\color{red}{1.01}$ | $\color{red}{1.04}$ |
| `browser` | $\color{green}{0.54}$ | $\color{green}{0.62}$ | $\color{green}{0.70}$ | $\color{green}{0.67}$ | $\color{green}{0.78}$ | $\color{green}{0.75}$ | $\color{green}{0.79}$ | $\color{green}{0.71}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ |
| `drag` | $\color{green}{0.55}$ | $\color{green}{0.58}$ | $\color{green}{0.55}$ | $\color{green}{0.55}$ | $\color{green}{0.84}$ | $\color{green}{0.73}$ | $\color{green}{0.76}$ | $\color{green}{0.69}$ | $\color{green}{0.66}$ | $\color{green}{0.68}$ |
| `video` | $\color{green}{0.80}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.99}$ | $\color{green}{0.97}$ | $\color{green}{0.92}$ | $\color{green}{0.87}$ | $\color{green}{0.89}$ | $\color{green}{0.84}$ |
| `walk` | $\color{green}{0.83}$ | $\color{green}{0.85}$ | $\color{green}{0.94}$ | $\color{green}{0.83}$ | $\color{green}{0.93}$ | $\color{green}{0.89}$ | $\color{green}{0.87}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ | $\color{green}{0.78}$ |

### A keyframe at dial 65: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.94}$ | $\color{green}{0.88}$ | $\color{green}{0.83}$ | $\color{green}{0.80}$ | $\color{green}{0.86}$ | $\color{green}{0.84}$ | $\color{green}{0.82}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.73}$ |
| `still` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | 1.00 |
| `uncover` | $\color{red}{1.02}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.98}$ | 1.00 | 1.00 | 1.00 |
| `flood` | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 |
| `browser` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | 1.00 |
| `drag` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 | 1.00 |
| `video` | $\color{red}{1.06}$ | $\color{red}{1.06}$ | $\color{red}{1.05}$ | $\color{red}{1.05}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | 1.00 | 1.00 |
| `walk` | $\color{red}{1.03}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | 1.00 | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ |

### A keyframe at dial 65: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ |
| `still` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `uncover` | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `flood` | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `browser` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `drag` | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |

### A keyframe at dial 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.38}$ | $\color{green}{0.43}$ | $\color{green}{0.48}$ | $\color{green}{0.43}$ | $\color{green}{0.55}$ | $\color{green}{0.56}$ | $\color{green}{0.56}$ | $\color{green}{0.55}$ | $\color{green}{0.53}$ | $\color{green}{0.56}$ |
| `still` | $\color{green}{0.51}$ | $\color{green}{0.62}$ | $\color{green}{0.65}$ | $\color{green}{0.61}$ | $\color{green}{0.75}$ | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.68}$ | $\color{green}{0.75}$ | $\color{green}{0.69}$ |
| `uncover` | $\color{green}{0.51}$ | $\color{green}{0.55}$ | $\color{green}{0.59}$ | $\color{green}{0.65}$ | $\color{green}{0.74}$ | $\color{green}{0.69}$ | $\color{green}{0.67}$ | $\color{green}{0.70}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ |
| `flood` | $\color{green}{0.53}$ | $\color{green}{0.70}$ | $\color{green}{0.75}$ | $\color{green}{0.77}$ | 1.00 | $\color{green}{0.98}$ | 1.00 | 1.00 | $\color{green}{0.98}$ | $\color{red}{1.02}$ |
| `browser` | $\color{green}{0.50}$ | $\color{green}{0.61}$ | $\color{green}{0.56}$ | $\color{green}{0.63}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.68}$ | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.76}$ |
| `drag` | $\color{green}{0.51}$ | $\color{green}{0.57}$ | $\color{green}{0.54}$ | $\color{green}{0.52}$ | $\color{green}{0.71}$ | $\color{green}{0.66}$ | $\color{green}{0.68}$ | $\color{green}{0.57}$ | $\color{green}{0.64}$ | $\color{green}{0.58}$ |
| `video` | $\color{green}{0.78}$ | $\color{green}{0.88}$ | $\color{green}{0.96}$ | $\color{green}{0.90}$ | $\color{green}{0.98}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.98}$ | $\color{green}{0.95}$ |
| `walk` | $\color{green}{0.75}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.91}$ | $\color{red}{1.02}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.91}$ | $\color{green}{0.90}$ |

### A keyframe at dial 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.95}$ | $\color{green}{0.90}$ | $\color{green}{0.85}$ | $\color{green}{0.82}$ | $\color{green}{0.89}$ | $\color{green}{0.87}$ | $\color{green}{0.84}$ | $\color{green}{0.79}$ | $\color{green}{0.78}$ | $\color{green}{0.77}$ |
| `still` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `uncover` | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.97}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `flood` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ |
| `browser` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `drag` | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `video` | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ |
| `walk` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 | $\color{green}{0.99}$ | 1.00 |

### A keyframe at dial 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.6}$ | $\color{green}{+0.3}$ | $\color{green}{+0.6}$ | $\color{green}{+0.1}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `still` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `browser` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ |
| `drag` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |

## Against libvpx, settles of each capture's first picture, a paused video's last

### A settle from dial 20 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.34}$ | $\color{green}{0.34}$ | $\color{green}{0.32}$ | $\color{green}{0.30}$ | $\color{green}{0.41}$ | $\color{green}{0.43}$ | $\color{green}{0.39}$ | $\color{green}{0.43}$ | $\color{green}{0.40}$ | $\color{green}{0.33}$ |
| `still` | $\color{green}{0.52}$ | $\color{green}{0.68}$ | $\color{green}{0.70}$ | $\color{green}{0.66}$ | $\color{green}{0.82}$ | $\color{green}{0.80}$ | $\color{green}{0.76}$ | $\color{green}{0.80}$ | $\color{green}{0.81}$ | $\color{green}{0.76}$ |
| `uncover` | $\color{green}{0.52}$ | $\color{green}{0.55}$ | $\color{green}{0.60}$ | $\color{green}{0.67}$ | $\color{green}{0.76}$ | $\color{green}{0.73}$ | $\color{green}{0.65}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ |
| `flood` | $\color{green}{0.48}$ | $\color{green}{0.69}$ | $\color{green}{0.68}$ | $\color{green}{0.75}$ | 1.00 | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `browser` | $\color{green}{0.53}$ | $\color{green}{0.67}$ | $\color{green}{0.75}$ | $\color{green}{0.66}$ | $\color{green}{0.80}$ | $\color{green}{0.78}$ | $\color{green}{0.74}$ | $\color{green}{0.79}$ | $\color{green}{0.80}$ | $\color{green}{0.74}$ |
| `drag` | $\color{green}{0.53}$ | $\color{green}{0.53}$ | $\color{green}{0.51}$ | $\color{green}{0.49}$ | $\color{green}{0.75}$ | $\color{green}{0.69}$ | $\color{green}{0.65}$ | $\color{green}{0.65}$ | $\color{green}{0.59}$ | $\color{green}{0.57}$ |
| `video` | $\color{green}{0.75}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{red}{1.18}$ | $\color{red}{1.13}$ | $\color{red}{1.14}$ | $\color{red}{1.10}$ | $\color{red}{1.14}$ | $\color{red}{1.11}$ |
| `walk` | $\color{green}{0.75}$ | $\color{green}{0.92}$ | $\color{red}{1.02}$ | $\color{green}{0.96}$ | $\color{red}{1.17}$ | $\color{red}{1.08}$ | $\color{red}{1.09}$ | $\color{red}{1.08}$ | $\color{red}{1.11}$ | $\color{red}{1.04}$ |

### A settle from dial 20 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{red}{1.04}$ | $\color{green}{0.96}$ | $\color{green}{0.93}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.92}$ | $\color{green}{0.95}$ | $\color{green}{0.92}$ |
| `still` | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ |
| `uncover` | $\color{green}{0.95}$ | $\color{green}{0.93}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ |
| `flood` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ |
| `browser` | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ |
| `drag` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ |
| `video` | $\color{green}{0.93}$ | $\color{green}{0.92}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.83}$ | $\color{green}{0.82}$ | $\color{green}{0.80}$ | $\color{green}{0.78}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ |
| `walk` | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.90}$ | $\color{green}{0.90}$ | $\color{green}{0.81}$ | $\color{green}{0.81}$ | $\color{green}{0.79}$ | $\color{green}{0.78}$ | $\color{green}{0.77}$ | $\color{green}{0.77}$ |

### A settle from dial 20 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ | $\color{green}{+1.0}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ |
| `still` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ |
| `uncover` | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `flood` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `browser` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ |
| `drag` | 0.0 | 0.0 | 0.0 | 0.0 | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ |
| `walk` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ |

### A settle from dial 65 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.49}$ | $\color{green}{0.49}$ | $\color{green}{0.41}$ | $\color{green}{0.45}$ | $\color{green}{0.68}$ | $\color{green}{0.65}$ | $\color{green}{0.61}$ | $\color{green}{0.60}$ | $\color{green}{0.60}$ | $\color{green}{0.59}$ |
| `still` | $\color{red}{1.05}$ | $\color{red}{1.19}$ | $\color{red}{1.22}$ | $\color{red}{1.14}$ | $\color{red}{1.46}$ | $\color{red}{1.30}$ | $\color{red}{1.22}$ | $\color{red}{1.33}$ | $\color{red}{1.36}$ | $\color{red}{1.32}$ |
| `uncover` | $\color{green}{0.98}$ | $\color{red}{1.03}$ | $\color{red}{1.08}$ | $\color{red}{1.20}$ | $\color{red}{1.33}$ | $\color{red}{1.27}$ | $\color{red}{1.22}$ | $\color{red}{1.27}$ | $\color{red}{1.32}$ | $\color{red}{1.42}$ |
| `flood` | $\color{green}{0.77}$ | $\color{red}{1.04}$ | $\color{red}{1.08}$ | $\color{red}{1.14}$ | $\color{red}{1.54}$ | $\color{red}{1.64}$ | $\color{red}{1.68}$ | $\color{red}{1.72}$ | $\color{red}{1.59}$ | $\color{red}{1.70}$ |
| `browser` | $\color{red}{1.01}$ | $\color{red}{1.19}$ | $\color{red}{1.35}$ | $\color{red}{1.20}$ | $\color{red}{1.44}$ | $\color{red}{1.32}$ | $\color{red}{1.23}$ | $\color{red}{1.30}$ | $\color{red}{1.40}$ | $\color{red}{1.28}$ |
| `drag` | $\color{green}{0.94}$ | $\color{green}{0.88}$ | $\color{green}{0.87}$ | $\color{green}{0.83}$ | $\color{red}{1.26}$ | $\color{red}{1.12}$ | $\color{red}{1.06}$ | $\color{green}{0.99}$ | $\color{green}{0.87}$ | $\color{green}{0.88}$ |
| `video` | $\color{red}{1.10}$ | $\color{red}{1.32}$ | $\color{red}{1.38}$ | $\color{red}{1.36}$ | $\color{red}{1.61}$ | $\color{red}{1.47}$ | $\color{red}{1.36}$ | $\color{red}{1.38}$ | $\color{red}{1.48}$ | $\color{red}{1.34}$ |
| `walk` | $\color{red}{1.04}$ | $\color{red}{1.27}$ | $\color{red}{1.33}$ | $\color{red}{1.27}$ | $\color{red}{1.57}$ | $\color{red}{1.47}$ | $\color{red}{1.41}$ | $\color{red}{1.33}$ | $\color{red}{1.36}$ | $\color{red}{1.35}$ |

### A settle from dial 65 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.93}$ | $\color{red}{1.01}$ | $\color{green}{0.93}$ | 1.00 | $\color{green}{0.96}$ | $\color{green}{0.99}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ |
| `still` | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ |
| `uncover` | $\color{green}{0.91}$ | $\color{green}{0.90}$ | $\color{green}{0.91}$ | $\color{green}{0.92}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ |
| `flood` | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `browser` | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ |
| `drag` | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.92}$ | $\color{green}{0.92}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ |
| `video` | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.93}$ |
| `walk` | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ |

### A settle from dial 65 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+1.6}$ | $\color{green}{+1.8}$ | $\color{green}{+0.7}$ | $\color{green}{+2.1}$ | $\color{green}{+0.7}$ | $\color{green}{+1.6}$ | $\color{green}{+0.9}$ | $\color{green}{+1.6}$ | $\color{green}{+1.5}$ | $\color{green}{+0.9}$ |
| `still` | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.0}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `uncover` | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ |
| `flood` | $\color{green}{+0.9}$ | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+0.9}$ | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ | $\color{green}{+1.5}$ | $\color{green}{+1.5}$ | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ |
| `browser` | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.2}$ | $\color{green}{+1.0}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `drag` | $\color{green}{+1.1}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `video` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `walk` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |

### A settle from dial 80 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.39}$ | $\color{green}{0.41}$ | $\color{green}{0.40}$ | $\color{green}{0.43}$ | $\color{green}{0.57}$ | $\color{green}{0.58}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ | $\color{green}{0.55}$ | $\color{green}{0.55}$ |
| `still` | $\color{green}{0.81}$ | $\color{green}{0.92}$ | 1.00 | $\color{green}{0.96}$ | $\color{red}{1.10}$ | $\color{red}{1.08}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.08}$ | $\color{red}{1.05}$ |
| `uncover` | $\color{green}{0.90}$ | $\color{green}{0.87}$ | $\color{green}{0.93}$ | 1.00 | $\color{red}{1.12}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.03}$ | $\color{red}{1.05}$ | $\color{red}{1.09}$ |
| `flood` | $\color{green}{0.99}$ | $\color{red}{1.29}$ | $\color{red}{1.38}$ | $\color{red}{1.41}$ | $\color{red}{1.91}$ | $\color{red}{1.91}$ | $\color{red}{1.96}$ | $\color{red}{1.87}$ | $\color{red}{1.83}$ | $\color{red}{1.89}$ |
| `browser` | $\color{green}{0.85}$ | $\color{green}{0.93}$ | $\color{red}{1.04}$ | $\color{green}{0.98}$ | $\color{red}{1.12}$ | $\color{red}{1.06}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.09}$ | $\color{red}{1.04}$ |
| `drag` | $\color{green}{0.75}$ | $\color{green}{0.73}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ | $\color{green}{0.99}$ | $\color{green}{0.86}$ | $\color{green}{0.88}$ | $\color{green}{0.81}$ | $\color{green}{0.79}$ | $\color{green}{0.75}$ |
| `video` | $\color{red}{1.05}$ | $\color{red}{1.21}$ | $\color{red}{1.24}$ | $\color{red}{1.15}$ | $\color{red}{1.52}$ | $\color{red}{1.35}$ | $\color{red}{1.35}$ | $\color{red}{1.30}$ | $\color{red}{1.44}$ | $\color{red}{1.32}$ |
| `walk` | $\color{green}{0.93}$ | $\color{red}{1.20}$ | $\color{red}{1.21}$ | $\color{red}{1.14}$ | $\color{red}{1.46}$ | $\color{red}{1.43}$ | $\color{red}{1.33}$ | $\color{red}{1.33}$ | $\color{red}{1.45}$ | $\color{red}{1.31}$ |

### A settle from dial 80 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.83}$ | $\color{green}{0.87}$ | $\color{green}{0.72}$ | $\color{red}{1.13}$ | $\color{green}{0.56}$ | $\color{green}{0.83}$ | $\color{green}{0.75}$ | $\color{green}{0.94}$ | $\color{green}{0.91}$ | $\color{green}{0.75}$ |
| `still` | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.64}$ | $\color{green}{0.67}$ | $\color{green}{0.70}$ | $\color{green}{0.76}$ | $\color{green}{0.79}$ | $\color{green}{0.72}$ | $\color{green}{0.73}$ | $\color{green}{0.71}$ |
| `uncover` | $\color{green}{0.70}$ | $\color{green}{0.69}$ | $\color{green}{0.65}$ | $\color{green}{0.69}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ | $\color{green}{0.75}$ | $\color{green}{0.73}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ |
| `flood` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.08}$ | $\color{red}{1.10}$ | $\color{red}{1.08}$ | $\color{red}{1.07}$ | $\color{red}{1.08}$ | $\color{red}{1.09}$ |
| `browser` | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.64}$ | $\color{green}{0.67}$ | $\color{green}{0.70}$ | $\color{green}{0.76}$ | $\color{green}{0.79}$ | $\color{green}{0.72}$ | $\color{green}{0.73}$ | $\color{green}{0.71}$ |
| `drag` | $\color{green}{0.80}$ | $\color{green}{0.80}$ | $\color{green}{0.80}$ | $\color{green}{0.80}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ |
| `video` | $\color{green}{0.83}$ | $\color{green}{0.82}$ | $\color{green}{0.81}$ | $\color{green}{0.79}$ | $\color{green}{0.79}$ | $\color{green}{0.79}$ | $\color{green}{0.80}$ | $\color{green}{0.81}$ | $\color{green}{0.85}$ | $\color{green}{0.82}$ |
| `walk` | $\color{green}{0.85}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.82}$ | $\color{green}{0.82}$ | $\color{green}{0.81}$ | $\color{green}{0.84}$ | $\color{green}{0.85}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ |

### A settle from dial 80 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | 0.0 | $\color{green}{+0.6}$ | $\color{red}{-0.4}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{red}{-0.1}$ |
| `still` | 0.0 | 0.0 | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `uncover` | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `flood` | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ | $\color{green}{+0.7}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `drag` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | 0.0 | 0.0 | $\color{green}{+0.1}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ |
| `walk` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |

### A settle from dial 89 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.38}$ | $\color{green}{0.36}$ | $\color{green}{0.36}$ | $\color{green}{0.36}$ | $\color{green}{0.53}$ | $\color{green}{0.52}$ | $\color{green}{0.52}$ | $\color{green}{0.51}$ | $\color{green}{0.55}$ | $\color{green}{0.54}$ |
| `still` | $\color{green}{0.72}$ | $\color{green}{0.82}$ | $\color{green}{0.84}$ | $\color{green}{0.82}$ | $\color{green}{0.88}$ | $\color{green}{0.92}$ | $\color{green}{0.83}$ | $\color{green}{0.90}$ | $\color{green}{0.91}$ | $\color{green}{0.89}$ |
| `uncover` | $\color{green}{0.85}$ | $\color{green}{0.79}$ | $\color{green}{0.85}$ | $\color{green}{0.85}$ | $\color{green}{0.89}$ | $\color{green}{0.91}$ | $\color{green}{0.87}$ | $\color{green}{0.90}$ | $\color{green}{0.90}$ | $\color{green}{0.90}$ |
| `flood` | $\color{red}{1.07}$ | $\color{red}{1.37}$ | $\color{red}{1.39}$ | $\color{red}{1.38}$ | $\color{red}{1.68}$ | $\color{red}{1.71}$ | $\color{red}{1.66}$ | $\color{red}{1.68}$ | $\color{red}{1.75}$ | $\color{red}{1.77}$ |
| `browser` | $\color{green}{0.75}$ | $\color{green}{0.79}$ | $\color{green}{0.91}$ | $\color{green}{0.84}$ | $\color{green}{0.89}$ | $\color{green}{0.92}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ | $\color{green}{0.90}$ | $\color{green}{0.85}$ |
| `drag` | $\color{green}{0.69}$ | $\color{green}{0.67}$ | $\color{green}{0.64}$ | $\color{green}{0.62}$ | $\color{green}{0.82}$ | $\color{green}{0.76}$ | $\color{green}{0.75}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.68}$ |
| `video` | $\color{green}{0.88}$ | $\color{red}{1.07}$ | $\color{red}{1.10}$ | $\color{red}{1.06}$ | $\color{red}{1.44}$ | $\color{red}{1.33}$ | $\color{red}{1.29}$ | $\color{red}{1.31}$ | $\color{red}{1.40}$ | $\color{red}{1.29}$ |
| `walk` | $\color{green}{0.87}$ | $\color{red}{1.09}$ | $\color{red}{1.07}$ | $\color{red}{1.08}$ | $\color{red}{1.39}$ | $\color{red}{1.31}$ | $\color{red}{1.30}$ | $\color{red}{1.31}$ | $\color{red}{1.28}$ | $\color{red}{1.34}$ |

### A settle from dial 89 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.98}$ | $\color{red}{1.44}$ | $\color{red}{1.16}$ | $\color{red}{1.55}$ | $\color{green}{0.62}$ | $\color{green}{0.80}$ | $\color{green}{0.80}$ | $\color{green}{0.66}$ | $\color{green}{0.94}$ | $\color{green}{0.66}$ |
| `still` | $\color{green}{0.70}$ | $\color{green}{0.65}$ | $\color{green}{0.76}$ | $\color{green}{0.74}$ | $\color{green}{0.58}$ | $\color{green}{0.79}$ | $\color{green}{0.75}$ | $\color{green}{0.66}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ |
| `uncover` | $\color{red}{1.05}$ | $\color{green}{0.98}$ | $\color{green}{0.77}$ | $\color{green}{0.75}$ | $\color{green}{0.39}$ | $\color{green}{0.43}$ | $\color{green}{0.44}$ | $\color{green}{0.49}$ | $\color{green}{0.48}$ | $\color{green}{0.54}$ |
| `flood` | $\color{green}{0.79}$ | $\color{green}{0.82}$ | $\color{green}{0.78}$ | $\color{green}{0.82}$ | $\color{green}{0.81}$ | $\color{green}{0.82}$ | $\color{green}{0.78}$ | $\color{green}{0.79}$ | $\color{green}{0.81}$ | $\color{green}{0.83}$ |
| `browser` | $\color{green}{0.70}$ | $\color{green}{0.65}$ | $\color{green}{0.67}$ | $\color{green}{0.74}$ | $\color{green}{0.58}$ | $\color{green}{0.79}$ | $\color{green}{0.75}$ | $\color{green}{0.66}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ |
| `drag` | $\color{green}{0.53}$ | $\color{green}{0.54}$ | $\color{green}{0.53}$ | $\color{green}{0.53}$ | $\color{green}{0.41}$ | $\color{green}{0.40}$ | $\color{green}{0.42}$ | $\color{green}{0.40}$ | $\color{green}{0.41}$ | $\color{green}{0.39}$ |
| `video` | $\color{green}{0.75}$ | $\color{green}{0.76}$ | $\color{green}{0.86}$ | $\color{green}{0.88}$ | $\color{red}{1.03}$ | $\color{red}{1.08}$ | $\color{red}{1.19}$ | $\color{red}{1.24}$ | $\color{red}{1.19}$ | $\color{red}{1.32}$ |
| `walk` | $\color{green}{0.75}$ | $\color{green}{0.81}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{red}{1.05}$ | $\color{red}{1.09}$ | $\color{red}{1.18}$ | $\color{red}{1.27}$ | $\color{red}{1.15}$ | $\color{red}{1.24}$ |

### A settle from dial 89 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ |
| `still` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `browser` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `drag` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |

## Test material

The captures are what a wlshare session handed `Stream`, written by its own
`--capture-frames` flag in the capture container, lossless, with the
rectangles each frame came with, `zstd` compressed: `<scenario>-<WxH>[@2].wlcap.zst`,
`@2` a desktop drawn at scale 2, as the README.md beside them says. Where
a machine keeps them is in its `CLAUDE.local.md`, not here; the harness
reads `tmp/samples/` or the directory `CAPTURES` names, and `ONLY` keeps
the captures whose name holds it.

| Scenario | Kind | What a frame names |
|---|---|---|
| `terminal` | quiet | a resize into the size, then a terminal typed into one key at a time and scrolled: a line or two a frame |
| `still` | quiet | a page in Chromium left alone: two keyframes, the second the first again to the pixel |
| `uncover` | quiet | a floating terminal over a page moved aside 20 points a tenth of a second, uncovering a strip each move, then closed: the whole surface, by the compositor's damage |
| `flood` | busy | a terminal flooded with coloured random lines, every frame the whole window new |
| `browser` | busy | a long page in Chromium scrolled by wheel, by key and back: the whole surface |
| `drag` | busy | a floating terminal dragged around the desktop: a quarter of the picture |
| `video` | busy | a 4K clip of coloured explosions played full screen, every pixel the video's, paused on its first frame, which is black, until the scenario goes; a keyframe asked at 10 s |
| `walk` | busy | the video while the client reads late from 4 s to 10 s, so the dial walks down to 20 or 30 and partway back up, to 41 to 75, then paused where it stands, the desktop settling at 90 on that frame, which is the picture its keyframes and settles are measured on; captured again so on 2026-10-15: the capture before stopped the clip, which left the screen black, and its session settled that |

Sizes: the ten `capture-frames.sh` runs by default, which vp9-wasm's
benchmark has samples of: 1280×800, 1440×900, 1600×1000 and 1920×1080 at
scale 1, and the same four desktops at scale 2, 2560×1600, 2880×1800,
3200×2000 and 3840×2160, with 2048×1536 and 3456×2168. The set is the
`frame-captures` of 2026-10-13 in the artifacts' backup
(`/Volumes/dasdata/backup/vp9-artifacts/frame-captures/`, 80 files), its
`uncover` trimmed on 2026-10-14 to begin where the terminal first moves,
and `walk` captured again on 2026-10-15 (its old files in
`archive/frame-captures-walk-stopped/`). `video` from 2560×1600 up is made
from the 1920×1080 session frame for frame with the clip scaled to the
size, and `walk` from 1920×1080 up from the 1600×1000 session, since the
capture container does not keep up there: `walk` was captured on this
machine, whose container plays 1920×1080 at 12 to 22 frames a second, and
the Mac's, which played it at 25, was not to hand. Each is measured at quality 90, the dial
the captures were made at, on four threads.

The tables are of those captures cut to 120 frames each, under the same
names, but `walk`, kept whole, read with `CAPTURES` naming their directory
(`frame-samples/` beside the captures, 9.3 GB against their 20). wlshare's
`scripts/capture-samples.sh` cuts them (`vp9cap trim --frames`) from each
capture's longest run of a size as vp9-wasm's samples are cut: a quiet
scenario's first 120 frames, a busy one's the 120 in a row whose frames
said the most pixels changed, the earliest where several tie; `still` (2
frames) and `uncover` (48 or 49) are shorter and kept whole, and `walk`
(174 to 207) is kept whole since its dial walks down, back up and settles
over all of it, a cut of it keeping only the walk down. A cut's first
frame is the picture as it stood, whole, with a keyframe asked: most cuts
begin where their capture does, which is that already, and only `flood`'s
(frame 4 to 6 on) and `browser` at 1600×1000's (frame 97 on) are a
keyframe the session never asked for, which `Key` counts apart.
`terminal`'s begins at its resize into the size, so its first picture is a
terminal of that size. A cut keeps nothing of what comes later in its
capture: `video`'s keyframe at 10 s. A cut is a third to a half of its
capture's frames.

The eighteen decoded samples the first gate read, and the libvpx-coded
samples `vp9-samples.sh` makes for vp9-wasm, are not encoder material: the
first are decodes of decodes, the second are coded by the encoder under
test's rival.
