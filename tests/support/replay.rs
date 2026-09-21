//! Running and comparing corpus files.
//!
//! [`execute_file`] runs a file to move a database's state forward, the way
//! pg_regress runs it, in a fresh session. [`replay_file`] does the same in a
//! fresh session but also asks the Oracle and the Subject about every
//! statement first and compares their answers:
//!
//! 1. Skip it when it is not a statement, when `pg-sql` cannot parse it, or
//!    when the Oracle's session is in a failed transaction.
//! 2. If the Subject wants it, reload the Catalog if a statement since the
//!    last load may have changed it, and read the search path.
//! 3. Ask the Oracle (Parse and Describe) and the Subject the same question.
//! 4. Compare, then run the statement.

use std::fmt::Write as _;

use pg_catalog::Catalog;
use pg_sql::ast::Statement;

use super::oracle::{Oracle, OracleOutcome, Server};
use super::plan::PlannedFile;
use super::subject::{Subject, SubjectOutcome};

/// Why a statement was not compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// A psql meta-command or other non-statement text.
    NotAStatement,
    /// `pg-sql` did not parse it, so the Subject never saw it.
    ParseFailed,
    /// The Oracle's session was in a failed transaction.
    AbortedTransaction,
    /// The Catalog could not be loaded from the Oracle's session.
    CatalogUnavailable,
}

/// The result of comparing one statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Skipped(SkipReason),
    Unsupported,
    Agree,
    /// The Oracle rejected for a reason this crate never checks by design:
    /// literal text (SQLSTATE class 22) or scanner-level syntax the parser
    /// accepts (`42601`, `0A000`), while the Subject analysed the statement.
    KnownDivergence,
    Disagree,
}

/// The counts for one file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub items: usize,
    pub skipped: usize,
    pub skipped_not_a_statement: usize,
    pub skipped_parse_failed: usize,
    pub skipped_aborted_transaction: usize,
    pub skipped_catalog_unavailable: usize,
    pub unsupported: usize,
    pub agree: usize,
    pub known_divergence: usize,
    pub disagree: usize,
}

impl Counts {
    fn record(&mut self, verdict: &Verdict) {
        self.items += 1;
        match verdict {
            Verdict::Skipped(reason) => {
                self.skipped += 1;
                match reason {
                    SkipReason::NotAStatement => self.skipped_not_a_statement += 1,
                    SkipReason::ParseFailed => self.skipped_parse_failed += 1,
                    SkipReason::AbortedTransaction => self.skipped_aborted_transaction += 1,
                    SkipReason::CatalogUnavailable => self.skipped_catalog_unavailable += 1,
                }
            }
            Verdict::Unsupported => self.unsupported += 1,
            Verdict::Agree => self.agree += 1,
            Verdict::KnownDivergence => self.known_divergence += 1,
            Verdict::Disagree => self.disagree += 1,
        }
    }
}

/// The counts for one file plus a human-readable report of every
/// disagreement.
#[derive(Debug)]
pub struct FileResult {
    pub counts: Counts,
    pub report: String,
    pub catalog_loads: usize,
}

/// What the replay needs to know about a statement's syntax.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    /// Running it leaves the catalog as it was.
    pub keeps_catalog: bool,
    /// The harness may run it.
    pub runnable: bool,
    /// It creates, alters or drops something cluster-wide.
    pub cluster_ddl: bool,
    /// It is, or embeds, a query that Analysis would resolve.
    pub has_query: bool,
}

/// Render one frozen item as psql would send it to the server: psql's own
/// terminators (`\;`, `\g`, `\gset`, `\crosstabview` and kin) are client
/// syntax the server never sees, and `pg-psql` strips them. No variables are
/// bound; the corpus sets its own with `\set`, which nothing models yet.
pub fn render(item: &str) -> Option<String> {
    let rendered = pg_psql::render(item, &pg_psql::Variables::new()).ok()?;
    Some(rendered.sql().trim().to_owned())
}

