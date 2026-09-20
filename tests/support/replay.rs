//! Replaying one corpus file against the Oracle and the Subject.
//!
//! For every statement, in file order:
//!
//! 1. Skip it when it is not a statement, when `pg-sql` cannot parse it, or
//!    when the Oracle's session is in a failed transaction.
//! 2. Reload the Catalog if a statement since the last load may have changed
//!    it, and read the search path.
//! 3. Ask the Oracle (Parse and Describe) and the Subject the same question.
//! 4. Compare, then run the statement so the server's state moves forward.

use std::fmt::Write as _;

use pg_catalog::Catalog;
use pg_sql::ast::Statement;

use super::corpus::{CorpusFile, ItemKind};
use super::oracle::{Oracle, OracleOutcome};
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

/// Session state carried from one file to the next.
pub struct Session {
    pub oracle: Oracle,
    catalog: Option<Catalog>,
    catalog_dirty: bool,
    pub catalog_loads: usize,
}

impl Session {
    pub fn new(oracle: Oracle) -> Self {
        Self {
            oracle,
            catalog: None,
            catalog_dirty: true,
            catalog_loads: 0,
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
        }
        Ok(())
    }
}

/// What the replay needs to know about a statement's syntax.
#[derive(Clone, Copy, Debug)]
struct Shape {
    /// Running it leaves the catalog as it was.
    keeps_catalog: bool,
    /// The harness may run it.
    runnable: bool,
}

/// Parse one statement with `pg-sql`, dropping the terminator the frozen
/// range keeps, the way `pg-sql`'s own harness does. `None` when `pg-sql`
/// rejects it.
fn parse(sql: &str) -> Option<Shape> {
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
            | Statement::Commit(_)
            | Statement::End(_)
    )
}

/// `COPY ... FROM STDIN` would put the connection into copy mode, and every
/// other `COPY` names a file the server cannot reach. None is run.
fn runnable(statement: &Statement<'_>) -> bool {
    !matches!(statement, Statement::Copy(_))
}

fn compare(oracle: &OracleOutcome, subject: &SubjectOutcome) -> Verdict {
    match (oracle, subject) {
        (_, SubjectOutcome::Unsupported(_)) => Verdict::Unsupported,
        (
            OracleOutcome::Accepted { params, columns },
            SubjectOutcome::Analysed {
                params: subject_params,
                columns: subject_columns,
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

/// Replay one file.
pub fn replay(session: &mut Session, file: &CorpusFile, subject: &mut dyn Subject) -> FileResult {
    let mut counts = Counts::default();
    let mut report = String::new();
    let loads_before = session.catalog_loads;

    for (index, item) in file.items.iter().enumerate() {
        let sql = file.text_of(item);
        let verdict = replay_one(
            session,
            subject,
            sql,
            item.kind,
            &mut report,
            &file.name,
            index,
        );
        counts.record(&verdict);
    }

    FileResult {
        counts,
        report,
        catalog_loads: session.catalog_loads - loads_before,
    }
}

fn replay_one(
    session: &mut Session,
    subject: &mut dyn Subject,
    sql: &str,
    kind: ItemKind,
    report: &mut String,
    file: &str,
    index: usize,
) -> Verdict {
    if kind != ItemKind::Statement {
        return Verdict::Skipped(SkipReason::NotAStatement);
    }
    let Some(shape) = parse(sql) else {
        return Verdict::Skipped(SkipReason::ParseFailed);
    };

    let aborted = match session.oracle.in_failed_transaction() {
        Ok(aborted) => aborted,
        Err(error) => {
            recover(session, &error);
            false
        }
    };
    if aborted {
        // Still run it: it may be the ROLLBACK that ends the failed transaction.
        run(session, shape, sql);
        return Verdict::Skipped(SkipReason::AbortedTransaction);
    }

    if let Err(error) = session.ensure_catalog() {
        eprintln!("{file} item {index}: cannot load the catalog: {error}");
        run(session, shape, sql);
        return Verdict::Skipped(SkipReason::CatalogUnavailable);
    }
    let search_path = session
        .oracle
        .search_path()
        .unwrap_or_else(|error| panic!("cannot read search_path: {error}"));
    let oracle_outcome = match session.oracle.describe(sql) {
        Ok(outcome) => outcome,
        Err(error) => {
            recover(session, &error);
            return Verdict::Skipped(SkipReason::AbortedTransaction);
        }
    };
    let catalog = session.catalog.as_ref().expect("catalog loaded above");
    let subject_outcome = subject.analyze(catalog, &search_path, sql);
    let verdict = compare(&oracle_outcome, &subject_outcome);

    if verdict == Verdict::Disagree {
        let _ = writeln!(
            report,
            "--- {file} item {index}\n{sql}\n  oracle:  {oracle_outcome:?}\n  subject: {subject_outcome:?}\n"
        );
    }

    run(session, shape, sql);
    verdict
}

fn run(session: &mut Session, shape: Shape, sql: &str) {
    if shape.runnable
        && let Err(error) = session.oracle.execute(sql)
    {
        recover(session, &error);
    }
    if !shape.keeps_catalog {
        session.catalog_dirty = true;
    }
}

/// The connection is gone, which some corpus statements cause on purpose.
/// Reconnect; the session state is lost, so the catalog must be reloaded.
fn recover(session: &mut Session, error: &postgres::Error) {
    eprintln!("oracle connection lost ({error}); reconnecting");
    session
        .oracle
        .reconnect()
        .unwrap_or_else(|error| panic!("cannot reconnect to the oracle: {error}"));
    session.catalog_dirty = true;
}
