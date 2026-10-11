# The encoder of screen-vp9-native

How the VP9 encoder in this crate works: what it codes, how a frame goes
through it, how a block is decided, and what holds every stream it makes to a
decoder. Why it is this way, in the order it was built, is in
[plan.md](plan.md); the numbers it was decided on are in
[benchmarks.md](benchmarks.md).

## What it is

A VP9 encoder in Rust for a desktop picture, 8-bit 4:4:4 (profile 1, BT.601
at studio swing) and nothing else, built for what the wlshare daemon and the
remotex gateway ask of it. Three things shape everything in it:

- **Nearly every frame comes with rectangles.** A user says where the picture
  changed, and a frame costs those rectangles, not the picture.
- **The quantizer is pinned to a dial**, 1 to 100, not to a bitrate. There is
  no rate control, no lag and no dropped frame, and a keyframe only where one
  is asked. The dial moves between frames without a keyframe.
- **A decoder is the reference.** The encoder reconstructs every frame as a
  decoder does, to the bit, and codes the next against that.

Encoding only: there is no decoder in what a user builds. libvpx's decoder is
a dev-dependency (`src/decoder.rs`) that the tests hold every frame to.

## What a user holds

| Type | What it is |
|---|---|
| `Picture` | The BT.601 planes the encoder reads, converted from packed RGB or `B, G, R, X`, whole or a rectangle at a time, on up to four threads |
| `Encoder` | One stream at one picture size: `encode(picture, keyframe, changed, out)`, `settle`, `set_quality`, `coarsest`, `set_speed` |
| `Stream` | An `Encoder` and its `Picture` together, which is how both users drive it: read a packed picture where it changed, then code what was read since the last frame |
| `walk::QualityWalk` | The dial and frame rate a link will bear, and when a quiet picture is owed a settle |
| `frame_header`, `codec_string` | What a frame says about itself, and the WebCodecs string for a stream |

A **settle** is the whole picture coded once at the ceiling as an inter frame,
the dial left where the walk had it: what a desktop that went quiet while the
link had it coarse is owed. `coarsest()` says the coarsest dial any block a
decoder holds was last coded at, which is what the walk decides a settle by.

`Speed::Usual` takes every decision below; `Speed::Fastest` takes an intra
block's mode from its line sums alone and codes no 4×4 blocks.

## A frame, step by step

`Encoder::encode` in `src/encoder/mod.rs`. Each step that is spread over
threads is a queue of work the pool's threads take from, this one among
them. A frame that may code 1024 8×8s or fewer (`LIGHT_FRAME`) is coded on
this thread alone.

| Step | Where | What it does |
|---|---|---|
| Name | `Encoder::name` | Marks the 8×8s the rectangles touch as the ones the frame may code, and the superblocks they lie in, with the ring around them, as touched |
| Compare | `Encoder::compare` | Per 8×8: *still* where the source is the previous source, *unchanged* where it is still and was last coded at this dial. Copies the new source over the previous where the frame may code it |
| Search | `motion::Search::update` | Brings the hash index of the frame before up to date where 8×8s went stale, and gives each superblock the vectors it offers its blocks. A keyframe reads nothing of it and takes no anchors: the first frame after it that changes the picture does (`catch_up`), before it makes the changed rows the previous source (`keep`); a picture settled after a keyframe takes none and searches nothing |
| Tiles | `tile::code` | Decides every block and reconstructs it, a superblock row at a time; the bools go onto a tape per row, counted but not coded |
| Headers | `header`, `probs` | Chooses the probability updates from what the tapes count, the segmentation's coding, and the loop filter's level from the quantizer |
| Tapes and filter | `Encoder::finish` | Codes each tile column's tapes at the updated probabilities (`tape::code`), and runs the loop filter over the reconstruction, as a decoder does before it keeps the frame, over superblocks with an active block only. Neither waits on the other: a tile column and a plane are each a job of the one wake |
| Keep | `Encoder::merge` | Writes the touched superblocks back into the reference; a frame that touched most of them keeps its reconstruction as the reference and writes the rest into that, and one that touched every superblock swaps the two |

Afterwards every named 8×8 is marked as coded at the frame's dial.

## The modules

All under `src/encoder/`.

