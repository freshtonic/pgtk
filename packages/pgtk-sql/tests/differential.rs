//! Differential parser test: pgtk-sql vs the raw parser of the target
//! version's pinned PostgreSQL release, over the psql submissions of the
//! frozen PostgreSQL 17.9 regression corpus.
//!
//! The statement set is what psql sends to the server for each regression
//! script (`baselines/postgresql-17.9-submissions.json`, derived by
//! `pgtk-psql/tests/submissions.rs`, issue #10). No psql syntax, COPY data or
//! `\.` line goes to either parser.
//! See docs/plans/2026-05-21-differential-parser-testing-design.md and
//! docs/differential-baseline.md.

mod support;

use std::collections::{BTreeMap, BTreeSet};

use pgtk_oracle::parse_ok;
use support::baseline::{
    Baseline, FrozenStatements, FrozenSubmissions, GapOutcome, OutcomeTotals,
    PendingOverAcceptances, SubmissionKind, VersionBaseline, VersionGap, render_version_baseline,
};
use support::diff_check::{OVER_ACCEPTANCE, Outcome, check_submission};

fn fixture_name(name: &str) -> &str {
    name.strip_prefix("r#").unwrap_or(name)
}

fn baseline_name(name: &str) -> String {
    format!("{}.sql", fixture_name(name))
}

/// The differential result of one frozen submission with this build's
/// oracle. `None` for a submission that the differential does not check.
struct SubmissionResult {
    oracle_accepts: bool,
    outcome: Outcome,
}

impl SubmissionResult {
    /// The version gap of the submission, or `None` when pgtk-sql agrees
    /// with the oracle.
    fn gap_outcome(&self) -> Option<GapOutcome> {
        match &self.outcome {
            Outcome::Pass => None,
            Outcome::Skip(_) => Some(GapOutcome::Skip),
            Outcome::Fail(_) => Some(GapOutcome::Fail),
        }
    }
}

/// Check every frozen server submission of one corpus file against this
/// build's oracle. `name` is the baseline file name, for example `join.sql`.
fn check_corpus_file(name: &str) -> Vec<Option<SubmissionResult>> {
    let frozen = FrozenSubmissions::pinned().file(name);
    // The frozen corpus is read by Git blob ID, so its identity is exact
    // even when the oracle is a different PostgreSQL release.
    let text = frozen.source();
    let submissions = frozen
        .submissions(&text)
        .unwrap_or_else(|error| panic!("{name}: cannot load frozen submissions: {error}"));
    submissions
        .iter()
        .map(|submission| {
            (submission.kind == SubmissionKind::Server).then(|| SubmissionResult {
                oracle_accepts: parse_ok(&submission.text),
                outcome: check_submission(&submission.text),
            })
        })
        .collect()
}

