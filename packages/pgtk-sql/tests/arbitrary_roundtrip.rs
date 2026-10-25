//! The `Arbitrary` round trip of the AST (issue #49).
//!
//! The test samples `Statement` values with `Arbitrary` from fixed bytes, so
//! each run checks the same statements. For each sampled statement:
//!
//! 1. It renders the statement.
//! 2. It parses the rendering and renders the parsed statement again. The two
//!    renderings must be equal (a fixpoint).
//! 3. With `postgres-oracle`, the raw parser of the target version must
//!    accept the rendering.
//!
//! The test reports the yield: the attempts, the statements that it checked
//! and the count of each skip reason. A yield below `MIN_YIELD_PER_MILLE`
//! fails the test, so a change to the derives cannot make the test empty.
//!
//! `KNOWN_FAILURES` lists the failures that the test found and that have an
//! issue. The test reports the count of each one. A failure that no entry
//! matches fails the test. When you fix an issue, remove its entries.
//!
//! `PGTK_SQL_ARBITRARY_ATTEMPTS` sets the number of attempts. The default is
//! the merge gate's count. The release gate sets a larger count.
//! `PGTK_SQL_ARBITRARY_FAILURES` names a file. The test writes each unknown
//! failure to it, one for each line, so that you can sort them into issues.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;

use pgtk_sql::ast::Statement;
use pgtk_sql::formatter::format_tokens_sql;
use recursa::PrettyConfig;
use regex::Regex;

/// The attempts of the merge gate (`cargo loop`, `scripts/merge-gate`).
const DEFAULT_ATTEMPTS: usize = 100_000;

/// The bytes that one attempt gives to `Arbitrary`. Fewer bytes give
/// shallower statements, because `Arbitrary` chooses a leaf when it has no
/// bytes.
const ENTROPY_PER_ATTEMPT: usize = 256;

/// The seed of the byte generator. Change it only with the pinned yield.
const SEED: u64 = 0x5047_544b_5f53_514c;

/// The lowest yield, in statements checked for each 1,000 attempts. The
/// measured yield is 893 (pg17, 1,000,000 attempts).
const MIN_YIELD_PER_MILLE: usize = 880;

/// The stack of the thread that samples and parses. `recursa-explorer`
/// samples on a 64 MiB stack for the same grammar.
const STACK_BYTES: usize = 64 * 1024 * 1024;

/// The unknown failures that the test shows at most.
const MAX_SHOWN_FAILURES: usize = 20;

fn attempts() -> usize {
    match std::env::var("PGTK_SQL_ARBITRARY_ATTEMPTS") {
        Ok(value) => value
            .parse()
            .unwrap_or_else(|_| panic!("PGTK_SQL_ARBITRARY_ATTEMPTS is not a count: {value:?}")),
        Err(_) => DEFAULT_ATTEMPTS,
    }
}

/// The bytes of one attempt: SplitMix64 from the seed and the attempt
/// number, so each attempt reproduces alone.
fn entropy(attempt: usize) -> Vec<u8> {
    let mut state = SEED ^ (attempt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut bytes = Vec::with_capacity(ENTROPY_PER_ATTEMPT);
    while bytes.len() < ENTROPY_PER_ATTEMPT {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        bytes.extend_from_slice(&z.to_le_bytes());
    }
    bytes.truncate(ENTROPY_PER_ATTEMPT);
    bytes
}

/// Why an attempt checked no statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Skip {
    /// `Arbitrary` used all the bytes before it completed a value.
    NotEnoughData,
    /// The bytes describe no value, for example when a declared rejection
    /// refused each retry.
    IncorrectFormat,
    /// `Arbitrary` gave another error.
    OtherSampleError,
    /// The rendering is empty.
    EmptyRendering,
}

/// How a checked statement failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FailureKind {
    /// pgtk-sql cannot lex its own rendering.
    Lex,
    /// pgtk-sql cannot parse its own rendering.
    Parse,
    /// The second rendering has other tokens than the first.
    Fixpoint,
    /// The second rendering has the same tokens as the first, but other
    /// whitespace between them.
    Layout,
    /// The PostgreSQL raw parser rejects the rendering.
    Oracle,
}

use FailureKind::{Fixpoint, Layout, Lex, Oracle, Parse};