/// Parse one statement with `pg-sql`, dropping the terminator the frozen
/// range keeps, the way `pg-sql`'s own harness does. `None` when `pg-sql`
/// rejects it.
pub fn parse(sql: &str) -> Option<Shape> {
    let lexed = pg_sql::lex(sql);
    if lexed.errors().next().is_some() {
        return None;
    }
    let terminator = lexed
        .tokens()
        .last()
        .filter(|token| token.kind() == pg_sql::TokenKind::SEMI)
        .map(|token| token.span());
    let lexed = match terminator {
        None => lexed,
        Some(terminator) => {
            let mut builder = pg_sql::LexBuilder::new(sql);
            for token in lexed.tokens() {
                if token.span() != terminator {
                    builder.append(token.kind(), token.span()).ok()?;
                }
            }
            builder.finish()
        }
    };
    let mut input = lexed.input();
    let parsed = Statement::parse_without_spans(&mut input).ok()?;
    if !input.is_eof() {
        return None;
    }
    let statement = parsed.ast();
    Some(Shape {
        keeps_catalog: keeps_catalog(statement),
        runnable: runnable(statement),
        cluster_ddl: cluster_ddl(statement),
        has_query: has_query(statement),
    })
}

/// Does running this statement leave the catalog as it was? Everything not
/// listed is treated as DDL, which is the safe direction.
fn keeps_catalog(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::Query(_)
            | Statement::With(_)
            | Statement::Insert(_)
            | Statement::Update(_)
            | Statement::Delete(_)
            | Statement::Merge(_)
            | Statement::Explain(_)
            | Statement::Copy(_)
            | Statement::Truncate(_)
            | Statement::Vacuum(_)
            | Statement::Analyze(_)
            | Statement::Checkpoint(_)
            | Statement::Cluster(_)
            | Statement::Reindex(_)
            | Statement::Refresh(_)
            | Statement::Comment(_)
            | Statement::SecurityLabel(_)
            | Statement::Grant(_)
            | Statement::Revoke(_)
            | Statement::Lock(_)
            | Statement::Notify(_)
            | Statement::Listen(_)
            | Statement::Unlisten(_)
            | Statement::Show(_)
            | Statement::Fetch(_)
            | Statement::Move(_)
            | Statement::Close(_)
            | Statement::Declare(_)
            | Statement::Savepoint(_)
            | Statement::Release(_)
            | Statement::Set(_)
            | Statement::Reset(_)
            | Statement::SetConstraints(_)
            | Statement::Discard(_)
            | Statement::Load(_)
            | Statement::Prepare(_)
            | Statement::Deallocate(_)
            | Statement::Execute(_)
            | Statement::Begin(_)
            | Statement::StartTransaction(_)
    )
}

/// `COPY ... FROM STDIN` would put the connection into copy mode, and every
/// other `COPY` names a file the server cannot reach. None is run.
fn runnable(statement: &Statement<'_>) -> bool {
    !matches!(statement, Statement::Copy(_))
}

/// Statements that touch objects shared by every database in the cluster.
/// Role membership grants are cluster-wide too, so `GRANT` and `REVOKE`
/// count, which is conservative for object grants.
fn cluster_ddl(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::CreateRole(_)
            | Statement::CreateUser(_)
            | Statement::CreateGroup(_)
            | Statement::CreateTablespace(_)
            | Statement::CreateDatabase(_)
            | Statement::CreateSubscription(_)
            | Statement::DropRole(_)
            | Statement::DropUser(_)
            | Statement::DropGroup(_)
            | Statement::DropTablespace(_)
            | Statement::DropDatabase(_)
            | Statement::DropSubscription(_)
            | Statement::DropOwned(_)
            | Statement::AlterRole(_)
            | Statement::AlterUser(_)
            | Statement::AlterGroup(_)
            | Statement::AlterTablespace(_)
            | Statement::AlterDatabase(_)
            | Statement::AlterSubscription(_)
            | Statement::AlterDefaultPrivileges(_)
            | Statement::Reassign(_)
            | Statement::Grant(_)
            | Statement::Revoke(_)
    )
}

/// Statements Analysis resolves: DML, and Utility Statements with a query
/// inside them.
fn has_query(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::Query(_)
            | Statement::With(_)
            | Statement::Insert(_)
            | Statement::Update(_)
            | Statement::Delete(_)
            | Statement::Merge(_)
            | Statement::Explain(_)
            | Statement::Declare(_)
            | Statement::Prepare(_)
            | Statement::Copy(_)
            | Statement::CreateView(_)
            | Statement::CreateMaterializedView(_)
            | Statement::CreateTable(_)
            | Statement::CreateRule(_)
            | Statement::Call(_)
    )
}