fn run_corpus_file(name: &str) {
    // Fixture names that collide with Rust keywords (`async`, `box`, `enum`)
    // are written as raw identifiers in `corpus_tests!`. `stringify!` keeps
    // the `r#` prefix, but the fixture file name has none — strip it so
    // the name matches the baseline.
    let name = fixture_name(name);
    let baseline_name = baseline_name(name);
    let frozen = FrozenSubmissions::pinned().file(&baseline_name);
    let version = VersionBaseline::active();
    assert_eq!(
        (pgtk_oracle::POSTGRES_REF, pgtk_oracle::POSTGRES_COMMIT),
        (version.oracle_ref.as_str(), version.oracle_commit.as_str()),
        "the {} version baseline is not from this oracle",
        version.feature
    );
    let recorded_accepts = version.oracle_accepts(&baseline_name);
    let listed_gaps = version.gaps_in(&baseline_name);
    let pending = PendingOverAcceptances::pinned();
    let pending_here = pending.entries_in(&baseline_name);

    let results = check_corpus_file(&baseline_name);
    assert_eq!(
        results.len(),
        frozen.ranges().len(),
        "{name}: the submission count changed from the frozen submissions"
    );

    let mut actual = OutcomeTotals {
        checked: 0,
        pass: 0,
        skip: 0,
        fail: 0,
    };
    let mut excluded = BTreeMap::<&str, usize>::new();
    let mut oracle_changes = Vec::new();
    let mut skips = Vec::new();
    let mut failures = Vec::new();
    let mut unlisted_gaps = Vec::new();
    let mut stale_gaps = Vec::new();
    let mut stale_pending = Vec::new();
    let mut expected_gaps = 0;

    for (i, result) in results.iter().enumerate() {
        let Some(result) = result else {
            let reason = frozen.kinds()[i]
                .exclusion()
                .expect("only server SQL is checked");
            *excluded.entry(reason).or_default() += 1;
            continue;
        };
        actual.checked += 1;
        if Some(result.oracle_accepts) != recorded_accepts[i] {
            oracle_changes.push(format!(
                "  submission {i}: the oracle now {} it",
                if result.oracle_accepts {
                    "accepts"
                } else {
                    "rejects"
                }
            ));
        }
        // A known over-acceptance (#95) applies only where this version's
        // oracle rejects the submission.
        if let Some(entry) = pending_here.get(&i)
            && !result.oracle_accepts
        {
            assert!(
                !listed_gaps.contains_key(&i),
                "{name}: submission {i} is both a pending over-acceptance and a version gap"
            );
            match &result.outcome {
                Outcome::Fail(reason) if reason.starts_with(OVER_ACCEPTANCE) => expected_gaps += 1,
                other => {
                    stale_pending.push(format!("  submission {i} ({}): now {other:?}", entry.cause))
                }
            }
            continue;
        }
        let gap = result.gap_outcome();
        match (listed_gaps.get(&i), gap) {
            (Some(listed), Some(gap))
                if listed.pg_sql == gap && listed.oracle_accepts == result.oracle_accepts =>
            {
                // An expected version gap: the grammar work of the version
                // removes it.
                expected_gaps += 1;
                continue;
            }
            (Some(listed), _) => {
                stale_gaps.push(format!(
                    "  submission {i}: listed as pg_sql={} oracle={}, now {:?} with oracle {}",
                    listed.pg_sql.name(),
                    if listed.oracle_accepts {
                        "accept"
                    } else {
                        "reject"
                    },
                    result.outcome,
                    if result.oracle_accepts {
                        "accept"
                    } else {
                        "reject"
                    },
                ));
                continue;
            }
            (None, Some(gap)) => unlisted_gaps.push(format!(
                "  submission {i}: {} (oracle {}): {:?}",
                gap.name(),
                if result.oracle_accepts {
                    "accept"
                } else {
                    "reject"
                },
                result.outcome
            )),
            (None, None) => {}
        }
        match &result.outcome {
            Outcome::Pass => actual.pass += 1,
            Outcome::Skip(reason) => {
                actual.skip += 1;
                skips.push(format!("  submission {i}: {reason}"));
            }
            Outcome::Fail(reason) => {
                actual.fail += 1;
                failures.push(format!("  submission {i}: {reason}"));
            }
        }
    }

    eprintln!(
        "[{name}] submissions={} pass={} skip={} fail={} expected_version_gaps={} excluded={:?}",
        results.len(),
        actual.pass,
        actual.skip,
        actual.fail,
        expected_gaps,
        excluded
    );
    assert!(
        oracle_changes.is_empty(),
        "{name}: the oracle outcomes differ from the {} version baseline; \
         regenerate it (docs/differential-baseline.md):\n{}",
        version.feature,
        oracle_changes.join("\n")
    );
    assert!(
        stale_pending.is_empty(),
        "{name}: pending over-acceptances (baselines/pending-over-acceptance.json) \
         changed; remove the entry of each fixed submission \
         (docs/differential-baseline.md):\n{}",
        stale_pending.join("\n")
    );
    assert!(
        stale_gaps.is_empty() && unlisted_gaps.is_empty(),
        "{name}: the expected version gaps of {} are stale; regenerate the version \
         baseline and review the change (docs/differential-baseline.md)\n\
         listed gaps that changed:\n{}\nunlisted disagreements:\n{}",
        version.feature,
        stale_gaps.join("\n"),
        unlisted_gaps.join("\n")
    );
    assert_eq!(
        actual.fail,
        0,
        "{name}: differential failures were introduced:\n{}",
        failures.join("\n")
    );
    assert_eq!(
        actual.skip,
        0,
        "{name}: submissions that are not expected version gaps were skipped:\n{}",
        skips.join("\n")
    );
    assert_eq!(
        actual.pass + expected_gaps,
        actual.checked,
        "{name}: every checked submission passes or is an expected version gap"
    );
}

