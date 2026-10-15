# What the encoder was measured at

The numbers the encoder's decisions were taken on. How the encoder works is
in [encoder.md](encoder.md); what was built in which order, and why, in
[plan.md](plan.md).

The releases named here, 0.0.1-alpha.2 to alpha.5, are those of
`screen-vp9-native`, the crate this encoder was until it came home to
`screen-vp9` as `Coder::Native`. The tables are against libvpx as
`screen-vp9` 0.0.18 sets it up, the crate's other coder (`src/libvpx.rs`),
by the three measurements in `src/native/mod.rs`:
`captures_against_libvpx` plays each capture into both encoders, frame by
frame, turn about; `keyframes_against_libvpx` codes each capture's first
picture as a keyframe at dials 40, 65 and 90, and `video`'s and `walk`'s
last of more than one colour, since both begin on the clip paused on
black and `walk` ends on black; `settles_against_libvpx`
codes it at one dial and settles it at 90, the picture unchanged. The last
two keep the least time of five passes, each on one encoder kept across
them, as a session's is. `tools/tables.py` makes the tables below from
their logs.

They were run on 2026-10-14 on a six-core Intel i5-8500T (x86-64-v3) under
WSL2, four threads, the machine idle and one measurement at a time, on the
build after release 0.0.18 that has the vectors between pixels back
([plan.md](plan.md#where-a-videos-bytes-go-on-2026-10-14)) and the
search's probe measuring only, over the captures of 2026-10-13 cut to 120
frames each, `walk` whole ([Test material](#test-material)): the three
took 49 minutes. A time on this machine moves by a tenth from one run to
the next, so a ratio within a tenth of 1.00 is on the line. The same
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

- **A video is a little over the line in bytes at every size, and over
  libvpx's picture.** `video` takes 1.04 to 1.06 of libvpx's bytes over
  its frames at +0.4 to +0.7 dB; `walk`, the same video as the dial walks
  down to 20 and back up to 89, 1.08 to 1.12 at +0.4 to +0.5. Without
  vectors between pixels, at 0.0.18, they were 1.22 to 1.27 and 1.16 to
  1.25 (`walk` cut to its walk down), at up to 0.6 dB under libvpx's
  picture: where a video's bytes go is measured in
  [plan.md](plan.md#where-a-videos-bytes-go-on-2026-10-14). The test
  pattern the set before played instead had them at 0.95 to 1.10 of the
  bytes and over libvpx's picture without them.
- **Everything else over a whole capture is libvpx's picture or better, at
  fewer bytes, but `drag` from 2048 to 3456 wide**, 0.1 to 0.4 dB under at
  0.42 to 0.52 of the bytes: `browser` +0.8 to +1.6 dB at 0.20–0.34×,
  `uncover` +0.7 to +1.1 at 0.43–0.89×, `flood` +0.3 to +0.7 at
  0.76–0.98×, `still` +0.4 at 0.98–1.00×, `terminal` +0.1 to +0.4 at
  0.81–0.99×, `drag` elsewhere +0.2 to +0.3 at 0.22–0.27×. That is 75
  cells of 80 at libvpx's picture or over.
- **Every capture at every size takes less time than libvpx**, frame for
  frame and summed: a line of text 0.13–0.19, `flood` 0.35–0.47,
  `browser` 0.36–0.42, `drag` 0.43–0.54, `uncover` 0.48–0.65, `walk`
  0.71–0.78, `video` 0.80–0.91, `still`'s two keyframes 0.44–0.72.
- **A keyframe inside a capture is 0.1 to 0.6 dB over libvpx's** at
  0.97–1.01× the bytes (`terminal`'s 0.77–0.95×), in 0.40 to 0.94 of its
  time but `flood`'s at 3200 and 3840 wide, 1.10 and 1.03.
- **A first picture coded alone as a keyframe is 0.2 to 0.7 dB over
  libvpx's at its bytes** at every dial for the screens but `terminal`,
  at 0.97–1.03× the bytes. **`terminal`'s at dial 40 is 2.4 to 7.7 dB
  under libvpx's**, at 0.64–0.87× its bytes: a terminal with a few lines
  in it, 1.6 to 6.2 kB, where libvpx keeps the text at 49 to 55 dB and
  this encoder at 46 to 47.5 at every size. At 65 and 90 the same picture
  is 0.1 to 0.6 dB over libvpx's at 0.74–0.95× the bytes. **A picture of
  the video** is level with libvpx's: at 40, 0.86–1.01× the bytes at +0.1
  to −0.3 dB, under from 1600 wide for `walk`'s and 2560 for `video`'s; at
  65 and 90, 0.97–1.06× at +0.2 to +0.6. Up to 1920 wide a keyframe takes 0.40 to 0.93 of libvpx's time;
  from 2048 wide 0.56 to 1.03, nine cells of 144 over 1.00, eight
  `flood`'s and one `video`'s, by 0.03 at most.
- **A settle of a busy picture takes more time than libvpx's** from 65:
  `still`, `uncover` and `browser` 1.05 to 1.38 of it from 1440 wide, for
  1.1 to 1.5 dB more picture at 0.93–1.00× the bytes; `flood` 0.99 to
  1.62 from 1440 wide; a picture of the video 1.21 to 1.56 from 1440 wide, at +0.1 to
  +0.4 dB and 0.95–1.00× the bytes; `drag` 0.80 to 1.20. From 80 `flood`
  is 1.26 to 1.89 of it from 1440 wide, at 1.02 to 1.13 of the bytes, the
  video 1.13 to 1.50 at 0.80–0.94× the bytes, and `still`, `uncover` and
  `browser` 0.84 to 1.11, on the line; from 89 `flood` is 1.32 to 1.68
  from 1440 wide, the video 1.00 to 1.47, and the others 0.34 to 0.92.
  From 20 a settle is 0.32 to 0.96 of libvpx's time but the video's,
  0.67 to 1.14. `terminal`'s settles are 0.32 to 0.62 of it.
- **What is over the line in bytes, besides the video over its frames**:
  a settle from 89 of a picture of the video from 2048 wide (1.06 to
  1.52×, at +0.4 to +0.5 dB), of `terminal` (1.04 to 1.82× at seven
  sizes, a few hundred bytes) and of `uncover` up to 1920 wide (1.16 to
  1.75×); a settle of `flood` from 80 (1.02 to 1.13×) and from 65 (1.02
  to 1.03× from 2048 wide); single cells of `terminal`'s settles from 20,
  65 and 80 (up to 1.18×); and a line of text at three sizes (1.12 to
  1.15×, ten to thirty bytes a frame).

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
| `terminal` | $\color{green}{0.94}$ | $\color{green}{0.99}$ | $\color{green}{0.96}$ | $\color{green}{0.94}$ | $\color{green}{0.81}$ | $\color{green}{0.82}$ | $\color{green}{0.86}$ | $\color{green}{0.86}$ | $\color{green}{0.82}$ | $\color{green}{0.82}$ |
| `still` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `uncover` | $\color{green}{0.89}$ | $\color{green}{0.87}$ | $\color{green}{0.67}$ | $\color{green}{0.72}$ | $\color{green}{0.51}$ | $\color{green}{0.49}$ | $\color{green}{0.50}$ | $\color{green}{0.54}$ | $\color{green}{0.57}$ | $\color{green}{0.43}$ |
| `flood` | $\color{green}{0.90}$ | $\color{green}{0.76}$ | $\color{green}{0.85}$ | $\color{green}{0.92}$ | $\color{green}{0.93}$ | $\color{green}{0.95}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ |
| `browser` | $\color{green}{0.27}$ | $\color{green}{0.24}$ | $\color{green}{0.29}$ | $\color{green}{0.34}$ | $\color{green}{0.23}$ | $\color{green}{0.20}$ | $\color{green}{0.20}$ | $\color{green}{0.20}$ | $\color{green}{0.22}$ | $\color{green}{0.20}$ |
| `drag` | $\color{green}{0.24}$ | $\color{green}{0.24}$ | $\color{green}{0.25}$ | $\color{green}{0.22}$ | $\color{green}{0.42}$ | $\color{green}{0.50}$ | $\color{green}{0.52}$ | $\color{green}{0.47}$ | $\color{green}{0.47}$ | $\color{green}{0.27}$ |
| `video` | $\color{red}{1.06}$ | $\color{red}{1.06}$ | $\color{red}{1.06}$ | $\color{red}{1.06}$ | $\color{red}{1.05}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ | $\color{red}{1.04}$ |
| `walk` | $\color{red}{1.12}$ | $\color{red}{1.12}$ | $\color{red}{1.12}$ | $\color{red}{1.11}$ | $\color{red}{1.10}$ | $\color{red}{1.10}$ | $\color{red}{1.09}$ | $\color{red}{1.08}$ | $\color{red}{1.09}$ | $\color{red}{1.09}$ |

### Encode time, the median of the per-frame ratios

Under 1.00 is faster than libvpx, frame for frame.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.18}$ | $\color{green}{0.15}$ | $\color{green}{0.15}$ | $\color{green}{0.13}$ | $\color{green}{0.19}$ | $\color{green}{0.16}$ | $\color{green}{0.15}$ | $\color{green}{0.15}$ | $\color{green}{0.14}$ | $\color{green}{0.15}$ |
| `still` | $\color{green}{0.44}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.61}$ | $\color{green}{0.68}$ | $\color{green}{0.66}$ | $\color{green}{0.66}$ | $\color{green}{0.65}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ |
| `uncover` | $\color{green}{0.65}$ | $\color{green}{0.63}$ | $\color{green}{0.64}$ | $\color{green}{0.60}$ | $\color{green}{0.53}$ | $\color{green}{0.51}$ | $\color{green}{0.48}$ | $\color{green}{0.51}$ | $\color{green}{0.48}$ | $\color{green}{0.48}$ |
| `flood` | $\color{green}{0.35}$ | $\color{green}{0.38}$ | $\color{green}{0.42}$ | $\color{green}{0.43}$ | $\color{green}{0.47}$ | $\color{green}{0.47}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ | $\color{green}{0.46}$ | $\color{green}{0.45}$ |
| `browser` | $\color{green}{0.36}$ | $\color{green}{0.37}$ | $\color{green}{0.38}$ | $\color{green}{0.39}$ | $\color{green}{0.42}$ | $\color{green}{0.41}$ | $\color{green}{0.40}$ | $\color{green}{0.39}$ | $\color{green}{0.36}$ | $\color{green}{0.39}$ |
| `drag` | $\color{green}{0.43}$ | $\color{green}{0.46}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ | $\color{green}{0.51}$ | $\color{green}{0.51}$ | $\color{green}{0.51}$ | $\color{green}{0.49}$ | $\color{green}{0.50}$ | $\color{green}{0.54}$ |
| `video` | $\color{green}{0.80}$ | $\color{green}{0.85}$ | $\color{green}{0.88}$ | $\color{green}{0.85}$ | $\color{green}{0.91}$ | $\color{green}{0.87}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.81}$ |
| `walk` | $\color{green}{0.71}$ | $\color{green}{0.78}$ | $\color{green}{0.75}$ | $\color{green}{0.73}$ | $\color{green}{0.77}$ | $\color{green}{0.73}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ |

### Encode time, the whole capture's

Every frame's time summed, this encoder's over libvpx's: where the long frames weigh.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.21}$ | $\color{green}{0.17}$ | $\color{green}{0.18}$ | $\color{green}{0.16}$ | $\color{green}{0.22}$ | $\color{green}{0.20}$ | $\color{green}{0.19}$ | $\color{green}{0.19}$ | $\color{green}{0.17}$ | $\color{green}{0.17}$ |
| `still` | $\color{green}{0.44}$ | $\color{green}{0.57}$ | $\color{green}{0.60}$ | $\color{green}{0.59}$ | $\color{green}{0.64}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.67}$ | $\color{green}{0.68}$ |
| `uncover` | $\color{green}{0.61}$ | $\color{green}{0.59}$ | $\color{green}{0.58}$ | $\color{green}{0.56}$ | $\color{green}{0.52}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ | $\color{green}{0.48}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ |
| `flood` | $\color{green}{0.35}$ | $\color{green}{0.39}$ | $\color{green}{0.41}$ | $\color{green}{0.44}$ | $\color{green}{0.48}$ | $\color{green}{0.47}$ | $\color{green}{0.48}$ | $\color{green}{0.47}$ | $\color{green}{0.47}$ | $\color{green}{0.46}$ |
| `browser` | $\color{green}{0.35}$ | $\color{green}{0.38}$ | $\color{green}{0.38}$ | $\color{green}{0.41}$ | $\color{green}{0.42}$ | $\color{green}{0.39}$ | $\color{green}{0.40}$ | $\color{green}{0.37}$ | $\color{green}{0.36}$ | $\color{green}{0.37}$ |
| `drag` | $\color{green}{0.44}$ | $\color{green}{0.45}$ | $\color{green}{0.50}$ | $\color{green}{0.46}$ | $\color{green}{0.51}$ | $\color{green}{0.51}$ | $\color{green}{0.51}$ | $\color{green}{0.48}$ | $\color{green}{0.64}$ | $\color{green}{0.53}$ |
| `video` | $\color{green}{0.83}$ | $\color{green}{0.87}$ | $\color{green}{0.90}$ | $\color{green}{0.85}$ | $\color{green}{0.93}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.81}$ |
| `walk` | $\color{green}{0.71}$ | $\color{green}{0.83}$ | $\color{green}{0.77}$ | $\color{green}{0.75}$ | $\color{green}{0.80}$ | $\color{green}{0.76}$ | $\color{green}{0.73}$ | $\color{green}{0.73}$ | $\color{green}{0.73}$ | $\color{green}{0.71}$ |

### Luma PSNR, this encoder less libvpx, in dB

Above zero is a better picture than libvpx at the same dial.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ |
| `still` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.7}$ | $\color{green}{+1.1}$ | $\color{green}{+1.0}$ | $\color{green}{+1.0}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.7}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ |
| `browser` | $\color{green}{+0.8}$ | $\color{green}{+1.0}$ | $\color{green}{+1.0}$ | $\color{green}{+0.9}$ | $\color{green}{+1.5}$ | $\color{green}{+1.6}$ | $\color{green}{+1.6}$ | $\color{green}{+1.6}$ | $\color{green}{+1.5}$ | $\color{green}{+1.6}$ |
| `drag` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{red}{-0.4}$ | $\color{red}{-0.3}$ | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |

