//! Public strict PostgreSQL-statement seam.
//!
//! Issue #9 restores this one-statement entry point before document framing,
//! psql behavior, formatting provenance, or recovery are introduced.

use pgtk_sql::{
    LexErrorCode,
    ast::{Statement, dml::values::SelectClause, shared::with_clause::WithBody},
    lex,
};
use recursa::{ParseErrorKind, Span};

#[test]
fn parses_one_complete_semantically_typed_statement() {
    let lexed = lex("SELECT 1");
    assert!(lexed.errors().next().is_none());

    let mut input = lexed.input();
    let parsed = Statement::parse(&mut input).expect("strict statement");

    assert!(input.is_eof(), "strict parsing must consume the statement");
    assert!(matches!(
        parsed.ast(),
        Statement::Query(query)
            if matches!(
                &query.clause,
                SelectClause::Select(_)
            )
    ));
}

#[test]
fn spans_feature_controls_statement_provenance() {
    let lexed = lex("SELECT 1");
    let mut input = lexed.input();
    let parsed = Statement::parse(&mut input).expect("strict statement");
    assert_eq!(parsed.source().is_some(), cfg!(feature = "spans"));

    let mut input = lexed.input();
    let parsed =
        Statement::parse_without_spans(&mut input).expect("strict statement without provenance");
    assert_eq!(parsed.source(), None);

    #[cfg(feature = "spans")]
    {
        let mut input = lexed.input();
        let parsed = Statement::parse_spanned(&mut input).expect("strict spanned statement");
        assert_eq!(parsed.source(), Some("SELECT 1"));
    }

    #[cfg(not(feature = "spans"))]
    {
        let mut input = lexed.input();
        let parsed = Statement::parse_spanned(&mut input).expect("strict statement");
        assert_eq!(parsed.source(), None);
    }
}

/// PostgreSQL's `VariableSetStmt` owns the literal scope prefixes before the
/// shared `set_rest`. In particular, `SET SESSION CHARACTERISTICS` must not
/// reduce `SESSION` as a generic scope before its special rest is known.
#[test]
fn variable_set_stmt_keeps_scope_prefixes_and_special_rests() {
    for source in [
        "SET work_mem TO '64MB'",
        "SET LOCAL work_mem = '64MB'",
        "SET SESSION work_mem = '64MB'",
        "SET TRANSACTION ISOLATION LEVEL SERIALIZABLE",
        "SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY",
        "SET SESSION SESSION AUTHORIZATION DEFAULT",
        "SET LOCAL TIME ZONE 'UTC'",
        "SET XML OPTION DOCUMENT",
        "SET ROLE TO DEFAULT",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );

        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }
}

/// `UESCAPE` remains an ordinary bare label unless it introduces the Unicode
/// string escape-clause shape. The LR token feed classifies the latter
/// with `UESCAPE_LA` while preserving the source spelling in the AST.
#[test]
fn unicode_escape_clause_and_bare_uescape_label_remain_distinct() {
    for source in [
        "SELECT U&'d!0061t' UESCAPE '!'",
        "SELECT U&'d\\0061t' UESCAPE '!'",
        "SELECT U&'d\\0061t' /* intervening trivia */ UESCAPE '!'",
        "SELECT U&'d\\0061t' UESCAPE",
        "SELECT 1 UESCAPE",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );

        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }

    let source = "SELECT U&'d!0061t' UESCAPE 1";
    let lexed = lex(source);
    assert!(
        lexed.errors().next().is_none(),
        "lexical errors in {source:?}"
    );
    let mut input = lexed.input();
    assert!(
        Statement::parse(&mut input).is_err() || !input.is_eof(),
        "a UESCAPE keyword without a string literal must not consume a numeric literal"
    );
}

#[test]
fn query_statement_owns_every_postgresql_query_prefix() {
    for source in [
        "SELECT 1",
        "VALUES (1)",
        "TABLE example",
        "WITH example AS (SELECT 1) SELECT * FROM example",
        "WITH RECURSIVE recursive AS (SELECT 1) SELECT * FROM recursive",
        "WITH example AS (SELECT 1), recursive AS (SELECT 2) SELECT * FROM recursive",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );

        let mut input = lexed.input();
        let parsed = Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict query {source:?}: {error}"));

        assert!(input.is_eof(), "strict query left input for {source:?}");
        // gram.y `SelectStmt` covers both shapes; pgtk-sql factors the
        // `with_clause` prefix once for every statement that admits it.
        let is_query = match parsed.ast() {
            Statement::Query(_) => true,
            Statement::With(with) => matches!(with.body, WithBody::Query(_)),
            _ => false,
        };
        assert!(
            is_query,
            "query prefix selected a non-query statement for {source:?}"
        );
    }
}

#[test]
fn recursive_immediately_after_with_commits_to_the_clause_modifier() {
    let lexed = lex("WITH recursive AS (SELECT 1) SELECT 1");
    assert!(lexed.errors().next().is_none());

    let mut input = lexed.input();
    assert!(
        Statement::parse(&mut input).is_err(),
        "PostgreSQL treats the immediate RECURSIVE as the modifier, not as a CTE name"
    );
}

#[test]
fn parses_explain_without_optional_settings_as_a_guarded_statement() {
    let lexed = lex("EXPLAIN SELECT 1");
    assert!(lexed.errors().next().is_none());

    let mut input = lexed.input();
    let parsed = Statement::parse(&mut input).expect("strict EXPLAIN statement");

    assert!(input.is_eof(), "strict parsing must consume EXPLAIN's body");
    let Statement::Explain(explain) = parsed.ast() else {
        panic!("EXPLAIN must select the semantically typed statement variant");
    };
    assert!(explain.options().is_none());
    assert!(matches!(
        explain.statement(),
        pgtk_sql::ast::utility::explain::ExplainableStmt::Query(query)
            if matches!(
                &query.clause,
                SelectClause::Select(_)
            )
    ));
}

#[test]
fn lexical_failure_has_a_stable_code_region_and_anchor() {
    let lexed = lex("SELECT /* unterminated");
    let errors = lexed.errors().collect::<Vec<_>>();

    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].diagnostic_code(),
        LexErrorCode::UnterminatedNested
    );
    assert_eq!(
        errors[0].span(),
        Span::new(7, 22).expect("valid error span")
    );
    assert_eq!(errors[0].anchor(), Span::new(7, 9).expect("valid anchor"));
}

#[test]
fn strict_parse_failure_does_not_construct_a_statement() {
    let lexed = lex("SELECT FROM");
    assert!(lexed.errors().next().is_none());

    let mut input = lexed.input();
    let original_cursor = input.cursor();
    let original_source = input.source();
    let error = Statement::parse(&mut input).expect_err("invalid strict statement");

    // PostgreSQL permits an empty SELECT target list, so FROM is committed
    // before the missing relation is diagnosed at end of input.
    assert_eq!(error.kind(), ParseErrorKind::UnexpectedEof);
    assert_eq!(error.code(), "RCA4101");
    assert_eq!(error.span(), Span::new(11, 11).expect("valid EOF span"));
    assert_eq!(error.anchor(), Span::new(11, 11).expect("valid EOF anchor"));
    assert_eq!(error.found(), None);
    assert_eq!(
        input.cursor(),
        original_cursor,
        "strict failure must restore the exact public input cursor"
    );
    assert_eq!(
        input.source(),
        original_source,
        "strict failure must preserve the public input source"
    );
}

#[test]
fn strict_parse_rejects_a_terminated_empty_from_list() {
    let lexed = lex("SELECT FROM;");
    assert!(lexed.errors().next().is_none());

    let mut input = lexed.input();
    assert!(
        Statement::parse(&mut input).is_err(),
        "PostgreSQL requires a from-list item before the terminator"
    );
}

#[test]
fn strict_parse_rejects_a_trailing_comma_from_list() {
    let lexed = lex("SELECT FROM emp,");
    assert!(lexed.errors().next().is_none());

    let mut input = lexed.input();
    let parsed = Statement::parse(&mut input);
    assert!(
        parsed.is_err() || !input.is_eof(),
        "PostgreSQL requires a from-list item after the comma"
    );
}

/// A Pratt extender that PostgreSQL's shift preference makes unreachable as a
/// bare output alias must extend the target expression, not end it.
///
/// `target_el: a_expr BareColLabel` lists AND, OR, NOT, IN, COLLATE and the
/// rest of the extender keywords in `bare_label_keyword`, but bison always
/// shifts them into `a_expr`, so no PostgreSQL statement can reach them as a
/// bare alias. `SelectBareAliasName` therefore excludes them, which removes
/// the caller-FOLLOW overlap that made the Pratt loop report `RCA4102`.
#[test]
fn select_targets_extend_on_every_operator_keyword() {
    for source in [
        "SELECT a OR b",
        "SELECT a AND b",
        "SELECT a COLLATE \"C\"",
        "SELECT a NOT LIKE 'b'",
        "SELECT a IN (1, 2)",
        "SELECT 'abc' LIKE 'a%'",
        "SELECT 'abc' ILIKE 'A%'",
        "SELECT 'abc' SIMILAR TO 'a%'",
        "SELECT 1 BETWEEN 0 AND 2",
        "SELECT t AT TIME ZONE 'GMT'",
        "SELECT a ISNULL",
        "SELECT a NOTNULL",
        "SELECT bool 't' or bool 'f' AS true",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }
}

/// A bare output alias still accepts every keyword PostgreSQL can reach there.
#[test]
fn select_targets_keep_every_reachable_bare_alias() {
    for source in [
        "SELECT a alias",
        "SELECT a value",
        "SELECT a true",
        "SELECT a AS between",
        "SELECT a AS collate",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }
}