fn compare(oracle: &OracleOutcome, subject: &SubjectOutcome) -> Verdict {
    match (oracle, subject) {
        (_, SubjectOutcome::Unsupported(_)) => Verdict::Unsupported,
        (
            OracleOutcome::Accepted { params, columns },
            SubjectOutcome::Analysed {
                params: subject_params,
                columns: subject_columns,
                ..
            },
        ) => {
            let same_columns = columns.len() == subject_columns.len()
                && columns.iter().zip(subject_columns).all(|(a, b)| {
                    a.name == b.name
                        && a.type_oid == b.type_oid
                        && a.typmod == b.typmod
                        && a.table_oid == b.table_oid
                        && a.column_id == b.column_id
                });
            if params == subject_params && same_columns {
                Verdict::Agree
            } else {
                Verdict::Disagree
            }
        }
        // Rejections raised outside parse analysis while the Subject
        // analysed: data errors from input functions (class 22, 54 and 55,
        // and 42602), domain checks folded at parse time (class 23),
        // scanner-level syntax (42601, 0A000), the rewriter's
        // policy recursion and restricted-view checks (42P17, 55000), and
        // catalog lookups done by reg* input functions.
        (OracleOutcome::Rejected(error), SubjectOutcome::Analysed { reg_literals, .. })
            if error.sqlstate.starts_with("22")
                || error.sqlstate.starts_with("23")
                || error.sqlstate.starts_with("54")
                || error.sqlstate.starts_with("55")
                || matches!(
                    error.sqlstate.as_str(),
                    "42601" | "42602" | "0A000" | "42P17" | "42501" | "42P21"
                )
                || (*reg_literals
                    && matches!(
                        error.sqlstate.as_str(),
                        "42883"
                            | "42P01"
                            | "42704"
                            | "42P03"
                            | "42725"
                            | "42P05"
                            | "42501"
                            | "3F000"
                    )) =>
        {
            Verdict::KnownDivergence
        }
        (
            OracleOutcome::Rejected(error),
            SubjectOutcome::Diagnostic {
                sqlstate,
                message,
                position,
            },
        ) => {
            if &error.sqlstate == sqlstate
                && &error.message == message
                && &error.position == position
            {
                Verdict::Agree
            } else {
                Verdict::Disagree
            }
        }
        _ => Verdict::Disagree,
    }
}

/// One session's replay state.
struct Session {
    oracle: Oracle,
    catalog: Option<Catalog>,
    catalog_dirty: bool,
    catalog_loads: usize,
    /// The catalog fingerprint at the last load.
    fingerprint: Option<String>,
}

impl Session {
    fn open(server: &Server, database: &str) -> Self {
        let oracle = server
            .session(database)
            .unwrap_or_else(|error| panic!("cannot open a session in {database}: {error}"));
        Self {
            oracle,
            catalog: None,
            catalog_dirty: true,
            catalog_loads: 0,
            fingerprint: None,
        }
    }

    /// Reload the Catalog if a statement since the last load may have
    /// changed it. Done before the Oracle is asked anything, because a Parse
    /// failure inside an open transaction aborts that transaction, and the
    /// loader's queries would fail after it.
    fn ensure_catalog(&mut self) -> Result<(), pg_catalog::LoadError> {
        if self.catalog_dirty || self.catalog.is_none() {
            let loaded = self.oracle.catalog()?;
            self.catalog = Some(loaded.catalog);
            self.catalog_dirty = false;
            self.catalog_loads += 1;
            self.fingerprint = self.oracle.catalog_fingerprint().ok();
        }
        Ok(())
    }

    fn run(&mut self, shape: Shape, sql: &str) {
        if shape.runnable
            && let Err(error) = self.oracle.execute(sql)
        {
            self.recover(&error);
        }
        if !shape.keeps_catalog {
            self.catalog_dirty = true;
        } else if !self.catalog_dirty {
            // A statement that keeps the catalog by its kind may still
            // change it: SELECT INTO, or a function that runs DDL.
            match self.oracle.catalog_fingerprint() {
                Ok(fingerprint) => {
                    if self.fingerprint.as_ref() != Some(&fingerprint) {
                        self.catalog_dirty = true;
                    }
                }
                Err(error) => self.recover(&error),
            }
        }
    }

    /// Something went wrong talking to the Oracle. When the connection is
    /// gone, which some corpus statements cause on purpose, reconnect; the
    /// session state is lost either way, so the catalog must be reloaded.
    fn recover(&mut self, error: &postgres::Error) {
        if self.oracle.is_closed() {
            eprintln!("oracle connection lost ({error}); reconnecting");
            self.oracle
                .reconnect()
                .unwrap_or_else(|error| panic!("cannot reconnect to the oracle: {error}"));
        }
        self.catalog_dirty = true;
    }
}

