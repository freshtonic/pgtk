#![doc = include_str!("../README.md")]
//!
//! # Details
//!
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
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The version feature whose pinned PostgreSQL release [`installation`] and
/// [`Server::start`] use.
pub const FEATURE: &str = "pg17";

/// The port in the name of the socket of a server that does not listen on TCP.
/// Each server has its own socket directory, so every such server can use the
/// same port.
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
    port: u16,
    tcp: bool,
}

/// The settings of a server that [`Builder::start`] starts. [`Server::builder`]
/// makes one.
///
/// By default a server listens only on its Unix socket, and every connection
/// is trusted.
#[must_use]
pub struct Builder {
    feature: String,
    tcp: bool,
    host_auth: Option<String>,
    password: Option<String>,
    settings: Vec<(String, String)>,
    files: Vec<(String, Vec<u8>)>,
    init_sql: Vec<String>,
}

impl Builder {
    /// Also listen on `127.0.0.1`, on a free port ([`Server::tcp_address`]).
    pub fn tcp(mut self) -> Self {
        self.tcp = true;
        self
    }

    /// Authenticate the TCP connections with `method` (`trust`, `password`,
    /// `md5` or `scram-sha-256`), the `--auth-host` of `initdb`. The Unix
    /// socket stays trusted. Implies [`Builder::tcp`].
    pub fn host_auth(mut self, method: &str) -> Self {
        self.host_auth = Some(method.to_owned());
        self.tcp()
    }

    /// The password of the [`SUPERUSER`], stored with the default
    /// `password_encryption`.
    pub fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_owned());
        self
    }

    /// Set the server parameter `name` to `value` in `postgresql.conf`.
    pub fn setting(mut self, name: &str, value: &str) -> Self {
        self.settings.push((name.to_owned(), value.to_owned()));
        self
    }

    /// Write `contents` to the file `name` in the data directory, with mode
    /// 0600, before the server starts. A relative path in a setting, such as
    /// `ssl_cert_file`, names a file in the data directory.
    pub fn file(mut self, name: &str, contents: impl Into<Vec<u8>>) -> Self {
        self.files.push((name.to_owned(), contents.into()));
        self
    }

    /// Run `sql` in the database `postgres` as the [`SUPERUSER`] after the
    /// server starts, in the order of the calls.
    pub fn init_sql(mut self, sql: &str) -> Self {
        self.init_sql.push(sql.to_owned());
        self
    }

    /// Make an empty cluster with the [`SUPERUSER`] in a new directory, and
    /// start a server on it.
    pub fn start(self) -> Server {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let installation = installation_of(&self.feature);
        // A short directory: the socket path must fit in `sun_path`.
        let root = std::env::temp_dir().join(format!(
            "pgtk-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)
            .unwrap_or_else(|error| panic!("cannot make {}: {error}", root.display()));
        let port = if self.tcp { free_port() } else { PORT };
        let server = Server {
            root,
            installation,
            port,
            tcp: self.tcp,
        };

        let mut initdb = installation.command("initdb");
        initdb
            .arg("-D")
            .arg(server.data_dir())
            .args(["-U", SUPERUSER, "--locale=C", "-E", "UTF8"]);
        match &self.host_auth {
            Some(method) => initdb.args(["--auth-local=trust", &format!("--auth-host={method}")]),
            None => initdb.args(["-A", "trust"]),
        };
        if let Some(password) = &self.password {
            let file = server.root.join("password");
            write(&file, password.as_bytes());
            initdb.arg(format!("--pwfile={}", file.display()));
        }
        run(&mut initdb);
        watch(installation, &server.root, &server.data_dir());

        for (name, contents) in &self.files {
            let path = server.data_dir().join(name);
            write(&path, contents);
            std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))
                .unwrap_or_else(|error| {
                    panic!("cannot set the mode of {}: {error}", path.display())
                });
        }
        if !self.settings.is_empty() {
            let mut conf = std::fs::read_to_string(server.data_dir().join("postgresql.conf"))
                .expect("initdb writes postgresql.conf");
            for (name, value) in &self.settings {
                conf.push_str(&format!("{name} = '{}'\n", value.replace('\'', "''")));
            }
            write(&server.data_dir().join("postgresql.conf"), conf.as_bytes());
        }

        server.start_again();
        for sql in &self.init_sql {
            server.psql(sql);
        }
        server
    }
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
        Self::builder_of(feature).start()
    }

    /// A [`Builder`] for a server of [`FEATURE`].
    pub fn builder() -> Builder {
        Self::builder_of(FEATURE)
    }

    /// A [`Builder`] for a server of the pinned release of the version feature
    /// `feature`.
    pub fn builder_of(feature: &str) -> Builder {
        Builder {
            feature: feature.to_owned(),
            tcp: false,
            host_auth: None,
            password: None,
            settings: Vec::new(),
            files: Vec::new(),
            init_sql: Vec::new(),
        }
    }

    fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    /// The directory of the server's socket, the `host` of a connection.
    pub fn socket_dir(&self) -> &Path {
        &self.root
    }

    /// The port of the server: the port of its TCP address, and the port in
    /// the name of its socket.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The TCP address of a server that [`Builder::tcp`] made.
    ///
    /// # Panics
    ///
    /// Panics when the server listens only on its socket.
    pub fn tcp_address(&self) -> SocketAddr {
        assert!(
            self.tcp,
            "the server listens only on its socket; use Builder::tcp"
        );
        SocketAddr::from((Ipv4Addr::LOCALHOST, self.port))
    }

    /// A connection string for the database `dbname` as the [`SUPERUSER`],
    /// over the socket.
    pub fn connection_string(&self, dbname: &str) -> String {
        format!(
            "host={} port={} user={SUPERUSER} dbname={dbname}",
            self.socket_dir().display(),
            self.port
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
            .args(["-p", &self.port.to_string(), "-U", SUPERUSER, name]));
        self.connection_string(name)
    }

    /// Run `sql` with `psql` in the database `postgres` as the [`SUPERUSER`],
    /// over the socket, and return the output. An error ends the test.
    pub fn psql(&self, sql: &str) -> String {
        run(self
            .installation
            .command("psql")
            .arg("-h")
            .arg(self.socket_dir())
            .args([
                "-p",
                &self.port.to_string(),
                "-U",
                SUPERUSER,
                "-d",
                "postgres",
            ])
            .args(["-X", "-q", "-v", "ON_ERROR_STOP=1", "-c", sql]))
    }

    /// Stop the server, and keep its cluster. [`Server::start_again`] starts it
    /// again.
    pub fn stop(&self) {
        run(self
            .installation
            .command("pg_ctl")
            .arg("-D")
            .arg(self.data_dir())
            .args(["-m", "fast", "-w", "stop"]));
    }

    /// Start the server on its cluster, and wait until it accepts connections.
    pub fn start_again(&self) {
        let listen = if self.tcp { "127.0.0.1" } else { "" };
        run(self
            .installation
            .command("pg_ctl")
            .arg("-D")
            .arg(self.data_dir())
            .arg("-o")
            .arg(format!(
                "-p {} -k {} -c listen_addresses={listen} -c fsync=off -c synchronous_commit=off -c full_page_writes=off",
                self.port,
                self.socket_dir().display()
            ))
            .arg("-l")
            .arg(self.root.join("server.log"))
            .args(["-w", "-t", "120", "start"]));
    }

    /// Whether the server runs and accepts connections (`pg_isready`).
    pub fn is_ready(&self) -> bool {
        self.installation
            .command("pg_isready")
            .arg("-h")
            .arg(self.socket_dir())
            .args(["-p", &self.port.to_string(), "-U", SUPERUSER])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}