/// `index_elem` and `part_elem` take a column, a windowless function call, or a
/// parenthesized expression — never a bare `a_expr` — so `COLLATE` belongs to
/// the element rather than to an expression that would swallow it.
#[test]
fn index_shaped_elements_take_a_restricted_target() {
    for source in [
        "CREATE TABLE t (a text) PARTITION BY RANGE (a COLLATE \"POSIX\")",
        "INSERT INTO t VALUES (0) ON CONFLICT (fruit COLLATE \"C\" text_pattern_ops) DO NOTHING",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }
}

/// Delimiters and keyword prefixes authored around a list or an optional
/// belong to the whole clause, not to each repeated element.
///
/// A field-level `#[tok(...)]` attachment binds to the innermost repeated or
/// optional element, so `#[tok(LPAREN, this, RPAREN)]` on a separated `Vec`
/// declares `(a), (b)` rather than `(a, b)`, and the same attachment on an
/// `Option` disappears with the option instead of surrounding it. Every
/// clause below must therefore carry its fixed syntax on a node of its own.
#[test]
fn declared_lists_and_optionals_span_their_delimiters() {
    #[cfg_attr(
        not(feature = "since-pg15"),
        allow(unused_mut, reason = "the 15 forms extend the list")
    )]
    let mut sources = vec![
        // PARTITION BY key lists (#34).
        "CREATE TABLE t (a int, b int) PARTITION BY RANGE (a, b)",
        "CREATE TABLE t (a int, b int) PARTITION BY HASH (a part_test_int4_ops, b part_test_text_ops)",
        // ON CONFLICT arbiters and DO UPDATE assignment lists (#35).
        "INSERT INTO t VALUES (0) ON CONFLICT (key, fruit) DO NOTHING",
        "INSERT INTO t VALUES (0) ON CONFLICT (name) DO UPDATE SET population = 1, altitude = 2",
        // REFERENCES column lists (#38).
        "CREATE TABLE t (a int, b int, FOREIGN KEY (a, b) REFERENCES p (x, y))",
        // EXECUTE parameter lists (#43).
        "EXECUTE q3('AAAAxx', 5, 10.5)",
        // SELECT ... INTO [TABLE] target (#46).
        "SELECT * INTO TABLE sitmp1 FROM onek",
        "SELECT * INTO sitmp1 FROM onek",
        "SELECT * INTO TEMP TABLE sitmp1 FROM onek",
        // WITH CHECK OPTION without LOCAL or CASCADED (#49).
        "CREATE VIEW v AS SELECT 1 WITH CHECK OPTION",
        "CREATE VIEW v AS SELECT 1 WITH LOCAL CHECK OPTION",
        "CREATE VIEW v AS SELECT 1 WITH CASCADED CHECK OPTION",
    ];
    // MERGE and the column list of ON DELETE SET are added in 15:
    // docs/research/postgres-14-19-sql-syntax-changes.md, PostgreSQL 15,
    // "New statements" and "Changes to existing statements".
    #[cfg(feature = "since-pg15")]
    sources.extend([
        // MERGE insert column lists and UPDATE SET assignment lists (#36).
        "MERGE INTO target t USING source s ON t.tid = s.sid WHEN NOT MATCHED THEN INSERT (tid, balance) VALUES (s.sid, s.delta)",
        "MERGE INTO target t USING source s ON t.tid = s.sid WHEN MATCHED THEN UPDATE SET balance = 1, tid = 2",
        // ON DELETE SET column lists (#38).
        "CREATE TABLE t (a int REFERENCES p (x) ON DELETE SET NULL (a))",
        "CREATE TABLE t (a int REFERENCES p (x) ON DELETE SET DEFAULT (a))",
    ]);
    for source in sources {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }
}

/// `NONE` is PostgreSQL's missing-operand marker in an `oper_argtypes` pair
/// (#52).
///
/// `NONE` is a `COL_NAME` keyword, and `Typename`'s `type_function_name` path
/// admits neither `COL_NAME` nor therefore `NONE`. gram.y consequently spells
/// `'(' NONE ',' Typename ')'` as its own `oper_argtypes` alternative, and so
/// does `OperatorArgType`.
#[test]
fn operator_argtypes_accept_none_in_either_position() {
    for source in [
        "DROP OPERATOR # (NONE, int4)",
        "DROP OPERATOR # (int4, NONE)",
        "DROP OPERATOR IF EXISTS !!! (NONE, bigint)",
        "ALTER OPERATOR @#@ (NONE, bigint) OWNER TO regress_user",
        "COMMENT ON OPERATOR ! (bigint, NONE) IS 'test'",
        "COMMENT ON OPERATOR @#@ (NONE, bigint) IS NULL",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }
}

/// `NONE` stays out of the general type-name language: it is a `COL_NAME`
/// keyword, and PostgreSQL's `GenericType: type_function_name` excludes those
/// (#52).
#[test]
fn none_is_not_a_general_type_name() {
    for source in ["CREATE TABLE t (a NONE)", "CREATE DOMAIN d AS NONE"] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        let parsed = Statement::parse(&mut input);
        assert!(
            parsed.is_err() || !input.is_eof(),
            "NONE must not be a general type name, but {source:?} parsed"
        );
    }
}

/// The remaining section-A clauses of #59, each of which carried its
/// delimiter or lead-in keyword on every element rather than on the clause.
#[test]
fn repaired_clauses_accept_the_ordinary_multi_element_spelling() {
    for source in [
        // FunctionWithArgs.args in CREATE CAST.
        "CREATE CAST (int AS text) WITH FUNCTION f(int, text)",
        // CreateAggregateOrderByInner.args.
        "CREATE AGGREGATE ag (ORDER BY int, text) (SFUNC = f, STYPE = internal)",
        // CreateAggregateOrderedTail.ordered.
        "CREATE AGGREGATE ag (int ORDER BY int, text) (SFUNC = f, STYPE = internal)",
        // TextSearchTokenList.tokens.
        "ALTER TEXT SEARCH CONFIGURATION c ALTER MAPPING FOR a, b WITH d",
        "ALTER TEXT SEARCH CONFIGURATION c DROP MAPPING FOR a, b",
        // TSConfigAddMapping.tokens.
        "ALTER TEXT SEARCH CONFIGURATION c ADD MAPPING FOR a, b WITH d",
        // GeneratedIdentityConstraint.seq_options.
        "CREATE TABLE t (a int GENERATED ALWAYS AS IDENTITY (START WITH 44 INCREMENT BY 2))",
        // DeleteUsingClause.tables.
        "DELETE FROM t USING a, b",
        // AggregateArgs::Types.
        "DROP AGGREGATE ag(int, text)",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }
}

/// The per-element spellings that the same clauses used to accept are not
/// PostgreSQL and must now be rejected (#59).
#[test]
fn repaired_clauses_reject_the_per_element_spelling() {
    for source in [
        "CREATE CAST (int AS text) WITH FUNCTION f(int), (text)",
        "CREATE AGGREGATE ag (ORDER BY int, ORDER BY text) (SFUNC = f, STYPE = internal)",
        "CREATE AGGREGATE ag (int ORDER BY int, ORDER BY text) (SFUNC = f, STYPE = internal)",
        "ALTER TEXT SEARCH CONFIGURATION c ALTER MAPPING FOR a, FOR b WITH d",
        "ALTER TEXT SEARCH CONFIGURATION c DROP MAPPING FOR a, FOR b",
        "ALTER TEXT SEARCH CONFIGURATION c ADD MAPPING FOR a, FOR b WITH d",
        "CREATE TABLE t (a int GENERATED ALWAYS AS IDENTITY (START WITH 44) (INCREMENT BY 2))",
        "DELETE FROM t USING a, USING b",
        "DROP AGGREGATE ag(int), (text)",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        let parsed = Statement::parse(&mut input);
        assert!(
            parsed.is_err() || !input.is_eof(),
            "the per-element spelling {source:?} must be rejected"
        );
    }
}

/// `AexprConst: func_name '(' func_arg_list opt_sort_clause ')' Sconst`
/// includes the sort-clause production solely to make PostgreSQL's LALR
/// grammar conflict-free, then its rule action rejects it. pgtk-sql has no
/// rule-action escape hatch, so typed literals keep only the accepted
/// structural `func_arg_list` shape; named arguments remain a pre-existing
/// semantic-action gap. The surrounding function application remains free to
/// carry each legal argument form.
#[test]
fn function_calls_keep_variadic_and_typed_literal_boundaries() {
    for source in [
        "SELECT f(1, 2)",
        "SELECT f(VARIADIC values)",
        "SELECT f(1, VARIADIC values)",
        "SELECT f(DISTINCT value ORDER BY value)",
        "SELECT made_up_type(12) 'value'",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
    }

    for source in [
        "SELECT made_up_type(* ) 'value'",
        "SELECT made_up_type(ALL value) 'value'",
        "SELECT made_up_type(DISTINCT value) 'value'",
        "SELECT made_up_type(VARIADIC values) 'value'",
        "SELECT made_up_type(1, VARIADIC values) 'value'",
        "SELECT made_up_type(12 ORDER BY 1) 'value'",
    ] {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        let parsed = Statement::parse(&mut input);
        assert!(
            parsed.is_err() || !input.is_eof(),
            "typed literal form {source:?} must be rejected"
        );
    }
}

