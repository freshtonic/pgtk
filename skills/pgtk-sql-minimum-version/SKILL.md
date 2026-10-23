---
name: pgtk-sql-minimum-version
description: Find the oldest PostgreSQL version (14 to 19 beta) that accepts a SQL statement, with the pgtk-sql Rust crate and no server. Also find the first clause that an older server rejects, with the SQLSTATE and message that server gives. Use when the user asks which PostgreSQL version a query or migration needs, wants a CI check that SQL runs on an older server, or asks about PostgreSQL version compatibility of SQL in Rust.
license: MIT
compatibility: Needs a Rust project that can depend on pgtk-sql 0.1 (Rust 1.88 or later).
---

# Find the minimum PostgreSQL version of a statement

pgtk-sql records each grammar change of PostgreSQL 15 to 19 as a version gate. A scan of a parsed
statement gives the first construct that needs a newer version than the one you give. Two scans
are necessary:

- `MinimumVersion::minimum_version_above(target)` on the parse: the syntax (clauses, statement
  kinds, optional parts that became optional).
- `pgtk_sql::minimum_version::lexical_minimum_version_above(tokens, target)` on the tokens: the
  lexical changes, for example `1_000` (PostgreSQL 16).

## Dependencies

```toml
[dependencies]
pgtk-sql = { version = "0.1", default-features = false, features = ["pg18", "spans"] }
```

- **Build with the newest grammar that you need.** A build parses the SQL of its own version
  only (the default is `pg17`). A `pg17` build cannot parse SQL that only PostgreSQL 18 accepts,
  so it cannot tell you that the SQL needs 18. Use the version feature of the newest server
  that you support, and check against the oldest server.
- Use exactly one version feature: `pg14`, `pg15`, `pg16`, `pg17`, `pg18` or `pg19-beta`.
- `spans` gives the location of the construct, and `pgtk_sql::document::parse_sql` for a file.

## Steps

1. Parse one statement with `Statement::parse(&mut lex(sql).input())`, or a file with
   `pgtk_sql::document::parse_sql`. Keep the parse value (the `ArenaParsed`), not only the AST,
   because the scan reads the gates from it.
2. Call `minimum_version_above(server)` on the parse. `None` means that `server` accepts the
   syntax. `minimum_version()` gives the lowest version that accepts all of the statement.
3. Call `lexical_minimum_version_above(lex(text).tokens(), server)` on the tokens of the same
   statement. Give it the tokens of one statement only.
4. Report the result that comes first in the source. A `VersionRequirement` gives `version()`,
   `span()`, `sqlstate()`, `message()` and `hint()`. When `message()` is `None`, the older
   server gives a plain syntax error.

## Example: one statement

```rust
use pgtk_sql::{MinimumVersion, TargetVersion, ast::Statement, lex};

let lexed = lex("SELECT * FROM (SELECT 1)");
let parsed = Statement::parse(&mut lexed.input()).expect("PostgreSQL 17 accepts it");

// PostgreSQL 15 needs an alias on a subquery in FROM. PostgreSQL 16 does not.
let found = parsed.minimum_version_above(TargetVersion::Pg15).expect("15 rejects it");
assert_eq!(found.version(), TargetVersion::Pg16);
assert_eq!(found.sqlstate(), Some("42601"));
assert_eq!(found.message(), Some("subquery in FROM must have an alias"));

// PostgreSQL 16 accepts it.
assert!(parsed.minimum_version_above(TargetVersion::Pg16).is_none());
```

## Example: check a file against a server version

```rust
use pgtk_sql::minimum_version::lexical_minimum_version_above;
use pgtk_sql::{MinimumVersion, TargetVersion, VersionRequirement, lex};

/// Each statement of `source` that `server` cannot parse: the byte offset of the construct in
/// `source`, and what it needs.
fn too_new(source: &str, server: TargetVersion) -> Result<Vec<(usize, VersionRequirement)>, String> {
    let document = pgtk_sql::document::parse_sql(source).map_err(|error| error.to_string())?;
    let mut found = Vec::new();
    for item in document.items() {
        if item.ast().statement.is_none() {
            continue; // an empty statement
        }
        let Some(bounds) = item.source_bounds() else {
            continue;
        };
        let start = bounds.start() as usize;
        let text = &source[start..bounds.end() as usize];
        // The syntactic scan of the parse. Its span is an offset in `source`.
        let syntactic = item
            .minimum_version_above(server)
            .map(|req| (req.span().map_or(start, |span| span.start() as usize), req));
        // The lexical scan of the tokens of this statement. Its span is an offset in `text`.
        let lexical = lexical_minimum_version_above(lex(text).tokens(), server)
            .map(|req| (start + req.span().map_or(0, |span| span.start() as usize), req));
        // The construct that comes first is the one that the server reports.
        if let Some(first) = [syntactic, lexical].into_iter().flatten().min_by_key(|(at, _)| *at) {
            found.push(first);
        }
    }
    Ok(found)
}

let migration = "\
SELECT 1;
MERGE INTO t USING s ON t.a = s.a WHEN MATCHED THEN DELETE;
SELECT 1_000;
";
let found = too_new(migration, TargetVersion::Pg14).expect("PostgreSQL 17 accepts the file");
for (at, req) in &found {
    eprintln!(
        "byte {at}: needs PostgreSQL {} ({})",
        req.version().major(),
        req.message().unwrap_or("syntax error in the older server"),
    );
}
// MERGE needs 15, and the digit separator needs 16.
let versions: Vec<_> = found.iter().map(|(_, req)| req.version()).collect();
assert_eq!(versions, [TargetVersion::Pg15, TargetVersion::Pg16]);
assert_eq!(found[0].0, migration.find("MERGE").unwrap());
```

## Pitfalls

- If the parse fails, the SQL is not valid for the grammar of the build. It can be valid for a
  newer version. Build with a newer version feature to find out.
- `pgtk_sql::TARGET_VERSION` is the grammar of the build. A check against a version newer than
  that has no meaning.
- `minimum_version_above` is on the parse value (`ArenaParsed`). A detached `Statement`, for
  example from `document.statements()`, does not have it. Use `document.items()`.
- The answer covers the grammar (`gram.y` and `scan.l`) only. A function, a type or a catalog
  change of a newer version is not a grammar change. To check those, analyse the statement with
  `pgtk-analyze` and a catalog of the older server.
