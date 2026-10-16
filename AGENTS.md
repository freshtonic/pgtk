# pgtk

pgtk is a Cargo workspace of crates related to relational databases.

## Building

pgtk builds with Cargo and with Buck2. See `docs/adr/0001-dual-buck2-cargo-build.md`.

- Cargo: `cargo build --workspace` and `cargo test --workspace`.
- Buck2: `tools/buck2 build //...` and `tools/buck2 test //...`. `tools/buck2` needs
  [dotslash](https://dotslash-cli.com) (`cargo install dotslash`).

### Third-party dependencies

1. Add the version to the root `[workspace.dependencies]`. Use `dep.workspace = true` in the crate.
2. Run `cargo` to update `Cargo.lock`. Only `cargo` changes `Cargo.lock`.
3. Run `scripts/update-third-party`. Commit `third-party/rust/` with the change.
4. Add the dep to the crate's BUCK file as `//third-party/rust:<crate>`.

If reindeer reports a missing fixup, write `third-party/rust/fixups/<crate>/fixups.toml`.

### New crates

Each crate in `packages/` needs a hand-written `BUCK` file with a `rust_library` and a `rust_test`.
Keep its deps the same as the `[dependencies]` in its `Cargo.toml`.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues and use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the five default triage labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context: a root `CONTEXT-MAP.md` points to one `CONTEXT.md` for each package under `packages/`. See `docs/agents/domain.md`.
