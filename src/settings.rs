//! The session state that changes what a Statement means.

use pg_catalog::{Catalog, SearchPath};

/// The PostgreSQL major version whose analysis rules and diagnostic wording
/// an Analysis applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TargetVersion {
    /// PostgreSQL 14.
    V14,
    /// PostgreSQL 15.
    V15,
    /// PostgreSQL 16.
    V16,
    /// PostgreSQL 17.
    V17,
}

impl TargetVersion {
    /// The major number.
    pub fn major(self) -> u32 {
        match self {
            Self::V14 => 14,
            Self::V15 => 15,
            Self::V16 => 16,
            Self::V17 => 17,
        }
    }

    /// The version for a major number, when it is one this crate supports.
    pub fn from_major(major: u32) -> Option<Self> {
        match major {
            14 => Some(Self::V14),
            15 => Some(Self::V15),
            16 => Some(Self::V16),
            17 => Some(Self::V17),
            _ => None,
        }
    }
}

/// Session state supplied by the caller for every Analysis.
#[derive(Clone, Debug)]
pub struct Settings {
    /// The Target Version.
    pub target_version: TargetVersion,
    /// The schemas an unqualified name is looked up in, already resolved:
    /// `$user` substituted and temporary schemas removed. Mandatory.
    pub search_path: SearchPath,
    /// PostgreSQL's `transform_null_equals`: rewrite `x = NULL` to
    /// `x IS NULL`.
    pub transform_null_equals: bool,
}

impl Settings {
    /// Settings for one version with the given search path and every other
    /// setting at PostgreSQL's default.
    pub fn new<'a>(
        catalog: &Catalog,
        target_version: TargetVersion,
        search_path: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        Self {
            target_version,
            search_path: SearchPath::new(catalog, search_path),
            transform_null_equals: false,
        }
    }
}
