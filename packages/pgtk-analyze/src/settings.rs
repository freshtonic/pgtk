//! The session state that changes what a Statement means.

use pgtk_catalog::{Catalog, SearchPath};
use pgtk_sql::{TARGET_VERSION, TargetVersion};

/// A Target Version newer than the Grammar Version of this build.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "Target Version {} is newer than the Grammar Version {} of this build",
    target.major(),
    grammar.major()
)]
pub struct TargetVersionError {
    /// The Target Version the caller asked for.
    pub target: TargetVersion,
    /// The Grammar Version of this build.
    pub grammar: TargetVersion,
}

/// Session state supplied by the caller for every Analysis.
#[derive(Clone, Debug)]
pub struct Settings {
    target_version: TargetVersion,
    /// The schemas an unqualified name is looked up in, already resolved:
    /// `$user` substituted and temporary schemas removed. Mandatory.
    pub search_path: SearchPath,
    /// PostgreSQL's `transform_null_equals`: rewrite `x = NULL` to
    /// `x IS NULL`.
    pub transform_null_equals: bool,
}

impl Settings {
    /// Settings for one Target Version with the given search path and every
    /// other setting at PostgreSQL's default. The Target Version must not be
    /// newer than the Grammar Version of this build.
    ///
    /// For a Target Version older than the Grammar Version,
    /// [`Analyzer::analyze_sql`](crate::Analyzer::analyze_sql) rejects each
    /// construct the Target Version cannot parse, and
    /// [`Analyzer::analyze`](crate::Analyzer::analyze) does not check yet
    /// (`NOTES.md`).
    ///
    /// ```
    /// use pgtk_analyze::{GRAMMAR_VERSION, Settings, TargetVersion};
    ///
    /// let catalog = pgtk_catalog::fixture::pg17();
    ///
    /// // Analyse for the server that runs the SQL. Here, PostgreSQL 14.
    /// let settings = Settings::new(&catalog, TargetVersion::Pg14, ["public"]).unwrap();
    /// assert_eq!(settings.target_version(), TargetVersion::Pg14);
    ///
    /// // A Target Version newer than the grammar of this build is an error.
    /// if GRAMMAR_VERSION < TargetVersion::Pg19Beta {
    ///     assert!(Settings::new(&catalog, TargetVersion::Pg19Beta, ["public"]).is_err());
    /// }
    /// ```
    pub fn new<'a>(
        catalog: &Catalog,
        target_version: TargetVersion,
        search_path: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, TargetVersionError> {
        if target_version > TARGET_VERSION {
            return Err(TargetVersionError {
                target: target_version,
                grammar: TARGET_VERSION,
            });
        }
        Ok(Self {
            target_version,
            search_path: SearchPath::new(catalog, search_path),
            transform_null_equals: false,
        })
    }

    /// The Target Version: the PostgreSQL major version whose rules and
    /// diagnostic wording an Analysis applies.
    pub fn target_version(&self) -> TargetVersion {
        self.target_version
    }
}
