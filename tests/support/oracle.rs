//! The Oracle: a live PostgreSQL server asked, through the extended protocol,
//! what it makes of one statement. A Parse plus Describe is the closest thing
//! to `parse_analyze` a stock server exposes: it reports acceptance with the
//! parameter and result column types, or the Diagnostic analysis raised.
//!
//! [`Server`] is an administrative connection that makes and drops
//! databases. [`Oracle`] is one session in one database.

use pg_catalog::Loaded;
use postgres::error::ErrorPosition;
use postgres::{Client, NoTls};

/// One result column as Describe reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleColumn {
    pub name: String,
    pub type_oid: u32,
    pub typmod: i32,
    pub table_oid: Option<u32>,
    pub column_id: Option<i16>,
}

/// The Diagnostic the server raised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleError {
    pub sqlstate: String,
    pub message: String,
    pub position: Option<u32>,
    pub detail: Option<String>,
    pub hint: Option<String>,
}

/// What the server said about one statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OracleOutcome {
    Accepted {
        params: Vec<u32>,
        columns: Vec<OracleColumn>,
    },
    Rejected(OracleError),
}

/// The session settings pg_regress applies, so message text, date literals
/// and the like match the expected output files.
const SESSION_SETUP: &str = "
    SET lc_messages TO 'C';
    SET lc_monetary TO 'C';
    SET lc_numeric TO 'C';
    SET lc_time TO 'C';
    SET bytea_output TO 'hex';
    SET timezone_abbreviations TO 'Default';
    SET timezone TO 'America/Los_Angeles';
    SET datestyle TO 'Postgres, MDY';
    SET intervalstyle TO 'postgres_verbose';
    SET statement_timeout TO '60s';
";

/// An administrative connection to the server the URL names.
pub struct Server {
    config: postgres::Config,
    client: Client,
}

impl Server {
    pub fn connect(url: &str) -> Result<Self, postgres::Error> {
        let config: postgres::Config = url.parse()?;
        let client = config.connect(NoTls)?;
        Ok(Self { config, client })
    }

    /// The connection settings, so another thread can open its own
    /// connections with [`Server::from_config`]. A `Client` is `Send` but
    /// not `Sync`, so a `Server` itself is never shared.
    pub fn config(&self) -> &postgres::Config {
        &self.config
    }

    /// A new administrative connection with the same settings.
    pub fn from_config(config: &postgres::Config) -> Result<Self, postgres::Error> {
        let client = config.connect(NoTls)?;
        Ok(Self {
            config: config.clone(),
            client,
        })
    }

    /// Database names matching a `LIKE` pattern.
    pub fn databases_like(&mut self, pattern: &str) -> Vec<String> {
        self.client
            .query(
                "SELECT datname FROM pg_database WHERE datname LIKE $1 ORDER BY datname",
                &[&pattern],
            )
            .expect("list databases")
            .iter()
            .map(|row| row.get(0))
            .collect()
    }

    /// Make `name` as a copy of `template`. The template must have no
    /// sessions open, which is why every session is closed before a snapshot.
    pub fn create_database(&mut self, name: &str, template: &str) {
        self.client
            .batch_execute(&format!(
                "CREATE DATABASE {name} TEMPLATE {template} LOCALE 'C'"
            ))
            .unwrap_or_else(|error| {
                panic!("cannot create database {name} from {template}: {error}")
            });
    }

    /// Drop `name` if it exists, ending any session in it.
    pub fn drop_database(&mut self, name: &str) {
        self.client
            .batch_execute(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
            .unwrap_or_else(|error| panic!("cannot drop database {name}: {error}"));
    }

    /// Open a session in `database` with the pg_regress settings applied.
    pub fn session(&self, database: &str) -> Result<Oracle, postgres::Error> {
        let mut config = self.config.clone();
        config.dbname(database);
        let client = Oracle::open(&config)?;
        Ok(Oracle { config, client })
    }
}

/// One session in one database.
pub struct Oracle {
    config: postgres::Config,
    client: Client,
}

impl Oracle {
    fn open(config: &postgres::Config) -> Result<Client, postgres::Error> {
        let mut client = config.connect(NoTls)?;
        client.batch_execute(SESSION_SETUP)?;
        Ok(client)
    }

    /// Drop the connection and make a new one to the same database. Used
    /// after the server ends the session, which some corpus statements do on
    /// purpose.
    pub fn reconnect(&mut self) -> Result<(), postgres::Error> {
        self.client = Self::open(&self.config)?;
        Ok(())
    }

    /// Is the session inside a failed transaction? PostgreSQL rejects every
    /// statement until it is rolled back, which is not an analysis outcome.
    pub fn in_failed_transaction(&mut self) -> Result<bool, postgres::Error> {
        match self.client.simple_query("SELECT 1") {
            Ok(_) => Ok(false),
            Err(error) => match error.as_db_error() {
                Some(db) if db.code().code() == "25P02" => Ok(true),
                _ => Err(error),
            },
        }
    }

    /// Parse and Describe one statement without running it.
    pub fn describe(&mut self, sql: &str) -> Result<OracleOutcome, postgres::Error> {
        match self.client.prepare(sql) {
            Ok(statement) => Ok(OracleOutcome::Accepted {
                params: statement.params().iter().map(|ty| ty.oid()).collect(),
                columns: statement
                    .columns()
                    .iter()
                    .map(|column| OracleColumn {
                        name: column.name().to_owned(),
                        type_oid: column.type_().oid(),
                        typmod: column.type_modifier(),
                        table_oid: column.table_oid(),
                        column_id: column.column_id(),
                    })
                    .collect(),
            }),
            Err(error) => match error.as_db_error() {
                Some(db) => Ok(OracleOutcome::Rejected(OracleError {
                    sqlstate: db.code().code().to_owned(),
                    message: db.message().to_owned(),
                    position: db.position().and_then(|position| match position {
                        ErrorPosition::Original(offset) => Some(*offset),
                        ErrorPosition::Internal { .. } => None,
                    }),
                    detail: db.detail().map(str::to_owned),
                    hint: db.hint().map(str::to_owned),
                })),
                None => Err(error),
            },
        }
    }

    /// Run one statement through the simple protocol, as psql would, to move
    /// the server's state forward. A server-side error is not a harness
    /// failure; only a broken connection is.
    pub fn execute(&mut self, sql: &str) -> Result<(), postgres::Error> {
        match self.client.simple_query(sql) {
            Ok(_) => Ok(()),
            Err(error) if error.as_db_error().is_some() => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// The effective search path, with `$user` resolved and missing schemas
    /// dropped, as `current_schemas(false)` reports it.
    pub fn search_path(&mut self) -> Result<Vec<String>, postgres::Error> {
        let row = self
            .client
            .query_one("SELECT current_schemas(false)", &[])?;
        Ok(row.get::<_, Vec<String>>(0))
    }

    /// Load a Catalog from the session, seeing whatever the session sees,
    /// uncommitted DDL included.
    pub fn catalog(&mut self) -> Result<Loaded, pg_catalog::LoadError> {
        pg_catalog::load(&mut self.client)
    }
}
