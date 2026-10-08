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
- **Acceptance gate**, on each of the twelve 4:4:4 captures, at the capture's own
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
| Inter modes | Zero, nearest, near and new motion vectors to a quarter pixel, at EIGHTTAP, the frame's one filter; the skip flag for a block that did not change | Eighth-pixel vectors, a filter per block |
| Transforms | A size per block, which it signals: an inter block's the one whose luma costs least in error and bits, quantized at each; an intra block's 4×4 where it varies, otherwise the largest up to 16×16 | — |
| Loop filter | libvpx's filter at the level its real-time encoder takes from the quantizer, an intra block's one more by the deltas a decoder starts with | — |
| Probabilities | The coefficient, transform size and skip probabilities updated in each frame's header from the frame's own bools, and kept for the next; no backward adaptation, which libvpx's real-time encoder does not use either | Updates to the other probabilities |
| Tiles and threads | Tile columns by the width rule the crate has now, one thread per tile column; the source compared and hashed in a band of rows per thread | A wavefront over superblock rows |
| Segmentation, resizing, altref, lossless, high bit depth, error-resilient mode | Never | — |

## Where the speed should come from

- **Unchanged blocks:** compare each block with the previous source and code it
  as a skip without searching. A quality change makes every block dirty for one
  frame.
- **Scrolls and window drags:** hash-based whole-pixel motion search, which
  finds exact matches cheaply.
- **SIMD kernels:** NEON on arm64, where it is always there, and AVX2 on
  amd64, which the crate is built for (`-C target-feature=+avx2`; a build
  without it is refused), each with a scalar reference it is tested
  against: block difference, forward and inverse transforms, quantization.
- **Threading:** one thread per tile column, and the comparison with the
  previous source and the motion search's hashing in a band of rows per
  thread.

## Phases

1. **Harness.** Done: `examples/bench.rs`, interleaved timing of two
   candidates on a capture, with bytes and PSNR from a libvpx decode. Measured
   against itself first for its noise floor.
2. **Spike, scalar.** Done. Keyframes and inter frames with skip, zero motion and DC
   intra. Every frame decoded by libvpx, the encoder's own reconstruction
   matching it bit for bit; checked in Chrome, Firefox and Safari. Measured on
   the six captures. Here is where to decide whether to go on: a scalar
   encoder that is not within about three times libvpx on the animation
   captures is unlikely to be brought under it by SIMD.
3. **Motion.** Done: motion vector prediction, hash search, partition
   decisions.
4. **SIMD and threading**, in three parts, each measured on the six captures:
   1. **Threading.** Done: tile columns, and the comparison and hashing in
      bands of rows.
   2. **NEON.** Done: transforms, quantization, block difference and the
      moments the partition decision reads, each tested against its scalar
      reference. DC prediction and block copy are not hot and stay scalar.
   3. **AVX2.** Done, timed on an amd64 machine, which runs the whole test
      suite as well: the same kernels, a 4×4's in AVX2's lower half, with the crate
      built for AVX2 rather than choosing it at run time, so there is no
      scalar path on amd64. Scalar, NEON and AVX2 code the same streams to
      the byte, and the whole test suite passes on amd64.
5. **The second column**, row by row, only where a capture still fails the
   gate, and each kept only if it pays there. In the order of what each is
   worth to libvpx held to this subset, measured by switching libvpx's
   features off one at a time: forward probability updates (12–32% of the
   bytes) and per-block transform size (up to 28%), done; sub-pixel motion
   (up to 36% on video, nothing on desktop), done, its filter in NEON and
   AVX2: it cut the bytes 3–10% where it pays, for 30–45% more
   time there, and is searched only on frames it has been paying on. Then
   the loop filter (6–20%), done, eight lines at a time in NEON and SSE:
   it cut the bytes 4–15% on eleven of the twelve captures and 2% on
   desktop, and raised PSNR 0.6–1.8 dB, for 5–19% more time, each plane
   filtered on a thread of its own. Leaving a block at the zero vector
   unfiltered, so that an unchanged block is not filtered again frame after
   frame, moved nothing. Then the golden frame (2–9%) and 4×4 blocks (19%
   on desktop alone). V, H and TM bought nothing. Besides these, motion the
   hash search cannot match, because the captures are decodes and what
   moves in them never arrives exactly: each changed superblock's row and
   column sums matched against the frame before, 64 pixels each way, done.
   It cut desktop's bytes 23% and proxmox-hidpi's 9%, for 5–9% more time.
