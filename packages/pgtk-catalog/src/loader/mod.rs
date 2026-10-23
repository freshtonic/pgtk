//! The loader: it turns query rows into a [`crate::Catalog`].
//!
//! The loader does no I/O. The caller runs the SQL and feeds the rows back.
//! [`crate::Executor`] and [`crate::load`] do that for a driver.
//!
//! The order of work is always the same:
//!
//! 1. Run [`Loader::BOOTSTRAP_QUERY`] and give the rows to [`Loader::new`].
//!    The loader reads the server version and refuses a version that this
//!    crate does not support.
//! 2. Run every query that [`Loader::queries`] gives, and feed each result
//!    to [`Loader::feed`].
//! 3. Call [`Loader::finish`].
//!
//! ```
//! use pgtk_catalog::{Loader, QueryId};
//!
//! let mut loader = Loader::new(vec![vec![Some("170004".to_owned())]]).unwrap();
//! assert_eq!(loader.version().major, 17);
//! for (id, sql) in loader.queries().collect::<Vec<_>>() {
//!     assert!(!sql.is_empty());
//!     let _ = id;
//! }
//! ```

mod build;
mod node_tree;
mod parse;
pub mod queries;

pub use queries::QueryId;

use std::collections::HashMap;

use crate::catalog::{Catalog, ServerVersion};
use crate::diagnostic::Diagnostic;
use crate::error::LoadError;
use crate::loader::parse::Cells;

/// The rows of one query.
///
/// Every cell is the text form of the value. A SQL NULL is `None`.
pub type Rows = Vec<Vec<Option<String>>>;

/// The result of a load: the catalog, and every note about it.
#[derive(Debug)]
pub struct Loaded {
    /// The catalog that the loader assembled.
    pub catalog: Catalog,
    /// Every note about an entity that the loader could not fully keep.
    ///
    /// An empty list means that the loader kept every entity.
    pub diagnostics: Vec<Diagnostic>,
}

/// The lowest major version that this crate supports.
pub(crate) const MIN_MAJOR: u16 = 14;

/// The highest major version that this crate supports.
///
/// The `pg19-beta` feature adds PostgreSQL 19, because the catalog of 19 can
/// still change before 19.0.
const MAX_MAJOR: u16 = if cfg!(feature = "pg19-beta") { 19 } else { 18 };

/// The part of the crate that turns query rows into a [`Catalog`].
///
/// A loader holds the rows that the caller feeds it, then assembles the
/// catalog in one step. It does no I/O.
#[derive(Debug, Clone)]
pub struct Loader {
    /// The version of the server, from the bootstrap row.
    version: ServerVersion,
    /// The rows of each query that the caller fed.
    rows: HashMap<QueryId, Rows>,
}

impl Loader {
    /// SQL that gives one row and one column: `server_version_num` as text.
    ///
    /// This is the same SQL as [`QueryId::Version`]. Run it first, because
    /// the loader needs the version before it accepts anything else.
    pub const BOOTSTRAP_QUERY: &'static str = "SELECT current_setting('server_version_num')::text";

    /// Start a loader from the rows of [`Loader::BOOTSTRAP_QUERY`].
    ///
    /// # Errors
    ///
    /// This gives [`LoadError::Parse`] when the rows do not hold a number,
    /// and [`LoadError::UnsupportedVersion`] when the major version is not
    /// 14 through 18. With the `pg19-beta` feature, 19 is supported too.
    pub fn new(bootstrap_rows: Rows) -> Result<Self, LoadError> {
        let empty: Vec<Option<String>> = Vec::new();
        let first = bootstrap_rows.first().unwrap_or(&empty);
        let number = Cells::new(QueryId::Version, 0, first).u32(0)?;
        let version = ServerVersion {
            major: (number / 10_000) as u16,
            minor: (number % 10_000) as u16,
        };
        if version.major < MIN_MAJOR || version.major > MAX_MAJOR {
            return Err(LoadError::UnsupportedVersion(version));
        }
        Ok(Loader {
            version,
            rows: HashMap::new(),
        })
    }

    /// The version of the server that the bootstrap row named.
    pub fn version(&self) -> ServerVersion {
        self.version
    }

    /// Every query to run, with its SQL text.
    ///
    /// The order does not matter. [`QueryId::Version`] is not here, because
    /// [`Loader::BOOTSTRAP_QUERY`] already answered it.
    pub fn queries(&self) -> impl Iterator<Item = (QueryId, &'static str)> {
        QueryId::ALL
            .iter()
            .copied()
            .filter(|id| *id != QueryId::Version)
            .map(|id| (id, queries::sql(id, self.version.major)))
    }

