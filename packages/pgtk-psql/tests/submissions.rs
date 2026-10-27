//! The frozen submissions of the PostgreSQL 17.9 regression corpus (issue #10).
//!
//! The PostgreSQL-oracle differential of pgtk-sql gives the raw parser only
//! what psql sends to the server: each query buffer of each regression
//! script, as [`pgtk_psql::Rendered::submissions`] gives it. pgtk-sql cannot
//! depend on pgtk-psql, so the submissions are frozen in
//! `packages/pgtk-sql/baselines/postgresql-17.9-submissions.json`. This test
//! derives them again from the frozen corpus (read by Git blob ID) and
//! requires the same bytes. A change of pgtk-psql that moves a submission thus
//! fails here, and the artifact and the version baselines are regenerated
//! and reviewed together (`packages/pgtk-sql/docs/differential-baseline.md`).
//!
//! The corpus scripts are the scripts of PostgreSQL 17.9, and pg_regress runs
//! them with the psql of the same release, so the derivation is that of the
//! `pg17` build. pg_regress gives psql two variables
//! (`src/test/regress/pg_regress_main.c`):
//! `-v HIDE_TABLEAM=on -v HIDE_TOAST_COMPRESSION=on`.
//!
//! ```sh
//! cargo test -p pgtk-psql --test submissions -- --ignored regenerate_frozen_submissions
//! ```
#![cfg(feature = "pg17")]

use std::collections::BTreeSet;
use std::ops::Range;

use pgtk_psql::{MetaCommand, PsqlItem, Variables};

/// The legacy statement spans of pgtk-sql name the corpus files and their
/// blobs.
const LEGACY_STATEMENTS: &str =
    include_str!("../../pgtk-sql/baselines/postgresql-17.9-statements.json");
/// The frozen submissions, relative to this crate.
const ARTIFACT: &str = "../pgtk-sql/baselines/postgresql-17.9-submissions.json";
/// The PostgreSQL 17.9 commit of the corpus.
const POSTGRES_GITLINK: &str = "6d396980fc5aed4f1a525e0bd75cb16b25ed40ca";

/// The variables that pg_regress gives psql.
fn regression_variables() -> Variables {
    [("HIDE_TABLEAM", "on"), ("HIDE_TOAST_COMPRESSION", "on")]
        .into_iter()
        .collect()
}

/// One corpus file, with its legacy statement spans.
struct CorpusFile {
    name: String,
    blob: String,
    bytes: usize,
    legacy_ranges: Vec<Range<usize>>,
    legacy_kinds: Vec<u8>,
}

fn parse_range(encoded: &str) -> Range<usize> {
    let (start, end) = encoded.split_once(':').expect("encoded byte range");
    start.parse().expect("range start")..end.parse().expect("range end")
}

fn corpus() -> Vec<CorpusFile> {
    let document: serde_json::Value =
        serde_json::from_str(LEGACY_STATEMENTS).expect("legacy statement spans");
    assert_eq!(
        document["postgres"]["gitlink"].as_str(),
        Some(POSTGRES_GITLINK)
    );
    document["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|file| {
            let ranges = file["byte_ranges"].as_str().expect("byte ranges");
            CorpusFile {
                name: file["file"].as_str().expect("file").to_owned(),
                blob: file["source_git_blob"].as_str().expect("blob").to_owned(),
                bytes: file["source_bytes"].as_u64().expect("bytes") as usize,
                legacy_ranges: if ranges.is_empty() {
                    Vec::new()
                } else {
                    ranges.split(',').map(parse_range).collect()
                },
                legacy_kinds: file["legacy_item_kinds"]
                    .as_str()
                    .expect("kinds")
                    .bytes()
                    .collect(),
            }
        })
        .collect()
}

impl CorpusFile {
    /// The frozen source, read by its Git blob ID from `vendor/postgres`.
    fn source(&self) -> String {
        let repository = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vendor/postgres");
        let output = std::process::Command::new("git")
            .args(["-C", repository, "cat-file", "blob", &self.blob])
            .output()
            .unwrap_or_else(|error| panic!("cannot run git cat-file: {error}"));
        assert!(
            output.status.success(),
            "cannot read the frozen corpus blob {} of {}: {}",
            self.blob,
            self.name,
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).expect("UTF-8 corpus file");
        assert_eq!(text.len(), self.bytes, "{}: blob size", self.name);
        text
    }
}

