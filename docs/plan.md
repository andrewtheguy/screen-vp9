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
before's video, a test pattern, and which is in it again. The first four
open items were taken up on 2026-10-15, [in a section of their
own](#taken-up-on-2026-10-15). The other open-source encoders were read the
same day for what they decide differently, [in a section of its
own](#read-in-the-other-encoders-on-2026-10-15): six open items came of
it, 20 to 25. They were taken up overnight on 2026-10-16, [in a section of
their own](#taken-up-on-2026-10-16).

## What it is for

The wlshare daemon's, which holds a `Stream` coded by it where the client
asks and hands it a desktop as drawn. The remotex gateway takes the crate too,
but codes with libvpx alone: it is built for amd64's baseline, under this
encoder's floor.

- **Nearly every frame comes with rectangles**: the damage since the
  client's last frame. A
  terminal typed into names 0.4% of a 1920×1080 picture a frame, a window
  dragged a quarter, and a page scrolled, a video and a window uncovered
  all of it, because the compositor damages the whole surface.
- **A whole picture is a keyframe, a stream's first frame, or a settle**: a
  quiet picture coded once at the ceiling after the quality walk had it
  coarser.
- **Frames are paced, so the cost that matters is one frame's latency**, on
  every core but one or two, eight at most: four threads on the six-core
  hosts wlshare and remotex were tuned on.
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

[benchmarks.md](benchmarks.md), one run of 2026-10-15 over every size of
the cut captures, `walk` whole, on the build after release 0.0.19 that
updates an inter frame's mode, partition and vector probabilities, takes
the 32×32 intra transform, codes an intra square of an inter frame inter
at its vector too and cuts a smaller transform size's trial short, `walk`
captured again the same day, its clip paused for the settle:

- **A video is inside the gate, over libvpx's picture.** `video` takes
  1.00 to 1.05 of libvpx's bytes at +0.6 to +0.9 dB, 1.00 from 2880
  wide, and `walk`, whole, 1.01 to 1.08 at +0.4 to +0.7, where the gate
  the plan began with allows
  15% more bytes at most; without the vectors between pixels they were
  1.22 to 1.27 and 1.16 to 1.25 at up to 0.6 dB under. Everything else
  over a whole capture is at fewer bytes than libvpx's and at its picture
  or better, but `drag` from 2048 to 3456 wide, 0.2 to 0.4 dB under at
  0.40 to 0.49 of the bytes: 75 cells of 80.
- Every capture at every size takes less time than libvpx but `video`
  at 2048×1536, on the line at 1.01: 0.14 to 1.01 of it frame for frame,
  a line of text 0.14 to 0.22, `walk` 0.71 to 0.81, `video` 0.85 to
  1.01. The frame `walk`'s session settles, the video paused, is 0.86 to
  1.21 of libvpx's time up to 1920 wide and 1.16 to 1.57 from 2048, one
  frame's time, at 0.84 to 0.92 of its bytes and +0.3 to +0.9 dB.
- A first picture coded alone as a keyframe is over libvpx's at every
  dial and size, 0.1 to 0.8 dB at 0.86 to 1.06 of its bytes, `terminal`'s
  at dial 40 0.5 to 5.9 dB at 0.65 to 0.88 of them where it was 2.4 to
  7.7 dB under; in 0.38 to 0.98 of libvpx's time up to 1920 wide and 0.49
  to 1.04 from 2048 wide.
- A settle of a busy picture takes more time than libvpx's: 0.98 to 1.46
  of it from 65 for 1.0 to 1.5 dB more picture at 0.90 to 0.97 of the
  bytes, a picture of the video 1.04 to 1.61, and `flood`'s 0.99 to 1.96
  from 80 and 89; a settle of the video from 89 takes 1.03 to 1.32 of
  libvpx's bytes from 2048 wide.

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
as they were, and the gate has gone since ([item 1](#what-is-open)). Against the build before, the two and a build held to half
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
and `walk`, cut to its walk down, 1.06 to 1.09 at +0.3 to +0.5 in 0.69 to
0.76 (whole, 1.08 to 1.12 in 0.71 to 0.78). What a capture
that moves by whole pixels pays is the search's probe and the model's
verdict, inside this machine's tenth. The shares above of what is left,
the intra blocks and the partitions, were read before the vectors came
back.

## Taken up on 2026-10-15

The first four open items, one after another overnight, each change
played turn about against the build before over the cut captures, four
threads, the machine idle.

### Taken

| Change | What it bought |
|---|---|
| Nothing is looked for between pixels where each whole vector around a whole one costs more than `SHARP`, 7, times as much again: the picture sits on whole pixels | `drag`, `browser` and `uncover` 0.84 to 0.93 of their time at their bytes and picture; the rest within 0.3% of their bytes and 0.03 dB; `video` and `walk`'s time the same |
| An inter frame's compressed header updates its inter modes', intra or inter's, the reference's, the luma intra modes' and the partitions' probabilities where its own bools save more than the update costs, as it did the transform sizes' and skips' | every capture that moves 0.94 to 0.99 of its bytes, `browser`, `drag` and `terminal` the most; no decision moves, so the picture and the time are the same |
| And the vectors', each set to what the frame's bools measure where that saves more than its seven bits (libvpx's `update_mv`) | another 0.987 to 0.998 of the bytes |
| An intra square of 32×32 and up takes the 32×32 transform, not 16×16 as libvpx's keyframe | `terminal`'s keyframe at dial 40 4.1 to 8.3 dB of luma better for 0.1 to 0.8% more bytes, now over libvpx's; every other keyframe up to 0.5 dB better at its bytes but `browser`'s at 1280×800 at 90, −0.07; the settles −0.02 to +0.36 dB; the captures within 0.4% of their bytes in the same time |
| An intra square of an inter frame chosen over a vector, where the vector's residual is within `INTRA_OVER`, 2, of intra's variance with its bits, is coded inter at the whole vector too, taken where it codes no coefficient, else its luma coded intra and the cheaper taken in error and bits with each mode's bits, the square that takes the vector searched between pixels then; decided before the partition is written, since four 4×4s are a split (item 22) | over the target captures turn about with the build before: `video` 0.990 to 0.997 of its bytes and `walk` 0.989 to 0.998 at the same picture in 0.98 to 1.04 of the time; `browser`, `drag`, `terminal` and `uncover` within 0.3% of their bytes and 0.03 dB; `flood`, for reference, 0.997 to 0.998 in 1.07 to 1.14. Against libvpx the same day, `video` 1.00 to 1.05 of its bytes at +0.6 to +0.9 dB in 0.86 to 0.95 of its time and `walk` 1.01 to 1.08 at +0.4 to +0.7 in 0.71 to 0.80, but `video` at 2048×1536@2, the cell on the line: 1.01 to 1.02 of libvpx's time in three runs, where the build before it read 0.99 to 1.00 in three. Predicting the luma alone for the trial and not coding a square that takes the vector at it twice changed no stream and no time |
| A smaller transform size's trial stops at the transform block that takes its cost past the size above's (item 23), and the luma `refine` predicted at the vector it found is kept for `trial` in place of predicting it again (item 25) | every stream the same to the byte; the captures a median of 0.99 of the build before's time, `browser` 0.96 to 0.98 at every size, `video` and `walk` 0.97 to 1.07; the settles a median of 0.98 to 0.99 a pair of dials, from 65 `still`, `video` and `browser` 0.96 to 0.99, `uncover` 0.93 to 0.99: inside the tenth this machine moves by, in one direction over the 80 captures, and nothing to keep them out |
| An intra trial costs each transform block as `reconstruct` keeps it: its levels put to the `worth_coding` test before they are priced, so a block that will code nothing is weighed as nothing and its error as what nothing leaves, where it was weighed at the levels and bits it was never to code (a review's finding: an inter square's cost had the test, the intra it was held against had not). The trial's bits at its own context are moved from the test's at context 0 by the first token (`recontext`), one estimate a block as before | every target within 0.3% of its bytes and 0.03 dB but `browser` at 1280×800, 0.993 at +0.15 dB, and `walk` at 2880×1800@2 and 3456×2168@2, 1.006 at +0.11 and 0.997 at −0.09; the keyframes 0.994 to 1.001 of their bytes at −0.06 to +0.01 dB, the modes now chosen with the test in; the captures a median of 1.00 of the time, the settles 0.99, the keyframes 1.02, which was 1.07 with the estimate run twice |

Each update is chosen from the frame's own bools after its blocks are
decided, as the coefficients' were, so a frame's probabilities are what
the frames before it updated them to, from the keyframe's defaults; a
decoder keeps them in the frame context it refreshes. Without the updates
an inter frame coded every mode, partition and vector at VP9's defaults:
`video` at 1920×1080 paid 125 kB for its partitions against libvpx's 33,
72 kB for its references against 24, with LAST its only one. They are
67 and 53 kB now, and `video` is 1.040 of libvpx's bytes there, from
1.065.

`terminal`'s keyframe at 40 was a ground of one grey, 47, coded as 48
from the first block on: every block after predicted 48 from its
neighbours, and at dial 40 a 16×16's DC moves its mean two grey levels, so
nothing could take the one back. The picture off by one everywhere is a
squared error of one a pixel, 47 dB however well the text is coded;
libvpx's ground came out 47. A 32×32's DC is dequantized at half the step
and moves the mean one grey: the first flat 64×64 after the text takes
the level back, and the ground is 47 from there on.

### Measured and not taken

- **The transform size trial cut short** where a size's cost passes the
  size above it, which ends the trial anyway: every stream the same to
  the byte, and no time that showed.
- **A square judged at the refined vector.** `choose` decides intra
  against inter and whether to split on whole-pixel residuals, before
  the search between pixels; judging a square the whole vector would split
  or code intra at the vector it refines to took `video` and `walk` to
  0.96 to 0.98 of their bytes for 0.1 dB less in 1.11 to 1.27 of their
  time, the squares coarser, and cost `flood`, `terminal` and `uncover`
  up to 0.28 dB. Judging only intra against inter that way, the split
  left to the whole vector, was 0.98 to 0.99 of `video`'s bytes for
  −0.03 to −0.07 dB, and still cost the others 0.1 to 0.3 dB: the intra
  blocks of a frame are worth their bytes as this encoder codes them.
- **`INTRA_BITS` at 48 and 96**: within 0.4% of the bytes, `walk` up to
  0.12 dB less.
- **A square split where an 8×8 in it is busy**, for a few lines of text
  on a flat ground: it never fired on `terminal`, whose text squares are
  split already; the flat ground was the loss.
- **The 4×4 split at 10 steps, an intra block's rounding at 56/128, its
  DC at 64/128**: none moved `terminal`'s keyframe at 40 by more than
  0.4 dB.
- **The vectors priced at the inter mode probabilities the frame starts
  from**, now that they move from frame to frame, not at VP9's defaults:
  within 0.3% of the bytes and 0.04 dB, the time the same.
- **Item 22 with the search between pixels in its trial**, the inter
  candidate refined and coded at both vectors as an inter block is: twice
  the bytes, `video` 0.981 to 0.993 and `walk` 0.987 to 0.992, for 1.03
  to 1.10 of a video's time ungated and 0.99 to 1.07 gated at
  `INTRA_OVER` 2 or 4, since on a video the squares the moments give intra
  are near inter anyway and the trial runs on most of them; `flood` 1.08
  to 1.24 ungated and 1.07 to 1.14 gated. At the @2 sizes a video's time
  is on libvpx's line, so the whole vector alone is taken, which is inside
  the run's noise on every target.
- **A refined vector taken without the trial at both** where its luma
  residual's deviation is under a fraction of the whole vector's, the
  second line of item 18, three fractions each turn about with the build
  before over `browser`, `uncover`, `video` and `walk`: at 3/4 `video`
  0.987 to 0.999 of its bytes for 0.05 to 0.07 dB less in 0.93 to 0.99 of
  its time and `walk` 0.991 to 1.002 at −0.10 to +0.10 dB, but `browser`
  1.003 to 1.011 and `uncover` 1.002 to 1.011 of their bytes at −0.05 to
  +0.14 dB for none of their time; at 7/8 the same the more, `video` −0.08
  to −0.11 dB in 0.91 to 0.97; at 1/2 `video` 0.994 to 0.997 at −0.02 dB
  and `walk` 0.993 to 0.999 at −0.09 to +0.03 in the same time. Where the
  residual says the refined vector is clearly better, the coefficients say
  otherwise often enough that the trial earns its tenth: skipping it
  trades the picture for bytes along the line on a video and costs the
  users' frames both.
- **The quarter-pixel level at fewer points**, where the half-pixel level
  went where the whole pixels around said: the one quarter pixel the
  parabola's least rounds to, or a quarter pixel from the best towards
  it on each axis and their corner, in place of the four around the best
  and a corner (libvpx's `FIRST_LEVEL_CHECKS`). Each turn about with the
  build before: `video` 1.002 to 1.006 of its bytes at −0.02 to −0.03 dB
  in 0.95 to 1.03 of its time, `walk` 1.000 to 1.005 at −0.11 to +0.09 in
  0.93 to 1.00; `browser` and `uncover` within 0.3% and 0.04 dB in 0.93 to
  1.02, the one point 1.19 once on `uncover` at 2048×1536. The quarter
  level's four points are not where a video's time is, and the parabola
  through whole pixels misses the quarter pixel often enough to cost
  bytes and picture both.
- **A square coded whole and in quarters, and the cheaper taken** (item
  20), in four forms, each turn about with the build before. The whole
  first, at its vector searched between pixels, then the quarters cut
  short once they cost more, their decisions kept for the partition,
  over the 80 captures: `video` 0.933 to 0.982 of its bytes at −0.02 to
  −0.06 dB and `walk` 0.938 to 0.964 at −0.17 to +0.04, in 1.30 to 1.62
  and 1.12 to 1.33 of their time; `browser` 1.004 to 1.038 at ±0.05 dB in
  1.03 to 1.07, `terminal` and `uncover` within 0.6% in 1.02 to 1.15;
  `flood` 0.995 in 1.26 to 1.41. The quarters first, as they are decided
  without it, then the whole at its whole vector cut short past their
  cost and searched between pixels only where it wins, by the luma alone:
  `video` 0.966 to 0.996 and `walk` 0.967 to 0.987 at the same picture in
  1.08 to 1.24, but `browser` 1.000 to 1.041, and −0.27 dB once. The
  chroma's residual weighed on both sides as the moments measure it:
  `browser` 0.996 to 1.008, but `video` 0.986 to 0.998 and `walk` 0.990
  to 0.996 in 1.13 to 1.24, the deviation not being the error coded, and
  too heavy on a video. The whole tried only where every quarter came out
  inter at its whole pixels, four blocks at one vector: `video` 0.998 to
  1.000, nothing, so the 8×8s on a fifth of a video's area are not four
  blocks at one vector. Tried where every quarter is inter at any vector,
  at its largest transform size alone: `video` 0.991 to 0.998 and `walk`
  0.988 to 0.996 at the same picture in 1.00 to 1.12 of their time, the
  users' frames within 0.3% in 0.98 to 1.09. A hundredth of the bytes for
  five to eight of the time where the video is on libvpx's line, which is
  what item 22's search in the trial was refused on: not taken. The
  browser's bytes under the luma alone are from squares with intra or
  unchanged quarters, since inter quarters cost it nothing; and deciding
  the quarters before the whole costs 2 to 3% of every capture's time by
  itself, in the predictions made again and the decisions kept and
  brought back.

### Measured, for what comes next

- **What the search between pixels costs.** At 2048×1536, held to whole
  pixels `video` and `walk` take 0.70 of the time for 1.2 of the bytes;
  taking the refined vector without coding both, 0.90 to 0.95, the second
  trial being a tenth of a video's time and the search the rest.
- **Where `video`'s time goes at 2048×1536@2**, profiled by symbol on the
  build of item 22 with the trial and search as they are, of the coder's
  own samples: the filter between pixels 13% (across 6, down 5, the
  prediction of a chosen vector 2), the forward transform 7, `partition`
  6, the residual's moments 6, the token estimate 5 and the tokens
  written 2, the tape 4, the quantizer 4, `choose` 3, `trial` 3,
  `intra_mode` 3, `refine`'s costing of a vector 3, the subtraction 2,
  and the whole-pixel search 4 in all (the hashing 2, its index 1, the
  projections 1). Nothing is a tenth: the time is the trial's body, and
  the search between pixels is where the plan's levers were, both
  measured above to trade the picture.
- **`drag` from 2048 wide is under libvpx's picture whole as well as cut**:
  over its 402 frames 0.09 to 0.26 of libvpx's bytes at 0.12 to 0.34 dB
  under from 2048 to 3456 wide and +0.67 at 3840, where the cut, its
  busiest 120 frames, is 0.25 to 0.49 at 0.18 to 0.42 under. After the
  keyframe libvpx spends 2.5 to 30 kB a frame on the still desktop, 50.0
  dB to 51.4 by frame 9, where this encoder's frames are 1.4 to 1.9 kB
  and stay near its keyframe's 50.3 to 50.8: a rate, not a picture worse
  at its bytes.
- **Where `video`'s bytes are still over**, at 1920×1080 against libvpx:
  intra coefficients 1008 kB to 357, its intra 4×4s and 8×8s 3.1% of the
  area to 0.7%; transform sizes 100 kB to 48, intra modes 97 to 17, inter
  modes 165 to 93, vectors 397 to 289 (libvpx's 37 kB of filters aside);
  inter 8×8s 20% of the area to libvpx's 5% and 16×16s 7% to its 26%,
  this encoder coding no rectangle. Its inter coefficients are 4.76 MB to
  5.52, at 0.6 dB more picture.

## Read in the other encoders, on 2026-10-15

What the other open-source VP9 encoders do that this one does not, read
and not measured, each weighed against how this encoder decides the same
thing rather than taken as better for being theirs. Read: SVT-VP9 (Intel's,
dormant since 2020: 4:2:0, no tiles, no segmentation, a fixed transform
size a block size, one bool coder a picture), the VP9 encoders written in
Rust (`rusty_vp9`, Apache-2.0, 4:2:0, a third more bytes than libvpx by
its own count; `rivet-vp9`, source-available with no patent grant, read
and nothing taken; `zvidlib-vp9-encoder`, MIT, profile 0, with a
lookahead; `rvp9`, BSD-3, libvpx's realtime path line for line, so
nothing libvpx has not), rav1e for the shape of its decisions, libvpx's
own realtime path at the speed the tables run it and libaom's after it,
and OpenH264's screen mode. None is a realtime encoder but libvpx's path
and SVT's, and none codes a desktop in fewer bytes than this one at its
picture.

What they have in common and this encoder has not: **a square is coded
whole before it is split**, and split only where coding it whole came out
dear per pixel, the split's trials cut short once their running cost
passes the whole's (rav1e's `encode_partition_bottomup`, zvidlib's
`search_partition`, `rusty_vp9`'s `rd_pick_partition`, SVT's
`inter_depth_decision`). This encoder splits by rules on the moments
(`SPLIT_VARIANCE`, `INTER_SPLIT`) and never weighs the whole against the
split in bits; it is the one thing read that bears on the video's inter
8×8s where libvpx codes 16×16s ([item 1](#what-is-open)). Rectangles,
which libvpx's variance partition takes from the same moments this
encoder measures, every Rust encoder keeps at its slowest speed alone.

What libvpx's realtime path does cheaper, not better: it ranks modes and
vectors by the sum of the absolute Hadamard coefficients of the quantized
residual and costs no tokens until the block is coded (`block_yrd`); its
transform size is a rule on the residual's variance against the AC step
(`calculate_tx_size`), never above 16×16; it tries intra in an inter
frame only where the inter cost passes a penalty scaled by the quantizer
(`vp9_get_intra_cost_penalty`); its search between pixels measures a
candidate with a 2-tap filter, and from the whole-pixel costs around the
winner can take one predicted point a level (`_pruned_more`); and libaom
after it picks a block's sub-pixel precision from its residual at the
whole vector (`subpel_select`). SVT codes the few best candidates inter and
intra both at the full cost and takes the cheaper, and skips the intra
trial where the best inter candidate kept no coefficient
(`full_loop_escape`); it interpolates a superblock's search area once and
shares it among the superblock's blocks. zvidlib keeps one luma prediction
a vector a superblock, shared by every partition level and the search
between pixels. `rusty_vp9` cuts a smaller transform size's trial short
once its running cost passes the size above (`rd_cost_y`).

**Why libvpx sharpens a still desktop after a keyframe** ([item
3](#what-is-open)) is not cyclic refresh, which `aq_mode` 0 leaves off: the
harness hands a frame its session knew no damage for to both encoders
whole, and libvpx codes every block of it at the zero vector and keeps
the residual its thresholds pass, the keyframe's quantization error
coded again, where this encoder marks the 8×8s still and at this dial
and leaves them in the inactive segment. On a frame of rectangles libvpx
does as this encoder does. So it is this encoder's to choose, on the
whole frames after a keyframe. That those frames of `drag` carry no
rectangles is to be confirmed on the capture.

Read and not taken: the Laplacian and variance models of a block's rate
(libvpx's, `rusty_vp9`'s and `rvp9`'s), which this encoder measured wrong
both ways on 2026-10-14; rav1e's token rate from a table by the
distortion, which rav1e ships off at every speed; a gate on the search
between pixels by a SAD threshold (zvidlib, `rusty_vp9`), the gate that
missed a fractional scroll here; SVT's fixed transform size and its
decisions blind to chroma, which in 4:4:4 is two thirds of the pixels;
libaom's hash search, exhaustive and per frame and used for intra block
copy alone, where this encoder's anchors are sparse and kept; rav1e's
assembly and its dispatch at run time, against the floor; and nothing
read splits the bool coder across threads as the tapes and tile rows do,
or costs tokens cheaper than the tables. Neither libvpx at this
configuration nor this encoder adapts probabilities backward (libvpx 1.16
defaults `frame_parallel_decoding_mode` on), so neither has the bitstream
the other has not.

## Taken up on 2026-10-16

The open items of 2026-10-15 in their order, overnight, each change played
turn about against the build before over the cut captures, four threads,
the machine idle.

### Taken

| Change | What it bought |
|---|---|
| An inter block at a whole vector other than zero that its superblock's anchors gave, content arrived whole from the frame before, weighs its transform blocks' bits at `CARRIED_WEIGHT`, 17, half `RATE_WEIGHT`, in the transform size's trial and in which levels are kept; intra against inter is weighed at `RATE_WEIGHT` still (items 3 and 24) | `drag` 0.975 to 0.983 of its bytes for +0.59 to +0.68 dB from 2048 wide, which was 0.13 to 0.43 dB under libvpx's picture and is 0.2 to 0.5 over, and 1.02 to 1.03 for +0.97 to +1.06 dB under it; `browser` 1.02 to 1.03 of its bytes for +0.20 to +0.40 dB, but 1.22 at 1600×1000 for +0.20, still a quarter of libvpx's; `uncover` 1.004 to 1.044 for +0.07 to +0.23; `terminal` and `flood` within 1.1% and 0.06 dB; `video`, `still` and every keyframe and settle the same to the byte, `walk` within 0.4% and 0.11 dB; the time of the three that code more 1.02 to 1.03 of the build before's |

### Measured and not taken

- **The still 8×8s of a frame of rectangles coded again at the dial**
  until each was coded two, three or four times since its source last
  changed (item 24 as it was written, on the rectangles `drag`'s frames
  after its keyframe carry, there being no whole frame among them): every
  stream within 0.5% of its bytes and 0.05 dB. At the quantizer a block
  was coded at, its error quantizes to nothing, and `drag`'s rectangles
  hold the moved window and what it uncovered, few 8×8s still.
- **`RATE_WEIGHT` at 25, 17 and 12 over the whole frame**, against libvpx
  on every scenario at 1920×1080 and 2048×1536: `drag` from 2048 wide
  took fewer bytes for 0.8 dB more at 17, the one capture off the curve;
  everywhere else bytes for decibels along it, `video` 1.04 of its bytes
  for 0.1 dB, `walk` 1.06 for 0.35. Held to the inter blocks, 17 cost
  `video` 1.024 for 0.05 dB and the intra blocks alone moved `drag` 0.17
  dB; held to inter blocks at anchored vectors, `video` is untouched.
- **The 4×4 split at 10 for 80, and a 4×4 offered the 8×8's mode** where
  it is a diagonal (item 11), alone over the keyframes and captures of
  `browser`, `terminal`, `uncover`, `video` and `walk` at 1920×1080 and
  `walk` at 2048×1536@2. At 10 the screens' keyframes 0.990 to 1.001 of
  their bytes for +0.01 to +0.55 dB, `terminal`'s at 40 the most, but a
  video's 1.06 to 1.13 at −0.31 to +0.17, and `walk`'s capture 1.02 to
  1.03: a busy picture's 8×8s are in 4×4s, which is why a churning
  picture costs more at a low dial with it. The mode the moments choose
  for an 8×8 split so is never a diagonal, so offering it changes no
  stream; a diagonal there wants the 8×8 coded each way first, the
  keyframe's time.
- **`INTRA_MODE_GAIN` at 4, 8, 32 and 64** for 16 (item 9), alone over
  the keyframes at 40, 65 and 90 and the captures of `browser`,
  `terminal`, `drag`, `uncover`, `video` and `walk` at 1920×1080 and two
  at 2048×1536@2: every keyframe within 0.3% and 0.02 dB, every capture
  within 1% and 0.03 dB but `terminal` at 4, −0.13 dB. An 8×8's mode is
  chosen by coding it each way; the gain only ranks a larger square's
  modes by their moments. Left at 16.
- **A switchable filter per block** (item 1's last line): built as an
  experiment, the regular filter in a scalar pass, each inter block's
  filter coded as VP9 codes it when a frame's is switchable, an inactive
  block's and a quiet superblock's included, at the default probabilities;
  a block between pixels takes the filter whose luma residual is less, its
  luma coded again at regular where that wins, a block at whole pixels the
  filter that costs least beside its neighbours'. libvpx's decoder held
  every test's stream. Turn about with it off: `video` 0.988 to 1.009 of
  its bytes at −0.01 to −0.05 dB from 1280 to 3840 wide, `walk` 1.016 to
  1.018, the screens 0.999 to 1.028, `terminal` the most, in the symbols.
  With their probabilities updated the screens' symbols would cost next
  to nothing; a video's gain is a percent at most, for a second
  prediction of every block between pixels where its time is on
  libvpx's line, and the kernels across and down would be written again
  for taps whose pairs pass sixteen bits in `maddubs`. Not taken.
- **The intra blocks of an inter frame, five ways** (what item 22 left,
  the reading [below](#measured-for-what-comes-next-1)), each turn about
  with the build before over `video` and `walk` at 1920×1080 and
  2048×1536@2 and `browser`, `uncover`, `terminal` and `flood` or `drag`
  at 1920×1080. `INTRA_OVER` at 4, 8 and without limit, every intra
  square near or far coded inter too: within 0.8% of the bytes and 0.11
  dB, no time. Their transform held to 16×16 as libvpx's is: nothing, and
  `walk` −0.1 dB. No 4×4s in an inter frame: `video` 0.986 to 0.992 of its
  bytes and `walk` 0.975 to 0.981 at −0.04 to −0.06 dB, `terminal` 1.004
  at −0.13. Split at 4, 16 and 64 times `SPLIT_VARIANCE`'s variance in an
  inter frame: `video` 0.994 to 1.000 at −0.02 to −0.13 dB, `walk` 0.97 to
  0.99 at −0.02 to −0.14. Each is bytes for decibels along the curve on a
  video, and not the 2.25 times libvpx's bytes a pixel of intra the
  reading found: what makes an intra block of a video dear is not its
  size, its transform, its 4×4s or how often it is coded both ways.
- **A square coded whole first and its quarters tried only where the
  whole is dear a pixel** (item 20's `rusty_vp9` gate on the whole-first
  form of 2026-10-15), the whole's coded cost a pixel under 1/16 to 4/16
  of a step squared keeping it, each turn about with the build before
  over `video`, `walk`, `browser`, `uncover` and `terminal` at 1920×1080
  and 2048×1536@2: at 1/16 `video` 0.955 to 0.989 of its bytes at −0.31
  to −0.36 dB in 1.08 to 1.26 of its time, `walk` 0.92 to 0.95 at −0.5,
  `browser` 1.03 to 1.20 at −0.04 to −0.23, `terminal` 1.07 at −0.19; at
  2/16 and up the whole kept nearly everywhere, `video` −0.7 dB and the
  screens 1.06 to 1.40 of their bytes at −0.1 to −1.9 dB. A whole that
  codes cheap a pixel is still coded better in quarters, and the gate
  keeps it whole: ungated, the form is what 2026-10-15 measured, `video`
  0.96 to 0.98 of its bytes in 1.5 to 1.6 of its time. The chroma counted
  as it is coded, item 20's other line, is not tried: weighed by its
  moments it already cost `video` 1.13 to 1.24 of its time, and coding it
  costs more.
- **The transform sizes under the largest tried only where the largest
  leaves a residual past a threshold on the moments** (item 23's second
  line, libvpx's `calculate_tx_size` as a gate on the trial, and item
  17's untried line), the residual's variance a pixel over its planes
  under 4/16, 16/16 or 64/16 of a step squared taking the largest alone,
  over the settles of `still`, `uncover`, `browser`, `video`, `walk` and
  `terminal` at 1920×1080 and 2048×1536@2: from 65 0.71 to 0.90 of the
  bytes and time at −0.5 to −3.5 dB even at 4/16; from 80 0.35 to 0.83 at
  −0.2 to −0.7; from 89 0.41 to 0.95 at −0.03 to −0.11. A settle's
  residual is the coarse dial's quantization error, under a sixteenth of a
  step squared nearly everywhere, and the smaller sizes are what code it:
  a settle needs them from every dial.
- **The search between pixels to a precision the block picks** (item 25's
  second line, libaom's `subpel_select`), alone over `video`, `walk`,
  `browser`, `uncover` and `drag` at 1920×1080 and 2048×1536@2. None where
  the residual at the whole vector already varies by under 4/16, 8/16 or
  16/16 of a step squared a pixel: `video` 1.04 to 1.11 of its bytes at
  −0.3 to −0.9 dB and `walk` 1.04 to 1.08 at −0.4 to −1.2, for a fifth
  of their time, the screens within 0.2%; a block the whole pixels
  predict well enough to code little still predicts better between them.
  A half pixel alone where the motion is 4 or 8 pixels or more: within 1%
  and 0.05 dB, and no time, the quarter level being four points.
- **`CARRIED_WEIGHT` in intra against inter too** (`decide`), and **in
  `worth_coding` alone**: the first took `terminal`'s text inter, 0.6% more
  bytes for 0.10 dB less at 1920×1080; the second kept nothing of
  `drag`'s gain, which is in the transform size the trial takes. 12 and 8
  for 17 bought `drag` 0.05 to 0.1 dB more and `browser` 2 to 4% more of
  its bytes.

### Measured, for what comes next

- **Where `video`'s bytes are over at 1920×1080**, read with
  `tools/vpxinspect/` on the build of item 24 (`VPXI_FRAMES` per frame):
  the intra blocks of its inter frames, its two keyframes being of black.
  Their coefficients are 694 kB to libvpx's 254 and their chroma's 222 to
  103, their modes 86 to 17, on 4.6% of the area to libvpx's 3.8%: 2.25
  times libvpx's bytes a pixel of intra, the intra 4×4s 1.1% of the area
  and 64×64s 1.7%, where libvpx's are 16×16s and 32×32s. The inter
  coefficients are 0.89 of libvpx's. Rectangles are on no measurable share
  of libvpx's area (item 21).

## What is open

In the order it is taken up, set again on 2026-10-14 for the captures
made again. What they found the picture short in comes first, since a
stream coded worse than libvpx's at more bytes is not one a user would
take for less time:

1. **The rest of the video's bytes**, 1.00 to 1.05 of libvpx's (`walk` 1.01 to 1.08)
   with the vectors between pixels back and an inter frame's
   probabilities updated ([taken](#taken-up-on-2026-10-15)). Read again
   with `tools/vpxinspect/`, what is over is this encoder's own: intra
   coefficients at three times libvpx's, its intra 4×4s and 8×8s four
   times libvpx's area, and inter 8×8s where libvpx codes 16×16s and
   rectangles. Judging a square at its refined vector did not take the
   intra blocks away without costing the screens their picture
   ([not taken](#measured-and-not-taken)); what is untried is a square
   coded inter and intra both and the cheaper taken, as `trial` takes a
   vector, and the rectangles. A switchable filter per block, the one
   part of libvpx's sub-pixel coding not taken, held libvpx within 3.4%
   of its bytes when it was taken away, and is [measured and not
   taken](#measured-and-not-taken-1) here on 2026-10-16: a percent of a
   video's bytes at most. Coding a square both ways is item 22, taken;
   rectangles are item 21, which libvpx does not code here.

   **The search's judgement, taken on 2026-10-14.** A Laplacian model of
   the coded residual judged both whether a refined vector was taken and,
   summed over a frame, whether the next frame refined every block or
   probed one in 32 (`SEARCH_PAYS`, `SEARCH_PROBE`), and it was wrong both
   ways. No line of the gate did better than its sixteenth: four builds
   turn about over every cut capture, at a thirty-second every stream
   within 0.2%; at a sixty-fourth `browser` at 1600×1000 0.93 of the bytes
   and `flood` 11–20% more time for 0.2% fewer; with the gate open
   `browser` 0.77–0.97 of the bytes at five sizes, but `terminal` 0.6–1.7%
   more, the model taking vectors on text that did not pay. The probe's
   count started again each superblock row, so a sparse row was never
   probed; counted across frames, a probed block that took its vector cost
   `browser` up to 7.1%, a block refined alone among blocks that were not
   leaving their vectors predicting worse, so a probe was made to measure
   only. And a page gains in bursts, frame 58 of `browser` at 1600×1000
   20.4 kB without the search and 2.8 kB with it, scrolled by a fraction
   of a pixel between frames that move by whole ones: a gate that reads
   the frames before is a frame late on every one, whole superblocks as
   probes or single blocks.

   So the gate is gone and every block is refined, and a block the search
   found another vector for is coded at both by `trial`, its luma and each
   vector's mode and bits at `RATE_WEIGHT`, and takes the cheaper: the
   model is gone with the gate. Against the build before, turn about over
   every cut capture: `browser` 0.77–1.00 of its bytes at +0.0 to +0.18
   dB in 1.11–1.28 of its time, `uncover` 0.93–1.00 in 1.12–1.26, `video`
   0.99–1.01 at +0.11 to +0.14 dB in 1.04–1.17, `walk` within 0.7% at up
   to +0.12 in 0.97–1.12, `flood` 0.995–0.998 at +0.05 to +0.11 in
   1.03–1.17, `terminal` within 0.4% in 1.00–1.10, and `drag`, which
   gains nothing, in 1.19–1.25; the median 1.13. Against libvpx `video`
   is 0.84 to 0.98 of its time frame for frame, the most at 2048×1536@2,
   on the line, where the gate allows 1.00. The tables of
   [benchmarks.md](benchmarks.md) are of this build.
2. **`terminal`'s keyframe at dial 40**: taken on 2026-10-15, [the
   32×32 intra transform](#taken-up-on-2026-10-15). A ground of one grey
   coded a level off from the first block stayed off, a 16×16's DC moving
   its mean two levels at that dial; it is over libvpx's picture now.
3. **`drag` from 2048 wide**: taken on 2026-10-16, [the dragged window's
   bits weighed lighter](#taken-up-on-2026-10-16). Its frames after the
   keyframe are frames of rectangles and not whole frames, and what they
   hold is the window moved and what it uncovered, few 8×8s still: the
   picture libvpx gains there is the moved window's error coded again at
   its vector, frame after frame, which this encoder's `RATE_WEIGHT`
   weighed as not worth its bits. It is over libvpx's picture at every
   size now.
4. **A frame of the video for the keyframes and the settles**: taken on
   2026-10-14. The two measurements took each capture's first picture,
   which for `video` and `walk` is the clip paused on black; they take
   these two's last picture of more than one colour now, and `walk` is
   measured whole, where its cut kept the walk down alone. `walk` ended on
   black too, the clip stopped, and the settle its session asked for was
   of that; on 2026-10-15 wlshare's walk pauses the clip where it stands
   instead, partway back up the dial, and `walk` was captured again: 174
   to 207 frames, the dial down to 20 or 30, back up to 41 to 75, and the
   paused frame settled at 90, which is the picture its keyframe and settle
   are measured on too. It is captured at 1280×800 to 1600×1000 and made
   from the 1600×1000 session from 1920×1080 up, the container on this
   machine playing 1920×1080 at 12 to 22 frames a second and the dial
   walking only to 40 there.

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

9. **Never measured, from the first plan's list**: `INTRA_MODE_GAIN`,
   [measured on 2026-10-16](#measured-and-not-taken-1) and left at 16;
   vectors ranked by their residual's squares, which is how `choose`
   ranks them; and `terminal` and `flood` captured at quality 100, which
   wants captures made at that dial, the harness playing each at its
   own: not made.
10. **The conversion was not the same on arm64 and amd64**: measured and
   made one, [above](#arm64-on-2026-10-11). Left: the decoder's conversion
   back to `B, G, R, X` differs by path as well, and `yuv` has no mode for
   it that does not; that is a viewer's pixels and not a stream.
11. **The 4×4 split threshold**: [measured on
   2026-10-16](#measured-and-not-taken-1). At 10 a busy picture's 8×8s are
   4×4s, a video's keyframe 1.06 to 1.13 of its bytes, which is why a
   churning picture costs more at a low dial; the screens' keyframes gain
   up to 0.55 dB. A diagonal 4×4 where the 8×8 would take one wants the
   8×8 coded each way first, which is untried for its time.
12. **An intra block's rounding past 48/128**: 56/128 buys a keyframe
   another 0.4 dB for 0.5% more bytes at dial 65 and up, and costs `walk`
   3% of its capture for 0.11 dB and `drag`'s capture 0.06 dB. Why
   `drag` loses is not known; a rounding by block size or by frame kind
   is untried.
13. **arm64's floor**: measured, and NEON stays the floor,
   [above](#arm64-on-2026-10-11).
14. **The first keyframe of a stream runs on cold buffers**: measured on
    2026-10-16 at 1920×1080, the first 1.1 to 1.2 times the second on
    `browser`, `still`, `drag` and `terminal` (25.6 ms to 21.8 on
    `browser`), where it was 130 ms to 40: a few milliseconds once a
    stream, and left.
15. **A gate that fails a build.** The comparison is in the tests again, so
    it can be one; its lines are not decided, and a time on this machine
    moves by a tenth from run to run.
16. **`Speed::Fastest` was not measured in this replan.**

Read in the other encoders on 2026-10-15
([above](#read-in-the-other-encoders-on-2026-10-15)), each measured turn
about against the build before over the cut captures and weighed on the
frame of rectangles first, as every change is; 20 to 22 bear on item 1,
23 on item 17, 24 on item 3 and 25 on item 18:

20. **A square coded whole before it is split**, [measured in four forms
    and not taken](#measured-and-not-taken): the whole first and the
    quarters cut short past it (rav1e, zvidlib) takes `video` to 0.93 to
    0.98 of its bytes in 1.3 to 1.6 of its time; the quarters first and
    the whole after them, cut short past their cost, a hundredth of the
    bytes in 1.00 to 1.12. What it is for is the video's inter 8×8s on a
    fifth of the area where libvpx's are a twentieth, and they are not
    four blocks at one vector: a whole that wins is coded over quarters
    whose vectors differ, at more residual for fewer vector bits. What
    is left of it: the whole's prediction kept for its quarters at the
    same vector (item 25), which the whole-first form needs to be cheap;
    the chroma in the comparison as it is coded and not as its deviation,
    since by the luma alone the screens' squares with intra quarters lose
    up to 4% of their bytes, not tried for its time; and `rusty_vp9`'s
    gate on the whole's cost per pixel, [measured and not
    taken](#measured-and-not-taken-1) on 2026-10-16: it keeps whole what
    the quarters code better.
21. **Rectangles**: not taken up, [measured on
    2026-10-16](#measured-for-what-comes-next-1): libvpx codes none on
    `video` and `walk` at 1920×1080, its inter area 8×8s 5%, 16×16s 26%,
    32×32s 5% and 64×64s 60%, a rectangle on no measurable share of it.
    What a video's bytes are short of in the partition is squares: this
    encoder's 8×8s on a fifth of the area where libvpx's 16×16s are, and
    the partitions, inter modes and vectors that cost, 65, 168 and 400 kB
    to libvpx's 33, 93 and 289.
22. **A square coded inter and intra both, the cheaper taken**, as `trial`
    takes a vector, with the two escapes read: no intra trial where the
    inter block kept no coefficient (SVT's `full_loop_escape`), and none
    where the inter cost is under a penalty scaled by the quantizer
    (libvpx's `vp9_get_intra_cost_penalty`), which is why libvpx's intra
    is rarer in a frame of the video, its intra 4×4s and 8×8s a quarter
    of this encoder's area. `INTRA_BITS` is the penalty's cousin on the
    moments; the trial is on the coded block. Taken
    ([measured](#taken-up-on-2026-10-15)) at the whole vector, with the
    penalty the other way about (`INTRA_OVER`): a third to a half of a
    percent of a video's bytes for a fiftieth of its time, which puts
    `video` at 2048×1536@2 a hundredth or two over libvpx's time, the
    cell that was on the line (item 18 is where that is bought back).
    With the search between pixels in the trial it is a percent, for a
    twentieth of a video's time ([not taken](#measured-and-not-taken)).
    Read again on 2026-10-16 ([measured](#measured-for-what-comes-next-1)),
    the intra blocks of a video's inter frames cost 2.25 times libvpx's
    bytes a pixel of intra, and five ways of coding them less or larger
    are [measured and not taken](#measured-and-not-taken-1): what makes
    them dear is open.
23. **The settle's smaller transform sizes cut short.** A smaller size's
    trial stops as soon as its running cost, transform block by transform
    block, passes the size above's (`rusty_vp9`'s `rd_cost_y`), which
    changes no stream; and the sizes under the largest are tried only
    where the largest leaves a residual past a threshold on the moments,
    libvpx's `calculate_tx_size` as a gate on the trial and not in its
    place, since the trials are the settle's 1.0 to 1.4 dB from 65. The
    first is taken ([measured](#taken-up-on-2026-10-15), a few hundredths
    of a busy settle); the second, with item 17's untried line, is
    [measured and not taken](#measured-and-not-taken-1) on 2026-10-16: at
    any threshold it takes a settle's decibels.
24. **A still desktop coded finer on the whole frames after a keyframe**:
    its premise was not `drag`'s ([measured and
    taken another way](#taken-up-on-2026-10-16)). `drag`'s frames after the
    keyframe carry rectangles, and the still 8×8s in them coded again at
    the dial until coded two to four times bought nothing: at the same
    quantizer their residual quantizes to nothing. Item 3 is taken by the
    weight of a bit at an anchored vector instead.
25. **A superblock's predictions kept.** One luma prediction a vector a
    superblock, with a mask of the 8×8s it holds, shared by the
    partition levels and the search between pixels (zvidlib's
    `InterBuffers`), so that `trial` reads the prediction `refine` made
    and does not predict the refined vector again, item 18's first line,
    which is taken ([measured](#taken-up-on-2026-10-15): the pass down
    alone from the patch the search filtered across, inside the tenth);
    and the search between pixels at a precision the block's residual at
    the whole vector picks (libaom's `subpel_select`), [measured and not
    taken](#measured-and-not-taken-1) on 2026-10-16: whole pixels where the
    residual is small cost a video 4 to 11% of its bytes and up to a
    decibel. SVT's one interpolation of a superblock's search area shared
    by its blocks is untried.

Of a very busy picture alone, and last:

17. **A settle of a busy picture takes up to twice libvpx's time**, more
   from 2048 wide than under it ([benchmarks.md](benchmarks.md), the run
   of 2026-10-15): from 65 `still`, `uncover` and `browser` 0.98 to 1.46,
   `flood` 0.77 to 1.72 and a picture of the video 1.04 to 1.61, 1.33 to
   1.61 from 2048 wide; from 80 and 89 `flood` 0.99 to 1.96 and the video
   0.87 to 1.52. The settle `walk`'s own session asks for, its clip paused
   partway up the dial at 41 to 75, is 0.86 to 1.21 of libvpx's time up to
   1920 wide and 1.16 to 1.57 from 2048, one frame's time, at 0.84 to 0.92
   of its bytes and +0.3 to +0.9 dB. libvpx's settle
   time read at two levels because the measurement made a fresh encoder a
   pass and its second frame paid page faults, or did not; on encoders
   kept warm it is 0.5 to 0.65 of what it read, and this is what is left.
   The time is in the transform sizes under the largest, which are the
   settle's 1.0 to 1.4 dB over libvpx's from 65; from 80 they buy 0.0 to
   0.2 dB. Untried: the smaller sizes only where the largest leaves more
   than some error, and what a settle from 80 or 89 needs of them at all:
   [measured and not taken](#measured-and-not-taken-1) on 2026-10-16, the
   smaller sizes buying a settle from 80 0.2 to 0.7 dB and from 89 up to
   0.1. The forward half of that time is taken above, with the picture kept.
18. **The frames of `video` and `walk`**, 0.85 to 1.01 and 0.71 to 0.81 of libvpx's time
    frame for frame, `video`'s most at 2048×1536 on the line at 1.01, the nearest the line of the whole captures. Of a
    video's time at 2048×1536 the search between pixels is a fifth and
    coding a refined block at both vectors a tenth
    ([measured](#measured-for-what-comes-next)). The refined vector's
    luma prediction is kept for the trial (item 25, taken, inside the
    tenth), and the trial at both vectors only where the residual says
    they are near is [measured and not taken](#measured-and-not-taken):
    at any fraction it trades the picture, and so does the quarter-pixel
    level at the one point or the axes the half-pixel costs point to, in
    place of the four around the best and a corner. The whole-pixel search
    is 4% of the video's coder ([profiled](#measured-for-what-comes-next)):
    the row hashes of an 8×8 row taken once and not twice, the
    projections' sums in sixteen bits and a table of its own are under a
    hundredth each, and left. What is left is the trial's body, which
    items 20 and 21 reshape.
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
vectors between pixels back and its frames' probabilities updated a video
is 1 to 9% over libvpx's bytes at a better picture, in 0.73 to 0.97 of
libvpx's time: at 2048×1536 the time is on the line, and what a video
costs is now the risk to the gate.