    /// Give the loader the rows of one query.
    ///
    /// A second feed of one query replaces the first.
    pub fn feed(&mut self, query: QueryId, rows: Rows) {
        self.rows.insert(query, rows);
    }

    /// Assemble the catalog.
    ///
    /// # Errors
    ///
    /// This gives [`LoadError::MissingRows`] when a query that
    /// [`Loader::queries`] named was never fed, [`LoadError::Parse`] when a
    /// value that the loader must read has a bad form, and
    /// [`LoadError::Build`] when the builder rejects the result.
    pub fn finish(self) -> Result<Loaded, LoadError> {
        for (id, _) in self.queries() {
            if !self.rows.contains_key(&id) {
                return Err(LoadError::MissingRows { query: id });
            }
        }
        build::assemble(self.version, &self.rows, false)
    }
}

#[cfg(test)]
mod tests {
    use super::{Loader, QueryId, queries};
    use crate::error::LoadError;

    fn bootstrap(number: &str) -> Vec<Vec<Option<String>>> {
        vec![vec![Some(number.to_owned())]]
    }

    #[test]
    fn the_bootstrap_query_is_the_version_query() {
        assert_eq!(
            Loader::BOOTSTRAP_QUERY.trim(),
            queries::sql(QueryId::Version, super::MIN_MAJOR).trim()
        );
    }

    #[test]
    fn it_reads_the_major_and_the_minor_version() {
        let loader = Loader::new(bootstrap("170004")).unwrap();
        assert_eq!(loader.version().major, 17);
        assert_eq!(loader.version().minor, 4);
        assert_eq!(loader.version().to_string(), "17.4");
    }

    #[test]
    fn it_takes_every_supported_version() {
        for major in 14..=18 {
            let loader = Loader::new(bootstrap(&format!("{major}0002"))).unwrap();
            assert_eq!(loader.version().major, major);
        }
    }

    #[test]
    fn it_refuses_a_version_outside_the_range() {
        for number in ["130010", "200000"] {
            match Loader::new(bootstrap(number)) {
                Err(LoadError::UnsupportedVersion(_)) => {}
                other => panic!("expected an unsupported version, got {other:?}"),
            }
        }
    }

    #[test]
    fn it_takes_19_only_with_the_pg19_beta_feature() {
        let result = Loader::new(bootstrap("190000"));
        if cfg!(feature = "pg19-beta") {
            assert_eq!(result.unwrap().version().major, 19);
        } else {
            match result {
                Err(LoadError::UnsupportedVersion(_)) => {}
                other => panic!("expected an unsupported version, got {other:?}"),
            }
        }
    }

    #[test]
    fn the_constraints_query_reads_the_new_columns_only_from_18() {
        for major in 14..=17 {
            let sql = queries::sql(QueryId::Constraints, major);
            assert!(!sql.contains("k.conenforced"), "version {major}");
            assert!(!sql.contains("k.conperiod"), "version {major}");
        }
        for major in 18..=19 {
            let sql = queries::sql(QueryId::Constraints, major);
            assert!(sql.contains("k.conenforced"), "version {major}");
            assert!(sql.contains("k.conperiod"), "version {major}");
        }
    }

    #[test]
    fn it_refuses_a_bootstrap_row_that_holds_no_number() {
        match Loader::new(bootstrap("seventeen")) {
            Err(LoadError::Parse { query, .. }) => assert_eq!(query, QueryId::Version),
            other => panic!("expected a parse error, got {other:?}"),
        }
        match Loader::new(Vec::new()) {
            Err(LoadError::Parse { query, .. }) => assert_eq!(query, QueryId::Version),
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn queries_names_every_query_except_the_version() {
        let loader = Loader::new(bootstrap("170004")).unwrap();
        let ids: Vec<QueryId> = loader.queries().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), QueryId::ALL.len() - 1);
        assert!(!ids.contains(&QueryId::Version));
        for id in QueryId::ALL {
            assert!(*id == QueryId::Version || ids.contains(id), "query {id:?}");
        }
    }

    #[test]
    fn finish_reports_the_first_query_that_no_one_fed() {
        let loader = Loader::new(bootstrap("170004")).unwrap();
        match loader.finish() {
            Err(LoadError::MissingRows { query }) => assert_ne!(query, QueryId::Version),
            other => panic!("expected missing rows, got {other:?}"),
        }
    }
}
