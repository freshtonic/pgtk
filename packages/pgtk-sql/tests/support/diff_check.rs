// The oracle comparison needs PostgreSQL itself; the lexing and formatting
// helpers do not. Gating only the oracle half lets an oracle-free suite — the
// nested/flat parity suite — mount this module for `lex_statement_source`
// without pulling in the `pgtk-oracle` FFI bridge.
#[cfg(feature = "postgres-oracle")]
use crate::support::Stmt;
#[cfg(feature = "postgres-oracle")]
use pgtk_oracle::{Equal, parse_equal, parse_ok};
use std::fmt;
use std::ops::Range;

#[cfg(feature = "postgres-oracle")]
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail(String), // human-readable reason
    Skip(String), // grammar gap reason
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrictDiagnostic {
    pub code: String,
    pub region: Range<usize>,
    pub anchor: Range<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StrictStatementFailure {
    diagnostic: Option<StrictDiagnostic>,
    summary: String,
}

impl StrictStatementFailure {
    pub fn diagnostic(&self) -> Option<&StrictDiagnostic> {
        self.diagnostic.as_ref()
    }
}

impl fmt::Display for StrictStatementFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.summary)
    }
}

/// Lex one frozen legacy statement slice while excluding its document-level
/// semicolon from the significant tokens presented to `Statement`.
///
/// The legacy extractor deliberately leaves trivia after the terminator in the
/// statement's source range. Rebuilding the token stream keeps that complete
/// source and every retained token span unchanged; only the final significant
/// `SEMI` record is omitted. Lexically invalid input is returned verbatim so
/// its mechanism-specific diagnostic and anchor are not weakened by rebuilding.
///
/// `pub(crate)` so the interim benchmark harness (`benches/parse.rs`, which
/// mounts this module) times pgtk-sql through the same statement lexing seam.
pub(crate) fn lex_statement_source(source: &str) -> pgtk_sql::LexResult<'_> {
    let lexed = pgtk_sql::lex(source);
    if lexed.errors().next().is_some() {
        return lexed;
    }

    let terminator = lexed
        .tokens()
        .last()
        .filter(|token| token.kind() == pgtk_sql::TokenKind::SEMI)
        .map(|token| token.span());
    let Some(terminator) = terminator else {
        return lexed;
    };

    let mut statement = pgtk_sql::LexBuilder::new(source);
    for token in lexed.tokens() {
        if token.span() != terminator {
            statement
                .append(token.kind(), token.span())
                .expect("tokens copied from pgtk-sql lexing retain valid ordered spans");
        }
    }
    statement.finish()
}

/// Strictly parse one extracted statement with pgtk-sql and return its formatted
/// SQL or a stable diagnostic summary for the grammar gap.
pub(crate) fn pgsql_format(source: &str) -> Result<String, StrictStatementFailure> {
    let lexed = lex_statement_source(source);
    if let Some(error) = lexed.errors().next() {
        let diagnostic = StrictDiagnostic {
            code: error.code().to_owned(),
            region: error.span().range(),
            anchor: error.anchor().range(),
        };
        return Err(StrictStatementFailure {
            summary: format!(
                "pgtk-sql lexical failure {} at {:?}, anchor {:?}",
                diagnostic.code, diagnostic.region, diagnostic.anchor
            ),
            diagnostic: Some(diagnostic),
        });
    }
    let (formatted, cursor) = format_one_statement(&lexed)?;
    if let Some(cursor) = cursor {
        let trailing = lexed
            .tokens()
            .nth(cursor)
            .expect("a non-EOF public cursor addresses a significant token");
        return Err(StrictStatementFailure {
            diagnostic: None,
            summary: format!(
                "pgtk-sql parse left trailing input {} at {:?}, token cursor {cursor}",
                trailing.kind(),
                trailing.span().range()
            ),
        });
    }
    Ok(formatted)
}

