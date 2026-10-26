//! Parse a file of SQL statements, and print the kind and the location of each statement.
//!
//! This is the smallest useful pgtk-sql program. It reads one file, gives the text to
//! [`pgtk_sql::document::parse_sql`], and prints one line for each statement. When PostgreSQL
//! rejects the SQL, the example prints the statement that fails, the location of the error and
//! the message, and it exits with status 1.
//!
//! The `document` module needs the `spans` feature.
//!
//! ```sh
//! cargo run -p pgtk-sql --features spans --example parse -- migrations/0001_init.sql
//! echo 'SELECT 1; SELECT FROM WHERE;' | cargo run -p pgtk-sql --features spans --example parse -- -
//! ```
//!
//! For each statement, the output gives the line, the column, the byte range and the kind:
//!
//! ```text
//! 1:1    0..8     Query
//! 1:11   10..37   Insert
//! ```

use std::io::Read;
use std::process::ExitCode;

use pgtk_sql::document::{self, SqlParseError};

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: parse <file.sql>   (use - to read standard input)");
        return ExitCode::from(2);
    };
    let source = match read_source(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::from(2);
        }
    };

    match document::parse_sql(&source) {
        Ok(document) => {
            // `items` gives each statement with its location in the source. An empty statement
            // (`;;`) is an item with no statement in it.
            for item in document.items() {
                let Some(statement) = &item.ast().statement else {
                    continue;
                };
                let span = item
                    .source_bounds()
                    .expect("the spans feature keeps the location");
                let (line, column) = line_and_column(&source, span.start() as usize);
                println!(
                    "{:<6} {:<8} {}",
                    format!("{line}:{column}"),
                    format!("{}..{}", span.start(), span.end()),
                    kind(statement),
                );
            }
            ExitCode::SUCCESS
        }
        Err(SqlParseError::Rejected(rejection)) => {
            // The rejection names the first statement that fails. Its diagnostics give the
            // location of each error in that statement.
            let island = rejection.island();
            let (line, column) = line_and_column(&source, island.start() as usize);
            eprintln!(
                "{path}:{line}:{column}: PostgreSQL rejects this statement: {}",
                source[island.range()].trim()
            );
            for diagnostic in rejection.diagnostics() {
                let at = diagnostic.span().start() as usize;
                let (line, column) = line_and_column(&source, at);
                eprintln!("  {path}:{line}:{column}: {}", diagnostic.message());
            }
            ExitCode::FAILURE
        }
        Err(fatal @ SqlParseError::Fatal(_)) => {
            // A fatal error is a bug in pgtk-sql, not in the SQL.
            eprintln!("{path}: {fatal}");
            ExitCode::FAILURE
        }
    }
}

/// Reads the file, or standard input for `-`.
fn read_source(path: &str) -> std::io::Result<String> {
    if path == "-" {
        let mut source = String::new();
        std::io::stdin().read_to_string(&mut source)?;
        Ok(source)
    } else {
        std::fs::read_to_string(path)
    }
}

/// The name of the `Statement` variant, for example `Query`, `Insert` or `CreateTable`.
///
/// A real tool uses a `match` on the variants that it knows. This example takes the name from
/// the `Debug` text, so that it works for each of the statement kinds.
fn kind(statement: &pgtk_sql::ast::Statement<'_>) -> String {
    let debug = format!("{statement:?}");
    debug
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// The 1-based line and column (in characters) of a byte offset.
fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = before[line_start..].chars().count() + 1;
    (line, column)
}