/// Asserts that each of `sources` lexes and that the strict parse rejects it.
fn assert_rejects(sources: &[&str], why: &str) {
    for source in sources {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        let parsed = Statement::parse(&mut input);
        assert!(
            parsed.is_err() || !input.is_eof(),
            "{why}, but {source:?} parsed"
        );
        // The raw parser of the target version must agree.
        #[cfg(feature = "postgres-oracle")]
        assert!(
            pgtk_oracle::parse_error(source).is_some(),
            "PostgreSQL accepts {source:?}"
        );
    }
}

/// Asserts that each of `sources` parses as one complete statement.
fn assert_accepts(sources: &[&str]) {
    for source in sources {
        let lexed = lex(source);
        assert!(
            lexed.errors().next().is_none(),
            "lexical errors in {source:?}"
        );
        let mut input = lexed.input();
        Statement::parse(&mut input)
            .unwrap_or_else(|error| panic!("strict statement {source:?}: {error}"));
        assert!(input.is_eof(), "strict parse left input for {source:?}");
        // The raw parser of the target version must agree.
        #[cfg(feature = "postgres-oracle")]
        if let Some(error) = pgtk_oracle::parse_error(source) {
            panic!("PostgreSQL rejects {source:?}: {error}");
        }
    }
}

/// gram.y `Sconst` is `SCONST` only: a bit string or a hex string is no
/// `Sconst` (issue #52).
#[test]
fn a_bit_or_hex_string_is_no_sconst() {
    assert_rejects(
        &[
            "CREATE DATABASE id A X''",
            "ALTER USER A PASSWORD B''",
            "CREATE ROLE A PASSWORD B''",
            "CREATE ROLE A VALID UNTIL X''",
            "CREATE SERVER s FOREIGN DATA WRAPPER w OPTIONS (host B'')",
            "CREATE SUBSCRIPTION s CONNECTION X'' PUBLICATION p",
            "COPY t FROM STDIN WITH DELIMITER B'1'",
        ],
        "a bit or hex string is no Sconst",
    );
    assert_accepts(&[
        "CREATE ROLE A PASSWORD 'secret'",
        "CREATE ROLE A PASSWORD E'secret'",
        "CREATE SERVER s FOREIGN DATA WRAPPER w OPTIONS (host U&'h')",
    ]);
}

/// The language of a transform is gram.y `name`, never a string (issue #52).
#[test]
fn the_language_of_a_transform_is_a_name() {
    assert_rejects(
        &[
            "DROP TRANSFORM FOR A LANGUAGE ''",
            "CREATE TRANSFORM FOR A LANGUAGE ''(TO SQL WITH FUNCTION A)",
            "CREATE OR REPLACE TRANSFORM FOR A LANGUAGE ''(TO SQL WITH FUNCTION A)",
        ],
        "the language of a transform is a name",
    );
    assert_accepts(&[
        "DROP TRANSFORM FOR A LANGUAGE plpgsql",
        "CREATE TRANSFORM FOR A LANGUAGE plpgsql (TO SQL WITH FUNCTION A)",
    ]);
}

/// The role statements that gram.y rejects as a syntax error (issue #53).
#[test]
fn role_statements_keep_the_forms_of_gram_y() {
    assert_rejects(
        &[
            // `ALTER ROLE ALL` takes only SET or RESET.
            "ALTER ROLE ALL WITH",
            "ALTER USER ALL WITH",
            "ALTER ROLE ALL RENAME TO r",
            "ALTER GROUP ALL ADD USER r",
            // A role grant names a `privilege_list`, never `ALL`.
            "GRANT ALL TO A",
            "REVOKE ALL FROM A",
            "GRANT ALL(users) TO A",
            "GRANT ALL PRIVILEGES TO A",
            "REVOKE ADMIN OPTION FOR ALL FROM A",
            // `SET ROLE` takes no `DEFAULT` without `TO` or `=`.
            "SET ROLE DEFAULT",
            "SET LOCAL ROLE DEFAULT",
            "SET SESSION ROLE DEFAULT",
            // Only a plain `PASSWORD` takes `NULL`.
            "ALTER ROLE A ENCRYPTED PASSWORD NULL",
            "CREATE ROLE A ENCRYPTED PASSWORD NULL",
            "CREATE USER A ENCRYPTED PASSWORD NULL",
            // A role grant is no CREATE SCHEMA element.
            "CREATE SCHEMA AUTHORIZATION A GRANT r TO A",
        ],
        "gram.y has no such role statement",
    );
    assert_accepts(&[
        "ALTER ROLE ALL SET work_mem = '1MB'",
        "ALTER USER ALL IN DATABASE d RESET ALL",
        "ALTER ROLE r WITH LOGIN",
        "ALTER ROLE CURRENT_USER SET work_mem = '1MB'",
        "ALTER GROUP CURRENT_ROLE ADD USER r",
        "ALTER ROLE r RENAME TO s",
        "ALTER GROUP g RENAME TO h",
        "GRANT r TO A",
        "GRANT ALL ON t TO A",
        "REVOKE r FROM A",
        "REVOKE ALL ON t FROM A",
        "SET ROLE TO DEFAULT",
        "SET ROLE = DEFAULT",
        "SET ROLE NONE",
        "ALTER ROLE A PASSWORD NULL",
        "ALTER ROLE A ENCRYPTED PASSWORD 'x'",
        "CREATE SCHEMA AUTHORIZATION A GRANT SELECT ON t TO A",
    ]);
}

/// The function and operator signatures that gram.y rejects (issue #54).
#[test]
fn signatures_keep_the_forms_of_gram_y() {
    assert_rejects(
        &[
            // An event trigger function takes no arguments.
            "CREATE EVENT TRIGGER id ON title EXECUTE FUNCTION A(0)",
            "CREATE EVENT TRIGGER id ON title EXECUTE PROCEDURE A('x')",
            // `oper_argtypes` has no `(NONE, NONE)`.
            "DROP OPERATOR !(NONE, NONE)",
            "DROP OPERATOR % (NONE, NONE)",
            "ALTER OPERATOR & (NONE, NONE) OWNER TO r",
            "COMMENT ON OPERATOR # (NONE, NONE) IS ''",
            // `aggr_args` has no default, and the old form takes no
            // definition list after it.
            "CREATE AGGREGATE A(A = A)(id)",
            "ALTER AGGREGATE A(A = .0) OWNER TO A",
            "CREATE OR REPLACE AGGREGATE A(A = <= )(city)",
            // `func_args` has no default.
            "DROP ROUTINE A(JSON = NOT FALSE)",
            "DROP FUNCTION A(IN A DECIMAL = 1)",
            "ALTER ROUTINE A(A JSON[] = NULL) OWNER TO A",
            "DROP PROCEDURE p(int DEFAULT 1)",
            "COMMENT ON FUNCTION f(int = 1) IS ''",
            "GRANT EXECUTE ON FUNCTION f(int = 1) TO r",
        ],
        "gram.y has no such signature",
    );
    assert_accepts(&[
        "CREATE EVENT TRIGGER id ON ddl_command_start EXECUTE FUNCTION A()",
        "DROP OPERATOR !(NONE, int)",
        "DROP OPERATOR !(int, NONE)",
        "CREATE AGGREGATE A(BASETYPE = int, SFUNC = f, STYPE = int)",
        "CREATE AGGREGATE A(int)(SFUNC = f, STYPE = int)",
        "ALTER AGGREGATE A(int) OWNER TO A",
        "DROP FUNCTION A(IN A DECIMAL)",
        "CREATE FUNCTION f(a int = 1) RETURNS int LANGUAGE sql AS 'SELECT a'",
        "CREATE PROCEDURE p(a int DEFAULT 1) LANGUAGE sql AS ''",
    ]);
}

/// gram.y `qualified_name` has at most three parts: its action rejects a
/// longer name (issue #50). `any_name` takes every length.
#[test]
fn a_qualified_name_has_at_most_three_parts() {
    assert_rejects(
        &[
            "LOCK tags.uuid .rating .priority",
            "REINDEX INDEX tags.id .id .email",
            "ANALYZE count.tags .priority .uuid",
            "SELECT * FROM a.b.c.d",
            "DELETE FROM t USING a.b.c.d",
            "CREATE STATISTICS s ON a, b FROM a.b.c.d",
            "INSERT INTO a.b.c.d VALUES (1)",
            "CREATE TABLE a.b.c.d (x int)",
            "ALTER TABLE a.b.c.d RENAME TO e",
            "CREATE TRIGGER t AFTER INSERT ON a.b.c.d EXECUTE FUNCTION f()",
            "SET CONSTRAINTS a.b.c.d IMMEDIATE",
            "GRANT SELECT ON TABLE a.b.c.d TO r",
            // The actions that read an `any_name` with
            // `makeRangeVarFromAnyName`.
            "CREATE TYPE a.b.c.d AS (x int)",
            "ALTER TYPE a.b.c.d ADD ATTRIBUTE x int",
            "ALTER TYPE a.b.c.d RENAME ATTRIBUTE x TO y",
        ],
        "a qualified_name has at most three parts",
    );
    assert_accepts(&[
        "LOCK a.b.c",
        "SELECT * FROM a.b.c",
        "SELECT * FROM a.b.c.d()",
        "DROP TABLE a.b.c.d",
        "DROP RULE r ON a.b.c.d",
        "COMMENT ON TABLE a.b.c.d IS ''",
        "SELECT x COLLATE a.b.c.d",
        "CREATE TYPE a.b.c AS (x int)",
        "CREATE TYPE a.b.c.d AS ENUM ('x')",
        "CREATE TYPE a.b.c.d",
        "ALTER TYPE a.b.c ADD ATTRIBUTE x int",
        "ALTER TYPE a.b.c.d RENAME TO e",
        "ALTER TYPE a.b.c.d ADD VALUE 'x'",
        "CREATE DOMAIN a.b.c.d AS int",
        "SET a.b.c.d = 1",
    ]);
}