/// Parse one `Statement` from the start of `lexed` and format it. The cursor
/// is `None` at the end of the input, or the index of the next significant
/// token.
fn format_one_statement(
    lexed: &pgtk_sql::LexResult<'_>,
) -> Result<(String, Option<usize>), StrictStatementFailure> {
    use pgtk_sql::ast::Statement;

    let mut input = lexed.input();
    let parsed = Statement::parse(&mut input).map_err(|error| {
        let expected = error.expected().collect::<Vec<_>>().join(", ");
        let diagnostic = StrictDiagnostic {
            code: error.code().to_owned(),
            region: error.span().range(),
            anchor: error.anchor().range(),
        };
        StrictStatementFailure {
            summary: format!(
                "pgtk-sql parse failure {} ({:?}) at {:?}, anchor {:?}, found {:?}, expected [{}]",
                diagnostic.code,
                error.kind(),
                diagnostic.region,
                diagnostic.anchor,
                error.found(),
                expected
            ),
            diagnostic: Some(diagnostic),
        }
    })?;
    let cursor = (!input.is_eof()).then(|| input.cursor());
    let ast = parsed.ast();
    Ok((
        pgtk_sql::formatter::format_tokens_sql(ast, recursa::PrettyConfig::default()),
        cursor,
    ))
}

/// Strictly parse one psql submission with pgtk-sql and return its formatted
/// SQL, or a stable diagnostic summary for the grammar gap.
///
/// psql sends the whole query buffer as one query string, and the string can
/// hold more than one statement (`\;`, or a variable whose value holds a
/// `;`). A `;` inside a statement, as in a `BEGIN ATOMIC` body, is not a
/// split. So each statement is the shortest run of tokens up to a `;` (or
/// to the end) that parses as one whole `Statement`. A submission with no `;`
/// before its end parses exactly as [`pgsql_format`] parses it, with the same
/// diagnostic. The formatted statements are joined with `;`, which the raw
/// parser reads as the same statement list.
// Only the differential uses the submission check; each test target mounts
// this module.
#[cfg(feature = "postgres-oracle")]
#[allow(dead_code)]
pub(crate) fn pgsql_format_submission(source: &str) -> Result<String, StrictStatementFailure> {
    let lexed = lex_statement_source(source);
    if lexed.errors().next().is_some() {
        return pgsql_format(source);
    }
    let tokens = lexed
        .tokens()
        .map(|token| (token.kind(), token.span()))
        .collect::<Vec<_>>();
    let part = |range: std::ops::Range<usize>| {
        let mut part = pgtk_sql::LexBuilder::new(source);
        for (kind, span) in &tokens[range] {
            part.append(*kind, *span)
                .expect("tokens copied from pgtk-sql lexing retain valid ordered spans");
        }
        part.finish()
    };
    let mut formatted = Vec::new();
    let mut from = 0;
    while from < tokens.len() {
        // The places where the statement can end: each `;` and the end.
        let ends = (from..tokens.len())
            .filter(|&end| tokens[end].0 == pgtk_sql::TokenKind::SEMI)
            .chain([tokens.len()]);
        let mut statement = None;
        for end in ends {
            let lexed = part(from..end);
            let result = format_one_statement(&lexed);
            match result {
                Ok((text, None)) => {
                    statement = Some((text, end));
                    break;
                }
                // The last place: the diagnostic of the whole remainder.
                Ok((_, Some(cursor))) if end == tokens.len() => {
                    let (kind, span) = tokens[from + cursor];
                    return Err(StrictStatementFailure {
                        diagnostic: None,
                        summary: format!(
                            "pgtk-sql parse left trailing input {kind} at {:?}, token cursor {}",
                            span.range(),
                            from + cursor
                        ),
                    });
                }
                Err(failure) if end == tokens.len() => return Err(failure),
                _ => {}
            }
        }
        let (text, end) = statement.expect("the last place returns or ends the statement");
        formatted.push(text);
        from = end + 1;
    }
    Ok(formatted.join(";\n"))
}

/// The first word of the failure of a statement that the oracle rejects and
/// pgtk-sql accepts, when the oracle also rejects pgtk-sql's rendering.
#[cfg(feature = "postgres-oracle")]
pub const OVER_ACCEPTANCE: &str = "over-acceptance";

