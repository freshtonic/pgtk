# pgtk-sql

**A PostgreSQL parser that agrees with PostgreSQL.**

[![crates.io](https://img.shields.io/crates/v/pgtk-sql.svg)](https://crates.io/crates/pgtk-sql)
[![docs.rs](https://img.shields.io/docsrs/pgtk-sql)](https://docs.rs/pgtk-sql)
[![MIT licence](https://img.shields.io/badge/licence-MIT-blue.svg)](https://github.com/freshtonic/pgtk/blob/main/LICENSE)

Most SQL parsers speak "approximately PostgreSQL". pgtk-sql speaks the PostgreSQL of one exact
release. It accepts each statement that PostgreSQL's `gram.y` accepts, and it rejects each
statement that `gram.y` rejects. You select the release with a Cargo feature: PostgreSQL 14, 15,
16, 17, 18 or 19 beta.

pgtk-sql is part of [pgtk](https://github.com/freshtonic/pgtk), the PostgreSQL toolkit for Rust.

## The AST is the grammar

Most SQL ASTs are loose. pg_query.rs gives one enum with a variant for each of the hundreds of
node kinds of PostgreSQL, and a child is a `Node` that can be any of them. sqlparser-rs gives one
AST for all SQL dialects, so its values can hold syntax that PostgreSQL does not accept. With
these ASTs, the compiler cannot tell you that a value is valid SQL. Your code must check.

pgtk-sql uses algebraic data types to make the AST match the grammar exactly:

- **A struct is a sequence.** Its fields are the parts of a grammar rule, in order.
- **An enum is a choice.** Its variants are the alternatives of a grammar rule.
- **Cardinality is exact.** `Option<T>` is zero or one, `zero_or_many!(T)` is zero or more, and
  `one_or_many!(T)` is one or more: a list type that cannot be empty. An empty list exists only
  where the grammar permits one.
- **Versions are in the types.** A field that PostgreSQL 17 added exists only in the builds for
  PostgreSQL 17 and later.

This is `MERGE`, from the source of pgtk-sql:

```rust,ignore
recursa::ast_node! {
    pub struct MergeStmt {
        #[tok(MERGE, INTO, this)]
        pub target: boxed!(MergeTarget),
        #[tok(USING, this)]
        pub source: boxed!(TableRef),
        #[tok(ON, this)]
        pub condition: boxed!(Expr),
        /// PostgreSQL's `merge_when_list` is one-or-more.
        pub when_clauses: one_or_many!(WhenClause),
        #[config(since = pg17)]
        pub returning: Option<boxed!(ReturningClause)>,
    }
}
```

The declaration is the grammar: [recursa](https://crates.io/crates/recursa) generates the lexer
and the LR parser from these types. A `MERGE` with no `WHEN` clause cannot exist, because the
type has no value for it. The goal is that each value that you can make is a statement that
PostgreSQL can parse. Where a value is not, the type is too loose, and that is a bug in pgtk-sql.

The design gives three results:

- **Your `match` is the specification.** When you handle each variant, you handle each form
  that PostgreSQL accepts, and the compiler tells you when you forget one.
- **Generated SQL is almost always valid.** With the `arbitrary` feature, each AST type
  derives `arbitrary::Arbitrary`, so a fuzz test or a property test gets typed statements, not
  random bytes. Today, PostgreSQL accepts about 80% of the random statements. Each of the other
  20% shows a place where a type permits more than PostgreSQL does. A test measures this on each
  run, and issues
  [#50 to #61](https://github.com/freshtonic/pgtk/issues?q=is%3Aissue+label%3Abug+%22pgtk-sql+accepts%22)
  record the gaps that remain.
- **Traversal is free.** Each AST type derives `Visit` and `VisitMut`, so a lint or a rewrite
  is a visitor, not a recursive function for each node type.

## Why pgtk-sql?

- **Proven parity.** On PostgreSQL's own regression suite, pgtk-sql and the raw parser of
  PostgreSQL agree on all **43,474** statements, for each of the six versions. The test suite
  links the real PostgreSQL parser and compares again on each run.
- **Pure Rust.** No C library to build or link, unlike the bindings to PostgreSQL's parser.
- **Fast.** On the same statements, pgtk-sql parses at **35 MiB/s**, more than **2×** the speed
  of `sqlparser`. The flat AST is faster again, at **40 MiB/s**.
- **Spans.** With the `spans` feature, each node knows its location in the source text.
- **A flat AST.** One contiguous store for each parse, with handles in place of pointers. It is
  fast to make and fast to drop.
- **Version answers.** pgtk-sql tells you the oldest PostgreSQL version that accepts a
  statement, and which clause needs it.
- **More than a parser.** A formatter and railroad diagrams of the grammar.

## Quick start

```toml
[dependencies]
pgtk-sql = "0.1"
```

Parse one statement:

```rust
use pgtk_sql::{ast::Statement, lex};

let lexed = lex("SELECT id, name FROM users WHERE id = $1");
let mut input = lexed.input();
let statement = Statement::parse(&mut input).expect("PostgreSQL accepts it");
```

Parse a document of statements (this needs the `spans` feature):

```rust
let document = pgtk_sql::document::parse_sql("SELECT 1; INSERT INTO t VALUES (2);")
    .expect("PostgreSQL accepts both statements");
assert_eq!(document.statements().len(), 2);
```

`parse_sql` is a strict parse: it stops at the first statement that PostgreSQL rejects. An editor
needs one result for each statement. `parse_sql_recovering` continues after each rejection, at the
next `;` that is not in parentheses, in a string or in a `BEGIN ATOMIC … END` body:

```rust
use pgtk_sql::document::{RecoveredItem, parse_sql_recovering};

let source = "SELECT 1; SELECT FROM WHERE; SELECT 2;";
let document = parse_sql_recovering(source).expect("no framing invariant fails");
for item in document.items() {
    let text = &source[item.span().range()];
    match item {
        RecoveredItem::Statement { .. } => println!("accepted: {text}"),
        RecoveredItem::Rejected(rejection) => println!("{rejection}"),
    }
}
assert_eq!(document.statements().len(), 2);
assert_eq!(document.rejections().count(), 1);
```

Find the oldest PostgreSQL version that accepts a statement:

```rust
use pgtk_sql::{MinimumVersion, TargetVersion, ast::Statement, lex};

let lexed = lex("SELECT * FROM (SELECT 1)");
let mut input = lexed.input();
let statement = Statement::parse(&mut input).expect("PostgreSQL 16 accepts it");

// PostgreSQL 15 needs an alias on a subquery in FROM. PostgreSQL 16 does not.
let found = statement.minimum_version_above(TargetVersion::Pg15).unwrap();
assert_eq!(found.version(), TargetVersion::Pg16);
assert_eq!(found.message(), Some("subquery in FROM must have an alias"));
```

To know what a statement means (its types, its columns, its errors), give it to
[`pgtk-analyze`](https://crates.io/crates/pgtk-analyze). To parse a psql script, give it to
`pgtk-psql` first.

## Examples

The `examples/` folder has programs that you can copy and change. Each one needs the `spans`
feature.

| Example | What it does |
|---|---|
| `parse` | Parses a SQL file, and prints the kind and the location of each statement. It shows the location and the message when PostgreSQL rejects the SQL. |
| `version_check` | Prints the oldest PostgreSQL version that accepts each statement, and the clause that needs it. It fails when a statement needs a newer server than `--server`. |
| `migration_lint` | A linter for migrations, written as a visitor: `CREATE INDEX` without `CONCURRENTLY`, `ADD COLUMN ... NOT NULL` with no default, `DROP COLUMN`, `ALTER COLUMN ... TYPE`, and `UPDATE` or `DELETE` with no `WHERE`. |

```sh
cargo run -p pgtk-sql --features spans --example parse -- migrations/0001.sql
cargo run -p pgtk-sql --features spans --example version_check -- --server 15 migrations/*.sql
cargo run -p pgtk-sql --features spans --example migration_lint -- migrations/*.sql
```

## Features

| Feature | What it does |
|---|---|
| `pg14` ... `pg19-beta` | Selects the PostgreSQL version. Use exactly one. The default is `pg17`. |
| `spans` | Keeps the source span of each node. The generated `parse_without_spans` entry points skip that cost in any build. |
| `arbitrary` | Derives `arbitrary::Arbitrary` for the AST. |
| `postgres-oracle` | Development only: builds PostgreSQL's raw parser for the differential tests. |

## Build time

The build script of pgtk-sql generates the parser with `recursa-codegen`.
Cargo compiles a build script and its dependencies with `opt-level = 0` by
default. At that level, the build script runs for approximately one minute.
To make it faster, add this to the `Cargo.toml` of your workspace root:

```toml
[profile.dev.build-override]
opt-level = 3

[profile.release.build-override]
opt-level = 3
```

With this setting, the build script takes approximately 7 seconds. The
setting changes only how Cargo compiles build scripts and procedural macros,
not your code. Cargo reads profiles only from the workspace root, so pgtk-sql
cannot set it for you. The pgtk workspace has the same setting.

## Target versions

A pgtk-sql build reproduces the raw parser of one PostgreSQL major version, its
target version. Select it with exactly one Cargo feature:

| Feature | PostgreSQL release |
|---|---|
| `pg14` | 14.24 |
| `pg15` | 15.19 |
| `pg16` | 16.15 |
| `pg17` (default) | 17.11 |
| `pg18` | 18.6 |
| `pg19-beta` | `REL_19_STABLE` at `b73d13c` |

```toml
pgtk-sql = { version = "0.1", default-features = false, features = ["pg16", "spans"] }
```

The features are mutually exclusive: the build stops with an error for zero
or for two or more. Cargo feature unification is the usual cause of two. A
crate between your crate and pgtk-sql must use `default-features = false` on
pgtk-sql and forward the version features, as `pgtk-psql` does.
`pgtk_sql::TARGET_VERSION` reports the selection. The grammar of each
version is complete: the differential baseline of each version has no
expected gap. See ADR 0009 and `docs/plans/2026-09-22-target-versions.md`.

`cargo loop` is the edit loop: it tests pgtk-sql in the merge gate's
configuration, so `cargo loop --test embedded` and the gate share one build.
`scripts/merge-gate` proves that configuration before a merge, and
`scripts/release-gate` builds and tests every target version before a
release (ADR 0011). `docs/notes/build-times.md` records what the build costs
and what was tried against it.

### The minimum version of a statement

A build fixes the grammar, but the server that runs the SQL can be older.
`pgtk_sql::minimum_version` answers, for one parsed statement, the lowest target
version whose grammar accepts it, and the first construct that a given older
version rejects. Where `gram.y` raises a specific `ereport` in that older
version, the answer carries its SQLSTATE, message and hint.

```rust
use pgtk_sql::{MinimumVersion, TargetVersion, ast::Statement, lex};

let lexed = lex("SELECT * FROM (SELECT 1)");
let mut input = lexed.input();
let parsed = Statement::parse(&mut input).expect("PostgreSQL 16 accepts it");

if let Some(found) = parsed.minimum_version_above(TargetVersion::Pg15) {
    // "subquery in FROM must have an alias", 42601, needs PostgreSQL 16,
    // with the span of the subquery that lacks it.
    eprintln!("{:?} {:?}: {:?}", found.version(), found.span(), found.message());
}
```

The answer comes from the version gates, which are declarations that Recursa
records, so no list can drift from them. `docs/minimum-version.md` describes
the classes of gate, the lexical scan that goes with the parsed value, and
what the answer does not cover. See ADR 0010.

## Oracles

Two oracles check the parsers against PostgreSQL itself, both built from the
pinned release of the build's target version (`pgtk-oracle/pins.tsv`):

| Crate | Links | Checked by |
|---|---|---|
| `pgtk-oracle` | PostgreSQL's raw parser | `cargo test -p pgtk-sql --features postgres-oracle --test differential` |
| `pgtk-psql-oracle` | psql's `psqlscan.l` and `psqlscanslash.l` | `cargo test -p pgtk-psql --features psql-oracle` |

Each first build extracts the pinned commit from the `vendor/postgres` object
store and builds PostgreSQL. Fetch the pins first with
`scripts/fetch-postgres-pins`. See `docs/differential-baseline.md` and
`docs/psql-oracle.md`.

## Toolchain

pgtk-sql needs Rust 1.88 or later (`rust-version`). In the pgtk workspace,
`rust-toolchain.toml` pins the toolchain that CI uses. If your shell exports
`RUSTUP_TOOLCHAIN`, it overrides the file; unset it in this workspace.

## Flat AST

The grammar root selects Recursa's `flat` flag, so every `ast_node!`
declaration has a second concrete representation beside the nested arena AST:
one contiguous, handle-addressed store per parse. `Statement::parse_flat` and
`Statement::parse_flat_without_spans` sit beside `parse` and
`parse_without_spans`, run the same lexer and the same LR automaton, and
return a `FlatAst` that renders exactly what the detached nested AST renders.

`Expr`, `ColumnRef`, `FuncCall`, `SelectStmt` and `Statement` carry
`#[flat(pool)]`, which gives each its own typed pool; every other declaration
reaches the shared word pool. `tests/flat_parity.rs` proves nested/flat
recognition and rendering parity over every statement the differential
baseline pins, with no PostgreSQL oracle needed:

```bash
cargo test -p pgtk-sql --test flat_parity
cargo test -p pgtk-sql --features spans --test flat_parity
```

The design lives in Recursa's `docs/flat-ast-design.md`.

## Benchmarks

The `parse` bench measures pgtk-sql parser throughput and compares it against
its own Flat AST (the `pgtk-sql-flat` engine), `sqlparser-rs`
(`PostgreSqlDialect`), and the raw parser of the target version's pinned
PostgreSQL release (17.11 by default). `pgtk-sql` and
`pgtk-sql-flat` share the lex pass and the automaton, so the pair isolates the
representation; the harness refuses to report timings unless the two accept
exactly the same statements.

Both bench targets need the `postgres-oracle` feature. Without it, Cargo skips
the target and does not report an error. The first run builds the PostgreSQL
oracle, so it is slow.

Run the `parse` bench:

```bash
cargo bench -p pgtk-sql --features postgres-oracle --bench parse
```

The bench has no filter. It always runs all of these benchmarks:

- `corpus/<file>`: one benchmark for each file of the frozen PostgreSQL 17.9
  regression corpus (`src/test/regress/sql`). The bench reads the files from
  the `vendor/postgres` submodule, so that submodule must be checked out. It
  times only the statements that all four engines accept. The report counts
  the statements that it removes.
- `stress/<file>`: one benchmark for each file in `fixtures/stress/`. The
  shapes are `insert_values`, `bool_chain`, `select_list`, `nested_subquery`
  and `in_list`, each at a number of sizes.

For each engine, the bench warms up for 100 ms. Then it takes samples until it
has a minimum of 5 samples and 500 ms of time, and it reports the median. It
stops a benchmark after 3 s or 500 samples.

Each run writes `report.md`, `time.svg`, `throughput.svg` and `data.json` to
`docs/benchmarks/<timestamp>-<commit>/`, and prints the path of the report.
Set `PG_SQL_PARITY_MAX` to a ratio to make the run fail when the geomean of the
pgtk-sql/sqlparser times is more than that ratio. The release gate uses this.

### Profiling

The `flame` bench parses one workload in a loop for a fixed time, so that a
sampling profiler sees only the parser frames:

```bash
cargo bench -p pgtk-sql --features postgres-oracle --bench flame -- --list
cargo bench -p pgtk-sql --features postgres-oracle --bench flame -- corpus --duration 10
```

`--engine pgtk-sql-flat`, `--engine sqlparser` or `--engine postgres` times a
different engine. `--count-allocs` counts the allocations of one pass, and
does not time the loop. `scripts/flame-profile discovery` profiles each
workload of the discovery suite.

### Regenerating stress fixtures

Stress fixtures live in `fixtures/stress/` and are deterministic. Regenerate
with:

```bash
cargo run --bin gen-stress -p pgtk-sql
```
