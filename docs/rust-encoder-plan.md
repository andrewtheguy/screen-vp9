# A Rust VP9 encoder for screen-vp9

A plan for replacing the libvpx *encoder* behind `Encoder` with one written in
Rust, built only for what wlshare and the remotex gateway ask of this crate.
Decoding stays with libvpx permanently.

## Decisions

- **Scope:** encode only. `Decoder` stays libvpx, which also makes libvpx the
  independent decoder every encoder test needs.
- **Location:** a module inside this crate, behind the existing `Encoder` API.
  It is meant for screen-vp9 only.
- **No fallback:** when the Rust encoder passes the gate, the libvpx encoder
  path is deleted, not kept behind a feature. Until then the module is reachable
  only from tests and the benchmark.
- **Acceptance gate**, on each of the six captures, at the same quality setting:
  - median paired encode time at or below libvpx;
  - total bytes at most 15% above libvpx.

  PSNR is reported beside both and is not gated.
- **Benchmarking is interleaved.** Both encoders get the same frame back to
  back, the order alternating, and the result is the per-frame ratio, so load
  on the machine lands on both. See `examples/bench.rs`.

## What the two users need

Both use the same narrow surface: `Encoder::new`, `encode` with a
keyframe-on-request flag, `set_quality`, `set_speed` and `Picture`.

- 8-bit 4:2:0 (profile 0) and 4:4:4 (profile 1), BT.601 at studio swing.
- A keyframe only when asked; otherwise inter frames against the previous frame.
- A fixed quantizer from the 1–100 dial, changeable between frames. No rate
  control.
- Tile columns, for threading.

## Bitstream subset

| Area | First version | Deferred |
|---|---|---|
| Reference frames | Previous frame only, single reference | Golden frame, compound |
| Block sizes | 64×64 down to 8×8 | 4×4 blocks |
| Intra modes | DC, V, H, TM — which needs ADST as well as DCT, since VP9 ties the transform type to the intra mode | Directional modes |
| Inter modes | Zero, nearest, near and new motion vectors, whole-pixel only | Sub-pixel motion and its filters |
| Transforms | 4×4 to 32×32 | Lossless |
| Loop filter | Level 0, so the encoder needs none | A real filter, needed at low quality |
| Probabilities | Defaults, no adaptation | Backward adaptation or forward updates |
| Segmentation, resizing, altref | Never | — |

## Where the speed should come from

- **Unchanged blocks:** compare each block with the previous source and code it
  as a skip without searching. A quality change makes every block dirty for one
  frame.
- **Scrolls and window drags:** hash-based whole-pixel motion search, which
  finds exact matches cheaply.
- **SIMD kernels:** AVX2 and NEON chosen at run time, each with a scalar
  reference it is tested against: block difference, forward and inverse
  transforms, quantization, intra prediction, block copy.
- **Threading:** one thread per tile column for entropy coding, mode decision
  as a wavefront over superblock rows.

## Phases

1. **Harness.** `examples/bench.rs`: interleaved timing of two candidates on a
   capture, with bytes and PSNR from a libvpx decode. Measured against itself
   first for its noise floor.
2. **Spike, scalar.** Keyframes and inter frames with skip, zero motion and the
   four intra modes. Every frame decoded by libvpx, the encoder's own
   reconstruction matching it bit for bit; checked in Chrome, Firefox and
   Safari. Measured on the six captures.
3. **Motion.** Motion vector prediction, hash search, partition decisions.
4. **SIMD and threading.**
5. **Compression.** Sub-pixel motion, probability adaptation, loop filter,
   transform size choice — each kept only if it pays on the captures.
6. **Switch over.** The libvpx encoder replaced behind the same API, a release
   tagged, the pins bumped in wlshare and remotex, and `AGENTS.md` reworded:
   the crate then encodes in Rust and speaks to libvpx only to decode.

## Test material

Six IVF captures, kept out of git in `tmp/` (copies in
`/mnt/dasdata/backup/screen-vp9-capture`):

| Capture | Content | Size, chroma, quality | Threads |
|---|---|---|---|
| `desktop-q100-444` | wlshare: terminal typing and scrolling, browser scroll, dragged window | 1440×900, 4:4:4, 100 | 4 |
| `proxmox-q100-444` | wlshare: full-frame shader animation | 1728×902, 4:4:4, 100 | 4 |
| `proxmox-hidpi-q100-444` | the same at HiDPI | 3456×1804, 4:4:4, 100 | 4 |
| `windows-rdp-420` | remotex gateway from RDP: animation | 1440×900, 4:2:0, 90 | 5 |
| `mac-hp-420` | remotex gateway from a Mac's HEVC | 1440×900, 4:2:0, 90 | 5 |
| `mac-hp-444` | the same at 4:4:4 | 1440×900, 4:4:4, 90 | 5 |

The thread counts are what each program picks on the six-core machine the
baseline is measured on. The frames fed to an encoder are libvpx decodes of
these captures, so they are near the source, not the source.

## Reuse and licence

The bool coder, probability tables, scan orders, motion vector prediction,
inverse transforms and predictors can be adapted from `rusty_vp9`, which is
Apache-2.0 and goes into this MIT crate with attribution. Mode decision, motion
search, forward transforms and all SIMD are new. `rivet-vp9` is source-available
only and none of it is used.

## Main risk

Four of the six captures are full-frame animation with no cheap frames. There
the cost is transform, quantization and entropy coding, which libvpx already
does with tuned AVX2. Phase 2 is where to decide whether to go on: a scalar
encoder that is not within about three times libvpx on those captures is
unlikely to be brought under it by SIMD.