/// A TCP port on `127.0.0.1` that no socket uses now.
fn free_port() -> u16 {
    std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("bind a free port on 127.0.0.1")
}

fn write(path: &Path, contents: &[u8]) {
    std::fs::write(path, contents)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
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
    fn a_tcp_server_authenticates_with_its_password() {
        let server = Server::builder()
            .host_auth("scram-sha-256")
            .password("secret")
            .setting("pgtk.note", "it's set")
            .init_sql("CREATE TABLE made_by_init_sql ()")
            .start();
        let address = server.tcp_address();
        let psql = |password: &str| {
            server
                .installation()
                .command("psql")
                .args(["-h", "127.0.0.1", "-p", &address.port().to_string()])
                .args(["-U", SUPERUSER, "-d", "postgres", "-X", "-tA"])
                .args(["-c", "SHOW pgtk.note"])
                .env("PGPASSWORD", password)
                .output()
                .expect("run psql")
        };
        let accepted = psql("secret");
        assert!(accepted.status.success());
        assert_eq!(String::from_utf8_lossy(&accepted.stdout).trim(), "it's set");
        assert!(!psql("wrong").status.success());
        assert!(
            server
                .psql("SELECT 'made' FROM pg_class WHERE relname = 'made_by_init_sql'")
                .contains("made")
        );
    }

    #[test]
    fn a_stopped_server_starts_again() {
        let server = Server::builder().tcp().start();
        assert!(server.is_ready());
        server.stop();
        assert!(!server.is_ready());
        server.start_again();
        assert!(server.is_ready());
    }

    #[test]
    fn a_server_runs_the_release_of_its_feature() {
        let server = Server::start_of("pg16");
        assert_eq!(server.installation().major(), 16);
        server.create_database("probe");
    }
}
