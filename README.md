# pg-catalog

A semantic Rust model of one PostgreSQL database's system catalog, loaded
from a live server into an immutable `Catalog` that a consumer such as
`pg-type-check` can interrogate.

Status: design complete, implementation in progress. See `docs/spec.md`.
