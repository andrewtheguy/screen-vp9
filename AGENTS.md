# Repository instructions

- Strict no backward-compatibility or legacy paths no matter what.
- One crate, `screen-vp9`: how the wlshare daemon and the remotex gateway code
  a desktop as VP9, 8-bit 4:4:4 and nothing else, which each pins by a release
  tag of this repository. It holds two coders behind one `Stream` and one
  `Encoder`, which their maker names (`Coder`): libvpx set up for a desktop
  (`src/libvpx.rs`, the one place libvpx is spoken to, its decoder with it),
  and the encoder written here in Rust (`src/native/`), which is EXPERIMENTAL
  and was `screen-vp9-native` until it came home here. The Rust coder is the
  `native` feature, off unless asked for: its kernels need NEON or AVX2, and
  a build without it is libvpx alone, for any machine libvpx runs on. Nothing
  outside `src/native/` may need either. A libvpx setting, how
  the Rust coder codes, the conversion in front of them, the codec string or
  the quality walk changes here and reaches the users as a pin bump. Nothing
  about a wire belongs here: how a frame is framed, where a link's lag is read
  from and which picture to encode are each user's own.
- Those two are its only users, so the API is theirs alone and changes
  whenever a better shape for them is found: no caller elsewhere is kept
  working. Nothing a user calls differs by coder but the name it gives.
- A coder's speed (`Speed`) is not public and not legacy: the Rust coder's
  `Fastest` is work in progress for an optimization it may yet become, and
  only the tests and the measurements move it.
- The Rust coder's tests hold every frame it makes to libvpx's decoder, and
  the `*_against_libvpx` measurements play the captures into libvpx's encoder
  beside it. `docs/` is the Rust coder's: how it codes, its plan and its
  benchmarks.
- A measurement is run on an idle machine, alone: no build, test, profile or
  second measurement beside it.
- After changes run `cargo test --features native` and `cargo clippy
  --all-targets --features native -- -D warnings`, and both again without the
  feature. The crate's own tests are made with the Rust coder and run only
  with it; libvpx's are in `src/libvpx.rs` and run either way. Every encoder
  gets an independent decoder in its tests.
- Do not run `cargo fmt`. Errors are `thiserror`: every caller branches on them.
- A release is the version in `Cargo.toml` bumped and tagged `v<version>` on
  `main`, with a GitHub release for the tag (`gh release create v<version>
  --verify-tag --title v<version>`) whose notes say what changed since the
  last tag, breaking API first; users pin the tag.
