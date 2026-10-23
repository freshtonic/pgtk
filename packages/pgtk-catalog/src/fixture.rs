//! The fixtures: a stock catalog of each supported PostgreSQL version,
//! embedded in the crate.
//!
//! A test that needs a realistic catalog does not need a server. It calls
//! [`pg17`], for example, and gets the catalog of a stock PostgreSQL 17
//! database. A catalog holds every built-in type of its version, every system
//! relation, and the functions, operators, and casts of `pg_catalog`.
//! [`pg19_beta`] needs the `pg19-beta` feature too.
//!
//! `cargo xtask fixture <postgres-url>` writes the embedded file of the
//! version of the server, such as `fixtures/pg18.json.zst`. The file holds
//! the JSON of the catalog, compressed with zstd. The database is the
//! `postgres` database of a new cluster that `initdb --locale=C -E UTF8`
//! made, as `pgtk-test-server` makes it.

use crate::catalog::Catalog;

/// Decompress and read one embedded fixture file.
///
/// # Panics
///
/// This panics when the embedded bytes are corrupt. The bytes ship with the
/// crate, so a panic means a damaged build, not a mistake of the caller.
fn read(bytes: &[u8]) -> Catalog {
    let json = zstd::decode_all(bytes).expect("the embedded fixture decompresses");
    serde_json::from_slice(&json).expect("the embedded fixture holds a catalog")
}

/// The catalog of a stock PostgreSQL 14 database.
///
/// Each call builds a new catalog, so a caller that needs the catalog more
/// than once keeps the value.
///
/// # Panics
///
/// This panics when the embedded bytes are corrupt, as [`pg17`] does.
///
/// # Example
///
/// ```
/// let catalog = pgtk_catalog::fixture::pg14();
/// assert_eq!(catalog.metadata().server_version.major, 14);
/// ```
pub fn pg14() -> Catalog {
    read(include_bytes!("../fixtures/pg14.json.zst"))
}

/// The catalog of a stock PostgreSQL 15 database.
///
/// Each call builds a new catalog, so a caller that needs the catalog more
/// than once keeps the value.
///
/// # Panics
///
/// This panics when the embedded bytes are corrupt, as [`pg17`] does.
///
/// # Example
///
/// ```
/// let catalog = pgtk_catalog::fixture::pg15();
/// assert_eq!(catalog.metadata().server_version.major, 15);
/// ```
pub fn pg15() -> Catalog {
    read(include_bytes!("../fixtures/pg15.json.zst"))
}

/// The catalog of a stock PostgreSQL 16 database.
///
/// Each call builds a new catalog, so a caller that needs the catalog more
/// than once keeps the value.
///
/// # Panics
///
/// This panics when the embedded bytes are corrupt, as [`pg17`] does.
///
/// # Example
///
/// ```
/// let catalog = pgtk_catalog::fixture::pg16();
/// assert_eq!(catalog.metadata().server_version.major, 16);
/// ```
pub fn pg16() -> Catalog {
    read(include_bytes!("../fixtures/pg16.json.zst"))
}

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
/// let catalog = pgtk_catalog::fixture::pg17();
/// assert_eq!(catalog.metadata().server_version.major, 17);
/// ```
pub fn pg17() -> Catalog {
    read(include_bytes!("../fixtures/pg17.json.zst"))
}

/// The catalog of a stock PostgreSQL 18 database.
///
/// Each call builds a new catalog, so a caller that needs the catalog more
/// than once keeps the value.
///
/// # Panics
///
/// This panics when the embedded bytes are corrupt, as [`pg17`] does.
///
/// # Example
///
/// ```
/// let catalog = pgtk_catalog::fixture::pg18();
/// assert_eq!(catalog.metadata().server_version.major, 18);
/// ```
pub fn pg18() -> Catalog {
    read(include_bytes!("../fixtures/pg18.json.zst"))
}

/// The catalog of a stock PostgreSQL 19 beta database.
///
/// This needs the `pg19-beta` feature. The catalog of 19 can still change
/// before 19.0, so this fixture can change too. Each call builds a new
/// catalog, so a caller that needs the catalog more than once keeps the
/// value.
///
/// # Panics
///
/// This panics when the embedded bytes are corrupt, as [`pg17`] does.
///
/// # Example
///
/// ```
/// let catalog = pgtk_catalog::fixture::pg19_beta();
/// assert_eq!(catalog.metadata().server_version.major, 19);
/// ```
#[cfg(feature = "pg19-beta")]
pub fn pg19_beta() -> Catalog {
    read(include_bytes!("../fixtures/pg19.json.zst"))
}
