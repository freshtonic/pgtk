# Use PostgreSQL raw_parser as the pgtk-sql oracle

Status: accepted

The ported `pgtk-sql` repository keeps a private `pgtk-oracle` workspace crate that
compares parses through PostgreSQL's FFI `raw_parser`, rather than restoring the
retired Testcontainers execution harness. The repository owns a PostgreSQL
source submodule at `vendor/postgres`, pinned to the exact PostgreSQL point
release used by the legacy implementation: PostgreSQL 17.9. Updating the
oracle to a later PostgreSQL release is separate work; moving branches and
floating versions are not acceptable oracle inputs.

Before grammar migration, the port records a named differential baseline for
the pinned PostgreSQL release. Completion may not reduce corpus coverage,
introduce unexplained losses of accepted statements, or add skipped grammar
cases. The complete current Rust test suite, formatter goldens and idempotence
checks, file-level recovery behaviour, benchmarks, and the comparison against
`sqlparser` remain port targets. Recursa capabilities needed for file recovery
or comment-preserving formatting require separate design sessions rather than
`pgtk-sql`-specific parser exceptions.

## Amendment (2026-09-22)

ADR 0009 pins each target version to its latest minor release. For
PostgreSQL 17, `vendor/postgres` now pins 17.11 (`REL_17_11`, #71). The
differential corpus stays frozen at 17.9; see `docs/differential-baseline.md`.