/// gram.y `name_list` holds plain names (`ColId`), which take no dot (issue
/// #50).
#[test]
fn a_name_list_holds_plain_names() {
    assert_rejects(
        &[
            "DROP SERVER code.id",
            "DROP SCHEMA city.url",
            "DROP LANGUAGE url.tag",
            "DROP PROCEDURAL LANGUAGE url.tag",
            "DROP ACCESS METHOD a.b",
            "DROP EVENT TRIGGER IF EXISTS a.b",
            "DROP EXTENSION a.b CASCADE",
            "DROP FOREIGN DATA WRAPPER a.b",
            "DROP PUBLICATION a.b",
            "GRANT USAGE ON SCHEMA a.b TO r",
            "GRANT ALL ON DATABASE a.b TO r",
            "GRANT ALL ON FOREIGN DATA WRAPPER a.b TO r",
            "GRANT ALL ON FOREIGN SERVER a.b TO r",
            "GRANT ALL ON LANGUAGE a.b TO r",
            "GRANT ALL ON TABLESPACE a.b TO r",
            "GRANT ALL ON ALL TABLES IN SCHEMA a.b TO r",
            "ALTER DEFAULT PRIVILEGES IN SCHEMA a.b GRANT SELECT ON TABLES TO r",
            "CREATE SCHEMA AUTHORIZATION r GRANT ALL ON TABLESPACE a.b TO r",
            // `opt_definition` takes `definition`, a list of plain names.
            "CREATE TABLE t (a int UNIQUE WITH (a.b))",
            "CREATE TABLE t (a int, PRIMARY KEY (a) WITH (a.b = 1))",
            "CREATE DOMAIN d AS int PRIMARY KEY WITH (a.b)",
        ],
        "a name_list holds plain names",
    );
    assert_accepts(&[
        "DROP SCHEMA a, b CASCADE",
        "DROP FOREIGN DATA WRAPPER a",
        "DROP FOREIGN TABLE a.b.c.d",
        "GRANT USAGE ON SCHEMA a, b TO r",
        "GRANT USAGE ON TYPE a.b.c.d TO r",
        "ALTER DEFAULT PRIVILEGES IN SCHEMA a GRANT SELECT ON TABLES TO r",
        "CREATE TABLE t (a int UNIQUE WITH (fillfactor = 70))",
        "CREATE INDEX i ON t (a) WITH (a.b = 1)",
    ]);
}

/// gram.y `Typename` and `SimpleTypename` (issue #51).
#[test]
fn type_names_keep_the_forms_of_gram_y() {
    assert_rejects(
        &[
            // `opt_array_bounds` or `ARRAY`, never both.
            "DROP TYPE A[0] ARRAY",
            "DROP DOMAIN INT[] ARRAY",
            "DROP FUNCTION A(A A[0] ARRAY)",
            "SELECT 1::int[] ARRAY[2]",
            // `opt_type_modifiers` is `'(' expr_list ')'`, never empty.
            "DROP TYPE A()[0]",
            "DROP DOMAIN BIT()[]",
            "CREATE DOMAIN city BIT()[]",
            "SELECT 1::numeric()",
            // `%TYPE` follows `type_function_name attrs`, a dotted name.
            "DROP FUNCTION A(A A% TYPE)",
            "DROP PROCEDURE A(A A% TYPE)",
            "CREATE FUNCTION A(A A% TYPE) RETURNS int AS ''",
            // `SeqOptElem: AS SimpleTypename` and `XMLSERIALIZE(... AS
            // SimpleTypename)` take no array decoration.
            "CREATE SEQUENCE score AS INTEGER[]",
            "ALTER SEQUENCE amount AS UNKNOWN ARRAY",
            "CREATE UNLOGGED SEQUENCE country AS JSON[]",
            "CREATE TABLE t (a int GENERATED ALWAYS AS IDENTITY (AS int[]))",
            "SELECT XMLSERIALIZE(CONTENT x AS text[])",
            // `ConstInterval '(' Iconst ')' Sconst` takes no qualifier.
            "SELECT INTERVAL(0) '' YEAR",
            "CREATE POLICY currency ON item.uuid FOR ALL USING(INTERVAL(0) '' YEAR TO MONTH)",
            "ALTER POLICY d ON end_date TO CURRENT_ROLE WITH CHECK(INTERVAL(0) '' DAY)",
        ],
        "the type name is not a gram.y Typename",
    );
    assert_accepts(&[
        "DROP TYPE A[0][]",
        "DROP TYPE A ARRAY[0]",
        "DROP TYPE A(1)",
        "CREATE FUNCTION A(A a.b% TYPE) RETURNS int AS ''",
        "CREATE FUNCTION A(A a.b[]) RETURNS int AS ''",
        "CREATE SEQUENCE score AS INTEGER",
        "SELECT XMLSERIALIZE(CONTENT x AS varchar(3))",
        "SELECT INTERVAL(0) ''",
        "SELECT INTERVAL '' YEAR TO MONTH",
    ]);
}

/// The non-empty lists and the one-time clauses of gram.y (issue #56).
#[test]
fn lists_and_clauses_keep_the_cardinality_of_gram_y() {
    assert_rejects(
        &[
            // A list that gram.y has as non-empty.
            "DROP AGGREGATE",
            "DROP AGGREGATE IF EXISTS CASCADE",
            "UPDATE t SET",
            "UPDATE t SET () = (1)",
            "CREATE TRIGGER t AFTER INSERT ON x REFERENCING FOR ROW EXECUTE FUNCTION f()",
            "WITH RECURSIVE a AS (SELECT 1) CYCLE SET c USING p SELECT 1",
            "VACUUM ()",
            "CLUSTER ()",
            "REINDEX () TABLE t",
            "EXECUTE p()",
            "CREATE INDEX ON t ()",
            "CREATE INDEX ON t USING m ()",
            "CREATE INDEX ON t (a) INCLUDE ()",
            "CREATE INDEX ON t (a) WITH ()",
            "CREATE TABLESPACE s LOCATION '' WITH ()",
            "ALTER TABLESPACE s SET ()",
            "ALTER TABLESPACE s RESET ()",
            "INSERT INTO t VALUES ()",
            "VALUES ()",
            "SELECT DISTINCT ON () 1",
            "SELECT 1 GROUP BY GROUPING SETS ()",
            "SELECT * FROM ROWS FROM (f() AS ())",
            "CREATE TABLE t (a int, PRIMARY KEY ())",
            "CREATE TABLE t (a int, UNIQUE ())",
            "CREATE TABLE t (a int, FOREIGN KEY () REFERENCES u)",
            "CREATE TABLE t (a int, PRIMARY KEY (a) INCLUDE ())",
            "CREATE TABLE t (a int) WITH ()",
            "CREATE TABLE t PARTITION OF u () DEFAULT",
            "CREATE TABLE t OF u ()",
            "CREATE TABLE t (a int) PARTITION BY RANGE ()",
            "CREATE TABLE t PARTITION OF u FOR VALUES IN ()",
            "CREATE TABLE t PARTITION OF u FOR VALUES FROM () TO (1)",
            "CREATE TABLE t PARTITION OF u FOR VALUES FROM (1) TO ()",
            "CREATE TABLE t PARTITION OF u FOR VALUES WITH ()",
            "CREATE TABLE t () INHERITS ()",
            "CREATE FOREIGN TABLE t PARTITION OF u () DEFAULT SERVER s",
            "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS ()) AS t",
            "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS (NESTED '$' COLUMNS ())) AS t",
            // A statement that needs an action.
            "ALTER SERVER s",
            "ALTER FOREIGN DATA WRAPPER w",
            // One option list, one FROM SQL and one TO SQL.
            "DROP DATABASE d (FORCE), (FORCE)",
            "CREATE TRANSFORM FOR int LANGUAGE l (TO SQL WITH FUNCTION f, TO SQL WITH FUNCTION f)",
            "CREATE TRANSFORM FOR int LANGUAGE l (FROM SQL WITH FUNCTION f, FROM SQL WITH FUNCTION f)",
            "CREATE TRANSFORM FOR int LANGUAGE l (FROM SQL WITH FUNCTION f, TO SQL WITH FUNCTION f, \
             TO SQL WITH FUNCTION f)",
            // `CREATE AGGREGATE` needs its definition list.
            "CREATE AGGREGATE a(*)",
        ],
        "gram.y wants another number of items",
    );
    assert_accepts(&[
        "CREATE TABLE t ()",
        "CREATE FOREIGN TABLE t () SERVER s",
        "CREATE TYPE t AS ()",
        "CREATE TYPE t AS ENUM ()",
        "CREATE RULE r AS ON INSERT TO t DO ()",
        "SELECT 1 GROUP BY ()",
        "SELECT 1 GROUP BY GROUPING SETS (())",
        "CREATE TRIGGER t AFTER INSERT ON x EXECUTE FUNCTION f()",
        "DROP DATABASE d (FORCE)",
        "DROP DATABASE d WITH (FORCE)",
        "CREATE TRANSFORM FOR int LANGUAGE l (TO SQL WITH FUNCTION f, FROM SQL WITH FUNCTION g)",
        "CREATE TRANSFORM FOR int LANGUAGE l (FROM SQL WITH FUNCTION f, TO SQL WITH FUNCTION g)",
        "CREATE TRANSFORM FOR int LANGUAGE l (TO SQL WITH FUNCTION f)",
        "ALTER SERVER s VERSION '2'",
        "ALTER FOREIGN DATA WRAPPER w NO VALIDATOR",
    ]);
}

