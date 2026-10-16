//! Group snapshots: one stopped cluster per schedule group holding the state
//! the corpus leaves after that group, so a file can be replayed on its own
//! server, started from a copy of the snapshot before its group.
//!
//! A snapshot is a whole cluster, not a database, so it holds the roles,
//! tablespaces and other objects the whole cluster shares as well.
//! Snapshots persist in the container between runs. Their directory carries
//! the vendored PostgreSQL commit and a format number, so a change to either
//! makes the harness rebuild them.

use std::time::Instant;

use super::cluster::{Container, PORT_BASE, ROOT};
use super::oracle::Server;
use super::plan::Plan;
use super::replay::execute_file;

/// Bump when the way files are executed changes, so old snapshots are not
/// reused.
const FORMAT: u32 = 2;

/// The snapshot directories for one corpus commit.
pub struct Snapshots {
    dir: String,
}

impl Snapshots {
    pub fn new(gitlink: &str) -> Self {
        Self {
            dir: format!("{ROOT}/snapshots/{}-{FORMAT}", &gitlink[..8]),
        }
    }

    /// The cluster holding the state before any group runs.
    fn base(&self) -> String {
        format!("{}/base", self.dir)
    }

    /// The cluster holding the state after `group` and every group before it.
    fn after_group(&self, group: usize) -> String {
        format!("{}/g{group:02}", self.dir)
    }

    /// The snapshot a file in `group` starts from.
    pub fn before_group(&self, group: usize) -> String {
        match group.checked_sub(1) {
            Some(previous) => self.after_group(previous),
            None => self.base(),
        }
    }

    /// Make every snapshot the plan needs, unless they exist already and
    /// `rebuild` is false. Snapshots of other formats or commits are removed.
    ///
    /// Every file runs in schedule order on one server, with a fresh session
    /// for each file, as pg_regress runs them. The server is stopped after
    /// each group so that the copy of its cluster is consistent.
    pub fn ensure(&self, container: &Container, plan: &Plan, rebuild: bool) {
        let complete = format!("{}/complete", self.dir);
        if !rebuild
            && container
                .exec(&format!("[ -f {complete} ] && echo yes || true"))
                .trim()
                == "yes"
        {
            eprintln!(
                "snapshots present; reusing them (set PG_ANALYZE_ORACLE_REBUILD=1 to rebuild)"
            );
            return;
        }

        let started = Instant::now();
        let build = format!("{ROOT}/build");
        container.exec(&format!(
            "rm -rf {ROOT}/snapshots {build} && mkdir -p {}",
            self.dir
        ));
        container.init(&build);
        container.copy(&build, &self.base());
        let server = Server::at_port(PORT_BASE);
        for (group, files) in plan.groups.iter().enumerate() {
            container.start(&build, PORT_BASE);
            for file in files {
                execute_file(&server, file);
            }
            container.stop(&build);
            container.copy(&build, &self.after_group(group));
            eprintln!("snapshot after group {group:02} ({} files)", files.len());
        }
        container.exec(&format!("rm -rf {build} {build}.log && touch {complete}"));
        eprintln!(
            "built {} snapshots in {:.1?}",
            plan.groups.len() + 1,
            started.elapsed()
        );
    }
}
