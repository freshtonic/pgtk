---
name: pgtk-sql-check-migration
description: Lint PostgreSQL migration files in Rust with the pgtk-sql crate. Parse each statement exactly as PostgreSQL does, then walk the typed AST with a recursa Visitor to find unsafe changes, such as CREATE INDEX without CONCURRENTLY, DROP COLUMN, or UPDATE and DELETE with no WHERE. Use when the user wants a migration linter, a schema-change check in CI, or a custom SQL lint rule for PostgreSQL in Rust.
license: MIT
compatibility: Needs a Rust project that can depend on pgtk-sql 0.1 and recursa 0.1 (Rust 1.88 or later).
---

# Lint a migration with pgtk-sql

pgtk-sql parses SQL with the grammar of one PostgreSQL release. Each AST type is a grammar rule,
and each type derives `recursa::Visit`. A lint rule is a `Visitor` for the node types that the
rule examines. You do not write a recursive walk.

## Dependencies

```toml
[dependencies]
pgtk-sql = { version = "0.1", features = ["spans"] }
# The derive macro and the visitor traits. Use the version that pgtk-sql uses.
recursa = "0.1"
```

- The `spans` feature is necessary for `pgtk_sql::document::parse_sql`, which parses a file of
  statements.
- The default grammar is PostgreSQL 17 (`pg17`). To select a different release, use
  `default-features = false` and one of `pg14` ... `pg19-beta`. Use the release of your server.
- The build script of pgtk-sql takes about one minute at `opt-level = 0`. Add
  `[profile.dev.build-override] opt-level = 3` to the workspace root `Cargo.toml` to make it
  approximately 7 seconds.

## Steps

1. Parse the file with `pgtk_sql::document::parse_sql`. An error means that PostgreSQL rejects
   the SQL. Report the error and stop. Do not lint SQL that does not parse.
2. Make a struct for the lint. Derive `recursa::TotalVisitor` on it. In
   `#[total_visitor(...)]`, give:
   - `dispatch = [...]`: the full path of each node type that the lint examines, with
     `<'static>`.
   - `error = ...`: the error type of the visitor.
   - `event = pgtk_sql::__RecursaVisitEvent`: always this value for pgtk-sql.
3. Implement `recursa::Visitor<NodeType<'input>>` for each type in `dispatch`. Read the typed
   fields of the node in `enter`, and return `ControlFlow::Continue(())`.
4. Call `recursa::Visit::visit` on each statement with the visitor.

## Example

