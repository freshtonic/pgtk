//! The Subject: the thing under test, asked the same question as the Oracle.

use pg_catalog::Catalog;

use super::replay::Shape;

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
    /// Analysis succeeded; this is its Result Descriptor, in the terms a
    /// Describe message uses: a domain is reported as its base type.
    Analysed {
        params: Vec<u32>,
        columns: Vec<SubjectColumn>,
        /// An `unknown` literal was read as a `reg*` type, whose input
        /// function looks the text up in the catalog and raises a
        /// catalog error, not a data error, when it is missing.
        reg_literals: bool,
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
///
/// It must be `Send` because each worker thread drives its own copy.
pub trait Subject: Send {
    /// Does `analyze` read the Catalog for a statement of this shape? The
    /// harness reloads the Catalog only before statements the Subject
    /// wants, since a reload costs about a hundred milliseconds; the others
    /// are analysed against whatever Catalog it has.
    fn wants(&self, shape: Shape) -> bool;

    /// Analyse `sql`, one statement without its terminator, as PostgreSQL
    /// would with `search_path` in effect.
    fn analyze(&mut self, catalog: &Catalog, search_path: &[String], sql: &str) -> SubjectOutcome;
}

/// The real Analyzer as a Subject.
#[derive(Clone, Copy)]
pub struct AnalyzerSubject;

impl Subject for AnalyzerSubject {
    fn wants(&self, shape: Shape) -> bool {
        shape.has_query
    }

    fn analyze(&mut self, catalog: &Catalog, search_path: &[String], sql: &str) -> SubjectOutcome {
        use pg_analyze::{AnalyzeError, Analyzer, Settings, TargetVersion};
        let settings = Settings::new(
            catalog,
            TargetVersion::V17,
            search_path.iter().map(String::as_str),
        );
        let analyzer = Analyzer::new(catalog, settings);
        // getBaseTypeAndTypmod, as SendRowDescriptionMessage applies it.
        let base = |mut ty: pg_catalog::Handle<pg_catalog::Type>, mut typmod: i32| {
            while let pg_catalog::TypeKind::Domain(d) = &catalog[ty].kind {
                if typmod == -1 {
                    typmod = d.typmod.0;
                }
                ty = d.base;
            }
            (catalog[ty].oid.0, typmod)
        };
        match analyzer.analyze_sql(sql, &[]) {
            Ok(analysis) => {
                let reg_literals = analysis.coercions.iter().any(|(_, steps)| {
                    steps.iter().any(|step| {
                        step.path == pg_analyze::CoercionPath::UnknownLiteral
                            && catalog[step.target].name.starts_with("reg")
                    })
                });
                SubjectOutcome::Analysed {
                    params: analysis
                        .result
                        .params
                        .iter()
                        .map(|&t| catalog[t].oid.0)
                        .collect(),
                    columns: analysis
                        .result
                        .columns
                        .iter()
                        .map(|c| {
                            let (type_oid, typmod) = base(c.ty, c.typmod);
                            SubjectColumn {
                                name: c.name.clone(),
                                type_oid,
                                typmod,
                                table_oid: c.origin.map(|(r, _)| catalog[r].oid.0),
                                column_id: c.origin.map(|(_, attnum)| attnum),
                            }
                        })
                        .collect(),
                    reg_literals,
                }
            }
            Err(AnalyzeError::Diagnostic(d)) => SubjectOutcome::Diagnostic {
                sqlstate: d.sqlstate.to_owned(),
                message: d.message,
                position: d.position,
            },
            Err(AnalyzeError::Unsupported(u)) => SubjectOutcome::Unsupported(u.what),
        }
    }
}

/// The Subject before there is an analyzer: everything is Unsupported.
#[derive(Clone, Copy)]
pub struct Stub;

impl Subject for Stub {
    fn wants(&self, _: Shape) -> bool {
        false
    }

    fn analyze(&mut self, _: &Catalog, _: &[String], _: &str) -> SubjectOutcome {
        SubjectOutcome::Unsupported("no analyzer yet".to_owned())
    }
}
