# pg-catalog

A semantic Rust model of one PostgreSQL database's system catalog, loaded
from a live server into an immutable `Catalog` that a consumer such as
`pg-type-check` can interrogate.

Status: design complete, implementation in progress. See `docs/spec.md`.

## Usage

Load a catalog from a live server. The `postgres` feature makes
`postgres::Client` an `Executor`.

```rust
let mut client = postgres::Client::connect(url, postgres::NoTls)?;
let loaded = pg_catalog::load(&mut client)?;
let catalog = loaded.catalog;
```

`loaded.diagnostics` holds one note per entity that the loader could not
fully keep. The list is empty for a stock database.

Load a catalog with no server. The `fixture` feature embeds a stock
PostgreSQL 17 catalog in the crate.

```rust
let catalog = pg_catalog::fixture::pg17();
```

Look up a type by name, then read it.

```rust
use pg_catalog::{Builtin, QualifiedName, SearchPath};

let name = QualifiedName::new("pg_catalog", "int4");
let int4 = catalog.type_named(&name).unwrap();
assert_eq!(catalog.as_builtin(int4), Some(Builtin::Int4));

// Or resolve an unqualified name against a search path.
let path = SearchPath::new(&catalog, ["public"]);
let same = catalog.type_visible(&path, "int4").unwrap();
```
