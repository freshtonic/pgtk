# The Loader is sans-IO and driver-agnostic

`pgtk-catalog` does not depend on any PostgreSQL client crate in its core, and
the core never performs I/O. The Loader exposes the catalog queries it needs
and accepts their rows as text; it builds a Catalog from those rows. A thin
`Executor` trait (sync, with an async twin behind a feature) and feature-gated
adapters for common drivers sit on top for convenience. Every catalog query
casts its columns to `text` so an adapter never decodes PostgreSQL binary
formats. The sibling `pgtk-proto` crate suggests consumers that speak the wire
protocol directly and have no conventional driver; a hard dependency on one
driver, or an async-only trait, would exclude them and pull a runtime into
every consumer.