## Against libvpx, the frames apart

The time columns are each encoder's median time for a frame of that kind, the
time ratio the median of the per-frame ratios, and the bytes the mean per frame.

### `terminal`, a line of text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 117 | 2.19 | $\color{green}{0.40}$ | $\color{green}{0.18}$ | 116 B | 102 B | $\color{green}{0.88}$ | $\color{green}{+0.1}$ |
| 1440×900 | 117 | 3.35 | $\color{green}{0.51}$ | $\color{green}{0.15}$ | 118 B | 132 B | $\color{red}{1.12}$ | $\color{green}{+0.1}$ |
| 1600×1000 | 117 | 3.48 | $\color{green}{0.49}$ | $\color{green}{0.14}$ | 124 B | 124 B | 1.00 | $\color{green}{+0.1}$ |
| 1920×1080 | 117 | 4.29 | $\color{green}{0.56}$ | $\color{green}{0.13}$ | 145 B | 137 B | $\color{green}{0.94}$ | $\color{green}{+0.1}$ |
| 2048×1536@2 | 117 | 4.41 | $\color{green}{0.84}$ | $\color{green}{0.19}$ | 224 B | 196 B | $\color{green}{0.88}$ | $\color{green}{+0.3}$ |
| 2560×1600@2 | 117 | 5.47 | $\color{green}{0.87}$ | $\color{green}{0.16}$ | 185 B | 171 B | $\color{green}{0.92}$ | $\color{green}{+0.3}$ |
| 2880×1800@2 | 117 | 6.86 | $\color{green}{1.07}$ | $\color{green}{0.15}$ | 191 B | 220 B | $\color{red}{1.15}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 117 | 8.11 | $\color{green}{1.21}$ | $\color{green}{0.15}$ | 192 B | 215 B | $\color{red}{1.12}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 117 | 9.37 | $\color{green}{1.31}$ | $\color{green}{0.14}$ | 217 B | 207 B | $\color{green}{0.95}$ | $\color{green}{+0.3}$ |
| 3840×2160@2 | 117 | 10.1 | $\color{green}{1.48}$ | $\color{green}{0.15}$ | 244 B | 232 B | $\color{green}{0.95}$ | $\color{green}{+0.4}$ |

### `terminal`, a scroll

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 10.7 | $\color{green}{6.62}$ | $\color{green}{0.62}$ | 51 kB | 49 kB | $\color{green}{0.96}$ | 0.0 |
| 1440×900 | 1 | 13.2 | $\color{green}{7.64}$ | $\color{green}{0.58}$ | 50 kB | 48 kB | $\color{green}{0.96}$ | 0.0 |
| 1600×1000 | 1 | 12.4 | $\color{green}{7.10}$ | $\color{green}{0.57}$ | 50 kB | 48 kB | $\color{green}{0.96}$ | 0.0 |
| 1920×1080 | 1 | 15.2 | $\color{green}{8.41}$ | $\color{green}{0.55}$ | 51 kB | 48 kB | $\color{green}{0.96}$ | $\color{green}{+0.1}$ |
| 2048×1536@2 | 1 | 22.8 | $\color{green}{13.5}$ | $\color{green}{0.59}$ | 135 kB | 107 kB | $\color{green}{0.79}$ | $\color{green}{+0.2}$ |
| 2560×1600@2 | 1 | 25.6 | $\color{green}{20.2}$ | $\color{green}{0.79}$ | 134 kB | 107 kB | $\color{green}{0.80}$ | $\color{green}{+0.2}$ |
| 2880×1800@2 | 1 | 36.2 | $\color{green}{18.5}$ | $\color{green}{0.51}$ | 133 kB | 109 kB | $\color{green}{0.82}$ | $\color{green}{+0.2}$ |
| 3200×2000@2 | 1 | 30.8 | $\color{green}{16.4}$ | $\color{green}{0.53}$ | 121 kB | 99 kB | $\color{green}{0.82}$ | $\color{green}{+0.3}$ |
| 3456×2168@2 | 1 | 34.7 | $\color{green}{16.9}$ | $\color{green}{0.49}$ | 130 kB | 104 kB | $\color{green}{0.80}$ | $\color{green}{+0.2}$ |
| 3840×2160@2 | 1 | 36.2 | $\color{green}{17.9}$ | $\color{green}{0.50}$ | 133 kB | 107 kB | $\color{green}{0.80}$ | $\color{green}{+0.3}$ |

