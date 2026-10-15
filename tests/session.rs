//! What a Session adds to an Analysis, against the bundled PostgreSQL 17
//! Catalog: no server needed.

use pg_analyze::{AnalyzeError, Analyzer, PreparedStatement, ResultDescriptor};
use pg_analyze::{GRAMMAR_VERSION, Session, SessionEffect, Settings};
use pg_catalog::Catalog;

fn catalog() -> Catalog {
    pg_catalog::fixture::pg17()
}

/// The Result Descriptor, or the Diagnostic, as one line.
fn describe(catalog: &Catalog, session: &Session, sql: &str) -> String {
    let settings = Settings::new(catalog, GRAMMAR_VERSION, ["public"]).unwrap();
    let analyzer = Analyzer::new(catalog, settings);
    match analyzer.analyze_sql(sql, &[], session) {
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

/// The Session Effect the Statement reports.
fn effect(catalog: &Catalog, session: &Session, sql: &str) -> Option<SessionEffect> {
    let settings = Settings::new(catalog, GRAMMAR_VERSION, ["public"]).unwrap();
    let analyzer = Analyzer::new(catalog, settings);
    analyzer
        .analyze_sql(sql, &[], session)
        .expect("the statement analyses")
        .session_effect
}

/// Analyse `sql` and fold its Effect into `session`, as a caller replaying
/// statements in order does.
fn run(catalog: &Catalog, session: &mut Session, sql: &str) {
    let reported = effect(catalog, session, sql).expect("the statement has an Effect");
    session.apply(&reported);
}

#[test]
fn prepare_then_execute() {
    let catalog = catalog();
    let mut session = Session::new();
    let sql = "PREPARE p1(int) AS SELECT $1 AS one, 'x'::text AS two";

    // PREPARE is a Utility Statement: its own answer is empty.
    assert_eq!(describe(&catalog, &session, sql), "");
    // A name the Session does not hold answers with no columns, as
    // `UtilityTupleDescriptor` does. PostgreSQL raises `26000` when the
    // Statement runs, not when a Describe asks.
    assert_eq!(describe(&catalog, &session, "EXECUTE p1(1)"), "");

    run(&catalog, &mut session, sql);
    let prepared = session.prepared("p1").expect("the Effect added it");
    assert_eq!(prepared.param_types.len(), 1);
    assert_eq!(catalog[prepared.param_types[0]].name, "int4");
    assert_eq!(
        describe(&catalog, &session, "EXECUTE p1(1)"),
        "one:int4(-1), two:text(-1)"
    );
    // The arguments are not analysed and their count is not checked.
    assert_eq!(
        describe(&catalog, &session, "EXECUTE P1(1, 2, 3)"),
        "one:int4(-1), two:text(-1)"
    );
    // CREATE TABLE AS EXECUTE and EXPLAIN EXECUTE do not read the Session.
    assert_eq!(
        describe(&catalog, &session, "CREATE TABLE t AS EXECUTE nosuch"),
        ""
    );
    assert_eq!(
        describe(&catalog, &session, "EXPLAIN EXECUTE nosuch"),
        "QUERY PLAN:text(-1)"
    );

    run(&catalog, &mut session, "DEALLOCATE p1");
    assert!(session.prepared("p1").is_none());
}

#[test]
fn execute_analyses_the_prepared_query_again() {
    let catalog = catalog();
    let mut session = Session::new();
    // The query does not analyse, so the PREPARE reports no Effect at all.
    let sql = "PREPARE p2 AS SELECT nosuchcolumn FROM pg_class";
    assert_eq!(describe(&catalog, &session, sql), "");
    assert!(effect(&catalog, &session, sql).is_none());

    // A prepared statement whose result columns are no longer the ones the
    // PREPARE computed is refused, as the plan cache refuses it.
    session.add_prepared(PreparedStatement {
        name: "stale".to_owned(),
        param_types: Vec::new(),
        result: ResultDescriptor::default(),
        source: "PREPARE stale AS SELECT 1 AS a".to_owned(),
    });
    assert_eq!(
        describe(&catalog, &session, "EXECUTE stale"),
        "ERROR 0A000 cached plan must not change result type @None"
    );
    // The Diagnostic of the prepared query has the position it has in the
    // text of the PREPARE.
    session.add_prepared(PreparedStatement {
        name: "gone".to_owned(),
        param_types: Vec::new(),
        result: ResultDescriptor::default(),
        source: "PREPARE gone AS SELECT * FROM nosuchtable".to_owned(),
    });
    assert_eq!(
        describe(&catalog, &session, "EXECUTE gone"),
        "ERROR 42P01 relation \"nosuchtable\" does not exist @Some(31)"
    );
    // A prepared statement the caller built without a text answers from
    // the Result Descriptor it holds.
    session.add_prepared(PreparedStatement {
        name: "byhand".to_owned(),
        param_types: Vec::new(),
        result: ResultDescriptor::default(),
        source: String::new(),
    });
    assert_eq!(describe(&catalog, &session, "EXECUTE byhand"), "");
}

#[test]
fn declare_then_fetch() {
    let catalog = catalog();
    let mut session = Session::new();
    // A cursor the Session does not hold answers with no columns, as
    // `UtilityTupleDescriptor` does. PostgreSQL raises `34000` when the
    // Statement runs, not when a Describe asks.
    assert_eq!(describe(&catalog, &session, "FETCH ALL FROM c1"), "");

    let sql = "DECLARE c1 CURSOR FOR SELECT relname FROM pg_class";
    // The query is analysed, and the cursor is the Effect.
    assert_eq!(describe(&catalog, &session, sql), "");
    run(&catalog, &mut session, sql);
    let cursor = session.cursor("c1").expect("the Effect added it");
    assert!(!cursor.scroll);
    assert!(!cursor.hold);
    assert_eq!(
        describe(&catalog, &session, "FETCH ALL FROM c1"),
        "relname:name(-1)"
    );
    assert_eq!(
        describe(&catalog, &session, "FETCH BACKWARD 2 IN c1"),
        "relname:name(-1)"
    );
    // A MOVE reads the cursor and answers with no columns.
    assert_eq!(describe(&catalog, &session, "MOVE FORWARD 1 IN c1"), "");
    assert_eq!(describe(&catalog, &session, "MOVE ALL IN nosuch"), "");

    run(&catalog, &mut session, "CLOSE c1");
    assert!(session.cursor("c1").is_none());
}

#[test]
fn a_transaction_end_closes_the_cursors_that_are_not_held() {
    let catalog = catalog();
    let mut session = Session::new();
    run(
        &catalog,
        &mut session,
        "DECLARE plain CURSOR FOR SELECT 1 AS a",
    );
    run(
        &catalog,
        &mut session,
        "DECLARE held SCROLL CURSOR WITH HOLD FOR SELECT 2 AS b",
    );
    assert!(session.cursor("held").expect("declared").hold);
    assert!(session.cursor("held").expect("declared").scroll);

    assert!(matches!(
        effect(&catalog, &session, "COMMIT"),
        Some(SessionEffect::TransactionEnd)
    ));
    // ROLLBACK TO SAVEPOINT does not end the transaction.
    assert!(effect(&catalog, &session, "ROLLBACK TO SAVEPOINT s1").is_none());

    session.end_transaction();
    assert!(session.cursor("plain").is_none());
    assert_eq!(
        describe(&catalog, &session, "FETCH ALL FROM held"),
        "b:int4(-1)"
    );
}

#[test]
fn a_set_of_a_custom_parameter_makes_a_placeholder() {
    let catalog = catalog();
    let mut session = Session::new();
    assert_eq!(
        describe(&catalog, &session, "SHOW custom.thing"),
        "ERROR 42704 unrecognized configuration parameter \"custom.thing\" @None"
    );
    // A parameter name without a dot is no placeholder.
    assert!(effect(&catalog, &session, "SET work_mem = '1MB'").is_none());

    run(&catalog, &mut session, "SET LOCAL Custom.Thing TO 'hello'");
    let placeholder = session.placeholder("CUSTOM.THING").expect("it is set");
    assert_eq!(placeholder.name, "custom.thing");
    assert_eq!(placeholder.value, "'hello'");
    assert_eq!(
        describe(&catalog, &session, "SHOW custom.thing"),
        "custom.thing:text(-1)"
    );
    // A parameter the Catalog knows is answered from the Catalog.
    assert_eq!(
        describe(&catalog, &session, "SHOW work_mem"),
        "work_mem:text(-1)"
    );
}
