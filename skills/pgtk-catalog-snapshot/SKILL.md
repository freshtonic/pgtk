---
name: pgtk-catalog-snapshot
description: Load the system catalog (schemas, tables, columns, types, functions, operators, casts) of a live PostgreSQL 14 to 17 database into Rust with the pgtk-catalog crate, save it as a JSON snapshot, and load the snapshot later with no server. Use when the user wants an offline copy of a database schema for tests or CI, wants to give pgtk-analyze the tables of an application, or wants to read a PostgreSQL schema from Rust.
license: MIT
compatibility: Needs a Rust project that can depend on pgtk-catalog 0.1 and serde_json, and a PostgreSQL 14 to 17 server for the save step (Rust 1.88 or later).
---

# Save a catalog snapshot with pgtk-catalog

pgtk-catalog reads `pg_catalog` of one database into an immutable `Catalog`. With the `serde`
feature, a `Catalog` is `Serialize` and `Deserialize`. Save it once from a server that has the
schema of the application. Then CI and tests read the file and need no server.

## Dependencies

```toml
[dependencies]
# `postgres`: an Executor for postgres::Client. `serde`: save and load a Catalog.
pgtk-catalog = { version = "0.1", features = ["postgres", "serde"] }
postgres = "0.19"
serde_json = "1"
```

- For `tokio-postgres`, use the feature `tokio-postgres` and call `pgtk_catalog::load_async`.
- For a different driver, implement `pgtk_catalog::Executor` (one method, `query`, that gives
  each value as text) and give it to `pgtk_catalog::load`.
- The program that only reads the snapshot needs the `serde` feature only.

## Steps

1. Connect to a database that has the current schema, for example a database where the
   migrations ran. The loader reads `pg_catalog` only, so a role that can connect is enough.
2. Call `pgtk_catalog::load(&mut client)`. It gives a `Loaded` with `catalog` and `diagnostics`.
   The load is an error for a server older than 14 or newer than 17.
3. Look at `loaded.diagnostics`. Each note is an entity that the loader could not keep fully.
   For a stock database the list is empty. Print the notes; do not ignore them silently.
4. Write the catalog with `serde_json::to_writer(file, &catalog)`.
5. Later, read it with `serde_json::from_reader::<_, pgtk_catalog::Catalog>(file)`.

## Example: save from a server

```rust,no_run
use std::fs::File;
use std::io::{BufWriter, Write};

fn save(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = postgres::Client::connect(url, postgres::NoTls)?;
    let loaded = pgtk_catalog::load(&mut client)?;
    for note in &loaded.diagnostics {
        eprintln!("note: {note}");
    }
    let metadata = loaded.catalog.metadata();
    eprintln!("database {} on PostgreSQL {}", metadata.database, metadata.server_version);

    let mut file = BufWriter::new(File::create(path)?);
    serde_json::to_writer(&mut file, &loaded.catalog)?;
    file.flush()?;
    Ok(())
}

save("postgresql://postgres@localhost/app", "catalog.json").expect("save the snapshot");
```

## Example: load the snapshot and use it

```rust
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use pgtk_catalog::{Catalog, QualifiedName, SearchPath};

fn load(path: &std::path::Path) -> Result<Catalog, Box<dyn std::error::Error>> {
    let file = BufReader::new(File::open(path)?);
    Ok(serde_json::from_reader(file)?)
}

// The built-in catalog (feature `fixture`) stands in for a catalog from a server here.
let path = std::env::temp_dir().join("pgtk-catalog-snapshot-skill.json");
let mut file = BufWriter::new(File::create(&path).unwrap());
serde_json::to_writer(&mut file, &pgtk_catalog::fixture::pg17()).unwrap();
file.flush().unwrap();
drop(file);

let catalog = load(&path).expect("read the snapshot");
assert_eq!(catalog.metadata().server_version.major, 17);

// Find a table, then read its columns.
let path_list = SearchPath::new(&catalog, ["public"]);
let pg_class = catalog.relation_visible(&path_list, "pg_class").expect("pg_class exists");
let columns: Vec<String> =
    catalog[pg_class].columns.iter().take(2).map(|c| c.name.to_string()).collect();
assert_eq!(columns, ["oid", "relname"]);

let int4 = catalog.type_named(&QualifiedName::new("pg_catalog", "int4"));
assert!(int4.is_some());
# std::fs::remove_file(&path).ok();
```

Give the `Catalog` to pgtk-analyze to check queries against it: see the
`pgtk-analyze-describe-query` and `pgtk-analyze-check-queries` skills.

## Pitfalls

- A snapshot has a format version (`Catalog::FORMAT_VERSION`). When a new pgtk-catalog changes
  the format, reading an old file is an error (`catalog format version N does not match M`).
  Save the snapshot again after you update pgtk-catalog.
- Save the snapshot again after each migration, else CI checks against an old schema.
- The metadata has the time of the load (`loaded_at_unix_secs`), so two saves of the same
  schema are not the same bytes. Do not compare the files byte for byte.
- The snapshot holds the catalog of one database only. It holds no table data. It holds the
  names and definitions of the schema, so keep it with the source code of the application.
- Use the same server major version as production. The catalog differs between versions (for
  example, functions and types that a version added).