### `drag`, a quarter of the picture

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 118 | 10.9 | $\color{green}{4.71}$ | $\color{green}{0.43}$ | 5850 B | 282 B | $\color{green}{0.05}$ | $\color{green}{+0.3}$ |
| 1440×900 | 118 | 10.7 | $\color{green}{4.83}$ | $\color{green}{0.46}$ | 5841 B | 305 B | $\color{green}{0.05}$ | $\color{green}{+0.3}$ |
| 1600×1000 | 118 | 11.1 | $\color{green}{5.09}$ | $\color{green}{0.47}$ | 5725 B | 291 B | $\color{green}{0.05}$ | $\color{green}{+0.2}$ |
| 1920×1080 | 118 | 11.6 | $\color{green}{5.20}$ | $\color{green}{0.46}$ | 6546 B | 284 B | $\color{green}{0.04}$ | $\color{green}{+0.2}$ |
| 2048×1536@2 | 118 | 21.3 | $\color{green}{10.9}$ | $\color{green}{0.51}$ | 7332 B | 1093 B | $\color{green}{0.15}$ | $\color{red}{-0.4}$ |
| 2560×1600@2 | 118 | 22.2 | $\color{green}{11.2}$ | $\color{green}{0.51}$ | 5708 B | 1084 B | $\color{green}{0.19}$ | $\color{red}{-0.3}$ |
| 2880×1800@2 | 118 | 24.1 | $\color{green}{12.2}$ | $\color{green}{0.51}$ | 5329 B | 1124 B | $\color{green}{0.21}$ | $\color{red}{-0.2}$ |
| 3200×2000@2 | 118 | 25.8 | $\color{green}{12.5}$ | $\color{green}{0.49}$ | 6453 B | 1117 B | $\color{green}{0.17}$ | $\color{red}{-0.1}$ |
| 3456×2168@2 | 118 | 27.9 | $\color{green}{13.4}$ | $\color{green}{0.50}$ | 6359 B | 1101 B | $\color{green}{0.17}$ | $\color{red}{-0.1}$ |
| 3840×2160@2 | 118 | 25.1 | $\color{green}{13.3}$ | $\color{green}{0.54}$ | 14 kB | 1123 B | $\color{green}{0.08}$ | $\color{green}{+0.3}$ |

### `browser`, a page scrolled

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 118 | 19.1 | $\color{green}{6.84}$ | $\color{green}{0.36}$ | 9303 B | 1213 B | $\color{green}{0.13}$ | $\color{green}{+0.8}$ |
| 1440×900 | 118 | 21.8 | $\color{green}{8.02}$ | $\color{green}{0.37}$ | 13 kB | 1829 B | $\color{green}{0.14}$ | $\color{green}{+1.0}$ |
| 1600×1000 | 119 | 25.6 | $\color{green}{9.49}$ | $\color{green}{0.38}$ | 15 kB | 3351 B | $\color{green}{0.23}$ | $\color{green}{+1.0}$ |
| 1920×1080 | 118 | 28.5 | $\color{green}{11.0}$ | $\color{green}{0.39}$ | 12 kB | 2298 B | $\color{green}{0.19}$ | $\color{green}{+0.9}$ |
| 2048×1536@2 | 118 | 36.8 | $\color{green}{15.5}$ | $\color{green}{0.42}$ | 40 kB | 6374 B | $\color{green}{0.16}$ | $\color{green}{+1.5}$ |
| 2560×1600@2 | 118 | 48.8 | $\color{green}{19.7}$ | $\color{green}{0.41}$ | 50 kB | 7271 B | $\color{green}{0.14}$ | $\color{green}{+1.6}$ |
| 2880×1800@2 | 118 | 56.5 | $\color{green}{22.1}$ | $\color{green}{0.40}$ | 52 kB | 7030 B | $\color{green}{0.14}$ | $\color{green}{+1.6}$ |
| 3200×2000@2 | 118 | 67.7 | $\color{green}{25.8}$ | $\color{green}{0.39}$ | 65 kB | 7879 B | $\color{green}{0.12}$ | $\color{green}{+1.6}$ |
| 3456×2168@2 | 118 | 76.5 | $\color{green}{27.7}$ | $\color{green}{0.36}$ | 74 kB | 11 kB | $\color{green}{0.15}$ | $\color{green}{+1.5}$ |
| 3840×2160@2 | 118 | 79.3 | $\color{green}{29.6}$ | $\color{green}{0.39}$ | 73 kB | 9223 B | $\color{green}{0.13}$ | $\color{green}{+1.6}$ |

### `flood`, a window of new text

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 119 | 136.6 | $\color{green}{47.7}$ | $\color{green}{0.35}$ | 734 kB | 657 kB | $\color{green}{0.90}$ | $\color{green}{+0.3}$ |
| 1440×900 | 119 | 131.4 | $\color{green}{48.2}$ | $\color{green}{0.38}$ | 913 kB | 695 kB | $\color{green}{0.76}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 119 | 164.1 | $\color{green}{69.8}$ | $\color{green}{0.42}$ | 1157 kB | 978 kB | $\color{green}{0.84}$ | $\color{green}{+0.4}$ |
| 1920×1080 | 119 | 257.1 | $\color{green}{112.1}$ | $\color{green}{0.43}$ | 1592 kB | 1462 kB | $\color{green}{0.92}$ | $\color{green}{+0.3}$ |
| 2048×1536@2 | 119 | 216.2 | $\color{green}{103.0}$ | $\color{green}{0.47}$ | 1438 kB | 1342 kB | $\color{green}{0.93}$ | $\color{green}{+0.6}$ |
| 2560×1600@2 | 119 | 279.9 | $\color{green}{131.1}$ | $\color{green}{0.47}$ | 1855 kB | 1765 kB | $\color{green}{0.95}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 119 | 367.4 | $\color{green}{171.1}$ | $\color{green}{0.47}$ | 2387 kB | 2325 kB | $\color{green}{0.97}$ | $\color{green}{+0.7}$ |
| 3200×2000@2 | 119 | 449.4 | $\color{green}{205.9}$ | $\color{green}{0.46}$ | 2913 kB | 2830 kB | $\color{green}{0.97}$ | $\color{green}{+0.7}$ |
| 3456×2168@2 | 119 | 537.8 | $\color{green}{245.8}$ | $\color{green}{0.46}$ | 3452 kB | 3373 kB | $\color{green}{0.98}$ | $\color{green}{+0.7}$ |
| 3840×2160@2 | 119 | 603.5 | $\color{green}{269.6}$ | $\color{green}{0.45}$ | 3835 kB | 3745 kB | $\color{green}{0.98}$ | $\color{green}{+0.7}$ |

### `uncover`, a window moved off a page

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 47 | 7.53 | $\color{green}{4.86}$ | $\color{green}{0.65}$ | 3304 B | 2715 B | $\color{green}{0.82}$ | $\color{green}{+0.9}$ |
| 1440×900 | 47 | 8.27 | $\color{green}{5.21}$ | $\color{green}{0.63}$ | 3480 B | 2746 B | $\color{green}{0.79}$ | $\color{green}{+0.8}$ |
| 1600×1000 | 47 | 8.86 | $\color{green}{5.55}$ | $\color{green}{0.64}$ | 3654 B | 1346 B | $\color{green}{0.37}$ | $\color{green}{+0.8}$ |
| 1920×1080 | 47 | 9.67 | $\color{green}{5.80}$ | $\color{green}{0.60}$ | 3993 B | 1692 B | $\color{green}{0.42}$ | $\color{green}{+0.7}$ |
| 2048×1536@2 | 45 | 18.3 | $\color{green}{10.4}$ | $\color{green}{0.53}$ | 15 kB | 5159 B | $\color{green}{0.34}$ | $\color{green}{+1.1}$ |
| 2560×1600@2 | 47 | 24.4 | $\color{green}{12.4}$ | $\color{green}{0.51}$ | 17 kB | 5514 B | $\color{green}{0.33}$ | $\color{green}{+1.0}$ |
| 2880×1800@2 | 47 | 27.1 | $\color{green}{13.4}$ | $\color{green}{0.48}$ | 17 kB | 5668 B | $\color{green}{0.33}$ | $\color{green}{+1.0}$ |
| 3200×2000@2 | 47 | 28.4 | $\color{green}{14.4}$ | $\color{green}{0.51}$ | 16 kB | 5259 B | $\color{green}{0.32}$ | $\color{green}{+0.9}$ |
| 3456×2168@2 | 47 | 31.2 | $\color{green}{15.0}$ | $\color{green}{0.48}$ | 17 kB | 5934 B | $\color{green}{0.34}$ | $\color{green}{+0.8}$ |
| 3840×2160@2 | 47 | 31.8 | $\color{green}{15.2}$ | $\color{green}{0.48}$ | 26 kB | 5605 B | $\color{green}{0.21}$ | $\color{green}{+0.7}$ |

