//! `pg-catalog` is a semantic Rust model of one PostgreSQL database's system
//! catalog: schemas, types, relations, columns, functions, operators, casts,
//! and extensions, loaded from a live server into an immutable [`Catalog`].
//!
//! The vocabulary is defined in `CONTEXT.md`; the design in `docs/design.md`
//! and `docs/adr/`; the implementation contract in `docs/spec.md`.

// Modules are added by the implementation waves described in docs/spec.md.
