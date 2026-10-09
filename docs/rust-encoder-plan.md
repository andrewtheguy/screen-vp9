# A Rust VP9 encoder for screen-vp9

A plan for replacing the libvpx *encoder* behind `Encoder` with one written in
Rust, built for exactly what wlshare and the remotex gateway ask of this crate
and nothing more. Decoding stays with libvpx permanently.

Replanned on 2026-10-09, after phases 1 to 5 were built, against the `main`
of each user as it stood that day: wlshare 0.0.61, remotex 0.0.376, vp9-wasm
0.0.7 and this crate's v0.0.12. The encoder is complete to its subset; what it
was measured on and tuned to was not a screen, and the frame both users hand
it most was built last and never measured. What stands, what was decided on
bad evidence and what is done next, in order, are in
[What was decided backwards](#what-was-decided-backwards),
[Where it stands](#where-it-stands-on-real-captures) and [Phases](#phases).

## Decisions

- **Scope:** encode only. `Decoder` stays libvpx, which also makes libvpx the
  independent decoder every encoder test needs.
- **4:4:4 only, kept.** The encoder codes 8-bit 4:4:4, VP9 profile 1, BT.601
  at studio swing. There is no 4:2:0, whatever else is replanned: one chroma
  is what keeps the encoder small. At switch over `Chroma::Subsampled` leaves
  this crate, and wlshare and remotex drop their 4:2:0 stream with it:
  wlshare its `WLS0` encoding, remotex its `420` chroma and the browser's
  chroma answer that selected it. A browser with no software VP9 (iOS,
  iPadOS) refuses the stream by name at `VideoDecoder.configure`, as it
  refuses any configuration it lacks; that is accepted.
- **Location:** a module inside this crate, behind the existing `Encoder` API
  and so behind `Stream`, which is what both users hold.
- **Branch:** the work is merged into `rust-encoder`, a long-lived branch off
  `main`, so that `main` keeps taking other changes and releases.
  `rust-encoder` takes `main` in as it moves and is merged into `main` once,
  at switch over.
- **No fallback:** when the Rust encoder passes the gate, the libvpx encoder
  path is deleted, not kept behind a feature. Until then the module is
  reachable only from tests.
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
- **Acceptance gate**, on the wlshare captures of
  [Test material](#test-material), each played into both encoders as the
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
  and `encode_rgb` (remotex, from its mirror), `set_quality`, `quality()` and
  `coarsest()`. `Stream` stays as it is over the Rust encoder, less its 4:2:0
  half.
- **Nearly every frame comes with rectangles.** wlshare hands the damage since
  the client's last frame, remotex the rectangles its mirror changed in since
  the round before. A frame is the whole picture only when it is a keyframe,
  the stream's first, a settle, or the one after a frame the encoder produced
  nothing for. What the rectangles cover depends on the desktop: a terminal
  typed into names 0.4% of a 1920×1080 picture a frame, a window dragged
  24%, and a page scrolled in Chromium, a video and a window uncovered name
  all of it, because the compositor reports the whole surface damaged. The
  Rust encoder always produces a frame.
- **A settle is a whole frame at the dial after the walk has moved it.** Both
  users walk the dial down on a slow link (`walk::QualityWalk`) and, once the
  picture has been quiet, encode the whole picture once at the ceiling, owed
  by `coarsest()`. Its source is unchanged almost everywhere; what it codes
  is a coarser frame's quantization error.
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

Not needed, and not built: `Speed` and `set_speed`. Neither user calls them;
only `examples/bench.rs` does, to name libvpx's two speeds. `Speed` is
deleted at switch over.

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

## Where it stands on real captures

Every capture of 2026-10-09 at the nine sizes, played into both encoders on four
threads on an Apple M2 Max by `captures_against_libvpx`, this encoder against
libvpx. Each cell is the Rust encoder's bytes over libvpx's, the median of the
per-frame time ratios, and its luma PSNR less libvpx's, over the whole
capture. The log is the harness's; the tables are made from it.

The captures at scale 1, over every frame:

| Capture | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 |
|---|---|---|---|---|
| `terminal` | 0.89 / 1.56 / −0.4 | 0.95 / 1.51 / −0.5 | 0.93 / 1.61 / −0.6 | 0.93 / 1.50 / −0.7 |
| `uncover` | 1.04 / 0.57 / −0.6 | 1.06 / 0.64 / −0.6 | 1.02 / 0.74 / −0.6 | 1.03 / 0.67 / −0.6 |
| `flood` | 0.90 / 0.57 / −0.4 | 0.80 / 0.49 / −0.3 | 0.86 / 0.53 / −0.4 | 0.94 / 0.55 / −0.4 |
| `browser` | 0.20 / 0.60 / −0.1 | 0.39 / 0.67 / −0.3 | 0.50 / 0.69 / −0.3 | 0.54 / 0.72 / −0.3 |
| `drag` | 0.15 / 0.72 / 0.0 | 0.16 / 0.82 / 0.0 | 0.17 / 0.84 / −0.1 | 0.13 / 0.87 / +0.4 |
| `video` | 1.01 / 0.73 / −0.1 | 1.01 / 0.75 / −0.2 | 1.00 / 0.79 / −0.2 | 0.98 / 0.93 / −0.1 |
| `walk` | 1.08 / 0.78 / −0.4 | 1.08 / 0.81 / −0.5 | 1.08 / 0.86 / −0.4 | 1.06 / 0.97 / −0.3 |

The captures at scale 2, over every frame:

| Capture | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3840×2160@2 |
|---|---|---|---|---|---|
| `terminal` | 0.83 / 1.14 / −0.4 | 0.84 / 0.99 / −0.5 | 0.82 / 1.00 / −0.6 | 0.92 / 0.91 / −0.3 | 0.88 / 0.90 / −0.5 |
| `uncover` | 1.07 / 0.58 / −0.3 | 1.08 / 0.53 / −0.4 | 1.09 / 0.58 / −0.5 | 1.07 / 0.51 / −0.5 | 1.07 / 0.52 / −0.6 |
| `flood` | 0.94 / 0.48 / −0.5 | 0.97 / 0.47 / −0.6 | 0.98 / 0.47 / −0.6 | 0.98 / 0.46 / −0.6 | 0.99 / 0.45 / −0.6 |
| `browser` | 0.37 / 0.68 / −0.3 | 0.17 / 0.62 / −0.1 | 0.18 / 0.67 / 0.0 | 0.20 / 0.66 / 0.0 | 0.16 / 0.68 / −0.1 |
| `drag` | 0.27 / 0.93 / −0.7 | 0.35 / 0.86 / −0.7 | 0.43 / 0.92 / −0.8 | 0.48 / 0.90 / −0.7 | 0.15 / 0.98 / +0.4 |
| `video` | 0.93 / 0.78 / 0.0 | 0.89 / 0.70 / 0.0 | 0.88 / 0.73 / 0.0 | 0.84 / 0.69 / 0.0 | 0.87 / 0.73 / 0.0 |
| `walk` | 0.99 / 0.83 / −0.1 | 0.93 / 0.72 / −0.1 | 0.95 / 0.78 / −0.1 | 0.94 / 0.75 / 0.0 | 0.92 / 0.76 / +0.1 |

The frames apart, libvpx → this encoder, as the median time per frame, the
mean bytes per frame, and the PSNR difference:

| Frames, scale 1 | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 |
|---|---|---|---|---|
| `terminal`, a line of text (≈ 260 a capture) | 0.63 → 0.99 ms, 119 → 152 B, −0.4 dB | 0.89 → 1.34 ms, 126 → 205 B, −0.5 dB | 0.91 → 1.46 ms, 121 → 179 B, −0.6 dB | 1.22 → 1.84 ms, 135 → 188 B, −0.7 dB |
| `terminal`, a scroll (7 or 8) | 5.1 → 4.7 ms, 40 → 34 kB, −0.2 dB | 5.2 → 4.4 ms, 46 → 41 kB, −0.3 dB | 5.6 → 5.0 ms, 65 → 58 kB, −0.6 dB | 6.2 → 4.5 ms, 62 → 55 kB, −0.4 dB |
| `drag`, a quarter of the picture (≈ 390) | 4.1 → 3.0 ms, 5750 → 635 B, 0.0 dB | 3.9 → 3.2 ms, 6469 → 752 B, 0.0 dB | 4.0 → 3.3 ms, 6209 → 769 B, −0.1 dB | 4.2 → 3.6 ms, 8944 → 812 B, +0.4 dB |
| `walk`, the settle (1) | 1.65 → 3.63 ms, 205 → 213 B, +0.1 dB | 2.16 → 4.53 ms, 252 → 433 B, 0.0 dB | 2.12 → 4.89 ms, 340 → 405 B, 0.0 dB | 2.31 → 4.68 ms, 200 → 398 B, 0.0 dB |

| Frames, scale 2 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3840×2160@2 |
|---|---|---|---|---|---|
| `terminal`, a line of text (≈ 260 a capture) | 1.57 → 1.80 ms, 203 → 265 B, −0.4 dB | 2.19 → 2.17 ms, 196 → 290 B, −0.5 dB | 2.68 → 2.70 ms, 191 → 392 B, −0.6 dB | 3.4 → 3.1 ms, 214 → 398 B, −0.3 dB | 4.1 → 3.7 ms, 222 → 431 B, −0.5 dB |
| `terminal`, a scroll (7 or 8) | 10.9 → 8.2 ms, 109 → 86 kB, −0.3 dB | 12.4 → 9.0 ms, 123 → 99 kB, −0.2 dB | 17.6 → 11.1 ms, 120 → 91 kB, −0.4 dB | 13.9 → 9.0 ms, 122 → 105 kB, −0.2 dB | 16.4 → 12.5 ms, 171 → 143 kB, −0.3 dB |
| `drag`, a quarter of the picture (≈ 390) | 8.5 → 7.9 ms, 7259 → 1609 B, −0.7 dB | 9.1 → 7.8 ms, 6985 → 2003 B, −0.7 dB | 9.3 → 8.6 ms, 6179 → 2272 B, −0.8 dB | 10.4 → 9.4 ms, 6064 → 2362 B, −0.7 dB | 10.1 → 10.0 ms, 26625 → 3080 B, +0.4 dB |
| `walk`, the settle (1) | 2.61 → 7.83 ms, 354 → 825 B, 0.0 dB | — | 4.5 → 13.2 ms, 492 → 1420 B, 0.0 dB | 5.6 → 13.0 ms, 504 → 1424 B, 0.0 dB | — |

| Keyframes of, scale 1 | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 |
|---|---|---|---|---|
| `uncover`, a page under a terminal | 25 → 14 ms, 1.05×, −0.8 dB | 21 → 13 ms, 1.04×, −0.8 dB | 25 → 16 ms, 1.04×, −0.8 dB | 32 → 20 ms, 1.04×, −0.8 dB |
| `drag`, a terminal of text | 17 → 9 ms, 1.04×, −0.7 dB | 14 → 9 ms, 1.03×, −0.7 dB | 15 → 9 ms, 1.03×, −0.7 dB | 21 → 12 ms, 1.00×, −0.7 dB |
| `flood` | 15 → 9 ms, 1.03×, −0.8 dB | 12 → 9 ms, 1.03×, −0.7 dB | 16 → 11 ms, 1.04×, −0.7 dB | 21 → 13 ms, 1.04×, −0.7 dB |
| `video` | 8 → 6 ms, 0.97×, −1.0 dB | 8 → 6 ms, 0.94×, −0.9 dB | 8 → 7 ms, 0.93×, −0.9 dB | 8 → 8 ms, 0.92×, −0.8 dB |

| Keyframes of, scale 2 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3840×2160@2 |
|---|---|---|---|---|---|
| `uncover`, a page under a terminal | 31 → 20 ms, 1.08×, −0.8 dB | 39 → 25 ms, 1.08×, −0.8 dB | 45 → 31 ms, 1.08×, −0.7 dB | 48 → 33 ms, 1.08×, −0.7 dB | 76 → 46 ms, 1.07×, −0.7 dB |
| `drag`, a terminal of text | 21 → 17 ms, 1.06×, −0.8 dB | 31 → 16 ms, 1.06×, −0.7 dB | 29 → 18 ms, 1.06×, −0.8 dB | 40 → 25 ms, 1.07×, −0.8 dB | 60 → 27 ms, 1.05×, −0.7 dB |
| `flood` | 19 → 13 ms, 1.08×, −0.8 dB | 30 → 16 ms, 1.06×, −0.8 dB | 24 → 18 ms, 1.06×, −0.8 dB | 34 → 25 ms, 1.07×, −0.8 dB | 70 → 30 ms, 1.07×, −0.8 dB |
| `video` | 10 → 10 ms, 0.95×, −0.7 dB | 13 → 12 ms, 0.93×, −0.6 dB | 15 → 14 ms, 0.92×, −0.7 dB | 18 → 17 ms, 0.89×, −0.6 dB | 20 → 20 ms, 0.91×, −0.7 dB |

After phase 7.1, the frames of a line of text and the drags again, the
rest of the tables unchanged:

| After 7.1, scale 1 | 1280×800 | 1440×900 | 1600×1000 | 1920×1080 |
|---|---|---|---|---|
| `terminal`, a line of text | 0.59 → 0.75 ms, 119 → 126 B, −0.4 dB | 0.80 → 1.00 ms, 126 → 155 B, −0.5 dB | 0.93 → 1.08 ms, 121 → 126 B, −0.6 dB | 1.12 → 1.28 ms, 135 → 138 B, −0.7 dB |
| `drag`, a quarter of the picture | 4.0 → 2.8 ms, 5750 → 622 B, −0.1 dB | 3.7 → 2.8 ms, 6469 → 695 B, −0.1 dB | 3.9 → 3.0 ms, 6209 → 709 B, −0.2 dB | 4.1 → 3.2 ms, 8944 → 767 B, +0.4 dB |

| After 7.1, scale 2 | 2048×1536@2 | 2560×1600@2 | 2880×1800@2 | 3200×2000@2 | 3840×2160@2 |
|---|---|---|---|---|---|
| `terminal`, a line of text | 1.51 → 1.76 ms, 203 → 190 B, −0.4 dB | 2.08 → 2.11 ms, 196 → 191 B, −0.5 dB | 2.53 → 2.60 ms, 191 → 230 B, −0.6 dB | 3.2 → 3.0 ms, 214 → 221 B, −0.3 dB | 4.1 → 3.7 ms, 222 → 224 B, −0.5 dB |
| `drag`, a quarter of the picture | 8.2 → 7.8 ms, 7259 → 1669 B, −0.7 dB | 8.9 → 7.7 ms, 6985 → 2042 B, −0.7 dB | 9.4 → 8.6 ms, 6179 → 2243 B, −0.8 dB | 10.6 → 9.4 ms, 5970 → 2325 B, −0.7 dB | 10.1 → 10.0 ms, 26625 → 3008 B, +0.4 dB |

What the real captures say, which the decoded samples could not:

- **Where the picture arrives exact and the compositor damages the whole
  surface — a page scrolled, a window dragged — this encoder makes a sixth
  to a third of libvpx's bytes at the same picture**, in two thirds of the
  time. libvpx is told by the active map that everything may have changed
  and searches for a scroll it finds poorly; the exact 8×8 comparison and
  the hash search find it whole. On a stream a slow link carries, and on
  what vp9-wasm has to parse, this is the largest thing the encoder has.
- **A frame of a line of text, the common frame, is where it loses.** In
  one and two tile columns, where the stream is loop filtered, it takes
  half as long again as libvpx (1.5 to 1.6× at 1280 to 1920 wide); in four,
  where neither filters, it is level to a tenth slower. At every size it spends more bytes
  on such a frame, from a quarter more at 1280×800 to nearly double at
  3840×2160: about a bit for each of the 2040 superblocks outside the
  rectangles, which each cost a partition, a skip, an intra-or-inter, a
  reference and a mode symbol, where libvpx's inactive segment costs its
  segment symbol. Profiled at 1280×800, the encoder's own time on these
  frames is a quarter spawning and joining threads, a quarter rebuilding
  the motion index, a quarter loop filtering the whole frame, and the rest
  the frame, with every unchanged block copied reference to reconstruction.
- **The picture is 0.4 to 0.8 dB under libvpx's at the same dial** on every
  capture of text or new content, and 0.7 to 1.0 dB on keyframes, where the
  bytes are level or fewer: the encoder is trading picture for bytes
  everywhere it is not finding motion. Where it finds motion (`browser`,
  `drag`, `video`) the picture is libvpx's or better.
- **Busy whole frames pass.** `flood`, the slowest content there is, codes
  in 0.45 to 0.57 of libvpx's time at 0.90 to 0.99 of its bytes; `video` at
  0.7 to 0.9 of the time and 0.87 to 1.0 of the bytes; keyframes in half
  libvpx's time from 1920 wide up. The whole-frame half of the first gate
  was the right half to have passed; it was passed at a coarser picture.
- **A settle costs twice libvpx's time and up to three times its bytes**,
  on a frame that should be nearly free: the blocks are unchanged and only the
  dial moved.
- **Nothing of what phase 5 added for soft pictures shows here yet**: the
  numbers above are with the sub-pixel search, EIGHTTAP_SMOOTH, the
  projection search and the 32×32 cap all in. Whether each pays on exact
  pixels is phase 9's question, not answered by this table.

## Bitstream subset

The least VP9 that a decoder accepts and that the gate can be met with. The
second column is kept; the third is built only where a real capture fails
the gate without it; the fourth is built already and kept only where a real
capture shows it paying, else deleted with its kernels.

| Area | Kept | Only if the gate needs it | Built, kept only where real captures pay |
|---|---|---|---|
| Profile and colour | Profile 1, 8-bit 4:4:4; the keyframe header says BT.601, studio swing | — | — |
| Reference frames | The previous frame only: the keyframe refreshes every slot, an inter frame refreshes one and reads it as `LAST` | Golden frame, if `uncover` or `drag` shows what a drag reveals coded intra at a cost worth a second reference | — |
| Segmentation | A frame told where the picture changed codes the blocks outside in a segment that skips and is not filtered, its map whole or temporal; a whole frame leaves segmentation off | — | — |
| Block sizes | 64×64 down to 8×8, partition chosen by the encoder | 4×4 blocks, for text in keyframes (19% of a desktop's bytes in libvpx) | An inter block 32×32 at most |
| Intra modes | DC, V, H and TM, costed from line sums; in an inter frame tried only where the previous frame does not code the square whole | The six diagonal modes | — |
| Inter modes | Zero, nearest, near and new vectors of whole pixels; the skip flag for a block that did not change | — | Quarter-pixel vectors, their search, `convolve` and EIGHTTAP_SMOOTH; the projection search |
| Transforms | A size per block, which it signals; an intra block's luma below 32×32 in the DCT and ADST its mode pairs | 4×4 wherever a block varies, for text | — |
| Loop filter | libvpx's level from the quantizer; none in four tile columns (`main`'s rule); in one and two, the edges of the blocks coded, which is what a decoder filters when the segment outside is at level 0 | — | The filter at all in one and two columns, against its bytes and dB on real captures |
| Probabilities | Coefficient, transform size and skip probabilities updated in each frame's header from the frame's own bools, kept for the next | Updates to the other probabilities | — |
| Tiles and threads | Tile columns by `main`'s rule, their superblock rows a wavefront over the threads; one pool for the encoder's life | — | — |
| Resizing, altref, compound, lossless, high bit depth, error-resilient mode | Never | — | — |

## Decided on the recordings, to measure again

Each of these was dropped, tuned or kept by a number taken on the Mac
screen recording or the gateway's decodes, where nothing arrived exactly,
flats were not flat and the picture had been filtered already. None of
those numbers holds on a capture. Each is measured again on the capture
named, in the phase named, and what the first plan dropped is built where
the capture says it pays.

| Decision | Decided on | Measured again on | Phase |
|---|---|---|---|
| 4×4 blocks not built: 19% of libvpx's bytes on the one desktop recording, judged not worth it | the recordings | keyframes and `small` frames of `terminal`, `drag`, `uncover`: text | 8 |
| The six diagonal intra modes not built; V, H and TM "bought nothing", then built and cut 1–10% | the recordings, then the samples | keyframes of text | 8 |
| The golden frame not built: 2–9% in libvpx on the recordings | the recordings | `uncover`, `drag`: what a drag reveals | 9 |
| An unchanged block at the zero vector left unfiltered "moved nothing" | the recordings, where no block is ever unchanged exactly | `terminal` `small` frames; subsumed by the segment at level 0 | 7 |
| The loop filter kept in one and two columns for 0.6–1.8 dB | the recordings, filtered twice before | `terminal` and `flood` at 1280×800 to 1920×1080: bytes, dB, encoder and decoder time; then `main`'s `loop_filtered` rule, measured with vp9-wasm on `vp9-samples.sh` streams | 7, then `main` |
| Sub-pixel vectors, searched where they pay (`SEARCH_PAYS`, `SEARCH_PROBE`) | proxmox and mac-hp recordings | `browser`, `drag`, `video`; deleted with `convolve` if the hashes find everything | 9 |
| EIGHTTAP_SMOOTH as the frame's one filter; the switchable per-block filter not built | the four quiet four-column samples: a shader, a halftone, reeded glass | `video`, `walk`; on text, the regular filter or none | 9 |
| The projection search, `REACH` 64 | the recordings, "what moves never arrives exactly" | `browser`, `drag`, `terminal` scrolls, which arrive exactly | 9 |
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

## Where the speed should come from

In the order the users' frames come:

- **A frame of rectangles costs the rectangles.** Everything per frame is
  bounded by the 8×8s named, not the picture: the comparison, the index
  retaken, the reconstruction written, the loop filter run, the tapes
  counted. A superblock outside every rectangle is one partition symbol and
  its segment's few bools on the tape, and nothing else: no measure, no
  moments, no copy, no filter.
- **One set of threads for the stream's life**, handed a frame's phases with
  one wake each; a frame spawns nothing.
- **Unchanged blocks:** compare each 8×8 with the previous source and code it
  as a skip without searching.
- **A settle is zero vectors with residuals.** Where the source is the
  previous source and only the dial moved, the block is the previous frame
  plus its quantization error at the new step: no candidates, no search, no
  intra trial.
- **Scrolls and window drags:** the hash search, which finds exact matches
  cheaply, and on exact pixels finds them.
- **Keyframes and new content:** the intra tools text wants, and partitions
  that follow edges rather than variance against a step tuned on soft
  pictures.
- **SIMD kernels** as built; **the wavefront** as built.

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
[Test material](#test-material), by `captures_against_libvpx`.

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
      segments are mixed costs its entropy on every block. libvpx and ffvp9
      decode every stream alike; vp9-wasm 0.0.7 (main at 4c847c7) does not:
      its second keyframe decodes wrong after a first keyframe of other
      bytes, with no segmented frame between, deterministically on one
      thread or four, which is its own defect and is left to it.
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
      everything.
   5. **The comparison, the `coded_at` scan, the tapes and the counts over
      the named 8×8s only**, in the bands and rows that hold them; a
      superblock outside the rectangles costs its symbols and no scratch.
8. **Quality back to libvpx's at the same dial.** Measured by the picture
   line of the gate, before any more speed on whole frames, since every byte
   measured against a coarser picture is a false byte. Diagnosed on
   keyframes first, where the gap is 0.7 to 1.0 dB at the same bytes on
   every capture and so is intra's; then on the inter frames of `flood` and
   `uncover`, where it is 0.4 to 0.6 dB and the bytes are level. Suspects,
   in order, with their rows of
   [Decided on the recordings](#decided-on-the-recordings-to-measure-again):
   1. The partition rule: a square is kept whole where its variance is under
      an eighth of a step, which was best on soft pictures and codes crisp
      text in 32×32 transforms. Retune on real captures, or decide by the
      edges the line sums already see.
   2. 4×4 transforms wherever a block varies, and 4×4 blocks, for text.
   3. `RATE_WEIGHT`, which trades coefficients for bits at a weight fitted
      to bytes alone; refit to the gate's PSNR.
   4. The intra modes: whether the six diagonal modes buy anything on text.
9. **Whole frames: keyframes, scrolls, settles.** Measured by the other time
   line of the gate on `flood`, `browser`, `video`, `walk` and `uncover`,
   with the rest of the rows of
   [Decided on the recordings](#decided-on-the-recordings-to-measure-again).
   1. The settle path: an unchanged 8×8 at a coarser `coded_at` codes as a
      zero vector with its residual, skipping candidates, search, intra
      trial and the transform size trial where the residual's size is below
      a step; a settle is then libvpx's two hundred bytes in less than its
      time, not twice both.
   2. The hash search on exact pixels: whether the projections, the
      sub-pixel refinement and `SEARCH_PAYS` still find anything the hashes
      miss on `browser`, `drag` and `video`; what finds nothing is deleted
      with its kernels (`convolve`, the search in `tile::refine`, the
      one-filter header field back to its default).
   3. The 32×32 cap on inter blocks, retried on real scrolls, where one
      vector for a 64×64 should be cheapest.
   4. The golden frame, only if `uncover` or `drag` shows it paying.
10. **Switch over.** The libvpx encoder, `Chroma::Subsampled` and `Speed`
    removed behind the same API, `AGENTS.md` reworded, a release tagged;
    then in wlshare and remotex: the pin bumped, 4:2:0 dropped,
    `-C target-feature=+avx2` set for their amd64 builds (a `.cargo/config.toml`
    or `RUSTFLAGS` in each container build), and wlshare's
    `scripts/vp9-samples.sh` run once so that vp9-wasm's benchmark decodes
    what the Rust encoder writes. The crate then encodes in Rust and speaks
    to libvpx only to decode.

## Test material

The captures are what a wlshare session handed `Stream`, written by its own
`--capture-frames` flag in the capture container, lossless, with the
rectangles each frame came with, `zstd` compressed: `<scenario>-<WxH>[@2].wlcap.zst`,
`@2` a desktop drawn at scale 2, as `docker/capture/README.md` beside them
says. Where this machine keeps them is in its `CLAUDE.local.md`, not here;
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

Sizes: the nine `capture-frames.sh` runs by default, which vp9-wasm's
benchmark has samples of: 1280×800, 1440×900, 1600×1000 and 1920×1080 at
scale 1, and the same four desktops at scale 2, 2560×1600, 2880×1800,
3200×2000 and 3840×2160, with 2048×1536. Each is measured at quality 90, the
dial the captures were made at, and at the dials the `walk` reaches, on four
threads and on five; `terminal` and `flood` once more at 100, the dial the
first plan measured wlshare at.

The eighteen decoded samples the first gate read, and the libvpx-coded
samples `vp9-samples.sh` makes for vp9-wasm, are not encoder material: the
first are decodes of decodes, the second are coded by the encoder under
test's rival. `streams_against_libvpx` still reads the former, as a check
that a stream decodes and matches, and decides nothing.

## Reuse and licence

What a decoder has to match exactly follows libvpx, the decoder the tests
check against: the bool coder, intra prediction, inverse transforms and motion
vector prediction are ported from its C, and every table (probabilities, scan
orders, quantizer steps) is generated from it by `tools/vp9_tables.py` at the
commit libvpx-prebuilt pins. libvpx is BSD-3-Clause; its licence is in
`LICENSE-libvpx`. Mode decision, motion search, forward transforms and all
SIMD are new. Neither `rusty_vp9` nor `rivet-vp9` is used.

## Main risk

That phase 8 costs the time phase 5 bought: libvpx codes text with 4×4 blocks
and ten intra modes, and matching its picture at the same dial may take the
whole-frame time back over its own. The rectangle half of the gate is where
this encoder can win by an order rather than a fraction, and is why it is
measured apart: an encoder that codes a keystroke in a tenth of libvpx's time
and a scroll in the same time is the one worth switching to, and one that
codes a scroll in half the time and a keystroke in the same is not.
