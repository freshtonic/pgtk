//! Analyse the same queries for each PostgreSQL version from 14 to the version of this build.
//!
//! The Target Version in the [`Settings`] is the server that runs the SQL. It can be older than
//! the Grammar Version, which is the version that this build of pgtk-sql parses. For an older
//! Target Version, [`Analyzer::analyze_sql`] rejects each clause that the older server cannot
//! parse, with the error that the older server gives.
//!
//! ```sh
//! cargo run -p pgtk-analyze --example target_version
//! cargo run -p pgtk-analyze --example target_version -- "SELECT 0x42F"
//! ```
//!
//! Part of the output:
//!
//! ```text
//! SELECT * FROM (SELECT 1)
//!   PostgreSQL 14: 42601: subquery in FROM must have an alias
//!   PostgreSQL 15: 42601: subquery in FROM must have an alias
//!   PostgreSQL 16: ok
//!   PostgreSQL 17: ok
//! ```

use pgtk_analyze::{AnalyzeError, Analyzer, GRAMMAR_VERSION, Session, Settings, TargetVersion};

/// Queries that only a newer server accepts.
const QUERIES: &[&str] = &[
    // PostgreSQL 16: a subquery in FROM needs no alias.
    "SELECT * FROM (SELECT 1)",
    // PostgreSQL 16: an integer literal can have `_` between the digits.
    "SELECT 1_000_000",
    // PostgreSQL 15: MERGE.
    "MERGE INTO pg_class c USING pg_namespace n ON c.relnamespace = n.oid WHEN MATCHED THEN DO NOTHING",
    // Each version accepts it.
    "SELECT relname FROM pg_class",
];

const VERSIONS: &[TargetVersion] = &[
    TargetVersion::Pg14,
    TargetVersion::Pg15,
    TargetVersion::Pg16,
    TargetVersion::Pg17,
    TargetVersion::Pg18,
    TargetVersion::Pg19Beta,
];

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let queries = if args.is_empty() {
        QUERIES.iter().map(|query| query.to_string()).collect()
    } else {
        args
    };
    let catalog = pgtk_catalog::fixture::pg17();
    let session = Session::new();

    for query in &queries {
        println!("{query}");
        // A Target Version newer than the Grammar Version is an error of `Settings::new`.
        for &version in VERSIONS
            .iter()
            .filter(|&&version| version <= GRAMMAR_VERSION)
        {
            let settings = Settings::new(&catalog, version, ["public"])
                .expect("the version is not newer than the Grammar Version");
            let analyzer = Analyzer::new(&catalog, settings);
            let answer = match analyzer.analyze_sql(query, &[], &session) {
                Ok(_) => "ok".to_owned(),
                Err(AnalyzeError::Diagnostic(error)) => {
                    format!("{}: {}", error.sqlstate, error.message)
                }
                Err(AnalyzeError::Unsupported(unsupported)) => {
                    format!("not analysed: {}", unsupported.what)
                }
            };
            println!("  PostgreSQL {}: {answer}", version.major());
        }
        println!();
    }
}
