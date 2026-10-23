//! Find the oldest PostgreSQL version that accepts each statement of a set of SQL files.
//!
//! A pgtk-sql build parses the SQL of one PostgreSQL version (PostgreSQL 17 by default), but the
//! server that runs your migrations can be older. This example reads each file, and for each
//! statement it prints the oldest PostgreSQL version that accepts the statement, and the clause
//! that needs that version. When a statement needs a newer server than `--server`, it prints the
//! error that the older server gives, and the example exits with status 1.
//!
//! Two scans give the answer:
//!
//! - [`MinimumVersion`] scans the parsed statement. It finds the clauses that an older grammar
//!   does not have, for example `MERGE` (PostgreSQL 15) or a subquery in `FROM` with no alias
//!   (PostgreSQL 16).
//! - [`lexical_minimum_version`] scans the tokens. It finds the literals that an older lexer
//!   does not read, for example `0x42F` or `1_000` (PostgreSQL 16).
//!
//! ```sh
//! cargo run -p pgtk-sql --features spans --example version_check -- --server 15 migrations/*.sql
//! ```
//!
//! The output gives one line for each statement:
//!
//! ```text
//! migrations/0001.sql:1:1: PostgreSQL 14: CREATE TABLE t (id int)
//! migrations/0002.sql:1:1: PostgreSQL 16: SELECT * FROM (SELECT 1)
//!   needs PostgreSQL 16 for `(SELECT 1)` (ParenQueryRef.alias)
//!   error: PostgreSQL 15 gives 42601: subquery in FROM must have an alias at `(SELECT 1)`
//! migrations/0003.sql:1:1: PostgreSQL 16: SELECT 0x42F
//!   needs PostgreSQL 16 for `0x42F` (tokens!.lexer_tokens.IntegerLit)
//!   error: PostgreSQL 15 gives a syntax error at `0x42F`
//! ```

use std::process::ExitCode;

use pgtk_sql::minimum_version::{lexical_minimum_version, lexical_minimum_version_above};
use pgtk_sql::{MinimumVersion, TargetVersion, VersionRequirement, document};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut server = None;
    let mut paths = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--server" {
            server = args.next().as_deref().and_then(version_of);
            if server.is_none() {
                eprintln!("--server takes a major version from 14 to 19");
                return ExitCode::from(2);
            }
        } else {
            paths.push(arg);
        }
    }
    if paths.is_empty() {
        eprintln!("usage: version_check [--server <major>] <file.sql>...");
        return ExitCode::from(2);
    }
    // With no --server, check against the version that this build parses.
    let server = server.unwrap_or(pgtk_sql::TARGET_VERSION);

    let mut failed = false;
    for path in &paths {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{path}: {error}");
                failed = true;
                continue;
            }
        };
        failed |= !check_file(path, &source, server);
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Checks each statement of one file. Returns `false` when a statement needs a newer server, or
/// when the file does not parse.
fn check_file(path: &str, source: &str, server: TargetVersion) -> bool {
    let document = match document::parse_sql(source) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("{path}: {error}");
            return false;
        }
    };
    // One lexical pass over the whole file. Each statement takes the tokens inside its bounds.
    let lexed = pgtk_sql::lex(source);
    let tokens = lexed.tokens().collect::<Vec<_>>();

    let mut ok = true;
    for item in document.items() {
        if item.ast().statement.is_none() {
            continue;
        }
        let bounds = item
            .source_bounds()
            .expect("the spans feature keeps the location");
        let inside = || {
            tokens.iter().copied().filter(move |token| {
                token.span().start() >= bounds.start() && token.span().end() <= bounds.end()
            })
        };

        // The oldest version is the higher answer of the two scans.
        let minimum = later(item.minimum_version(), lexical_minimum_version(inside()));
        let version = minimum.map_or(TargetVersion::Pg14, |found| found.version());
        let (line, column) = line_and_column(source, bounds.start() as usize);
        println!(
            "{path}:{line}:{column}: PostgreSQL {}: {}",
            version.major(),
            one_line(&source[bounds.range()]),
        );
        if let Some(found) = minimum {
            println!(
                "  needs PostgreSQL {} for `{}` ({})",
                found.version().major(),
                clause(source, &found),
                found.construct(),
            );
        }

        // The error to report is the first clause, in source order, that the server rejects.
        let rejected = earlier(
            item.minimum_version_above(server),
            lexical_minimum_version_above(inside(), server),
        );
        if let Some(rejected) = rejected {
            ok = false;
            let reason = match (rejected.sqlstate(), rejected.message()) {
                (Some(code), Some(message)) => format!("{code}: {message}"),
                _ => "a syntax error".to_owned(),
            };
            println!(
                "  error: PostgreSQL {} gives {reason} at `{}`",
                server.major(),
                clause(source, &rejected),
            );
        }
    }
    ok
}

/// The answer that needs the later version.
fn later(
    a: Option<VersionRequirement>,
    b: Option<VersionRequirement>,
) -> Option<VersionRequirement> {
    match (a, b) {
        (Some(a), Some(b)) if b.version() > a.version() => Some(b),
        (Some(a), _) => Some(a),
        (None, b) => b,
    }
}

/// The answer that comes first in the source.
fn earlier(
    a: Option<VersionRequirement>,
    b: Option<VersionRequirement>,
) -> Option<VersionRequirement> {
    let start = |found: &VersionRequirement| found.span().map_or(u32::MAX, |span| span.start());
    match (a, b) {
        (Some(a), Some(b)) if start(&b) < start(&a) => Some(b),
        (Some(a), _) => Some(a),
        (None, b) => b,
    }
}

/// The source text of the clause that a requirement names.
///
/// A requirement that comes from a missing value (an alias that is not there) has the span of
/// the clause around the missing value.
fn clause<'s>(source: &'s str, found: &VersionRequirement) -> &'s str {
    found
        .span()
        .map_or("?", |span| one_line(&source[span.range()]))
}

/// The PostgreSQL version of a major version number.
fn version_of(major: &str) -> Option<TargetVersion> {
    Some(match major {
        "14" => TargetVersion::Pg14,
        "15" => TargetVersion::Pg15,
        "16" => TargetVersion::Pg16,
        "17" => TargetVersion::Pg17,
        "18" => TargetVersion::Pg18,
        "19" => TargetVersion::Pg19Beta,
        _ => return None,
    })
}

/// The first line of a text, with no trailing `;` or white space.
fn one_line(text: &str) -> &str {
    text.trim()
        .lines()
        .next()
        .unwrap_or_default()
        .trim_end_matches(';')
}

/// The 1-based line and column (in characters) of a byte offset.
fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = before[line_start..].chars().count() + 1;
    (line, column)
}