/// Run every runnable statement of `file` in `database`, in a fresh session,
/// without comparing anything.
pub fn execute_file(server: &Server, database: &str, file: &PlannedFile) {
    let mut session = Session::open(server, database);
    for item in &file.items {
        if let (Some(sql), Some(shape)) = (&item.sql, item.shape) {
            session.run(shape, sql);
        }
    }
}

/// Replay `file` in `database`, in a fresh session, comparing the Oracle and
/// the Subject on every statement.
pub fn replay_file(
    server: &Server,
    database: &str,
    file: &PlannedFile,
    subject: &mut dyn Subject,
) -> FileResult {
    let mut session = Session::open(server, database);
    let mut counts = Counts::default();
    let mut report = String::new();
    let mut unsupported: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    for (index, item) in file.items.iter().enumerate() {
        let verdict = match (&item.sql, item.shape) {
            (Some(sql), Some(shape)) => replay_one(
                &mut session,
                subject,
                sql,
                shape,
                &mut report,
                &mut unsupported,
                &file.name,
                index,
            ),
            (Some(_), None) => Verdict::Skipped(SkipReason::ParseFailed),
            (None, _) => Verdict::Skipped(SkipReason::NotAStatement),
        };
        counts.record(&verdict);
    }
    if !unsupported.is_empty() {
        let _ = writeln!(report, "--- unsupported in {}", file.name);
        for (reason, n) in &unsupported {
            let _ = writeln!(report, "  {n:5}  {reason}");
        }
    }

    FileResult {
        counts,
        report,
        catalog_loads: session.catalog_loads,
    }
}

#[allow(clippy::too_many_arguments)]
fn replay_one(
    session: &mut Session,
    subject: &mut dyn Subject,
    sql: &str,
    shape: Shape,
    report: &mut String,
    unsupported: &mut std::collections::BTreeMap<String, usize>,
    file: &str,
    index: usize,
) -> Verdict {
    let aborted = match session.oracle.in_failed_transaction() {
        Ok(aborted) => aborted,
        Err(error) => {
            session.recover(&error);
            false
        }
    };
    if aborted {
        // Still run it: it may be the ROLLBACK that ends the failed transaction.
        session.run(shape, sql);
        return Verdict::Skipped(SkipReason::AbortedTransaction);
    }

    // A statement the Subject reads the Catalog for gets a fresh one and
    // the current search path; any other is analysed against whatever
    // Catalog the session has, loading one only when it has none yet.
    let wants = subject.wants(shape);
    let load = if wants || session.catalog.is_none() {
        session.ensure_catalog()
    } else {
        Ok(())
    };
    if let Err(error) = load {
        eprintln!("{file} item {index}: cannot load the catalog: {error}");
        session.run(shape, sql);
        return Verdict::Skipped(SkipReason::CatalogUnavailable);
    }
    let search_path = if wants {
        match session.oracle.search_path() {
            Ok(path) => path,
            Err(error) => {
                // A failed transaction that the probe did not see, or a lost
                // connection: either way this statement is not compared.
                session.recover(&error);
                session.run(shape, sql);
                return Verdict::Skipped(SkipReason::AbortedTransaction);
            }
        }
    } else {
        Vec::new()
    };
    let oracle_outcome = match session.oracle.describe(sql) {
        Ok(outcome) => outcome,
        Err(error) => {
            session.recover(&error);
            if !wants {
                session.run(shape, sql);
            }
            return Verdict::Skipped(SkipReason::AbortedTransaction);
        }
    };
    let catalog = session.catalog.as_ref().expect("catalog loaded above");
    let subject_outcome = subject.analyze(catalog, &search_path, sql);
    let verdict = compare(&oracle_outcome, &subject_outcome);
    if let SubjectOutcome::Unsupported(reason) = &subject_outcome {
        *unsupported.entry(reason.clone()).or_default() += 1;
    }

    if verdict == Verdict::Disagree {
        let _ = writeln!(
            report,
            "--- {file} item {index}\n{sql}\n  oracle:  {oracle_outcome:?}\n  subject: {subject_outcome:?}\n"
        );
    }

    session.run(shape, sql);
    verdict
}
