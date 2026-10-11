# Repository instructions

- Strict no backward-compatibility or legacy paths no matter what.
- One crate, `screen-vp9-native`: a VP9 encoder in Rust for a desktop picture,
  8-bit 4:4:4 and nothing else, which the wlshare daemon and the remotex gateway
  can each take in place of `screen-vp9`, pinned by a release tag of this
  repository. How a desktop is coded, the conversion in front of the encoder,
  the codec string or the quality walk changes here and reaches them as a
  pin bump. Nothing about a wire belongs here: how a frame is framed, where a
  link's lag is read from and which picture to encode are each user's own.
- Those two are its only users, so the API is theirs alone and changes
  whenever a better shape for them is found: no caller elsewhere is kept
  working. It is the same shape as `screen-vp9`'s — the same `Stream` and
  `Encoder` calls with the same signatures, the same `walk` — so that a user
  holds one or the other without adapting either. A change to that shape is
  made in both repositories.
- Nothing a user builds links libvpx. It is a dev-dependency, which only the
  tests compile: the decoder in `src/decoder.rs`, and in `src/reference.rs` its
  encoder as `screen-vp9` sets it up, which the `*_against_libvpx` measurements
  play the captures into beside this one.
- A measurement is run on an idle machine, alone: no build, test, profile or
  second measurement beside it.
- After changes run `cargo test` and `cargo clippy --all-targets -- -D warnings`.
  Every frame the encoder makes in a test goes through that independent decoder.
- Do not run `cargo fmt`. Errors are `thiserror`: every caller branches on them.
- A release is the version in `Cargo.toml` bumped and the `Release` workflow
  run by hand on the commit: it runs CI's tests, tags `v<version>` and
  publishes that as a GitHub prerelease, always a prerelease and from any
  branch. Users pin the tag.