/// The places where gram.y wants a narrower form than an expression (issue
/// #59).
#[test]
fn narrow_places_keep_the_forms_of_gram_y() {
    assert_rejects(
        &[
            // `CURRENT OF` is only in `where_or_current_clause`.
            "SELECT WHERE CURRENT OF rating",
            "SELECT ALL WHERE CURRENT OF product",
            "CREATE INDEX ON t (a) WHERE CURRENT OF c",
            "SELECT count(*) FILTER (WHERE CURRENT OF c) FROM t",
            // `UESCAPE` follows only a Unicode identifier.
            "SELECT 1 AS product UESCAPE ''",
            "DELETE FROM t RETURNING X'' AS description UESCAPE ''",
            // A `def_arg` type takes `opt_type_modifiers`, an `expr_list`.
            "CREATE OPERATOR ~<=~ (country = A(BIT(.0)))",
            "CREATE OPERATOR <@ (tag = A(BIT ARRAY) ARRAY)",
            "ALTER TYPE email SET (order_date = A(JSON ARRAY[0]))",
            "CREATE AGGREGATE a(int) (SFUNC = f(bit(1)), STYPE = int)",
            // An operator class parameter is a `def_arg`.
            "CREATE INDEX ON t (a ops (x = ARRAY[]))",
            "CREATE INDEX ON t (a ops (x = U&'' | 1))",
            // `XMLATTRIBUTES` is only the first argument of `XMLELEMENT`.
            "SELECT XMLATTRIBUTES(1 AS a)",
            "CALL p(XMLATTRIBUTES(X'' AS enabled))",
            "SELECT XMLELEMENT(NAME a, 'c', XMLATTRIBUTES(1 AS b))",
            // `XMLEXISTS` and `XMLTABLE` take `c_expr`.
            "SELECT XMLEXISTS('a' || 'b' PASSING x)",
            "SELECT XMLEXISTS('a' PASSING x IS NOT NULL)",
            "SELECT * FROM XMLTABLE('a' || 'b' PASSING x COLUMNS a int) t",
            // The legacy form of `JSON_OBJECT` takes no clauses, and the
            // `VALUE` form takes a `c_expr` key.
            "SELECT JSON_OBJECT(a RETURNING text)",
            "SELECT JSON_OBJECT(a NULL ON NULL)",
            "SELECT JSON_OBJECT(DEFAULT VALUE 1)",
            "SELECT JSON_OBJECTAGG(DEFAULT VALUE 1)",
            // An array constructor holds elements or sub-arrays, not both.
            "SELECT ARRAY[1, [2]]",
            "SELECT ARRAY[[1], 2]",
            "SELECT ARRAY[[$0, []]]",
            // `json_behavior_clause_opt` is ON EMPTY, then ON ERROR.
            "SELECT JSON_QUERY(a, '$' NULL ON EMPTY NULL ON EMPTY)",
            "SELECT JSON_QUERY(a, '$' NULL ON ERROR NULL ON EMPTY)",
            "SELECT JSON_QUERY(a, '$' NULL ON ERROR NULL ON ERROR)",
            // `JSON_EXISTS` and `JSON_TABLE` take only ON ERROR.
            "SELECT JSON_EXISTS(a, '$' TRUE ON EMPTY)",
            "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS (a int) NULL ON EMPTY) AS t",
        ],
        "gram.y wants a narrower form",
    );
    assert_accepts(&[
        "UPDATE t SET a = 1 WHERE CURRENT OF c",
        "DELETE FROM t WHERE CURRENT OF c",
        "SELECT 1 AS U&\"a\" UESCAPE '!'",
        "CREATE AGGREGATE a(int) (SFUNC = balkifnull(int8, int4), STYPE = int8)",
        "CREATE AGGREGATE a(int) (SFUNC = f(a.b, 1), STYPE = int8)",
        "CREATE INDEX ON t (a ops (x = 1))",
        "CREATE INDEX ON t (a ops (x = 'y'))",
        "SELECT XMLELEMENT(NAME a, XMLATTRIBUTES(1 AS b), 'c')",
        "SELECT XMLEXISTS(('a' || 'b') PASSING x)",
        "SELECT JSON_OBJECT(a, b)",
        "SELECT JSON_OBJECT(a => 1)",
        "SELECT JSON_OBJECT(a : b RETURNING text)",
        "SELECT JSON_OBJECT(DEFAULT : 1)",
        "SELECT JSON_OBJECT(a VALUE 1, b : 2)",
        "SELECT JSON_OBJECTAGG(a VALUE 1)",
        "SELECT ARRAY[[1], [2]]",
        "SELECT ARRAY[[], []]",
        "SELECT ARRAY[]",
        "SELECT JSON_QUERY(a, '$' NULL ON EMPTY NULL ON ERROR)",
        "SELECT JSON_QUERY(a, '$' NULL ON ERROR)",
        "SELECT JSON_EXISTS(a, '$' TRUE ON ERROR)",
    ]);
}

/// A `)` before a set operator closes each form that can end a
/// `select_clause` (issues #60 and #74).
#[test]
fn a_close_before_a_set_operator_parses() {
    assert_accepts(&[
        "TABLE ONLY (t) UNION TABLE u",
        "TABLE ONLY (s.t) INTERSECT TABLE ONLY (u)",
        "(TABLE ONLY (t) EXCEPT TABLE u)",
        "SELECT * FROM ONLY (t) UNION TABLE u",
        "SELECT 1 GROUP BY () UNION SELECT 2",
        "SELECT 1 GROUP BY ALL () INTERSECT SELECT 2",
        "SELECT 1 GROUP BY ROLLUP (a) UNION SELECT 2",
        "SELECT 1 GROUP BY CUBE (a) UNION SELECT 2",
        "SELECT 1 GROUP BY GROUPING SETS (a, ()) EXCEPT SELECT 2",
        "TABLE ONLY (t) ORDER BY 1",
        "SELECT 1 GROUP BY () ORDER BY 1",
    ]);
}

/// A `)` before a set operator is a plain `)`, as in parser.c, which has no
/// lookahead token for it. Each closing rule of the grammar takes it, so each
/// construct that ends in `)` can end the left operand of a set operation
/// (issue #74). One statement for each construct.
#[test]
fn each_construct_that_ends_in_a_close_can_end_a_set_operand() {
    assert_accepts(&[
        "SELECT 1 FROM t WHERE a IN (1) UNION SELECT 2",
        "SELECT 1 FROM t WHERE a IN (SELECT 1) UNION SELECT 2",
        "SELECT 1 FROM t WHERE a = ANY (ARRAY[1]) UNION SELECT 2",
        "SELECT 1 FROM t WHERE a = ANY (SELECT 1) INTERSECT SELECT 2",
        "SELECT 1 FROM t WHERE EXISTS (SELECT 1) EXCEPT SELECT 2",
        "SELECT ROW(1, 2) UNION SELECT ROW(3, 4)",
        "SELECT CAST(1 AS int) UNION SELECT 2",
        "SELECT COALESCE(1, 2) UNION SELECT 2",
        "SELECT EXTRACT(year FROM now()) UNION SELECT 2",
        "SELECT TREAT(a AS int) FROM t UNION SELECT 2",
        "SELECT count(*) OVER (PARTITION BY a) FROM t UNION SELECT 2",
        "SELECT count(*) FILTER (WHERE a > 1) FROM t UNION SELECT 2",
        "SELECT percentile_cont(0.5) WITHIN GROUP (ORDER BY a) FROM t UNION SELECT 2",
        "SELECT count(*) OVER w FROM t WINDOW w AS (PARTITION BY a) UNION SELECT 2",
        "SELECT 1 FROM t TABLESAMPLE system (10) UNION SELECT 2",
        "SELECT 1 FROM t TABLESAMPLE system (10) REPEATABLE (1) UNION SELECT 2",
        "SELECT 1 FROM t JOIN u USING (a) UNION SELECT 2",
        "SELECT 1 FROM ROWS FROM (f(1)) UNION SELECT 2",
        "SELECT 1 FROM XMLTABLE('/a' PASSING x COLUMNS a int) UNION SELECT 2",
        "SELECT XMLELEMENT(NAME a) UNION SELECT 2",
        "SELECT * FROM JSON_TABLE(j, '$' COLUMNS (a int)) UNION SELECT 2",
        "SELECT JSON_OBJECT() UNION SELECT 2",
        // The forms that a parenthesized query takes, which gram.y reads
        // through `select_with_parens` (REL_17_11 gram.y 12652-12686).
        "(SELECT 1) UNION SELECT 2",
        "((SELECT 1)) UNION SELECT 2",
        "SELECT (((SELECT 2)) UNION SELECT 2)",
        "SELECT 1 WHERE 1 IN ((SELECT 1) UNION SELECT 2)",
        "SELECT * FROM ((SELECT 1) UNION SELECT 2) s",
        "INSERT INTO t (a) (SELECT 1) UNION SELECT 2",
    ]);
}

/// gram.y `Sconst` is `SCONST`, which scan.l gives for a dollar-quoted
/// string and a continued string too (issue #74).
#[test]
fn an_sconst_takes_each_string_spelling() {
    assert_accepts(&[
        "CREATE ROLE r PASSWORD $$secret$$",
        "CREATE ROLE r PASSWORD $p$secret$p$",
        "CREATE ROLE r PASSWORD 'se'\n'cret'",
        "COPY t FROM STDIN WITH DELIMITER $$,$$",
        "CREATE DATABASE d LOCALE $$C$$",
    ]);
}

