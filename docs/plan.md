# The plan of the encoder, and what was decided

The record of how the VP9 encoder in this crate was planned, built and
measured: the decisions, the evidence each was taken on, and the phases in
the order they were built. What the encoder is now is in
[encoder.md](encoder.md), and the tables it was measured by in
[benchmarks.md](benchmarks.md).

It began as a plan for replacing the libvpx *encoder* behind `screen-vp9`'s `Encoder` with
one written in Rust, built for exactly what wlshare and the remotex gateway
ask of that crate and nothing more, on a branch of that repository that was to
be merged once the encoder passed a gate against libvpx.

**That changed on 2026-10-09.** The work was forked into this repository as
`screen-vp9-native`, and everything of `screen-vp9` the encoder replaces was
deleted here: the libvpx encoder, `Chroma` and its 4:2:0 half, the decoder as
an API. `screen-vp9` stays as it is. Either user takes this crate in its place
when it chooses to, and a client that needs 4:2:0 is not cut off by a pin
bump on a crate both share. libvpx is a dev-dependency here: its decoder, in
`src/decoder.rs`, is what the tests hold every frame to, and nothing a user
builds links it.

What follows is the record as it was written on the branch, phase by phase,
and reads in its own tense: "this crate" is `screen-vp9`, `main` is its
`main`, and the comparisons with libvpx were made by a harness that played
each capture into both encoders (`captures_against_libvpx`,
`keyframes_against_libvpx`, `settles_against_libvpx`, `examples/bench.rs` and
`tools/gate_tables.py`), which is in this repository's history up to the
commit that removed the libvpx encoder. What is here now measures this
encoder alone, with libvpx decoding: `captures_measured`,
`keyframes_measured` and `settles_measured` in `src/encoder/mod.rs`. A
comparison with libvpx is made from `screen-vp9`, on the same captures.

