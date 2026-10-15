# The plan of the encoder, and what was decided

What the encoder is for, what is decided about it, what was measured again
once its scope was final, and what is taken up next and in which order.
What the encoder is now is in [encoder.md](encoder.md), and the tables it
is measured by in [benchmarks.md](benchmarks.md). The phase-by-phase record
of how it was built on `screen-vp9`'s branch is in this file's history,
up to the commit that replaced it with this one.

Replanned on 2026-10-10, at release 0.0.1-alpha.2, with the use and the
scope final; the open items were taken up again the same day, after release
0.0.1-alpha.3. Replanned again on 2026-10-11, when amd64's floor went from
AVX2 to x86-64-v3 and stopped being a ceiling as well: what that bought and
what it opens is [its own section](#the-floor-raised-on-2026-10-11), and the
open items are in a new order for it. arm64's open items were taken up the
same day on the M2 Max, [in a section of their own](#arm64-on-2026-10-11).
Replanned again on 2026-10-14, when the captures were made again with a
video that is a video and an `uncover` that shows its terminal: what they
found is [its own section](#the-captures-made-again-on-2026-10-14), and
the open items are in a new order for it. Where a video's bytes go was
measured the same day, [in a section of its own](#where-a-videos-bytes-go-on-2026-10-14):
sub-pixel motion, which the scope had out on the strength of the set
before's video, a test pattern, and which is in it again.

## What it is for

One crate, taken by the wlshare daemon and the remotex gateway in place of
`screen-vp9`, pinned by a release tag. Both hold a `Stream` and hand it a
desktop as drawn:

- **Nearly every frame comes with rectangles**: the damage since the
  client's last frame (wlshare) or what the mirror changed in (remotex). A
  terminal typed into names 0.4% of a 1920×1080 picture a frame, a window
  dragged a quarter, and a page scrolled, a video and a window uncovered
  all of it, because the compositor damages the whole surface.
- **A whole picture is a keyframe, a stream's first frame, or a settle**: a
  quiet picture coded once at the ceiling after the quality walk had it
  coarser.
- **Frames are paced, so the cost that matters is one frame's latency**, on
  every core but one or two, eight at most: four threads on the six-core
  hosts both were tuned on.
- **The dial is 90 in practice**, and the walk's steps under it on a slow
  link, down to 20.

## Scope, final

- **Encode only, 8-bit 4:4:4, VP9 profile 1, BT.601 at studio swing.** No
  4:2:0, no other depth, no decoder as an API.
- **One crate with libvpx**, since the encoder came home to `screen-vp9`
  as `Coder::Native`, where it was a crate of its own, `screen-vp9-native`,
  that linked no libvpx outside its tests. libvpx's decoder holds every
  frame a test makes, and its encoder, the crate's other coder, is what the
  `*_against_libvpx` measurements play the captures into beside this one.
- **NEON on arm64 and x86-64-v3 on amd64 are the floor**, the build refused
  under them, and there is no ceiling: the encoder uses whatever its floor
  has without asking the machine, and the floor goes up when a higher one
  is measured to buy enough. No processor under it is kept working by this
  encoder; it is built with libvpx alone, by hand. The scalar lane is the
  reference the kernels are tested against.
- **A frame told where the picture changed is in segments**, the blocks
  outside the rectangles a segment that skips and is not loop filtered:
  the stream vp9-wasm was built to decode and libvpx's active map makes.
- **The decoders a stream is held to are the browsers' own**: libvpx in
  every test, and ffvp9 through `ffmpeg -c:v vp9 -f framemd5` on the
  streams the harness writes with `IVF` set. vp9-wasm is checked, and is
  not the gate.
- **The bitstream subset** is the table in
  [encoder.md](encoder.md#bitstream-subset). Out for good: compound
  prediction, the golden and alternate frames, backward adaptation,
  lossless, rectangular partitions. **Sub-pixel vectors are in**, to a
  quarter pixel, at EIGHTTAP_SMOOTH as the frame's one filter: they were
  out on the set before, whose video was a test pattern that moved as a
  scroll does, and on a video they are what its bytes were short of
  ([measured and taken](#where-a-videos-bytes-go-on-2026-10-14)).
- **`Speed::Fastest` stays, out of the public API**: the intra mode by line
  sums and no 4×4s. Neither user calls it; it is work in progress for an
  optimization it may yet become, moved only by the tests and the
  measurements.
- **No backward compatibility**, in the API or in a stream's shape: a
  change reaches a user as a pin bump.

## How it is measured

- **Against libvpx, both encoders handed the same frame back to back, the
  order alternating**, on the `frame-captures` set of 2026-10-13: eight
  scenarios at ten sizes, each cut to 120 frames
  ([benchmarks.md](benchmarks.md#test-material)) and played as the session
  played it (the same rectangles, dial and keyframe asks). The three
  measurements take 41 minutes over the cuts.
- **On an idle machine, one measurement at a time.** A run made beside a
  build or a profile is thrown away.
- **Three measurements**: a capture whole (`captures_against_libvpx`), each
  first picture as a keyframe at dials 40, 65 and 90
  (`keyframes_against_libvpx`), and each settled to 90 from 20, 65, 80 and
  89 (`settles_against_libvpx`). `ALONE=1` runs this encoder without
  libvpx, for a sweep of a constant.
- **What is aimed at**, per capture and size: a picture no worse than
  libvpx's at the same dial, no more bytes, and less time, the frames of
  rectangles first. Luma PSNR is read before bytes and time: a stream
  coarser at the same dial is not at the same dial.
- **The machine the tables are made on is the six-core AVX2 one**, an
  i5-8500T, which is x86-64-v3, the floor. The Mac
  runs the tests, for the NEON kernels, and a NEON kernel is measured on it
  as an AVX2 one is there, each build turn about with the one before; its own tables, in
  [benchmarks-m2-max.md](benchmarks-m2-max.md), are each line's median of
  three runs (`tools/median_of_runs.py`), its times moving by up to three
  fifths from one run to the next on a desktop in use.

## Where it stands

[benchmarks.md](benchmarks.md), one run of 2026-10-14 over every size of
the cut captures, release 0.0.18, the whole captures run again with the
vectors between pixels back:

- **A video is inside the gate again, over libvpx's picture.** `video`
  takes 1.04 to 1.06 of libvpx's bytes at +0.4 to +0.7 dB and `walk` 1.06
  to 1.09 at +0.3 to +0.5, where the gate the plan began with allows 15%
  more bytes at most; without the vectors between pixels they were 1.22
  to 1.27 and 1.16 to 1.25 at up to 0.6 dB under. Everything else over a
  whole capture is at fewer bytes than libvpx's and at its picture or
  better, but `drag` from 2048 to 3456 wide, 0.1 to 0.4 dB under at 0.42
  to 0.52 of the bytes: 75 cells of 80.
- Every capture at every size takes less time than libvpx: 0.13 to 0.90
  of it frame for frame, a line of text 0.13 to 0.19, `walk` 0.69 to
  0.76, `video` 0.78 to 0.90.
- A first picture coded alone as a keyframe is 0.1 to 0.7 dB over
  libvpx's at its bytes, but **`terminal`'s at dial 40, 2.4 to 7.7 dB
  under** at 0.64 to 0.87 of them; in 0.45 to 0.90 of libvpx's time up to
  1920 wide and 0.56 to 1.05 from 2048 wide.
- A settle of a busy picture takes more time than libvpx's: 1.01 to 1.68
  of it from 65 for 0.8 to 1.5 dB more picture at the same bytes, and
  `flood`'s 1.27 to 1.85 from 80 and 89.

On the M2 Max ([benchmarks-m2-max.md](benchmarks-m2-max.md), the median of
three runs of 2026-10-10 on the set before) the lines were those of the
AVX2 machine then: a whole capture 0.19 to 0.74 of libvpx's time frame for
frame, a picture level or better in 68 cells of 70; a keyframe alone 0.49
to 0.92 of its time, no cell over 1.00; a settle of a busy picture 0.96 to
1.42 of it from 65. It is not run again on the new set.

## Measured again in this replan

On the AVX2 machine, four threads, each run alone.

### Taken

| Change | What it bought |
|---|---|
| A transform block whose prediction is the source is not transformed or quantized; the loop filter passes an edge that is one value across | a flat 1920×1080 keyframe 12.9 → 7.9 ms, a flat 3840×2160 one 37 → 25 ms, a busy keyframe 5–10%; the same bytes |
| The token loops walk one table of position, band and neighbours a transform size and type | with the row below, a busy 1920×1080 keyframe on one thread 165 → 113 ms after the row above; the same bytes |
| The luma a 4×4 trial coded is kept, not coded again | |
| Tile rows on a busy frame, two or four by what its heaviest tile column recorded | the bools of a tile column were coded by one thread, a third of a busy keyframe's time: 17.6 → 7.7 ms at 1920×1080, 33.5 → 19.7 at 3840×2160, for 0.02% more bytes |
| A 4×4 block is tried in TM beside DC, V and H | +0.16 to +0.18 dB on keyframes at 1920×1080 for 3% of their time |

| A phase is a queue its caller works too, waiting on no thread that has not woken; a worker spins 0.1 ms before it parks; a frame naming 1024 8×8s or fewer is coded on one thread | a line of text 2.1 → 0.41 ms at 1280×800, 2.06 → 0.57 at 1920×1080, 3.23 → 1.48 at 3840×2160; the same bytes |
| A superblock a frame does not touch is recorded without being looked at | `uncover`'s frames 2.34 → 1.15 ms at 1280×800 |
| A keyframe leaves the search's anchors to the frame after it | a first picture's keyframe 5 to 25% less; the frame after pays the 4 to 9 ms |
| The reference is made by whichever is fewer to copy | `video` 26.4 → 23.4 ms at 2560×1600, `walk` 23.3 → 19.9 |

| The anchors a keyframe left wait for the first frame that changes the picture | a settle straight after a keyframe 3 ms less at 2048×1536 |

| The search marks the 8×8s whose anchors are taken again a row at a time, keeps what they held only when the tables are brought up to date by it, fills the tables in one phase, spreads a hash over the bits its table reads and picks a table without a division | the search 13% less on `video` and `flood` at 1920×1080; a frame of `browser` 5% less, of `drag` 2 to 4%, of `walk` at 2560×1600 4%, of `video` nothing to 1% |

| An intra block's coefficients are rounded up at 48/128 of a step, as libvpx's (`vpx_quantize_b`), where they were at an inter block's 42/128 (`vp9_quantize_fp`) | a keyframe 0.4 to 0.6 dB better at dials 65 to 90 for 0.1 to 0.4% more bytes, 0.35 to 0.47 at 40 for 1.3 to 2.1%; a whole capture 0.03 to 0.17 dB for −0.7 to +1.1%; a settle of one dial step 0.44 to 1.04 of libvpx's bytes at +0.2 to +0.8 dB, where it was 0.8 to 1.7 at −0.4 to +0.6 |

| A settling square of one value a plane, with the pixels coded above it and to its left that value too, is DC with nothing to code | a flat desktop settled from dial 20, `browser`, 0.47 to 0.92 of libvpx's bytes where it was 4.0 to 9.6: 1594 → 158 bytes at 1920×1080 against libvpx's 296, exact as before; the other captures' settles from 20 a hundredth or two fewer at 1920×1080; `walk`'s capture at 1920×1080 0.18% more bytes and 0.03 dB less, the rest the same to the byte |

The five rows before the last three are release 0.0.1-alpha.4 and the two
after them 0.0.1-alpha.5, `screen-vp9-native`'s; the last is `screen-vp9`'s,
of 2026-10-11. Only the last two change a stream.
Tile rows were out of the subset until now. A tile row splits the bool
coder and nothing else, libvpx and ffvp9 decode every frame of the captures
played through them this way, and vp9-wasm reads the header's tile rows;
vp9-wasm has not been run on these streams.

### Left as they were

| Constant | Tried | Result |
|---|---|---|
| `RATE_WEIGHT`, 35 | 25, 50 | bytes for decibels along one curve, on keyframes and on whole captures |
| `SPLIT_VARIANCE`, 8 | 4, 16 | nothing |
| `INTRA_BITS`, 24 | 12, 48 | nothing in bytes, −0.01 to −0.02 dB |
| `NEW_VECTOR_GAIN`, 4 | 2, 8 | −0.7% bytes for −0.10 dB and 5% more time; +0.4% for +0.14 dB and 8% more time |
| `INTER_SPLIT`, 1 | 2, 4 | −1.1% and −1.7% bytes for −0.03 and −0.05 dB |
| An inter block 32×32 at most | 64×64 allowed | 7% less time, −0.34 dB, `drag` and `browser` −1.5 and −0.7 dB |
| Three or four modes for a 4×4 | all ten, pruned | +0.3 dB more for 20–25% more keyframe time |

### Tried and not taken

- **The intra modes ranked by prediction error, the best few coded**: slower
  (132 ms against 125 over five keyframes) and −0.3 dB.
- **A 4×4 split at 10 steps in place of libvpx's 80**: +0.1 to +0.2 dB on
  keyframes for 1% fewer bytes in the same time. Two tests fail with it: a
  hatch is then coded in 4×4s, which are not tried in a diagonal mode, and
  the quality walk does not bring a churning picture under its link. Open,
  below.

- **A settle's inter blocks in the largest transform alone**: 25 to 30%
  less time and 1.9 to 2.4 dB less, which is libvpx's picture (`uncover`
  at 1920×1080 from 65: 16.3 ms and 43.80 dB against 22 to 26 and 45.70).
  The settle's time is its picture; a settle is there to reach the ceiling
  in one frame, so it stays.
- **An inter block whose largest transform codes nothing tries no
  smaller**: a settle of one dial step 20 to 35% fewer bytes for 0.07 to
  0.24 dB less, one from 65 2 to 4% fewer for 0.15 to 0.4 dB less, the
  time the same.
- **A superblock whose anchors give nothing has them taken no more.** On
  `video` and `walk` the anchors find next to nothing (without them 0.06%
  and 0.3% more bytes) and the projections everything (without them 17%
  and 16% more), where on `drag`, `flood` and `browser` it is the anchors
  (32%, 42% and sixteen times more bytes without). A superblock that
  changed eight frames running with no vector of its own, or a frame's,
  from an anchor of it or to one, had its anchors left untaken but for two
  frames in sixteen: the search 21% less on `video` and 26% on `walk`,
  which is 2 to 3% of their frames, at bytes within 0.1%. Counting only
  the superblock a vector arrives in, and not the one it comes from, cost
  `flood` 3.3% more bytes: what scrolls comes from where the new text
  appears. Four frames and two in thirty-two saved 3.4 and 4.8% and cost
  `walk` 0.09 dB. Not taken: what begins to move where the anchors were
  left is found up to sixteen frames late, for a fiftieth of a frame.

- **Any square of an inter frame that DC predicts to the pixel coded so**,
  and not a settling one alone: 1.2 to 2.9% fewer bytes over `browser`,
  `terminal`, `uncover` and `drag` at 1920×1080, and `drag`'s frames of
  rectangles 0.17 dB less for their 4.2%. Where the decibels go is not
  known; a block coded intra offers the blocks after it no vector.

## The floor, raised on 2026-10-11

amd64 was built with `-C target-feature=+avx2`, the one feature its kernels
are written in. It is built for `-C target-cpu=x86-64-v3` now, Haswell and
Excavator on: AVX2 with BMI1, BMI2, LZCNT, POPCNT, MOVBE, FMA and F16C. Both users
and this repository forbid a legacy path, and a processor under the floor
has the build without the `native` feature, so nothing was weighed against
it.

**What the level buys with no line changed is a hundredth.** The same tree
built three ways, each build played turn about with the others three times
over the 1920×1080 captures, keyframes and settles, four threads, the
machine idle; a line's time is its median of the three, a capture's the
geometric mean of its lines, over the `+avx2` build's:

| Build | Whole captures | Keyframes alone | Settles |
|---|---|---|---|
| `x86-64-v3` | 0.98 to 1.00 | 0.98 to 1.02 | 0.98 to 1.02 |
| `skylake`, the machine's own | 0.98 to 1.00 | 0.98 to 1.00 | 0.98 to 1.05 |

Every stream is the same to the byte in all three. The compiler did take
the level up: of the encoder's 206 `bsr`, 202 are `lzcnt` (the bool
coder's shift back to 128 among them), 607 shifts by a register are
`shlx`, `shrx` and `sarx`, 64 multiplies `mulx`. That is 0 to 2% on a
machine whose times move by a tenth, so it is on the line and not a
result. The floor is raised for what can be written on it, not for this.

**What x86-64-v3 has, and what this encoder can do with each:**

| | Use here |
|---|---|
| AVX2, sixteen `i16` or thirty-two `u8` a register | The kernels are AVX2 already, but the transforms and the quantizer in eight `i32`, the loop filter in an SSE register of eight `i16`, and the search's hashes and the intra predictions not at all. This is where the time is: the open items 5 to 8 below |
| `tzcnt`, `blsr`, `lzcnt`, `popcnt` | A compare's flags walked bit by bit: the search's anchors (item 7) |
| `shlx`, `shrx`, `mulx`, `andn`, `movbe` | The compiler's, taken already, above |
| `pext`, `pdep` | Not used: they are microcoded on Excavator and on Zen before Zen 3, eighteen cycles and up, and those are inside the floor |
| FMA, F16C | Nothing: the encoder has no floating point but a table of costs made once |

**What the floor has nothing for**: the token loops, each token's context
read from the two before it, and the bool coder, each bool's range from
the last. They are chains, and were made shorter by tile rows and by tables, not
by a wider register.

**The floor is x86-64-v3 for now.** It moves up only where a higher one
is measured to buy enough to justify it, and nothing is: AVX-512 was
dropped from the open items on 2026-10-11. x86-64-v4 runs only where
the processor and the operating system both expose AVX512F, AVX512BW,
AVX512CD, AVX512DQ and AVX512VL, which is by machine and not by
generation: Intel's desktop and laptop processors from Alder Lake on
ship without them as a rule, and no AMD one before Zen 4 has them. The
machine the tables are made on does not, so
nothing written for it could be measured or tested here. Between the
two levels there is no feature a codec reads: the `skylake` build above,
which adds the machine's own tuning, ADX, AES and CLMUL, measures the
same.

**arm64's floor is where it was**, NEON. Its ceiling is gone the same way,
and the dot product (`sdot`, `udot`, Armv8.2's, in every Apple M) was
measured as the next floor and is not worth one:
[arm64, on 2026-10-11](#arm64-on-2026-10-11).

**What a user changes with the pin**: `-C target-cpu=x86-64-v3` where its
`.cargo/config.toml` has `-C target-feature=+avx2`, the build refused
otherwise, and its own words for which processor the daemon and the
gateway need.

## Written on the floor

Each against the build before it, turn about three times at 1920×1080 on
four threads, a line's time its median and a capture's the geometric mean
of its lines; every stream the same to the byte.

| Change | Keyframes alone | Settles | Whole captures |
|---|---|---|---|
| A residual, its forward transform under 32×32 and the coefficients the quantizer reads in 16 bits a value: sixteen a register (`vpmaddwd` for a pair of products, `vpmulhuw` for a level), where they were eight of 32 with `vpmulld` | 0.85 to 0.95 | 0.93 to 0.98 | 0.94 to 0.98 |
| An intra prediction's row above copied whole where the picture has it, and TM written a row at a time in 16 bits for the compiler to vectorize | 0.91 to 1.02 | 0.97 to 1.03 | 0.94 to 1.02 |
| The loop filter's edges two at a time where two that lie one after the other take the same filter: sixteen lines an AVX2 register, each eight with its own level | 0.97 to 1.00 | 0.97 to 1.01 | 0.96 to 1.03 |
| The levels in 16 bits, and a block's tokens and the contexts they are coded at made sixteen coefficients at a time into a map (as far into the block as the scan reaches), which the loops that cost the tokens and record them only read | 0.91 to 1.03 | 0.92 to 1.00 | 0.95 to 1.02 |
| The loop filter passes a row of a superblock's edges at once where every line across them is one value, before it looks at any one edge | 0.87 to 0.98 | 0.97 to 1.00 | 0.96 to 1.00 |
| The search's row hashes in three multiplies a position where they were eight (a hash of eight pixels made of two of four, each of two of two), its anchors picked from a row of flags walked eight at a time by `tzcnt`, and its scratch not cleared between rows | the same | the same | 0.98 to 1.02 |
| The sums an intra mode is chosen by kept in 16 bits a line and multiplied in 32, where they were 64 a value and a plane at a time, and every intra prediction written a row at a time from a line of means along its edge, where a diagonal was written a pixel at a time and read back | 0.95 to 1.00 | 0.98 to 1.04 | 0.98 to 1.02 |
| A block's residual against the previous frame measured four 8×8s side by side at a time, thirty-two pixels a register (`vpsadbw` for the sums, the differences' sizes squared by `vpmaddwd`), and the previous frame read once for the four | the same | 0.94 to 0.98 | 0.83 to 1.01 |

The forward transforms are libvpx's to the bit as they were: the old ones
and these gave the same coefficients for every size and type over the
residuals that reach furthest (±255 signed as each of a transform's own
patterns, and as noise) and over noise, and the scalar reference panics on
a sum or a rounded value past 16 bits, which the tests hold the vectors to.
The 32×32 stays in 32 bits: its second pass leaves 16. NEON is the same
kernel in eight `i16` (`vmlal`, `vqrshrn`), its tests run on the Mac and
its time not measured. The quantizer on NEON reads the 16 bits and works in
32 as before: in eight `i16` it measured nothing, below.

The token map is held to coding a token at a time from the scan's own
neighbour tables, over every size, type and context. Costing the tokens
was 10 to 12% of the coder's time, nearly all of it in dense 4×4 and 8×8
blocks. On keyframes five of the seven captures are 0.91 to 0.96, and
`browser` and `terminal`, whose blocks are large with little in them, are
1.00 and 1.03, inside what this machine moves by: with the whole block
mapped whatever the scan reached, one or the other was 1.02 in each of
two runs.

## arm64, on 2026-10-11

On the M2 Max, each build turn about with the one before three times over
the 1920×1080 captures, keyframes and settles, four threads. A desktop in
use moves its times too much for a line's milliseconds to be read, so a
line's time is its ratio to libvpx's in the same run, the median of the
three, over the build before's; a capture's is the geometric mean of its
lines. Every stream the same to the byte.

| Change | Keyframes alone | Settles | Whole captures |
|---|---|---|---|
| A block's residual against the previous frame measured two 8×8s a register, sixteen pixels (`vabd` for the differences' sizes, `vpadal` for the sums), where it was an 8×8 at a time | 0.99 to 1.00 | 0.97 to 1.01 | 0.97 to 1.00 |
| A residual sixteen pixels a register (`vsubl` and `vsubl2` of one load), and a 4×4's in one, where it was eight and a row of four | 0.94 to 0.99 | 0.98 to 1.01 | 0.96 to 1.00 |

Tried and not taken: **the quantizer in eight `i16`**, as amd64's is in
sixteen, `sqdmulh` for the product shifted right 15: 0.97 to 1.00 on
keyframes and settles and 0.97 to 1.02 on whole captures, nothing past
what the machine moves by. Its kernel is 2.6% of a keyframe's samples.

**Where arm64's time is**, by samples under the pool's threads with the
first row above in (`samply`): on keyframes the bool coder 12%, the token
estimate 10%, the block walk 9%, the residual 6% before the second row,
the intra mode 5%, the forward transform and the intra predictions 4 to
5% each, the loop filter's edges 4%, and threads parked 15%. On the
captures the threads are parked 29% of the samples, the bool coder 14%,
the token estimate 6%.

**The dot product is not worth a floor.** What it would take is the moments
(1.8% of a keyframe's samples), the line sums (1.3%) and the residual
against the previous frame (2.1% of the captures'). The search's hashes are
polynomials with 32-bit multipliers, which `udot`'s 8-bit weights cannot
make without a different hash and so a different stream. Halved, that is a
hundredth or a hundredth and a half, under what this machine moves by; and
a floor of Armv8.2 with the dot product would have both users build their
Linux arm64 with it and leave the Cortex-A72 (the Raspberry Pi 4, the first
Graviton) under it. NEON stays the floor.

**The loop filter on NEON** is eight lines a register in `i16`, where
AVX2's pairs two edges for sixteen; libvpx's NEON filters sixteen lines in
`u8`. It is 4% of a keyframe's samples, so a rewrite would buy at most two
hundredths there. Not written.

**The conversion is one formula on every machine now.** All 2^24 colours
through `yuv`'s Balanced mode, on each of its paths: NEON made a luma one
under AVX2's in 41% of colours and never over, and the chroma of the two
differed by one in 17%. Balanced is three conversions, not two: NEON with
RDM (every Apple M, Graviton 2 on), NEON without it, which is the scalar
path's, and AVX2 and SSE's. Within one machine a colour converts the same
wherever it lies in a row. Its professional mode, 15 bits a coefficient,
makes the same planes on all five paths, within 0.51 of a code value of
BT.601's for every colour, where Balanced was up to 1.2 off on each path; a test
holds the conversion to its integer formula on every target. On the M2 Max
it is 1.6 times Balanced's time on one thread (0.77 ms a 1920×1080 against
0.49) and on four, as a whole picture is converted, 0.23 ms against 0.18,
and 0.97 a 3840×2160 against 0.96. Every stream of both coders changes with
it. The i5's times for it are not measured.

## The captures made again, on 2026-10-14

Two captures of the set of 2026-10-09 did not show what they were named
for. `uncover`'s page was drawn over its terminal, so from 2.3 s on no
frame changed a byte; it is `still` now. `video` was a test pattern in a
1280×720 window, little of the desktop, its motion as easy as a scroll.
The set of 2026-10-13 has an `uncover` whose terminal is moved off the
page, a `still`, and a 4K clip played full screen, every pixel of the
desktop the video's. Each capture is measured cut to 120 frames by
wlshare's `scripts/capture-samples.sh`, the way vp9-wasm's samples are
cut, on release 0.0.18, every stream the one it was. What the new set
showed:

- **The video is the first content this encoder codes worse than
  libvpx**: `video` and `walk` from 0.95–1.10 of libvpx's bytes over its
  picture to 1.16–1.27 under it, 0.0 to 0.1 dB up to 1920 wide and 0.3 to
  0.6 from 2048. That is past the gate. Where the bytes go is [the
  next section](#where-a-videos-bytes-go-on-2026-10-14). What a video can use that this encoder has not: sub-pixel
  vectors and the interpolation filters, compound prediction, and the
  golden and alternate frames, the first now in the subset
  ([Scope](#scope-final)); and the rate weight and the vector gains were
  set on screen content.
- **`drag` from 2048 to 3456 wide is 0.1 to 0.4 dB under libvpx's
  picture** at 0.42 to 0.52 of its bytes, where the set before had it over at
  every size. Both encoders leave the loop filter out of four tile
  columns, so it is not that. The cut is `drag`'s first 120 frames of
  402, so it may be the window's and not `drag`'s.
- **A keyframe of `terminal` at dial 40 is 2.4 to 7.7 dB under
  libvpx's**, at 0.64 to 0.87 of its bytes: the first picture of a
  terminal at its size, a few lines on a flat ground, which libvpx keeps
  at 49 to 55 dB in 1.8 to 6 kB and this encoder at 46 to 47.5 at every
  size. It was not measured before: the set before's first picture of
  `terminal` was the desktop before its resize, which the tables leave
  out. At 65 and 90 the same keyframe is over libvpx's.
- **`uncover` is a busy picture now**, rectangles over the whole surface
  every frame: 0.47 to 0.65 of libvpx's time, +0.7 to +1.1 dB at 0.43 to
  0.89 of its bytes.
- **The keyframes and settles of `video` and `walk` say nothing**: they
  are coded from each capture's first picture, which for these two is the
  black frame the clip is paused on. The tables leave them out.

## Where a video's bytes go, on 2026-10-14

Read with an inspection build of libvpx 1.16.0, `tools/vpxinspect/`: its
decoder counts what each read of a stream cost, by kind, and what its
blocks were, and its encoder can be held to whole pixels, to a half, to
one filter or to the last frame alone, and is libvpx unchanged otherwise
(its stock stream is the shipped one to the byte). The streams are the
harness's of the cut captures, both encoders at the capture's dials, the
same quantizer frame for frame.

**On `video` at 1920×1080**, 6.46 MB from libvpx and 8.13 from this
encoder, over its 120 frames, the first two of them black keyframes:

| | libvpx | libvpx, whole pixels | this encoder |
|---|---|---|---|
| intra blocks' coefficients | 0.36 MB | 0.49 | 1.37 |
| inter blocks' coefficients | 5.52 | 6.45 | 5.99 |
| partitions, modes, references, transform sizes, skips | 0.24 | 0.26 | 0.72 |
| vectors and filters | 0.33 | 0.12 | 0.04 |
| luma PSNR | 50.16 dB | 49.69 | 50.01 |

The two are within 5% of each other while little of the clip moves, part
from frame 12 as more of it does, and from frame 18 this encoder's is
1.18 to 1.42 of libvpx's bytes on every frame. libvpx's vectors are
sub-pixel on 31.5% of the area, 95% of what moves, 5.9% of the area at a
half pixel; its filter is the smooth one on 60% of that and the
regular on the rest. Its golden frame is on 28% of the area and worth
nothing: without it libvpx's stream is 0.07% smaller.

**libvpx held to less, over `video` and `walk` at every size**, its bytes
and PSNR against libvpx's own stream:

| libvpx held to | `video` to 1920 | `video` from 2048 | `walk` to 1920 | `walk` from 2048 |
|---|---|---|---|---|
| whole pixels | +13 to +15%, −0.40 to −0.47 dB | +18 to +21%, −1.30 to −1.50 | +7 to +9%, −0.26 to −0.32 | +9 to +12%, −0.76 to −0.83 |
| half pixels | +1.9 to +2.5%, −0.03 to −0.04 | +1.7 to +2.4%, −0.08 to −0.14 | +0.3 to +0.8%, +0.02 to −0.02 | +0.5 to +0.8%, −0.06 to −0.08 |
| half pixels, the smooth filter | +2.4 to +3.4%, 0.00 to −0.01 | +1.2 to +1.8%, −0.05 to −0.09 | −0.1 to +0.6%, +0.04 to 0.00 | −0.1 to 0.0%, −0.01 to −0.02 |
| half pixels, bilinear | +1.9 to +2.4%, −0.01 to −0.02 | +3.3 to +4.1%, −0.14 to −0.24 | +0.2 to +0.6%, −0.01 to −0.02 | +1.4 to +1.6%, −0.09 to −0.12 |
| half pixels, the regular filter | +5.2 to +5.3%, −0.24 to −0.32 | +11.6 to +13.2%, −0.90 to −1.14 | +3.7 to +4.3%, −0.20 to −0.21 | +6.8 to +7.6%, −0.56 to −0.63 |

A quarter pixel with the regular filter is +2.8 to +3.8% and −0.18 to
−0.30 dB up to 1920 wide, and within a percent of a half pixel with it
from 2048.
Against libvpx held to whole pixels, this encoder
is over its picture at every size: `video` 1.10 to 1.11 of its bytes at
+0.31 to +0.32 dB up to 1920 wide and 1.03 to 1.05 at +0.89 to +0.93 from
2048, `walk` 1.14 to 1.15 at +0.19 to +0.33 and 1.06 to 1.10 at +0.29 to
+0.51.

What it says:

- **A video's bytes are sub-pixel motion.** Without it libvpx codes a
  video as this encoder does, or worse, and this encoder's 16 to 27% is
  what sub-pixel vectors buy libvpx. The @2 sizes lose the most, the clip
  drawn at twice the pixels and so smoother at each.
- **Half pixels and one filter are nearly all of it.** Held to a half
  pixel and the smooth filter, libvpx is within 3.4% of its bytes and
  0.09 dB of its picture; bilinear, the cheapest filter, within 4.1% and
  0.24 dB. Neither needs the per-block filter VP9 can switch: the frame
  names its one filter. The regular filter alone is the worst of the
  three on this video.
- **What is left is this encoder's own**, a tenth of its bytes at most against
  libvpx held to whole pixels, for 0.3 dB more picture: intra blocks in
  frames of a video, 4.3% of the area against libvpx's 2.8%, 4.2% in 4×4
  and 8×8 blocks where libvpx's are 0.8%, at 2.8 of its coefficients'
  bytes, and partitions and modes at 2.8 of its bytes.
- **The golden frame is not what a video is short of**, on this clip.

What sub-pixel vectors would cost in time, the search and the
interpolation on every frame that moves, is not measured here: libvpx's
times in these runs were each beside this encoder's, not beside each
other's.

**Taken, the same day.** This encoder had vectors between pixels until
phase 9 deleted them on the set before's test pattern, where they bought a
twentieth of a decibel: quarter pixels, EIGHTTAP_SMOOTH as the frame's one
filter with its NEON and AVX2 kernels, libvpx's pruned search on each
inter block's luma, kept where a Laplacian model of the coded residual
says it pays, and run on every block of a frame only where the frames
before saved a sixteenth (`SEARCH_PAYS`), else on one in 32. They are back
as they were. Against the build before, the two and a build held to half
pixels played turn about capture by capture over every cut capture, four
threads, the machine idle:

| | `video` | `walk` | every other capture |
|---|---|---|---|
| quarter pixels: bytes, PSNR | 0.83–0.86×, +0.52 to +1.28 dB | 0.87–0.91×, +0.40 to +0.79 | 0.99–1.03×, −0.11 to +0.05 |
| quarter pixels: time frame for frame | 1.24–1.30 | 1.17–1.31 | median 1.02, within the tenth |
| half pixels: bytes, PSNR | 0.86–0.88×, +0.44 to +1.04 | 0.89–0.93×, +0.30 to +0.66 | 0.99–1.03×, −0.04 to +0.05 |
| half pixels: time frame for frame | 1.05–1.19 | 1.07–1.28 | median 1.01, within the tenth |

The quarter is kept: three hundredths of the bytes and 0.07 to 0.24 dB
over the half, for a tenth of a video's time, which is still
under libvpx's. Against libvpx ([benchmarks.md](benchmarks.md)) `video` is
1.04 to 1.06 of its bytes at +0.4 to +0.7 dB in 0.78 to 0.90 of its time,
and `walk` 1.06 to 1.09 at +0.3 to +0.5 in 0.69 to 0.76. What a capture
that moves by whole pixels pays is the search's probe and the model's
verdict, inside this machine's tenth. The shares above of what is left,
the intra blocks and the partitions, were read before the vectors came
back.

## What is open

In the order it is taken up, set again on 2026-10-14 for the captures
made again. What they found the picture short in comes first, since a
stream coded worse than libvpx's at more bytes is not one a user would
take for less time:

1. **The rest of the video's bytes**, 4 to 9% over libvpx's with the
   vectors between pixels back ([taken](#where-a-videos-bytes-go-on-2026-10-14)):
   read again with `tools/vpxinspect/` on the new streams, then this
   encoder's own share, which before the vectors came back was the 4×4
   and 8×8 intra blocks it codes in a frame of a video, five times
   libvpx's area, whose threshold (the 4×4 split at 80 steps, item 11)
   and intra-against-inter weight were set on screen content, and its
   partitions and modes at 2.8 of libvpx's bytes. A switchable filter per
   block, the one part of libvpx's sub-pixel coding not taken, held
   libvpx within 3.4% of its bytes when it was taken away. And the
   search's probe: its count starts again with each superblock row of each
   frame, so a row with fewer than `SEARCH_PROBE` blocks to refine is never
   probed, and a frame whose rows are all like that cannot turn the search
   back on, a small video in a window among them. Counted across frames
   instead, `browser` took 1.8 to 7.1% more bytes at up to 0.14 dB less
   and `video` the same: the search, once a page probes it on, takes
   vectors on a page that do not pay, which its model does not see. That is
   to be fixed first, and the count after it.
2. **`terminal`'s keyframe at dial 40**: why a few lines on a flat ground
   lose 2.4 to 7.7 dB to libvpx at dial 40 and none at 65. The 4×4 split
   at 80 steps (item 11) and the intra rounding (item 12) are the
   candidates already named; neither was tried on a picture this sparse.
3. **`drag` from 2048 wide**: whether its 0.1 to 0.4 dB under libvpx's
   picture is `drag`'s or the cut's, measured on the capture whole and on
   its busiest window.
4. **A frame of the video for the keyframes and the settles**: the two
   measurements take each capture's first picture, which for `video` and
   `walk` is black. Taking a picture from inside the cut for these two,
   such as its last, makes their rows mean something.

Then, set again on 2026-10-11 for the raised floor, what a kernel can be
rewritten for, because it changes no stream: the tests hold the bytes, and the measurement is time alone. Each
is measured against the build before it turn about, as the floor was, and
taken only if it shows past the tenth this machine moves by on the frames
it is for. A very busy picture, the whole screen changing many times a
second, is the edge of a remote desktop and not its middle, so what is
only about one still comes last.

On the floor, no stream changed:

5. **The inverse transforms in sixteen `i16` a register.** The forward
   ones are, above, and so are the levels, which the token loops read
   through a map. The inverse has to stay the decoder's to the bit, and
   a decoder's wraps at 16 bits where libvpx's C does: the sums that
   leave 16 bits are found before it is believed. It is 0.9% of the
   coder's time on a keyframe, 1.3% on a settle and 1.9% on a whole
   capture, so it is not written until something else has made that
   more. The 32×32 forward, whose second pass leaves 16 bits, is in 32
   still.
6. **The loop filter in one and two columns**: measured, and it stays.
   Without it at 1280×800 (one column) and 1920×1080 (two) a whole
   capture is 0.2 to 0.7 dB of luma further from its source (`walk`
   alone not) at 0.93 to 1.05 of the bytes, and a keyframe that is not flat 0.25 to 0.3 dB
   at the same bytes (`walk` alone not), for 0.85 to 0.99 of a capture's time. The filter
   itself is two edges to an AVX2 register and passes a flat row of a
   superblock's edges at once, above: a flat picture's keyframe was a
   fifth in the filter finding each edge even, one at a time, and is
   0.87 to 0.89 of what it was. On a keyframe that is not flat the
   filter is still 4 to 11% of the time.
7. **The search's hashes**: written, above, as loops the compiler makes
   AVX2 and NEON of, and held to a window at a time by a test. The
   anchors of a 1920×1080 frame take one thread 3.1 ms where they took
   4.8 (`anchor_time`), which no frame of the captures shows: the
   hashing was 8% of the coder's time on `drag` and 3% on `walk` and
   `video`, and is spread over the threads. What is left of the search
   is filling the tables again (4% of `drag`) and the projections (5%).
8. **The intra predictions**: written, and they show on a keyframe.
   Choosing the mode was 7% of a run of keyframes and predicting 7%. The
   first added each 8×8's row and column sums into 64 bits a value, a
   plane at a time; they are 16 bits, and their products with the edge
   32. The second gathered its edges through calls for four bytes, and
   wrote a diagonal mode a pixel at a time, which the residual after it
   then read eight at once and waited on; each is now a line of means
   along the edge and every row a stretch of it, held to the pixel at a
   time by a test. Six of seven keyframes are 0.95 to 0.98 of the build
   before and `walk`'s the same; the settles and the captures are within
   the run's noise. The quantizer on NEON in eight `i16`, as amd64's is in
   sixteen, was tried and measured nothing:
   [arm64, on 2026-10-11](#arm64-on-2026-10-11).

And one the list did not have, found when the four were done and the
captures profiled by line: **the residual against the previous frame**,
which every vector a square is offered is ranked by. It was measured an
8×8 at a time, each with its own read of the previous frame per plane,
and was a sixth of the coder's time on the captures: at 1920×1080 a
vector other than zero is measured on two to four times as many 8×8s as
the zero vector is. Four side by side are now one read and one kernel,
where a row of a square has four not yet measured and the picture holds
them. `browser`'s capture is 0.83 of the build before, `drag`'s frames
0.89, the other captures 0.96 to 0.98 but `terminal`'s, which measures
few, and the settles 0.94 to 0.98. On arm64 they are two 8×8s a
register now, [above](#arm64-on-2026-10-11). Looked at with it and left: the scan for
what has not changed is 32 pixels a register already, the compiler's,
and waits on memory; and no kernel of a keyframe is over 7% of it, the
bool coder's chain 10%.

Then, as before:

9. **Never measured, from the first plan's list**: the updates to the other
   probabilities on frames of a line of text, `INTRA_MODE_GAIN`, vectors ranked
   by their residual's squares on `browser` and `drag`, and `terminal` and
   `flood` captured at quality 100.
10. **The conversion was not the same on arm64 and amd64**: measured and
   made one, [above](#arm64-on-2026-10-11). Left: the decoder's conversion
   back to `B, G, R, X` differs by path as well, and `yuv` has no mode for
   it that does not; that is a viewer's pixels and not a stream.
11. **The 4×4 split threshold**: what makes it safe at 10 is a diagonal mode
   for a 4×4 where the 8×8 would have taken one, and knowing why a churning
   picture costs more at a low dial with it.
12. **An intra block's rounding past 48/128**: 56/128 buys a keyframe
   another 0.4 dB for 0.5% more bytes at dial 65 and up, and costs `walk`
   3% of its capture for 0.11 dB and `drag`'s capture 0.06 dB. Why
   `drag` loses is not known; a rounding by block size or by frame kind
   is untried.
13. **arm64's floor**: measured, and NEON stays the floor,
   [above](#arm64-on-2026-10-11).
14. **The first keyframe of a stream runs on cold buffers** (130 ms against
    40 for the next at 1920×1080).
15. **A gate that fails a build.** The comparison is in the tests again, so
    it can be one; its lines are not decided, and a time on this machine
    moves by a tenth from run to run.
16. **`Speed::Fastest` was not measured in this replan.**

Of a very busy picture alone, and last:

17. **A settle of a busy picture takes 1.0 to 1.9 of libvpx's time**:
   `still`, `uncover`, `browser` and `flood` 1.01 to 1.68 from 65, and
   `flood` 1.27 to 1.85 from 80 and 89, more from 2048 wide than under
   it. libvpx's settle
   time read at two levels because the measurement made a fresh encoder a
   pass and its second frame paid page faults, or did not; on encoders
   kept warm it is 0.5 to 0.65 of what it read, and this is what is left.
   The time is in the transform sizes under the largest, which are the
   settle's 1.0 to 1.4 dB over libvpx's from 65; from 80 they buy 0.0 to
   0.2 dB. Untried: the smaller sizes only where the largest leaves more
   than some error, and what a settle from 80 or 89 needs of them at all.
   The forward half of that time is taken above, with the picture kept.
18. **The frames of `video` and `walk`**, 0.56 to 0.70 of libvpx's time on
    the clip (0.64 to 0.83 on the test pattern before), the nearest the
    line of the whole captures but `uncover` and `still`. Item 1 comes
    before it: their time is for bytes over the gate. Profiled on the
    test pattern: the token estimate,
    the block walk, the intra mode and the transforms 5 to 11% each, and
    the search 9 to 12%, now gone over: its projections, its hashing and
    its tables' upkeep 0.3 to 0.45 ms each of an 11 ms frame at 1920×1080,
    none of them one thing. What is left untried in it: the row hashes of
    an 8×8 row taken once and not twice, the projections' sums in sixteen
    bits, a table of its own in place of the standard one; a hundredth of
    a frame each, and item 7 takes the first two with it.
19. **The tile phase of a busy keyframe** runs four threads at 75 to 80% of
    four (27 ms of wall time for 86 of processor time at 1920×1080). No
    thread was found waiting; what is left is memory, and not looked into.

## Main risk

That the picture costs the time: each step toward libvpx's picture on text
(4×4 blocks, more modes for them) is paid for on keyframes, which are where
this encoder is level with libvpx and not ahead. The frame of rectangles is
where it is ahead by a half to two thirds, and it is the frame both users
hand it most; a change is weighed on that frame first.

And, since the captures were made again, that a video is past what this
encoder's subset could code at libvpx's bytes. The 16 to 27% was sub-pixel
motion ([measured](#where-a-videos-bytes-go-on-2026-10-14)), and with the
vectors between pixels back a video is 4 to 9% over libvpx's bytes at a
better picture, and a sixth to a third more of its own time, still under
libvpx's.
