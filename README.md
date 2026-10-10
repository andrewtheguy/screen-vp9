# screen-vp9-native

VP9 for a desktop picture, coded in Rust: what
[wlshare](https://github.com/andrewtheguy/wlshare) and the
[remotex](https://github.com/andrewtheguy/remotex) gateway can code a screen
with in place of [screen-vp9](https://github.com/andrewtheguy/screen-vp9),
whose encoder is libvpx's. Nothing a user builds links libvpx.

- `Encoder`: 8-bit 4:4:4, VP9 profile 1, and nothing else. A quantizer pinned
  to a 1–100 quality dial rather than a bitrate, no lag, no dropped frames, no
  keyframe that was not asked for, and the colour matrix and range declared in
  the bitstream. There is no 4:2:0: a browser with no software VP9 refuses the
  stream by name at `VideoDecoder.configure`.
- The stream's shape follows the picture's width and the threads, for the
  decoder's sake: one tile column under 1440 wide, two from 1440, four from
  2048 and eight from 5760, never more than the threads. A stream in four
  columns or more, 2048 wide on four threads, is coded without the loop
  filter, which a software decoder's threads otherwise wait on.
- `Picture`: the BT.601 studio-swing planes the encoder reads, converted from
  packed RGB or `B, G, R, X`, the whole picture or the rectangle that changed,
  on up to four of the encoder's threads.
- A frame told where its picture changed codes the 8×8s those rectangles touch
  and leaves every other block as the decoder has it, in a segment a decoder
  reads nothing more of and does not loop filter.
- `Stream`: the encoder and its picture together, which is how both users
  drive it. It reads a packed picture where that changed, straight from the
  framebuffer or the mirror and for no longer than the conversion takes, and
  then codes what it read since the last frame. It reads and codes the whole
  picture where there is none to be a change to, and says how coarse the
  coarsest block a decoder holds is, which is what a settle is owed for: the
  whole picture as one inter frame at the ceiling, read from nowhere since
  the planes hold it, the dial left where the link had it.
- `frame_header`: the profile and keyframe bit a frame says about itself;
  `codec_string`: the WebCodecs string a browser's `VideoDecoder` is configured
  with for a stream's size and frame rate.
- `walk::QualityWalk`: the quality and frame rate a link will bear, walked on
  the dial, so a stream wlshare codes and one remotex codes answer a slow link
  the same way. It is told what each frame left the client holding and says
  when a quiet desktop is owed its settle, half a second on.

The kernels are NEON on arm64 and AVX2 on amd64, and the build is refused
without them. NEON is in every arm64 target's baseline; AVX2 is not in
amd64's, so a user's amd64 build sets `-C target-feature=+avx2`, as this
repository's `.cargo/config.toml` does.

Every frame the encoder makes in a test is decoded by libvpx, from
[libvpx-prebuilt](https://github.com/andrewtheguy/libvpx-prebuilt)'s static
archive: a dev-dependency, which no build of a user fetches or links. How the
encoder works is in [docs/encoder.md](docs/encoder.md), what it was measured
at in [docs/benchmarks.md](docs/benchmarks.md), and how it was planned and
decided in [docs/plan.md](docs/plan.md).

Use it by release tag, each of which the `Release` workflow publishes as a
prerelease once the tests pass on every target:

```toml
screen-vp9-native = { git = "https://github.com/andrewtheguy/screen-vp9-native", tag = "v0.0.1-alpha.2" }
```
