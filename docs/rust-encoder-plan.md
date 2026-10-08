# A Rust VP9 encoder for screen-vp9

A plan for replacing the libvpx *encoder* behind `Encoder` with one written in
Rust, built for exactly what wlshare and the remotex gateway ask of this crate
and nothing more. Decoding stays with libvpx permanently.

## Decisions

- **Scope:** encode only. `Decoder` stays libvpx, which also makes libvpx the
  independent decoder every encoder test needs.
- **4:4:4 only.** The encoder codes 8-bit 4:4:4, VP9 profile 1, BT.601 at
  studio swing. There is no 4:2:0. At switch over `Chroma::Subsampled` leaves
  this crate, and wlshare and remotex drop their 4:2:0 stream with it: wlshare
  its `WLS0` encoding, remotex its `420` chroma and the browser's chroma answer
  that selected it. A browser with no software VP9 (iOS, iPadOS) refuses the
  stream by name at `VideoDecoder.configure`, as it refuses any configuration
  it lacks; that is accepted.
- **Location:** a module inside this crate, behind the existing `Encoder` API.
  It is meant for screen-vp9 only.
- **Branch:** the work is merged into `rust-encoder`, a long-lived branch off
  `main`, so that `main` keeps taking other changes and releases. `rust-encoder`
  takes `main` in as it moves and is merged into `main` once, at switch over.
- **No fallback:** when the Rust encoder passes the gate, the libvpx encoder
  path is deleted, not kept behind a feature. Until then the module is reachable
  only from tests and the benchmark.
- **Acceptance gate**, on each of the six 4:4:4 captures, at the capture's own
  quality and thread count:
  - median paired encode time at or below libvpx;
  - total bytes at most 15% above libvpx.

  PSNR is reported beside both and is not gated.
- **Benchmarking is interleaved.** Both encoders get the same frame back to
  back, the order alternating, and the result is the per-frame ratio, so load
  on the machine lands on both. See `examples/bench.rs`.

## What the two users need

Everything either user calls on an encoder, read from `remotex/src/vp9.rs` and
`wlshare/crates/wlshare-rfb/src/vp9.rs`:

- `Encoder::new(width, height, chroma, quality, threads)`, with `threads` the
  count each program picks for its machine.
- `encode(&picture, keyframe, &mut out)`: a keyframe when the flag asks for
  one, otherwise an inter frame against the previous frame. The return says
  whether the frame written is a keyframe; the Rust encoder always writes a
  frame, so the `None` for "libvpx produced nothing" goes with libvpx.
- `set_quality` and `quality()`: a fixed quantizer from the 1–100 dial, moved
  between frames without a keyframe. No rate control.
- `Picture`, which already converts RGB and `B, G, R, X` to the planes and
  stays as it is, less its 4:2:0 half.

Not needed, and not built: `Speed` and `set_speed`. Neither user calls them;
only the benchmark does, to name libvpx's two speeds. The Rust encoder has one
speed, and `Speed` is deleted at switch over.

Untouched by this work: `Decoder`, `frame_header`, `codec_string`, `walk`.

## Bitstream subset

The least VP9 that a decoder accepts and that the gate can be met with. Each
row's second column is built only if the first fails the gate on a capture.

| Area | Built | Only if the gate needs it |
|---|---|---|
| Profile and colour | Profile 1, 8-bit 4:4:4; the keyframe header says BT.601, studio swing | — |
| Reference frames | The previous frame only: the keyframe refreshes every slot, an inter frame refreshes one and reads it as `LAST` | Golden frame, compound |
| Block sizes | 64×64 down to 8×8, partition chosen by the encoder | 4×4 blocks |
| Intra modes | DC only, which keeps every transform a DCT | V, H and TM, which bring ADST with them |
| Inter modes | Zero, nearest, near and new motion vectors, whole-pixel only; the skip flag for a block that did not change | Sub-pixel motion and its filters |
| Transforms | One size per block, the largest that fits up to 32×32; no per-block size choice is signalled | Per-block size choice |
| Loop filter | Level 0, so the encoder needs none | A real filter, at low quality |
| Probabilities | Defaults every frame, no adaptation and no forward updates; the header says the decoder is not to adapt either | Backward adaptation |
| Tiles and threads | Tile columns by the width rule the crate has now, one thread per tile column | A wavefront over superblock rows |
| Segmentation, resizing, altref, lossless, high bit depth, error-resilient mode | Never | — |