### `video`, a frame of the video

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 118 | 18.8 | $\color{green}{15.9}$ | $\color{green}{0.80}$ | 36 kB | 39 kB | $\color{red}{1.06}$ | $\color{green}{+0.4}$ |
| 1440×900 | 118 | 20.7 | $\color{green}{18.6}$ | $\color{green}{0.85}$ | 43 kB | 46 kB | $\color{red}{1.06}$ | $\color{green}{+0.4}$ |
| 1600×1000 | 118 | 23.8 | $\color{green}{22.0}$ | $\color{green}{0.88}$ | 50 kB | 53 kB | $\color{red}{1.06}$ | $\color{green}{+0.5}$ |
| 1920×1080 | 118 | 27.7 | $\color{green}{25.1}$ | $\color{green}{0.85}$ | 55 kB | 58 kB | $\color{red}{1.06}$ | $\color{green}{+0.5}$ |
| 2048×1536@2 | 118 | 42.9 | $\color{green}{42.4}$ | $\color{green}{0.91}$ | 91 kB | 95 kB | $\color{red}{1.05}$ | $\color{green}{+0.6}$ |
| 2560×1600@2 | 118 | 50.0 | $\color{green}{45.9}$ | $\color{green}{0.87}$ | 97 kB | 101 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 2880×1800@2 | 118 | 63.1 | $\color{green}{55.4}$ | $\color{green}{0.84}$ | 114 kB | 118 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 3200×2000@2 | 118 | 76.1 | $\color{green}{65.2}$ | $\color{green}{0.84}$ | 132 kB | 137 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 3456×2168@2 | 118 | 86.1 | $\color{green}{75.9}$ | $\color{green}{0.84}$ | 146 kB | 153 kB | $\color{red}{1.04}$ | $\color{green}{+0.6}$ |
| 3840×2160@2 | 118 | 90.2 | $\color{green}{75.2}$ | $\color{green}{0.81}$ | 137 kB | 143 kB | $\color{red}{1.04}$ | $\color{green}{+0.7}$ |

### `walk`, a frame as the dial walks

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 246 | 15.4 | $\color{green}{10.7}$ | $\color{green}{0.71}$ | 14 kB | 16 kB | $\color{red}{1.12}$ | $\color{green}{+0.6}$ |
| 1440×900 | 254 | 17.4 | $\color{green}{13.7}$ | $\color{green}{0.78}$ | 20 kB | 23 kB | $\color{red}{1.12}$ | $\color{green}{+0.5}$ |
| 1600×1000 | 236 | 19.7 | $\color{green}{15.0}$ | $\color{green}{0.75}$ | 18 kB | 21 kB | $\color{red}{1.12}$ | $\color{green}{+0.5}$ |
| 1920×1080 | 232 | 22.9 | $\color{green}{17.4}$ | $\color{green}{0.73}$ | 20 kB | 23 kB | $\color{red}{1.12}$ | $\color{green}{+0.5}$ |
| 2048×1536@2 | 230 | 36.0 | $\color{green}{28.5}$ | $\color{green}{0.77}$ | 34 kB | 38 kB | $\color{red}{1.10}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 232 | 41.2 | $\color{green}{31.1}$ | $\color{green}{0.74}$ | 37 kB | 40 kB | $\color{red}{1.10}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 232 | 52.1 | $\color{green}{37.6}$ | $\color{green}{0.71}$ | 43 kB | 47 kB | $\color{red}{1.09}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 232 | 62.3 | $\color{green}{45.0}$ | $\color{green}{0.71}$ | 50 kB | 55 kB | $\color{red}{1.08}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 232 | 72.9 | $\color{green}{52.8}$ | $\color{green}{0.72}$ | 56 kB | 61 kB | $\color{red}{1.09}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 232 | 72.9 | $\color{green}{53.6}$ | $\color{green}{0.71}$ | 55 kB | 60 kB | $\color{red}{1.09}$ | $\color{green}{+0.4}$ |

### Keyframes inside `terminal`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 10.3 | $\color{green}{3.28}$ | $\color{green}{0.45}$ | 3460 B | 3294 B | $\color{green}{0.95}$ | $\color{green}{+0.5}$ |
| 1440×900 | 2 | 11.0 | $\color{green}{3.60}$ | $\color{green}{0.44}$ | 3490 B | 3155 B | $\color{green}{0.90}$ | $\color{green}{+0.3}$ |
| 1600×1000 | 2 | 13.5 | $\color{green}{4.95}$ | $\color{green}{0.49}$ | 3281 B | 2804 B | $\color{green}{0.85}$ | $\color{green}{+0.6}$ |
| 1920×1080 | 2 | 15.3 | $\color{green}{5.34}$ | $\color{green}{0.51}$ | 3716 B | 3063 B | $\color{green}{0.82}$ | $\color{green}{+0.1}$ |
| 2048×1536@2 | 2 | 19.5 | $\color{green}{6.74}$ | $\color{green}{0.51}$ | 8042 B | 7125 B | $\color{green}{0.89}$ | $\color{green}{+0.5}$ |
| 2560×1600@2 | 2 | 19.6 | $\color{green}{8.72}$ | $\color{green}{0.62}$ | 8494 B | 7412 B | $\color{green}{0.87}$ | $\color{green}{+0.5}$ |
| 2880×1800@2 | 2 | 23.1 | $\color{green}{10.5}$ | $\color{green}{0.59}$ | 8808 B | 7452 B | $\color{green}{0.85}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 2 | 46.0 | $\color{green}{22.1}$ | $\color{green}{0.77}$ | 7907 B | 6222 B | $\color{green}{0.79}$ | $\color{green}{+0.5}$ |
| 3456×2168@2 | 2 | 35.6 | $\color{green}{15.2}$ | $\color{green}{0.56}$ | 9751 B | 7634 B | $\color{green}{0.78}$ | $\color{green}{+0.6}$ |
| 3840×2160@2 | 2 | 35.4 | $\color{green}{18.5}$ | $\color{green}{0.52}$ | 10 kB | 7797 B | $\color{green}{0.77}$ | $\color{green}{+0.5}$ |

### Keyframes inside `uncover`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 26.1 | $\color{green}{11.1}$ | $\color{green}{0.42}$ | 105 kB | 105 kB | 1.00 | $\color{green}{+0.3}$ |
| 1440×900 | 1 | 30.1 | $\color{green}{12.8}$ | $\color{green}{0.43}$ | 120 kB | 117 kB | $\color{green}{0.98}$ | $\color{green}{+0.3}$ |
| 1600×1000 | 1 | 36.1 | $\color{green}{16.2}$ | $\color{green}{0.45}$ | 160 kB | 159 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |
| 1920×1080 | 1 | 40.6 | $\color{green}{21.8}$ | $\color{green}{0.54}$ | 204 kB | 202 kB | $\color{green}{0.99}$ | $\color{green}{+0.4}$ |
| 2048×1536@2 | 1 | 39.9 | $\color{green}{25.2}$ | $\color{green}{0.63}$ | 246 kB | 246 kB | 1.00 | $\color{green}{+0.3}$ |
| 2560×1600@2 | 1 | 48.1 | $\color{green}{27.2}$ | $\color{green}{0.56}$ | 261 kB | 260 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |
| 2880×1800@2 | 1 | 66.9 | $\color{green}{31.1}$ | $\color{green}{0.46}$ | 293 kB | 285 kB | $\color{green}{0.97}$ | $\color{green}{+0.3}$ |
| 3200×2000@2 | 1 | 71.1 | $\color{green}{42.5}$ | $\color{green}{0.60}$ | 385 kB | 381 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |
| 3456×2168@2 | 1 | 80.5 | $\color{green}{53.1}$ | $\color{green}{0.66}$ | 456 kB | 452 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |
| 3840×2160@2 | 1 | 83.8 | $\color{green}{55.2}$ | $\color{green}{0.66}$ | 483 kB | 479 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |

