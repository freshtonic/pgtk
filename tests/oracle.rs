//! The Oracle harness: replay PostgreSQL's regression corpus against a live
//! server and against this crate, and hold the per-file outcome counts to a
//! pinned baseline.
//!
//! The test reads the connection string from `PG_ANALYZE_ORACLE_URL` and
//! passes with a message when it is unset. The server must be the major
//! version the baseline is for, and it must be disposable: the corpus
//! creates and drops objects, roles and databases freely.
//!
//! ```sh
//! docker run -d --name pg-analyze-oracle-17 -e POSTGRES_USER=oracle \
//!   -e POSTGRES_PASSWORD=oracle -e POSTGRES_DB=regression -p 54317:5432 postgres:17
//! PG_ANALYZE_ORACLE_URL=postgres://oracle:oracle@localhost:54317/regression \
//!   cargo test --test oracle -- --nocapture
//! ```
//!
//! Set `PG_ANALYZE_UPDATE_BASELINE=1` to pin the run's counts instead of
//! checking them. Every disagreement is written to `target/oracle/<file>.txt`.

mod support;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use support::baseline::{Baseline, FileCounts};
use support::corpus::{FrozenSplit, schedule};
use support::oracle::Oracle;
use support::replay::{Session, replay};
use support::subject::Stub;

const URL_VARIABLE: &str = "PG_ANALYZE_ORACLE_URL";
const UPDATE_VARIABLE: &str = "PG_ANALYZE_UPDATE_BASELINE";

#[test]
fn regression_corpus_matches_the_pinned_baseline() {
    let Ok(url) = std::env::var(URL_VARIABLE) else {
        println!("skipping the oracle harness: set {URL_VARIABLE} to run it against a server");
        return;
    };
    let oracle = Oracle::connect(&url).unwrap_or_else(|error| {
        let detail = error
            .as_db_error()
            .map(|db| db.message().to_owned())
            .unwrap_or_default();
        panic!("cannot connect: {error} {detail}")
    });
    let mut session = Session::new(oracle);
    let major = session
        .oracle
        .catalog()
        .expect("load the catalog once to learn the server version")
        .catalog
        .metadata()
        .server_version
        .major;

    let split = FrozenSplit::load();
    let report_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/oracle");
    std::fs::create_dir_all(&report_dir).expect("create report dir");

    let mut subject = Stub;
    let mut files = BTreeMap::new();
    let started = Instant::now();
    for name in schedule() {
        if !split.file_names().any(|known| known == name) {
            continue;
        }
        let file = split.read(&name);
        let file_started = Instant::now();
        let result = replay(&mut session, &file, &mut subject);
        eprintln!(
            "{name}: {:?} ({} catalog loads, {:.1?})",
            result.counts,
            result.catalog_loads,
            file_started.elapsed()
        );
        let report_path = report_dir.join(format!("{name}.txt"));
        if result.report.is_empty() {
            let _ = std::fs::remove_file(&report_path);
        } else {
            std::fs::write(&report_path, &result.report).expect("write report");
        }
        files.insert(name, FileCounts::from(result.counts));
    }
    eprintln!(
        "replayed {} files in {:.1?} with {} catalog loads",
        files.len(),
        started.elapsed(),
        session.catalog_loads
    );

    let actual = Baseline::from_results(u32::from(major), files);
    if std::env::var_os(UPDATE_VARIABLE).is_some() {
        actual.save();
        eprintln!("pinned {}", Baseline::path(actual.postgres_major).display());
        return;
    }
    let Some(pinned) = Baseline::load(actual.postgres_major) else {
        panic!(
            "no baseline at {}; run once with {UPDATE_VARIABLE}=1 to pin one",
            Baseline::path(actual.postgres_major).display()
        );
    };
    let differences = pinned.differences(&actual);
    assert!(
        differences.is_empty(),
        "the run differs from the pinned baseline; pin it with {UPDATE_VARIABLE}=1 if the change is intended:\n{}",
        differences.join("\n")
    );
}