// The issue of each group of known failures.
const EREPORTS: u32 = 55;
const CREATE_FORMS: u32 = 57;
const OTHER_FORMS: u32 = 58;
const EXPRESSIONS: u32 = 59;
const ROUND_TRIP: u32 = 60;
const LAYOUT: u32 = 61;

/// One known failure. It matches a failure of its kind when `sql` matches
/// the rendering (with each run of whitespace made one space) and `detail`
/// matches the detail of the failure. For an oracle failure, the detail is
/// the PostgreSQL error message.
struct KnownFailure {
    kind: FailureKind,
    issue: u32,
    sql: &'static str,
    detail: &'static str,
    reason: &'static str,
}

const fn known(
    kind: FailureKind,
    issue: u32,
    sql: &'static str,
    detail: &'static str,
    reason: &'static str,
) -> KnownFailure {
    KnownFailure {
        kind,
        issue,
        sql,
        detail,
        reason,
    }
}

/// The known failures, in match order: the first entry that matches a
/// failure counts it.
#[rustfmt::skip]
const KNOWN_FAILURES: &[KnownFailure] = &[
    known(Parse, ROUND_TRIP, r#"TABLE ONLY\("#, r#"^unexpected "\)""#, "`TABLE ONLY (name)`"),
    known(Parse, ROUND_TRIP, r#"(GROUP BY (DISTINCT|ALL) ?|GROUPING SETS)\(\s?\)"#, r#"^unexpected "\)""#, "an empty grouping list"),
    known(Parse, ROUND_TRIP, r#""#, r#""#, "an expression without the parentheses that its place needs"),
    known(Fixpoint, ROUND_TRIP, r#""#, r#"^parentheses only"#, "an operand without the parentheses that its operator needs"),
    known(Layout, LAYOUT, r#""#, r#""#, "a line break or a space that the second rendering puts in another place"),
    known(Oracle, EREPORTS, r#""#, r#"aggregates cannot have output arguments|duplicate trigger events|unrecognized row security option|CREATE SCHEMA IF NOT EXISTS cannot include schema elements|STDIN/STDOUT not allowed with PROGRAM|UNENCRYPTED PASSWORD is no longer supported|ordered-set aggregate with a VARIADIC|OUT and INOUT arguments|column number must be in range|MATCH PARTIAL not yet implemented|unrecognized partitioning strategy|WITH TIES cannot be specified|WHERE clause not allowed with COPY TO|invalid publication object list|column specification not allowed for schema|invalid schema name|improper use of|unrecognized JSON encoding|type modifier cannot have parameter name|WITH CHECK OPTION not supported on recursive views|a column list with|WHERE clause not allowed for schema|unrecognized JSON|multiple ORDER BY clauses not allowed|frame start cannot be|frame end cannot be"#, "a statement that a gram.y action rejects with ereport()"),
    known(Oracle, EREPORTS, r#"UESCAPE"#, r#"invalid Unicode escape"#, "an invalid UESCAPE character"),
    known(Oracle, EREPORTS, r#"FOR VALUES WITH\("#, r#"(modulus|remainder) for hash partition must be specified"#, "a hash partition bound with no MODULUS or no REMAINDER"),
    known(Oracle, OTHER_FORMS, r#"^ALTER (STATISTICS|INDEX|MATERIALIZED VIEW|TABLE|VIEW|SEQUENCE|FOREIGN TABLE) "#, r#""(SCHEMA|DEPENDS|RENAME|OWNER|SET|ATTACH)""#, "an ALTER action that PostgreSQL rejects for this object or with IF EXISTS"),
    known(Oracle, CREATE_FORMS, r#"(TEMP|TEMPORARY) UNLOGGED"#, r#""UNLOGGED""#, "TEMP and UNLOGGED together"),
    known(Oracle, OTHER_FORMS, r#"^EXPLAIN(\(.*?\))? CREATE (TEMP |TEMPORARY |UNLOGGED )?TABLE "#, r#""[^)]|end of input"#, "EXPLAIN of a CREATE TABLE that is not CREATE TABLE AS"),
    known(Oracle, OTHER_FORMS, r#"^EXPLAIN(\(.*?\))? CREATE (TEMP |TEMPORARY |UNLOGGED )?TABLE (IF NOT EXISTS )?[\w.]+\(\)"#, r#""\)""#, "EXPLAIN of a CREATE TABLE with an empty column list"),
    known(Oracle, CREATE_FORMS, r#"USING [a-z_]+( OF|\()"#, r#""(OF|\(|USING)""#, "the clauses of CREATE TABLE in an order PostgreSQL rejects"),
    known(Oracle, CREATE_FORMS, r#"INDEX (CONCURRENTLY )?IF NOT EXISTS ON"#, r#""ON""#, "CREATE INDEX IF NOT EXISTS with no index name"),
    known(Oracle, OTHER_FORMS, r#"^SECURITY LABEL (FOR \S+ )?ON (OPERATOR|TRIGGER|POLICY|CONSTRAINT|RULE|CAST|COLLATION|CONVERSION|TEXT SEARCH|STATISTICS|TRANSFORM|ACCESS METHOD|EXTENSION|FOREIGN DATA|SERVER|OPERATOR CLASS|OPERATOR FAMILY|USER MAPPING|DEFAULT)"#, r#""#, "SECURITY LABEL on an object type that PostgreSQL rejects"),
    known(Oracle, OTHER_FORMS, r#"ON PROCEDURES"#, r#""PROCEDURES""#, "ALTER DEFAULT PRIVILEGES ON PROCEDURES"),
    known(Oracle, OTHER_FORMS, r#"^(EXPLAIN )?(VACUUM|CLUSTER|ANALYZE|ANALYSE)"#, r#""(FULL|FREEZE|VERBOSE|ANALYZE|\(|ON)""#, "parenthesized options mixed with the old option keywords"),
    known(Oracle, OTHER_FORMS, r#"DEFAULT VALUES"#, r#""DEFAULT""#, "DEFAULT VALUES with a column list or OVERRIDING"),
    known(Oracle, OTHER_FORMS, r#"TABLES IN SCHEMA \w+\("#, r#""\(""#, "a column list on TABLES IN SCHEMA"),
    known(Oracle, CREATE_FORMS, r#"(MODULUS|REMAINDER) ?[^0-9 ]"#, r#""#, "a hash partition bound that is not an integer"),
    known(Oracle, CREATE_FORMS, r#"PARTITION OF"#, r#""(SERVER|ON|TABLESPACE|DEFAULT)"|end of input"#, "PARTITION OF with no partition bound, or with both a bound and DEFAULT"),
    known(Oracle, CREATE_FORMS, r#"VIEW .*USING"#, r#""USING""#, "a view with an access method"),
    known(Oracle, CREATE_FORMS, r#"TABLE .*(USING \S+ PARTITION OF|PARTITION OF .* USING|OF )"#, r#""(PARTITION|USING|WITH|LIKE|OF)""#, "the clauses of CREATE TABLE in an order PostgreSQL rejects"),
    known(Oracle, CREATE_FORMS, r#"^CREATE STATISTICS IF NOT EXISTS( ON|\()"#, r#""(\(|ON)""#, "CREATE STATISTICS IF NOT EXISTS with no name"),
    known(Oracle, OTHER_FORMS, r#"^ALTER (INDEX|TABLE|FOREIGN TABLE) .* ALTER( COLUMN)? ?[-+]? ?\d"#, r#""(\(|\+|-|[A-Z]+)""#, "a signed column number, or an action that PostgreSQL rejects for a column number"),
    known(Oracle, CREATE_FORMS, r#"RECURSIVE VIEW [a-z_. ]+ AS"#, r#""AS""#, "a recursive view with no column list"),
    known(Oracle, CREATE_FORMS, r#"REFERENCES"#, r#""(ON|DELETE|UPDATE)""#, "repeated or misplaced referential actions"),
    known(Oracle, CREATE_FORMS, r#"UNIQUE NULLS (NOT )?DISTINCT USING INDEX [a-z]"#, r#""USING""#, "UNIQUE NULLS [NOT] DISTINCT with USING INDEX"),
    known(Oracle, OTHER_FORMS, r#"OIDS"#, r#""OIDS""#, "the old COPY option OIDS"),
    known(Oracle, EXPRESSIONS, r#"OVERLAPS"#, r#""OVERLAPS""#, "OVERLAPS with an operand that is not a row"),
    known(Oracle, CREATE_FORMS, r#"^CREATE SCHEMA .* CREATE (TEMP |TEMPORARY |UNLOGGED )?TABLE .* AS "#, r#""#, "a CREATE SCHEMA element of a kind that PostgreSQL rejects"),
    known(Oracle, EXPRESSIONS, r#"\.\*[^()]*\("#, r#""\(""#, "a function call on `name.*`"),
    known(Oracle, OTHER_FORMS, r#"ON CONFLICT"#, r#""(\)|WHERE)""#, "ON CONFLICT with an empty target, or WHERE with no column target"),
    known(Oracle, OTHER_FORMS, r#"^ALTER TYPE .* SET\("#, r#""(\)|,)""#, "an ALTER TYPE SET option with no value"),
    known(Oracle, CREATE_FORMS, r#"INTO (UNLOGGED |TEMP |TEMPORARY )?[\w.][\w. ]* USING"#, r#""USING""#, "SELECT INTO with an access method"),
    known(Oracle, EXPRESSIONS, r#"\]\([^)]*\) '"#, r#""\(""#, "a subscripted name as the type of a typed literal"),
    known(Oracle, EXPRESSIONS, r#"JSON_TABLE\("#, r#"only string constants are supported in JSON_TABLE path specification"#, "a JSON_TABLE path that is not a string constant"),
    known(Oracle, OTHER_FORMS, r#"^ALTER "#, r#"",""#, "a list of ALTER actions where PostgreSQL allows one"),
    known(Oracle, CREATE_FORMS, r#"RECURSIVE VIEW .* WITH"#, r#""WITH""#, "a recursive view with options"),
];

struct Failure {
    attempt: usize,
    kind: FailureKind,
    rendered: String,
    detail: String,
}

#[derive(Default)]
struct Report {
    attempts: usize,
    checked: usize,
    skips: BTreeMap<Skip, usize>,
    failures: Vec<Failure>,
}

fn classify_sample_error(message: &str) -> Skip {
    // `arbitrary::Error`'s `Display` texts.
    if message.contains("not enough") {
        Skip::NotEnoughData
    } else if message.contains("not of the correct format") {
        Skip::IncorrectFormat
    } else {
        Skip::OtherSampleError
    }
}

/// The text before `index`, at most `bytes` long, with each run of
/// whitespace made one space.
fn context(text: &str, index: usize, bytes: usize) -> String {
    let mut start = index.saturating_sub(bytes);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    one_line(&text[start..index])
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The token texts of `text`, without the whitespace between them.
fn tokens(text: &str) -> Vec<String> {
    Statement::explorer()
        .lex(text)
        .tokens()
        .map(|token| token.text().to_owned())
        .collect()
}

/// Checks one rendering: the parse, the fixpoint and the oracle.
fn check(rendered: &str) -> Result<(), (FailureKind, String)> {
    let lexed = pgtk_sql::lex(rendered);
    if let Some(error) = lexed.errors().next() {
        return Err((Lex, format!("{error:?}")));
    }
    let mut input = lexed.input();
    let parsed = Statement::parse(&mut input).map_err(|error| {
        let range = error.span().range();
        let before = context(rendered, range.start, 40);
        let token = rendered.get(range).unwrap_or("");
        (Parse, format!("unexpected {token:?} after {before:?}"))
    })?;
    if !input.is_eof() {
        return Err((Parse, "trailing input".to_owned()));
    }
    let again = format_tokens_sql(parsed.ast(), PrettyConfig::default());
    if again != rendered {
        let (first, second) = (tokens(rendered), tokens(&again));
        if first == second {
            return Err((Layout, again));
        }
        let bare = |tokens: Vec<String>| {
            tokens
                .into_iter()
                .filter(|token| token != "(" && token != ")")
                .collect::<Vec<_>>()
        };
        if bare(first) == bare(second) {
            return Err((Fixpoint, format!("parentheses only: {again}")));
        }
        return Err((Fixpoint, again));
    }
    #[cfg(feature = "postgres-oracle")]
    if let Some(error) = pgtk_oracle::parse_error(rendered) {
        return Err((Oracle, error));
    }
    Ok(())
}

fn run(attempts: usize) -> Report {
    let adapter = Statement::explorer();
    let config = PrettyConfig::default();
    let mut report = Report {
        attempts,
        ..Report::default()
    };
    for attempt in 0..attempts {
        let document = match adapter
            .sample(&entropy(attempt))
            .expect("the arbitrary feature configures sampling")
        {
            Ok(document) => document,
            Err(error) => {
                *report
                    .skips
                    .entry(classify_sample_error(error.message()))
                    .or_default() += 1;
                continue;
            }
        };
        let rendered = document
            .pretty(&config)
            .expect("Statement configures Pretty")
            .rendered;
        if rendered.trim().is_empty() {
            *report.skips.entry(Skip::EmptyRendering).or_default() += 1;
            continue;
        }
        report.checked += 1;
        if let Err((kind, detail)) = check(&rendered) {
            report.failures.push(Failure {
                attempt,
                kind,
                rendered,
                detail,
            });
        }
    }
    report
}

/// The index of the first known failure that matches `failure`.
fn known_failure(rules: &[(Regex, Regex)], failure: &Failure) -> Option<usize> {
    let sql = one_line(&failure.rendered);
    KNOWN_FAILURES
        .iter()
        .zip(rules)
        .position(|(known, (sql_rule, detail_rule))| {
            known.kind == failure.kind
                && sql_rule.is_match(&sql)
                && detail_rule.is_match(&failure.detail)
        })
}

#[test]
fn arbitrary_statements_round_trip_and_postgres_accepts_them() {
    let attempts = attempts();
    let report = std::thread::Builder::new()
        .name("arbitrary-roundtrip".to_owned())
        .stack_size(STACK_BYTES)
        .spawn(move || run(attempts))
        .expect("spawn the sampling thread")
        .join()
        .expect("the sampling thread panicked");

    let rules: Vec<(Regex, Regex)> = KNOWN_FAILURES
        .iter()
        .map(|known| {
            let compile =
                |pattern| Regex::new(pattern).unwrap_or_else(|e| panic!("{:?}: {e}", known.reason));
            (compile(known.sql), compile(known.detail))
        })
        .collect();
    let mut known_counts = vec![0usize; KNOWN_FAILURES.len()];
    let mut unknown = Vec::new();
    for failure in &report.failures {
        match known_failure(&rules, failure) {
            Some(index) => known_counts[index] += 1,
            None => unknown.push(failure),
        }
    }

    let per_mille = report.checked * 1000 / report.attempts.max(1);
    let mut summary = String::new();
    writeln!(
        summary,
        "arbitrary round trip: {} attempts, {} statements checked ({per_mille} per mille), \
         {} passed, oracle {}",
        report.attempts,
        report.checked,
        report.checked - report.failures.len(),
        if cfg!(feature = "postgres-oracle") {
            "on"
        } else {
            "off"
        },
    )
    .unwrap();
    for (skip, count) in &report.skips {
        writeln!(summary, "  skipped, {skip:?}: {count}").unwrap();
    }
    for (known, count) in KNOWN_FAILURES.iter().zip(&known_counts) {
        if *count > 0 {
            writeln!(
                summary,
                "  known failure, issue #{} ({:?}): {count}: {}",
                known.issue, known.kind, known.reason
            )
            .unwrap();
        }
    }
    writeln!(summary, "  unknown failures: {}", unknown.len()).unwrap();
    // Write past the capture of the test harness, so that each run shows the
    // yield, also a run that passes.
    std::io::stderr()
        .write_all(summary.as_bytes())
        .expect("write the summary");

    if let Ok(path) = std::env::var("PGTK_SQL_ARBITRARY_FAILURES") {
        let mut lines = String::new();
        for failure in &unknown {
            writeln!(
                lines,
                "{}\t{:?}\t{:?}\t{:?}",
                failure.attempt, failure.kind, failure.rendered, failure.detail
            )
            .unwrap();
        }
        std::fs::write(&path, lines).unwrap_or_else(|e| panic!("write {path}: {e}"));
    }

    let mut message = String::new();
    for failure in unknown.iter().take(MAX_SHOWN_FAILURES) {
        writeln!(
            message,
            "attempt {} {:?}:\n  rendered: {}\n  detail:   {}",
            failure.attempt, failure.kind, failure.rendered, failure.detail
        )
        .unwrap();
    }
    assert!(
        unknown.is_empty(),
        "{} of {} checked statements failed, and no known failure matches them:\n{message}",
        unknown.len(),
        report.checked
    );

    assert!(
        per_mille >= MIN_YIELD_PER_MILLE,
        "the yield {per_mille} per mille is below the pinned floor {MIN_YIELD_PER_MILLE}"
    );
}
