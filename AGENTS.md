# pgtk

pgtk is a Cargo workspace of crates related to relational databases.

## Building

pgtk builds with Cargo and with Buck2. See `docs/adr/0001-dual-buck2-cargo-build.md`.

- Cargo: `cargo build --workspace` and `cargo test --workspace`.
- Buck2: `tools/buck2 build //...` and `tools/buck2 test //...`. `tools/buck2` needs
  [dotslash](https://dotslash-cli.com) (`cargo install dotslash`).

### CI

The GitHub Actions jobs run on Linux runners only. Do not add a macOS runner. Run the macOS
builds locally.

### Third-party dependencies

1. Add the version to the root `[workspace.dependencies]`. Use `dep.workspace = true` in the crate.
2. Run `cargo` to update `Cargo.lock`. Only `cargo` changes `Cargo.lock`.
3. Run `scripts/update-third-party`. Commit `third-party/rust/` with the change.
4. Add the dep to the crate's BUCK file as `//third-party/rust:<crate>`.

If reindeer reports a missing fixup, write `third-party/rust/fixups/<crate>/fixups.toml`.

### New crates

Each crate in `packages/` needs a hand-written `BUCK` file with a `rust_library` and a `rust_test`.
Keep its deps the same as the `[dependencies]` in its `Cargo.toml`. The Cargo-only crates
(`pgtk-oracle`, `pgtk-psql-oracle`, `pgtk-test-server`, `pgtk-test-features`,
`pgtk-catalog-xtask`, `pgtk-skills`) have no `BUCK` file. See the ADR.

If the crate has a build script that uses recursa-codegen, run it with
`first_party_build_script` from `tools/rust/build_script.bzl`. `packages/pgtk-sql/BUCK` is the
model.

### Tests

`cargo test --workspace` runs every test. `pgtk-test-features` turns on the features that gate
tests.

The README of each crate is its crate documentation (`#![doc = include_str!("../README.md")]`), and
its Rust examples are doctests. Keep them compiling. If an example needs a feature or a version,
gate the `include_str!` for doctests (see `packages/pgtk-sql/src/lib.rs`). Do not put hidden `# `
lines in a README, because GitHub and crates.io show them. If you gate a new test behind a feature, add that feature to
`packages/pgtk-test-features/Cargo.toml`. A test that needs a PostgreSQL server uses
`pgtk-test-server`, never a server that the developer must supply.

### Agent skills for users

`skills/` holds the agent skills for the users of the published crates (see `skills/README.md`).
`packages/pgtk-skills` compiles and runs their Rust code blocks as doctests. When you change an
API that a skill uses, update the skill in the same change.

### Formatting and lints

Every package must pass `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings` and
`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`. CI runs all three.

Each commit subject is a [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/),
for example `fix(pgtk-sql): reject a second WITH in CREATE DATABASE`. CI runs
`scripts/check-conventional-commits` on each pull request, and release-plz makes the changelogs
from the subjects.

CI also checks the published crates with Rust 1.88, their `rust-version`, and the licences of
their dependencies against `deny.toml`. A new dependency of a published crate must have a licence
that `deny.toml` permits.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues and use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the five default triage labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context: a root `CONTEXT-MAP.md` lists the packages that have a `CONTEXT.md`, and how they relate. See `docs/agents/domain.md`.
