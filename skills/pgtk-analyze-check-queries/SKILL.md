---
name: pgtk-analyze-check-queries
description: Check the SQL queries of an application in CI, with no database server, with the pgtk-analyze Rust crate and a saved PostgreSQL catalog snapshot (JSON). Each query is type-checked as PostgreSQL does, and each error is printed with its SQLSTATE, message, file, line and column. Use when the user wants to validate .sql files or embedded queries in CI, catch a misspelt column or a type error before deployment, or check queries against a schema without a running PostgreSQL.
license: MIT
compatibility: Needs a Rust project that can depend on pgtk-analyze 0.1, pgtk-catalog 0.1 (serde feature), pgtk-sql 0.1 (spans feature) and serde_json (Rust 1.88 or later).
---

# Check the queries of an application against a catalog snapshot

The CI job reads a catalog snapshot (a JSON file that a developer saved from the database, see
the `pgtk-catalog-snapshot` skill). Then it analyses each query with pgtk-analyze. PostgreSQL is
not necessary in CI.

## Dependencies

```toml
[dependencies]
pgtk-analyze = "0.1"
pgtk-catalog = { version = "0.1", features = ["serde"] }
pgtk-sql = { version = "0.1", features = ["spans"] }
serde_json = "1"
```

- `pgtk-catalog/serde` reads the snapshot. `pgtk-sql/spans` gives `parse_sql`, which divides a
  file into statements.
- Use the same version feature (`pg14` ... `pg19-beta`, default `pg17`) on pgtk-analyze and on
  pgtk-sql. Use `default-features = false` on both to select a version other than 17.

## Steps

1. Read the snapshot: `serde_json::from_reader::<_, pgtk_catalog::Catalog>(file)`.
2. Make the `Settings` and the `Analyzer` once. Use the Target Version and the search path of the
   production server.
3. For each `.sql` file, divide it into statements with `pgtk_sql::document::parse_sql`. Use
   `document.items()` and `item.source_bounds()` to get the text and the byte offset of each
   statement. A parse error is an error of the file.
4. Analyse each statement with `analyzer.analyze_sql(text, &[], &session)`. If a statement
   changes the session (`PREPARE`, `DECLARE`, `SET`), apply `analysis.session_effect` before the
   next statement of the same file.
5. For an `AnalyzeError::Diagnostic`, convert `d.position` (a 1-based character index in the
   statement text) to a line and a column of the file. Print the SQLSTATE and the message.
6. For an `AnalyzeError::Unsupported`, print a warning, not an error: the SQL can be correct.
7. Exit with a failure status if there is one or more errors.

## Example

```rust
use pgtk_analyze::{AnalyzeError, Analyzer, Session, Settings, TargetVersion};

/// One problem in one file.
struct Problem {
    line: usize,
    column: usize,
    text: String,
    is_error: bool,
}

/// The 1-based line and column of a byte offset in `source`.
fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, before[line_start..].chars().count() + 1)
}

/// Analyse each statement of one SQL file.
fn check_file(analyzer: &Analyzer<'_>, source: &str) -> Vec<Problem> {
    let document = match pgtk_sql::document::parse_sql(source) {
        Ok(document) => document,
        Err(error) => {
            return vec![Problem { line: 1, column: 1, text: error.to_string(), is_error: true }];
        }
    };
    let mut session = Session::new();
    let mut problems = Vec::new();
    for item in document.items() {
        let (Some(_), Some(bounds)) = (&item.ast().statement, item.source_bounds()) else {
            continue; // an empty statement
        };
        let start = bounds.start() as usize;
        // The extent of the statement includes its `;`.
        let text = source[start..bounds.end() as usize].trim_end_matches(';');
        match analyzer.analyze_sql(text, &[], &session) {
            Ok(analysis) => {
                if let Some(effect) = &analysis.session_effect {
                    session.apply(effect);
                }
            }
            Err(AnalyzeError::Diagnostic(d)) => {
                // `position` counts characters from 1 in the statement text.
                let in_text = d.position.map_or(0, |p| {
                    text.char_indices().nth(p as usize - 1).map_or(text.len(), |(i, _)| i)
                });
                let (line, column) = line_column(source, start + in_text);
                let text = format!("ERROR {}: {}", d.sqlstate, d.message);
                problems.push(Problem { line, column, text, is_error: true });
            }
            Err(AnalyzeError::Unsupported(u)) => {
                let (line, column) = line_column(source, start);
                let text = format!("not checked: pgtk-analyze does not support {}", u.what);
                problems.push(Problem { line, column, text, is_error: false });
            }
        }
    }
    problems
}

// In CI, read the snapshot file:
//     let catalog: pgtk_catalog::Catalog =
//         serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))?;
// This example makes the same JSON from the built-in catalog.
let json = serde_json::to_string(&pgtk_catalog::fixture::pg17()).unwrap();
let catalog: pgtk_catalog::Catalog = serde_json::from_str(&json).expect("a catalog snapshot");

let settings = Settings::new(&catalog, TargetVersion::Pg17, ["public"]).unwrap();
let analyzer = Analyzer::new(&catalog, settings);

let good = "SELECT relname FROM pg_class WHERE oid = $1;\nSELECT count(*) FROM pg_namespace;\n";
assert!(check_file(&analyzer, good).is_empty());

let bad = "SELECT relname FROM pg_class;\nSELECT nspnmae\n  FROM pg_namespace;\n";
let problems = check_file(&analyzer, bad);
for p in &problems {
    eprintln!("queries/bad.sql:{}:{}: {}", p.line, p.column, p.text);
}
assert_eq!(problems.len(), 1);
assert!(problems[0].is_error);
assert_eq!((problems[0].line, problems[0].column), (2, 8));
assert_eq!(problems[0].text, "ERROR 42703: column \"nspnmae\" does not exist");
```

To run it as a CI step, put `check_file` in a small binary. The binary reads the snapshot path
and the SQL directory from its arguments, calls `check_file` for each `.sql` file, prints each
problem as `path:line:column: text`, and calls `std::process::exit(1)` when a problem has
`is_error`.

## Pitfalls

- Save a new snapshot after each migration. An old snapshot gives errors for new columns, and
  hides errors for removed columns.
- The Target Version must not be newer than the grammar of the build
  (`pgtk_analyze::GRAMMAR_VERSION`). To check SQL for an older server, the build can stay new:
  `analyze_sql` rejects each construct that the Target Version cannot parse.
- pgtk-analyze does not reproduce PostgreSQL's syntax errors: `parse_sql` rejects the file. Its
  message is from pgtk-sql, not from PostgreSQL.
- Analysis does not run the query. Errors that PostgreSQL raises at execution (for example, a
  literal that is not valid input for its type, or a constraint violation) are not found.
- Queries in Rust source (strings) are not in `.sql` files. Collect them in a test that calls
  `analyze_sql` for each one, or move them to `.sql` files.
