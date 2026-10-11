# Repository instructions

- Strict no backward-compatibility or legacy paths no matter what.
- One crate, `screen-vp9`: the one place libvpx is spoken to for the wlshare
  daemon and the remotex gateway, which each pin it by a release tag of this
  repository. A libvpx setting, the conversion in front of
  it, the codec string or the quality walk changes here and reaches them as a
  pin bump. Nothing about a wire belongs here: how a frame is framed, where a
  link's lag is read from and which picture to encode are each user's own.
- After changes run `cargo test` and `cargo clippy --all-targets -- -D warnings`.
  Every encoder gets an independent decoder in its tests.
- Do not run `cargo fmt`. Errors are `thiserror`: every caller branches on them.
- A release is the version in `Cargo.toml` bumped and tagged `v<version>` on
  `main`, with a GitHub release for the tag (`gh release create v<version>
  --verify-tag --title v<version>`) whose notes say what changed since the
  last tag, breaking API first; users pin the tag.
