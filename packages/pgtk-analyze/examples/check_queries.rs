//! A CI check: analyse each query of an application against a catalog, with no server.
//!
//! The example reads each `.sql` file, splits it into statements, and analyses each statement.
//! It prints each error as PostgreSQL prints it: the SQLSTATE, the message, the line of the
//! statement with a caret under the location, and the `DETAIL` and the `HINT`. It exits with
//! status 1 when a query fails, so a CI job fails too.
//!
//! The catalog comes from a file that the `snapshot` example of pgtk-catalog saved from a
//! database with the migrations of the application. With no `--catalog`, the example uses the
//! built-in catalog of a stock PostgreSQL 17 database.
//!
//! ```sh
//! cargo run -p pgtk-catalog --features postgres,serde --example snapshot -- \
//!     save postgresql://postgres@localhost/app catalog.json
//! cargo run -p pgtk-analyze --example check_queries -- --catalog catalog.json queries/*.sql
//! ```
//!
//! For an error, the output is:
//!
//! ```text
//! queries/accounts.sql:4:8: ERROR:  42703: column "nmae" does not exist
//! LINE 4: SELECT nmae FROM accounts WHERE id = $1;
//!                ^
//!
//! queries/accounts.sql:6:23: ERROR:  42P01: relation "nope" does not exist
//! LINE 6: SELECT plan || 1 FROM nope;
//!                               ^
//!
//! 2 of 4 queries failed
//! ```

use std::process::ExitCode;

use pgtk_analyze::{AnalyzeError, Analyzer, Diagnostic, GRAMMAR_VERSION, Session, Settings};
use pgtk_catalog::Catalog;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut catalog_path = None;
    let mut paths = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--catalog" {
            catalog_path = args.next();
        } else {
            paths.push(arg);
        }
    }
    if paths.is_empty() {
        eprintln!("usage: check_queries [--catalog <file.json>] <file.sql>...");
        return ExitCode::from(2);
    }

    let catalog = match &catalog_path {
        Some(path) => match read_catalog(path) {
            Ok(catalog) => catalog,
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::from(2);
            }
        },
        None => pgtk_catalog::fixture::pg17(),
    };
    let search_path = catalog.metadata().search_path.clone();
    let settings = Settings::new(
        &catalog,
        GRAMMAR_VERSION,
        search_path.iter().map(|name| name.as_str()),
    )
    .expect("the Grammar Version is never newer than itself");
    // Build the Analyzer once, and give it each statement.
    let analyzer = Analyzer::new(&catalog, settings);

    let mut total = 0;
    let mut failed = 0;
    for path in &paths {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::from(2);
            }
        };
        let document = match pgtk_sql::document::parse_sql(&source) {
            Ok(document) => document,
            Err(error) => {
                eprintln!("{path}: {error}");
                failed += 1;
                continue;
            }
        };
        // Each query of an application runs alone, so each one gets an empty Session.
        let session = Session::new();
        for item in document.items() {
            if item.ast().statement.is_none() {
                continue;
            }
            let bounds = item
                .source_bounds()
                .expect("the spans feature keeps the location");
            let start = bounds.start() as usize;
            let text = &source[bounds.range()];
            total += 1;
            match analyzer.analyze_sql(text, &[], &session) {
                Ok(_) => {}
                Err(AnalyzeError::Diagnostic(diagnostic)) => {
                    failed += 1;
                    report(path, &source, start, text, &diagnostic);
                }
                Err(AnalyzeError::Unsupported(unsupported)) => {
                    // Not an error in the query: pgtk-analyze cannot analyse it yet.
                    let (line, column) = line_and_column(&source, start);
                    eprintln!(
                        "{path}:{line}:{column}: skipped: pgtk-analyze cannot analyse {}\n",
                        unsupported.what
                    );
                }
            }
        }
    }

    if failed == 0 {
        println!("{total} queries are valid");
        ExitCode::SUCCESS
    } else {
        eprintln!("{failed} of {total} queries failed");
        ExitCode::FAILURE
    }
}

/// Prints one error as psql prints it, with the location in the file.
fn report(path: &str, source: &str, start: usize, text: &str, diagnostic: &Diagnostic) {
    // `span` is a byte range in the statement text. `position` is PostgreSQL's 1-based
    // character position, for a client that has only that.
    let offset = diagnostic.span.as_ref().map(|span| start + span.start);
    let (line, column) = offset.map_or_else(
        || line_and_column(source, start),
        |offset| line_and_column(source, offset),
    );
    eprintln!(
        "{path}:{line}:{column}: ERROR:  {}: {}",
        diagnostic.sqlstate, diagnostic.message
    );
    if offset.is_some() {
        let line_text = source.lines().nth(line - 1).unwrap_or_default();
        let prefix = format!("LINE {line}: ");
        eprintln!("{prefix}{line_text}");
        eprintln!("{:width$}^", "", width = prefix.len() + column - 1);
    } else {
        eprintln!("STATEMENT:  {}", text.trim());
    }
    if let Some(detail) = &diagnostic.detail {
        eprintln!("DETAIL:  {detail}");
    }
    if let Some(hint) = &diagnostic.hint {
        eprintln!("HINT:  {hint}");
    }
    eprintln!();
}

/// Reads a catalog that was saved as JSON.
fn read_catalog(path: &str) -> Result<Catalog, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

/// The 1-based line and column (in characters) of a byte offset.
fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = before[line_start..].chars().count() + 1;
    (line, column)
}
