//! Describe a query with no server: the name, type and source column of each result column, and
//! the type of each parameter. This is what `\gdesc` in psql prints, and what a driver learns
//! from a Describe message.
//!
//! The example uses the built-in PostgreSQL 17 catalog. Give `--catalog` to use a catalog that
//! the `snapshot` example of pgtk-catalog saved from your database.
//!
//! ```sh
//! cargo run -p pgtk-analyze --example describe -- \
//!     "SELECT c.oid, c.relname, length(c.relname) FROM pg_class c WHERE c.relnamespace = \$1"
//! cargo run -p pgtk-analyze --example describe -- --catalog catalog.json "SELECT * FROM accounts"
//! ```
//!
//! The output:
//!
//! ```text
//! column   type     source
//! oid      oid      pg_catalog.pg_class.oid
//! relname  name     pg_catalog.pg_class.relname
//! length   int4
//!
//! parameter  type
//! $1         oid
//! ```

use std::process::ExitCode;

use pgtk_analyze::{AnalyzeError, Analyzer, GRAMMAR_VERSION, Session, Settings};
use pgtk_catalog::Catalog;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut catalog_path = None;
    let mut sql = None;
    while let Some(arg) = args.next() {
        if arg == "--catalog" {
            catalog_path = args.next();
        } else {
            sql = Some(arg);
        }
    }
    let Some(sql) = sql else {
        eprintln!("usage: describe [--catalog <file.json>] <sql>");
        return ExitCode::from(2);
    };

    let catalog = match catalog_path {
        // A catalog that pgtk-catalog's `snapshot` example saved.
        Some(path) => match read_catalog(&path) {
            Ok(catalog) => catalog,
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::from(2);
            }
        },
        // The built-in catalog of a stock PostgreSQL 17 database.
        None => pgtk_catalog::fixture::pg17(),
    };

    // Analyse for the version that this build parses, with the search path of the catalog.
    let search_path = catalog.metadata().search_path.clone();
    let settings = Settings::new(
        &catalog,
        GRAMMAR_VERSION,
        search_path.iter().map(|name| name.as_str()),
    )
    .expect("the Grammar Version is never newer than itself");
    let analyzer = Analyzer::new(&catalog, settings);

    // No declared parameter types: the analysis infers the type of each `$n`.
    let analysis = match analyzer.analyze_sql(&sql, &[], &Session::new()) {
        Ok(analysis) => analysis,
        Err(AnalyzeError::Diagnostic(diagnostic)) => {
            eprintln!("ERROR:  {}: {}", diagnostic.sqlstate, diagnostic.message);
            return ExitCode::FAILURE;
        }
        Err(AnalyzeError::Unsupported(unsupported)) => {
            eprintln!("pgtk-analyze cannot analyse this yet: {}", unsupported.what);
            return ExitCode::FAILURE;
        }
    };

    let result = &analysis.result;
    if result.columns.is_empty() {
        println!("the statement returns no rows");
    } else {
        let rows = result
            .columns
            .iter()
            .map(|column| {
                // `origin` is the table column that the value comes from, when it does.
                let source = column
                    .origin
                    .map_or_else(String::new, |(relation, position)| {
                        let name = catalog.qualified_name_of_relation(relation);
                        let column = catalog[relation]
                            .column_at(position)
                            .map_or("?", |column| column.name.as_str());
                        format!("{name}.{column}")
                    });
                [column.name.clone(), type_name(&catalog, column.ty), source]
            })
            .collect::<Vec<_>>();
        print_table(["column", "type", "source"], &rows);
    }

    if !result.params.is_empty() {
        println!();
        let rows = result
            .params
            .iter()
            .enumerate()
            .map(|(index, ty)| [format!("${}", index + 1), type_name(&catalog, *ty)])
            .collect::<Vec<_>>();
        print_table(["parameter", "type"], &rows);
    }
    ExitCode::SUCCESS
}

/// Reads a catalog that was saved as JSON.
fn read_catalog(path: &str) -> Result<Catalog, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

/// The name of a type, with no `pg_catalog.` for a built-in type.
fn type_name(catalog: &Catalog, ty: pgtk_analyze::TypeRef) -> String {
    let name = catalog.qualified_name_of_type(ty);
    if name.schema.as_str() == "pg_catalog" {
        name.name.to_string()
    } else {
        name.to_string()
    }
}

/// Prints rows in columns of equal width.
fn print_table<const N: usize>(header: [&str; N], rows: &[[String; N]]) {
    let mut widths = header.map(str::len);
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.len());
        }
    }
    let line = |cells: [&str; N]| {
        let text = cells
            .iter()
            .zip(widths)
            .map(|(cell, width)| format!("{cell:width$}"))
            .collect::<Vec<_>>()
            .join("  ");
        println!("{}", text.trim_end());
    };
    line(header);
    for row in rows {
        line(row.each_ref().map(String::as_str));
    }
}