/// The submissions of one file, as the artifact records them.
struct Derived {
    ranges: Vec<Range<usize>>,
    kinds: String,
    /// `(submission index, source extent, rendered text)` of each rewrite
    /// inside a submission.
    rewrites: Vec<(usize, Range<usize>, String)>,
    texts: Vec<String>,
}

fn span_of(source: &str, part: &str) -> Range<usize> {
    let start = part.as_ptr() as usize - source.as_ptr() as usize;
    start..start + part.len()
}

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// Whether `text` holds no SQL token but `;`: psql sends it, and the
/// server has nothing to parse.
fn is_empty_query(text: &str) -> bool {
    let lexed = pgtk_sql::lex(text);
    lexed.errors().next().is_none()
        && lexed
            .tokens()
            .all(|token| token.kind() == pgtk_sql::TokenKind::SEMI)
}

/// Whether a substituted value holds the text of an interpolation, `:name`.
///
/// psql copies an interpolation of a variable that is not set into the value
/// of a `\set` (`\set filename :abs_srcdir '/data/x'`), so a variable can hold
/// the unresolved name of another one. Such a value comes from the execution
/// (`\getenv`, `\gset`, `LASTOID`) as much as the unbound variable does.
fn holds_an_interpolation(text: &str) -> bool {
    text.as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b':' && (pair[1].is_ascii_alphabetic() || pair[1] == b'_'))
}

