# pgtk-catalog

**Your PostgreSQL database, as a Rust value.**

[![crates.io](https://img.shields.io/crates/v/pgtk-catalog.svg)](https://crates.io/crates/pgtk-catalog)
[![docs.rs](https://img.shields.io/docsrs/pgtk-catalog)](https://docs.rs/pgtk-catalog)
[![MIT licence](https://img.shields.io/badge/licence-MIT-blue.svg)](https://github.com/freshtonic/pgtk/blob/main/LICENSE)

pgtk-catalog loads the system catalog of one PostgreSQL database into an immutable `Catalog`:
its schemas, types, tables, columns, functions, operators, casts, collations and extensions.
Load it once from a live server. Then ask it questions with no server, no connection and no SQL.

pgtk-catalog is part of [pgtk](https://github.com/freshtonic/pgtk), the PostgreSQL toolkit for
Rust. It gives [`pgtk-analyze`](https://crates.io/crates/pgtk-analyze) the knowledge that it
needs to type-check SQL.

## Why pgtk-catalog?

- **A model, not a copy.** The `pg_*` tables are a storage format. pgtk-catalog gives you types
  with meaning: a domain knows its base type, a range knows its subtype, an array knows its
  element type, and a table knows its columns, its primary key and its parents.
- **Typed handles.** Each entity has a `Handle<T>`. A handle is small, it is `Copy`, and it
  cannot point to an entity of a different kind. `catalog[handle]` gives you the entity.
- **PostgreSQL's name rules.** Look up a name in a schema, or resolve an unqualified name
  against a search path, the way the server does.
- **One build for all server versions.** One binary reads PostgreSQL 14, 15, 16 and 17. The
  loader finds the server version when it connects.
- **Bring your own driver.** Adapters for `postgres` and `tokio-postgres` are included. For any
  other driver, implement a trait with one method.
- **Works offline.** The `fixture` feature includes a stock PostgreSQL 17 catalog in the crate,
  for tests and for tools that never connect to a server.
- **Safe and small.** `unsafe` is forbidden in this crate. The only required dependency is
  `thiserror`.

## Quick start

```toml
[dependencies]
pgtk-catalog = { version = "0.1", features = ["postgres"] }
```

Load a catalog from a live server:

```rust,ignore
let mut client = postgres::Client::connect(url, postgres::NoTls)?;
let loaded = pgtk_catalog::load(&mut client)?;
let catalog = loaded.catalog;
```

`loaded.diagnostics` holds one note for each entity that the loader could not fully keep. For a
stock database, the list is empty.

Or use the built-in PostgreSQL 17 catalog (the `fixture` feature), with no server:

```rust,ignore
let catalog = pgtk_catalog::fixture::pg17();
```

Look up a type by name, then read it:

```rust,ignore
use pgtk_catalog::{Builtin, QualifiedName, SearchPath};

let name = QualifiedName::new("pg_catalog", "int4");
let int4 = catalog.type_named(&name).unwrap();
assert_eq!(catalog.as_builtin(int4), Some(Builtin::Int4));

// Or resolve an unqualified name against a search path.
let path = SearchPath::new(&catalog, ["public"]);
let same = catalog.type_visible(&path, "int4").unwrap();
assert_eq!(same, int4);
```

Try the `inspect` example. It finds each entity with a given name: a type, a relation, each
function overload and each operator overload.

```sh
cargo run -p pgtk-catalog --features fixture --example inspect -- int4
cargo run -p pgtk-catalog --features fixture --example inspect -- information_schema.tables
```

## Use a different driver

Implement `Executor` for your client, and give it to `load`. With the `async` feature,
`AsyncExecutor` and `load_async` do the same for an async client.

```rust,ignore
impl pgtk_catalog::Executor for MyClient {
    type Error = MyError;

    // Rows is Vec<Vec<Option<String>>>: each value in text format, None for NULL.
    fn query(&mut self, sql: &str) -> Result<pgtk_catalog::Rows, Self::Error> {
        todo!("run the SQL text and give back each row")
    }
}
```

The loader reads `pg_catalog` only. It never reads `information_schema`.

## Features

| Feature | What it does |
|---|---|
| `postgres` | `Executor` for `postgres::Client`. |
| `tokio-postgres` | `AsyncExecutor` for `tokio_postgres::Client`. Turns on `async`. |
| `async` | The `AsyncExecutor` trait and `load_async`. |
| `serde` | `Serialize` and `Deserialize` for a `Catalog`. Save a catalog, and load it again later. |
| `fixture` | The built-in PostgreSQL 17 catalog. Turns on `serde`. |

## Documents

- `CONTEXT.md`: the language of the crate.
- `docs/spec.md`: the specification that the implementation follows.
- `docs/design.md` and `docs/adr/`: the design decisions, and why.

## Licence

MIT.
