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
- **Acceptance gate**, on each of the eighteen 4:4:4 samples, at the quality
  and thread count the remotex gateway codes them at:
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
| Intra modes | DC, V, H and TM, each costed from the sums of its square's rows and columns and the pixels along its edges; in an inter frame tried only where the previous frame does not code the square whole | The six diagonal modes |
| Inter modes | Zero, nearest, near and new motion vectors to a quarter pixel, at EIGHTTAP_SMOOTH, the frame's one filter; the skip flag for a block that did not change | Eighth-pixel vectors |
| Transforms | A size per block, which it signals: an inter block's the one whose luma costs least in error and bits, quantized at each; an intra block's 4×4 where it varies, otherwise the largest up to 16×16; an intra block's luma below 32×32 in the DCT and ADST its mode pairs | — |
| Loop filter | libvpx's filter at the level its real-time encoder takes from the quantizer, an intra block's one more by the deltas a decoder starts with; none in four tile columns, where the libvpx encoder leaves it out for a decoder's threads | — |
| Probabilities | The coefficient, transform size and skip probabilities updated in each frame's header from the frame's own bools, and kept for the next; no backward adaptation, which libvpx's real-time encoder does not use either | Updates to the other probabilities |
| Tiles and threads | Tile columns by the width rule the crate has now, their superblock rows a wavefront over the threads; the source compared and hashed in a band of rows per thread | — |
| Segmentation, resizing, altref, lossless, high bit depth, error-resilient mode | Never | — |

## Where the speed should come from

- **Unchanged blocks:** compare each block with the previous source and code it
  as a skip without searching. A quality change makes every block dirty for one
  frame.
- **Scrolls and window drags:** hash-based whole-pixel motion search, which
  finds exact matches cheaply.
- **SIMD kernels:** NEON on arm64 and AVX2 on amd64, which the crate is
  built for, refusing a build without them (amd64 needs
  `-C target-feature=+avx2`), each with a scalar reference it is tested
  against: block difference, forward and inverse transforms, quantization.
- **Threading:** the superblock rows of every tile column as a wavefront
  over the threads, each a superblock behind the row above it, which leaves
  it what a block below reads across its top edge; and the comparison with
  the previous source and the motion search's hashing in a band of rows per
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
   on desktop alone). V, H and TM bought nothing on the captures. Besides these, motion the
   hash search cannot match, because the captures are decodes and what
   moves in them never arrives exactly: each changed superblock's row and
   column sums matched against the frame before, 64 pixels each way, done.
   It cut desktop's bytes 23% and proxmox-hidpi's 9%, for 5–9% more time.
   On the eighteen samples, coded in main's tile columns and without the
   loop filter in four: each tile column's superblock rows coded as a
   wavefront, done, as vp9-wasm reconstructs and libvpx's row-based
   multithreading codes. It took the quiet samples' median time on the Mac
   from 1.07–1.64× libvpx's to 0.84–1.01×, and the busy samples' from
   0.75–1.19× to 0.48–0.73×. Four tile columns cost no bytes themselves:
   coded in one, the four-column samples come out the same size. What
   costs is the loop filter they leave out: libvpx's own encoder leaves
   it out too, and its 4×4 blocks and a filter per block make up more
   of it there. Then V, H
   and TM, done, with the ADST: they cut every sample's bytes 1–10%, the
   four-column quiet ones' most, for up to 5% more time, and fourteen of
   the eighteen samples pass on the Mac. The four that do not are quiet,
   in four columns: 2048, 2560, 3456 and 3840 wide, at 1.17–1.29×
   libvpx's bytes. What moves in them is a shader animation, a halftone's
   dots and a reeded glass's stripes under moving light, which libvpx
   predicts between pixels, smoothed. EIGHTTAP_SMOOTH as the frame's one
   filter, done: it cut their bytes 7–12% and cost nothing elsewhere; and
   libvpx coded at it alone comes out no bigger than at its filter per
   block, which is therefore not built.
6. **Switch over.** The libvpx encoder, `Chroma::Subsampled` and `Speed`
   removed behind the same API, a release tagged, the pins bumped in wlshare
   and remotex as they drop 4:2:0, and `AGENTS.md` reworded: the crate then
   encodes in Rust and speaks to libvpx only to decode.

## Test material

Eighteen 4:4:4 IVF samples of 120 frames, kept out of git in `tmp/`. Where a
machine keeps the copies they come from is in that machine's `CLAUDE.local.md`,
not here; their directory's own `README.md` describes them. Each is cut from a
remotex gateway capture of 2026-10-08, decoded by libvpx and coded again by
this crate's libvpx encoder at quality 90, starting with a keyframe: a quiet
one from each capture's start, where little of the desktop moves, and a busy
one from the 300 frames of it that took the most bytes. Their names carry the
tile columns and loop filter that encode used, which both encoders use on them
here too.

| Samples | From the gateway's session with | Size |
|---|---|---|
| `quiet-` and `busy-1280x800-1col-lf` | Windows over RDP: a shader animation, a news page scrolled, a terminal printing, a documentation page, side by side | 1280×800 |
| `quiet-` and `busy-1440x900-2col-lf` | the same desktop | 1440×900 |
| `quiet-` and `busy-1600x1000-2col-lf` | the same desktop | 1600×1000 |
| `quiet-` and `busy-1920x1080-2col-lf` | the same desktop | 1920×1080 |
| `quiet-` and `busy-2048x1536-4col-nolf` | a virtual Mac, the same four panes, at HiDPI | 2048×1536 |
| `quiet-` and `busy-2560x1600-4col-nolf` | the same Mac | 2560×1600 |
| `quiet-` and `busy-2880x1800-4col-nolf` | the same Mac | 2880×1800 |
| `quiet-` and `busy-3456x2168-4col-nolf` | the Windows desktop at HiDPI | 3456×2168 |
| `quiet-` and `busy-3840x2160-4col-nolf` | the Mac | 3840×2160 |

All are measured at quality 90 on five threads, what remotex picks on the
six-core machine the baseline is measured on: the cores less one. The frames
fed to an encoder are libvpx decodes of the samples, so they are near the
source, not the source. Phases 1 to 5 above were measured on full captures,
the wlshare ones among them, before these samples were cut.

Beside each sample is a `.framemd5` of every decoded frame. That tests a
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

The busy samples are full-frame animation and scrolling with almost no cheap
frames. There the cost is transform, quantization and entropy coding, which
libvpx already does with tuned AVX2 and NEON. Phase 2 is where that is
measured and the decision to go on is made.
