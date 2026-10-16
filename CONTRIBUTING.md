# Contributing and making your own version

Formiga is a personal project rather than an openly contributed community project, and Formiga
Farm is part of it. I am not currently accepting pull requests, feature submissions, or requests
to maintain changes in this repository. Unsolicited contributions may be closed without review.

You are warmly encouraged to use the code as a starting point for your own experiments and
creatures. Fork it, remix it, improve it, or take it in a completely different direction under the
terms of the [MIT License](LICENSE).

If you publish a version of your own, please:

- Preserve the license and copyright notice.
- Give the project a distinct name and icon so people do not mistake it for an official Formiga
  release.
- Clearly describe it as an independent fork or derivative.

## Working on your version

- Install [rustup](https://rustup.rs). The pinned toolchain, Rust 1.97.1 with rustfmt and Clippy,
  installs itself from `rust-toolchain.toml` the first time you run `cargo`.
- Set `FORMIGA_FARM_DATA_DIR` to a scratch folder while you work, so your drafts and presets stay
  apart from an installed copy's:

  ```sh
  FORMIGA_FARM_DATA_DIR=/tmp/formiga-farm-dev cargo run --release -p formiga-farm
  ```

- Farm never reads or writes a colony's save. What passes between Farm and Formiga Desktop is
  written up in [docs/CONTRACT.md](docs/CONTRACT.md). The golden fixtures in
  `crates/formiga-farm-contract/tests/fixtures` are every version as it was first written, and are
  never rewritten to make a test pass: a new version's files go beside the old ones.

Before sharing a build, run the same gate CI runs on macOS and Windows:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
