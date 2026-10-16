# Repository instructions

- Strict no backward-compatibility or legacy paths no matter what.
- One crate, `screen-vp9`: how the wlshare daemon and the remotex gateway code
  a desktop as VP9, 8-bit 4:4:4 and nothing else, which each pins by a release
  tag of this repository. `main` is libvpx alone (`src/libvpx.rs`, the one
  place libvpx is spoken to, its decoder with it), behind one `Stream` and one
  `Encoder`, which their maker names the coder of (`Coder`). There is one
  coder; the name is kept so that another can take libvpx's place or stand
  beside it without a user's calls changing but for the name it gives.
  Nothing on `main` may need more than its target's baseline, and nothing on
  it is built for a CPU floor. A libvpx setting, the conversion in front of
  it, the codec string or the quality walk changes here and reaches the users
  as a pin bump. Nothing about a wire belongs here: how a frame is framed,
  where a link's lag is read from and which picture to encode are each user's
  own.
- The encoder written here in Rust is EXPERIMENTAL and lives on the
  `native-encoder` branch, with its docs, measurements and tools, and is not
  released from `main`. A feature is built on `main` for libvpx and is never
  held back because that coder lacks it: the branch takes `main` in and answers
  what it does not do with an error. Nothing on `main` is shaped for it.
- Those two are its only users, so the API is theirs alone and changes
  whenever a better shape for them is found: no caller elsewhere is kept
  working. Nothing a user calls differs by coder but the name it gives.
- A coder's speed (`Speed`) is not public: only the tests move it.
- The crate's own tests (`src/lib.rs`) are of what every coder shares and run
  on each; libvpx's are in `src/libvpx.rs`. Every encoder gets an independent
  decoder in its tests.
- A measurement is run on an idle machine, alone: no build, test, profile or
  second measurement beside it.
- After changes run `cargo test` and `cargo clippy --all-targets -- -D
  warnings`.
- Do not run `cargo fmt`. Errors are `thiserror`: every caller branches on them.
- A release is the version in `Cargo.toml` bumped and tagged `v<version>` on
  `main`, with a GitHub release for the tag (`gh release create v<version>
  --verify-tag --title v<version>`) whose notes say what changed since the
  last tag, breaking API first; users pin the tag.