| Module | What is in it |
|---|---|
| `mod.rs` | The `Encoder`, the steps above, the reconstruction's layout in tile columns, and the tests with their measuring harness |
| `tile.rs` | One superblock row of a tile column: the partition, each block's mode, transform size and reconstruction |
| `motion.rs` | The whole-pixel motion search: hashed anchors for what moves exactly, projections for what does not |
| `mv.rs` | Motion vectors: the clamping of a predicted vector and the coding of a new one |
| `predict.rs` | Intra prediction as a decoder makes it, all ten modes |
| `transform.rs`, `dct.rs` | Forward and inverse transforms, DCT and ADST; `dct.rs` is generated from libvpx |
| `quantize.rs` | The fixed quantizer at libvpx's realtime roundings, an inter block's and an intra block's |
| `tokens.rs` | A transform block's levels as tokens: recorded, and costed beforehand |
| `tape.rs` | A row's bools and tokens as recorded, coded once the probabilities are chosen |
| `probs.rs` | The probabilities kept from frame to frame and the updates a header carries |
| `header.rs` | The uncompressed and compressed headers, and segmentation |
| `bool_writer.rs` | VP9's boolean coder |
| `loopfilter.rs` | The loop filter, ported from libvpx at its 4:4:4 path |
| `pixels.rs` | Kernels over pixels: moments, line sums, a residual |
| `lanes.rs` | Arithmetic over one value or a vector of them, so a kernel is written once |
| `pool.rs` | The threads, made once for the encoder's life |
| `tables.rs` | VP9's fixed tables, generated from libvpx |

## Bitstream subset

The least VP9 that a decoder accepts and that a desktop is coded well with.

| Area | Coded | Left out |
|---|---|---|
| Profile and colour | Profile 1, 8-bit 4:4:4; the keyframe header says BT.601, studio swing | 4:2:0, high bit depth |
| Reference frames | The previous frame only: a keyframe refreshes every slot, an inter frame refreshes one and reads it as `LAST` | Golden and altref frames, compound prediction |
| Segmentation | Every inter frame is in two segments. The inactive one skips (`SEG_LVL_SKIP`) and is not filtered (`SEG_LVL_ALT_LF` at −63); its map is coded whole or against the last frame's, whichever is fewer bits | Segments for anything else |
| Block sizes | 64×64 down to 8×8, square; an intra 8×8 of text as four 4×4s. An inter block is 32×32 at most | Rectangular partitions; 4×4 inter blocks |
| Intra modes | All ten. Blocks above 8×8 choose among DC, V, H and TM by line sums; an 8×8 and each 4×4 are coded each way (a 4×4 in DC, V, H and TM) | — |
| Inter modes | Zero, nearest, near and new vectors of whole pixels | Vectors between pixels, so no interpolation filter runs; the header names EIGHTTAP |
| Transforms | A size per block, 4×4 to 32×32, which the block signals; an intra block's luma below 32×32 in the DCT/ADST hybrid its mode pairs | Lossless (the WHT) |
| Loop filter | libvpx's level from the quantizer, in one and two tile columns; none in four or more | Per-segment or per-mode deltas beyond the inactive segment's |
| Probabilities | Coefficient, transform size and skip probabilities updated in each frame's header from the frame's own bools | Updates to the other probabilities; backward adaptation |
| Tiles | Tile columns by the width and the threads: one under 1440 wide, two from 1440, four from 2048, eight from 5760, never more than the threads. Tile rows, two or four, on a frame whose heaviest tile column recorded enough that its bools are worth a coder each: a tile row splits the bool coder and nothing else | |
| Frame | — | Resizing, error-resilient mode |

## How a block is decided

`tile.rs`. A superblock is measured once, per 8×8: the moments of its source
and, in an inter frame, of its source less the previous frame where it is. A
superblock the previous frame holds to the pixel is found by comparing the
two and is not measured at all. The partition tree is then walked from 64×64 down, and a square is coded
whole where `choose` finds a way to, and split where it does not.

In an inter frame a square is, in this order:

1. **Unchanged**, where every 8×8 of it is still and at this dial, or where
   the previous frame holds it exactly (its residual at the zero vector is
   nothing, at whatever dial it was coded). It goes in the inactive segment:
   a decoder reads its segment and nothing else, and does not filter it.
2. **Settling**, where its source is still but the dial moved: the previous
   frame where it is, with its residual, which is that frame's quantization
   error. No vector is searched. An intra mode is taken only where DC's
   residual is smaller.
3. **Moved or new**: measured at the zero vector, at the two vectors a
   decoder predicts for it, and at the vectors the search offers its
   superblock, ranked by the squares of the residual. A searched vector has
   to beat the predicted ones by a quarter (`NEW_VECTOR_GAIN`), since it
   costs its bits. An intra mode is taken where its residual beats the best
   vector's by more than its modes cost (`INTRA_BITS` at `RATE_WEIGHT`).

A square is split rather than coded whole where its residual varies by more
than a fraction of the quantizer's step (`SPLIT_VARIANCE`, `INTER_SPLIT`),
where part of it is unchanged and the rest did not move, and where it is a
64×64 predicted from the previous frame.

