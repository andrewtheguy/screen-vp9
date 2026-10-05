# screen-vp9

VP9 for a desktop picture, as [wlshare](https://github.com/andrewtheguy/wlshare)
and the [remotex](https://github.com/andrewtheguy/remotex) gateway both code it.

- One libvpx encoder configuration: a quantizer pinned to a 1–100 quality dial
  rather than a bitrate, screen-content tuning, no lag, no dropped frames, and
  the colour matrix and range declared in the bitstream.
- `Picture`: the 8-bit 4:2:0 or 4:4:4 BT.601 studio-swing planes libvpx reads,
  converted from packed RGB or `B, G, R, X`.
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
screen-vp9 = { git = "https://github.com/andrewtheguy/screen-vp9", tag = "v0.0.7" }
```
