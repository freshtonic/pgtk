# Development

pgtk builds with Cargo and with Buck2. Both builds use the same `Cargo.lock`. For the design, see
`docs/adr/0001-dual-buck2-cargo-build.md`.

## Prerequisites

- **Rust**: install [rustup](https://rustup.rs). rustup reads `rust-toolchain.toml` and installs
  the pinned toolchain when you first run `cargo`.
- **Buck2**: install [dotslash](https://dotslash-cli.com) (`cargo install dotslash`).
  `tools/buck2` runs the pinned buck2 (`tools/buck2.dotslash`). You do not install buck2
  yourself. Always run buck2 through `tools/buck2`, because it also writes the build settings
  (see [The remote cache](#the-remote-cache)).
- **C toolchain**: `/usr/bin/clang`, `/usr/bin/clang++` and `/usr/bin/ar`. The Buck2 C++
  toolchain uses these absolute paths.
- **PostgreSQL build tools**: `make`, `bison`, `flex`, `perl` and libxml2 (on Debian and Ubuntu,
  `libxml2-dev`). The tests build the pinned PostgreSQL releases (see
  [The PostgreSQL builds](#the-postgresql-builds)). They do not need a PostgreSQL server or
  Docker.
- **jq**: the gate scripts need it.
- **reindeer**: only when you change a third-party dependency. Install the revision that CI uses
  (`REINDEER_REV` in `.github/workflows/ci.yml`):

  ```sh
  cargo install --locked --git https://github.com/facebookincubator/reindeer --rev <REINDEER_REV> reindeer
  ```

## Get the source

The tests read the PostgreSQL sources from the `vendor/postgres` submodule. They read some frozen
files by Git blob ID, so the submodule needs its full history.

```sh
git clone --recurse-submodules git@github.com:freshtonic/pgtk.git
cd pgtk
packages/pgtk-sql/scripts/fetch-postgres-pins
```

`fetch-postgres-pins` gets the pinned PostgreSQL release of each Target Version
(`packages/pgtk-oracle/pins.tsv`) into the object store of the submodule. Run it again when
`pins.tsv` changes.

If you cloned without `--recurse-submodules`, run `git submodule update --init` first.

## Cargo

Run these commands from the repository root.

```sh
cargo build --workspace
cargo test --workspace
```

`cargo test --workspace` runs every test of the repository for the default Target Version
(pg17):

- The tests against the oracles (`differential`, `accepted_legacy_gaps`, `psql_oracle`).
- The pgtk-catalog tests of all features, and the live loader tests against PostgreSQL 14, 15,
  16 and 17.
- The pgtk-analyze oracle harness, which replays the regression corpus against PostgreSQL 17.

The `pgtk-test-features` member turns on the features that these tests need. Cargo unifies the
features of a package across the packages of one build, so only a workspace build gets them.

The first run on a machine is slow:

- The pgtk-sql and pgtk-psql build scripts generate the grammar with recursa-codegen.
- The tests build PostgreSQL 14, 15, 16 and 17 (about one minute each). See
  [The PostgreSQL builds](#the-postgresql-builds).
- The oracle harness builds its snapshots (about one minute).

Later runs are incremental. The oracle harness takes about 5 minutes in a debug build. The other
tests take less than a minute.

### One crate

`cargo test -p <crate>` uses the features of that crate only, so it skips the tests that a
feature gates. Give the features yourself:

| Crate | Command for all its tests |
| --- | --- |
| pgtk-sql | `cargo test -p pgtk-sql --features postgres-oracle` |
| pgtk-psql | `cargo test -p pgtk-psql --features psql-oracle` |
| pgtk-catalog | `cargo test -p pgtk-catalog --all-features` |
| pgtk-analyze | `cargo test -p pgtk-analyze` |

To run one test target, add `--test <name>`, for example
`cargo test -p pgtk-sql --test embedded`.

### Other Target Versions

The version features (`pg14`, `pg15`, `pg16`, `pg17`, `pg18`, `pg19-beta`) are mutually
exclusive. To build another version, turn off the default features:

```sh
cargo test -p pgtk-sql --no-default-features --features pg16
```

### Gate scripts

The gate scripts run a full configuration. They find their crate directory themselves, so you can
start them from any directory.

| Script | What it proves |
| --- | --- |
| `packages/pgtk-sql/scripts/merge-gate` | pg17 with `spans`, `arbitrary` and both oracles. Run it before a merge. |
| `packages/pgtk-sql/scripts/release-gate [pgN...]` | Each Target Version (all six if you give none). Run it before a release, and for the oldest affected version when you change a version gate. |
| `packages/pgtk-analyze/scripts/gate-versions [pgN...]` | clippy and the tests of pgtk-analyze for each Grammar Version. |

In `packages/pgtk-sql`, `cargo loop` runs the merge gate configuration of pgtk-sql. Use it as the
edit loop:

```sh
cd packages/pgtk-sql
cargo loop --test embedded
```

### Tests against a PostgreSQL server

The live tests start their own servers with `pgtk-test-server`. Each server listens only on a Unix
socket in a directory of its own, so the tests run in parallel and use no TCP port.

- **pgtk-catalog loader** (`tests/loader_live.rs`): one server of each supported release for the
  test process. Each test makes its own database on each server.
- **pgtk-analyze oracle** (`tests/oracle.rs`): one server for each corpus file that it replays.
  `packages/pgtk-analyze/docs/oracle-harness.md` describes the design and the variables, for
  example `PG_ANALYZE_ORACLE_FILES=int4,join` to replay only some files. The harness is faster in a
  release build:

  ```sh
  cargo test -p pgtk-analyze --release --test oracle
  ```

### Formatting and lints

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Buck2

Run these commands from the repository root.

```sh
tools/buck2 build //...
tools/buck2 test //...
```

To build or test one crate, give its package:

```sh
tools/buck2 test //packages/pgtk-sql/...
```

Buck2 builds less than Cargo:

- Buck2 builds the default features of each crate only. For pgtk-sql it also turns on `spans`.
  The other Target Versions and the oracle features are Cargo-only.
- `pgtk-oracle`, `pgtk-psql-oracle`, `pgtk-test-server`, `pgtk-test-features` and
  `pgtk-catalog-xtask` are Cargo-only. They have no BUCK file. So Buck2 does not run the oracle
  tests or the live tests.
- The Buck2 tests are the unit tests of each crate, and the pgtk-sql integration tests that need
  only the library (`STANDALONE_TESTS` and the embedded tests in `packages/pgtk-sql/BUCK`). The
  other integration tests read fixtures or link an oracle, and run only under Cargo.

Buck2 writes only to `buck-out/`, and Cargo writes only to `target/`. The two builds do not
change each other's files.

### The remote cache

Buck2 can use the BuildBuddy remote cache. All actions still run on your machine: Buck2 reads
the outputs of an action from the cache when the cache has them, and it uploads the outputs of
the actions that it runs.

To use the cache, set the API key before you run `tools/buck2`:

```sh
export BUILDBUDDY_API_KEY=<key>
tools/buck2 build //...
```

Without the key, Buck2 uses no remote cache. CI gets the key from the `BUILDBUDDY_API_KEY`
secret.

Buck2 does not hash the system toolchain, so `tools/buck2` puts a toolchain ID into the key of
each action. The ID is a hash of `rustc -vV`, the clang version, the OS and CPU, and the macOS
SDK or the C library version. `tools/buck2` calculates it again before each command and writes it
to `.buckconfig.pgtk`, so an update of the toolchain gets new cache keys at once.

## Change a dependency

Only `cargo` changes `Cargo.lock`. After a change to a third-party dependency, vendor it for
Buck2 and commit the result. AGENTS.md has the full procedure.

```sh
cargo update -w                 # or cargo add, then cargo build
scripts/update-third-party      # vendors into third-party/rust/ and writes third-party/rust/BUCK
```

If reindeer reports a missing fixup, write `third-party/rust/fixups/<crate>/fixups.toml`. CI
fails if `third-party/rust/` does not agree with `Cargo.lock`.

## The PostgreSQL builds

`packages/pgtk-oracle/scripts/build-pg.sh` builds a pinned PostgreSQL release from the
`vendor/postgres` object store. It builds each release once for each machine:

- `pgtk-oracle` links the raw parser of the pinned release, and `pgtk-psql-oracle` links its psql
  lexer.
- `pgtk-test-server` installs the server of a pinned release, and starts servers of it for the
  live tests.

The builds go in `${XDG_CACHE_HOME:-~/.cache}/pgtk-sql/postgres`. To use another directory, set
`PG_SQL_POSTGRES_CACHE`. A build that has other configure options, for example a build that was
moved, is made again.

If a build fails, run the script directly to see the full output:
`packages/pgtk-oracle/scripts/build-pg.sh pg17`.
