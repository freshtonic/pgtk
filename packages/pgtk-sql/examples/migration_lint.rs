//! A linter for database migrations, written as a visitor over the typed AST.
//!
//! Each rule finds a statement that can lock a busy table for a long time, or that changes
//! more rows than the author wanted:
//!
//! | Rule | Finds |
//! |---|---|
//! | `index-not-concurrent` | `CREATE INDEX` without `CONCURRENTLY` |
//! | `add-column-not-null` | `ADD COLUMN ... NOT NULL` with no `DEFAULT` |
//! | `drop-column` | `DROP COLUMN` |
//! | `alter-column-type` | `ALTER COLUMN ... TYPE` |
//! | `update-without-where` | `UPDATE` with no `WHERE` |
//! | `delete-without-where` | `DELETE` with no `WHERE` |
//!
//! A rule is a `match` on the typed nodes. The AST is the grammar, so a field such as
//! `CreateIndexStmt::concurrently` or `UpdateStmt::where_clause` is the clause itself, and the
//! compiler tells you the forms that a rule must handle. The visitor finds each node at any
//! depth, for example an `UPDATE` inside `EXPLAIN` or inside a `WITH` query.
//!
//! ```sh
//! cargo run -p pgtk-sql --features spans --example migration_lint -- migrations/*.sql
//! ```
//!
//! With no file, the example lints a built-in migration:
//!
//! ```text
//! <example>:2:1: index-not-concurrent: CREATE INDEX without CONCURRENTLY locks the table against writes
//! <example>:3:1: add-column-not-null: ADD COLUMN email NOT NULL with no DEFAULT fails on a table with rows
//! ```

use std::ops::ControlFlow;
use std::process::ExitCode;

use pgtk_sql::ast::ddl::index::CreateIndexStmt;
use pgtk_sql::ast::ddl::table::{
    AlterColumnAction, AlterColumnTarget, AlterTableCmd, ColConstraintElem, ColumnDef,
};
use pgtk_sql::ast::dml::delete::DeleteStmt;
use pgtk_sql::ast::dml::update::UpdateStmt;
use pgtk_sql::document;
use recursa::{TotalVisitor, Visit, VisitBreak, Visitor};

/// The migration that the example lints when no file is given.
const EXAMPLE: &str = "\
CREATE TABLE accounts (id bigint PRIMARY KEY, name text NOT NULL);
CREATE INDEX accounts_name ON accounts (name);
ALTER TABLE accounts ADD COLUMN email text NOT NULL;
ALTER TABLE accounts ADD COLUMN plan text NOT NULL DEFAULT 'free';
ALTER TABLE accounts ALTER COLUMN name TYPE varchar(100), DROP COLUMN legacy;
UPDATE accounts SET plan = 'pro';
EXPLAIN DELETE FROM accounts;
CREATE INDEX CONCURRENTLY accounts_email ON accounts (email);
";

/// One finding of a rule.
struct Finding {
    rule: &'static str,
    message: String,
}

/// The linter. The derive gives the generated walk one hook for each node type in `dispatch`.
/// The walk calls [`Visitor::enter`] for each node of those types, at any depth.
#[derive(Default, TotalVisitor)]
#[total_visitor(
    dispatch = [
        pgtk_sql::ast::ddl::index::CreateIndexStmt<'a>,
        pgtk_sql::ast::ddl::table::AlterTableCmd<'a>,
        pgtk_sql::ast::dml::update::UpdateStmt<'a>,
        pgtk_sql::ast::dml::delete::DeleteStmt<'a>,
    ],
    error = std::convert::Infallible,
    // The generated visit events live in the grammar module, which is the crate root.
    event = pgtk_sql::__RecursaVisitEvent,
)]
struct Lint {
    findings: Vec<Finding>,
}

impl Lint {
    fn report(&mut self, rule: &'static str, message: impl Into<String>) {
        self.findings.push(Finding {
            rule,
            message: message.into(),
        });
    }
}

type Flow = ControlFlow<VisitBreak<std::convert::Infallible>>;

impl Visitor<CreateIndexStmt<'_>> for Lint {
    type Error = std::convert::Infallible;

    fn enter(&mut self, node: &CreateIndexStmt<'_>) -> Flow {
        if !node.concurrently {
            self.report(
                "index-not-concurrent",
                "CREATE INDEX without CONCURRENTLY locks the table against writes",
            );
        }
        ControlFlow::Continue(())
    }
}

