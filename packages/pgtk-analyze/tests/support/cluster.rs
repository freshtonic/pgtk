//! The servers the Oracle runs in.
//!
//! Every replayed file gets a PostgreSQL server of its own: a cluster copied
//! from the snapshot before the file's group and started on a port that only
//! one worker uses. Roles, tablespaces, settings, statistics and every other
//! object the whole cluster shares therefore belong to one file, so no file
//! can see what another file does, whatever runs at the same time.
//!
//! The servers run inside one Docker container that the harness makes and
//! keeps between runs. The container publishes one port for each worker, and
//! it holds the snapshots under [`ROOT`]. Only one harness run may use the
//! container at a time: a run stops every server it finds when it starts.

use std::process::Command;

/// Where the harness keeps its clusters inside the container.
pub const ROOT: &str = "/oracle";

/// The host and container port of worker 0. Worker `n` uses
/// `PORT_BASE + n`, and the snapshot build uses `PORT_BASE`.
pub const PORT_BASE: u16 = 54400;

/// The number of ports the container publishes, so the most workers a run
/// can have.
pub const MAX_WORKERS: usize = 32;

/// Bump when the container's layout changes: its ports, its mounts or what
/// [`Container::ensure`] prepares inside it. A container of another layout
/// must be removed by hand.
const LAYOUT: &str = "1";

/// The settings each cluster adds to the defaults. Durability is off because
/// every cluster is disposable and copies are I/O bound.
const SETTINGS: &str = "
listen_addresses = '*'
fsync = off
synchronous_commit = off
full_page_writes = off
";

/// The harness's container.
pub struct Container {
    name: String,
}

impl Container {
    /// Find or make the container for `image`, and start it if it is
    /// stopped.
    pub fn ensure(image: &str) -> Self {
        let name = format!(
            "pgtk-analyze-oracle-{}",
            image
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect::<String>()
        );
        let inspect = Command::new("docker")
            .args([
                "container",
                "inspect",
                "--format",
                "{{.State.Running}} {{index .Config.Labels \"pgtk-analyze.oracle\"}}",
                &name,
            ])
            .output()
            .unwrap_or_else(|error| panic!("cannot run docker: {error}"));
        if inspect.status.success() {
            let state = String::from_utf8_lossy(&inspect.stdout);
            let mut words = state.split_whitespace();
            let running = words.next() == Some("true");
            if words.next() != Some(LAYOUT) {
                panic!("container {name} has another layout; remove it with `docker rm -f {name}`");
            }
            if !running {
                docker(&["start", &name]);
            }
        } else {
            let last = PORT_BASE + MAX_WORKERS as u16 - 1;
            let ports = format!("127.0.0.1:{PORT_BASE}-{last}:{PORT_BASE}-{last}");
            let label = format!("pgtk-analyze.oracle={LAYOUT}");
            // Parallel query uses dynamic shared memory in /dev/shm, and
            // Docker's default of 64 MB is too small for many servers.
            docker(&[
                "run",
                "--detach",
                "--name",
                &name,
                "--label",
                &label,
                "--shm-size=2g",
                "--publish",
                &ports,
                image,
                "sleep",
                "infinity",
            ]);
            docker(&[
                "exec",
                &name,
                "bash",
                "-ec",
                &format!("mkdir -p {ROOT} && chown postgres:postgres {ROOT}"),
            ]);
        }
        Self { name }
    }

    /// Run a bash script as the `postgres` user in the container, and
    /// return what it writes to standard output.
    pub fn exec(&self, script: &str) -> String {
        docker(&["exec", "-u", "postgres", &self.name, "bash", "-ec", script])
    }

    /// The major version of the server binaries.
    pub fn major(&self) -> u32 {
        let version = self.exec("pg_config --version");
        version
            .trim()
            .strip_prefix("PostgreSQL ")
            .and_then(|rest| rest.split(['.', ' ']).next())
            .and_then(|major| major.parse().ok())
            .unwrap_or_else(|| panic!("cannot read the version from {version:?}"))
    }

    /// Stop every server an earlier run left behind and remove its cluster.
    pub fn reset(&self) {
        self.exec(&format!(
            "for pid in {ROOT}/build/postmaster.pid {ROOT}/work/*/postmaster.pid; do
                 [ -f \"$pid\" ] && pg_ctl -D \"$(dirname \"$pid\")\" -m immediate stop >/dev/null || true
             done
             rm -rf {ROOT}/build {ROOT}/work
             mkdir -p {ROOT}/work"
        ));
    }

    /// Make an empty cluster in `dir` with the `oracle` superuser and the
    /// `regression` database, as pg_regress has it.
    pub fn init(&self, dir: &str) {
        self.exec(&format!(
            "initdb -D {dir} -U {user} -A trust --locale=C -E UTF8 >/dev/null
             cat >> {dir}/postgresql.conf <<'EOF'
{SETTINGS}
EOF
             echo 'host all all all trust' >> {dir}/pg_hba.conf",
            user = super::oracle::USER,
        ));
        self.start(dir, PORT_BASE);
        self.exec(&format!(
            "psql -X -q -p {PORT_BASE} -U {user} -d postgres -c 'CREATE DATABASE {database}'",
            user = super::oracle::USER,
            database = super::oracle::DATABASE,
        ));
        self.stop(dir);
    }

    /// Start the cluster in `dir` on `port`, and wait until it accepts
    /// connections.
    pub fn start(&self, dir: &str, port: u16) {
        self.exec(&format!(
            "pg_ctl -D {dir} -o '-p {port}' -l {dir}.log -w -t 120 start >/dev/null"
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

/// Run docker, and return its standard output. A failure ends the test.
fn docker(args: &[&str]) -> String {
    let output = Command::new("docker")
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("cannot run docker: {error}"));
    if !output.status.success() {
        panic!(
            "docker {} failed: {}",
            args.first().copied().unwrap_or_default(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).into_owned()
}