Replanned on 2026-10-09, after phases 1 to 5 were built, against the `main`
of each user as it stood that day: wlshare 0.0.61, remotex 0.0.376, vp9-wasm
0.0.7 and this crate's v0.0.12. The encoder is complete to its subset; what it
was measured on and tuned to was not a screen, and the frame both users hand
it most was built last and never measured. What stands and what was decided on
bad evidence are in [What was decided backwards](#what-was-decided-backwards),
what was built and in which order in [Phases](#phases), and what is done
next in [What is open](#what-is-open).

## What is open

In the order it is taken up:

1. **The tokens of a busy frame.** A settle of a busy picture is now 0.85
   to 0.95 of what it took (phase 11), and what is left of it is the block
   coder: counting, recording and coding tokens is a third of its
   processor time, the transforms and the quantizer a fifth. The token
   loops run some 55 instructions a coefficient with nothing waiting on
   anything, so they are cut only by doing less a coefficient: the token
   and energy of each level taken once, a vector of them at a time, in the
   three kernels, for perhaps a tenth of a settle. Not begun. Seen on the
   way and not looked into: a keyframe of a flat desktop takes 13 ms at
   1920×1080 and 38 at 3840×2160 to make a few hundred bytes.
2. **The run over every size after phases 9 and 11**, made from
   `screen-vp9` for libvpx's half, so that the tables of
   [benchmarks.md](benchmarks.md) are the encoder's as it is and not phase
   8's.
3. **The rows of [Decided on the recordings](#decided-on-the-recordings-to-measure-again)
   that no phase records a measurement for**: the updates to the other
   probabilities on `small` frames, the loop filter in one and two columns
   on `terminal` and `flood`, the forward updates on the same two, intra
   prediction and the block copy in NEON and AVX2 on keyframes of text,
   `NEW_VECTOR_GAIN` and `INTRA_MODE_GAIN` on `terminal` and `drag`,
   vectors ranked by their residual's squares on `browser` and `drag`, the
   ten modes on a 4×4 retried (phase 8.4), and `terminal` and `flood`
   captured at quality 100.
4. **A gate that fails a build** (phase 6's last line). With no libvpx
   encoder here it cannot be the gate of [Decisions](#decisions) as
   written; what this repository can assert is this encoder's own bytes
   and PSNR per capture, which are the same on every run. Not decided.
5. **The conversion is not the same on arm64 and amd64.** The `yuv`
   crate's NEON path makes a luma of 217 of an RGB grey of 235 where its
   AVX2 path makes 218 (0.8.19, `YuvConversionMode::Balanced`, BT.601 at
   studio swing; the exact value is 217.8), so one desktop is a level of
   luma apart on the two before the encoder has read it. Found when
   `a_settle_leaves_what_a_decoder_holds_exactly` failed on arm64 alone,
   and seen at that one grey: which values and which planes differ is not
   measured, nor whether another conversion mode agrees across the two.
   The test now looks for its grey. Not looked into.

## Decisions

- **Scope:** encode only. Decoding is libvpx's, which is the independent
  decoder every encoder test needs: a dev-dependency since the fork.
- **4:4:4 only, kept.** The encoder codes 8-bit 4:4:4, VP9 profile 1, BT.601
  at studio swing. There is no 4:2:0, whatever else is replanned: one chroma
  is what keeps the encoder small. At switch over `Chroma::Subsampled` leaves
  this crate, and wlshare and remotex drop their 4:2:0 stream with it:
  wlshare its `WLS0` encoding, remotex its `420` chroma and the browser's
  chroma answer that selected it. A browser with no software VP9 (iOS,
  iPadOS) refuses the stream by name at `VideoDecoder.configure`, as it
  refuses any configuration it lacks; that is accepted.
- **Location:** this crate's `Encoder`, and so behind `Stream`, which is what
  both users hold. Until the fork it was a module of `screen-vp9` reachable
  only from tests, on its `rust-encoder` branch, which took `main` in as it
  moved, and since the fork it takes `screen-vp9`'s `main` in the same way:
  v0.0.14 (a stream reads a picture and then codes it, two calls) is the
  last taken.
- **No fallback:** there is no libvpx encoder here, behind a feature or
  otherwise. The fallback is the other crate.
- **SIMD is the baseline, not a choice: NEON on arm64 and AVX2 on amd64,**
  the build refused without them (`src/encoder/lanes.rs`). The scalar lane
  is the reference the kernels are tested against and nothing runs on it.
  Done, and kept. Neither user's build passes `-C target-feature=+avx2`
  today; that is a line in each at switch over.
- **A frame told where the picture changed is coded as libvpx codes its
  active map: in segments.** New. The blocks outside the rectangles are a
  segment that skips (`SEG_LVL_SKIP`) and is not loop filtered
  (`SEG_LVL_ALT_LF` at −63), its map coded whole or as the last frame's,
  whichever is fewer bits, as libvpx's `choose_segmap_coding_method` picks.
  Three reasons. vp9-wasm 0.0.7 was built to decode exactly this stream and
  remotex 0.0.376 pins it for it, so it is the shape a damage frame has in
  every decoder a user runs today. The loop filter then stops at the change
  by the bitstream's own rule, in a decoder and in this encoder alike, where
  a skip flag leaves every block edge of a static desktop filtered again
  each frame. And the blocks outside cost a decoder their parse and nothing
  else. The branch's plan had segmentation under "never"; that was written
  before `main` coded the active map. Built in phase 7.1: every inter frame
  is in two segments, the inactive one every block left as the previous
  frame has it, named or not.
- **The decoders a stream is held to are the browsers' own**: libvpx, which
  is Chrome's and this crate's `Decoder`, in every test; and ffvp9, which is
  Firefox's, through `ffmpeg -c:v vp9 -f framemd5` on the streams the
  harness writes with `IVF` set. vp9-wasm is checked the same way but is
  not the gate: it may change, and no browser runs it unless asked to.
  ffmpeg sets its output up at the first frame's size and rescales every
  later frame of another size to it before hashing, whichever decoder it
  ran, so a stream that changes size is hashed in pieces cut at the
  keyframe that changes it; the harness writes one file per size.
- **Acceptance gate**, on the wlshare captures of
  [Test material](benchmarks.md#test-material), each played into both encoders as the
  session played it into its stream (the same rectangles, dial and keyframe
  asks), at the threads each user picks on a six-core host (four for wlshare,
  five for remotex), read by `captures_against_libvpx` in
  `src/encoder/mod.rs`, per capture:
  - *picture*: luma PSNR of libvpx's decode of the stream within 0.3 dB of
    libvpx's own at the same dial. A stream coarser at the same dial is not
    at the same dial, and its bytes and time mean nothing;
  - *bytes*: at most 15% above libvpx's over the capture;
  - *time, rectangles*: over the frames handed rectangles within a sixteenth
    of the picture (`small` and `mid` in the harness), median paired encode
    time at or below half libvpx's, which takes them as its active map;
  - *time, the rest*: over keyframes, whole frames and frames of larger
    rectangles, median paired encode time at or below libvpx's;
  - *latency*: a `terminal` frame at 3840×2160 on four threads in 1 ms or
    under, a quarter of libvpx's 4.1 and of this encoder's 3.7 today. The
    rectangles' own work is a fraction of that; the rest is what phase 7
    removes;
  - *bytes, by kind*: the frames of one kind in a capture (`small`, `mid`,
    `large`, `whole`, `key`) at most 15% above libvpx's together, since the
    bytes a link sees while someone types are the small frames' alone, which
    the capture's total hides under its scrolls.

  Nothing is kept on the decoded samples again.
- **Benchmarking is interleaved.** Both encoders get the same frame back to
  back, the order alternating, and the result is the per-frame ratio, so load
  on the machine lands on both. `examples/bench.rs` times libvpx against
  itself and is the harness's noise floor.

## What the two users do now

Read from `wlshare/crates/wlshare-rfb/src/vp9.rs` and `crates/wlshare/src/session.rs`,
`remotex/src/vp9.rs` and `src/encode.rs`, at the releases above:

- Both hold a `Stream`, never an `Encoder`: `Stream::new(width, height,
  chroma, quality, threads)`, `encode_bgrx` (wlshare, from its framebuffer)
  and `encode_rgb` (remotex, from its mirror), `settle_bgrx` and `settle_rgb`,
  `set_quality`, `quality()` and `coarsest()`. `Stream` stays as it is over
  the Rust encoder, less its 4:2:0 half. A frame's rectangles are converted
  and not the rows across them, on up to four of the encoder's threads
  (`Picture::read_bgrx_rect`).
- **Nearly every frame comes with rectangles.** wlshare hands the damage since
  the client's last frame, remotex the rectangles its mirror changed in since
  the round before. A frame is the whole picture only when it is a keyframe,
  the stream's first, a settle, or the one after a frame the encoder produced
  nothing for. What the rectangles cover depends on the desktop: a terminal
  typed into names 0.4% of a 1920×1080 picture a frame, a window dragged
  24%, and a page scrolled in Chromium, a video and a window uncovered name
  all of it, because the compositor reports the whole surface damaged. The
  Rust encoder always produces a frame.
- **A settle is a whole frame at the ceiling, the dial left where the walk
  had it.** Both users walk the dial down on a slow link (`walk::QualityWalk`),
  tell the walk what each frame left the client holding (`sent`, of
  `coarsest()`) and, when it says a quiet picture is owed one (`settle_at`,
  half a second on), code the whole picture once at the ceiling:
  `Stream::settle_bgrx`, which the Rust encoder's `settle` is. Its source is
  unchanged almost everywhere; what it codes is a coarser frame's
  quantization error. The frames of the motion after it are at the walk's
  dial again, and code only what they are told changed.
- **Frames are paced.** remotex codes at most one round per 33 ms, doubled up
  on a link that is behind; wlshare codes when its client asks. The cost that
  matters is one frame's latency, not throughput.
- **Threads:** wlshare every core but two, remotex every core but one, each
  at most eight: four and five on the six-core hosts both were tuned on.
- **The dial:** 90 as both users' ceiling in practice, and the walk's steps
  below it on a slow link, which the `walk` capture reaches down to 20.
- **The captures are wlshare's own.** `wlshare --capture-frames DIR` writes
  what a session hands `Stream`, frame by frame and exact — size, dial,
  keyframe ask, rectangles and the rows they span — as the `.wlcap` file of
  `crates/wlshare-rfb/src/capture.rs`. `scripts/capture-frames.sh` runs the
  scripted desktop of `docker/capture/` (a headless sway and the daemon) at
  the nine sizes vp9-wasm's benchmark has samples of, seven scenarios each:
  quiet `terminal` and `uncover`, busy `flood`, `browser`, `drag`, `video`
  and `walk`. `vp9cap` plays a capture into the pinned encoder again, and
  `scripts/vp9-samples.sh` makes vp9-wasm's benchmark samples of the
  captures that way, named `<quiet|busy>-<WxH>-<columns>col-<lf|nolf>` from
  each stream's own keyframe header. A screen-vp9 pin bump is one run of
  that script and no desktop played again. remotex is not instrumented, and
  need not be: what the gateway codes as VP9 arrives exact too — RDP's
  bitmaps, a Mac in Standard mode, any VNC server over ZRLE — so a wlshare
  capture of the same kind of desktop at its sizes stands for it.
- **vp9-wasm 0.0.7 decodes segmentation**, since 0.0.6 refused every frame
  libvpx coded with an active map and the page asked for a keyframe each
  time. Its routine benchmark now decodes the samples `vp9-samples.sh`
  makes.

Kept, and given a meaning: `Speed` and `set_speed`. Neither user calls them
today, and they stay in the API for a day one does (asked for on
2026-10-09). The Rust encoder codes `Usual` with every decision phase 8
builds, and `Fastest` with the proxies phase 5 built in their place: the
intra mode by its line sums rather than coded each way, and later the
transform size by rule and no search between pixels. `Fastest` is measured
on the captures too, so that it is the faster of the two at every size.

Untouched by this work: `Decoder`, `frame_header`, `codec_string`, `walk`, and
the tile column and loop filter rules of `main` (`tile_columns_log2`,
`loop_filtered`), which the encoder reads and a decoder's threads were
measured for.

## What was decided backwards

The encoder was built in the order a video codec is built — keyframes,
inter frames, motion, SIMD, then features one by one against libvpx's bytes
— and the frame both users hand it most was added in the branch's last
commit, after every measurement. In detail:

- **The pictures were not screens.** Every sample the gate measured on was a
  libvpx decode of an IVF the remotex gateway wrote, coded again at quality
  90 and decoded again; the gateway's own picture of the largest was Apple's
  High Performance HEVC decoded by FFmpeg, and the Windows ones the RDP
  client's. Nothing in them moved exactly, flats were not flat and edges
  rang, and that is what the last steps of phase 5 answered: sub-pixel
  vectors, the projection search for "motion the hash search cannot match",
  EIGHTTAP_SMOOTH as the frame's one filter, an inter block no larger than
  32×32, and the loop filter's 0.6 to 1.8 dB, which is what a filter is worth
  on a picture already filtered twice. A High Performance Mac's HEVC is not
  coded as VP9 at all any more (remotex 0.0.372 passes it through). What
  reaches this crate is pixels as drawn.
- **The gate read the wrong thing.** Bytes and time at the same dial, PSNR
  beside them ungated, and the encoder passed it 1 to 4.4 dB under libvpx on
  those samples. The constants that tuned it (`SPLIT_VARIANCE`,
  `RATE_WEIGHT`, `NEW_VECTOR_GAIN`, `INTRA_MODE_GAIN`, `SEARCH_PAYS`) were
  fitted to bytes at a coarser picture.
- **The gate never handed a frame with rectangles**, which is nearly every
  frame either user hands one. The encoder now codes only the 8×8s it is
  told changed, but the rest of a frame still costs the picture. Profiled on
  the `terminal` capture at 1280×800, four threads, where a frame names a
  line of text: a quarter of the encoder's time is `std::thread::scope`
  spawning and joining threads, five or six times a frame (the comparison,
  the hash update and the projections, the tiles, the tapes, the filter); a
  quarter is `motion::Search::update`, which rebuilds the whole index of the
  frame before as soon as any 8×8 is stale; a quarter is the loop filter run
  over every edge of the frame; and every unchanged block is copied from the
  reference into the reconstruction. libvpx codes those frames in 0.63 ms,
  this encoder in 0.99; at 1920×1080, 1.22 and 1.84.
- **The direction of `main` since v0.0.7** is taken in part: tile columns
  from 1440 and 2048 wide (v0.0.8, v0.0.10), no loop filter in four 4:4:4
  columns (v0.0.9), the rows that changed and the 8×8s named (v0.0.11),
  `Stream` and `coarsest` (v0.0.12). What was not taken is what the active
  map is in the bitstream, segmentation, which the plan listed under "never"
  while vp9-wasm 0.0.7 was being built to decode it; and what the common
  frame being small means for where a frame's time goes.
- **The eighteen samples are obsolete twice over.** They were decodes of
  decodes, and wlshare now makes vp9-wasm's samples of its own captures with
  `vp9-samples.sh`. They measure nothing about a screen and no decision is
  taken on them again.

What stands from phases 1 to 5 and is kept: the bitstream subset and the
reconstruction matching libvpx's decoder bit for bit, the bool coder and the
tape, the forward probability updates, the transform size per block, the
exact 8×8 comparison and skip, the hash search for exact motion, the
wavefront over superblock rows and the tile columns, V, H and TM with the
ADST, and every kernel in NEON and AVX2.

## Decided on the recordings, to measure again

The subset of VP9 the encoder codes, and what of it is left out, is the
table in [encoder.md](encoder.md#bitstream-subset). Each of these was dropped, tuned or kept by a number taken on the Mac
screen recording or the gateway's decodes, where nothing arrived exactly,
flats were not flat and the picture had been filtered already. None of
those numbers holds on a capture. Each is measured again on the capture
named, in the phase named, and what the first plan dropped is built where
the capture says it pays.

| Decision | Decided on | Measured again on | Phase |
|---|---|---|---|
| 4×4 blocks not built: 19% of libvpx's bytes on the one desktop recording, judged not worth it | the recordings | keyframes and `small` frames of `terminal`, `drag`, `uncover`: text | 8 |
| The six diagonal intra modes not built; V, H and TM "bought nothing", then built and cut 1–10% | the recordings, then the samples | keyframes of text | 8 |
| The golden frame not built: 2–9% in libvpx on the recordings | the recordings | `uncover`, `drag`: what a drag reveals; stays out, `drag` codes 2% of its tokens intra and `uncover` a third, of which only what was seen before could come from a golden frame | 9 |
| An unchanged block at the zero vector left unfiltered "moved nothing" | the recordings, where no block is ever unchanged exactly | `terminal` `small` frames; subsumed by the segment at level 0 | 7 |
| The loop filter kept in one and two columns for 0.6–1.8 dB | the recordings, filtered twice before | `terminal` and `flood` at 1280×800 to 1920×1080: bytes, dB, encoder and decoder time; then `main`'s `loop_filtered` rule, measured with vp9-wasm on `vp9-samples.sh` streams | 7, then `main` |
| Sub-pixel vectors, searched where they pay (`SEARCH_PAYS`, `SEARCH_PROBE`) | proxmox and mac-hp recordings | `browser`, `drag`, `video`: a twentieth of a decibel on `video` and nothing else; deleted with `convolve` | 9 |
| EIGHTTAP_SMOOTH as the frame's one filter; the switchable per-block filter not built | the four quiet four-column samples: a shader, a halftone, reeded glass | no vector is between pixels any more, so no filter runs; the header names EIGHTTAP | 9 |
| The projection search, `REACH` 64 | the recordings, "what moves never arrives exactly" | kept: without it `video` takes 1.17× libvpx's bytes and `walk` 1.25×, `browser` 0.32× from 0.28× | 9 |
| An inter block 32×32 at most | the four-column quiet samples | `browser` and `drag` scrolls, where one 64×64 vector should be cheapest | 9 |
| Vectors ranked by the residual's squares | the busy samples | kept unless `browser` or `drag` loses by it | 9 |
| `SPLIT_VARIANCE`, an eighth of a step | keyframes of the recordings | keyframes of text | 8 |
| `RATE_WEIGHT`, 0.14 of a step squared | six recordings, by bytes alone | the picture line of the gate | 8 |
| `NEW_VECTOR_GAIN`, `INTRA_MODE_GAIN` | the recordings | `terminal`, `drag` | 8 |
| Updates to the other probabilities never sent | the first plan, unmeasured | `small` frames, whose partition, skip and mode symbols are most of the frame | 7 |
| Eighth-pixel vectors never | the first plan, unmeasured | stays never: nothing on a screen moves by an eighth; `video` is the one capture that could say otherwise | 9 |
| Tile columns from 1440 and 2048 wide, no filter in four | gateway decodes, by vp9-wasm's time | `main`'s rule, not this plan's; re-measured there on `vp9-samples.sh` streams | `main` |
| The first plan's first version had no loop filter at all ("level 0, so the encoder needs none; a real filter, needed at low quality"); one was built because it paid on the recordings | the recordings | `walk`, the one capture at the low dials the first plan reserved it for, beside the row above | 7 |
| Forward probability updates built (12–32% of the samples' bytes); backward adaptation never | the samples; and vp9-wasm, which counts symbols only in a frame that adapts, which none of this crate's does | forward updates kept and measured again on `flood` and `terminal`; backward adaptation stays out for the decoder's sake | 7 |
| Intra prediction and block copy left scalar, "not hot" | the recordings, where little was intra | keyframes of text in phase 8, where intra is the frame; NEON and AVX2 if they show | 8 |
| Lossless (the WHT) never; compound prediction never | the first plan | stay never: the dial's finest step is q 8, not lossless, and vp9-wasm refuses compound | — |
| wlshare measured at quality 100, the gateway at 90 | the first plan's six captures | `terminal` and `flood` captured once more with `--quality 100`, beside the 90 of every capture here | 6 |

## Phases

1. **Harness.** Done: `examples/bench.rs`, interleaved timing of two
   candidates on a capture, with bytes and PSNR from a libvpx decode.
2. **Spike, scalar.** Done: keyframes and inter frames with skip, zero motion
   and DC intra, every frame decoded by libvpx and the encoder's own
   reconstruction matching it bit for bit; checked in Chrome, Firefox and
   Safari.
3. **Motion.** Done: motion vector prediction, hash search, partition
   decisions.
4. **SIMD and threading.** Done: tile columns, bands, the wavefront; NEON and
   AVX2 for the transforms, quantizer, block difference, moments, the
   sub-pixel filter and the loop filter, each tested against its scalar
   reference; scalar, NEON and AVX2 code the same streams to the byte.
5. **The second column of the first plan.** Done, and measured on the
   decoded samples: forward probability updates, transform size per block,
   sub-pixel motion, the loop filter, the projection search, the wavefront,
   V, H and TM with the ADST, EIGHTTAP_SMOOTH, vectors ranked by their
   residual's squares, an inter block 32×32 at most. The numbers each was
   kept on are in this file's history and are not repeated: they were taken
   on pictures no user hands this crate.

The replan, from here. Each phase is measured on the captures of
[Test material](benchmarks.md#test-material), by `captures_against_libvpx`.

6. **Real captures and the gate that reads them.** Done as far as the
   material: wlshare's capture pipeline above, and this crate's harness,
   which reads a `.wlcap` from the layout its writer documents and plays it
   into both encoders as the session did, reporting per kind of frame (a
   keyframe; the whole picture; rectangles within a superblock, within a
   sixteenth of the picture, over more) bytes, median time, luma PSNR and
   the paired time ratio. Left: the captures at all nine sizes
   (`scripts/capture-frames.sh` with its default sizes) kept beside the
   earlier artifacts, and the gate asserted rather than printed once phases
   7 and 8 have landed, so that it fails a build.
7. **A frame costs its rectangles.** Measured by the rectangle half of the
   gate, the bytes-by-kind line and the latency line, on `terminal` and
   `drag` at every size. In the order of what the profile and the bytes say
   each is worth:
   1. **Segmentation for the frame's active map**, the segment outside at
      skip and level 0; the map coded whole or as the last frame's by cost,
      and the map a decoder keeps tracked here (the last segmented frame's,
      cleared by a keyframe). With it, the loop filter over the edges of the
      blocks coded and no other, which is what a decoder now filters: the
      masks built from the coded superblocks only. This is also where the
      small frames' bytes go: the symbols of the superblocks outside collapse
      to the segment's. Decoded by libvpx in the tests as every frame is.
      **Done.** Every inter frame is segmented, a block in the inactive
      segment where its source is the previous frame's (named or not); the
      map coded whole or against the last frame's by the counted cost, the
      inactive segment's features written once after each keyframe; the loop
      filter's masks built for superblocks with an active block only. On
      `terminal`'s frames of a line of text, bytes against libvpx went from
      1.28–2.05× to 0.94–1.22× (level at 1280, 1600, 1920, 2560, 3200, 3840;
      1.2× at 1440 and 2880, which are two columns), and the paired time
      from 1.5–1.6× to 1.13–1.26× in one and two columns, 0.9–1.0× in four;
      `drag` kept its bytes (0.09–0.39× of libvpx's) once the map was
      predicted, having lost 5–8% with it coded whole, since a map whose
      segments are mixed costs its entropy on every block. libvpx, ffvp9
      and vp9-wasm decode every stream alike, frame for frame. (A first
      reading had vp9-wasm wrong from the second keyframe of
      `terminal-1920x1080`; that keyframe changes the picture's size, and
      what differed was ffmpeg's rescaling after its decoders, not any
      decoder.)
   2. **One pool of threads for the encoder's life**, parked between frames,
      each phase one wake: the comparison, the index, the tiles, the tapes,
      the filter. The band and row scheduling as now. **Done**
      (`src/encoder/pool.rs`): the five phases each hand the pool one
      closure, as `std::thread::scope` was handed, and the bytes are the
      same to the byte. `terminal`'s frames of a line of text went from
      1.13–1.26× libvpx's time to 0.85–1.0× at every size (0.50 ms at
      1280×800, 3.5 ms at 3840×2160), its scrolls from 0.57–0.80× to
      0.47–0.77×, `drag` at 1920×1080 from 0.77× to 0.70×.
   3. **The index of the frame before kept, not rebuilt**: anchors retaken
      and the table updated for the 8×8s that went stale, and the
      projections only over changed superblocks, as now. **Done**: the
      tables keep each hash's places exactly up to `MAX_REPEATS`, a hash
      that once had more says nothing until the tables are filled again,
      which they are after a keyframe or when more than a quarter of the
      8×8s went stale. `terminal`'s frames of a line of text went from
      0.85–1.0× libvpx's time to 0.80–0.95× (3.40 ms at 3840×2160); the
      bytes moved by a few in a million where a spoiled hash had been a
      match.
   4. **One reconstruction kept across frames rather than two swapped**: the
      blocks this frame codes written in place, the pixels they overwrite
      kept aside for the blocks of this frame that read them as reference,
      a copy the size of the rectangles. A keyframe or a whole frame writes
      everything. **Done**, the other way about: the reference stays whole
      through the frame, so nothing is kept aside; the frame writes into
      its reconstruction only the superblocks it touches (one with a named
      8×8, or a neighbour of one, which intra prediction or the loop filter
      reaches into), and once filtered writes those back into the
      reference, a superblock row to a thread. A frame that touches every
      superblock swaps the two, as before.
   5. **The comparison, the `coded_at` scan, the tapes and the counts over
      the named 8×8s only**, in the bands and rows that hold them; a
      superblock outside the rectangles costs its symbols and no scratch.
      **Done** as far as the profile asked: a tape's token counts are
      cleared in place and summed only where it recorded a token, the
      mode info is not cleared between frames (every 8×8's is written
      before it is read), and the index's stale scan skips rows with no
      change in them or below. With 4 and 5 together, `terminal`'s frames
      of a line of text went from 0.80–0.95× libvpx's time to 0.25–0.51×:
      0.28 ms at 1280×800 and 1.00 ms at 3840×2160 on four threads, the
      plan's latency line met at the median. Bytes unchanged. What the 4K
      frame still spends: half in the tile coder over 2040 superblocks of
      which a dozen are coded (partition, segment, mode info and nonzero
      contexts written per superblock), a quarter in the index (the stale
      scan and a job per 8×8 row), a tenth coding the tapes on the one
      tile column's thread.
8. **Quality back to libvpx's at the same dial.** Measured by the picture
   line of the gate, before any more speed on whole frames, since every byte
   measured against a coarser picture is a false byte. Diagnosed on
   keyframes first, where the gap was 0.7 to 1.0 dB at the same bytes on
   every capture and so was intra's; then on the inter frames of `flood`
   and `uncover`, where it was 0.4 to 0.6 dB and the bytes were level.
   **Found, by `keyframes_against_libvpx` on the captures' first pictures
   at five dials and a block-by-block dump of both keyframes:** the gap was
   1.2 to 2.7 dB on a keyframe, all of it libvpx's keyframe path at its
   speed 7, which codes every block under 16×16 each way with real
   transform and token costs over all ten intra modes, and codes an 8×8
   whose four 4×4 means differ by a few grey levels (its `thresholds[3]`
   over `fill_variance_4x4avg`) as four 4×4 blocks with a mode each, which
   is what text is. libvpx at its speed 9, which has none of that, is level
   with what this encoder had. The partition thresholds were not it: an
   eighth of a step and a whole step code the same picture, and wider bands
   are worse. Built, in order, each measured on the 1920×1080 keyframes at
   dials 40 and 90:
   1. **The 8×8 intra block's mode by coding it each way** (`intra_trial`),
      the error on the coefficients and the bits at the frame's
      probabilities with the mode's own: +0.2 dB, 3% fewer bytes, half as
      long again.
   2. **The six diagonal modes**, predicted as libvpx's decoder predicts
      them with the row above read on to the right for a 4×4: +0.05 to
      +0.1 dB.
   3. **Four 4×4 blocks for an 8×8 whose quarters' means vary**
      (`intra4_trial`, each 4×4's mode in turn by coding it): from −1.2 to
      −2.7 dB at level bytes to −0.4 to −0.7 dB at 4 to 12% fewer bytes on
      the keyframes, and over the captures at 1920×1080: `terminal`'s lines
      of text −0.17 dB at 0.97× the bytes (from −0.71 at 1.02×), `uncover`
      −0.17 dB (from −0.6), `flood` −0.04 dB at 0.94× the bytes, `drag`
      +0.27 dB at 0.09×. Keyframes took 1.05 to 1.42× libvpx's time, which
      was over the line.
   4. **The trials pruned as libvpx prunes its own** (`rd_pick_intra_sby_mode`
      and `rd_pick_intra4x4block` at its speed 7): an 8×8 tries all ten
      modes less the steep diagonals whose way the best so far does not
      point (`conditional_skipintra`) and stops at a mode that codes no
      coefficient; a 4×4 tries DC, V and H alone (`intra_y_mode_mask`);
      the trial's last transform block is not reconstructed. Keyframes at
      1920×1080, dials 40 and 90: from 1.05–1.42× libvpx's time to
      0.77–0.93× (`video` and `walk` at dial 40, the two smallest
      keyframes, 1.04 and 1.10×), at −0.64 to −0.89 dB from −0.41 to −0.68,
      the bytes 0.91–1.00× of libvpx's. The profile of a dial-40 keyframe
      before the pruning, for the record: a third of the block coder in
      the token bit estimate, a tenth each in prediction and the forward
      transform, and a quarter of the frame's wall time coding the tapes.
      What was tried and not kept: DC, V and H alone on the 8×8s too is the
      same time and picture, so the 8×8's diagonals are free once pruned;
      the ten modes pruned on the 4×4s as well buy 0.1 to 0.2 dB (−0.46 to
      −0.73) for 15% of libvpx's keyframe time (0.95–1.11×), over the line
      at dial 40, and are the first thing to retry if the rest of this
      phase leaves time; DC, V, H and TM on the 4×4s is between (0.83–1.07×,
      −0.59 to −0.82).
   5. **The chroma mode by cost, DC or the luma's** (`chroma_trial`), on
      the 8×8s and 4×4s the luma trial runs on, each coded over both
      chroma planes as the luma trial codes a mode; larger blocks' chroma
      follows the luma. libvpx's realtime encoder codes a keyframe's
      chroma as DC alone (`intra_uv_mode_mask`); this encoder coded it in
      the luma's mode; and the keyframe harness, now printing the PSNR
      over all three planes beside luma's, showed each end wrong
      somewhere: DC alone took `walk`'s keyframe from 0.62× libvpx's bytes
      to 0.97×, the luma's mode left `uncover`'s chroma a decibel under
      (−1.85 dB over all planes at dial 90 against −0.89 in luma). By cost
      the keyframes at 1920×1080 are 0.90–0.99× libvpx's bytes at each
      dial (from 0.91–1.00×), `walk` 0.61×, `uncover` −0.89 dB over all
      planes, the luma PSNR unchanged, for 3 to 8% of the keyframe's time
      (0.80–1.00× libvpx's; `walk`'s small keyframe 1.09–1.11×). The
      trial on every intra block too was the same bytes and picture at 5%
      more time, so the gain is in the small blocks.
   6. **`RATE_WEIGHT` refit to the picture.** First the units put right:
      libvpx's `rdmult` on a keyframe is 4.35 × dc_quant², and over its
      `RDDIV` and the error's shift weighs a bit at 0.137 × dc_quant² of
      coefficient error; this encoder's 140 weighed one at 0.14 ×
      ac_quant², which is 1.4× libvpx's at dial 90 and 2× at dial 40, not
      13× lighter as this item said. Swept on the 1920×1080 keyframes at
      200, 140, 100, 70 and 50, and at 140 on dc_quant² (libvpx's form,
      which is 70 at dial 40 and 100 at dial 90 and measured as those):
      each halving buys about 0.3 dB for 1 to 3% of the bytes, and the
      trade had not turned at 50. Then on every 1920×1080 capture at 140,
      50, 35 and 25, where the weight also chooses an inter block's
      transform size and vector: `browser`'s and `drag`'s inter frames got
      fewer bytes *and* a better picture (browser −0.24 dB at 0.53× to
      +0.66 dB at 0.54×, drag +0.25 at 0.124× to +1.14 at 0.126×), the
      rest the keyframes' trade. **At 35**, over every frame: `browser`
      +0.66 dB at 0.54× libvpx's bytes, `drag` +1.14 at 0.13×, `flood`
      +0.12 at 0.94×, `video` +0.45 at 1.00×, `uncover` +0.08 at 1.007×,
      `walk` +0.23 at 1.09× (1.06× at 140), `terminal` −0.09 at 0.90×
      (its lines of text −0.08 at 0.99×); the time unchanged. 25 goes on
      the same way (terminal −0.07 at 0.90×, uncover +0.13 at 1.01×, walk
      +0.31 at 1.11×). The weight stays on the AC step: on the DC step it
      would be lighter still at the coarse dials `walk` reaches, where its
      bytes are already over.
   7. **The transform size of an 8×8 intra block**, measured: this
      encoder codes one with 4×4 transforms where its pixels deviate by
      more than two steps, libvpx keeps it at 8×8. On the 1920×1080
      keyframes at dial 40 the 4×4s are never taken; at dial 90 they save
      `walk`'s keyframe 3% of its bytes and 0.15 dB and `browser`'s 0.3%
      and 0.04 dB, the rest a few bytes. Kept. The same rule on a 16×16
      block changed nothing in any row and is gone. With 4 to 7 the
      keyframes at 1920×1080 stand at 0.96–1.01× libvpx's bytes and −0.14
      to −0.37 dB (`walk` +0.2 to +0.3 at 0.64–0.78×), 0.79–1.07× its time
      (`walk`'s small keyframe 1.13×).
   **Done**, measured on every size of the `frame-captures` set, the
   tables in [Where it stands](benchmarks.md): the
   picture is libvpx's or better at the same dial on every capture at
   every size over all frames but three cells a tenth to three tenths of
   a decibel under, the keyframes inside the captures at 0.9–1.0× its
   time and −0.0 to −0.2 dB, the frames of a line of text at 0.25–0.52×
   its time. `Fastest` skips the trials and the 4×4s.
9. **Whole frames: keyframes, scrolls, settles.** Measured by the other time
   line of the gate on `flood`, `browser`, `video`, `walk` and `uncover`,
   with the rest of the rows of
   [Decided on the recordings](#decided-on-the-recordings-to-measure-again).
   1. The settle path: an unchanged 8×8 at a coarser `coded_at` codes as a
      zero vector with its residual, skipping candidates, search, intra
      trial and the transform size trial where the residual's size is below
      a step; a settle is then libvpx's two hundred bytes in less than its
      time, not twice both. Measured by `settles_against_libvpx`, new: each
      capture's first picture coded as a keyframe at one dial and then
      whole at another, the picture unchanged, as a caller sharpens a
      quiet picture after the dial walked; and by `FRAMES=1` on the
      capture harness, which prints every frame on a line, from which
      `walk`'s frames are summed by dial. Before, at 1920×1080: settles
      from 20 and 65 to 90 took 1.3–3.8× libvpx's time at 0.77–1.3× its
      bytes, and `walk`'s one settle in the captures 2.3× its bytes at
      1.85× its time. Built, in order:
      - **Still 8×8s told apart from unchanged ones** (`still` beside
        `unchanged` in `compare`): the source is the previous source there,
        at whatever dial. The index and the projections go stale by
        `still`, so a settle re-hashes and projects nothing; a square still
        throughout settles in `choose` as the zero vector with its residual,
        no candidate measured and no search, and the sub-pixel search is
        not run on it. An intra mode is still taken where DC's residual is
        smaller, which from a coarse dial it is where the neighbours have
        settled already: without that, settles from 20 lost 0.4 dB and 10%
        of the bytes. Time from 1.3–3.8× libvpx's to 1.0–3.0×.
      - **An inter block predicted whole, past the frame's edge rounded up
        to 8** (`predict_inter`). A 32×32 transform block at the bottom of
        a 1080-high picture overhangs its 135 rows of 8×8s, and the rows it
        overhangs held whatever the row buffer held: the residual there
        was garbage, coded to the end of the scan at a thousand tokens a
        block, in every inter frame that coded the bottom row, and unseen by
        a decoder, which predicts those rows from its border. Found by
        counting a settle's tokens: `browser`'s settle of a flat desktop
        was 5952 bytes against libvpx's 53, two rows of 32×32 blocks coded
        whole; now 418 and exact. Settles from 89 to 90 went from 3–6×
        libvpx's bytes to 1.4–2.0×, from 65 from 1.12–1.34× to 1.03–1.10×.
      - **A transform block that is not worth its bits codes nothing**
        (`worth_coding`, in the inter trial and in every fresh plane): its
        levels' error and bits at `RATE_WEIGHT` against the error of
        leaving the residual whole and the bits of an empty block, as
        libvpx's realtime encoder skips a block whose residual costs more
        than it saves (`vp9_pick_inter_mode`). A settle's residual, the
        previous frame's quantization error, leaves a lone level deep in
        the scan of many a block, a hundred zero tokens for one
        coefficient. Weighed at libvpx's own weight on the DC step (137)
        it took `browser` and `drag` a decibel under for 3% of their
        bytes; at `RATE_WEIGHT` it costs them nothing and takes the
        settles from 80 to level bytes and picture.
      - **An intra block charged its modes against a vector**
        (`INTRA_BITS`): in an inter frame a square was coded intra
        wherever an intra mode's residual varied less than the inter
        residual, by a hair or not, though its modes cost several bits
        more than a vector a decoder predicts. Now the inter residual may
        vary by what `INTRA_BITS` bits are worth at `RATE_WEIGHT` more.
        Swept at 6, 12, 24 and 48 on the 1920×1080 captures: `walk`'s
        frames at dial 20 went from 1.68× libvpx's bytes to 1.52, 1.41,
        1.34 and 1.31 at a picture +0.2 dB throughout, its frames at 23
        from 1.86× at −0.23 dB to 1.46× at −0.13, the capture whole from
        1.087× to 1.054× at 24; `drag` gained a quarter of a decibel and
        `browser` lost nothing up to 24 and 0.15 dB at 48. 24 kept.
      With the three, settles at 1920×1080 against libvpx: from 20 to 90,
      0.86–1.03× the bytes at +0.1 to +0.5 dB and 1.0–1.2× the time; from
      65, 1.03–1.10× at +0.8 to +1.4 dB and 1.7–2.1×; from 80, 1.0–1.25×
      at ±0.1 dB and 1.9–2.3×; from 89, 1.4–2.0× at −0.3 dB and 2.4–2.6×.
      Over the 1920×1080 captures the picture is as it was (`browser`
      +0.75 dB at 0.27× the bytes, `drag` +0.89 at 0.12×, `flood` +0.23 at
      0.76×, `video` +0.42 at 0.995×, `walk` +0.20 at 1.09×, `uncover`
      −0.32 at 1.02×). What `walk`'s bytes are, found by summing its
      frames by dial: at dial 90 they are 1.007× libvpx's; at 20 to 30,
      1.5–1.9× at the same picture, where each frame codes some seven
      hundred intra 8×8s chosen over inter by residual variance alone,
      which an intra block's modes cost several bits more than a
      predicted vector; that, the settle's time, which is the transform
      trial and the rest of the block coder, and the one settle's 405
      bytes against 167, which is four 32×32 blocks where libvpx codes a
      64×64, are what follows.
   2. The hash search on exact pixels: whether the projections, the
      sub-pixel refinement and `SEARCH_PAYS` still find anything the hashes
      miss on `browser`, `drag` and `video`; what finds nothing is deleted
      with its kernels (`convolve`, the search in `tile::refine`, the
      one-filter header field back to its default). **Done.** Each turned
      off in turn on the 1920×1080 captures. Without the projections
      `video` took 1.17× libvpx's bytes (from 0.99×), `walk` 1.25× (from
      1.05×) and `browser` 0.32× (from 0.28×): the test pattern moves by
      what the hashes do not find, and the projections stay. Without the
      search between pixels: `video` 0.992× at −0.05 dB (from 0.993×),
      `walk` 1.059× at +0.17 dB (from 1.054× at +0.24), `browser` 0.269× at
      the same picture (from 0.277×), `drag` the same, and `video`'s time
      0.83× libvpx's from 0.89×. Nothing on a screen moves by a quarter
      pixel but the one test pattern, and that by a twentieth of a decibel:
      the search, its Laplacian model, `SEARCH_PAYS` and `SEARCH_PROBE`,
      `convolve` with its NEON and AVX2 kernels and the filter table are
      deleted, 760 lines, and the header names EIGHTTAP, which no block
      uses. Vectors are whole pixels coded in the eighths the bitstream
      counts in.
   3. The 32×32 cap on inter blocks, retried on real scrolls, where one
      vector for a 64×64 should be cheapest. **Measured, kept.** With a
      64×64 predicted whole, on the 1920×1080 captures `browser` went from
      0.277× libvpx's bytes to 0.259× but from +0.73 dB to +0.11, `drag`
      from 0.119× to 0.106× but from +0.93 dB to −0.09, `walk` from 1.054×
      to 1.043× at the same picture, `video` the same; the settles from 65
      to 90 from 1.04–1.09× to 1.01–1.07× at half a decibel less, and
      `walk`'s one quiet settle from 393 bytes to 278 (libvpx 167). A
      64×64 with one vector is cheapest where nothing in it differs, and
      that is the quiet settle alone; on a scroll the quarters it gets
      wrong cost the picture. Not worth the half decibel.
   4. The golden frame, only if `uncover` or `drag` shows it paying.
      **Measured, not built.** Counting the tokens of each capture's inter
      frames by the block's kind at 1920×1080: `drag` codes 2% of its
      tokens in intra blocks, `uncover` 33%, where the page the terminal
      uncovers is coded fresh. Only the part of that which was on screen
      before the terminal moved could be predicted from a golden frame,
      the page under the terminal at the start never having been seen, and
      `uncover` stands at 1.02× libvpx's bytes. A second reference for a
      part of a sixth of one capture's bytes is not worth its slot, its
      refresh rule and a second search.
   **Done**, measured at 1920×1080 alone: the run over every size was
   skipped at the user's call to close the phase, so the tables in
   [Where it stands](benchmarks.md) are phase 8's;
   `tools/gate_tables.py` now reads the `whole` frames and a settles
   section too, for the next run. At 1920×1080 after phase 9, over every
   frame: `browser` +0.74 dB at 0.27× libvpx's bytes (from +0.8 at 0.29×),
   `drag` +1.22 at 0.12× (from +1.1), `flood` +0.23 at 0.76×, `video`
   +0.36 at 0.99× at 0.83× its time (from 0.89×), `walk` +0.17 at 1.06×
   (from +0.3 at 1.09×), `uncover` −0.32 at 1.02×, `terminal` −0.15 at
   0.82×. Settles from dial 20 or 65: 0.86–1.10× libvpx's bytes at +0.1 to
   +1.4 dB; from 80, 1.0–1.25× at ±0.1 dB; from 89, 1.4–2.0× at −0.3 dB.
   Over the line: a settle's time, 1.0–1.2× libvpx's from dial 20 and
   1.7–2.6× from 65 on, which is the block coder on every block of the
   picture, the transform trial and the three planes, as a keyframe's is;
   and `walk`'s one quiet settle at 2.3× libvpx's bytes (382 against 167),
   four 32×32 blocks where libvpx codes a 64×64.
10. **The fork, in place of a switch over.** Done on 2026-10-09: the crate
    renamed `screen-vp9-native`; the libvpx encoder, `Chroma`, the decoder as
    an API, the walk's `stays_at` (a dial this encoder cannot refuse) and
    the harness that compared the two encoders deleted; `Speed` kept; libvpx
    a dev-dependency. What the API is now: `Encoder::new(width, height,
    quality, threads)` and `Stream::new` the same, `encode` and `settle`
    returning whether the frame is a keyframe (this encoder always makes
    one), `set_quality` infallible, `Picture::new(width, height, threads)`,
    `codec_string(w, h, fps)`, and an `Error` of six cases, none of them
    libvpx's. A `Stream` is read and then coded, as `screen-vp9`'s is from
    v0.0.14: `read_bgrx` or `read_rgb` with where the picture changed, then
    `encode`, which codes what was read since the last frame, or `settle`,
    which reads nothing. Left for a user that takes it: the dependency swapped, its
    4:2:0 stream dropped (wlshare's `WLS0`, remotex's `420` chroma and the
    browser's answer that selected it), its own decoder for its tests,
    `-C target-feature=+avx2` set for its amd64 builds (a
    `.cargo/config.toml` or `RUSTFLAGS` in each container build), and
    wlshare's `scripts/vp9-samples.sh` run once so that vp9-wasm's benchmark
    decodes what this encoder writes. First released as `v0.0.1-alpha.1`, a prerelease
    as every release of this repository is, by its `Release` workflow.
11. **A settle costs what it sharpens.** What phase 9 left over the line: a
    settle's time, and `walk`'s one quiet settle at 2.3× libvpx's bytes.
    Measured by `settles_measured` at 1920×1080, the tables in
    [benchmarks.md](benchmarks.md#settles-before-and-after-phase-11).
    Found by timing a settle's phases: a settle of a flat desktop from dial
    65, which a decoder holds exactly already, took 22.6 ms to make 418
    bytes, the block coder predicting, transforming and quantizing every
    32×32 of the picture in two sizes and three planes to find no
    coefficient, and the loop filter then run over every block's edge.
    - **A square the previous frame holds exactly is left as it is**
      (`choose`): where the source less the previous frame at the zero
      vector is nothing, the square is in the inactive segment, as the
      blocks a frame skips are, at the largest size that is so, a 64×64
      among them. There is no finer to code it, so the dial it was last
      coded at says nothing; a decoder reads the segment and no more, and
      does not filter it, which leaves it exact where the filter over a
      skipped block's edges did not always. **Done.** Flat settles from
      dial 65 up: 418 and 373 bytes to 61 and 57, 22.6 and 11.7 ms to 4.7
      and 4.0. Busy pictures by the part of them that is exact: `video`
      and `walk` 0.55–0.7× the time from dial 65 up, `drag` 0.65–0.85×,
      `flood` and `uncover` 0.75–1.1×, which is inside the noise of the
      machine they were timed on. Over the
      captures played whole nothing moved but `walk`'s quiet settle, 387
      bytes to 122, under libvpx's 167: the 64×64 that phase 9.3 found
      cheapest there and could not have without half a decibel elsewhere.
    - **The transform size of a settling block by rule, measured and not
      taken.** Phase 9.1 planned the trial skipped "where the residual's
      size is below a step". The largest size alone on every still block
      cost 1 to 2.4 dB from dials 20 to 80 (`flood` from 65: 48.02 dB to
      45.62 at 0.86× the bytes); stopping at the largest size where it
      codes nothing cost up to 0.6 dB for 1 to 40% of the bytes (`flood`
      from 80: 47.65 to 47.20 at 0.73×), which is a coarser settle and not
      a faster one. A settle
      is the frame that is owed the picture; the trial stays.
    - **A superblock the previous frame holds to the pixel is not
      measured** (`Tile::exact`). The exact squares above were found by
      measuring every 8×8 of them, source and residual, 192 calls a
      superblock to find nothing. The superblock's pixels are compared
      with the reference's first, and only what differs is measured.
      **Done.** A quiet settle at 1920×1080 from 4.2 ms to 3.1, and 0.66
      to 0.75 of its time at 2560 and 3840 wide; a frame after the dial
      moves pays the same less.
    - **The tapes and the loop filter in one wake** (`Encoder::finish`).
      A tile column's tapes are coded by one thread and the filter took a
      plane to a thread, one after the other, with threads idle in both.
      They read nothing of each other's, so a tile column and a plane are
      each a job and a thread takes the next as it finishes one. **Done.**
      Busy settles and keyframes in one and two tile columns 0.89 to 0.96
      of their time; from 2048 wide there is no filter and nothing moves.
    - **Three cuts to the token estimate, measured and not taken.** Timed
      by a settle's own processor cycles, which hold to a part in a
      hundred: contexts read from the levels with no scratch array between
      positions, a transform size given up as soon as it costs what the
      best does, and a zero among zeros costed without its tables. One to
      two parts in a hundred of the cycles together and nothing in the
      time; the last saved four in a hundred of the instructions and none
      of the cycles, its branch missing as often as it saved.
    Every frame is the same to the byte after these as before. The tables
    are in [benchmarks.md](benchmarks.md#settles-and-keyframes-the-rest-of-phase-11).
    Left: [What is open](#what-is-open), 1.

## Main risk

That phase 8 costs the time phase 5 bought: libvpx codes text with 4×4 blocks
and ten intra modes, and matching its picture at the same dial may take the
whole-frame time back over its own. The rectangle half of the gate is where
this encoder can win by an order rather than a fraction, and is why it is
measured apart: an encoder that codes a keystroke in a tenth of libvpx's time
and a scroll in the same time is the one worth switching to, and one that
codes a scroll in half the time and a keystroke in the same is not.