impl Visitor<AlterTableCmd<'_>> for Lint {
    type Error = std::convert::Infallible;

    fn enter(&mut self, node: &AlterTableCmd<'_>) -> Flow {
        match node {
            // The four spellings of ADD COLUMN: with or without COLUMN, and IF NOT EXISTS.
            AlterTableCmd::AddColumn(cmd) => self.add_column(&cmd.column_def),
            AlterTableCmd::AddColumnBare(cmd) => self.add_column(&cmd.column_def),
            AlterTableCmd::AddColumnIfNotExists(cmd) => self.add_column(&cmd.column_def),
            AlterTableCmd::AddIfNotExists(cmd) => self.add_column(&cmd.column_def),
            AlterTableCmd::DropColumn(cmd) => self.report(
                "drop-column",
                format!(
                    "DROP COLUMN {} breaks each client that still reads it",
                    cmd.name.text()
                ),
            ),
            AlterTableCmd::DropColumnIfExists(_) => self.report(
                "drop-column",
                "DROP COLUMN breaks each client that still reads it",
            ),
            // A column number takes only SET STATISTICS, so only a column name can change type.
            AlterTableCmd::AlterColumn(cmd) => match &cmd.target {
                // `SET DATA TYPE t` and the short form `TYPE t`.
                AlterColumnTarget::Name(column) => match &column.action {
                    AlterColumnAction::SetDataType(_) | AlterColumnAction::Type(_) => self.report(
                        "alter-column-type",
                        "ALTER COLUMN ... TYPE can rewrite the table and its indexes",
                    ),
                    _ => {}
                },
                AlterColumnTarget::Number(_) => {}
            },
            _ => {}
        }
        ControlFlow::Continue(())
    }
}

impl Lint {
    fn add_column(&mut self, column: &ColumnDef<'_>) {
        let constraints = || column.quals.constraints().filter_map(|c| c.elem());
        let not_null = constraints().any(|elem| matches!(elem, ColConstraintElem::NotNull));
        let has_default = constraints().any(|elem| matches!(elem, ColConstraintElem::Default(_)));
        if not_null && !has_default {
            self.report(
                "add-column-not-null",
                format!(
                    "ADD COLUMN {} NOT NULL with no DEFAULT fails on a table with rows",
                    column.name.text()
                ),
            );
        }
    }
}

impl Visitor<UpdateStmt<'_>> for Lint {
    type Error = std::convert::Infallible;

    fn enter(&mut self, node: &UpdateStmt<'_>) -> Flow {
        if node.where_clause.is_none() {
            self.report(
                "update-without-where",
                "UPDATE with no WHERE changes each row",
            );
        }
        ControlFlow::Continue(())
    }
}

impl Visitor<DeleteStmt<'_>> for Lint {
    type Error = std::convert::Infallible;

    fn enter(&mut self, node: &DeleteStmt<'_>) -> Flow {
        if node.where_clause.is_none() {
            self.report(
                "delete-without-where",
                "DELETE with no WHERE removes each row",
            );
        }
        ControlFlow::Continue(())
    }
}

fn main() -> ExitCode {
    let paths = std::env::args().skip(1).collect::<Vec<_>>();
    let mut files = Vec::new();
    if paths.is_empty() {
        files.push(("<example>".to_owned(), EXAMPLE.to_owned()));
    }
    for path in paths {
        match std::fs::read_to_string(&path) {
            Ok(source) => files.push((path, source)),
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::from(2);
            }
        }
    }

    let mut count = 0;
    for (path, source) in &files {
        let document = match document::parse_sql(source) {
            Ok(document) => document,
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::from(2);
            }
        };
        for item in document.items() {
            let Some(statement) = &item.ast().statement else {
                continue;
            };
            let mut lint = Lint::default();
            // The walk cannot fail: the error type is `Infallible`.
            let _ = statement.visit(&mut lint);

            let start = item.source_bounds().map_or(0, |span| span.start() as usize);
            let (line, column) = line_and_column(source, start);
            for finding in lint.findings {
                count += 1;
                println!(
                    "{path}:{line}:{column}: {}: {}",
                    finding.rule, finding.message
                );
            }
        }
    }

    if count == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("{count} findings");
        ExitCode::FAILURE
    }
}

/// The 1-based line and column (in characters) of a byte offset.
fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = before[line_start..].chars().count() + 1;
    (line, column)
}
