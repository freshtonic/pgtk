//! Smoke checks against the bundled PostgreSQL 17 Catalog: no server needed.

use pgtk_analyze::{
    AnalyzeError, Analyzer, GRAMMAR_VERSION, Session, Settings, TargetVersion, TargetVersionError,
};
use pgtk_catalog::Catalog;

fn catalog() -> Catalog {
    pgtk_catalog::fixture::pg17()
}

fn describe(catalog: &Catalog, sql: &str) -> String {
    describe_for(catalog, GRAMMAR_VERSION, sql)
}

fn describe_for(catalog: &Catalog, target: TargetVersion, sql: &str) -> String {
    let settings = Settings::new(catalog, target, ["public"]).unwrap();
    let analyzer = Analyzer::new(catalog, settings);
    match analyzer.analyze_sql(sql, &[], &Session::new()) {
        Ok(analysis) => analysis
            .result
            .columns
            .iter()
            .map(|c| {
                let ty = &catalog[c.ty];
                format!("{}:{}({})", c.name, ty.name, c.typmod)
            })
            .collect::<Vec<_>>()
            .join(", "),
        Err(AnalyzeError::Diagnostic(d)) => {
            format!("ERROR {} {} @{:?}", d.sqlstate, d.message, d.position)
        }
        Err(AnalyzeError::Unsupported(u)) => format!("UNSUPPORTED {}", u.what),
    }
}

#[test]
fn constants_and_operators() {
    let catalog = catalog();
    assert_eq!(describe(&catalog, "SELECT 1"), "?column?:int4(-1)");
    assert_eq!(describe(&catalog, "SELECT 1 + 1"), "?column?:int4(-1)");
    assert_eq!(describe(&catalog, "SELECT 1 + 1.5"), "?column?:numeric(-1)");
    assert_eq!(describe(&catalog, "SELECT 'a' || 'b'"), "?column?:text(-1)");
    assert_eq!(describe(&catalog, "SELECT 'abc'"), "?column?:text(-1)");
    assert_eq!(describe(&catalog, "SELECT NULL"), "?column?:text(-1)");
    assert_eq!(describe(&catalog, "SELECT -1"), "?column?:int4(-1)");
    assert_eq!(describe(&catalog, "SELECT 3000000000"), "?column?:int8(-1)");
    assert_eq!(
        describe(&catalog, "SELECT true AND false"),
        "?column?:bool(-1)"
    );
    assert_eq!(describe(&catalog, "SELECT 1 = '1'"), "?column?:bool(-1)");
}

#[test]
fn an_unknown_parameter_in_the_target_list_is_text() {
    // resolveTargetListUnknowns coerces the parameter to text, and
    // variable_coerce_param_hook gives $1 that type.
    let catalog = catalog();
    assert_eq!(describe(&catalog, "SELECT $1"), "?column?:text(-1)");
    assert_eq!(
        describe(&catalog, "SELECT $1 AS four, sin($2) AS five"),
        "four:text(-1), five:float8(-1)"
    );
}

#[test]
fn casts_and_functions() {
    let catalog = catalog();
    assert_eq!(describe(&catalog, "SELECT 1::int8"), "int8:int8(-1)");
    assert_eq!(
        describe(&catalog, "SELECT CAST('x' AS varchar(10))"),
        "varchar:varchar(14)"
    );
    assert_eq!(describe(&catalog, "SELECT int4 '1'"), "int4:int4(-1)");
    assert_eq!(describe(&catalog, "SELECT abs(-1)"), "abs:int4(-1)");
    assert_eq!(
        describe(&catalog, "SELECT length('abc')"),
        "length:int4(-1)"
    );
    assert_eq!(describe(&catalog, "SELECT now()"), "now:timestamptz(-1)");
    assert_eq!(
        describe(&catalog, "SELECT coalesce(1, 2.5)"),
        "coalesce:numeric(-1)"
    );
    assert_eq!(describe(&catalog, "SELECT nullif(1, 2)"), "nullif:int4(-1)");
}

#[test]
fn columns_and_errors() {
    let catalog = catalog();
    assert_eq!(
        describe(
            &catalog,
            "SELECT relname, relkind FROM pg_class WHERE relname = 'x'"
        ),
        "relname:name(-1), relkind:char(-1)"
    );
    assert_eq!(
        describe(&catalog, "SELECT c.oid FROM pg_class c"),
        "oid:oid(-1)"
    );
    assert_eq!(
        describe(&catalog, "SELECT nosuch FROM pg_class"),
        "ERROR 42703 column \"nosuch\" does not exist @Some(8)"
    );
    assert_eq!(
        describe(&catalog, "SELECT 1 FROM nosuch"),
        "ERROR 42P01 relation \"nosuch\" does not exist @Some(15)"
    );
    assert_eq!(
        describe(&catalog, "SELECT 1 + 'a'::text"),
        "ERROR 42883 operator does not exist: integer + text @Some(10)"
    );
    assert_eq!(
        describe(&catalog, "SELECT nosuchfunc(1)"),
        "ERROR 42883 function nosuchfunc(integer) does not exist @Some(8)"
    );
    assert_eq!(
        describe(&catalog, "SELECT 1 FROM pg_class WHERE 1"),
        "ERROR 42804 argument of WHERE must be type boolean, not type integer @Some(30)"
    );
    assert_eq!(
        describe(&catalog, "SELECT *"),
        "ERROR 42601 SELECT * with no tables specified is not valid @Some(8)"
    );
}

#[test]
fn target_newer_than_grammar_is_refused() {
    let catalog = catalog();
    let all = [
        TargetVersion::Pg14,
        TargetVersion::Pg15,
        TargetVersion::Pg16,
        TargetVersion::Pg17,
        TargetVersion::Pg18,
        TargetVersion::Pg19Beta,
    ];
    for target in all {
        match Settings::new(&catalog, target, ["public"]) {
            Ok(settings) => {
                assert!(target.major() <= GRAMMAR_VERSION.major());
                assert_eq!(settings.target_version(), target);
            }
            Err(e) => {
                assert!(target.major() > GRAMMAR_VERSION.major());
                assert_eq!(
                    e,
                    TargetVersionError {
                        target,
                        grammar: GRAMMAR_VERSION,
                    }
                );
            }
        }
    }
}

/// A Grammar Version from 16 parses a subquery in FROM with no alias. A
/// Target Version before 16 still rejects it, as that server does.
#[cfg(feature = "since-pg16")]
#[test]
fn target_older_than_grammar_applies_its_rules() {
    let catalog = catalog();
    let sql = "SELECT * FROM (SELECT 1 AS a)";
    assert_eq!(
        describe_for(&catalog, TargetVersion::Pg16, sql),
        "a:int4(-1)"
    );
    assert_eq!(
        describe_for(&catalog, TargetVersion::Pg15, sql),
        "ERROR 42601 subquery in FROM must have an alias @Some(15)"
    );
}

/// MERGE arrived in 15, so a Target Version of 14 gives the syntax error
/// that server gives, before any name is resolved.
#[cfg(feature = "since-pg15")]
#[test]
fn syntax_newer_than_target_is_rejected() {
    let catalog = catalog();
    let sql = "MERGE INTO missing USING other ON true WHEN MATCHED THEN DELETE";
    assert_eq!(
        describe_for(&catalog, TargetVersion::Pg14, sql),
        "ERROR 42601 syntax error at or near \"MERGE\" @Some(1)"
    );
    assert_eq!(
        describe_for(&catalog, TargetVersion::Pg15, sql),
        "ERROR 42P01 relation \"missing\" does not exist @Some(12)"
    );
}