## Where the speed should come from

- **Unchanged blocks:** compare each block with the previous source and code it
  as a skip without searching. A quality change makes every block dirty for one
  frame.
- **Scrolls and window drags:** hash-based whole-pixel motion search, which
  finds exact matches cheaply.
- **SIMD kernels:** AVX2 and NEON chosen at run time, each with a scalar
  reference it is tested against: block difference, forward and inverse
  transforms, quantization, DC prediction, block copy.
- **Threading:** one thread per tile column.

## Phases

1. **Harness.** Done: `examples/bench.rs`, interleaved timing of two
   candidates on a capture, with bytes and PSNR from a libvpx decode. Measured
   against itself first for its noise floor.
2. **Spike, scalar.** Keyframes and inter frames with skip, zero motion and DC
   intra. Every frame decoded by libvpx, the encoder's own reconstruction
   matching it bit for bit; checked in Chrome, Firefox and Safari. Measured on
   the six captures. Here is where to decide whether to go on: a scalar
   encoder that is not within about three times libvpx on the animation
   captures is unlikely to be brought under it by SIMD.
3. **Motion.** Motion vector prediction, hash search, partition decisions.
4. **SIMD and threading.**
5. **The second column**, row by row, only where a capture still fails the
   gate, and each kept only if it pays there.
6. **Switch over.** The libvpx encoder, `Chroma::Subsampled` and `Speed`
   removed behind the same API, a release tagged, the pins bumped in wlshare
   and remotex as they drop 4:2:0, and `AGENTS.md` reworded: the crate then
   encodes in Rust and speaks to libvpx only to decode.

## Test material

Six 4:4:4 IVF captures, kept out of git in `tmp/`. Where a machine keeps the
copies they come from is in that machine's `CLAUDE.local.md`, not here. The
directory's own `README.md` describes the files; the 4:2:0 captures beside them
are not used.

| Capture | Content | Size | Quality | Threads | Frames |
|---|---|---|---|---|---|
| `desktop-q100-444` | wlshare: terminal typing and scrolling, browser scroll, dragged window | 1440×900 | 100 | 4 | 665 |
| `proxmox-q100-444` | wlshare: full-frame shader animation | 1728×902 | 100 | 4 | 822 |
| `proxmox-hidpi-q100-444` | the same at HiDPI | 3456×1804 | 100 | 4 | 212 |
| `mac-hp-444` | remotex gateway from a Mac's HEVC | 1440×900 | 90 | 5 | 743 |
| `mac-recording-q100-444` | a Mac screen recording, as wlshare codes it | 3456×2234 | 100 | 4 | 1874 |
| `mac-recording-q90-444` | the same, as remotex codes it | 3456×2234 | 90 | 5 | 1874 |

The thread counts are what each program picks on the six-core machine the
baseline is measured on: wlshare the cores less two, remotex the cores less
one. The frames fed to an encoder are libvpx decodes of these captures, so
they are near the source, not the source. The two `mac-recording` streams
were made from a Mac screen recording kept beside them, an H.264 4:2:0
recording, so their colour is no finer than 4:2:0 either.

Beside each capture is a `.framemd5` of every decoded frame. That tests a
decoder, not an encoder, and is not used here.

## Reuse and licence

The bool coder, probability tables, scan orders, motion vector prediction,
inverse transforms and the DC predictor can be adapted from `rusty_vp9`, which
is Apache-2.0 and goes into this MIT crate with attribution. Mode decision,
motion search, forward transforms and all SIMD are new. `rivet-vp9` is
source-available only and none of it is used.

## Main risk

Every capture but `desktop-q100-444` is full-frame animation or video with
almost no cheap frames. There the cost is transform, quantization and entropy coding, which
libvpx already does with tuned AVX2 and NEON. Phase 2 is where that is
measured and the decision to go on is made.
