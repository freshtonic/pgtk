//! The fixture: a stock PostgreSQL 17 catalog embedded in the crate.
//!
//! A test that needs a realistic catalog does not need a server. It calls
//! [`pg17`] and gets the catalog of a stock PostgreSQL 17 database. The
//! catalog holds every built-in type, every system relation, and the
//! functions, operators, and casts of `pg_catalog`.
//!
//! `cargo xtask fixture <postgres-url>` writes the embedded file. The file
//! holds the JSON of the catalog, compressed with zstd.

use crate::catalog::Catalog;

/// The embedded fixture file. `cargo xtask fixture` writes it.
const PG17_BYTES: &[u8] = include_bytes!("../fixtures/pg17.json.zst");

/// The catalog of a stock PostgreSQL 17 database.
///
/// The call decompresses the embedded bytes and reads the JSON. Each call
/// builds a new catalog, so a caller that needs the catalog more than once
/// keeps the value.
///
/// # Panics
///
/// This panics when the embedded bytes are corrupt. The bytes ship with the
/// crate, so a panic means a damaged build, not a mistake of the caller.
///
/// # Example
///
/// ```
/// let catalog = pg_catalog::fixture::pg17();
/// assert_eq!(catalog.metadata().server_version.major, 17);
/// ```
pub fn pg17() -> Catalog {
    let json = zstd::decode_all(PG17_BYTES).expect("the embedded fixture decompresses");
    serde_json::from_slice(&json).expect("the embedded fixture holds a catalog")
}
