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
- **One build for all server versions.** One binary reads PostgreSQL 14, 15, 16, 17 and 18, and
  19 beta with the `pg19-beta` feature. The loader finds the server version when it connects.
- **Bring your own driver.** Adapters for `postgres` and `tokio-postgres` are included. For any
  other driver, implement a trait with one method.
- **Works offline.** The `fixture` feature includes a stock catalog of each supported version in
  the crate, for tests and for tools that never connect to a server.
- **Safe and small.** `unsafe` is forbidden in this crate. The only required dependency is
  `thiserror`.

## Quick start

```toml
[dependencies]
pgtk-catalog = { version = "0.1", features = ["postgres"] }
```

Load a catalog from a live server:

```rust,no_run
let url = "postgresql://postgres@localhost/postgres";
let mut client = postgres::Client::connect(url, postgres::NoTls).expect("connect");
let loaded = pgtk_catalog::load(&mut client).expect("load the catalog");
let catalog = loaded.catalog;
```

`loaded.diagnostics` holds one note for each entity that the loader could not fully keep. For a
stock database, the list is empty.

Or use a built-in catalog (the `fixture` feature), with no server. There is one for each
version, from `fixture::pg14()` to `fixture::pg18()`, and `fixture::pg19_beta()` with `pg19-beta`:

```rust
let catalog = pgtk_catalog::fixture::pg17();
```

Look up a type by name, then read it:

```rust
use pgtk_catalog::{Builtin, QualifiedName, SearchPath};

let catalog = pgtk_catalog::fixture::pg17();
let name = QualifiedName::new("pg_catalog", "int4");
let int4 = catalog.type_named(&name).unwrap();
assert_eq!(catalog.as_builtin(int4), Some(Builtin::Int4));

// Or resolve an unqualified name against a search path.
let path = SearchPath::new(&catalog, ["public"]);
let same = catalog.type_visible(&path, "int4").unwrap();
assert_eq!(same, int4);
```

## Examples

The `examples/` folder has programs that you can copy and change.

| Example | Features | What it does |
|---|---|---|
| `inspect` | `fixture` | Finds each entity with a given name in the built-in catalog: a type, a relation, each function overload and each operator overload. |
| `describe_schema` | `postgres` | Loads the catalog of a live server, and prints the tables of a schema with their columns, types, primary keys and parents, like `\d` in psql. |
| `snapshot` | `postgres`, `serde` | Saves the catalog of a live server to a JSON file. Later, it loads the file with no server. A CI job can check queries with the file and no database. |

```sh
cargo run -p pgtk-catalog --features fixture --example inspect -- int4
cargo run -p pgtk-catalog --features fixture --example inspect -- information_schema.tables
cargo run -p pgtk-catalog --features postgres --example describe_schema -- postgresql://postgres@localhost/app public
cargo run -p pgtk-catalog --features postgres,serde --example snapshot -- save postgresql://postgres@localhost/app catalog.json
cargo run -p pgtk-catalog --features postgres,serde --example snapshot -- show catalog.json
```

## Use a different driver

Implement `Executor` for your client, and give it to `load`. With the `async` feature,
`AsyncExecutor` and `load_async` do the same for an async client.

```rust
struct MyClient;

impl pgtk_catalog::Executor for MyClient {
    type Error = std::io::Error;

    // Rows is Vec<Vec<Option<String>>>: each value in text format, None for NULL.
    fn query(&mut self, sql: &str) -> Result<pgtk_catalog::Rows, Self::Error> {
        todo!("run {sql} with your driver, and give back each row")
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
| `fixture` | The built-in catalogs of PostgreSQL 14 to 18, and of 19 beta with `pg19-beta`. Turns on `serde`. |
| `pg19-beta` | Loads a PostgreSQL 19 beta server. The 19 catalog can change before 19.0. |

## Documents

- `CONTEXT.md`: the language of the crate.
- `docs/spec.md`: the specification that the implementation follows.
- `docs/design.md` and `docs/adr/`: the design decisions, and why.

## Licence

MIT.
