//! Group snapshots: one database per schedule group holding the state the
//! corpus leaves after that group, so a file can be replayed on its own from
//! a copy of the snapshot before its group.
//!
//! Snapshots persist in the server between runs. Their names carry the
//! vendored PostgreSQL commit and a format number, so a change to either
//! makes the harness rebuild them.

use std::time::Instant;

use super::oracle::Server;
use super::plan::Plan;
use super::replay::execute_file;

/// Bump when the way files are executed changes, so old snapshots are not
/// reused.
const FORMAT: u32 = 1;

/// The naming scheme for one corpus commit.
pub struct Snapshots {
    prefix: String,
}

impl Snapshots {
    pub fn new(gitlink: &str) -> Self {
        Self {
            prefix: format!("pga_snap_{}_{FORMAT}_", &gitlink[..8]),
        }
    }

    /// The database holding the state before any group runs.
    pub fn base(&self) -> String {
        format!("{}base", self.prefix)
    }

    /// The database holding the state after `group` and every group before it.
    pub fn after_group(&self, group: usize) -> String {
        format!("{}g{group:02}", self.prefix)
    }

    /// The snapshot a file in `group` starts from.
    pub fn before_group(&self, group: usize) -> String {
        match group.checked_sub(1) {
            Some(previous) => self.after_group(previous),
            None => self.base(),
        }
    }

    /// Make every snapshot the plan needs, unless all of them exist already
    /// and `rebuild` is false. Older snapshots of other formats or commits
    /// are dropped.
    pub fn ensure(&self, server: &mut Server, plan: &Plan, rebuild: bool) {
        let existing = server.databases_like("pga_snap_%");
        let needed = std::iter::once(self.base())
            .chain((0..plan.groups.len()).map(|group| self.after_group(group)))
            .collect::<Vec<_>>();
        if !rebuild && needed.iter().all(|name| existing.contains(name)) {
            eprintln!(
                "snapshots present; reusing them (set PG_ANALYZE_ORACLE_REBUILD=1 to rebuild)"
            );
            return;
        }
        for name in existing
            .iter()
            .chain(server.databases_like("pga_work_%").iter())
        {
            server.drop_database(name);
        }

        let started = Instant::now();
        const BUILD: &str = "pga_build";
        server.drop_database(BUILD);
        server.create_database(BUILD, "template0");
        server.create_database(&self.base(), BUILD);
        for (group, files) in plan.groups.iter().enumerate() {
            for file in files {
                execute_file(server, BUILD, file);
            }
            server.create_database(&self.after_group(group), BUILD);
            eprintln!("snapshot after group {group:02} ({} files)", files.len());
        }
        server.drop_database(BUILD);
        eprintln!(
            "built {} snapshots in {:.1?}",
            needed.len(),
            started.elapsed()
        );
    }
}
