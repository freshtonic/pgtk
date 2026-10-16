# pgtk-analyze

PostgreSQL parse analysis in Rust. The crate takes one parsed Statement from
[`pgtk-sql`](../pgtk-sql) and a Catalog from [`pgtk-catalog`](../pgtk-catalog), and
resolves every name, operator, function and type as PostgreSQL's
`parse_analyze` does. It answers what a Statement means without a live
server: the type of each expression, the columns each `*` expands to, the
Result Descriptor a Describe would return, and the Diagnostic PostgreSQL would
raise, with the same SQLSTATE, message and cursor position.

The target is fidelity, bug for bug, to the PostgreSQL version the
Target Version selects. The measure is the Oracle harness, which replays
PostgreSQL's own regression corpus against a live PostgreSQL 17 and against
this crate and compares the answers statement by statement.

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
To keep the build of `pgtk-sql` fast, set the build-override profile in your
workspace (see [Build time](../pgtk-sql/README.md#build-time) in the `pgtk-sql`
README).

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