/// gram.y `set_rest_more: ROLE NonReservedWord_or_Sconst` (REL_17_11 gram.y
/// 1752): a `type_func_name_keyword` is a role name here, and the string is
/// any `Sconst` (issue #74).
#[test]
fn set_role_takes_a_non_reserved_word_or_an_sconst() {
    assert_accepts(&[
        "SET ROLE authorization",
        "SET ROLE left",
        "SET ROLE none",
        "SET ROLE 'r'",
        "SET ROLE E'r'",
        "SET ROLE $$r$$",
        "SET ROLE 'r'\n'x'",
    ]);
    assert_rejects(&["SET ROLE DEFAULT"], "DEFAULT is a reserved keyword");
}

/// Statements that a `gram.y` action always rejects with `ereport`, which
/// pgtk-sql rejects as a grammar shape (issue #55). The declared rejections
/// are in `tests/declared_rejections.rs`.
#[test]
fn statements_that_a_gram_y_action_rejects() {
    assert_rejects(
        &[
            // `aggr_arg` refuses the output modes.
            "CREATE AGGREGATE a(INOUT int) (SFUNC = f, STYPE = int)",
            "CREATE AGGREGATE a(x OUT int) (SFUNC = f, STYPE = int)",
            "ALTER AGGREGATE a(OUT int) RENAME TO b",
            "DROP AGGREGATE a(int ORDER BY OUT int)",
            // `CREATE SCHEMA IF NOT EXISTS` takes no schema element.
            "CREATE SCHEMA IF NOT EXISTS s CREATE SEQUENCE q",
            "CREATE SCHEMA IF NOT EXISTS AUTHORIZATION r CREATE TABLE t (a int)",
            // `PROGRAM` takes a string, not a standard stream.
            "COPY t TO PROGRAM STDIN",
            "COPY t FROM PROGRAM STDOUT",
            "COPY (SELECT 1) TO PROGRAM STDOUT",
            // `WHERE` only with `COPY FROM`.
            "COPY t TO STDOUT WHERE a > 1",
            "COPY t TO 'f' WITH WHERE true",
            // The legacy forms that the actions refuse.
            "CREATE ROLE r UNENCRYPTED PASSWORD 'x'",
            "ALTER ROLE r UNENCRYPTED PASSWORD 'x'",
            "CREATE TABLE t (a int REFERENCES p MATCH PARTIAL)",
            // A column list on `ON UPDATE SET NULL / SET DEFAULT`.
            "CREATE TABLE t (a int REFERENCES p ON UPDATE SET NULL (a))",
            "CREATE TABLE t (a int, FOREIGN KEY (a) REFERENCES p ON UPDATE SET DEFAULT (a))",
            // `WITH CHECK OPTION` on a recursive view.
            "CREATE RECURSIVE VIEW v (a) AS SELECT 1 WITH CHECK OPTION",
            "CREATE OR REPLACE RECURSIVE VIEW v (a) AS SELECT 1 WITH LOCAL CHECK OPTION",
            // A hash partition bound with no modulus or remainder, or a repeat.
            "CREATE TABLE p1 PARTITION OF p FOR VALUES WITH (MODULUS 2)",
            "CREATE TABLE p1 PARTITION OF p FOR VALUES WITH (REMAINDER 0)",
            "CREATE TABLE p1 PARTITION OF p FOR VALUES WITH (MODULUS 2, MODULUS 2)",
            "CREATE TABLE p1 PARTITION OF p FOR VALUES WITH (MODULUS 2, REMAINDER 0, REMAINDER 1)",
            // `preprocess_pubobj_list`: the first object names its kind, and a
            // schema continuation is a plain name.
            "CREATE PUBLICATION p FOR t",
            "ALTER PUBLICATION p ADD t",
            "ALTER PUBLICATION p DROP t*",
            "CREATE PUBLICATION p FOR TABLES IN SCHEMA s, t (a)",
            "CREATE PUBLICATION p FOR TABLES IN SCHEMA s, t WHERE (a > 1)",
            "CREATE PUBLICATION p FOR TABLES IN SCHEMA s, t*",
            "CREATE PUBLICATION p FOR TABLES IN SCHEMA s, ONLY t",
            "CREATE PUBLICATION p FOR TABLES IN SCHEMA s, a.b",
        ],
        "a gram.y action rejects the statement",
    );
    assert_accepts(&[
        "CREATE AGGREGATE a(IN int) (SFUNC = f, STYPE = int)",
        "CREATE AGGREGATE a(VARIADIC x int ORDER BY VARIADIC int) (SFUNC = f, STYPE = int)",
        "CREATE SCHEMA IF NOT EXISTS s",
        "CREATE SCHEMA IF NOT EXISTS AUTHORIZATION r",
        "CREATE SCHEMA s CREATE SEQUENCE q",
        "COPY t TO PROGRAM 'cat'",
        "COPY t FROM STDIN WHERE a > 1",
        "COPY (SELECT 1) TO STDOUT",
        "CREATE ROLE r ENCRYPTED PASSWORD 'x'",
        "CREATE TABLE t (a int REFERENCES p MATCH FULL ON DELETE SET NULL (a) ON UPDATE CASCADE)",
        "CREATE TABLE t (a int REFERENCES p ON UPDATE NO ACTION ON DELETE SET DEFAULT)",
        "CREATE RECURSIVE VIEW v (a) AS SELECT 1",
        "CREATE VIEW v AS SELECT 1 WITH CASCADED CHECK OPTION",
        "CREATE TABLE p1 PARTITION OF p FOR VALUES WITH (MODULUS 2, REMAINDER 0)",
        "CREATE TABLE p1 PARTITION OF p FOR VALUES WITH (REMAINDER 1, MODULUS 2)",
        "CREATE PUBLICATION p FOR TABLE t, u (a) WHERE (a > 1), ONLY v",
        "CREATE PUBLICATION p FOR TABLES IN SCHEMA s, current_schema, TABLE t, u",
        "ALTER PUBLICATION p ADD TABLE t, TABLES IN SCHEMA s, r",
        "CREATE TRIGGER t AFTER INSERT OR UPDATE OF a OR DELETE OR TRUNCATE ON x EXECUTE FUNCTION f()",
        "CREATE POLICY p ON t AS PERMISSIVE",
        "CREATE POLICY p ON t AS \"restrictive\"",
        "ALTER INDEX i ALTER 1 SET STATISTICS 100",
        "ALTER INDEX i ALTER 32767 SET STATISTICS 100",
        "SELECT U&'d!0061t' UESCAPE '!'",
    ]);
}

/// CREATE statement forms that gram.y does not have (issue #57).
#[test]
fn create_statements_keep_the_forms_of_gram_y() {
    assert_rejects(
        &[
            // `OptTemp` holds one persistence keyword.
            "CREATE TEMP UNLOGGED TABLE t (a int)",
            "CREATE TEMPORARY UNLOGGED TABLE t OF typ",
            // The clauses of CREATE TABLE are in the order of gram.y.
            "CREATE TABLE t USING heap OF typ",
            "CREATE TABLE t USING heap (a int)",
            "CREATE TABLE t USING heap PARTITION OF p DEFAULT",
            "CREATE TABLE t (a int) WITH OIDS",
            // `IF NOT EXISTS` takes a name.
            "CREATE INDEX IF NOT EXISTS ON t (a)",
            "CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS ON t (a)",
            "CREATE STATISTICS IF NOT EXISTS ON a, b FROM t",
            // A hash partition bound is an integer.
            "CREATE TABLE p1 PARTITION OF p FOR VALUES WITH (MODULUS DEFAULT, REMAINDER 0)",
            "CREATE TABLE p1 PARTITION OF p FOR VALUES WITH (MODULUS 2, REMAINDER 1 + 1)",
            // `PARTITION OF` takes exactly one bound.
            "CREATE TABLE p1 PARTITION OF p",
            "CREATE TABLE p1 PARTITION OF p FOR VALUES IN (1) DEFAULT",
            "CREATE FOREIGN TABLE p1 PARTITION OF p SERVER s",
            "CREATE FOREIGN TABLE p1 PARTITION OF p FOR VALUES IN (1) DEFAULT SERVER s",
            // A view has no access method.
            "CREATE VIEW v USING heap AS SELECT 1",
            "CREATE RECURSIVE VIEW v (a) USING heap AS SELECT 1",
            // A recursive view has a column list.
            "CREATE RECURSIVE VIEW v AS SELECT 1",
            "CREATE RECURSIVE VIEW v WITH (security_barrier) AS SELECT 1",
            // `key_actions`: one ON UPDATE and one ON DELETE at most.
            "CREATE TABLE t (a int REFERENCES p ON UPDATE CASCADE ON UPDATE RESTRICT)",
            "CREATE TABLE t (a int REFERENCES p ON DELETE CASCADE ON UPDATE RESTRICT ON DELETE NO ACTION)",
            // `UNIQUE USING INDEX` has no `NULLS [NOT] DISTINCT`.
            "CREATE TABLE t (a int, UNIQUE NULLS NOT DISTINCT USING INDEX i)",
            "ALTER TABLE t ADD UNIQUE NULLS DISTINCT USING INDEX i",
            // A CREATE SCHEMA element is no CREATE TABLE AS.
            "CREATE SCHEMA s CREATE TABLE t AS SELECT 1",
            // `into_clause` has no access method.
            "SELECT 1 INTO t USING heap",
            "SELECT 1 INTO UNLOGGED t USING heap",
            // `opt_with` comes once before the whole option list (issue #24).
            "CREATE DATABASE d WITH OWNER = x WITH TEMPLATE = y",
            "CREATE EXTENSION e WITH SCHEMA s WITH VERSION '1.0'",
            "CREATE GROUP g WITH SUPERUSER WITH CREATEDB",
            "CREATE ROLE r WITH LOGIN WITH SUPERUSER",
            "CREATE USER u WITH LOGIN WITH SUPERUSER",
        ],
        "gram.y has no such CREATE form",
    );
    assert_accepts(&[
        "CREATE TEMP TABLE t (a int)",
        "CREATE UNLOGGED TABLE t OF typ",
        "CREATE TABLE t OF typ (a WITH OPTIONS NOT NULL) PARTITION BY LIST (a) USING heap WITH (fillfactor = 70) TABLESPACE s",
        "CREATE TABLE t (a int) INHERITS (p) USING heap WITHOUT OIDS",
        "CREATE TABLE t USING heap AS SELECT 1",
        "CREATE TABLE t (a, b) USING heap WITH (fillfactor = 70) TABLESPACE s AS SELECT 1, 2 WITH NO DATA",
        "CREATE TEMP TABLE t ON COMMIT DROP AS SELECT 1",
        "CREATE INDEX IF NOT EXISTS i ON t (a)",
        "CREATE INDEX ON t (a)",
        "CREATE INDEX if ON t (a)",
        "CREATE STATISTICS IF NOT EXISTS s ON a, b FROM t",
        "CREATE STATISTICS s ON a, b FROM t",
        "CREATE TABLE p1 PARTITION OF p DEFAULT",
        "CREATE TABLE p1 PARTITION OF p FOR VALUES IN (1) USING heap",
        "CREATE FOREIGN TABLE p1 PARTITION OF p DEFAULT SERVER s",
        "CREATE RECURSIVE VIEW v (a) WITH (security_barrier) AS SELECT 1",
        "CREATE OR REPLACE TEMP VIEW v (a) AS SELECT 1",
        "CREATE TABLE t (a int REFERENCES p ON DELETE CASCADE ON UPDATE RESTRICT)",
        "CREATE TABLE t (a int, UNIQUE NULLS NOT DISTINCT (a))",
        "ALTER TABLE t ADD UNIQUE USING INDEX i",
        "CREATE SCHEMA s CREATE TABLE t (a int) CREATE VIEW v AS SELECT 1",
        "SELECT 1 INTO TEMP TABLE t",
        "CREATE DATABASE d WITH OWNER = x TEMPLATE = y",
        "CREATE DATABASE d WITH",
        "CREATE EXTENSION e WITH SCHEMA s VERSION '1.0'",
        "CREATE GROUP g WITH SUPERUSER CREATEDB",
        "CREATE ROLE r LOGIN",
        "CREATE USER u WITH LOGIN SUPERUSER",
    ]);
}

