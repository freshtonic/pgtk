//! The executor: the adapter that runs the SQL, and the two load functions.
//!
//! The crate does no I/O of its own. An executor runs one SQL text against a
//! live server and gives back every row as text. [`load`] drives a
//! [`Loader`] through one executor.
//!
//! The crate ships an executor for the `postgres` crate, under the feature
//! of the same name, and one for `tokio-postgres`. A caller with another
//! driver writes the trait by hand. It has one method.

#[cfg(feature = "async")]
use std::future::Future;

use crate::error::LoadError;
use crate::loader::{Loaded, Loader, QueryId, Rows};

/// An adapter that runs one SQL text and gives back its rows as text.
///
/// Every column of every query is text already, so the adapter reads each
/// cell as an optional string and never looks at the type of the column.
pub trait Executor {
    /// The error type of the driver.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Run one SQL text and give back every row.
    fn query(&mut self, sql: &str) -> Result<Rows, Self::Error>;
}

/// Load a catalog through an executor.
///
/// This runs [`Loader::BOOTSTRAP_QUERY`], then every query that the loader
/// names, then assembles the catalog.
///
/// # Errors
///
/// A failed query gives [`LoadError::Query`]. An unsupported server gives
/// [`LoadError::UnsupportedVersion`]. The other variants come from the
/// loader.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "postgres")]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut client = postgres::Client::connect("postgres://postgres@localhost/postgres", postgres::NoTls)?;
/// let loaded = pgtk_catalog::load(&mut client)?;
/// assert!(loaded.diagnostics.is_empty());
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
pub fn load<E: Executor>(executor: &mut E) -> Result<Loaded, LoadError> {
    let bootstrap = executor
        .query(Loader::BOOTSTRAP_QUERY)
        .map_err(|source| failed(QueryId::Version, source))?;
    let mut loader = Loader::new(bootstrap)?;
    // The iterator borrows the loader, and `feed` needs it by mutable
    // reference, so take the list first.
    let queries: Vec<(QueryId, &'static str)> = loader.queries().collect();
    for (id, sql) in queries {
        let rows = executor.query(sql).map_err(|source| failed(id, source))?;
        loader.feed(id, rows);
    }
    loader.finish()
}

/// The error of a query that the driver could not run.
fn failed<E: std::error::Error + Send + Sync + 'static>(query: QueryId, source: E) -> LoadError {
    LoadError::Query {
        query,
        source: Box::new(source),
    }
}

/// An adapter that runs one SQL text without blocking the thread.
///
/// This is the same contract as [`Executor`], for an async driver.
#[cfg(feature = "async")]
pub trait AsyncExecutor {
    /// The error type of the driver.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Run one SQL text and give back every row.
    fn query(&mut self, sql: &str) -> impl Future<Output = Result<Rows, Self::Error>> + Send;
}

/// Load a catalog through an async executor.
///
/// This does the same work as [`load`], and waits on the driver instead of
/// blocking the thread.
///
/// # Errors
///
/// The errors are the errors of [`load`].
///
/// # Example
///
/// With the `tokio-postgres` feature, a `tokio_postgres::Client` is an
/// [`AsyncExecutor`]:
///
/// ```no_run
/// # #[cfg(feature = "tokio-postgres")]
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let url = "postgres://postgres@localhost/postgres";
/// let (mut client, connection) = tokio_postgres::connect(url, tokio_postgres::NoTls).await?;
/// // The connection does the I/O. Run it on a task of its own.
/// tokio::spawn(connection);
///
/// let loaded = pgtk_catalog::load_async(&mut client).await?;
/// println!("PostgreSQL {}", loaded.catalog.metadata().server_version);
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "async")]
pub async fn load_async<E: AsyncExecutor + Send>(executor: &mut E) -> Result<Loaded, LoadError> {
    let bootstrap = executor
        .query(Loader::BOOTSTRAP_QUERY)
        .await
        .map_err(|source| failed(QueryId::Version, source))?;
    let mut loader = Loader::new(bootstrap)?;
    let queries: Vec<(QueryId, &'static str)> = loader.queries().collect();
    for (id, sql) in queries {
        let rows = executor
            .query(sql)
            .await
            .map_err(|source| failed(id, source))?;
        loader.feed(id, rows);
    }
    loader.finish()
}

#[cfg(test)]
mod tests {
    use super::{Executor, load};
    use crate::error::LoadError;
    use crate::loader::{QueryId, Rows};
    use std::fmt;

    /// An executor that answers the version query and then fails.
    struct Failing {
        /// How many queries the executor already answered.
        answered: usize,
    }

    #[derive(Debug)]
    struct Broken;

    impl fmt::Display for Broken {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("the connection is broken")
        }
    }

    impl std::error::Error for Broken {}

    impl Executor for Failing {
        type Error = Broken;

        fn query(&mut self, _sql: &str) -> Result<Rows, Self::Error> {
            self.answered += 1;
            if self.answered == 1 {
                Ok(vec![vec![Some("170004".to_owned())]])
            } else {
                Err(Broken)
            }
        }
    }

    #[test]
    fn a_failed_query_names_the_query() {
        let mut executor = Failing { answered: 0 };
        match load(&mut executor) {
            Err(LoadError::Query { query, source }) => {
                assert_ne!(query, QueryId::Version);
                assert_eq!(source.to_string(), "the connection is broken");
            }
            other => panic!("expected a query error, got {other:?}"),
        }
    }

    /// An executor that fails at once.
    struct Dead;

    impl Executor for Dead {
        type Error = Broken;

        fn query(&mut self, _sql: &str) -> Result<Rows, Self::Error> {
            Err(Broken)
        }
    }

    #[test]
    fn a_failed_bootstrap_names_the_version_query() {
        match load(&mut Dead) {
            Err(LoadError::Query { query, .. }) => assert_eq!(query, QueryId::Version),
            other => panic!("expected a query error, got {other:?}"),
        }
    }
}
