//! The worker pool that replays files in parallel.
//!
//! Each file runs on a server of its own, started from a copy of the
//! snapshot before its group, after its earlier siblings in the group have
//! been executed on it. That gives every file exactly the state the serial
//! schedule gives it, objects the whole cluster shares included, and no
//! other file can change that state while it runs. Files therefore run in
//! any order. Inside one group pg_regress's own rule applies: files use
//! disjoint names.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use super::cluster::{Container, PORT_BASE, ROOT};
use super::oracle::Server;
use super::plan::{Plan, PlannedFile};
use super::replay::{FileResult, execute_file, replay_file};
use super::snapshots::Snapshots;
use super::subject::Subject;

/// Replay every file in `selected` with `jobs` workers, returning each
/// file's result keyed by name.
pub fn run(
    container: &Container,
    snapshots: &Snapshots,
    plan: &Plan,
    selected: &[&PlannedFile],
    jobs: usize,
    make_subject: impl Fn() -> Box<dyn Subject> + Sync,
) -> BTreeMap<String, FileResult> {
    let queue = Mutex::new(selected.iter().copied().collect::<VecDeque<_>>());
    let results = Mutex::new(BTreeMap::new());

    std::thread::scope(|scope| {
        for worker in 0..jobs {
            let queue = &queue;
            let results = &results;
            let make_subject = &make_subject;
            scope.spawn(move || {
                let mut subject = make_subject();
                loop {
                    let next = queue.lock().unwrap_or_else(|p| p.into_inner()).pop_front();
                    let Some(file) = next else { break };
                    let started = Instant::now();
                    let result = replay_on_own_server(
                        container,
                        snapshots,
                        plan,
                        file,
                        worker,
                        subject.as_mut(),
                    );
                    eprintln!(
                        "{}: {:?} ({} catalog loads, {:.1?}, worker {worker})",
                        file.name,
                        result.counts,
                        result.catalog_loads,
                        started.elapsed()
                    );
                    results
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .insert(file.name.clone(), result);
                }
            });
        }
    });

    results.into_inner().unwrap_or_else(|p| p.into_inner())
}

/// Copy the snapshot before the file's group, start a server on the copy,
/// execute the file's earlier siblings on it, replay the file, and discard
/// the server.
fn replay_on_own_server(
    container: &Container,
    snapshots: &Snapshots,
    plan: &Plan,
    file: &PlannedFile,
    worker: usize,
    subject: &mut dyn Subject,
) -> FileResult {
    let dir = format!("{ROOT}/work/w{worker:02}");
    let port = PORT_BASE + u16::try_from(worker).expect("a worker number fits in a port");
    container.copy(&snapshots.before_group(file.group), &dir);
    container.start(&dir, port);
    let server = Server::at_port(port);
    for sibling in &plan.groups[file.group][..file.index] {
        execute_file(&server, sibling);
    }
    let result = replay_file(&server, file, subject);
    container.discard(&dir);
    result
}