### Keyframes inside `drag`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 2 | 22.5 | $\color{green}{9.39}$ | $\color{green}{0.48}$ | 87 kB | 87 kB | 1.00 | $\color{green}{+0.4}$ |
| 1440×900 | 2 | 24.4 | $\color{green}{11.2}$ | $\color{green}{0.54}$ | 87 kB | 87 kB | 1.00 | $\color{green}{+0.4}$ |
| 1600×1000 | 2 | 24.3 | $\color{green}{11.3}$ | $\color{green}{0.54}$ | 87 kB | 87 kB | 1.00 | $\color{green}{+0.4}$ |
| 1920×1080 | 2 | 26.5 | $\color{green}{11.4}$ | $\color{green}{0.50}$ | 87 kB | 87 kB | 1.00 | $\color{green}{+0.4}$ |
| 2048×1536@2 | 2 | 36.3 | $\color{green}{21.8}$ | $\color{green}{0.69}$ | 207 kB | 207 kB | 1.00 | $\color{green}{+0.4}$ |
| 2560×1600@2 | 2 | 42.1 | $\color{green}{24.9}$ | $\color{green}{0.61}$ | 208 kB | 208 kB | 1.00 | $\color{green}{+0.3}$ |
| 2880×1800@2 | 2 | 67.5 | $\color{green}{32.6}$ | $\color{green}{0.64}$ | 208 kB | 208 kB | 1.00 | $\color{green}{+0.4}$ |
| 3200×2000@2 | 2 | 66.0 | $\color{green}{37.6}$ | $\color{green}{0.57}$ | 209 kB | 208 kB | 1.00 | $\color{green}{+0.3}$ |
| 3456×2168@2 | 2 | 83.8 | $\color{green}{59.5}$ | $\color{green}{0.71}$ | 209 kB | 208 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |
| 3840×2160@2 | 2 | 87.5 | $\color{green}{34.2}$ | $\color{green}{0.40}$ | 209 kB | 208 kB | $\color{green}{0.99}$ | $\color{green}{+0.3}$ |

### Keyframes inside `flood`

| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1280×800 | 1 | 101.2 | $\color{green}{53.6}$ | $\color{green}{0.53}$ | 775 kB | 779 kB | $\color{red}{1.01}$ | $\color{green}{+0.5}$ |
| 1440×900 | 1 | 90.8 | $\color{green}{62.9}$ | $\color{green}{0.69}$ | 1000 kB | 1007 kB | $\color{red}{1.01}$ | $\color{green}{+0.5}$ |
| 1600×1000 | 1 | 106.5 | $\color{green}{75.6}$ | $\color{green}{0.71}$ | 1250 kB | 1255 kB | 1.00 | $\color{green}{+0.5}$ |
| 1920×1080 | 1 | 135.4 | $\color{green}{106.5}$ | $\color{green}{0.79}$ | 1645 kB | 1656 kB | $\color{red}{1.01}$ | $\color{green}{+0.5}$ |
| 2048×1536@2 | 1 | 138.6 | $\color{green}{119.7}$ | $\color{green}{0.86}$ | 1433 kB | 1404 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 2560×1600@2 | 1 | 156.6 | $\color{green}{126.2}$ | $\color{green}{0.81}$ | 1786 kB | 1752 kB | $\color{green}{0.98}$ | $\color{green}{+0.3}$ |
| 2880×1800@2 | 1 | 187.4 | $\color{green}{173.1}$ | $\color{green}{0.92}$ | 2330 kB | 2279 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 3200×2000@2 | 1 | 246.0 | $\color{red}{270.8}$ | $\color{red}{1.10}$ | 2854 kB | 2797 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |
| 3456×2168@2 | 1 | 272.7 | $\color{green}{257.8}$ | $\color{green}{0.94}$ | 3559 kB | 3459 kB | $\color{green}{0.97}$ | $\color{green}{+0.4}$ |
| 3840×2160@2 | 1 | 284.5 | $\color{red}{292.8}$ | $\color{red}{1.03}$ | 3929 kB | 3853 kB | $\color{green}{0.98}$ | $\color{green}{+0.4}$ |

## Against libvpx, keyframes of each capture's first picture, a paused video's last

### A keyframe at dial 40: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.59}$ | $\color{green}{0.54}$ | $\color{green}{0.59}$ | $\color{green}{0.54}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.70}$ |
| `still` | $\color{green}{0.55}$ | $\color{green}{0.67}$ | $\color{green}{0.73}$ | $\color{green}{0.66}$ | $\color{green}{0.73}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.73}$ | $\color{green}{0.80}$ | $\color{green}{0.77}$ |
| `uncover` | $\color{green}{0.62}$ | $\color{green}{0.58}$ | $\color{green}{0.63}$ | $\color{green}{0.71}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ | $\color{green}{0.68}$ | $\color{green}{0.73}$ | $\color{green}{0.72}$ | $\color{green}{0.74}$ |
| `flood` | $\color{green}{0.69}$ | $\color{green}{0.83}$ | $\color{green}{0.81}$ | $\color{green}{0.89}$ | $\color{green}{0.98}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 |
| `browser` | $\color{green}{0.55}$ | $\color{green}{0.67}$ | $\color{green}{0.68}$ | $\color{green}{0.69}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ | $\color{green}{0.69}$ | $\color{green}{0.76}$ | $\color{green}{0.79}$ |
| `drag` | $\color{green}{0.55}$ | $\color{green}{0.61}$ | $\color{green}{0.59}$ | $\color{green}{0.59}$ | $\color{green}{0.81}$ | $\color{green}{0.73}$ | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.69}$ | $\color{green}{0.71}$ |
| `video` | $\color{green}{0.78}$ | $\color{green}{0.88}$ | $\color{green}{0.89}$ | $\color{green}{0.84}$ | $\color{green}{0.81}$ | $\color{green}{0.79}$ | $\color{green}{0.72}$ | $\color{green}{0.75}$ | $\color{green}{0.72}$ | $\color{green}{0.73}$ |
| `walk` | $\color{green}{0.78}$ | $\color{green}{0.84}$ | $\color{green}{0.79}$ | $\color{green}{0.80}$ | $\color{green}{0.77}$ | $\color{green}{0.75}$ | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.70}$ | $\color{green}{0.74}$ |

### A keyframe at dial 40: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.87}$ | $\color{green}{0.84}$ | $\color{green}{0.77}$ | $\color{green}{0.72}$ | $\color{green}{0.80}$ | $\color{green}{0.79}$ | $\color{green}{0.76}$ | $\color{green}{0.69}$ | $\color{green}{0.64}$ | $\color{green}{0.67}$ |
| `still` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.02}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | 1.00 |
| `uncover` | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.98}$ | 1.00 | 1.00 | 1.00 |
| `flood` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `browser` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | 1.00 | $\color{red}{1.02}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | 1.00 |
| `drag` | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | 1.00 |
| `video` | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.92}$ | $\color{green}{0.91}$ | $\color{green}{0.89}$ | $\color{green}{0.89}$ |
| `walk` | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.91}$ | $\color{green}{0.90}$ | $\color{green}{0.89}$ | $\color{green}{0.88}$ | $\color{green}{0.86}$ | $\color{green}{0.86}$ |

### A keyframe at dial 40: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{red}{-3.4}$ | $\color{red}{-4.2}$ | $\color{red}{-5.6}$ | $\color{red}{-6.0}$ | $\color{red}{-4.3}$ | $\color{red}{-4.3}$ | $\color{red}{-3.5}$ | $\color{red}{-2.4}$ | $\color{red}{-6.4}$ | $\color{red}{-7.7}$ |
| `still` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `flood` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `browser` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `drag` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ |
| `video` | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | 0.0 | 0.0 | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ |
| `walk` | $\color{green}{+0.1}$ | 0.0 | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.3}$ | $\color{red}{-0.3}$ | $\color{red}{-0.3}$ |

### A keyframe at dial 65: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.40}$ | $\color{green}{0.48}$ | $\color{green}{0.50}$ | $\color{green}{0.47}$ | $\color{green}{0.66}$ | $\color{green}{0.61}$ | $\color{green}{0.61}$ | $\color{green}{0.56}$ | $\color{green}{0.61}$ | $\color{green}{0.59}$ |
| `still` | $\color{green}{0.50}$ | $\color{green}{0.62}$ | $\color{green}{0.65}$ | $\color{green}{0.62}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.71}$ | $\color{green}{0.68}$ | $\color{green}{0.71}$ | $\color{green}{0.76}$ |
| `uncover` | $\color{green}{0.52}$ | $\color{green}{0.51}$ | $\color{green}{0.56}$ | $\color{green}{0.63}$ | $\color{green}{0.71}$ | $\color{green}{0.68}$ | $\color{green}{0.65}$ | $\color{green}{0.69}$ | $\color{green}{0.67}$ | $\color{green}{0.71}$ |
| `flood` | $\color{green}{0.58}$ | $\color{green}{0.72}$ | $\color{green}{0.77}$ | $\color{green}{0.81}$ | $\color{red}{1.03}$ | $\color{green}{0.97}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{red}{1.03}$ | $\color{red}{1.01}$ |
| `browser` | $\color{green}{0.50}$ | $\color{green}{0.65}$ | $\color{green}{0.65}$ | $\color{green}{0.55}$ | $\color{green}{0.73}$ | $\color{green}{0.70}$ | $\color{green}{0.69}$ | $\color{green}{0.67}$ | $\color{green}{0.75}$ | $\color{green}{0.73}$ |
| `drag` | $\color{green}{0.52}$ | $\color{green}{0.54}$ | $\color{green}{0.53}$ | $\color{green}{0.52}$ | $\color{green}{0.69}$ | $\color{green}{0.67}$ | $\color{green}{0.65}$ | $\color{green}{0.63}$ | $\color{green}{0.64}$ | $\color{green}{0.61}$ |
| `video` | $\color{green}{0.79}$ | $\color{green}{0.88}$ | $\color{green}{0.84}$ | $\color{green}{0.93}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.88}$ | $\color{green}{0.87}$ | $\color{green}{0.84}$ | $\color{green}{0.83}$ |
| `walk` | $\color{green}{0.79}$ | $\color{green}{0.89}$ | $\color{green}{0.90}$ | $\color{green}{0.91}$ | $\color{green}{0.92}$ | $\color{green}{0.93}$ | $\color{green}{0.86}$ | $\color{green}{0.82}$ | $\color{green}{0.80}$ | $\color{green}{0.76}$ |