fn derive(file: &CorpusFile, source: &str) -> Derived {
    let rendered = pgtk_psql::render(source, &regression_variables())
        .unwrap_or_else(|error| panic!("{}: not a psql document: {error}", file.name));
    let document = pgtk_psql::parse(source)
        .unwrap_or_else(|error| panic!("{}: not a psql document: {error}", file.name));
    // psql reports an unknown command and sends none of it, but pgtk-psql
    // keeps it in the rendered text (ADR 0013).
    let unknown = document
        .items
        .iter()
        .filter_map(|item| match item {
            PsqlItem::Command(command @ MetaCommand::Unknown(_)) => {
                Some(span_of(source, command.text()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let sql = rendered.sql();
    let mut derived = Derived {
        ranges: Vec::new(),
        kinds: String::new(),
        rewrites: Vec::new(),
        texts: Vec::new(),
    };
    for (index, submission) in rendered.submissions().iter().enumerate() {
        let extent = submission.rendered.clone();
        let text = &sql[extent.clone()];
        // The rewrites inside the submission. An empty rewrite at either
        // end is outside its source extent.
        let mut rebuilt = String::new();
        let mut cursor = submission.source.start;
        for (place, origin) in rendered.map().regions() {
            let inside = place.start >= extent.start
                && place.end <= extent.end
                && !(place.is_empty() && (place.start == extent.start || place.end == extent.end));
            if !inside {
                continue;
            }
            rebuilt.push_str(&source[cursor..origin.start]);
            rebuilt.push_str(&sql[place.clone()]);
            cursor = origin.end;
            derived
                .rewrites
                .push((index, origin, sql[place].to_owned()));
        }
        rebuilt.push_str(&source[cursor..submission.source.end]);
        assert_eq!(
            rebuilt, text,
            "{}: submission {index} is not its source with its rewrites",
            file.name
        );

        let first_rewrite = derived
            .rewrites
            .partition_point(|(rewrite, _, _)| *rewrite < index);
        let kind = if unknown
            .iter()
            .any(|command| overlaps(command, &submission.source))
        {
            'C'
        } else if rendered
            .unbound()
            .iter()
            .any(|variable| overlaps(&variable.source, &submission.source))
            || derived.rewrites[first_rewrite..]
                .iter()
                .any(|(_, _, text)| holds_an_interpolation(text))
        {
            'U'
        } else if is_empty_query(text) {
            'E'
        } else {
            'S'
        };
        derived.ranges.push(submission.source.clone());
        derived.kinds.push(kind);
        derived.texts.push(text.to_owned());
    }
    derived
}

/// The artifact text for the whole corpus.
fn render_artifact() -> String {
    let files = corpus();
    let mut lines = Vec::new();
    let mut totals = std::collections::BTreeMap::<char, usize>::new();
    let mut total = 0;
    for file in &files {
        let source = file.source();
        let derived = derive(file, &source);
        for kind in derived.kinds.chars() {
            *totals.entry(kind).or_default() += 1;
        }
        total += derived.ranges.len();
        let ranges = derived
            .ranges
            .iter()
            .map(|range| format!("{}:{}", range.start, range.end))
            .collect::<Vec<_>>()
            .join(",");
        let rewrites = derived
            .rewrites
            .iter()
            .map(|(index, origin, text)| {
                serde_json::json!([index, format!("{}:{}", origin.start, origin.end), text])
            })
            .collect::<Vec<_>>();
        // The keys in a fixed order, the name first.
        let value = |value: serde_json::Value| value.to_string();
        lines.push(format!(
            "{{\"file\":{},\"source_git_blob\":{},\"source_bytes\":{},\
             \"submission_count\":{},\"kinds\":{},\"byte_ranges\":{},\"rewrites\":{}}}",
            value(serde_json::json!(file.name)),
            value(serde_json::json!(file.blob)),
            file.bytes,
            derived.ranges.len(),
            value(serde_json::json!(derived.kinds)),
            value(serde_json::json!(ranges)),
            value(serde_json::json!(rewrites)),
        ));
    }
    let line = |value: serde_json::Value| serde_json::to_string(&value).expect("render JSON");
    format!(
        "{{\n  \"schema_version\": 1,\n  \"name\": \"postgresql-17.9\",\n  \
         \"postgres\": {},\n  \"corpus_root\": \"vendor/postgres/src/test/regress/sql\",\n  \
         \"derivation\": {},\n  \"encoding\": \"comma-separated-byte-ranges-v1\",\n  \
         \"kinds\": {},\n  \"totals\": {},\n  \"files\": [\n    {}\n  ]\n}}\n",
        line(serde_json::json!({"release": "17.9", "gitlink": POSTGRES_GITLINK})),
        line(serde_json::json!({
            "method": "pgtk_psql::Rendered::submissions",
            "psql": "pg17",
            "variables": {"HIDE_TABLEAM": "on", "HIDE_TOAST_COMPRESSION": "on"},
            "test": "packages/pgtk-psql/tests/submissions.rs",
        })),
        line(serde_json::json!({
            "S": "server SQL: the differential checks it",
            "E": "an empty query buffer: no SQL token but `;`. At a `;` psql sends an empty query, and at a send command it sends the previous query again",
            "U": "an interpolation of a variable that is not set, also through the value of another variable: the value comes from the execution (\\gset, \\getenv, LASTOID)",
            "C": "an unknown meta-command: psql sends none of it, and pgtk-psql keeps it in the text",
        })),
        line(serde_json::json!({
            "files": files.len(),
            "submissions": total,
            "S": totals.get(&'S').copied().unwrap_or(0),
            "E": totals.get(&'E').copied().unwrap_or(0),
            "U": totals.get(&'U').copied().unwrap_or(0),
            "C": totals.get(&'C').copied().unwrap_or(0),
        })),
        lines.join(",\n    "),
    )
}

fn artifact_path() -> String {
    format!("{}/{ARTIFACT}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_frozen_submissions_are_what_psql_sends() {
    let expected = render_artifact();
    let actual = std::fs::read_to_string(artifact_path())
        .unwrap_or_else(|error| panic!("cannot read {ARTIFACT}: {error}"));
    if actual != expected {
        let actual = actual.lines().collect::<BTreeSet<_>>();
        let changed = expected
            .lines()
            .filter(|line| !actual.contains(line))
            .map(|line| line.chars().take(120).collect::<String>())
            .collect::<Vec<_>>();
        panic!(
            "the psql submissions of the corpus changed; regenerate {ARTIFACT} and the \
             version baselines, and review the change \
             (packages/pgtk-sql/docs/differential-baseline.md):\n{}",
            changed.join("\n")
        );
    }
}

#[test]
#[ignore = "writes packages/pgtk-sql/baselines/postgresql-17.9-submissions.json"]
fn regenerate_frozen_submissions() {
    let path = artifact_path();
    std::fs::write(&path, render_artifact())
        .unwrap_or_else(|error| panic!("cannot write {path}: {error}"));
    eprintln!("wrote {path}");
}

/// The end of the last SQL token of a legacy statement range: the legacy
/// split kept the trivia after the terminator in the range.
fn legacy_core(source: &str, range: &Range<usize>) -> Range<usize> {
    let text = &source[range.clone()];
    let start = range.start + (text.len() - text.trim_start().len());
    let lexed = pgtk_sql::lex(text);
    let end = match lexed.tokens().last() {
        Some(token) if lexed.errors().next().is_none() => range.start + token.span().range().end,
        _ => range.start + text.trim_end().len(),
    };
    start..end.max(start)
}

/// The membership change from the legacy statement ranges to the psql
/// submissions, file by file. A diagnostic for the baseline review, not a
/// gate.
///
/// ```sh
/// cargo test -p pgtk-psql --test submissions -- --ignored --nocapture compare_with_the_legacy_ranges
/// ```
#[test]
#[ignore = "diagnostic listing for the baseline review"]
fn compare_with_the_legacy_ranges() {
    let verbose = std::env::var_os("PGTK_SUBMISSIONS_VERBOSE").is_some();
    let mut totals = [0usize; 13];
    println!(
        "file | legacy (S/E) | submissions (S/E/U/C) | same | merged | earlier start | \
         other end | new | legacy only (S/E)"
    );
    for file in corpus() {
        let source = file.source();
        let derived = derive(&file, &source);
        let legacy = file
            .legacy_ranges
            .iter()
            .map(|range| legacy_core(&source, range))
            .collect::<Vec<_>>();
        let mut matched_legacy = vec![false; legacy.len()];
        let (mut same, mut new) = (0, 0);
        // A submission that holds more than one legacy item, one that starts
        // before its legacy item (a block comment that psql keeps), and one
        // that ends at another place (a send command and its arguments).
        let (mut merged, mut earlier_start, mut other_end) = (0, 0, 0);
        for (index, range) in derived.ranges.iter().enumerate() {
            let overlapping = legacy
                .iter()
                .enumerate()
                .filter(|(_, core)| overlaps(core, range))
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            for i in &overlapping {
                matched_legacy[*i] = true;
            }
            let text = derived.texts[index].replace('\n', " ");
            let shown = text.chars().take(100).collect::<String>();
            match overlapping.as_slice() {
                [] => {
                    new += 1;
                    if verbose {
                        println!(
                            "  NEW {} {index} [{}] {shown}",
                            file.name,
                            &derived.kinds[index..=index]
                        );
                    }
                }
                [one] if legacy[*one] == *range => same += 1,
                many => {
                    let class = match many {
                        [one] if legacy[*one].end == range.end => {
                            earlier_start += 1;
                            "EARLIER-START"
                        }
                        [_] => {
                            other_end += 1;
                            "OTHER-END"
                        }
                        _ => {
                            merged += 1;
                            "MERGED"
                        }
                    };
                    if verbose {
                        println!(
                            "  {class} {} {index} [{}] legacy {:?} {shown}",
                            file.name,
                            &derived.kinds[index..=index],
                            many
                        );
                    }
                }
            }
        }
        let mut gone = [0usize; 2];
        for (i, matched) in matched_legacy.iter().enumerate() {
            if !matched {
                let kind = usize::from(file.legacy_kinds[i] == b'E');
                gone[kind] += 1;
                if verbose {
                    let text = source[legacy[i].clone()].replace('\n', " ");
                    println!(
                        "  LEGACY-ONLY {} {i} [{}] {}",
                        file.name,
                        file.legacy_kinds[i] as char,
                        text.chars().take(100).collect::<String>()
                    );
                }
            }
        }
        let count = |kind: char| derived.kinds.chars().filter(|k| *k == kind).count();
        let legacy_s = file.legacy_kinds.iter().filter(|k| **k == b'S').count();
        let row = [
            legacy_s,
            file.legacy_kinds.len() - legacy_s,
            count('S'),
            count('E'),
            count('U'),
            count('C'),
            same,
            merged,
            earlier_start,
            other_end,
            new,
            gone[0],
            gone[1],
        ];
        for (total, value) in totals.iter_mut().zip(row) {
            *total += value;
        }
        if row[7..].iter().any(|count| *count != 0) || count('S') != derived.ranges.len() {
            println!(
                "{} | {}/{} | {}/{}/{}/{} | {same} | {merged} | {earlier_start} | {other_end} \
                 | {new} | {}/{}",
                file.name, row[0], row[1], row[2], row[3], row[4], row[5], gone[0], gone[1]
            );
        }
    }
    println!("totals {totals:?}");
}
