//! The Subject: the thing under test, asked the same question as the Oracle.

use pg_catalog::Catalog;

/// One result column as the Subject reports it, in the Oracle's terms so the
/// two can be compared directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectColumn {
    pub name: String,
    pub type_oid: u32,
    pub typmod: i32,
    pub table_oid: Option<u32>,
    pub column_id: Option<i16>,
}

/// What the Subject said about one statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubjectOutcome {
    /// Analysis succeeded; this is its Result Descriptor.
    Analysed {
        params: Vec<u32>,
        columns: Vec<SubjectColumn>,
    },
    /// Analysis raised the Diagnostic PostgreSQL would raise.
    Diagnostic {
        sqlstate: String,
        message: String,
        position: Option<u32>,
    },
    /// The crate cannot analyse this statement yet. The reason is free text
    /// for the report; it is never compared.
    Unsupported(String),
}

/// A thing that analyses one statement against one Catalog.
pub trait Subject {
    /// Analyse `sql`, one statement without its terminator, as PostgreSQL
    /// would with `search_path` in effect.
    fn analyze(&mut self, catalog: &Catalog, search_path: &[String], sql: &str) -> SubjectOutcome;
}

/// The Subject before there is an analyzer: everything is Unsupported.
pub struct Stub;

impl Subject for Stub {
    fn analyze(&mut self, _: &Catalog, _: &[String], _: &str) -> SubjectOutcome {
        SubjectOutcome::Unsupported("no analyzer yet".to_owned())
    }
}