#[cfg(feature = "postgres-oracle")]
pub fn check_statement(stmt: &Stmt) -> Outcome {
    check_with(&stmt.source, pgsql_format)
}

/// The differential check of one psql submission: [`check_statement`], with
/// each statement of the query string parsed in turn
/// ([`pgsql_format_submission`]).
#[cfg(feature = "postgres-oracle")]
#[allow(dead_code)]
pub fn check_submission(source: &str) -> Outcome {
    check_with(source, pgsql_format_submission)
}

#[cfg(feature = "postgres-oracle")]
fn check_with(
    src: &str,
    pgsql_format: fn(&str) -> Result<String, StrictStatementFailure>,
) -> Outcome {
    if parse_ok(src) {
        // PostgreSQL accepts the input.
        let formatted = match pgsql_format(src) {
            Ok(formatted) => formatted,
            Err(diagnostic) => return Outcome::Skip(diagnostic.to_string()),
        };
        // (1) pgtk-sql must re-parse its own output.
        if let Err(diagnostic) = pgsql_format(&formatted) {
            return Outcome::Fail(format!(
                "pgtk-sql cannot re-parse its own output: {diagnostic}"
            ));
        }
        // (2) PostgreSQL must see the trees as identical.
        match parse_equal(src, &formatted) {
            Equal::Equal => Outcome::Pass,
            Equal::Differ => Outcome::Fail(format!(
                "reformat changed the parse tree\n  in:  {src}\n  out: {formatted}"
            )),
            Equal::ErrorRight => {
                Outcome::Fail(format!("PostgreSQL rejects pgtk-sql's output: {formatted}"))
            }
            Equal::ErrorLeft => unreachable!("parse_ok said the input is valid"),
        }
    } else {
        // PostgreSQL rejects the input. Version parity says that pgtk-sql
        // must reject it too, so an accept is a failure, whatever the
        // rendering is (#95). A rendering that PostgreSQL accepts is the
        // more specific failure: pgtk-sql changed the meaning of the input.
        let Ok(formatted) = pgsql_format(src) else {
            return Outcome::Pass;
        };
        if parse_ok(&formatted) {
            return Outcome::Fail(format!(
                "over-permissive: pgtk-sql turned PG-rejected SQL into \
                 PG-accepted SQL\n  in:  {src}\n  out: {formatted}"
            ));
        }
        Outcome::Fail(format!(
            "{OVER_ACCEPTANCE}: PostgreSQL rejects the statement and pgtk-sql \
             accepts it\n  in:  {src}\n  out: {formatted}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "postgres-oracle")]
    fn check(sql: &str) -> Outcome {
        super::check_statement(&Stmt {
            source: sql.to_string(),
        })
    }

    #[cfg(feature = "postgres-oracle")]
    #[test]
    fn faithful_statement_passes() {
        assert_eq!(check("SELECT 1 AS one"), Outcome::Pass);
    }

    // Added in 15: trailing junk after a numeric literal is a lexical error
    // (docs/research/postgres-14-19-sql-syntax-changes.md, PostgreSQL 15,
    // "Lexical and literal syntax"; commit 2549f0661).
    #[cfg(all(feature = "postgres-oracle", feature = "since-pg15"))]
    #[test]
    fn pg_rejected_input_passes_when_pgsql_also_rejects() {
        // PostgreSQL rejects trailing junk; pgtk-sql must not "fix" it into
        // valid SQL.
        assert_eq!(check("SELECT 123abc"), Outcome::Pass);
    }

    // PostgreSQL rejects the statement and its rendering, and pgtk-sql
    // accepts it: an over-acceptance (#95). `current_schema` is no `ColId`,
    // and the column names of a table alias in pgtk-sql still admit it.
    // Replace the example when that is fixed.
    #[cfg(feature = "postgres-oracle")]
    #[test]
    fn pg_rejected_input_fails_when_pgsql_accepts() {
        match check("SELECT * FROM t AS x (current_schema)") {
            Outcome::Fail(reason) => assert!(reason.starts_with(OVER_ACCEPTANCE), "{reason}"),
            other => panic!("expected an over-acceptance, got {other:?}"),
        }
    }

    // Before 15, PostgreSQL reads `123abc` as `123 AS abc` (the same research
    // entry), and so does pgtk-sql.
    #[cfg(all(feature = "postgres-oracle", not(feature = "since-pg15")))]
    #[test]
    fn numeric_junk_is_a_column_label_before_15() {
        assert_eq!(check("SELECT 123abc"), Outcome::Pass);
        assert_eq!(
            pgsql_format("SELECT 123abc").as_deref(),
            Ok("SELECT 123 abc")
        );
    }

    #[test]
    fn invalid_statement_reports_stable_parse_diagnostics() {
        // The former fixture here (`DELETE ... USING t1 JOIN t2 USING (a)`)
        // resolved when suffix-proving optional viability landed; an invalid
        // statement now carries the stable-diagnostic contract instead.
        let failure = pgsql_format("DELETE FROM t3 USING t1 WHERE;")
            .expect_err("an empty WHERE expression must fail");
        assert_eq!(
            failure.diagnostic(),
            Some(&StrictDiagnostic {
                code: "RCA4101".into(),
                region: 30..30,
                anchor: 30..30,
            })
        );
    }

    #[test]
    fn grammar_gap_reports_stable_lexical_diagnostics() {
        let diagnostic =
            pgsql_format("SELECT /* unterminated").expect_err("unterminated comment must fail");
        assert_eq!(
            diagnostic.to_string(),
            "pgtk-sql lexical failure RCA4002 at 7..22, anchor 7..9"
        );

        let after_terminator = pgsql_format("SELECT 1; /* unterminated")
            .expect_err("a post-terminator lexical failure must not be discarded");
        assert_eq!(
            after_terminator.to_string(),
            "pgtk-sql lexical failure RCA4002 at 10..25, anchor 10..12"
        );
    }

    #[test]
    fn final_terminator_before_owned_line_comment_is_document_framing() {
        pgsql_format("SELECT 1;\n\n-- advisory_lock cleanup")
            .expect("the legacy slice's final semicolon must not enter Statement parsing");
    }

    #[test]
    fn final_terminator_before_owned_block_comment_is_document_framing() {
        pgsql_format("SELECT 1 <> 2; /* legacy-owned ; block comment */")
            .expect("operators and comment text must not obscure the final SQL terminator");
    }

    #[test]
    fn semicolons_inside_dollar_strings_are_not_statement_terminators() {
        pgsql_format("SELECT $$body; -- still body$$;\n-- legacy-owned comment")
            .expect("only the significant semicolon after the dollar string is framing");
        pgsql_format("SELECT $$body; -- still body$$")
            .expect("an interior semicolon must remain part of its dollar string");
    }

    #[test]
    fn terminator_exclusion_preserves_owned_source_and_token_spans() {
        let source = " \nSELECT 1;\n\n-- advisory_lock cleanup";
        let lexed = lex_statement_source(source);
        let tokens = lexed.tokens().collect::<Vec<_>>();

        assert_eq!(lexed.source(), source);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text(), "SELECT");
        assert_eq!(tokens[0].span().range(), 2..8);
        assert_eq!(tokens[1].text(), "1");
        assert_eq!(tokens[1].span().range(), 9..10);
    }

    #[test]
    fn nonfinal_statement_separator_remains_significant() {
        let lexed = lex_statement_source("SELECT 1; SELECT 2;");
        let semicolons = lexed
            .tokens()
            .filter(|token| token.kind() == pgtk_sql::TokenKind::SEMI)
            .collect::<Vec<_>>();

        assert_eq!(semicolons.len(), 1);
        assert_eq!(semicolons[0].span().range(), 8..9);
    }

    // The frozen baseline contains 18 PostgreSQL-accepted statements that the
    // legacy grammar skipped. The corpus driver accounts for those skips per
    // file and rejects every new skip.
}
