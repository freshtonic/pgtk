//! Save the catalog of a live server to a JSON file, and load the file later with no server.
//!
//! This is how a CI job checks queries without a database. A developer saves the catalog once,
//! from a database that has the migrations of the application, and commits the file. The CI job
//! loads the file, and gives the catalog to `pgtk-analyze` (see its `check_queries` example).
//!
//! The example needs the `postgres` and `serde` features. `save` needs a server; `show` does
//! not.
//!
//! ```sh
//! # Save the catalog of a database to a file.
//! cargo run -p pgtk-catalog --features postgres,serde --example snapshot -- \
//!     save postgresql://postgres@localhost/app catalog.json
//!
//! # Later, with no server: load the file and print what it holds.
//! cargo run -p pgtk-catalog --features postgres,serde --example snapshot -- show catalog.json
//! ```
//!
//! A snapshot is a copy of the catalog at one time. Save it again after each migration.
//! The file records [`pgtk_catalog::Catalog::FORMAT_VERSION`]. A newer pgtk-catalog can need a
//! new file.

use std::process::ExitCode;

use pgtk_catalog::{Catalog, RelationKind};

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["save", url, path] => save(url, path),
        ["show", path] => show(path),
        _ => {
            eprintln!("usage: snapshot save <postgres-url> <file.json>");
            eprintln!("       snapshot show <file.json>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Loads the catalog from the server, and writes it to `path` as JSON.
fn save(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = postgres::Client::connect(url, postgres::NoTls)?;
    let loaded = pgtk_catalog::load(&mut client)?;
    for diagnostic in &loaded.diagnostics {
        eprintln!("note: {diagnostic}");
    }
    // With the `serde` feature, a Catalog is `Serialize` and `Deserialize`.
    let json = serde_json::to_string(&loaded.catalog)?;
    std::fs::write(path, json)?;
    println!("saved {} to {path}", summary(&loaded.catalog));
    Ok(())
}

/// Reads a catalog from `path`, with no server, and prints what it holds.
fn show(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read(path)?;
    let catalog: Catalog = serde_json::from_slice(&json)?;
    println!("{path}: {}", summary(&catalog));

    // The catalog answers each question with no connection.
    let tables = catalog
        .relations()
        .filter(|(_, relation)| relation.kind == RelationKind::Table)
        .map(|(handle, _)| catalog.qualified_name_of_relation(handle))
        .filter(|name| !matches!(name.schema.as_str(), "pg_catalog" | "information_schema"))
        .collect::<Vec<_>>();
    println!("{} user tables", tables.len());
    for name in tables {
        println!("  {name}");
    }
    Ok(())
}

/// One line about a catalog.
fn summary(catalog: &Catalog) -> String {
    let metadata = catalog.metadata();
    format!(
        "the catalog of database {} on PostgreSQL {}: {} schemas, {} relations, {} types, {} functions",
        metadata.database,
        metadata.server_version,
        catalog.schemas().len(),
        catalog.relations().len(),
        catalog.types().len(),
        catalog.functions().len(),
    )
}
