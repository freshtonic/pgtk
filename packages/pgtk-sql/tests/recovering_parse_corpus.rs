#![cfg(feature = "spans")]

//! The recovering parse against the strict parse, over the frozen corpus
//! (issue #77).
//!
//! The corpus is the one that the differential test reads
//! (`tests/support/baseline.rs`, `FrozenStatements::pinned()`): each
//! regression file of PostgreSQL 17.9, read by Git blob ID from the
//! `vendor/postgres` object database. No PostgreSQL oracle is necessary: the
//! tests compare `document::parse_sql_recovering` with `document::parse_sql`.
//!
//! 1. For each file, a document of the frozen statements that the strict
//!    parse accepts: the strict parse accepts the document, and the
//!    recovering parse gives the same items and the same statements.
//! 2. For each file as it is (a psql script, with directives, COPY payloads
//!    and statements that PostgreSQL rejects): when the strict parse accepts
//!    it, the two parses agree; when the strict parse rejects it, the first
//!    rejection of the recovering parse is the same rejection. The items of
//!    the recovering parse do not overlap and are in source order, and each
//!    item gives the same result as the strict parse of its own text.
//!
//! Run with `cargo test -p pgtk-sql --test recovering_parse_corpus`.

// Only part of the support module is used here, and rustc drops the module's
// own `#[test]` functions in this target, which strands its test imports.
#[allow(dead_code, unused_imports)]
#[path = "support/mod.rs"]
mod support;

use pgtk_sql::document::{self, RecoveredItem, SqlParseError};
use support::baseline::FrozenStatements;

/// Each frozen corpus file, as `(name, source)`.
fn corpus_files() -> Vec<(String, String)> {
    let frozen = FrozenStatements::pinned();
    frozen
        .file_names()
        .into_iter()
        .map(|name| (name.to_owned(), frozen.file(name).source()))
        .collect()
}

/// The span and the `Debug` text of the statement of each strict item.
fn strict_items(document: &document::SqlDocument<'_>) -> Vec<(recursa::Span, String)> {
    document
        .parts()
        .filter_map(|part| match part {
            pgtk_sql::CompletePart::Island(island) => Some((
                island.span(),
                format!("{:?}", island.parsed().ast().statement),
            )),
            _ => None,
        })
        .collect()
}

/// The span and the `Debug` text of the statement of each recovered item.
/// A rejection is `None`.
fn recovered_items(
    document: &document::RecoveredDocument<'_>,
) -> Vec<(recursa::Span, Option<String>)> {
    document
        .items()
        .map(|item| match item {
            RecoveredItem::Statement { span, parsed } => {
                (span, Some(format!("{:?}", parsed.ast().statement)))
            }
            RecoveredItem::Rejected(rejection) => (rejection.span(), None),
        })
        .collect()
}

fn recover<'a>(name: &str, source: &'a str) -> document::RecoveredDocument<'a> {
    document::parse_sql_recovering(source)
        .unwrap_or_else(|error| panic!("{name}: the recovering parse failed: {error}"))
}

#[test]
fn a_document_with_no_rejection_gives_the_strict_statements() {
    let frozen = FrozenStatements::pinned();
    let mut checked = 0_usize;
    for (name, text) in corpus_files() {
        let statements = frozen
            .file(&name)
            .statements(&text)
            .unwrap_or_else(|error| panic!("{name}: cannot load frozen statements: {error}"));
        // A line comment can end a statement, so each `;` goes on a new line.
        let mut source = String::new();
        for statement in statements {
            if document::parse_sql(statement).is_ok() {
                source.push_str(statement);
                source.push_str("\n;\n");
            }
        }
        let strict = document::parse_sql(&source).unwrap_or_else(|error| {
            panic!("{name}: the strict parse rejects the accepted statements: {error}")
        });
        let recovered = recover(&name, &source);
        assert!(recovered.is_accepted(), "{name}: {recovered:?}");
        let expected = strict_items(&strict)
            .into_iter()
            .map(|(span, statement)| (span, Some(statement)))
            .collect::<Vec<_>>();
        assert_eq!(recovered_items(&recovered), expected, "{name}");
        assert_eq!(
            format!("{:?}", recovered.statements()),
            format!("{:?}", strict.statements()),
            "{name}"
        );
        checked += expected.len();
    }
    assert!(checked > 40_000, "only {checked} items were compared");
}

#[test]
fn each_corpus_file_recovers_to_the_strict_result_of_each_item() {
    let mut rejections = 0_usize;
    for (name, source) in corpus_files() {
        let recovered = recover(&name, &source);
        match document::parse_sql(&source) {
            Ok(strict) => {
                assert!(recovered.is_accepted(), "{name}: {recovered:?}");
                let expected = strict_items(&strict)
                    .into_iter()
                    .map(|(span, statement)| (span, Some(statement)))
                    .collect::<Vec<_>>();
                assert_eq!(recovered_items(&recovered), expected, "{name}");
            }
            Err(SqlParseError::Rejected(strict)) => {
                let first = recovered
                    .rejections()
                    .next()
                    .unwrap_or_else(|| panic!("{name}: the strict parse rejects it"));
                assert_eq!(**first.rejection(), *strict, "{name}: the first rejection");
            }
            Err(SqlParseError::Fatal(fatal)) => panic!("{name}: {fatal}"),
        }

        let mut end = 0_usize;
        for item in recovered.items() {
            let span = item.span();
            assert!(
                span.start() as usize >= end && span.end() > span.start(),
                "{name}: item {span:?} after byte {end}"
            );
            assert!(
                source[end..span.start() as usize].trim().is_empty()
                    || pgtk_sql::document::parse_sql(&source[end..span.start() as usize])
                        .is_ok_and(|gap| gap.items().count() == 0),
                "{name}: text between items before {span:?} is not trivia"
            );
            end = span.end() as usize;
            let text = &source[span.range()];
            match item {
                RecoveredItem::Statement { parsed, .. } => {
                    let alone = document::parse_sql(text).unwrap_or_else(|error| {
                        panic!("{name}: recovered statement {text:?} alone: {error}")
                    });
                    let alone = alone.statements();
                    assert_eq!(
                        format!("{:?}", alone.first()),
                        format!("{:?}", parsed.ast().statement.as_ref()),
                        "{name}: {text:?}"
                    );
                }
                RecoveredItem::Rejected(_) => {
                    rejections += 1;
                    assert!(
                        document::parse_sql(text).is_err(),
                        "{name}: rejected text {text:?} is accepted alone"
                    );
                }
            }
        }
        assert!(
            pgtk_sql::document::parse_sql(&source[end..])
                .is_ok_and(|tail| tail.items().count() == 0),
            "{name}: text after the last item is not trivia"
        );
    }
    assert!(
        rejections > 0,
        "the corpus has psql scripts with rejections"
    );
}
