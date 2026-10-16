//! The Oracle: a live PostgreSQL server asked, through the extended protocol,
//! what it makes of one statement. A Parse plus Describe is the closest thing
//! to `parse_analyze` a stock server exposes: it reports acceptance with the
//! parameter and result column types, or the Diagnostic analysis raised.
//!
//! [`Server`] is one server the harness started. [`Oracle`] is one session
//! in its `regression` database.

use pgtk_catalog::Loaded;
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

/// The superuser every cluster the harness makes has.
pub const USER: &str = "oracle";

/// The database the corpus runs in, as pg_regress names it.
pub const DATABASE: &str = "regression";

/// One server the harness started, reached at a port on this host.
pub struct Server {
    config: postgres::Config,
}

impl Server {
    pub fn at_port(port: u16) -> Self {
        let mut config = postgres::Config::new();
        config
            .host("127.0.0.1")
            .port(port)
            .user(USER)
            .dbname(DATABASE);
        Self { config }
    }

    /// Open a session in the `regression` database with the pg_regress
    /// settings applied.
    pub fn session(&self) -> Result<Oracle, postgres::Error> {
        let client = Oracle::open(&self.config)?;
        Ok(Oracle {
            config: self.config.clone(),
            client,
        })
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

    /// Whether the server has ended the session.
    pub fn is_closed(&self) -> bool {
        self.client.is_closed()
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
    /// the server's state forward. `false` says the server rejected it,
    /// which is not a harness failure; only a broken connection is. The
    /// harness reads the answer to know whether the statement changed the
    /// session, since a statement that fails changes nothing.
    pub fn execute(&mut self, sql: &str) -> Result<bool, postgres::Error> {
        match self.client.simple_query(sql) {
            Ok(_) => Ok(true),
            Err(error) if error.as_db_error().is_some() => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// The effective search path, with `$user` resolved, missing schemas
    /// dropped, and the session's temporary schema and `pg_catalog` in the
    /// places PostgreSQL puts them, as `current_schemas(true)` reports it.
    pub fn search_path(&mut self) -> Result<Vec<String>, postgres::Error> {
        let row = self.client.query_one("SELECT current_schemas(true)", &[])?;
        Ok(row.get::<_, Vec<String>>(0))
    }

    /// A cheap summary of the catalogs the loader reads, to notice a
    /// change made by a statement whose kind does not announce DDL.
    pub fn catalog_fingerprint(&mut self) -> Result<String, postgres::Error> {
        let row = self.client.query_one(
            "SELECT (SELECT count(*) || ':' || coalesce(max(oid), 0) FROM pg_catalog.pg_class)
                 || '/' || (SELECT count(*) || ':' || coalesce(max(oid), 0) FROM pg_catalog.pg_type)
                 || '/' || (SELECT count(*) || ':' || coalesce(max(oid), 0) FROM pg_catalog.pg_proc)
                 || '/' || (SELECT count(*) || ':' || coalesce(max(oid), 0) FROM pg_catalog.pg_operator)
                 || '/' || (SELECT count(*) || ':' || coalesce(sum(atttypid::bigint), 0) FROM pg_catalog.pg_attribute)
                 || '/' || (SELECT count(*) || ':' || coalesce(max(oid), 0) FROM pg_catalog.pg_namespace)",
            &[],
        )?;
        Ok(row.get::<_, String>(0))
    }

    /// Load a Catalog from the session, seeing whatever the session sees,
    /// uncommitted DDL included.
    pub fn catalog(&mut self) -> Result<Loaded, pgtk_catalog::LoadError> {
        pgtk_catalog::load(&mut self.client)
    }
}
