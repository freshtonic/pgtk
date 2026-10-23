//! The fatal errors of a load.

use crate::builder::BuildError;
use crate::catalog::ServerVersion;
use crate::loader::QueryId;

/// An error that stops a load.
///
/// A load fails only for a bad server version, a failed query, a value that
/// makes a query wrong, or a failed build. Everything else becomes a
/// [`crate::Diagnostic`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LoadError {
    /// The server is older or newer than the range that this crate supports.
    #[cfg_attr(
        not(feature = "pg19-beta"),
        error("server version {0} is not supported; need 14 through 18")
    )]
    #[cfg_attr(
        feature = "pg19-beta",
        error("server version {0} is not supported; need 14 through 19")
    )]
    UnsupportedVersion(ServerVersion),
    /// The executor could not run a query.
    #[error("query {query:?} failed: {source}")]
    Query {
        /// The query that failed.
        query: QueryId,
        /// The error that the executor gave.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The caller never fed the rows of a query to the loader.
    #[error("query {query:?} was never fed to the loader")]
    MissingRows {
        /// The query whose rows are absent.
        query: QueryId,
    },
    /// A value that the loader must read had a form it could not read.
    #[error("query {query:?} row {row} column {column}: cannot parse {value:?}")]
    Parse {
        /// The query that produced the row.
        query: QueryId,
        /// The index of the row, which starts at 0.
        row: usize,
        /// The index of the column, which starts at 0.
        column: usize,
        /// The text of the value.
        value: String,
    },
    /// The builder rejected the entities that the loader assembled.
    #[error(transparent)]
    Build(#[from] BuildError),
}
