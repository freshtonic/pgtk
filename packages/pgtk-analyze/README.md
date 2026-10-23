# pgtk-analyze

**Know what your SQL means before the server sees it.**

[![crates.io](https://img.shields.io/crates/v/pgtk-analyze.svg)](https://crates.io/crates/pgtk-analyze)
[![docs.rs](https://img.shields.io/docsrs/pgtk-analyze)](https://docs.rs/pgtk-analyze)
[![MIT licence](https://img.shields.io/badge/licence-MIT-blue.svg)](https://github.com/freshtonic/pgtk/blob/main/LICENSE)

pgtk-analyze is PostgreSQL parse analysis in Rust: the type checker of PostgreSQL, without
PostgreSQL. It takes one statement from [`pgtk-sql`](https://crates.io/crates/pgtk-sql) and one
catalog from [`pgtk-catalog`](https://crates.io/crates/pgtk-catalog). Then it resolves each name,
operator, function and type as PostgreSQL's `parse_analyze` does.

It tells you what a live server would tell you:

- the type of each expression,
- the columns that each `*` expands to,
- the Result Descriptor that a Describe would return: the name, type and source column of each
  result column, and the type of each parameter,
- the error that PostgreSQL would raise, with the same SQLSTATE, message and cursor position.

pgtk-analyze is part of [pgtk](https://github.com/freshtonic/pgtk), the PostgreSQL toolkit for
Rust.

## Why pgtk-analyze?

- **Server answers, no server.** No container in CI, no database to migrate, no connection pool.
  Analysis is a function call.
- **Bug-for-bug fidelity.** The goal is not "close to PostgreSQL". The goal is the same answer
  as PostgreSQL, for each statement, including the strange cases.
- **Proven on PostgreSQL's own tests.** The Oracle harness replays PostgreSQL's regression suite
  against a live PostgreSQL 17 server and against this crate. **41,732** statements get the same
  answer, with **0 disagreements** and **0 unsupported** statements. Most of the 818 known
  divergences are errors that PostgreSQL raises outside parse analysis, for example when a literal is not
  valid input for its type.
- **Errors that users understand.** A typo in a column name gives the same `42703 column
  "relnmae" does not exist` as the server, at the same position.
- **Version aware.** Analyse for the server that the SQL will run on, from PostgreSQL 14.

## What it does

- Resolves column references, whole-row references, subqueries, WITH queries
  (recursive ones included), joins, VALUES, set operations, window
  definitions, grouping sets and locking clauses.
- Resolves operators and functions with PostgreSQL's overload rules:
  implicit coercions, polymorphism, named arguments, defaults, VARIADIC,
  aggregates with ORDER BY, DISTINCT, FILTER and WITHIN GROUP, window
  functions and set-returning functions.
- Analyses INSERT, UPDATE, DELETE and MERGE, with RETURNING and ON CONFLICT,
  and reproduces the part of the rewriter that runs before a Describe: a
  data-modifying Statement on a view, generated and identity columns, and
  rules with RETURNING.
- Analyses the SQL/JSON functions, JSON_TABLE, XMLTABLE, the XML expressions,
  CALL, SHOW and the utility statements PostgreSQL transforms, such as
  EXPLAIN, DECLARE and CREATE TABLE AS.
- Assigns collations as `parse_collate.c` does and reports collation
  mismatches at the points where PostgreSQL reports them.
- Answers EXECUTE, FETCH and SHOW of a custom parameter from the Session the
  caller supplies, and reports what a Statement would add to that Session or
  remove from it.

What it leaves unanalysed is what PostgreSQL leaves unanalysed at Parse time:
the query inside CREATE VIEW, CREATE RULE, PREPARE and COPY.

## Use

```rust
use pgtk_analyze::{AnalyzeError, Analyzer, Session, Settings, TargetVersion};

let catalog = pgtk_catalog::fixture::pg17(); // or load one from a server
// The Target Version is the server the SQL will run on. It must not be
// newer than the Grammar Version this build parses with.
let settings = Settings::new(&catalog, TargetVersion::Pg17, ["public"])
    .expect("the build parses PostgreSQL 17 or newer");
let analyzer = Analyzer::new(&catalog, settings);
let mut session = Session::new(); // empty: no prepared statement, no cursor

let sql = "SELECT relname, length(relname) FROM pg_class";
match analyzer.analyze_sql(sql, &[], &session) {
    Ok(analysis) => {
        for column in &analysis.result.columns {
            println!("{} {}", column.name, catalog[column.ty].name);
        }
        // What the Statement would change in the session, for the caller
        // to fold in before it analyses the next one.
        if let Some(effect) = &analysis.session_effect {
            session.apply(effect);
        }
    }
    Err(AnalyzeError::Diagnostic(d)) => println!("{} {}", d.sqlstate, d.message),
    Err(AnalyzeError::Unsupported(u)) => println!("unsupported: {}", u.what),
}
```

`analyze` takes a Statement already parsed by `pgtk-sql`; `analyze_sql` parses
the text first. The result is an `Analysis`: Annotation Tables keyed by the
Statement's own node handles (expression types, coercions, resolved columns,
operators and functions, star expansions, range tables and target lists),
the Result Descriptor and the Session Effect. A rejection is a `Diagnostic`
when PostgreSQL would reject the Statement, or an `Unsupported` when this
crate cannot analyse it yet; an Unsupported is never a wrong answer.

The `Session` is the state a Describe reads: the prepared statements, the
open cursors and the custom configuration placeholders a SET made. EXECUTE
answers with the prepared statement's Result Descriptor, analysing the
prepared query again as PostgreSQL's plan cache does; FETCH answers with the
cursor's descriptor, and SHOW of a placeholder with its name. A prepared
statement or a cursor the Session does not hold answers with an empty Result
Descriptor, as PostgreSQL's Describe does; PostgreSQL raises the error when
the Statement runs. A configuration parameter that does not exist is the
Diagnostic PostgreSQL raises for it. Analysis never changes the Session:
PREPARE, DECLARE, SET, DEALLOCATE, CLOSE and the end of a transaction report
a Session Effect, and the caller applies it.

The crate depends on `pgtk-sql`, `pgtk-catalog` and `recursa`.
To make the build of `pgtk-sql` faster, you can set the build-override profile
in your workspace (see "Build time" in the
[`pgtk-sql` README](https://crates.io/crates/pgtk-sql)).

## The Oracle harness

`tests/oracle.rs` replays the vendored regression corpus against live
PostgreSQL servers and pins the per-file counts in `baselines/`. The servers
are the pinned PostgreSQL 17 release, built from the vendored source
(`pgtk-test-server`). The harness starts one server for each file it replays.
It is part of the default suite:

```sh
cargo test --release --test oracle
```

`docs/oracle-harness.md` describes what is compared and the verdicts. The
smoke tests in `tests/smoke.rs` run against the bundled Catalog and need no
server.

## Documents

- `CONTEXT.md`: the language of the crate. Analysis, Statement, Catalog,
  Search Path, Settings, Session, Session Effect, Target Version, Annotation
  Table, Result Descriptor, Diagnostic, Unsupported, Rewrite, Oracle.
- `docs/adr/`: the decisions. Annotations over the flat AST rather than a
  resolved tree; literal text is never validated by the crate; one entry
  point per PostgreSQL version; the Session is an input and Analysis reports
  Session Effects.
- `docs/upstream-work.md`: what the sibling crates provide and what is still
  open there.

## Licence

MIT.
