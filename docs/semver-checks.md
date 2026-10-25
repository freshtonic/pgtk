# cargo-semver-checks

cargo-semver-checks compares the public API of a crate with its last release on crates.io, and
finds changes that need a new major version (a new minor version before 1.0).

## What is checked

`release-plz.toml` sets `semver_check = true` for `pgtk-catalog` and `pgtk-analyze`. release-plz
runs the check when it calculates the next versions, in `scripts/release.sh` and in the Release
workflow. A breaking change thus gives a new minor version, not a patch version. The check needs a
release on crates.io to compare with, so it starts with the second release of each crate.

CI does not run the check on each pull request. Before 1.0, a pull request can break the API on
purpose, and the version changes only in the release.

## Why pgtk-sql is not checked

The check is correct for pgtk-sql, but it costs much and it tells us little:

1. **Almost every change to the grammar is a breaking change, by design.** The AST is the
   grammar. When a fix adds a variant to an enum or a field to a struct, code that matches the
   enum exhaustively, or makes the struct, does not compile any more. That exhaustive `match` is
   the purpose of the design, so the AST types are not `#[non_exhaustive]`, and must not be. Each
   release that changes the grammar is therefore a new minor version, and the check only repeats
   that.
2. **The API is different for each version feature.** The six version features are mutually
   exclusive, and each one gives a different set of types and fields. One check covers one
   feature set. A full check needs six runs.
3. **It is expensive.** The check builds the rustdoc JSON of the current crate and of the
   release. For pgtk-sql, one rustdoc pass takes about 90 seconds and 6 GB of memory, because of
   the generated grammar (about 25 MB of Rust). Six features at two passes each is twelve such
   runs for each release.

Thus a release of pgtk-sql that changes the grammar takes a new minor version, by policy. Turn the
check on for pgtk-sql if a part of its public API that is written by hand (for example
`document`, `minimum_version` or `TargetVersion`) becomes stable, and needs protection.
