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
open items are in a new order for it.

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
  [encoder.md](encoder.md#bitstream-subset). Out for good: sub-pixel
  vectors and the interpolation filters, compound prediction, the golden
  and alternate frames, backward adaptation, lossless, rectangular
  partitions.
- **`Speed::Fastest` stays, out of the public API**: the intra mode by line
  sums and no 4×4s. Neither user calls it; it is work in progress for an
  optimization it may yet become, moved only by the tests and the
  measurements.
- **No backward compatibility**, in the API or in a stream's shape: a
  change reaches a user as a pin bump.

## How it is measured

- **Against libvpx, both encoders handed the same frame back to back, the
  order alternating**, on the `frame-captures` set of 2026-10-09: seven
  scenarios at ten sizes, each played as the session played it (the same
  rectangles, dial and keyframe asks).
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
  i5-8500T, which is x86-64-v3 and has no AVX-512: a floor above v3 has
  nothing here to be measured on, or to run its tests. The Mac
  runs the tests, for the NEON kernels; its own tables, in
  [benchmarks-m2-max.md](benchmarks-m2-max.md), are each line's median of
  three runs (`tools/median_of_runs.py`), its times moving by up to three
  fifths from one run to the next on a desktop in use.

## Where it stands

[benchmarks.md](benchmarks.md), one run of 2026-10-10 over every size:

- Over a whole capture, every one at every size takes less time than libvpx
  (0.14 to 0.82, frame for frame) at a picture level or better in 69 cells
  of 70 and at 0.12 to 1.10 of the bytes. A line of text is 0.14 to 0.19.
- A first picture coded alone as a keyframe is 0.2 to 0.6 dB over
  libvpx's at its bytes, in 0.53 to 0.92 of libvpx's time up to 1920 wide
  and 0.67 to 1.06 from 2048 wide.
- A settle of a busy picture takes more time than libvpx's: 1.12 to 1.77
  of it from 65, for 1.0 to 1.4 dB more picture at the same bytes, and
  0.93 to 1.30 from 89 for 0.3 to 0.4 dB more at fewer bytes.

On the M2 Max ([benchmarks-m2-max.md](benchmarks-m2-max.md), the median of
three runs of 2026-10-10) the lines are the same: a whole capture 0.19 to
0.74 of libvpx's time frame for frame, a picture level or better in 68
cells of 70; a keyframe alone 0.49 to 0.92 of its time, no cell over 1.00;
a settle of a busy picture 0.96 to 1.42 of it from 65.

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
| AVX2, sixteen `i16` or thirty-two `u8` a register | The kernels are AVX2 already, but the transforms and the quantizer in eight `i32`, the loop filter in an SSE register of eight `i16`, and the search's hashes and the intra predictions not at all. This is where the time is: the open items 1 to 4 below |
| `tzcnt`, `blsr`, `lzcnt`, `popcnt` | A compare's flags walked bit by bit: the search's anchors (item 3) |
| `shlx`, `shrx`, `mulx`, `andn`, `movbe` | The compiler's, taken already, above |
| `pext`, `pdep` | Not used: they are microcoded on Excavator and on Zen before Zen 3, eighteen cycles and up, and those are inside the floor |
| FMA, F16C | Nothing: the encoder has no floating point but a table of costs made once |

**What the floor has nothing for**: the token loops, each token's context
read from the two before it, and the bool coder, each bool's range from
the last. They are chains, and were made shorter by tile rows and by tables, not
by a wider register.

**Not raised further, to x86-64-v4.** AVX-512 would give the 16×16 and
32×32 transforms sixteen `i32` or thirty-two `i16` a register, masks for
the quantizer's dead zone and `vpcompressd` for nothing the token chain
can use. It is not measured, because the machine the tables are made on
cannot run it, and a floor is raised on a measurement. It also leaves out every Intel desktop
processor from Alder Lake on and every AMD one before Zen 4. It is taken
up when a host that has it is one this is deployed on. Between the two
levels there is no feature a codec reads: the `skylake` build above, which
adds the machine's own tuning, ADX, AES and CLMUL, measures the same.

**arm64's floor is where it was**, NEON. Its ceiling is gone the same way:
the dot product (`sdot`, `udot`, Armv8.2's, in every Apple M) is what the moments and the search's
hashes would take. Not measured, and not raised without it; open item 9.

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

The forward transforms are libvpx's to the bit as they were: the old ones
and these gave the same coefficients for every size and type over the
residuals that reach furthest (±255 signed as each of a transform's own
patterns, and as noise) and over noise, and the scalar reference panics on
a sum or a rounded value past 16 bits, which the tests hold the vectors to.
The 32×32 stays in 32 bits: its second pass leaves 16. NEON is the same
kernel in eight `i16` (`vmlal`, `vqrshrn`), its tests run on the Mac and
its time not measured. The quantizer on NEON reads the 16 bits and works in
32 as before.

The token map is held to coding a token at a time from the scan's own
neighbour tables, over every size, type and context. Costing the tokens
was 10 to 12% of the coder's time, nearly all of it in dense 4×4 and 8×8
blocks. On keyframes five of the seven captures are 0.91 to 0.96, and
`browser` and `terminal`, whose blocks are large with little in them, are
1.00 and 1.03, inside what this machine moves by: with the whole block
mapped whatever the scan reached, one or the other was 1.02 in each of
two runs.

## What is open

In the order it is taken up, set again on 2026-10-11 for the raised floor.
What a kernel can be rewritten for comes first now, because it changes no
stream: the tests hold the bytes, and the measurement is time alone. Each
is measured against the build before it turn about, as the floor was, and
taken only if it shows past the tenth this machine moves by on the frames
it is for. A very busy picture, the whole screen changing many times a
second, is the edge of a remote desktop and not its middle, so what is
only about one still comes last.

On the floor, no stream changed:

1. **The inverse transforms in sixteen `i16` a register.** The forward
   ones are, above, and so are the levels, which the token loops read
   through a map. The inverse has to stay the decoder's to the bit, and
   a decoder's wraps at 16 bits where libvpx's C does: the sums that
   leave 16 bits are found before it is believed. It is 0.9% of the
   coder's time on a keyframe, 1.3% on a settle and 1.9% on a whole
   capture, so it is not written until something else has made that
   more. The 32×32 forward, whose second pass leaves 16 bits, is in 32
   still.
2. **The loop filter in one and two columns**: measured, and it stays.
   Without it at 1280×800 (one column) and 1920×1080 (two) a whole
   capture is 0.2 to 0.7 dB of luma further from its source (`walk`
   alone not) at 0.93 to 1.05 of the bytes, and a keyframe that is not flat 0.25 to 0.3 dB
   at the same bytes (`walk` alone not), for 0.85 to 0.99 of a capture's time. The filter
   itself is two edges to an AVX2 register and passes a flat row of a
   superblock's edges at once, above: a flat picture's keyframe was a
   fifth in the filter finding each edge even, one at a time, and is
   0.87 to 0.89 of what it was. On a keyframe that is not flat the
   filter is still 4 to 11% of the time.
3. **The search's hashes**: written, above, as loops the compiler makes
   AVX2 and NEON of, and held to a window at a time by a test. The
   anchors of a 1920×1080 frame take one thread 3.1 ms where they took
   4.8 (`anchor_time`), which no frame of the captures shows: the
   hashing was 8% of the coder's time on `drag` and 3% on `walk` and
   `video`, and is spread over the threads. What is left of the search
   is filling the tables again (4% of `drag`) and the projections (5%).
4. **The intra predictions**: written, and they show on a keyframe.
   Choosing the mode was 7% of a run of keyframes and predicting 7%. The
   first added each 8×8's row and column sums into 64 bits a value, a
   plane at a time; they are 16 bits, and their products with the edge
   32. The second gathered its edges through calls for four bytes, and
   wrote a diagonal mode a pixel at a time, which the residual after it
   then read eight at once and waited on; each is now a line of means
   along the edge and every row a stretch of it, held to the pixel at a
   time by a test. Six of seven keyframes are 0.95 to 0.98 of the build
   before and `walk`'s the same; the settles and the captures are within
   the run's noise. Left: the quantizer on NEON in eight `i16`, as
   amd64's is in sixteen, with item 9.

Then, as before:

5. **Never measured, from the first plan's list**: the updates to the other
   probabilities on frames of a line of text, `INTRA_MODE_GAIN`, vectors ranked
   by their residual's squares on `browser` and `drag`, and `terminal` and
   `flood` captured at quality 100.
6. **The conversion is not the same on arm64 and amd64.** The `yuv`
   crate's NEON path makes a luma of 217 of an RGB grey of 235 where its
   AVX2 path makes 218. Which values and planes differ is not measured.
7. **The 4×4 split threshold**: what makes it safe at 10 is a diagonal mode
   for a 4×4 where the 8×8 would have taken one, and knowing why a churning
   picture costs more at a low dial with it.
8. **An intra block's rounding past 48/128**: 56/128 buys a keyframe
   another 0.4 dB for 0.5% more bytes at dial 65 and up, and costs `walk`
   3% of its capture for 0.11 dB and `drag`'s capture 0.06 dB. Why
   `drag` loses is not known; a rounding by block size or by frame kind
   is untried.
9. **arm64's floor**: the dot product for the moments and the hashes.
   No time is measured on arm64, where the tests alone are run, so a
   floor there has nothing to be raised on yet.
10. **The first keyframe of a stream runs on cold buffers** (130 ms against
    40 for the next at 1920×1080).
11. **A gate that fails a build.** The comparison is in the tests again, so
    it can be one; its lines are not decided, and a time on this machine
    moves by a tenth from run to run.
12. **`Speed::Fastest` was not measured in this replan.**
13. **x86-64-v4**, when a host with AVX-512 is one of the users': the
    measurement of the floor above, on it, with the 32×32 transforms
    written for it first.

Of a very busy picture alone, and last:

14. **A settle of a busy picture takes 1.1 to 1.8 of libvpx's time** from
   65 and from 80, more from 2048 wide than under it. libvpx's settle
   time read at two levels because the measurement made a fresh encoder a
   pass and its second frame paid page faults, or did not; on encoders
   kept warm it is 0.5 to 0.65 of what it read, and this is what is left.
   The time is in the transform sizes under the largest, which are the
   settle's 1.0 to 1.4 dB over libvpx's from 65; from 80 they buy 0.0 to
   0.2 dB. Untried: the smaller sizes only where the largest leaves more
   than some error, and what a settle from 80 or 89 needs of them at all.
   The forward half of that time is taken above, with the picture kept.
15. **The frames of `video` and `walk`**, 0.64 to 0.83 of libvpx's time, the
    nearest the line of the whole captures. Profiled: the token estimate,
    the block walk, the intra mode and the transforms 5 to 11% each, and
    the search 9 to 12%, now gone over: its projections, its hashing and
    its tables' upkeep 0.3 to 0.45 ms each of an 11 ms frame at 1920×1080,
    none of them one thing. What is left untried in it: the row hashes of
    an 8×8 row taken once and not twice, the projections' sums in sixteen
    bits, a table of its own in place of the standard one; a hundredth of
    a frame each, and item 3 takes the first two with it.
16. **The tile phase of a busy keyframe** runs four threads at 75 to 80% of
    four (27 ms of wall time for 86 of processor time at 1920×1080). No
    thread was found waiting; what is left is memory, and not looked into.

## Main risk

That the picture costs the time: each step toward libvpx's picture on text
(4×4 blocks, more modes for them) is paid for on keyframes, which are where
this encoder is level with libvpx and not ahead. The frame of rectangles is
where it is ahead by a half to two thirds, and it is the frame both users
hand it most; a change is weighed on that frame first.
