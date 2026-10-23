#![doc = include_str!("../README.md")]

#[cfg(feature = "parser")]
mod parser;

#[cfg(feature = "parser")]
pub use parser::{node_to_string, parse_equal, parse_ok, Equal};

/// The Git ref (a release tag, or a branch for a pre-release major) of the
/// PostgreSQL release that this oracle is built from.
pub const POSTGRES_REF: &str = env!("PG_ORACLE_POSTGRES_REF");

/// The Git commit of the PostgreSQL release that this oracle is built from.
pub const POSTGRES_COMMIT: &str = env!("PG_ORACLE_POSTGRES_COMMIT");