### A keyframe at dial 65: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.94}$ | $\color{green}{0.88}$ | $\color{green}{0.83}$ | $\color{green}{0.80}$ | $\color{green}{0.87}$ | $\color{green}{0.85}$ | $\color{green}{0.82}$ | $\color{green}{0.75}$ | $\color{green}{0.74}$ | $\color{green}{0.74}$ |
| `still` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | 1.00 |
| `uncover` | $\color{red}{1.02}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.01}$ | 1.00 | $\color{green}{0.98}$ | 1.00 | 1.00 | 1.00 |
| `flood` | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 |
| `browser` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | $\color{red}{1.02}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | 1.00 | 1.00 |
| `drag` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | 1.00 | 1.00 | 1.00 | 1.00 |
| `video` | $\color{red}{1.06}$ | $\color{red}{1.06}$ | $\color{red}{1.05}$ | $\color{red}{1.05}$ | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | 1.00 | 1.00 |
| `walk` | $\color{red}{1.04}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | 1.00 | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ |

### A keyframe at dial 65: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ |
| `still` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `uncover` | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `flood` | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `browser` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `drag` | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |

### A keyframe at dial 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.46}$ | $\color{green}{0.47}$ | $\color{green}{0.52}$ | $\color{green}{0.49}$ | $\color{green}{0.62}$ | $\color{green}{0.62}$ | $\color{green}{0.61}$ | $\color{green}{0.56}$ | $\color{green}{0.58}$ | $\color{green}{0.59}$ |
| `still` | $\color{green}{0.48}$ | $\color{green}{0.62}$ | $\color{green}{0.64}$ | $\color{green}{0.59}$ | $\color{green}{0.72}$ | $\color{green}{0.71}$ | $\color{green}{0.69}$ | $\color{green}{0.68}$ | $\color{green}{0.71}$ | $\color{green}{0.69}$ |
| `uncover` | $\color{green}{0.48}$ | $\color{green}{0.48}$ | $\color{green}{0.54}$ | $\color{green}{0.60}$ | $\color{green}{0.71}$ | $\color{green}{0.65}$ | $\color{green}{0.60}$ | $\color{green}{0.65}$ | $\color{green}{0.66}$ | $\color{green}{0.70}$ |
| `flood` | $\color{green}{0.53}$ | $\color{green}{0.75}$ | $\color{green}{0.74}$ | $\color{green}{0.76}$ | $\color{green}{0.93}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | 1.00 | $\color{green}{0.98}$ |
| `browser` | $\color{green}{0.48}$ | $\color{green}{0.59}$ | $\color{green}{0.60}$ | $\color{green}{0.60}$ | $\color{green}{0.70}$ | $\color{green}{0.67}$ | $\color{green}{0.68}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ |
| `drag` | $\color{green}{0.47}$ | $\color{green}{0.52}$ | $\color{green}{0.50}$ | $\color{green}{0.50}$ | $\color{green}{0.67}$ | $\color{green}{0.63}$ | $\color{green}{0.63}$ | $\color{green}{0.62}$ | $\color{green}{0.60}$ | $\color{green}{0.59}$ |
| `video` | $\color{green}{0.73}$ | $\color{green}{0.84}$ | $\color{green}{0.89}$ | $\color{green}{0.86}$ | $\color{red}{1.01}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.99}$ | $\color{green}{0.92}$ | $\color{green}{0.95}$ |
| `walk` | $\color{green}{0.74}$ | $\color{green}{0.93}$ | $\color{green}{0.89}$ | $\color{green}{0.89}$ | $\color{green}{0.95}$ | $\color{green}{0.93}$ | $\color{green}{0.92}$ | $\color{green}{0.92}$ | $\color{green}{0.90}$ | $\color{green}{0.87}$ |

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
| `video` | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ |
| `walk` | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | 1.00 | $\color{red}{1.01}$ | 1.00 | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ |

### A keyframe at dial 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.6}$ | $\color{green}{+0.3}$ | $\color{green}{+0.6}$ | $\color{green}{+0.1}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.5}$ |
| `still` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `browser` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `drag` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |

## Against libvpx, settles of each capture's first picture, a paused video's last

### A settle from dial 20 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.32}$ | $\color{green}{0.40}$ | $\color{green}{0.40}$ | $\color{green}{0.41}$ | $\color{green}{0.54}$ | $\color{green}{0.54}$ | $\color{green}{0.51}$ | $\color{green}{0.56}$ | $\color{green}{0.53}$ | $\color{green}{0.51}$ |
| `still` | $\color{green}{0.52}$ | $\color{green}{0.66}$ | $\color{green}{0.67}$ | $\color{green}{0.62}$ | $\color{green}{0.73}$ | $\color{green}{0.73}$ | $\color{green}{0.71}$ | $\color{green}{0.74}$ | $\color{green}{0.77}$ | $\color{green}{0.74}$ |
| `uncover` | $\color{green}{0.48}$ | $\color{green}{0.52}$ | $\color{green}{0.61}$ | $\color{green}{0.63}$ | $\color{green}{0.73}$ | $\color{green}{0.65}$ | $\color{green}{0.64}$ | $\color{green}{0.66}$ | $\color{green}{0.68}$ | $\color{green}{0.73}$ |
| `flood` | $\color{green}{0.46}$ | $\color{green}{0.71}$ | $\color{green}{0.55}$ | $\color{green}{0.74}$ | $\color{green}{0.96}$ | $\color{green}{0.94}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ |
| `browser` | $\color{green}{0.52}$ | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.60}$ | $\color{green}{0.75}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ | $\color{green}{0.78}$ | $\color{green}{0.73}$ |
| `drag` | $\color{green}{0.48}$ | $\color{green}{0.48}$ | $\color{green}{0.46}$ | $\color{green}{0.47}$ | $\color{green}{0.69}$ | $\color{green}{0.62}$ | $\color{green}{0.60}$ | $\color{green}{0.57}$ | $\color{green}{0.56}$ | $\color{green}{0.53}$ |
| `video` | $\color{green}{0.67}$ | $\color{green}{0.91}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{red}{1.06}$ | $\color{red}{1.04}$ | 1.00 | $\color{red}{1.02}$ | $\color{red}{1.06}$ | $\color{red}{1.04}$ |
| `walk` | $\color{green}{0.70}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{red}{1.14}$ | $\color{red}{1.05}$ | $\color{red}{1.05}$ | $\color{red}{1.04}$ | $\color{red}{1.06}$ | $\color{red}{1.04}$ |

### A settle from dial 20 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{red}{1.05}$ | 1.00 | $\color{green}{0.95}$ | $\color{red}{1.01}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{red}{1.01}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.98}$ |
| `still` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.97}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.99}$ |
| `uncover` | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ | $\color{green}{0.96}$ |
| `flood` | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{red}{1.01}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ |
| `browser` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.97}$ | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.99}$ |
| `drag` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | 1.00 | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 |
| `video` | $\color{green}{0.95}$ | $\color{green}{0.94}$ | $\color{green}{0.93}$ | $\color{green}{0.93}$ | $\color{green}{0.85}$ | $\color{green}{0.84}$ | $\color{green}{0.83}$ | $\color{green}{0.82}$ | $\color{green}{0.81}$ | $\color{green}{0.80}$ |
| `walk` | $\color{green}{0.93}$ | $\color{green}{0.92}$ | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.82}$ | $\color{green}{0.83}$ | $\color{green}{0.82}$ | $\color{green}{0.81}$ | $\color{green}{0.79}$ | $\color{green}{0.80}$ |

### A settle from dial 20 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ | $\color{green}{+1.0}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.4}$ |
| `still` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `uncover` | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `flood` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `browser` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `drag` | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{red}{-0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ |
| `walk` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.6}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ |

### A settle from dial 65 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.47}$ | $\color{green}{0.43}$ | $\color{green}{0.45}$ | $\color{green}{0.44}$ | $\color{green}{0.62}$ | $\color{green}{0.61}$ | $\color{green}{0.57}$ | $\color{green}{0.56}$ | $\color{green}{0.59}$ | $\color{green}{0.57}$ |
| `still` | $\color{green}{0.99}$ | $\color{red}{1.14}$ | $\color{red}{1.18}$ | $\color{red}{1.14}$ | $\color{red}{1.38}$ | $\color{red}{1.28}$ | $\color{red}{1.21}$ | $\color{red}{1.19}$ | $\color{red}{1.32}$ | $\color{red}{1.35}$ |
| `uncover` | $\color{green}{0.97}$ | $\color{red}{1.05}$ | $\color{red}{1.05}$ | $\color{red}{1.16}$ | $\color{red}{1.32}$ | $\color{red}{1.22}$ | $\color{red}{1.17}$ | $\color{red}{1.18}$ | $\color{red}{1.30}$ | $\color{red}{1.35}$ |
| `flood` | $\color{green}{0.79}$ | $\color{green}{0.99}$ | $\color{red}{1.07}$ | $\color{red}{1.15}$ | $\color{red}{1.54}$ | $\color{red}{1.60}$ | $\color{red}{1.58}$ | $\color{red}{1.62}$ | $\color{red}{1.55}$ | $\color{red}{1.59}$ |
| `browser` | $\color{green}{0.99}$ | $\color{red}{1.11}$ | $\color{red}{1.30}$ | $\color{red}{1.16}$ | $\color{red}{1.33}$ | $\color{red}{1.33}$ | $\color{red}{1.21}$ | $\color{red}{1.24}$ | $\color{red}{1.34}$ | $\color{red}{1.27}$ |
| `drag` | $\color{green}{0.91}$ | $\color{green}{0.91}$ | $\color{green}{0.82}$ | $\color{green}{0.80}$ | $\color{red}{1.20}$ | $\color{red}{1.07}$ | $\color{red}{1.02}$ | $\color{green}{0.95}$ | $\color{green}{0.93}$ | $\color{green}{0.88}$ |
| `video` | $\color{red}{1.05}$ | $\color{red}{1.33}$ | $\color{red}{1.33}$ | $\color{red}{1.35}$ | $\color{red}{1.56}$ | $\color{red}{1.43}$ | $\color{red}{1.39}$ | $\color{red}{1.35}$ | $\color{red}{1.41}$ | $\color{red}{1.31}$ |
| `walk` | $\color{red}{1.05}$ | $\color{red}{1.21}$ | $\color{red}{1.26}$ | $\color{red}{1.26}$ | $\color{red}{1.55}$ | $\color{red}{1.48}$ | $\color{red}{1.39}$ | $\color{red}{1.40}$ | $\color{red}{1.46}$ | $\color{red}{1.36}$ |

### A settle from dial 65 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.94}$ | $\color{red}{1.03}$ | $\color{green}{0.94}$ | $\color{red}{1.02}$ | 1.00 | $\color{red}{1.02}$ | 1.00 | $\color{green}{0.97}$ |
| `still` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ |
| `uncover` | $\color{green}{0.94}$ | $\color{green}{0.93}$ | $\color{green}{0.94}$ | $\color{green}{0.95}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ |
| `flood` | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{green}{0.98}$ | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.03}$ |
| `browser` | $\color{green}{0.99}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ |
| `drag` | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.97}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.94}$ | $\color{green}{0.93}$ |
| `video` | 1.00 | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.99}$ | $\color{green}{0.99}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.95}$ | $\color{green}{0.96}$ | $\color{green}{0.95}$ |
| `walk` | 1.00 | $\color{green}{0.98}$ | $\color{green}{0.99}$ | 1.00 | $\color{green}{0.97}$ | $\color{green}{0.98}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ | $\color{green}{0.97}$ | $\color{green}{0.96}$ |

### A settle from dial 65 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+1.6}$ | $\color{green}{+1.8}$ | $\color{green}{+0.7}$ | $\color{green}{+2.1}$ | $\color{green}{+0.7}$ | $\color{green}{+1.6}$ | $\color{green}{+0.9}$ | $\color{green}{+1.6}$ | $\color{green}{+1.5}$ | $\color{green}{+0.9}$ |
| `still` | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `uncover` | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.2}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ |
| `flood` | $\color{green}{+0.9}$ | $\color{green}{+0.9}$ | $\color{green}{+0.8}$ | $\color{green}{+0.9}$ | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ | $\color{green}{+1.5}$ | $\color{green}{+1.5}$ | $\color{green}{+1.4}$ | $\color{green}{+1.5}$ |
| `browser` | $\color{green}{+1.1}$ | $\color{green}{+1.1}$ | $\color{green}{+1.2}$ | $\color{green}{+1.1}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.4}$ | $\color{green}{+1.4}$ |
| `drag` | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.8}$ | $\color{green}{+0.9}$ | $\color{green}{+1.4}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ | $\color{green}{+1.3}$ |
| `video` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `walk` | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |

### A settle from dial 80 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.40}$ | $\color{green}{0.39}$ | $\color{green}{0.43}$ | $\color{green}{0.39}$ | $\color{green}{0.52}$ | $\color{green}{0.54}$ | $\color{green}{0.51}$ | $\color{green}{0.53}$ | $\color{green}{0.53}$ | $\color{green}{0.53}$ |
| `still` | $\color{green}{0.88}$ | $\color{green}{0.93}$ | $\color{red}{1.03}$ | $\color{green}{0.97}$ | $\color{red}{1.05}$ | $\color{red}{1.07}$ | $\color{red}{1.04}$ | 1.00 | $\color{red}{1.06}$ | $\color{red}{1.05}$ |
| `uncover` | $\color{green}{0.84}$ | $\color{green}{0.86}$ | $\color{green}{0.88}$ | $\color{red}{1.01}$ | $\color{red}{1.11}$ | $\color{red}{1.04}$ | $\color{red}{1.02}$ | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.06}$ |
| `flood` | $\color{green}{0.98}$ | $\color{red}{1.26}$ | $\color{red}{1.33}$ | $\color{red}{1.36}$ | $\color{red}{1.79}$ | $\color{red}{1.79}$ | $\color{red}{1.74}$ | $\color{red}{1.75}$ | $\color{red}{1.75}$ | $\color{red}{1.89}$ |
| `browser` | $\color{green}{0.87}$ | $\color{green}{0.91}$ | 1.00 | $\color{green}{0.98}$ | $\color{red}{1.05}$ | $\color{red}{1.08}$ | $\color{red}{1.02}$ | 1.00 | $\color{red}{1.10}$ | $\color{red}{1.04}$ |
| `drag` | $\color{green}{0.82}$ | $\color{green}{0.76}$ | $\color{green}{0.72}$ | $\color{green}{0.66}$ | $\color{green}{0.95}$ | $\color{green}{0.84}$ | $\color{green}{0.82}$ | $\color{green}{0.79}$ | $\color{green}{0.74}$ | $\color{green}{0.72}$ |
| `video` | 1.00 | $\color{red}{1.18}$ | $\color{red}{1.26}$ | $\color{red}{1.13}$ | $\color{red}{1.50}$ | $\color{red}{1.34}$ | $\color{red}{1.31}$ | $\color{red}{1.31}$ | $\color{red}{1.40}$ | $\color{red}{1.31}$ |
| `walk` | 1.00 | $\color{red}{1.18}$ | $\color{red}{1.26}$ | $\color{red}{1.22}$ | $\color{red}{1.47}$ | $\color{red}{1.39}$ | $\color{red}{1.41}$ | $\color{red}{1.39}$ | $\color{red}{1.50}$ | $\color{red}{1.40}$ |

### A settle from dial 80 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.84}$ | $\color{green}{0.93}$ | $\color{green}{0.74}$ | $\color{red}{1.18}$ | $\color{green}{0.58}$ | $\color{green}{0.86}$ | $\color{green}{0.82}$ | $\color{red}{1.03}$ | $\color{green}{0.97}$ | $\color{green}{0.80}$ |
| `still` | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.66}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ | $\color{green}{0.77}$ | $\color{green}{0.80}$ | $\color{green}{0.73}$ | $\color{green}{0.75}$ | $\color{green}{0.73}$ |
| `uncover` | $\color{green}{0.75}$ | $\color{green}{0.74}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ | $\color{green}{0.75}$ | $\color{green}{0.77}$ | $\color{green}{0.78}$ | $\color{green}{0.76}$ | $\color{green}{0.73}$ | $\color{green}{0.74}$ |
| `flood` | $\color{red}{1.03}$ | $\color{red}{1.02}$ | $\color{red}{1.03}$ | $\color{red}{1.03}$ | $\color{red}{1.11}$ | $\color{red}{1.13}$ | $\color{red}{1.10}$ | $\color{red}{1.09}$ | $\color{red}{1.11}$ | $\color{red}{1.12}$ |
| `browser` | $\color{green}{0.71}$ | $\color{green}{0.72}$ | $\color{green}{0.66}$ | $\color{green}{0.69}$ | $\color{green}{0.72}$ | $\color{green}{0.77}$ | $\color{green}{0.80}$ | $\color{green}{0.73}$ | $\color{green}{0.75}$ | $\color{green}{0.73}$ |
| `drag` | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.84}$ | $\color{green}{0.85}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.70}$ | $\color{green}{0.71}$ |
| `video` | $\color{green}{0.85}$ | $\color{green}{0.83}$ | $\color{green}{0.82}$ | $\color{green}{0.80}$ | $\color{green}{0.80}$ | $\color{green}{0.80}$ | $\color{green}{0.81}$ | $\color{green}{0.83}$ | $\color{green}{0.86}$ | $\color{green}{0.85}$ |
| `walk` | $\color{green}{0.86}$ | $\color{green}{0.86}$ | $\color{green}{0.85}$ | $\color{green}{0.86}$ | $\color{green}{0.87}$ | $\color{green}{0.87}$ | $\color{green}{0.89}$ | $\color{green}{0.91}$ | $\color{green}{0.94}$ | $\color{green}{0.91}$ |

