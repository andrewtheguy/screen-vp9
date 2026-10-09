# screen-vp9

VP9 for a desktop picture, as [wlshare](https://github.com/andrewtheguy/wlshare)
and the [remotex](https://github.com/andrewtheguy/remotex) gateway both code it.

- One libvpx encoder configuration: a quantizer pinned to a 1–100 quality dial
  rather than a bitrate, screen-content tuning, no lag, no dropped frames, and
  the colour matrix and range declared in the bitstream.
- The stream's shape follows the picture's width and the threads, for the
  decoder's sake: one tile column under 1440 wide, two from 1440, four from
  2048 and eight from 5760, never more than the threads. A 4:4:4 stream in
  four columns or more, 2048 wide on four threads, is coded without the loop
  filter, which a software decoder's threads otherwise wait on; that costs 2%
  to 4% more bytes and 0.5 to 0.9 dB of luma. 4:2:0 always keeps the filter.
- `Picture`: the 8-bit 4:2:0 or 4:4:4 BT.601 studio-swing planes libvpx reads,
  converted from packed RGB or `B, G, R, X`, the whole picture or the rows that
  changed.
- A frame told where its picture changed skips every block outside those
  rectangles, through libvpx's active map: a 4K frame with one small change
  encodes in 11 ms for 26, and a 1440p one in 5 for 12.
- `Stream`: the encoder and its picture together, fed a packed picture and
  where it changed, which is how both users drive it. It reads and codes the
  whole picture where a frame has none to be a change to, and says how coarse
  the coarsest block a decoder holds is, which is what a settle is owed for.
- `Decoder`: the planes back, for every test here and in a user.
- `frame_header`: the profile and keyframe bit a frame says about itself;
  `codec_string`: the WebCodecs string a browser's `VideoDecoder` is configured
  with for a stream's size, chroma and frame rate.
- `walk::QualityWalk`: the quality and frame rate a link will bear, walked on
  the dial, so a stream wlshare codes and one remotex codes answer a slow link
  the same way.

libvpx comes from [libvpx-prebuilt](https://github.com/andrewtheguy/libvpx-prebuilt)'s
static archive for macOS arm64, Linux x86_64 and aarch64 and Windows x86_64;
nothing is configured or installed at build time.

Use it by release tag:

```toml
screen-vp9 = { git = "https://github.com/andrewtheguy/screen-vp9", tag = "v0.0.12" }
```