/// ALTER and utility statement forms that gram.y does not have (issue #58).
#[test]
fn alter_and_utility_statements_keep_the_forms_of_gram_y() {
    assert_rejects(
        &[
            // `IF EXISTS` only where gram.y has it.
            "ALTER STATISTICS IF EXISTS s OWNER TO r",
            "ALTER STATISTICS IF EXISTS s RENAME TO t",
            "ALTER STATISTICS IF EXISTS s SET SCHEMA t",
            "ALTER INDEX IF EXISTS i ATTACH PARTITION j",
            "ALTER INDEX IF EXISTS i DEPENDS ON EXTENSION e",
            "ALTER MATERIALIZED VIEW IF EXISTS m DEPENDS ON EXTENSION e",
            // `DEPENDS ON EXTENSION` is no `alter_table_cmd`.
            "ALTER TABLE t DEPENDS ON EXTENSION e",
            "ALTER FOREIGN TABLE t NO DEPENDS ON EXTENSION e",
            // `RENAME` and `SET SCHEMA` are alone after the name.
            "ALTER MATERIALIZED VIEW m OWNER TO r, SET SCHEMA s",
            "ALTER MATERIALIZED VIEW m SET TABLESPACE s, RENAME TO n",
            // EXPLAIN takes only CREATE TABLE AS.
            "EXPLAIN CREATE TABLE t OF typ",
            "EXPLAIN CREATE TABLE t (a int)",
            "EXPLAIN CREATE TABLE t ()",
            // SECURITY LABEL has no such object kind.
            "SECURITY LABEL ON RULE r ON t IS 'x'",
            "SECURITY LABEL ON TRIGGER tr ON t IS 'x'",
            "SECURITY LABEL ON POLICY p ON t IS 'x'",
            "SECURITY LABEL ON CONSTRAINT c ON t IS 'x'",
            "SECURITY LABEL ON OPERATOR + (int, int) IS 'x'",
            // `defacl_privilege_target` has no `PROCEDURES`.
            "ALTER DEFAULT PRIVILEGES GRANT ALL ON PROCEDURES TO r",
            // The option list and the old option keywords do not mix.
            "VACUUM (VERBOSE) FULL",
            "VACUUM (VERBOSE) ANALYZE t",
            "ANALYZE VERBOSE (SKIP_LOCKED) t",
            "CLUSTER (VERBOSE) VERBOSE t",
            "CLUSTER (VERBOSE) i ON t",
            // `DEFAULT VALUES` takes no column list and no `OVERRIDING`.
            "INSERT INTO t (a) DEFAULT VALUES",
            "INSERT INTO t OVERRIDING SYSTEM VALUE DEFAULT VALUES",
            // `TABLES IN SCHEMA` takes no column list.
            "CREATE PUBLICATION p FOR TABLES IN SCHEMA s (a)",
            "ALTER PUBLICATION p ADD TABLES IN SCHEMA s (a)",
            // A column number has no sign, and takes only SET STATISTICS.
            "ALTER INDEX i ALTER 1 SET (fillfactor = 70)",
            "ALTER TABLE t ALTER - 1 SET STATISTICS 1",
            "ALTER TABLE t ALTER 1 DROP NOT NULL",
            "ALTER FOREIGN TABLE t ALTER 1 OPTIONS (ADD a 'b')",
            // `OIDS` is no COPY option.
            "COPY t TO STDOUT OIDS",
            "COPY t FROM STDIN WITH OIDS FREEZE",
            // `ON CONFLICT` with an empty target, or `WHERE` with no target.
            "INSERT INTO t DEFAULT VALUES ON CONFLICT () DO NOTHING",
            "INSERT INTO t VALUES (1) ON CONFLICT WHERE a > 1 DO NOTHING",
            "INSERT INTO t VALUES (1) ON CONFLICT ON CONSTRAINT c WHERE a > 1 DO NOTHING",
        ],
        "gram.y has no such ALTER or utility form",
    );
    assert_accepts(&[
        "ALTER STATISTICS IF EXISTS s SET STATISTICS 100",
        "ALTER STATISTICS s OWNER TO r",
        "ALTER INDEX i ATTACH PARTITION j",
        "ALTER INDEX IF EXISTS i RENAME TO j",
        "ALTER INDEX IF EXISTS i SET TABLESPACE s",
        "ALTER INDEX i DEPENDS ON EXTENSION e",
        "ALTER MATERIALIZED VIEW m NO DEPENDS ON EXTENSION e",
        "ALTER MATERIALIZED VIEW IF EXISTS m RENAME COLUMN a TO b",
        "ALTER MATERIALIZED VIEW m OWNER TO r, SET TABLESPACE s",
        "ALTER MATERIALIZED VIEW m SET SCHEMA s",
        "ALTER TABLE IF EXISTS t RENAME TO u",
        "EXPLAIN CREATE TABLE t AS SELECT 1",
        "EXPLAIN CREATE TABLE t (a) AS SELECT 1",
        "SECURITY LABEL ON TABLE t IS 'x'",
        "SECURITY LABEL FOR p ON FUNCTION f(int) IS 'x'",
        "ALTER DEFAULT PRIVILEGES GRANT ALL ON ROUTINES TO r",
        "VACUUM FULL FREEZE VERBOSE ANALYZE t",
        "VACUUM (VERBOSE, ANALYZE) t (a)",
        "ANALYZE VERBOSE t",
        "ANALYZE (SKIP_LOCKED) t",
        "CLUSTER VERBOSE i ON t",
        "CLUSTER (VERBOSE) t USING i",
        "INSERT INTO t DEFAULT VALUES",
        "INSERT INTO t (a) OVERRIDING USER VALUE VALUES (1)",
        "CREATE PUBLICATION p FOR TABLES IN SCHEMA s",
        "ALTER INDEX i ALTER 1 SET STATISTICS 100",
        "ALTER INDEX i ALTER a SET (fillfactor = 70)",
        "ALTER TABLE t ALTER a DROP NOT NULL",
        "COPY t TO STDOUT CSV HEADER",
        "INSERT INTO t VALUES (1) ON CONFLICT (a) WHERE a > 1 DO NOTHING",
        "INSERT INTO t VALUES (1) ON CONFLICT DO NOTHING",
        "INSERT INTO t VALUES (1) ON CONFLICT ON CONSTRAINT c DO NOTHING",
    ]);
}

/// `ALTER TYPE ... SET (...)` takes gram.y `operator_def_list`, whose value is
/// optional only from 17 (REL_17_11 gram.y 10252; issue #58).
#[test]
fn alter_type_set_takes_an_operator_def_list() {
    assert_accepts(&["ALTER TYPE t SET (send = none, receive = r)"]);
    #[cfg(feature = "since-pg17")]
    assert_accepts(&["ALTER TYPE t SET (send)"]);
    #[cfg(not(feature = "since-pg17"))]
    assert_rejects(
        &["ALTER TYPE t SET (send)", "ALTER TYPE t SET (a = 1, b)"],
        "an operator_def_elem has a value before 17",
    );
}

