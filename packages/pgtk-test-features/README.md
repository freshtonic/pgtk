# pgtk-test-features

**One `cargo test --workspace` that runs every test.**

Some tests of the pgtk crates need a Cargo feature, for example the oracle tests. Cargo unifies
the features of a package across one build. This crate has no code: its manifest asks for each
feature that gates a test, so a workspace build turns those features on.

pgtk-test-features is a development crate of [pgtk](https://github.com/freshtonic/pgtk). It is not
published.

## Use

```sh
cargo test --workspace
```

A consumer of a pgtk crate does not get these features. `cargo test -p <crate>` does not get them
either: give that command `--features` (see `DEVELOPMENT.md`).

If you put a new test behind a feature, add the feature to `Cargo.toml` of this crate.

## Licence

MIT.