```rust
use std::convert::Infallible;
use std::ops::ControlFlow;

use pgtk_sql::ast::ddl::index::CreateIndexStmt;
use pgtk_sql::ast::ddl::table::DropColumnCmd;
use pgtk_sql::ast::dml::delete::DeleteStmt;
use pgtk_sql::ast::dml::update::UpdateStmt;
use recursa::{Visit, VisitBreak, Visitor};

/// The findings for one statement.
#[derive(Default, recursa::TotalVisitor)]
#[total_visitor(
    dispatch = [
        pgtk_sql::ast::ddl::index::CreateIndexStmt<'static>,
        pgtk_sql::ast::ddl::table::DropColumnCmd<'static>,
        pgtk_sql::ast::dml::delete::DeleteStmt<'static>,
        pgtk_sql::ast::dml::update::UpdateStmt<'static>,
    ],
    error = Infallible,
    event = pgtk_sql::__RecursaVisitEvent,
)]
struct MigrationLint {
    findings: Vec<String>,
}

type Flow = ControlFlow<VisitBreak<Infallible>>;

impl<'input> Visitor<CreateIndexStmt<'input>> for MigrationLint {
    type Error = Infallible;
    fn enter(&mut self, node: &CreateIndexStmt<'input>) -> Flow {
        if !node.concurrently {
            self.findings
                .push("CREATE INDEX without CONCURRENTLY locks the table against writes".into());
        }
        ControlFlow::Continue(())
    }
}

impl<'input> Visitor<DropColumnCmd<'input>> for MigrationLint {
    type Error = Infallible;
    fn enter(&mut self, node: &DropColumnCmd<'input>) -> Flow {
        let column = node.name.text();
        self.findings
            .push(format!("DROP COLUMN {column} breaks the code that reads the column"));
        ControlFlow::Continue(())
    }
}

impl<'input> Visitor<UpdateStmt<'input>> for MigrationLint {
    type Error = Infallible;
    fn enter(&mut self, node: &UpdateStmt<'input>) -> Flow {
        if node.where_clause.is_none() {
            self.findings.push("UPDATE with no WHERE changes each row".into());
        }
        ControlFlow::Continue(())
    }
}

impl<'input> Visitor<DeleteStmt<'input>> for MigrationLint {
    type Error = Infallible;
    fn enter(&mut self, node: &DeleteStmt<'input>) -> Flow {
        if node.where_clause.is_none() {
            self.findings.push("DELETE with no WHERE removes each row".into());
        }
        ControlFlow::Continue(())
    }
}

/// Lint one migration file. Each finding is `(byte offset of the statement, message)`.
fn lint(source: &str) -> Result<Vec<(usize, String)>, String> {
    let document = pgtk_sql::document::parse_sql(source).map_err(|error| error.to_string())?;
    let mut findings = Vec::new();
    for item in document.items() {
        // An empty statement (a lone `;`) has no Statement.
        let Some(statement) = &item.ast().statement else {
            continue;
        };
        let offset = item.source_bounds().map_or(0, |span| span.start() as usize);
        let mut lint = MigrationLint::default();
        // The visitor never breaks, so the result is always `Continue`.
        let _ = statement.visit(&mut lint);
        findings.extend(lint.findings.into_iter().map(|message| (offset, message)));
    }
    Ok(findings)
}

let migration = "\
CREATE INDEX users_email ON users (email);
CREATE INDEX CONCURRENTLY users_name ON users (name);
ALTER TABLE users DROP COLUMN legacy_id;
DELETE FROM sessions;
";
let findings = lint(migration).expect("PostgreSQL accepts the migration");
for (offset, message) in &findings {
    println!("{offset}: {message}");
}
assert_eq!(findings.len(), 3);
assert_eq!(findings[0].0, 0);
assert!(findings[1].1.contains("legacy_id"));

// SQL that PostgreSQL rejects is an error, not a finding.
assert!(lint("CREATE INDEX ON;").is_err());
```

## Find the node type of a rule

- The AST types are in `pgtk_sql::ast`: `ddl` (CREATE, ALTER, DROP), `dml` (SELECT, INSERT,
  UPDATE, DELETE, MERGE), `tcl` (transactions), `session`, `cursor` and `utility`. Each type is
  declared with `recursa::ast_node!` in the source of pgtk-sql, and its doc comment gives the
  `gram.y` rule.
- A struct is a sequence: its fields are the parts of the rule. An enum is a choice: its
  variants are the alternatives. `Option<T>` is an optional part. A `bool` field such as
  `concurrently` is a keyword that is present or not.
- Search the source for the keyword. For example, `ALTER TABLE ... ADD COLUMN` is
  `pgtk_sql::ast::ddl::table::AlterTableCmd`, and its variants hold the command types.

## Pitfalls

- A dispatch type must be the path where the type is declared, not a re-export, because the
  derive computes the event variant from the path.
- Do not give `Visit::visit` an `ArenaParsed` value. Give it the statement:
  `item.ast().statement` or an element of `document.statements()`.
- A field with `#[config(since = pgN)]` exists only in the builds of PostgreSQL N and later. Code
  that reads it must use the same version feature.
- Return `ControlFlow::Break(VisitBreak::SkipChildren)` from `enter` to skip the children of a
  node, and `VisitBreak::Finished` to stop the walk.
- `parse_sql` does not accept psql meta-commands such as `\set`. Render a psql script with
  `pgtk-psql` first.