/// Regenerate the version baseline of this build's target version from its
/// oracle. It is not a normal test: run it explicitly and review the diff.
///
/// ```sh
/// cargo test -p pgtk-sql --no-default-features --features pg16,spans,postgres-oracle \
///   --test differential -- --ignored regenerate_version_baseline
/// ```
#[test]
#[ignore = "writes baselines/target-versions/<feature>.json"]
fn regenerate_version_baseline() {
    let feature = pgtk_sql::TARGET_VERSION.feature();
    let files = FrozenSubmissions::pinned()
        .file_names()
        .into_iter()
        .collect::<Vec<_>>();
    let workers = std::thread::available_parallelism().map_or(4, usize::from);
    let per_file = std::thread::scope(|scope| {
        let chunks = files.chunks(files.len().div_ceil(workers));
        let handles = chunks
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|file| (*file, check_corpus_file(file)))
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("corpus worker"))
            .collect::<Vec<_>>()
    });

    let mut oracle_accepts = BTreeMap::new();
    let mut gaps = Vec::new();
    for (file, results) in per_file {
        let ranges = FrozenSubmissions::pinned().file(file).ranges();
        let pending = PendingOverAcceptances::pinned().entries_in(file);
        for (index, result) in results.iter().enumerate() {
            let Some(result) = result else {
                continue;
            };
            if !result.oracle_accepts
                && let Some(entry) = pending.get(&index)
            {
                // A pending over-acceptance is in its own list, not a gap.
                eprintln!(
                    "pending over-acceptance {file}:{index} ({}): {:?}",
                    entry.cause, result.outcome
                );
                continue;
            }
            if let Some(pg_sql) = result.gap_outcome() {
                // The reason is not in the baseline; print it for the review.
                eprintln!(
                    "gap {file}:{index} {} oracle={}: {:?}",
                    pg_sql.name(),
                    if result.oracle_accepts {
                        "accept"
                    } else {
                        "reject"
                    },
                    result.outcome
                );
                gaps.push(VersionGap {
                    file: file.to_owned(),
                    submission_index: index,
                    byte_range: ranges[index].clone(),
                    pg_sql,
                    oracle_accepts: result.oracle_accepts,
                });
            }
        }
        oracle_accepts.insert(
            file.to_owned(),
            results
                .iter()
                .map(|result| result.as_ref().map(|result| result.oracle_accepts))
                .collect::<Vec<_>>(),
        );
    }
    gaps.sort_by(|a, b| (&a.file, a.submission_index).cmp(&(&b.file, b.submission_index)));

    let path = format!(
        "{}/baselines/target-versions/{feature}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::write(
        &path,
        render_version_baseline(feature, &oracle_accepts, &gaps),
    )
    .unwrap_or_else(|error| panic!("cannot write {path}: {error}"));
    eprintln!("wrote {path}: {} expected version gaps", gaps.len());
}

macro_rules! corpus_tests {
    ($($name:ident),* $(,)?) => {
        const CORPUS_FILES: &[&str] = &[$(stringify!($name)),*];

        $(
            #[test]
            fn $name() { run_corpus_file(stringify!($name)); }
        )*

        #[test]
        fn corpus_membership_matches_frozen_baseline() {
            let declared = CORPUS_FILES
                .iter()
                .map(|name| baseline_name(name))
                .collect::<BTreeSet<_>>();
            let expected = Baseline::pinned()
                .file_names()
                .into_iter()
                .map(str::to_owned)
                .collect::<BTreeSet<_>>();

            assert_eq!(declared, expected);
            assert_eq!(
                FrozenStatements::pinned().file_names(),
                Baseline::pinned().file_names()
            );
            assert_eq!(
                FrozenSubmissions::pinned().file_names(),
                Baseline::pinned().file_names()
            );
            for name in Baseline::pinned().file_names() {
                assert_eq!(
                    FrozenSubmissions::pinned().file(name).source_git_blob,
                    FrozenStatements::pinned().file(name).source_git_blob,
                    "{name}: the submissions and the legacy spans read the same blob"
                );
            }
            assert_eq!(FrozenStatements::pinned().total_statements(), 43_474);
            assert_eq!(FrozenSubmissions::pinned().total(), TOTAL_SUBMISSIONS);
        }
    };
}

