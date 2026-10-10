# Repository instructions

- Strict no backward-compatibility or legacy paths no matter what.
- One crate, `screen-vp9-native`: a VP9 encoder in Rust for a desktop picture,
  8-bit 4:4:4 and nothing else, which the wlshare daemon and the remotex gateway
  can each take in place of `screen-vp9`, pinned by a release tag of this
  repository. How a desktop is coded, the conversion in front of the encoder,
  the codec string or the quality walk changes here and reaches them as a
  pin bump. Nothing about a wire belongs here: how a frame is framed, where a
  link's lag is read from and which picture to encode are each user's own.
- Nothing a user builds links libvpx. It is a dev-dependency: the decoder in
  `src/decoder.rs`, which only the tests compile.
- After changes run `cargo test` and `cargo clippy --all-targets -- -D warnings`.
  Every frame the encoder makes in a test goes through that independent decoder.
- Do not run `cargo fmt`. Errors are `thiserror`: every caller branches on them.
- A release is the version in `Cargo.toml` bumped and the `Release` workflow
  run by hand on the commit: it runs CI's tests, tags `v<version>` and
  publishes that as a GitHub prerelease, always a prerelease and from any
  branch. Users pin the tag.
