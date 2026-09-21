//! The worker pool that replays files in parallel.
//!
//! Each file runs in its own database, copied from the snapshot before its
//! group, after its earlier siblings in the group have been executed into
//! that copy. That gives every file exactly the state the serial schedule
//! gives it, so files may run in any order, with one exception: objects
//! shared by the whole cluster, such as roles, are not in any snapshot.
//! Files that create, alter or drop them (`cluster_ddl`) therefore run one
//! at a time in schedule order, in the cluster lane, and no file of group
//! `g` starts until the cluster lane has finished every group before `g`.
//! Inside one group pg_regress's own rule applies: files use disjoint names.

use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex};
use std::time::Instant;

use super::oracle::Server;
use super::plan::{Plan, PlannedFile};
use super::replay::{FileResult, execute_file, replay_file};
use super::snapshots::Snapshots;
use super::subject::Subject;

/// The queue of work and the cluster lane's progress.
struct State {
    /// Cluster-DDL files still to run, in schedule order.
    cluster: Vec<usize>,
    /// Whether a worker is in the cluster lane right now.
    cluster_busy: bool,
    /// Other files still to run, in schedule order.
    regular: Vec<usize>,
    /// Files running right now.
    running: usize,
}

impl State {
    /// The last group whose cluster-DDL files have all finished, if any.
    fn cluster_done_through(&self, files: &[&PlannedFile]) -> Option<usize> {
        match self.cluster.first() {
            Some(&next) => files[next].group.checked_sub(1),
            None => Some(usize::MAX),
        }
    }

    /// A regular file may start once the cluster lane is past its group.
    fn regular_ready(&self, files: &[&PlannedFile], index: usize) -> bool {
        let group = files[index].group;
        group == 0
            || self
                .cluster_done_through(files)
                .is_some_and(|done| done >= group - 1)
    }
}

enum Task {
    Cluster(usize),
    Regular(usize),
}

/// Replay every file in `selected` with `jobs` workers, returning each
/// file's result keyed by name.
pub fn run(
    config: &postgres::Config,
    snapshots: &Snapshots,
    plan: &Plan,
    selected: &[&PlannedFile],
    jobs: usize,
    make_subject: impl Fn() -> Box<dyn Subject> + Sync,
) -> BTreeMap<String, FileResult> {
    let files = selected;
    let state = Mutex::new(State {
        cluster: (0..files.len()).filter(|&i| files[i].cluster_ddl).collect(),
        cluster_busy: false,
        regular: (0..files.len())
            .filter(|&i| !files[i].cluster_ddl)
            .collect(),
        running: 0,
    });
    let ready = Condvar::new();
    let results = Mutex::new(BTreeMap::new());

    std::thread::scope(|scope| {
        for worker in 0..jobs {
            let state = &state;
            let ready = &ready;
            let results = &results;
            let make_subject = &make_subject;
            scope.spawn(move || {
                let mut server = Server::from_config(config)
                    .unwrap_or_else(|error| panic!("worker {worker}: cannot connect: {error}"));
                let mut subject = make_subject();
                loop {
                    let task = {
                        let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
                        loop {
                            if guard.cluster.is_empty() && guard.regular.is_empty() {
                                break None;
                            }
                            if !guard.cluster_busy
                                && let Some(&index) = guard.cluster.first()
                            {
                                guard.cluster_busy = true;
                                guard.running += 1;
                                break Some(Task::Cluster(index));
                            }
                            if let Some(position) = guard
                                .regular
                                .iter()
                                .position(|&i| guard.regular_ready(files, i))
                            {
                                let index = guard.regular.remove(position);
                                guard.running += 1;
                                break Some(Task::Regular(index));
                            }
                            guard = ready.wait(guard).unwrap_or_else(|p| p.into_inner());
                        }
                    };
                    let Some(task) = task else { break };
                    let index = match task {
                        Task::Cluster(index) | Task::Regular(index) => index,
                    };
                    let file = files[index];
                    let started = Instant::now();
                    let result = replay_in_own_database(
                        &mut server,
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
                    let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
                    guard.running -= 1;
                    if let Task::Cluster(_) = task {
                        guard.cluster.remove(0);
                        guard.cluster_busy = false;
                    }
                    ready.notify_all();
                }
            });
        }
    });

    results.into_inner().unwrap_or_else(|p| p.into_inner())
}

/// Copy the snapshot before the file's group, execute the file's earlier
/// siblings into the copy, replay the file, and drop the copy.
fn replay_in_own_database(
    server: &mut Server,
    snapshots: &Snapshots,
    plan: &Plan,
    file: &PlannedFile,
    worker: usize,
    subject: &mut dyn Subject,
) -> FileResult {
    let database = format!("pga_work_{worker}");
    server.drop_database(&database);
    server.create_database(&database, &snapshots.before_group(file.group));
    for sibling in &plan.groups[file.group][..file.index] {
        execute_file(server, &database, sibling);
    }
    let result = replay_file(server, &database, file, subject);
    server.drop_database(&database);
    result
}