/// The number of frozen psql submissions (docs/differential-baseline.md).
const TOTAL_SUBMISSIONS: usize = 44_428;

// One entry per file in the PostgreSQL submodule's regression SQL
// corpus (vendor/postgres/src/test/regress/sql/). Generated with:
//   ls vendor/postgres/src/test/regress/sql/*.sql \
//     | xargs -n1 basename | sed 's/\.sql$//' \
//     | grep -E '^[A-Za-z_][A-Za-z0-9_]*$' | tr '\n' ','
//
// The four `collate.*` fixtures (collate.icu.utf8, collate.linux.utf8,
// collate.utf8, collate.windows.win1252) carry dots and are not valid Rust
// identifiers, so they cannot be macro-generated test functions and are
// omitted. The dotless `collate` fixture is included.
corpus_tests! {
    advisory_lock, aggregates, alter_generic, alter_operator, alter_table,
    amutils, arrays, r#async, bit, bitmapops, boolean, r#box, brin_bloom,
    brin_multi, brin, btree_index, case, char, circle, cluster, collate,
    combocid, comments, compression, constraints, conversion, copy, copy2,
    copydml, copyselect, create_aggregate, create_am, create_cast,
    create_function_c, create_function_sql, create_index_spgist,
    create_index, create_misc, create_operator, create_procedure,
    create_role, create_schema, create_table_like, create_table,
    create_type, create_view, database, date, dbsize, delete, dependency,
    domain, drop_if_exists, drop_operator, encoding, r#enum, equivclass,
    errors, euc_kr, event_trigger_login, event_trigger, explain,
    expressions, fast_default, float4, float8, foreign_data, foreign_key,
    functional_deps, generated, geometry, gin, gist, groupingsets, guc,
    hash_func, hash_index, hash_part, horology, identity, incremental_sort,
    index_including_gist, index_including, indexing, indirect_toast, inet,
    infinite_recurse, inherit, init_privs, insert_conflict, insert, int2,
    int4, int8, interval, join_hash, join, json_encoding, json,
    jsonb_jsonpath, jsonb, jsonpath_encoding, jsonpath, largeobject, limit,
    line, lock, lseg, macaddr, macaddr8, maintain_every, matview, md5,
    memoize, merge, misc_functions, misc_sanity, misc, money,
    multirangetypes, mvcc, name, namespace, numeric_big, numeric,
    numerology, object_address, oid, oidjoins, opr_sanity,
    partition_aggregate, partition_info, partition_join, partition_prune,
    password, path, pg_lsn, plancache, plpgsql, point, polygon,
    polymorphism, portals_p2, portals, predicate, prepare, prepared_xacts,
    privileges, psql_crosstab, psql, publication, random, rangefuncs,
    rangetypes, regex, regproc, reindex_catalog, reloptions,
    replica_identity, returning, roleattributes, rowsecurity, rowtypes,
    rules, sanity_check, security_label, select_distinct_on,
    select_distinct, select_having, select_implicit, select_into,
    select_parallel, select_views, select, sequence, spgist,
    sqljson_jsontable, sqljson_queryfuncs, sqljson, stats_ext, stats,
    strings, subscription, subselect, sysviews, tablesample, tablespace,
    temp, test_setup, text, tid, tidrangescan, tidscan, time, timestamp,
    timestamptz, timetz, transactions, triggers, truncate, tsdicts,
    tsearch, tsrf, tstypes, tuplesort, txid, type_sanity, typed_table,
    unicode, union, updatable_views, update, uuid, vacuum_parallel, vacuum,
    varchar, window, with, write_parallel, xid, xml, xmlmap,
}
