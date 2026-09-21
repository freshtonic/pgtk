# pg-analyze

PostgreSQL parse analysis in Rust. The crate takes one parsed Statement from
[`pg-sql`](../pg-sql) and a Catalog from [`pg-catalog`](../pg-catalog), and
resolves every name, operator, function and type as PostgreSQL's
`parse_analyze` does. It answers what a Statement means without a live
server: the type of each expression, the columns each `*` expands to, the
Result Descriptor a Describe would return, and the Diagnostic PostgreSQL would
raise, with the same SQLSTATE, message and cursor position.

The target is fidelity, bug for bug, to PostgreSQL 14 to 17 as the
`TargetVersion` selects. The measure is the Oracle harness, which replays
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

What it leaves unanalysed is what PostgreSQL leaves unanalysed at Parse time:
the query inside CREATE VIEW, CREATE RULE, PREPARE and COPY. EXECUTE and
FETCH need session state and are not supported.

## Use

```rust
use pg_analyze::{AnalyzeError, Analyzer, Settings, TargetVersion};

let catalog = pg_catalog::fixture::pg17(); // or load one from a server
let settings = Settings::new(&catalog, TargetVersion::V17, ["public"]);
let analyzer = Analyzer::new(&catalog, settings);

match analyzer.analyze_sql("SELECT relname, length(relname) FROM pg_class", &[]) {
    Ok(analysis) => {
        for column in &analysis.result.columns {
            println!("{} {}", column.name, catalog[column.ty].name);
        }
    }
    Err(AnalyzeError::Diagnostic(d)) => println!("{} {}", d.sqlstate, d.message),
    Err(AnalyzeError::Unsupported(u)) => println!("unsupported: {}", u.what),
}
```

`analyze` takes a Statement already parsed by `pg-sql`; `analyze_sql` parses
the text first. The result is an `Analysis`: Annotation Tables keyed by the
Statement's own node handles (expression types, coercions, resolved columns,
operators and functions, star expansions, range tables and target lists)
and the Result Descriptor. A rejection is a `Diagnostic` when PostgreSQL
would reject the Statement, or an `Unsupported` when this crate cannot
analyse it yet; an Unsupported is never a wrong answer.

The crate depends on the sibling checkouts `../pg-sql`, `../pg-catalog` and
`../recursa`.

## The Oracle harness

`tests/oracle.rs` replays the vendored regression corpus against a live
server and pins the per-file counts in `baselines/`. It needs a PostgreSQL 17
with the `regression` database:

```sh
PG_ANALYZE_ORACLE_URL=postgres://oracle:oracle@localhost:54317/regression \
cargo test --release --test oracle
```

`docs/oracle-harness.md` describes what is compared and the verdicts. The
smoke tests in `tests/smoke.rs` run against the bundled Catalog and need no
server.

## Documents

- `CONTEXT.md`: the language of the crate. Analysis, Statement, Catalog,
  Search Path, Settings, Target Version, Annotation Table, Result Descriptor,
  Diagnostic, Unsupported, Rewrite, Oracle.
- `docs/adr/`: the decisions. Annotations over the flat AST rather than a
  resolved tree; literal text is never validated by the crate; one entry
  point per PostgreSQL version.
- `docs/upstream-work.md`: what the sibling crates provide and what is still
  open there.

## Licence

MIT.
