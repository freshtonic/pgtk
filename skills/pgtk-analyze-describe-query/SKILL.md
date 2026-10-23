---
name: pgtk-analyze-describe-query
description: Get the result columns (name, type, source table column) and the parameter types ($1, $2, ...) of a PostgreSQL query in Rust with the pgtk-analyze crate, with no database server. It gives the same answer as a Describe message to PostgreSQL, or psql's \gdesc. Use when the user wants the types of a query, wants to generate Rust types from SQL, wants to type-check SQL without a database, or asks what a query returns.
license: MIT
compatibility: Needs a Rust project that can depend on pgtk-analyze 0.1 and pgtk-catalog 0.1 (Rust 1.88 or later).
---

# Describe a query with pgtk-analyze

pgtk-analyze does PostgreSQL's parse analysis in Rust. It resolves each name, operator, function
and type of one statement against a catalog. The Result Descriptor of the Analysis is what a
server returns for a Describe: the result columns and the parameter types.

## Dependencies

```toml
[dependencies]
pgtk-analyze = "0.1"
# `fixture`: the built-in catalog of a stock PostgreSQL 17 database.
pgtk-catalog = { version = "0.1", features = ["fixture"] }
```

- The catalog decides the answer. The built-in catalog (`pgtk_catalog::fixture::pg17()`) holds
  only the system tables, types and functions. For the tables of an application, load the
  catalog of its database: see the `pgtk-catalog-snapshot` skill.
- pgtk-analyze parses with the default grammar of pgtk-sql, PostgreSQL 17. For a different
  release, use `default-features = false` and one of `pg14` ... `pg19-beta` on pgtk-analyze.

## Steps

1. Get a `Catalog`.
2. Make `Settings::new(&catalog, target_version, search_path)`. The Target Version is the server
   that runs the SQL. It must not be newer than the grammar of the build
   (`pgtk_analyze::GRAMMAR_VERSION`), else `new` gives an error.
3. Make `Analyzer::new(&catalog, settings)` once, and use it for each query.
4. Call `analyzer.analyze_sql(sql, &declared_params, &Session::new())`. The text is **one**
   statement. `declared_params` gives a type for `$1`, `$2`, ... (`Some(type)`), or `None` to
   let analysis infer it. Use `&[]` to infer all.
5. Read `analysis.result`:
   - `result.columns`: each `ResultColumn` has `name`, `ty` (a type handle), `typmod` and
     `origin` (`Some((relation, position))` when the value comes directly from a table column).
   - `result.params`: the type of each parameter, in `$n` order.
   - Use `catalog[handle]` to read a type or a relation, and
     `catalog.qualified_name_of_type(ty)` for a name with its schema.
6. Handle the two errors:
   - `AnalyzeError::Diagnostic(d)`: PostgreSQL rejects the query. `d.sqlstate`, `d.message`,
     `d.position` (1-based character, as PostgreSQL reports it), `d.detail` and `d.hint`.
   - `AnalyzeError::Unsupported(u)`: pgtk-analyze cannot analyse the query yet. This is not a
     wrong answer. Do not report it as an error of the user's SQL.

## Example

```rust
use pgtk_analyze::{AnalyzeError, Analyzer, Session, Settings, TargetVersion};
use pgtk_catalog::Builtin;

let catalog = pgtk_catalog::fixture::pg17();
let settings = Settings::new(&catalog, TargetVersion::Pg17, ["public"])
    .expect("the grammar of the build is PostgreSQL 17 or newer");
let analyzer = Analyzer::new(&catalog, settings);
let session = Session::new();

let sql = "SELECT c.relname, c.relpages * 8192 AS bytes FROM pg_class c WHERE c.oid = $1";
let analysis = analyzer.analyze_sql(sql, &[], &session).expect("PostgreSQL accepts it");

for column in &analysis.result.columns {
    let ty = catalog.qualified_name_of_type(column.ty);
    // The table column that the value comes from, when there is one.
    let source = column.origin.map(|(relation, position)| {
        let relation = &catalog[relation];
        let name = relation.column_at(position).map(|c| c.name.to_string());
        format!("{}.{}", relation.name, name.unwrap_or_default())
    });
    println!("{} {ty} {source:?}", column.name);
}
let names: Vec<_> = analysis.result.columns.iter().map(|c| c.name.as_str()).collect();
assert_eq!(names, ["relname", "bytes"]);
assert_eq!(catalog.as_builtin(analysis.result.columns[1].ty), Some(Builtin::Int4));
assert!(analysis.result.columns[0].origin.is_some());
assert!(analysis.result.columns[1].origin.is_none());

// $1 is compared with an oid column, so its type is oid.
assert_eq!(analysis.result.params.len(), 1);
assert_eq!(catalog.as_builtin(analysis.result.params[0]), Some(Builtin::Oid));

// Declare the type of a parameter, as a client does in a Parse message.
let int8 = catalog.builtin(Builtin::Int8);
let declared = analyzer
    .analyze_sql("SELECT $1 + 1", &[Some(int8)], &session)
    .expect("PostgreSQL accepts it");
assert_eq!(catalog.as_builtin(declared.result.columns[0].ty), Some(Builtin::Int8));

// A parameter with no context has no type, and PostgreSQL rejects the query. Declare its type.
match analyzer.analyze_sql("SELECT $1", &[], &session) {
    Err(AnalyzeError::Diagnostic(d)) => assert_eq!(d.sqlstate, "42P18"),
    other => panic!("expected a Diagnostic, got {other:?}"),
}

// A query that PostgreSQL rejects gives the same error as the server.
match analyzer.analyze_sql("SELECT relnmae FROM pg_class", &[], &session) {
    Err(AnalyzeError::Diagnostic(d)) => {
        assert_eq!(d.sqlstate, "42703");
        assert_eq!(d.message, "column \"relnmae\" does not exist");
        assert_eq!(d.position, Some(8));
    }
    other => panic!("expected a Diagnostic, got {other:?}"),
}
```

## Pitfalls

- `analyze_sql` takes one statement. For a file of statements, see the
  `pgtk-analyze-check-queries` skill.
- A statement that pgtk-sql cannot parse is `Unsupported`, not a `Diagnostic`: pgtk-analyze does
  not reproduce PostgreSQL's syntax errors.
- A Utility Statement (for example `CREATE TABLE`) has `analysis.utility == true` and an empty
  Result Descriptor.
- The search path must name the schemas of the application, for example `["app", "public"]`.
  When the list does not name `pg_catalog`, it is searched first, as in PostgreSQL.
- Analysis never changes the `Session`. After `PREPARE`, `DECLARE` or `SET`, apply
  `analysis.session_effect` with `session.apply(effect)` before you analyse the next statement.
- When analysis cannot infer the type of a parameter, it gives the error of PostgreSQL:
  `42P18 could not determine data type of parameter $1`. Declare the type in `declared_params`.
