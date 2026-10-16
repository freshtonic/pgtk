//! The servers the Oracle runs in.
//!
//! Every replayed file gets a PostgreSQL server of its own: a cluster copied
//! from the snapshot before the file's group and started on a port that only
//! one worker uses. Roles, tablespaces, settings, statistics and every other
//! object the whole cluster shares therefore belong to one file, so no file
//! can see what another file does, whatever runs at the same time.
//!
//! The servers run on this host, from the pinned PostgreSQL release that
//! `pgtk-test-server` builds from the vendored source. The harness keeps their
//! clusters and the snapshots under [`Host::root`] in the target directory,
//! where the snapshots persist between runs. Each server listens only on a
//! Unix socket in the socket directory of the checkout, with the worker's port
//! in the socket name, so a run shares no port with any other process. Only
//! one run of the harness in a checkout may use the servers at a time: a run
//! stops every server it finds when it starts.

use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The port of worker 0. Worker `n` uses `PORT_BASE + n`, and the snapshot
/// build uses `PORT_BASE`. A port only names a socket in the socket
/// directory.
pub const PORT_BASE: u16 = 54400;

/// The most workers a run can have.
pub const MAX_WORKERS: usize = 32;

/// The settings each cluster adds to the defaults. Durability is off because
/// every cluster is disposable and copies are I/O bound.
const SETTINGS: &str = "
listen_addresses = ''
fsync = off
synchronous_commit = off
full_page_writes = off
";

/// The host that runs the harness's servers.
pub struct Host {
    root: String,
    sockets: PathBuf,
    path: OsString,
}

impl Host {
    /// Install the server binaries if the cache does not hold them, and make
    /// the directories of the harness.
    pub fn ensure() -> Self {
        let installation = pgtk_test_server::installation();
        let root = concat!(env!("CARGO_TARGET_TMPDIR"), "/pgtk-analyze-oracle").to_owned();
        // A short directory, named after the checkout: the socket path must
        // fit in `sun_path`.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        root.hash(&mut hasher);
        let sockets =
            std::env::temp_dir().join(format!("pgtk-oracle-{:08x}", hasher.finish() as u32));
        for dir in [Path::new(&root), sockets.as_path()] {
            std::fs::create_dir_all(dir)
                .unwrap_or_else(|error| panic!("cannot make {}: {error}", dir.display()));
        }
        Self {
            root,
            sockets,
            path: installation.path_env(),
        }
    }

    /// Where the harness keeps its clusters.
    pub fn root(&self) -> &str {
        &self.root
    }

    /// The directory of the servers' sockets.
    pub fn socket_dir(&self) -> &Path {
        &self.sockets
    }

    /// Run a bash script with the server binaries first in `PATH`, and
    /// return what it writes to standard output.
    pub fn exec(&self, script: &str) -> String {
        let output = Command::new("bash")
            .args(["-ec", script])
            .env("PATH", &self.path)
            .output()
            .unwrap_or_else(|error| panic!("cannot run bash: {error}"));
        if !output.status.success() {
            panic!(
                "the script failed: {}\n{script}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// The major version of the server binaries.
    pub fn major(&self) -> u32 {
        pgtk_test_server::installation().major()
    }

    /// Stop every server an earlier run left behind and remove its cluster.
    pub fn reset(&self) {
        self.exec(&format!(
            "for pid in {root}/build/postmaster.pid {root}/work/*/postmaster.pid; do
                 [ -f \"$pid\" ] && pg_ctl -D \"$(dirname \"$pid\")\" -m immediate stop >/dev/null || true
             done
             rm -rf {root}/build {root}/work
             mkdir -p {root}/work",
            root = self.root,
        ));
    }

    /// Make an empty cluster in `dir` with the `oracle` superuser and the
    /// `regression` database, as pg_regress has it.
    pub fn init(&self, dir: &str) {
        self.exec(&format!(
            "initdb -D {dir} -U {user} -A trust --locale=C -E UTF8 >/dev/null
             cat >> {dir}/postgresql.conf <<'EOF'
{SETTINGS}
EOF",
            user = super::oracle::USER,
        ));
        self.start(dir, PORT_BASE);
        self.exec(&format!(
            "psql -X -q -h {sockets} -p {PORT_BASE} -U {user} -d postgres -c 'CREATE DATABASE {database}'",
            sockets = self.sockets.display(),
            user = super::oracle::USER,
            database = super::oracle::DATABASE,
        ));
        self.stop(dir);
    }

    /// Start the cluster in `dir` on `port`, and wait until it accepts
    /// connections.
    pub fn start(&self, dir: &str, port: u16) {
        self.exec(&format!(
            "pg_ctl -D {dir} -o '-p {port} -k {sockets}' -l {dir}.log -w -t 120 start >/dev/null",
            sockets = self.sockets.display(),
        ));
    }

    /// Stop the cluster in `dir` cleanly, so a copy of it is consistent.
    pub fn stop(&self, dir: &str) {
        self.exec(&format!("pg_ctl -D {dir} -m fast -w stop >/dev/null"));
    }

    /// Copy the stopped cluster in `from` to `to`, replacing `to`.
    pub fn copy(&self, from: &str, to: &str) {
        self.exec(&format!("rm -rf {to} && cp -a {from} {to}"));
    }

    /// Stop the cluster in `dir` at once and remove it.
    pub fn discard(&self, dir: &str) {
        self.exec(&format!(
            "pg_ctl -D {dir} -m immediate stop >/dev/null || true
             rm -rf {dir} {dir}.log"
        ));
    }
}