A keyframe's squares are intra. An 8×8 is coded in each mode in turn, by its
error on the coefficients and its bits at the frame's probabilities
(`intra_trial`), pruned as libvpx's realtime encoder prunes its own; one
whose four quarters' means differ is four 4×4s, each with a mode of its own
(`intra4_trial`); the chroma is in DC or in the luma's mode, by cost
(`chroma_trial`).

An inter block's transform size is tried from the largest down until a size
costs more than the one above it (`trial`), error and bits weighed at
`RATE_WEIGHT`. A transform block whose levels are not worth their bits codes
nothing (`worth_coding`). An intra block's size is by rule.

The motion search (`motion.rs`) is for what moves on a screen. Luma is hashed
over 8×8 windows; a window whose hash falls in one sixty-fourth of its range
is an anchor, so content that moves takes its anchors with it, and an anchor
found in both frames gives a vector. Hashes are retaken only where the source
changed. What moves but does not arrive exactly a superblock finds by its
projections: its rows' and columns' sums matched against the frame before's,
64 pixels each way.

## Where a frame's time goes

The rules the encoder keeps to, in the order the users' frames come:

- **A frame of rectangles costs the rectangles.** The comparison, the index,
  the reconstruction, the loop filter and the tapes are each bounded by the
  8×8s named. A superblock outside every rectangle and the ring around
  them is one partition symbol and its segment's bools, recorded without
  its 8×8s being looked at (`Tile::quiet`): no measure, no copy, no
  filter, and its mode info not written again once both of the frames
  kept hold it.
- **One reference, kept whole.** A frame writes only the superblocks it
  touches into its reconstruction, and writes those back once filtered.
- **One pool of threads for the encoder's life**, and nothing waits on a
  thread that has not woken. A phase's work is a queue: the caller takes
  from it too and is done when it is empty, a worker that wakes later has
  missed the phase, and a worker spins a tenth of a millisecond after a
  phase before it parks, so it is there for the next. A frame that may
  code little wakes none. A frame spawns nothing.
- **A settle costs what it sharpens.** What a decoder holds exactly is left
  alone, a superblock of it for the price of comparing its pixels; the rest
  is the zero vector and a residual, with no search.
- **What is left of a frame once its blocks are decided is done at once.**
  A tile column's tapes are coded by one thread, bool after bool, and the
  loop filter takes a plane to a thread; they share the threads rather than
  one waiting for the other.
- **Scrolls and drags are found by hash**, which on exact pixels is cheap and
  exact.
- **Tile columns are coded at once**, and within a column the superblock
  rows are a wavefront: a row codes a superblock once the row above has
  coded the one above it. The frame is the same on any number of threads.
- **SIMD is the baseline**: NEON on arm64, AVX2 on amd64, and the build is
  refused without them. Each kernel is written once over `lanes.rs` and
  tested against its scalar form, which nothing else runs on.

## What a stream is held to

- **libvpx's decoder, in every test.** Every frame a test makes is decoded,
  and the decode is held to the encoder's own reconstruction pixel for
  pixel. libvpx is Chrome's decoder.
- **ffvp9, Firefox's**, by hand: the measuring harness writes each capture's
  stream as IVF when `IVF` names a directory, one file per picture size, and
  `ffmpeg -c:v vp9 -f framemd5` of it should agree with `-c:v libvpx-vp9`.
- **Scalar, NEON and AVX2 code the same stream to the byte.** arm64 is run
  on a Mac; amd64 here.

The measuring harness is three ignored tests in `src/encoder/mod.rs` that
read the captures in `tmp/captures/` (or the directory `CAPTURES` names) and
code each with this encoder and with libvpx's, set up in `src/reference.rs`
as `screen-vp9` sets it up, turn about: `captures_against_libvpx` plays each
capture as its session played it, `keyframes_against_libvpx` codes each
capture's first picture at a row of dials, and `settles_against_libvpx`
settles it from one dial to another. Each wants a release build and an idle
machine; `ALONE` leaves libvpx out. `tools/tables.py` makes the tables of
[benchmarks.md](benchmarks.md) from their logs.

## Reuse and licence

What a decoder has to match exactly follows libvpx, the decoder the tests
check against: the bool coder, intra prediction, inverse transforms, the loop
filter and motion vector prediction are ported from its C, and every table
(probabilities, scan orders, quantizer steps) is generated from it by
`tools/vp9_tables.py`, and the one-dimensional transforms by
`tools/vp9_dct.py`, at the commit libvpx-prebuilt pins. libvpx is
BSD-3-Clause; its licence is in `LICENSE-libvpx`. Mode decision, motion
search, forward transforms and all SIMD are new. Neither `rusty_vp9` nor
`rivet-vp9` is used.
