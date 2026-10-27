//! Record the oracle outcomes of one target version over the frozen psql
//! submissions of the regression corpus, with no pgtk-sql build of that
//! version (issue #10).
//!
//! A version baseline (`pgtk-sql/baselines/target-versions/<feature>.json`)
//! holds two parts: the oracle outcome of each submission, which only the
//! oracle gives, and the expected version gaps, which need a pgtk-sql build of
//! the version. pgtk-sql's `regenerate_version_baseline` writes both. This
//! test writes the first part, and keeps each recorded gap whose submission
//! and oracle outcome did not change. The differential test of the version
//! then checks the gaps (`pgtk-sql/docs/differential-baseline.md`).
//!
//! ```sh
//! cargo test -p pgtk-oracle --no-default-features --features pg16,parser \
//!   --test corpus_outcomes -- --ignored --nocapture
//! ```
//!
//! `PGTK_SEED_GAPS_FROM=pg17` also adds each skip gap of that version whose
//! submission this oracle accepts, when no gap of this version names it.

#![cfg(feature = "parser")]

#[allow(dead_code)]
#[path = "../../pgtk-sql/tests/support/submissions.rs"]
mod submissions;

use std::collections::{BTreeMap, BTreeSet};

use submissions::{
    parse_range, render_version_baseline, FrozenSubmissions, GapOutcome, SubmissionKind, VersionGap,
};

/// The version feature of this build.
fn feature() -> &'static str {
    if cfg!(feature = "pg19-beta") {
        "pg19-beta"
    } else if cfg!(feature = "pg18") {
        "pg18"
    } else if cfg!(feature = "pg17") {
        "pg17"
    } else if cfg!(feature = "pg16") {
        "pg16"
    } else if cfg!(feature = "pg15") {
        "pg15"
    } else {
        "pg14"
    }
}

fn baseline_path(feature: &str) -> String {
    format!(
        "{}/../pgtk-sql/baselines/target-versions/{feature}.json",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// The gaps of a version baseline file in the submission schema, read
/// without the checks of `VersionBaseline::load`. A file in an older schema
/// has none.
fn recorded_gaps(feature: &str) -> Vec<VersionGap> {
    let text = std::fs::read_to_string(baseline_path(feature)).unwrap_or_default();
    let Ok(document) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    if document["schema_version"].as_u64() != Some(2) {
        return Vec::new();
    }
    document["expected_version_gaps"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|gap| VersionGap {
            file: gap["file"].as_str().expect("gap file").to_owned(),
            submission_index: gap["submission_index"].as_u64().expect("gap index") as usize,
            byte_range: parse_range(gap["byte_range"].as_str().expect("gap range")),
            pg_sql: match gap["pg_sql"].as_str() {
                Some("skip") => GapOutcome::Skip,
                Some("fail") => GapOutcome::Fail,
                other => panic!("invalid gap outcome {other:?}"),
            },
            oracle_accepts: gap["oracle"].as_str() == Some("accept"),
        })
        .collect()
}

#[test]
#[ignore = "writes pgtk-sql/baselines/target-versions/<feature>.json"]
fn refresh_the_oracle_outcomes_of_the_version_baseline() {
    let feature = feature();
    assert_eq!(
        submissions::oracle_pin(feature),
        (pgtk_oracle::POSTGRES_REF, pgtk_oracle::POSTGRES_COMMIT),
        "this oracle is not the pin of {feature}"
    );
    let frozen = FrozenSubmissions::pinned();
    let mut oracle_accepts = BTreeMap::new();
    for name in frozen.file_names() {
        let file = frozen.file(name);
        let source = file.source();
        let outcomes = file
            .submissions(&source)
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .iter()
            .map(|submission| {
                (submission.kind == SubmissionKind::Server)
                    .then(|| pgtk_oracle::parse_ok(&submission.text))
            })
            .collect::<Vec<_>>();
        oracle_accepts.insert(name.to_owned(), outcomes);
    }

    let valid = |gap: &VersionGap| {
        frozen
            .file_names()
            .contains(gap.file.as_str())
            .then(|| frozen.file(&gap.file).ranges().get(gap.submission_index))
            .flatten()
            == Some(&gap.byte_range)
            && oracle_accepts[&gap.file][gap.submission_index] == Some(gap.oracle_accepts)
    };
    let mut gaps = Vec::new();
    for gap in recorded_gaps(feature) {
        if valid(&gap) {
            gaps.push(gap);
        } else {
            eprintln!(
                "dropped the gap {}:{}: its submission or oracle outcome changed",
                gap.file, gap.submission_index
            );
        }
    }
    if let Ok(seed) = std::env::var("PGTK_SEED_GAPS_FROM") {
        let named = gaps
            .iter()
            .map(|gap| (gap.file.clone(), gap.submission_index))
            .collect::<BTreeSet<_>>();
        for gap in recorded_gaps(&seed) {
            if gap.pg_sql == GapOutcome::Skip
                && !named.contains(&(gap.file.clone(), gap.submission_index))
                && valid(&gap)
            {
                eprintln!(
                    "seeded the skip gap {}:{} from {seed}",
                    gap.file, gap.submission_index
                );
                gaps.push(gap);
            }
        }
    }
    gaps.sort_by(|a, b| (&a.file, a.submission_index).cmp(&(&b.file, b.submission_index)));

    let path = baseline_path(feature);
    std::fs::write(
        &path,
        render_version_baseline(feature, &oracle_accepts, &gaps),
    )
    .unwrap_or_else(|error| panic!("cannot write {path}: {error}"));
    eprintln!("wrote {path}: {} expected version gaps", gaps.len());
}
