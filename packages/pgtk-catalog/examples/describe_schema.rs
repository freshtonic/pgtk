//! Load the catalog of a live server, and describe the tables of one schema, like `\d` in psql.
//!
//! For each table, view and materialized view of the schema, the example prints the columns
//! with their types, `NOT NULL`, `DEFAULT` and identity, the primary key, and the parent tables.
//! It reads the catalog once. After the load, each question is a function call, with no SQL.
//!
//! The example needs the `postgres` feature, and a server.
//!
//! ```sh
//! cargo run -p pgtk-catalog --features postgres --example describe_schema -- \
//!     postgresql://postgres@localhost/postgres public
//! ```
//!
//! The output for one table:
//!
//! ```text
//! table public.accounts
//!   id       bigint                    not null  identity always
//!   name     character varying(100)    not null
//!   plan     text                                default
//!   balance  numeric(10,2)
//!   created  timestamp with time zone
//!   primary key (id)
//!
//! table public.vip
//!   ...
//!   level    integer
//!   inherits public.accounts
//! ```

use std::process::ExitCode;

use pgtk_catalog::{
    Builtin, Catalog, Column, Generated, Handle, Identity, Relation, RelationKind, Type,
    TypmodDetail,
};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(url) = args.next() else {
        eprintln!("usage: describe_schema <postgres-url> [schema]");
        return ExitCode::from(2);
    };
    let schema_name = args.next().unwrap_or_else(|| "public".to_owned());

    let mut client = match postgres::Client::connect(&url, postgres::NoTls) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("cannot connect to {url}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let loaded = match pgtk_catalog::load(&mut client) {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("cannot load the catalog: {error}");
            return ExitCode::FAILURE;
        }
    };
    // Each diagnostic is an entity that the loader could not fully keep, for example a type
    // from an extension that it does not know. A stock database has none.
    for diagnostic in &loaded.diagnostics {
        eprintln!("note: {diagnostic}");
    }
    let catalog = loaded.catalog;

    let Some(schema) = catalog.schema_named(&schema_name) else {
        eprintln!("no schema named {schema_name}");
        return ExitCode::FAILURE;
    };
    println!(
        "database {} on PostgreSQL {}\n",
        catalog.metadata().database,
        catalog.metadata().server_version
    );

    let mut relations = catalog
        .relations()
        .filter(|(_, relation)| relation.schema == schema)
        .filter(|(_, relation)| relation.kind != RelationKind::Sequence)
        .collect::<Vec<_>>();
    relations.sort_by(|(_, a), (_, b)| a.name.as_str().cmp(b.name.as_str()));
    if relations.is_empty() {
        println!("no tables in schema {schema_name}");
    }
    for (handle, relation) in relations {
        describe(&catalog, handle, relation);
    }
    ExitCode::SUCCESS
}

/// Prints one relation.
fn describe(catalog: &Catalog, handle: Handle<Relation>, relation: &Relation) {
    let kind = match relation.kind {
        RelationKind::Table => "table",
        RelationKind::PartitionedTable => "partitioned table",
        RelationKind::View => "view",
        RelationKind::MaterializedView => "materialized view",
        RelationKind::ForeignTable => "foreign table",
        RelationKind::Sequence => "sequence",
        // A later version can add a kind of relation.
        _ => "relation",
    };
    println!("{kind} {}", catalog.qualified_name_of_relation(handle));

    let types = relation
        .columns
        .iter()
        .map(|column| column_type(catalog, column))
        .collect::<Vec<_>>();
    let name_width = relation
        .columns
        .iter()
        .map(|column| column.name.as_str().len())
        .max()
        .unwrap_or(0);
    let type_width = types.iter().map(String::len).max().unwrap_or(0);
    for (column, ty) in relation.columns.iter().zip(&types) {
        let mut line = format!(
            "  {:name_width$}  {:type_width$}  {:8}",
            column.name.as_str(),
            ty,
            if column.not_null { "not null" } else { "" },
        );
        match column.generated {
            Some(Generated::Stored) => line.push_str("  generated stored"),
            Some(Generated::Virtual) => line.push_str("  generated virtual"),
            // A later version can add a kind of generated column.
            Some(_) => line.push_str("  generated"),
            None => {}
        }
        if !column.is_generated() && column.has_default {
            // The model keeps that a default exists, not its text.
            line.push_str("  default");
        }
        match column.identity {
            Some(Identity::Always) => line.push_str("  identity always"),
            Some(Identity::ByDefault) => line.push_str("  identity by default"),
            // A later version can add a kind of identity column.
            Some(_) => line.push_str("  identity"),
            None => {}
        }
        println!("{}", line.trim_end());
    }

    if let Some(key) = &relation.primary_key {
        // The key holds attribute numbers. `column_at` gives the column of each one.
        let names = key
            .columns
            .iter()
            .filter_map(|position| relation.column_at(*position))
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>();
        println!("  primary key ({})", names.join(", "));
    }
    for parent in &relation.parents {
        println!("  inherits {}", catalog.qualified_name_of_relation(*parent));
    }
    println!();
}

/// The type of a column as psql prints it, for example `character varying(100)`.
fn column_type(catalog: &Catalog, column: &Column) -> String {
    let name = type_name(catalog, column.ty);
    let Some(builtin) = catalog.as_builtin(column.ty) else {
        return name;
    };
    match column.typmod.decode(builtin) {
        Some(TypmodDetail::CharLength(length)) | Some(TypmodDetail::BitLength(length)) => {
            format!("{name}({length})")
        }
        Some(TypmodDetail::Numeric { precision, scale }) => {
            format!("{name}({precision},{scale})")
        }
        Some(TypmodDetail::TimePrecision(precision)) => format!("{name}({precision})"),
        // A later version can add a kind of type modifier. Print the name only.
        Some(TypmodDetail::Interval { .. }) | Some(_) | None => name,
    }
}

/// The name of a type, with no `pg_catalog.` for a built-in type.
fn type_name(catalog: &Catalog, handle: Handle<Type>) -> String {
    let name = catalog.qualified_name_of_type(handle);
    if name.schema.as_str() == "pg_catalog" {
        // psql prints the SQL names of the common types.
        match catalog.as_builtin(handle) {
            Some(Builtin::Int2) => "smallint".to_owned(),
            Some(Builtin::Int4) => "integer".to_owned(),
            Some(Builtin::Int8) => "bigint".to_owned(),
            Some(Builtin::Bool) => "boolean".to_owned(),
            Some(Builtin::Varchar) => "character varying".to_owned(),
            Some(Builtin::Bpchar) => "character".to_owned(),
            Some(Builtin::Timestamptz) => "timestamp with time zone".to_owned(),
            Some(Builtin::Timestamp) => "timestamp without time zone".to_owned(),
            _ => name.name.to_string(),
        }
    } else {
        name.to_string()
    }
}
