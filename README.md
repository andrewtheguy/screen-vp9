# screen-vp9

VP9 for a desktop picture, as [wlshare](https://github.com/andrewtheguy/wlshare)
and the [remotex](https://github.com/andrewtheguy/remotex) gateway both code
it, which are its only users. One `Stream` and one `Encoder`, coded by the
coder their maker names:

- `Coder::Libvpx`: libvpx, set up for a desktop. The default.
- `Coder::Native`: the encoder written here in Rust, which was
  `screen-vp9-native` until it came home here. **EXPERIMENTAL**, and built
  only with the `native` feature.

The stream is the same kind either way and nothing else a user calls differs.

- `Encoder`: 8-bit 4:4:4, VP9 profile 1, and nothing else. A quantizer pinned
  to a 1–100 quality dial rather than a bitrate, no lag, no dropped frames, no
  keyframe that was not asked for, and the colour matrix and range declared in
  the bitstream. There is no 4:2:0: a browser with no software VP9 refuses the
  stream by name at `VideoDecoder.configure`.
- The stream's shape follows the picture's width and the threads, for the
  decoder's sake: one tile column under 1440 wide, two from 1440, four from
  2048 and eight from 5760, never more than the threads. A stream in four
  columns or more, 2048 wide on four threads, is coded without the loop
  filter, which a software decoder's threads otherwise wait on; that costs 2%
  to 4% more bytes and 0.5 to 0.9 dB of luma.
- 4:4:4 only, VP9 profile 1 (`PROFILE`): on a desktop the visible loss is a
  coloured glyph stem's colour, which 4:2:0 averages away at any quantizer.
- `Picture`: the BT.601 studio-swing planes an encoder reads, converted from
  packed RGB or `B, G, R, X`, the whole picture or the rectangle that changed,
  on up to four of the encoder's threads: a 4K picture in 2.9 ms for 5.5, and
  a window dragged across one in the quarter of it that moved.
- A frame told where its picture changed codes the blocks those rectangles
  touch and leaves every other as the decoder has it: through libvpx's active
  map, a 4K frame with one small change in 11 ms for 26, and by the Rust coder
  in a segment a decoder reads nothing more of and does not loop filter.
- `Stream`: the encoder and its picture together, which is how both users
  drive it. It reads a packed picture where that changed, straight from the
  framebuffer or the mirror and for no longer than the conversion takes, and
  then codes what it read since the last frame. It reads and codes the whole
  picture where there is none to be a change to, and says how coarse the
  coarsest block a decoder holds is, which is what a settle is owed for: the
  whole picture as one inter frame at the ceiling, read from nowhere since
  the planes hold it, the dial left where the link had it.
- `Decoder`: libvpx's, the planes back, for every test here and in a user; a
  frame that is not 8-bit 4:4:4 is refused.
- `frame_header`: the profile and keyframe bit a frame says about itself;
  `codec_string`: the WebCodecs string a browser's `VideoDecoder` is configured
  with for a stream's size and frame rate.
- `walk::QualityWalk`: the quality and frame rate a link will bear, walked on
  the dial, so a stream wlshare codes and one remotex codes answer a slow link
  the same way. It is told what each frame left the client holding and says
  when a quiet desktop is owed its settle, half a second on.

The Rust coder is built for NEON on arm64 and for x86-64-v3 on amd64 (AVX2,
BMI1 and BMI2, LZCNT, MOVBE, FMA: Haswell and Excavator on), and a build with
the `native` feature is refused under them. NEON is in every arm64 target's
baseline; x86-64-v3 is above amd64's, so a user's amd64 build with the
feature sets `-C target-cpu=x86-64-v3`, as this repository's
`.cargo/config.toml` does. That is a floor, raised when a higher one is
measured to buy enough, and no processor under it is kept working. Without
the feature the crate is libvpx alone and asks nothing of the machine that
libvpx does not, which is the build for an older processor.

libvpx is [libvpx-prebuilt](https://github.com/andrewtheguy/libvpx-prebuilt)'s
static archive, which the build fetches: one of the two coders, and the
decoder every frame the Rust coder makes in a test is held to. How the Rust
coder works is in [docs/encoder.md](docs/encoder.md), what it was measured at
in [docs/benchmarks.md](docs/benchmarks.md) (on an Apple M2 Max in
[docs/benchmarks-m2-max.md](docs/benchmarks-m2-max.md)), and how it was planned and
decided in [docs/plan.md](docs/plan.md).

Use it by release tag:

```toml
screen-vp9 = { git = "https://github.com/andrewtheguy/screen-vp9", tag = "v0.0.17" }
# and with the Rust coder:
screen-vp9 = { git = "https://github.com/andrewtheguy/screen-vp9", tag = "v0.0.17", features = ["native"] }
```
