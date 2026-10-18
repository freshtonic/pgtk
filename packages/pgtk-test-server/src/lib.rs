//! PostgreSQL servers for the pgtk tests, built from the vendored PostgreSQL
//! source.
//!
//! [`installation_of`] builds and installs the pinned PostgreSQL release of a
//! version feature (`pgtk-oracle/pins.tsv`) with
//! `pgtk-oracle/scripts/build-pg.sh`, once for each machine, in the shared
//! cache (ADR 0012 of pgtk-sql). [`Server::start_of`] starts a server of that
//! installation for one test process. [`installation`] and [`Server::start`]
//! use [`FEATURE`].
//!
//! A server listens only on a Unix socket in a directory of its own, and it
//! keeps its cluster in that directory too. Servers therefore share no port and
//! no data directory, so any number of them can run at the same time, from
//! one test process or from many.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The version feature whose pinned PostgreSQL release [`installation`] and
/// [`Server::start`] use.
pub const FEATURE: &str = "pg17";

/// The port in the name of a server's socket. Each server has its own socket
/// directory, so every server can use the same port.
pub const PORT: u16 = 5432;

/// The superuser of a server that [`Server::start`] makes.
pub const SUPERUSER: &str = "postgres";

/// An installed PostgreSQL server.
pub struct Installation {
    bin: PathBuf,
}

/// The installation of the pinned release of [`FEATURE`].
pub fn installation() -> &'static Installation {
    installation_of(FEATURE)
}

/// The installation of the pinned release of the version feature `feature`,
/// such as `pg16`. The first call for a feature in a process builds and
/// installs the release if the cache does not hold it yet.
///
/// # Panics
///
/// Panics when the build fails. The build needs a C toolchain, `make`,
/// `bison`, `flex`, `perl` and libxml2, and the pinned commit in the
/// `vendor/postgres` object store.
pub fn installation_of(feature: &str) -> &'static Installation {
    static INSTALLATIONS: OnceLock<Mutex<HashMap<String, &'static OnceLock<Installation>>>> =
        OnceLock::new();
    let cell = *INSTALLATIONS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .entry(feature.to_owned())
        .or_insert_with(|| Box::leak(Box::new(OnceLock::new())));
    // Outside the map lock, so two features build at the same time.
    cell.get_or_init(|| install(feature))
}

fn install(feature: &str) -> Installation {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../pgtk-oracle/scripts/build-pg.sh");
    let output = Command::new("bash")
        .arg(&script)
        .arg(feature)
        .env("PGTK_PG_INSTALL", "1")
        .stderr(Stdio::inherit())
        .output()
        .unwrap_or_else(|error| panic!("cannot run {}: {error}", script.display()));
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        panic!(
            "{} {feature} failed with {}:\n{stdout}",
            script.display(),
            output.status
        );
    }
    let bin = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("PGTK_PG_BIN="))
        .next_back()
        .unwrap_or_else(|| {
            panic!(
                "{} did not name the bin directory:\n{stdout}",
                script.display()
            )
        });
    Installation {
        bin: PathBuf::from(bin),
    }
}

impl Installation {
    /// The directory of the server binaries.
    pub fn bin_dir(&self) -> &Path {
        &self.bin
    }

    /// A `PATH` value that finds the binaries of this installation first.
    pub fn path_env(&self) -> OsString {
        let mut paths = vec![self.bin.clone()];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path));
        }
        std::env::join_paths(paths).expect("the bin directory is a valid PATH entry")
    }

    /// A command that runs the binary `tool` of this installation.
    pub fn command(&self, tool: &str) -> Command {
        Command::new(self.bin.join(tool))
    }

    /// The major version of the installation.
    pub fn major(&self) -> u32 {
        let version = run(self.command("pg_config").arg("--version"));
        version
            .trim()
            .strip_prefix("PostgreSQL ")
            .and_then(|rest| rest.split(['.', ' ']).next())
            .and_then(|major| major.parse().ok())
            .unwrap_or_else(|| panic!("cannot read the version from {version:?}"))
    }
}

/// A running server with a cluster of its own. Dropping it stops the server
/// and removes the cluster.
///
/// If the test process ends without dropping it, a watcher process stops the
/// server and removes the cluster.
pub struct Server {
    root: PathBuf,
    installation: &'static Installation,
}

