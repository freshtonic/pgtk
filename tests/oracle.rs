//! The Oracle harness: replay PostgreSQL's regression corpus against a live
//! server and against this crate, and hold the per-file outcome counts to a
//! pinned baseline. `docs/oracle-harness.md` explains the design.
//!
//! The test reads the connection string from `PG_ANALYZE_ORACLE_URL` and
//! passes with a message when it is unset. The server must be the major
//! version the baseline is for, and it must be disposable: the corpus
//! creates and drops objects, roles and databases freely.
//!
//! ```sh
//! docker run -d --name pg-analyze-oracle-17 -e POSTGRES_USER=oracle \
//!   -e POSTGRES_PASSWORD=oracle -e POSTGRES_DB=regression -p 54317:5432 \
//!   postgres:17 -c fsync=off -c synchronous_commit=off -c full_page_writes=off \
//!   -c max_connections=200
//! PG_ANALYZE_ORACLE_URL=postgres://oracle:oracle@localhost:54317/regression \
//!   cargo test --test oracle -- --nocapture
//! ```
//!
//! - `PG_ANALYZE_ORACLE_JOBS=n` sets the worker count (default: the CPU count).
//! - `PG_ANALYZE_ORACLE_FILES=int4,join` replays only those files and checks
//!   only their counts.
//! - `PG_ANALYZE_ORACLE_REBUILD=1` rebuilds the group snapshots the server
//!   keeps between runs.
//! - `PG_ANALYZE_UPDATE_BASELINE=1` pins the run's counts instead of checking
//!   them.
//!
//! Every disagreement is written to `target/oracle/<file>.txt`.

mod support;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use support::baseline::{Baseline, FileCounts};
use support::corpus::FrozenSplit;
use support::oracle::Server;
use support::plan::Plan;
use support::scheduler;
use support::snapshots::Snapshots;
use support::subject::{Stub, Subject};

const URL_VARIABLE: &str = "PG_ANALYZE_ORACLE_URL";
const JOBS_VARIABLE: &str = "PG_ANALYZE_ORACLE_JOBS";
const FILES_VARIABLE: &str = "PG_ANALYZE_ORACLE_FILES";
const REBUILD_VARIABLE: &str = "PG_ANALYZE_ORACLE_REBUILD";
const UPDATE_VARIABLE: &str = "PG_ANALYZE_UPDATE_BASELINE";

#[test]
fn regression_corpus_matches_the_pinned_baseline() {
    let Ok(url) = std::env::var(URL_VARIABLE) else {
        println!("skipping the oracle harness: set {URL_VARIABLE} to run it against a server");
        return;
    };
    let mut server = Server::connect(&url).unwrap_or_else(|error| {
        let detail = error
            .as_db_error()
            .map(|db| db.message().to_owned())
            .unwrap_or_default();
        panic!("cannot connect: {error} {detail}")
    });
    let major = {
        let mut probe = server.session("postgres").expect("open a probe session");
        u32::from(
            probe
                .catalog()
                .expect("load the catalog once to learn the server version")
                .catalog
                .metadata()
                .server_version
                .major,
        )
    };

    let started = Instant::now();
    let split = FrozenSplit::load();
    let plan = Plan::build(&split);
    eprintln!(
        "planned {} files in {} groups in {:.1?}",
        plan.files().count(),
        plan.groups.len(),
        started.elapsed()
    );

    let snapshots = Snapshots::new(split.gitlink());
    let rebuild = std::env::var_os(REBUILD_VARIABLE).is_some();
    snapshots.ensure(&mut server, &plan, rebuild);

    let only = std::env::var(FILES_VARIABLE).ok().map(|list| {
        list.split(',')
            .map(|name| {
                let name = name.trim();
                if name.ends_with(".sql") {
                    name.to_owned()
                } else {
                    format!("{name}.sql")
                }
            })
            .collect::<Vec<_>>()
    });
    let selected = plan
        .files()
        .filter(|file| only.as_ref().is_none_or(|only| only.contains(&file.name)))
        .collect::<Vec<_>>();
    if let Some(only) = &only {
        for name in only {
            assert!(
                selected.iter().any(|file| &file.name == name),
                "{name} is not in the corpus schedule"
            );
        }
    }

    let jobs = std::env::var(JOBS_VARIABLE)
        .ok()
        .and_then(|jobs| jobs.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let report_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/oracle");
    std::fs::create_dir_all(&report_dir).expect("create report dir");

    let replay_started = Instant::now();
    let results = scheduler::run(server.config(), &snapshots, &plan, &selected, jobs, || {
        Box::new(Stub) as Box<dyn Subject>
    });
    let loads = results
        .values()
        .map(|result| result.catalog_loads)
        .sum::<usize>();
    eprintln!(
        "replayed {} files with {jobs} workers in {:.1?} with {loads} catalog loads",
        results.len(),
        replay_started.elapsed()
    );

    let mut files = BTreeMap::new();
    for (name, result) in results {
        let report_path = report_dir.join(format!("{name}.txt"));
        if result.report.is_empty() {
            let _ = std::fs::remove_file(&report_path);
        } else {
            std::fs::write(&report_path, &result.report).expect("write report");
        }
        files.insert(name, FileCounts::from(result.counts));
    }

    let pinned = Baseline::load(major);
    if std::env::var_os(UPDATE_VARIABLE).is_some() {
        let updated = match (pinned, only.is_some()) {
            (Some(pinned), true) => pinned.with_files(files),
            _ => Baseline::from_results(major, files),
        };
        updated.save();
        eprintln!("pinned {}", Baseline::path(major).display());
        return;
    }
    let Some(pinned) = pinned else {
        panic!(
            "no baseline at {}; run once with {UPDATE_VARIABLE}=1 to pin one",
            Baseline::path(major).display()
        );
    };
    let actual = Baseline::from_results(major, files);
    let pinned = match &only {
        Some(_) => pinned.restricted_to(actual.files.keys().map(String::as_str)),
        None => pinned,
    };
    let differences = pinned.differences(&actual);
    assert!(
        differences.is_empty(),
        "the run differs from the pinned baseline; pin it with {UPDATE_VARIABLE}=1 if the change is intended:\n{}",
        differences.join("\n")
    );
}
