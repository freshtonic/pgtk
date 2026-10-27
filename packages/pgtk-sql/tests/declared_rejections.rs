//! `ConstraintAttributeSpec`'s own conflict checks, and the option checks of
//! `xmltable_column_el` and the precision check of `opt_float`, reproduced as
//! Recursa declared-rejection rules (`docs/declared-rejections.md` in Recursa;
//! `freshtonic/recursa#138`, `#141` and `#144`, `freshtonic/pg-sql#86`, `#85`,
//! `#87` and `#88`).
//!
//! `processCASbits`'s action (REL_17_11 gram.y 6098-6125, `ConstraintAttributeSpec`)
//! rejects two conflicts among the deferrability entries of a constraint
//! attribute list: the specific message for `NOT DEFERRABLE` beside
//! `INITIALLY DEFERRED` (REL_17_11 gram.y 6114), and the generic message for
//! `NOT DEFERRABLE` beside `DEFERRABLE` or `INITIALLY IMMEDIATE` beside
//! `INITIALLY DEFERRED` (REL_17_11 gram.y 6121). This suite proves the
//! declared rejection reports the right SQLSTATE, the right message, and a
//! span over the item that completed the pair — never the whole list — and
//! that the value, arena and flat parse entry points agree.

use pgtk_sql::ast::Statement;
use pgtk_sql::lex;
use recursa::{ArenaParse, ParseError};