6. **Switch over.** The libvpx encoder, `Chroma::Subsampled` and `Speed`
   removed behind the same API, a release tagged, the pins bumped in wlshare
   and remotex as they drop 4:2:0, and `AGENTS.md` reworded: the crate then
   encodes in Rust and speaks to libvpx only to decode.

## Test material

Twelve 4:4:4 IVF captures, kept out of git in `tmp/`. Where a machine keeps the
copies they come from is in that machine's `CLAUDE.local.md`, not here. The
directory's own `README.md` describes the files; the 4:2:0 captures beside them
are not used, and neither are the two made from a Mac screen recording, whose
colour was never finer than 4:2:0 and which neither program ever sent.

| Capture | Content | Size | Quality | Threads | Frames |
|---|---|---|---|---|---|
| `desktop-q100-444` | wlshare: terminal typing and scrolling, browser scroll, dragged window | 1440×900 | 100 | 4 | 665 |
| `proxmox-q100-444` | wlshare: full-frame shader animation | 1728×902 | 100 | 4 | 822 |
| `proxmox-hidpi-q100-444` | the same at HiDPI | 3456×1804 | 100 | 4 | 212 |
| `mac-hp-444` | remotex gateway from a Mac's HEVC | 1440×900 | 90 | 5 | 743 |
| `windows-elite-…-3456x2168-444` | remotex gateway from RDP: a shader animation, a news page scrolled, a terminal printing, a documentation page, side by side, at HiDPI | 3456×2168 | 90 | 5 | 887 |
| `windows-elite-…-1920x1080-444` | the same desktop | 1920×1080 | 90 | 5 | 1943 |
| `windows-elite-…-1600x1000-444` | the same desktop | 1600×1000 | 90 | 5 | 2333 |
| `windows-elite-…-1440x900-444` | the same desktop | 1440×900 | 90 | 5 | 1797 |
| `windows-elite-…-1280x800-444` | the same desktop | 1280×800 | 90 | 5 | 1898 |
| `macvmhighperf-…-3840x2160-444` | remotex gateway from a virtual Mac, the same four panes, at HiDPI | 3840×2160 | 90 | 5 | 1443 |
| `macvmhighperf-…-2560x1600-444` | the same Mac, at HiDPI | 2560×1600 | 90 | 5 | 1986 |
| `macvmhighperf-…-2880x1800-444` | the same Mac, at HiDPI | 2880×1800 | 90 | 5 | 2245 |

The thread counts are what each program picks on the six-core machine the
baseline is measured on: wlshare the cores less two, remotex the cores less
one. The frames fed to an encoder are libvpx decodes of these captures, so
they are near the source, not the source. The last eight were written by the
gateway as it coded them, each its own session; their full names carry the
session's start, and the first frame of each is black, the desktop not yet
painted. The 3456×2168 one's quality fell as low as 80 late in the session,
past the frames measured.

Beside each capture is a `.framemd5` of every decoded frame. That tests a
decoder, not an encoder, and is not used here.

## Reuse and licence

What a decoder has to match exactly follows libvpx, the decoder the tests
check against: the bool coder, intra prediction, inverse transforms and motion
vector prediction are ported from its C, and every table (probabilities, scan
orders, quantizer steps) is generated from it by `tools/vp9_tables.py` at the
commit libvpx-prebuilt pins. libvpx is BSD-3-Clause; its licence is in
`LICENSE-libvpx`. Mode decision, motion search, forward transforms and all
SIMD are new. Neither `rusty_vp9` nor `rivet-vp9` is used.

## Main risk

Every capture but `desktop-q100-444` is full-frame animation or video with
almost no cheap frames. There the cost is transform, quantization and entropy coding, which
libvpx already does with tuned AVX2 and NEON. Phase 2 is where that is
measured and the decision to go on is made.