/// A JSON_TABLE `EXISTS` column takes only a path and an `ON ERROR`
/// behavior (REL_17_11 gram.y 14228-14246).
#[cfg(feature = "since-pg17")]
#[test]
fn a_json_table_exists_column_has_no_wrapper_or_format() {
    assert_rejects(
        &[
            "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS (a int EXISTS WITH WRAPPER))",
            "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS (a int EXISTS FORMAT JSON))",
            "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS (a int EXISTS KEEP QUOTES))",
            "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS (a int EXISTS NULL ON EMPTY))",
        ],
        "an EXISTS column has no wrapper, format, quotes or ON EMPTY",
    );
    assert_accepts(&[
        "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS (a int EXISTS PATH '$.a' FALSE ON ERROR))",
        "SELECT * FROM JSON_TABLE('{}', '$' COLUMNS (a jsonb FORMAT JSON PATH '$.a' WITH WRAPPER KEEP QUOTES NULL ON EMPTY))",
    ]);
}

/// gram.y `a_expr: row OVERLAPS row` takes a row on each side: `ROW(...)` or
/// `(a, b)` (issue #59). A row takes no indirection: `(a, b).x` is a syntax
/// error, because only `'(' a_expr ')' opt_indirection` has one.
#[test]
fn overlaps_takes_a_row_on_each_side() {
    assert_rejects(
        &[
            "SELECT a OVERLAPS (3, 4) FROM t",
            "SELECT (a) OVERLAPS (3, 4) FROM t",
            "SELECT (1, 2) OVERLAPS (3)",
            "SELECT ARRAY[] OVERLAPS (1, 2)",
            "SELECT (1, 2) OVERLAPS ((3, 4))",
            "SELECT (1, 2) OVERLAPS (3, 4) OVERLAPS (5, 6)",
            "DELETE FROM t WHERE E'' OVERLAPS ARRAY[]",
            "SELECT (1, 2).x",
        ],
        "an operand of OVERLAPS is a row",
    );
    assert_accepts(&[
        "SELECT (1, 2) OVERLAPS (3, 4)",
        "SELECT ROW(1, 2) OVERLAPS ROW(3, 4)",
        "SELECT (1, 2) OVERLAPS ROW(3, 4)",
        "SELECT x < (1, 2) OVERLAPS (3, 4) FROM t",
        "SELECT NOT (1, 2) OVERLAPS (3, 4) AND true",
        "SELECT * FROM t WHERE (a, b) OVERLAPS (c, d) IS TRUE",
        "SELECT (1, 2, 3), ((1, 2)), ((1, 2)).f1",
        "SELECT (a).x, (a).* FROM t",
    ]);
}

/// gram.y `func_name: ColId indirection` and `check_func_name`: a function
/// name or the type name of a typed literal is dotted attribute names only,
/// never a `.*` or a subscript (issue #59). `check_indirection` refuses a `.*`
/// that is not the last element (issue #55).
#[test]
fn a_qualified_name_keeps_the_forms_of_gram_y() {
    assert_rejects(
        &[
            "EXECUTE p(t.* ())",
            "SELECT t.*() FROM t",
            "SELECT t.a.*(1) FROM t",
            "SELECT t.a[1](2) FROM t",
            "SELECT t.a[1] 'x' FROM t",
            "SELECT t.a[1](2) 'x' FROM t",
            "SELECT t.*.a FROM t",
            "SELECT t.*.* FROM t",
            "SELECT t.a[1].*.b FROM t",
            "SELECT t.a.*[1] FROM t",
        ],
        "gram.y refuses this qualified name",
    );
    assert_accepts(&[
        "SELECT pg_catalog.int4 '1'",
        "SELECT pg_catalog.numeric(10, 2) '1'",
        "SELECT s.f(1), s.f(1) 'x'",
        "SELECT t.*, t.a.*, t.a[1].*, t.a[1].b, t.a[1:2][3].c.d FROM t",
    ]);
}

/// The path of `JSON_TABLE` is a string constant, in parentheses or not: the
/// gram.y action refuses each other value (issue #59).
#[cfg(feature = "since-pg17")]
#[test]
fn the_path_of_json_table_is_a_string_constant() {
    assert_rejects(
        &[
            "SELECT * FROM JSON_TABLE(j, p COLUMNS (a int))",
            "SELECT * FROM JSON_TABLE(j, '$' || '' COLUMNS (a int))",
            "SELECT * FROM JSON_TABLE(j, B'1' COLUMNS (a int))",
            "SELECT * FROM JSON_TABLE(j, text '$' COLUMNS (a int))",
        ],
        "the path is a string constant",
    );
    assert_accepts(&[
        "SELECT * FROM JSON_TABLE(j, '$' COLUMNS (a int))",
        "SELECT * FROM JSON_TABLE(j, E'$' COLUMNS (a int))",
        "SELECT * FROM JSON_TABLE(j, $p$$$p$ COLUMNS (a int))",
        "SELECT * FROM JSON_TABLE(j, (('$')) AS p COLUMNS (a int))",
    ]);
}

/// gram.y `frame_extent` refuses some combinations of frame bounds in its
/// actions (REL_17_11 gram.y 16327-16383; issue #55). The grammar admits only
/// the combinations that the actions keep, except a start of `UNBOUNDED
/// FOLLOWING` and an end of `UNBOUNDED PRECEDING`, which stay so that the
/// keyword shifts.
#[test]
fn a_window_frame_keeps_the_bounds_that_gram_y_keeps() {
    assert_rejects(
        &[
            "SELECT count(*) OVER (ROWS 1 FOLLOWING) FROM t",
            "SELECT count(*) OVER (ROWS UNBOUNDED FOLLOWING) FROM t",
            "SELECT count(*) OVER (ROWS BETWEEN CURRENT ROW AND 1 PRECEDING) FROM t",
            "SELECT count(*) OVER (ROWS BETWEEN CURRENT ROW AND UNBOUNDED PRECEDING) FROM t",
            "SELECT count(*) OVER (ROWS BETWEEN 1 FOLLOWING AND CURRENT ROW) FROM t",
            "SELECT count(*) OVER (ROWS BETWEEN 1 FOLLOWING AND 1 PRECEDING) FROM t",
            "SELECT count(*) OVER (ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED PRECEDING) FROM t",
        ],
        "gram.y refuses this frame",
    );
    assert_accepts(&[
        "SELECT count(*) OVER (ROWS UNBOUNDED PRECEDING) FROM t",
        "SELECT count(*) OVER (ROWS 1 PRECEDING) FROM t",
        "SELECT count(*) OVER (ROWS CURRENT ROW) FROM t",
        "SELECT count(*) OVER (ROWS unbounded PRECEDING) FROM t",
        "SELECT count(*) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING) FROM t",
        "SELECT count(*) OVER (ROWS BETWEEN 1 PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
        "SELECT count(*) OVER (ROWS BETWEEN CURRENT ROW AND CURRENT ROW EXCLUDE TIES) FROM t",
        "SELECT count(*) OVER (ROWS BETWEEN CURRENT ROW AND 1 FOLLOWING) FROM t",
        "SELECT count(*) OVER (ROWS BETWEEN 1 FOLLOWING AND 2 FOLLOWING) FROM t",
        "SELECT count(*) OVER (ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED FOLLOWING) FROM t",
        "SELECT count(*) OVER (ROWS BETWEEN 1 PRECEDING AND \"unbounded\" FOLLOWING) FROM t",
    ]);
}

/// `check_indirection` refuses a `.*` that is not the last element of the
/// indirection of a parenthesized value or a sub-select too (REL_17_11 gram.y
/// 18779; issue #55).
#[test]
fn a_star_ends_the_indirection_of_a_parenthesized_value() {
    assert_rejects(
        &[
            "SELECT (a).*.b FROM t",
            "SELECT (a).*[1] FROM t",
            "SELECT (a).b.*.* FROM t",
            "SELECT (SELECT 1).*[1]",
            "SELECT (SELECT a FROM t).*.b",
        ],
        "a star ends the indirection",
    );
    assert_accepts(&[
        "SELECT (a).*, (a).b.*, (a).b[1].*, (a)[1].b FROM t",
        "SELECT (SELECT a FROM t).b[1], (SELECT a FROM t).*",
    ]);
}

/// gram.y `json_wrapper_behavior` (REL_17_11 gram.y 16854-16864):
/// `CONDITIONAL` and `UNCONDITIONAL` follow `WITH` only.
#[cfg(feature = "since-pg17")]
#[test]
fn a_json_wrapper_takes_its_behavior_after_with_only() {
    assert_rejects(
        &[
            "SELECT JSON_QUERY(j, '$' WITHOUT CONDITIONAL WRAPPER)",
            "SELECT JSON_QUERY(j, '$' WITHOUT UNCONDITIONAL ARRAY WRAPPER)",
        ],
        "the behavior follows WITH only",
    );
    assert_accepts(&[
        "SELECT JSON_QUERY(j, '$' WITHOUT WRAPPER)",
        "SELECT JSON_QUERY(j, '$' WITHOUT ARRAY WRAPPER)",
        "SELECT JSON_QUERY(j, '$' WITH WRAPPER)",
        "SELECT JSON_QUERY(j, '$' WITH CONDITIONAL ARRAY WRAPPER)",
        "SELECT JSON_QUERY(j, '$' WITH UNCONDITIONAL WRAPPER)",
    ]);
}