impl Server {
    /// [`Server::start_of`] for [`FEATURE`].
    pub fn start() -> Self {
        Self::start_of(FEATURE)
    }

    /// Make an empty cluster with the [`SUPERUSER`] in a new directory, and
    /// start a server of the pinned release of the version feature `feature`
    /// on it.
    pub fn start_of(feature: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let installation = installation_of(feature);
        // A short directory: the socket path must fit in `sun_path`.
        let root = std::env::temp_dir().join(format!(
            "pgtk-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)
            .unwrap_or_else(|error| panic!("cannot make {}: {error}", root.display()));
        let server = Self { root, installation };

        run(installation
            .command("initdb")
            .arg("-D")
            .arg(server.data_dir())
            .args(["-U", SUPERUSER, "-A", "trust", "--locale=C", "-E", "UTF8"]));
        watch(installation, &server.root, &server.data_dir());
        run(installation
            .command("pg_ctl")
            .arg("-D")
            .arg(server.data_dir())
            .arg("-o")
            .arg(format!(
                "-p {PORT} -k {} -c listen_addresses= -c fsync=off -c synchronous_commit=off -c full_page_writes=off",
                server.socket_dir().display()
            ))
            .arg("-l")
            .arg(server.root.join("server.log"))
            .args(["-w", "-t", "120", "start"]));
        server
    }

    fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    /// The directory of the server's socket, the `host` of a connection.
    pub fn socket_dir(&self) -> &Path {
        &self.root
    }

    /// A connection string for the database `dbname` as the [`SUPERUSER`].
    pub fn connection_string(&self, dbname: &str) -> String {
        format!(
            "host={} port={PORT} user={SUPERUSER} dbname={dbname}",
            self.socket_dir().display()
        )
    }

    /// The installation that the server runs.
    pub fn installation(&self) -> &'static Installation {
        self.installation
    }

    /// Make the empty database `name`, and return a connection string for it.
    pub fn create_database(&self, name: &str) -> String {
        run(self
            .installation
            .command("createdb")
            .arg("-h")
            .arg(self.socket_dir())
            .args(["-p", &PORT.to_string(), "-U", SUPERUSER, name]));
        self.connection_string(name)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self
            .installation
            .command("pg_ctl")
            .arg("-D")
            .arg(self.data_dir())
            .args(["-m", "immediate", "stop"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Start a process that stops the server on `data` and removes `root` when
/// this process ends.
#[expect(
    clippy::zombie_processes,
    reason = "the watcher must outlive this process, so this process does not wait for it"
)]
fn watch(installation: &Installation, root: &Path, data: &Path) {
    let script = r#"while kill -0 "$1" 2>/dev/null; do sleep 1; done
"$2" -D "$3" -m immediate stop >/dev/null 2>&1
rm -rf "$4""#;
    Command::new("/bin/sh")
        .args(["-c", script, "watch"])
        .arg(std::process::id().to_string())
        .arg(installation.bin_dir().join("pg_ctl"))
        .arg(data)
        .arg(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start the server watcher");
}

/// Run `command`, and return its standard output. A failure ends the test.
fn run(command: &mut Command) -> String {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("cannot run {command:?}: {error}"));
    if !output.status.success() {
        panic!(
            "{command:?} failed with {}:\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_servers_run_at_the_same_time() {
        let first = Server::start();
        let second = Server::start();
        for server in [&first, &second] {
            let url = server.create_database("probe");
            let host = server.socket_dir().to_owned();
            assert!(url.contains(&host.display().to_string()));
            assert!(host.join(format!(".s.PGSQL.{PORT}")).exists());
        }
        assert_ne!(first.socket_dir(), second.socket_dir());
    }

    #[test]
    fn dropping_a_server_removes_its_cluster() {
        let server = Server::start();
        let root = server.socket_dir().to_owned();
        drop(server);
        assert!(!root.exists());
    }

    #[test]
    fn the_installation_is_the_pinned_major() {
        assert_eq!(installation().major(), 17);
    }

    #[test]
    fn a_server_runs_the_release_of_its_feature() {
        let server = Server::start_of("pg16");
        assert_eq!(server.installation().major(), 16);
        server.create_database("probe");
    }
}
