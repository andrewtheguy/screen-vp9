# screen-vp9

VP9 for a desktop picture, as [wlshare](https://github.com/andrewtheguy/wlshare)
and the [remotex](https://github.com/andrewtheguy/remotex) gateway both code
it, which are its only users. One `Stream` and one `Encoder`, coded by the
coder their maker names: `Coder::Libvpx`, libvpx set up for a desktop, which
is the one there is. The name is kept so that another coder can take its place
without a user's calls changing but for the name.

The encoder written here in Rust, which was `screen-vp9-native`, lives on the
[`native-encoder`](https://github.com/andrewtheguy/screen-vp9/tree/native-encoder)
branch: **EXPERIMENTAL**, built for x86-64-v3 on amd64 and NEON on arm64, and
not released. `main` and its tags are libvpx alone and ask nothing of a machine
that libvpx does not.

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
  map, a 4K frame with one small change in 11 ms for 26.
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

libvpx is [libvpx-prebuilt](https://github.com/andrewtheguy/libvpx-prebuilt)'s
static archive, which the build fetches: the coder, and the decoder every
frame made in a test is held to.

Use it by release tag:

```toml
screen-vp9 = { git = "https://github.com/andrewtheguy/screen-vp9", tag = "v0.0.25" }
```