### A settle from dial 80 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | 0.0 | $\color{green}{+0.6}$ | $\color{red}{-0.4}$ | $\color{green}{+0.2}$ | 0.0 | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{red}{-0.1}$ |
| `still` | 0.0 | 0.0 | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `uncover` | $\color{red}{-0.2}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{red}{-0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `flood` | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.7}$ | $\color{green}{+0.8}$ | $\color{green}{+0.7}$ |
| `browser` | 0.0 | 0.0 | 0.0 | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ |
| `drag` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ | 0.0 | $\color{green}{+0.1}$ | $\color{green}{+0.1}$ |
| `video` | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `walk` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ |

### A settle from dial 89 to 90: time

This encoder's least time over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{0.38}$ | $\color{green}{0.38}$ | $\color{green}{0.34}$ | $\color{green}{0.37}$ | $\color{green}{0.51}$ | $\color{green}{0.52}$ | $\color{green}{0.49}$ | $\color{green}{0.48}$ | $\color{green}{0.53}$ | $\color{green}{0.52}$ |
| `still` | $\color{green}{0.69}$ | $\color{green}{0.78}$ | $\color{green}{0.82}$ | $\color{green}{0.84}$ | $\color{green}{0.87}$ | $\color{green}{0.87}$ | $\color{green}{0.84}$ | $\color{green}{0.85}$ | $\color{green}{0.87}$ | $\color{green}{0.89}$ |
| `uncover` | $\color{green}{0.90}$ | $\color{green}{0.88}$ | $\color{green}{0.82}$ | $\color{green}{0.89}$ | $\color{green}{0.92}$ | $\color{green}{0.82}$ | $\color{green}{0.83}$ | $\color{green}{0.84}$ | $\color{green}{0.87}$ | $\color{green}{0.86}$ |
| `flood` | $\color{red}{1.02}$ | $\color{red}{1.38}$ | $\color{red}{1.32}$ | $\color{red}{1.32}$ | $\color{red}{1.58}$ | $\color{red}{1.59}$ | $\color{red}{1.54}$ | $\color{red}{1.66}$ | $\color{red}{1.68}$ | $\color{red}{1.63}$ |
| `browser` | $\color{green}{0.72}$ | $\color{green}{0.77}$ | $\color{green}{0.83}$ | $\color{green}{0.84}$ | $\color{green}{0.91}$ | $\color{green}{0.92}$ | $\color{green}{0.84}$ | $\color{green}{0.83}$ | $\color{green}{0.88}$ | $\color{green}{0.89}$ |
| `drag` | $\color{green}{0.76}$ | $\color{green}{0.69}$ | $\color{green}{0.68}$ | $\color{green}{0.64}$ | $\color{green}{0.85}$ | $\color{green}{0.74}$ | $\color{green}{0.73}$ | $\color{green}{0.69}$ | $\color{green}{0.69}$ | $\color{green}{0.63}$ |
| `video` | $\color{green}{0.90}$ | 1.00 | $\color{red}{1.11}$ | $\color{red}{1.01}$ | $\color{red}{1.47}$ | $\color{red}{1.26}$ | $\color{red}{1.32}$ | $\color{red}{1.32}$ | $\color{red}{1.39}$ | $\color{red}{1.27}$ |
| `walk` | $\color{green}{0.88}$ | $\color{red}{1.08}$ | $\color{red}{1.18}$ | $\color{red}{1.04}$ | $\color{red}{1.41}$ | $\color{red}{1.34}$ | $\color{red}{1.33}$ | $\color{red}{1.39}$ | $\color{red}{1.43}$ | $\color{red}{1.36}$ |

### A settle from dial 89 to 90: bytes

This encoder's bytes over libvpx's.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{red}{1.04}$ | $\color{red}{1.77}$ | $\color{red}{1.26}$ | $\color{red}{1.82}$ | $\color{green}{0.69}$ | $\color{green}{0.92}$ | $\color{red}{1.15}$ | $\color{red}{1.05}$ | $\color{red}{1.14}$ | $\color{green}{0.83}$ |
| `still` | $\color{green}{0.70}$ | $\color{green}{0.72}$ | $\color{green}{0.76}$ | $\color{green}{0.88}$ | $\color{green}{0.69}$ | 1.00 | $\color{green}{0.94}$ | $\color{green}{0.79}$ | $\color{green}{0.89}$ | $\color{green}{0.84}$ |
| `uncover` | $\color{red}{1.75}$ | $\color{red}{1.59}$ | $\color{red}{1.19}$ | $\color{red}{1.16}$ | $\color{green}{0.47}$ | $\color{green}{0.55}$ | $\color{green}{0.59}$ | $\color{green}{0.61}$ | $\color{green}{0.60}$ | $\color{green}{0.65}$ |
| `flood` | $\color{green}{0.85}$ | $\color{green}{0.88}$ | $\color{green}{0.83}$ | $\color{green}{0.89}$ | $\color{green}{0.87}$ | $\color{green}{0.89}$ | $\color{green}{0.84}$ | $\color{green}{0.85}$ | $\color{green}{0.87}$ | $\color{green}{0.89}$ |
| `browser` | $\color{green}{0.70}$ | $\color{green}{0.72}$ | $\color{green}{0.84}$ | $\color{green}{0.88}$ | $\color{green}{0.69}$ | 1.00 | $\color{green}{0.94}$ | $\color{green}{0.79}$ | $\color{green}{0.89}$ | $\color{green}{0.84}$ |
| `drag` | $\color{green}{0.67}$ | $\color{green}{0.69}$ | $\color{green}{0.68}$ | $\color{green}{0.68}$ | $\color{green}{0.49}$ | $\color{green}{0.47}$ | $\color{green}{0.51}$ | $\color{green}{0.51}$ | $\color{green}{0.51}$ | $\color{green}{0.47}$ |
| `video` | $\color{green}{0.76}$ | $\color{green}{0.77}$ | $\color{green}{0.87}$ | $\color{green}{0.89}$ | $\color{red}{1.06}$ | $\color{red}{1.13}$ | $\color{red}{1.22}$ | $\color{red}{1.31}$ | $\color{red}{1.26}$ | $\color{red}{1.42}$ |
| `walk` | $\color{green}{0.82}$ | $\color{green}{0.84}$ | $\color{green}{0.96}$ | $\color{green}{0.99}$ | $\color{red}{1.22}$ | $\color{red}{1.22}$ | $\color{red}{1.34}$ | $\color{red}{1.42}$ | $\color{red}{1.35}$ | $\color{red}{1.52}$ |

### A settle from dial 89 to 90: luma PSNR

This encoder's less libvpx's, in dB.

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3456×2168@2 | 3840×2160@2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `terminal` | $\color{green}{+0.5}$ | $\color{green}{+0.6}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.2}$ |
| `still` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `uncover` | $\color{green}{+0.3}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ |
| `flood` | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `browser` | $\color{green}{+0.3}$ | $\color{green}{+0.3}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |
| `drag` | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.2}$ | $\color{green}{+0.3}$ | $\color{green}{+0.2}$ |
| `video` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ |
| `walk` | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.5}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ | $\color{green}{+0.4}$ |

## Test material

The captures are what a wlshare session handed `Stream`, written by its own
`--capture-frames` flag in the capture container, lossless, with the
rectangles each frame came with, `zstd` compressed: `<scenario>-<WxH>[@2].wlcap.zst`,
`@2` a desktop drawn at scale 2, as the README.md beside them says. Where
a machine keeps them is in its `CLAUDE.local.md`, not here; the harness
reads `tmp/captures/` or the directory `CAPTURES` names, and `ONLY` keeps
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
| `walk` | busy | the video while the client reads late from 4 s to 10 s, so the dial walks down to 20 and back up to 89, then stopped, which leaves the screen black, the desktop settling on that |

Sizes: the ten `capture-frames.sh` runs by default, which vp9-wasm's
benchmark has samples of: 1280×800, 1440×900, 1600×1000 and 1920×1080 at
scale 1, and the same four desktops at scale 2, 2560×1600, 2880×1800,
3200×2000 and 3840×2160, with 2048×1536 and 3456×2168. The set is the
`frame-captures` of 2026-10-13 in the artifacts' backup
(`/Volumes/dasdata/backup/vp9-artifacts/frame-captures/`, 80 files), its
`uncover` trimmed on 2026-10-14 to begin where the terminal first moves.
`video` and `walk` from 2560×1600 up are made from the 1920×1080 session
frame for frame with the clip scaled to the size, since the capture
container does not keep up there. Each is measured at quality 90, the dial
the captures were made at, on four threads.

The tables are of those captures cut to 120 frames each, under the same
names, but `walk`, kept whole, read with `CAPTURES` naming their directory
(`frame-samples/` beside the captures, 11.5 GB against their 22). wlshare's
`scripts/capture-samples.sh` cuts them (`vp9cap trim --frames`) from each
capture's longest run of a size as vp9-wasm's samples are cut: a quiet
scenario's first 120 frames, a busy one's the 120 in a row whose frames
said the most pixels changed, the earliest where several tie; `still` (2
frames) and `uncover` (48 or 49) are shorter and kept whole, and `walk`
(233 to 256) is kept whole since its dial walks down, back up and settles
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
