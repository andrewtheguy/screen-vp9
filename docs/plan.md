# The plan of the encoder, and what was decided

What the encoder is for, what is decided about it, what was measured again
once its scope was final, and what is taken up next and in which order.
What the encoder is now is in [encoder.md](encoder.md), and the tables it
is measured by in [benchmarks.md](benchmarks.md). The phase-by-phase record
of how it was built on `screen-vp9`'s branch is in this file's history,
up to the commit that replaced it with this one.

Replanned on 2026-10-10, at release 0.0.1-alpha.2, with the use and the
scope final; the open items were taken up again the same day, after release
0.0.1-alpha.3.

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
- **Nothing a user builds links libvpx.** It is a dev-dependency: its
  decoder holds every frame a test makes, and its encoder, set up as
  `screen-vp9` 0.0.14 sets it up, is what the `*_against_libvpx`
  measurements play the captures into beside this one.
- **NEON on arm64 and AVX2 on amd64 are the baseline**, the build refused
  without them. The scalar lane is the reference the kernels are tested
  against.
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
- **`Speed::Fastest` stays in the API**, the intra mode by line sums and no
  4×4s. Neither user calls it.
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
- **The machine the tables are made on is the six-core AVX2 one.** The Mac
  runs the tests, for the NEON kernels, and no measurement longer than a
  minute.

## Where it stands

[benchmarks.md](benchmarks.md), one run of 2026-10-10 over every size:

- Over a whole capture, every one at every size takes less time than libvpx
  (0.13 to 0.83, frame for frame) at a picture level or better in 68 cells
  of 70 and at 0.12 to 1.09 of the bytes. A line of text is 0.13 to 0.19.
- A first picture coded alone as a keyframe is 0.55 to 0.93 of libvpx's
  time up to 1920 wide and 0.67 to 1.04 from 2048 wide.
- A settle of a busy picture from 65 is 0.82 to 1.21 of libvpx's time, for
  1.2 to 1.5 dB more picture, where libvpx's time is at the higher of the
  two levels it comes out at, and up to 1.37 where it is at the lower.
- A settle of one dial step takes up to 1.7 of libvpx's bytes, and buys
  picture with them at 0.7 to 1.0 of libvpx's decibels a byte.

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

The five rows before the last are release 0.0.1-alpha.4 and the last is
after it; none changes a stream.
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

## What is open

In the order it is taken up:

1. **libvpx's settle time is one of two levels a process**, 1.3 to 1.4
   apart (`uncover` at 2048×1536 from 65: 23.3, 17.3 and 23.4 ms in three
   runs, at 1600×1000 18.7, 19.2 and 14.6). Why is not known, and until it is a
   settle's ratio is good to no better than that.
2. **A keyframe at the top of the dial** is 0.05 to 0.23 dB under libvpx's
   at 89 and up to 0.28 under at 90, at the same bytes, where at 65 it is
   0.02 to 0.20 over (`walk`'s is over at every dial, at three quarters of
   the bytes). It is what a settle of one dial step is short of:
   the settle itself buys as much picture as libvpx's (0.11 to 0.27 dB
   against 0.13 to 0.28) at 0.7 to 1.0 of libvpx's decibels a byte, 1.6 on
   `walk`, so its 0.8 to 1.7 of libvpx's bytes are answered.
3. **The frames of `video` and `walk`**, 0.64 to 0.83 of libvpx's time, the
   nearest the line of the whole captures. Profiled: the token estimate,
   the block walk, the intra mode and the transforms 5 to 11% each, and
   the search 9 to 12%, now gone over: its projections, its hashing and
   its tables' upkeep 0.3 to 0.45 ms each of an 11 ms frame at 1920×1080,
   none of them one thing. What is left untried in it: the row hashes of
   an 8×8 row taken once and not twice, the projections' sums in sixteen
   bits, a table of its own in place of the standard one; a hundredth of
   a frame each.
4. **The 4×4 split threshold**: what makes it safe at 10 is a diagonal mode
   for a 4×4 where the 8×8 would have taken one, and knowing why a churning
   picture costs more at a low dial with it.
5. **The transforms run in 32-bit lanes** (`vpmulld` on AVX2), where
   libvpx's are 16-bit. Not measured.
6. **The tile phase of a busy keyframe** runs four threads at 75 to 80% of
   four (27 ms of wall time for 86 of processor time at 1920×1080). No
   thread was found waiting; what is left is memory, and not looked into.
   The first keyframe of a stream runs on cold buffers (130 ms against 40
   for the next at 1920×1080).
7. **A flat desktop settled from dial 20** takes 4 to 10 times libvpx's
   bytes: one or two kilobytes against a few hundred bytes.
8. **`Speed::Fastest` was not measured in this replan.**
9. **Never measured, from the first plan's list**: the updates to the other
   probabilities on frames of a line of text, the loop filter in one and
   two columns on `terminal` and `flood`, `INTRA_MODE_GAIN`, vectors ranked
   by their residual's squares on `browser` and `drag`, and `terminal` and
   `flood` captured at quality 100.
10. **A gate that fails a build.** The comparison is in the tests again, so
    it can be one; its lines are not decided, and a time on this machine
    moves by a tenth from run to run.
11. **The conversion is not the same on arm64 and amd64.** The `yuv`
    crate's NEON path makes a luma of 217 of an RGB grey of 235 where its
    AVX2 path makes 218. Which values and planes differ is not measured.
12. **The M2 Max's numbers** were better against libvpx than this
    machine's, and have not been made again since the fork.

## Main risk

That the picture costs the time: each step toward libvpx's picture on text
(4×4 blocks, more modes for them) is paid for on keyframes, which are where
this encoder is level with libvpx and not ahead. The frame of rectangles is
where it is ahead by a half to two thirds, and it is the frame both users
hand it most; a change is weighed on that frame first.