/// Parses one complete statement and returns the error, through whichever
/// parse entry point `entry` names.
fn parse_err(source: &'static str, entry: Entry) -> ParseError {
    let lexed = lex(source);
    assert_eq!(lexed.errors().count(), 0, "lex errors in {source:?}");
    match entry {
        Entry::Value => {
            let mut input = lexed.input();
            Statement::parse(&mut input).expect_err("statement must be rejected")
        }
        Entry::Arena => {
            let mut input = lexed.input();
            Statement::parse_arena(&mut input).expect_err("statement must be rejected")
        }
        Entry::Flat => {
            let mut input = lexed.input();
            match Statement::parse_flat(&mut input) {
                Ok(_) => panic!("{source:?} must be rejected"),
                Err(error) => error,
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Entry {
    Value,
    Arena,
    Flat,
}

/// Asserts that every parse entry point rejects `source` with the same
/// declared rejection: runtime code `RCA4105`, the rule's `code`, `message`,
/// and a span over exactly `expect_span_text` within `source`.
fn assert_declared_rejection(
    source: &'static str,
    expect_code: &str,
    expect_message: &str,
    #[cfg_attr(not(feature = "spans"), allow(unused_variables))] expect_span_text: &str,
) {
    for entry in [Entry::Value, Entry::Arena, Entry::Flat] {
        let error = parse_err(source, entry);
        assert_eq!(
            error.code(),
            "RCA4105",
            "a declared rejection reports RCA4105 for {source:?}"
        );
        let note = error
            .rejection()
            .unwrap_or_else(|| panic!("{source:?} must fail as a declared rejection, got {error}"));
        assert_eq!(note.code(), Some(expect_code), "rule code for {source:?}");
        assert_eq!(
            note.message(),
            expect_message,
            "rule message for {source:?}"
        );
        assert_eq!(
            error.to_string(),
            expect_message,
            "a declared rejection's Display is the rule's message"
        );

        #[cfg(feature = "spans")]
        {
            let start = source
                .find(expect_span_text)
                .unwrap_or_else(|| panic!("{expect_span_text:?} not found in {source:?}"));
            let end = start + expect_span_text.len();
            let expected_span = recursa::Span::new(start, end).unwrap();
            assert_eq!(
                error.span(),
                expected_span,
                "rejection span for {source:?} must cover {expect_span_text:?}"
            );
        }
    }
}

#[test]
fn not_deferrable_beside_initially_deferred_is_the_specific_rule() {
    assert_declared_rejection(
        "CREATE TABLE t (a int, UNIQUE (a) NOT DEFERRABLE INITIALLY DEFERRED)",
        "42601",
        "constraint declared INITIALLY DEFERRED must be DEFERRABLE",
        "INITIALLY DEFERRED",
    );
}

#[test]
fn deferrable_beside_not_deferrable_is_the_generic_rule() {
    assert_declared_rejection(
        "CREATE TABLE t (a int, UNIQUE (a) DEFERRABLE NOT DEFERRABLE)",
        "42601",
        "conflicting constraint properties",
        "NOT DEFERRABLE",
    );
}

/// One XMLTABLE column whose option list is `options`.
macro_rules! xmltable_column {
    ($options:literal) => {
        concat!(
            "SELECT * FROM XMLTABLE('/r' PASSING x COLUMNS a int ",
            $options,
            ")"
        )
    };
}

#[test]
fn an_unknown_xmltable_option_name_is_unrecognized_even_when_it_is_the_first() {
    // REL_17_11 gram.y 14065, `xmltable_column_el`.
    assert_declared_rejection(
        xmltable_column!("foo 'a'"),
        "42601",
        "unrecognized column option",
        "foo 'a'",
    );
    assert_declared_rejection(
        xmltable_column!("NULL \"PATH\" 'a'"),
        "42601",
        "unrecognized column option",
        "\"PATH\" 'a'",
    );
}

#[test]
fn a_second_path_is_refused_whichever_spelling_it_has() {
    // REL_17_11 gram.y 14047: an `IDENT b_expr` option named `path` counts as PATH.
    assert_declared_rejection(
        xmltable_column!("PATH 'a' \"path\" 'b'"),
        "42601",
        "only one PATH value per column is allowed",
        "\"path\" 'b'",
    );
}

#[test]
fn a_second_default_is_refused_whichever_spelling_it_has() {
    // REL_17_11 gram.y 14038.
    assert_declared_rejection(
        xmltable_column!("DEFAULT 1 \"default\" 2"),
        "42601",
        "only one DEFAULT value is allowed",
        "\"default\" 2",
    );
}

#[test]
fn a_second_nullability_is_refused() {
    // REL_17_11 gram.y 14056.
    assert_declared_rejection(
        xmltable_column!("NULL NOT NULL"),
        "42601",
        "conflicting or redundant NULL / NOT NULL declarations",
        "NOT NULL",
    );
}

/// From 15 the `IDENT b_expr` action refuses the internal nullability name
/// itself (REL_15_19 gram.y 13867-13871); before 15 it is only unrecognized.
#[test]
fn the_internal_nullability_name_is_refused_by_name() {
    let message = if cfg!(feature = "since-pg15") {
        "option name cannot be used in XMLTABLE"
    } else {
        "unrecognized column option"
    };
    assert_declared_rejection(
        xmltable_column!("\"__pg__is_not_null\" true"),
        "42601",
        message,
        "\"__pg__is_not_null\" true",
    );
}

/// The `opt_float` action refuses a precision below 1 and one above 53, each
/// with its own message, at the `Iconst` (REL_17_11 gram.y 14474 and 14483).
#[test]
fn a_float_precision_out_of_range_is_refused_at_the_iconst() {
    assert_declared_rejection(
        "SELECT 1::float(0)",
        "22023",
        "precision for type float must be at least 1 bit",
        "0",
    );
    assert_declared_rejection(
        "CREATE TABLE t (a float(54))",
        "22023",
        "precision for type float must be less than 54 bits",
        "54",
    );
}

/// The `RoleId` action rejects every `RoleSpec` that is no plain name, each
/// with its own message, and `RoleSpec` itself rejects `none` (REL_17_11
/// gram.y 17315-17336 and 17361; issue #53).
#[test]
fn a_role_id_is_a_plain_role_name() {
    for (source, message, span) in [
        (
            "ALTER ROLE CURRENT_ROLE RENAME TO id",
            "CURRENT_ROLE cannot be used as a role name here",
            "CURRENT_ROLE",
        ),
        (
            "ALTER USER CURRENT_ROLE RENAME TO id",
            "CURRENT_ROLE cannot be used as a role name here",
            "CURRENT_ROLE",
        ),
        (
            "ALTER GROUP CURRENT_ROLE RENAME TO id",
            "CURRENT_ROLE cannot be used as a role name here",
            "CURRENT_ROLE",
        ),
        (
            "ALTER ROLE r RENAME TO current_user",
            "CURRENT_USER cannot be used as a role name here",
            "current_user",
        ),
        (
            "CREATE ROLE SESSION_USER",
            "SESSION_USER cannot be used as a role name here",
            "SESSION_USER",
        ),
        (
            "CREATE USER public",
            "role name \"public\" is reserved",
            "public",
        ),
        (
            "CREATE GROUP \"none\"",
            "role name \"none\" is reserved",
            "\"none\"",
        ),
        (
            "ALTER TABLE t OWNER TO none",
            "role name \"none\" is reserved",
            "none",
        ),
    ] {
        assert_declared_rejection(source, "42939", message, span);
    }
}

/// The action of gram.y `TriggerEvents` refuses an event that the list has
/// already, with or without a column list on `UPDATE`, at the event that
/// repeats (REL_17_11 gram.y 5972; issue #55).
#[test]
fn a_repeated_trigger_event_is_refused_at_the_repeat() {
    for (source, span) in [
        (
            "CREATE TRIGGER t AFTER insert OR INSERT ON x EXECUTE FUNCTION f()",
            "INSERT",
        ),
        (
            "CREATE TRIGGER t AFTER UPDATE OR DELETE OR UPDATE OF b ON x EXECUTE FUNCTION f()",
            "UPDATE OF b",
        ),
        (
            "CREATE TRIGGER t BEFORE delete OR DELETE ON x EXECUTE PROCEDURE f()",
            "DELETE",
        ),
        (
            "CREATE TRIGGER t AFTER truncate OR TRUNCATE ON x EXECUTE FUNCTION f()",
            "TRUNCATE",
        ),
    ] {
        assert_declared_rejection(source, "42601", "duplicate trigger events specified", span);
    }
}

/// gram.y `RowSecurityDefaultPermissive` reads `AS IDENT` and refuses each
/// name but `permissive` and `restrictive`, at the name (REL_17_11 gram.y
/// 5849; issue #55).
#[test]
fn a_policy_kind_other_than_permissive_or_restrictive_is_refused() {
    for (source, span) in [
        ("CREATE POLICY p ON t AS ugly", "ugly"),
        (
            "CREATE POLICY p ON t AS \"PERMISSIVE\" USING (true)",
            "\"PERMISSIVE\"",
        ),
    ] {
        assert_declared_rejection(source, "42601", "unrecognized row security option", span);
    }
}

/// gram.y `alter_table_cmd: ALTER opt_column Iconst SET STATISTICS` refuses a
/// column number below 1 or above 32767, at the number (REL_17_11 gram.y
/// 2487; issue #55).
#[test]
fn a_column_number_out_of_range_is_refused_at_the_number() {
    for (source, span) in [
        ("ALTER INDEX i ALTER 0 SET STATISTICS 100", "0"),
        (
            "ALTER INDEX i ALTER COLUMN 32768 SET STATISTICS 100",
            "32768",
        ),
        ("ALTER TABLE t ALTER 40000 SET STATISTICS 1", "40000"),
    ] {
        assert_declared_rejection(
            source,
            "22023",
            "column number must be in range from 1 to 32767",
            span,
        );
    }
}

/// From 16, gram.y `PartitionSpec` reads the strategy with
/// `parsePartitionStrategy`, which refuses each name but `list`, `range` and
/// `hash` (REL_16_15 gram.y 18756; issue #55).
#[cfg(feature = "since-pg16")]
#[test]
fn an_unknown_partition_strategy_is_refused_at_the_name() {
    assert_declared_rejection(
        "CREATE TABLE t (a int) PARTITION BY spread (a)",
        "22023",
        "unrecognized partitioning strategy",
        "spread",
    );
}

/// gram.y `json_format_clause` refuses each encoding but `utf8`, `utf16`
/// and `utf32` (REL_17_11 gram.y 16926; issue #55).
#[cfg(feature = "since-pg16")]
#[test]
fn an_unknown_json_encoding_is_refused_at_the_name() {
    assert_declared_rejection(
        "SELECT JSON_OBJECT('a' : 1 RETURNING bytea FORMAT JSON ENCODING latin1)",
        "22023",
        "unrecognized JSON encoding",
        "latin1",
    );
}

/// `base_yylex` refuses a `UESCAPE` string that is not one character that
/// `check_uescapechar` admits (REL_17_11 parser.c 278; issue #55). The
/// string is any `SCONST` spelling, and its value is what is checked.
#[test]
fn an_invalid_uescape_character_is_refused_at_the_string() {
    for (source, span) in [
        ("SELECT U&'d!0061t' UESCAPE ''", "''"),
        ("SELECT U&'d!0061t' UESCAPE 'ab'", "'ab'"),
        ("SELECT U&'d!0061t' UESCAPE 'a'", "'a'"),
        ("SELECT U&'d!0061t' UESCAPE '+'", "'+'"),
        ("SELECT U&'d!0061t' UESCAPE E'!!'", "E'!!'"),
        ("SELECT U&'d!0061t' UESCAPE $$ $$", "$$ $$"),
        ("SELECT U&'d!0061t' UESCAPE '!'\n'!'", "'!'\n'!'"),
    ] {
        assert_declared_rejection(source, "42601", "invalid Unicode escape character", span);
    }
}

/// Asserts the declared rejection of `source` in every storage, and, with the
/// oracle, that PostgreSQL refuses `source` with the same message.
fn assert_postgres_rejection(source: &'static str, code: &str, message: &str, span: &str) {
    assert_declared_rejection(source, code, message, span);
    #[cfg(feature = "postgres-oracle")]
    assert_eq!(
        pgtk_oracle::parse_error(source).as_deref(),
        Some(message),
        "PostgreSQL refuses {source:?} with the same message"
    );
}

/// Asserts that pgtk-sql and, with the oracle, PostgreSQL accept `source`.
fn assert_postgres_accepts(source: &'static str) {
    for entry in [Entry::Value, Entry::Arena, Entry::Flat] {
        let lexed = lex(source);
        let mut input = lexed.input();
        let accepted = match entry {
            Entry::Value => Statement::parse(&mut input).is_ok(),
            Entry::Arena => Statement::parse_arena(&mut input).is_ok(),
            Entry::Flat => Statement::parse_flat(&mut input).is_ok(),
        };
        assert!(accepted && input.is_eof(), "pgtk-sql accepts {source:?}");
    }
    #[cfg(feature = "postgres-oracle")]
    assert_eq!(
        pgtk_oracle::parse_error(source),
        None,
        "PostgreSQL accepts {source:?}"
    );
}

/// gram.y `a_expr: row OVERLAPS row` refuses a row that is not two values, at
/// the row (REL_17_11 gram.y 15002, 15007; issue #55).
#[test]
fn an_overlaps_row_holds_two_values() {
    assert_postgres_rejection(
        "SELECT (1, 2, 3) OVERLAPS (4, 5)",
        "42601",
        "wrong number of parameters on left side of OVERLAPS expression",
        "(1, 2, 3)",
    );
    assert_postgres_rejection(
        "SELECT (1, 2) OVERLAPS ROW(4)",
        "42601",
        "wrong number of parameters on right side of OVERLAPS expression",
        "ROW(4)",
    );
    assert_postgres_accepts("SELECT (1, 2) OVERLAPS ROW(3, 4)");
}

/// gram.y `AexprConst: func_name '(' func_arg_list opt_sort_clause ')'
/// Sconst` refuses a named argument (REL_17_11 gram.y 17250; issue #55).
#[test]
fn a_type_modifier_has_no_parameter_name() {
    assert_postgres_rejection(
        "SELECT foo(n := 20) 'x'",
        "42601",
        "type modifier cannot have parameter name",
        "n := 20",
    );
    assert_postgres_accepts("SELECT foo(20) 'x'");
}

/// gram.y `mergeTableFuncParameters` refuses an `OUT` or `INOUT` argument of
/// a `RETURNS TABLE` function (REL_17_11 gram.y 19129; issue #55). The error
/// has no position, so the span is the statement.
#[test]
fn a_table_function_has_no_output_argument() {
    for source in [
        "CREATE FUNCTION f(OUT a int) RETURNS TABLE(b int) AS 'x' LANGUAGE sql",
        "CREATE FUNCTION f(a INOUT int) RETURNS TABLE(b int) AS 'x' LANGUAGE sql",
    ] {
        assert_postgres_rejection(
            source,
            "42601",
            "OUT and INOUT arguments aren't allowed in TABLE functions",
            source,
        );
    }
    assert_postgres_accepts(
        "CREATE FUNCTION f(VARIADIC a int[]) RETURNS TABLE(b int) AS 'x' LANGUAGE sql",
    );
    assert_postgres_accepts("CREATE FUNCTION f(OUT a int) RETURNS int AS 'x' LANGUAGE sql");
}

/// gram.y `makeOrderedSetArgs` refuses an ordered-set aggregate whose last
/// direct argument is `VARIADIC` unless the aggregated arguments are one
/// `VARIADIC` argument of the same type (REL_17_11 gram.y 18843; issue #55).
#[test]
fn an_ordered_set_aggregate_with_a_variadic_direct_argument_repeats_it() {
    for (source, span) in [
        (
            "CREATE AGGREGATE a(VARIADIC int ORDER BY VARIADIC text) (sfunc = f, stype = int)",
            "ORDER BY VARIADIC text",
        ),
        (
            "CREATE AGGREGATE a(VARIADIC int ORDER BY int) (sfunc = f, stype = int)",
            "ORDER BY int",
        ),
        (
            "ALTER AGGREGATE a(VARIADIC int ORDER BY VARIADIC int, int) RENAME TO b",
            "ORDER BY VARIADIC int, int",
        ),
    ] {
        assert_postgres_rejection(
            source,
            "0A000",
            "an ordered-set aggregate with a VARIADIC direct argument must have one VARIADIC \
             aggregated argument of the same data type",
            span,
        );
    }
    assert_postgres_accepts(
        "CREATE AGGREGATE a(VARIADIC \"any\" ORDER BY VARIADIC \"any\") (sfunc = f, stype = int)",
    );
    assert_postgres_accepts(
        "CREATE AGGREGATE a(int ORDER BY VARIADIC text) (sfunc = f, stype = int)",
    );
}

/// gram.y `insertSelectOptions` refuses a second `ORDER BY`, `OFFSET`,
/// `LIMIT` or `WITH` around a parenthesized query, and `WITH TIES` without
/// `ORDER BY` (REL_17_11 gram.y 18880-18936; issue #55).
#[test]
fn a_query_takes_each_select_option_once() {
    for (source, message, span) in [
        (
            "(SELECT 1 ORDER BY 1) ORDER BY 2",
            "multiple ORDER BY clauses not allowed",
            "ORDER BY 2",
        ),
        (
            "(SELECT 1 OFFSET 1) OFFSET 2",
            "multiple OFFSET clauses not allowed",
            "OFFSET 2",
        ),
        (
            "(SELECT 1 LIMIT 1) LIMIT 2",
            "multiple LIMIT clauses not allowed",
            "LIMIT 2",
        ),
        (
            "SELECT 1 FETCH FIRST ROW WITH TIES",
            "WITH TIES cannot be specified without ORDER BY clause",
            "SELECT 1 FETCH FIRST ROW WITH TIES",
        ),
        (
            "WITH a AS (SELECT 1) (WITH b AS (SELECT 2) SELECT 3)",
            "multiple WITH clauses not allowed",
            "WITH a AS (SELECT 1)",
        ),
    ] {
        assert_postgres_rejection(source, "42601", message, span);
    }
    assert_postgres_accepts("(SELECT 1 ORDER BY 1) FETCH FIRST ROW WITH TIES");
    assert_postgres_accepts("(SELECT 1 LIMIT 1) OFFSET 2");
    assert_postgres_accepts("WITH a AS (SELECT 1) (SELECT 3)");
}

/// gram.y `frame_extent` refuses a frame that starts at `UNBOUNDED
/// FOLLOWING` or ends at `UNBOUNDED PRECEDING`, and reports the start first
/// (REL_17_11 gram.y 16335, 16359, 16364; issue #55). Each bound keeps its
/// arm, so the keyword shifts as in gram.y, and a declared rule refuses it.
#[test]
fn a_frame_bound_is_refused_at_its_bound() {
    for (source, message, span) in [
        (
            "SELECT sum(x) OVER (ROWS UNBOUNDED FOLLOWING) FROM t",
            "frame start cannot be UNBOUNDED FOLLOWING",
            "UNBOUNDED FOLLOWING",
        ),
        (
            "SELECT sum(x) OVER (ROWS BETWEEN UNBOUNDED FOLLOWING AND CURRENT ROW) FROM t",
            "frame start cannot be UNBOUNDED FOLLOWING",
            "UNBOUNDED FOLLOWING AND CURRENT ROW",
        ),
        (
            "SELECT sum(x) OVER (ROWS BETWEEN UNBOUNDED FOLLOWING AND UNBOUNDED PRECEDING) FROM t",
            "frame start cannot be UNBOUNDED FOLLOWING",
            "UNBOUNDED FOLLOWING AND UNBOUNDED PRECEDING",
        ),
        (
            "SELECT sum(x) OVER (ROWS BETWEEN 1 PRECEDING AND UNBOUNDED PRECEDING) FROM t",
            "frame end cannot be UNBOUNDED PRECEDING",
            "UNBOUNDED PRECEDING",
        ),
        (
            "SELECT sum(x) OVER (ROWS BETWEEN CURRENT ROW AND UNBOUNDED PRECEDING) FROM t",
            "frame end cannot be UNBOUNDED PRECEDING",
            "UNBOUNDED PRECEDING",
        ),
        (
            "SELECT sum(x) OVER (ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED PRECEDING) FROM t",
            "frame end cannot be UNBOUNDED PRECEDING",
            "UNBOUNDED PRECEDING",
        ),
    ] {
        assert_postgres_rejection(source, "42P20", message, span);
    }
    assert_postgres_accepts(
        "SELECT sum(x) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
}

/// A declared classification reads `U&"..." UESCAPE 'c'` as the name it
/// decodes to, as gram.y reads the `IDENT` that `base_yylex` returns: the
/// role name `none`, an XMLTABLE `path` option, a partitioning strategy and
/// a JSON encoding (Recursa `#[classify(via ...)]`).
#[test]
fn a_classification_reads_a_unicode_identifier_as_its_decoded_name() {
    assert_postgres_rejection(
        "GRANT g TO U&\"n!006fne\" UESCAPE '!'",
        "42939",
        "role name \"none\" is reserved",
        "U&\"n!006fne\" UESCAPE '!'",
    );
    assert_postgres_rejection(
        "CREATE ROLE U&\"publi!0063\" UESCAPE '!'",
        "42939",
        "role name \"public\" is reserved",
        "U&\"publi!0063\" UESCAPE '!'",
    );
    assert_postgres_rejection(
        "CREATE ROLE U&\"publi\\0063\"",
        "42939",
        "role name \"public\" is reserved",
        "U&\"publi\\0063\"",
    );
    // A decoded `CURRENT_USER` is a name, not the keyword.
    assert_postgres_accepts("CREATE ROLE U&\"CURRENT_USER\" UESCAPE '!'");
    assert_postgres_accepts("GRANT g TO U&\"n!006f\" UESCAPE '!'");
    assert_postgres_rejection(
        "SELECT * FROM XMLTABLE('/r' PASSING x COLUMNS a int PATH 'a' U&\"p!0061th\" UESCAPE '!' 'b')",
        "42601",
        "only one PATH value per column is allowed",
        "U&\"p!0061th\" UESCAPE '!' 'b'",
    );
    // PostgreSQL's message names the option; the rule's static message does
    // not (`XmlTableColumnOption`).
    let unrecognized =
        "SELECT * FROM XMLTABLE('/r' PASSING x COLUMNS a int U&\"p!0061x\" UESCAPE '!' 'b')";
    assert_declared_rejection(
        unrecognized,
        "42601",
        "unrecognized column option",
        "U&\"p!0061x\" UESCAPE '!' 'b'",
    );
    #[cfg(feature = "postgres-oracle")]
    assert_eq!(
        pgtk_oracle::parse_error(unrecognized).as_deref(),
        Some("unrecognized column option \"pax\""),
    );
    #[cfg(feature = "since-pg16")]
    {
        assert_postgres_accepts(
            "CREATE TABLE t (a int) PARTITION BY U&\"l!0069st\" UESCAPE '!' (a)",
        );
        let unrecognized = "CREATE TABLE t (a int) PARTITION BY U&\"l!0069\" UESCAPE '!' (a)";
        assert_declared_rejection(
            unrecognized,
            "22023",
            "unrecognized partitioning strategy",
            "U&\"l!0069\" UESCAPE '!'",
        );
        #[cfg(feature = "postgres-oracle")]
        assert_eq!(
            pgtk_oracle::parse_error(unrecognized).as_deref(),
            Some("unrecognized partitioning strategy \"li\""),
        );
        assert_postgres_accepts(
            "SELECT JSON_OBJECT('a' : 1 RETURNING bytea FORMAT JSON ENCODING U&\"utf!0038\" UESCAPE '!')",
        );
    }
}
