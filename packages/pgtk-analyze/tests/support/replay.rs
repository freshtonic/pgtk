//! Running and comparing corpus files.
//!
//! [`execute_file`] runs a file to move a database's state forward, the way
//! pg_regress runs it, in a fresh session. [`replay_file`] does the same in a
//! fresh session but also asks the Oracle and the Subject about every
//! statement first and compares their answers:
//!
//! 1. Skip it when it is not a statement, when `pgtk-sql` cannot parse it, or
//!    when the Oracle's session is in a failed transaction.
//! 2. If the Subject wants it, reload the Catalog if a statement since the
//!    last load may have changed it, and read the search path.
//! 3. Ask the Oracle (Parse and Describe) and the Subject the same question.
//! 4. Compare, then run the statement.
//! 5. Fold the Session Effect of a statement the server accepted into the
//!    Session the next statement is analysed against.

use std::fmt::Write as _;

use pgtk_analyze::{Cursor, PreparedStatement, ResultDescriptor, SessionEffect, TypeRef};
use pgtk_catalog::Catalog;
use pgtk_sql::ast::Statement;

use super::oracle::{Oracle, OracleOutcome, Server};
use super::plan::PlannedFile;
use super::subject::{Subject, SubjectOutcome};

/// The Session the harness keeps for the file it replays, so that EXECUTE,
/// FETCH and SHOW of a placeholder can be compared.
type AnalysisSession = pgtk_analyze::Session;

/// Why a statement was not compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// A psql meta-command or other non-statement text.
    NotAStatement,
    /// `pgtk-sql` did not parse it, so the Subject never saw it.
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
    /// It is, or embeds, a query that Analysis would resolve.
    pub has_query: bool,
    /// It opens a transaction block.
    pub begins_transaction: bool,
    /// It ends a transaction by rolling it back.
    pub rolls_back: bool,
    /// It loads a shared library, which deletes the placeholders in the
    /// extension's namespace.
    pub loads_library: bool,
    /// Its whole answer is read from the Session: EXECUTE, FETCH and MOVE.
    pub reads_session: bool,
}

/// Render one frozen item as psql would send it to the server: psql's own
/// terminators (`\;`, `\g`, `\gset`, `\crosstabview` and kin) are client
/// syntax the server never sees, and `pgtk-psql` strips them. No variables are
/// bound; the corpus sets its own with `\set`, which nothing models yet.
pub fn render(item: &str) -> Option<String> {
    let rendered = pgtk_psql::render(item, &pgtk_psql::Variables::new()).ok()?;
    Some(rendered.sql().trim().to_owned())
}

/// Parse one statement with `pgtk-sql`, dropping the terminator the frozen
/// range keeps, the way `pgtk-sql`'s own harness does. `None` when `pgtk-sql`
/// rejects it.
pub fn parse(sql: &str) -> Option<Shape> {
    let lexed = pgtk_sql::lex(sql);
    if lexed.errors().next().is_some() {
        return None;
    }
    let terminator = lexed
        .tokens()
        .last()
        .filter(|token| token.kind() == pgtk_sql::TokenKind::SEMI)
        .map(|token| token.span());
    let lexed = match terminator {
        None => lexed,
        Some(terminator) => {
            let mut builder = pgtk_sql::LexBuilder::new(sql);
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
        has_query: has_query(statement),
        begins_transaction: matches!(
            statement,
            Statement::Begin(_) | Statement::StartTransaction(_)
        ),
        rolls_back: matches!(statement, Statement::Rollback(_) | Statement::Abort(_)),
        loads_library: matches!(statement, Statement::Load(_)),
        reads_session: matches!(
            statement,
            Statement::Execute(_) | Statement::Fetch(_) | Statement::Move(_)
        ),
    })
}

/// Does running this statement leave the catalog as it was? Everything not
/// listed is treated as DDL, which is the safe direction.
fn keeps_catalog(statement: &Statement<'_>) -> bool {
    is_merge(statement)
        || matches!(
            statement,
            Statement::Query(_)
                | Statement::With(_)
                | Statement::Insert(_)
                | Statement::Update(_)
                | Statement::Delete(_)
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

/// MERGE, which the grammar has from 15.
fn is_merge(statement: &Statement<'_>) -> bool {
    #[cfg(feature = "since-pg15")]
    return matches!(statement, Statement::Merge(_));
    #[cfg(not(feature = "since-pg15"))]
    {
        let _ = statement;
        false
    }
}

/// Statements Analysis resolves: DML, and Utility Statements with a query
/// inside them.
fn has_query(statement: &Statement<'_>) -> bool {
    is_merge(statement)
        || matches!(
            statement,
            Statement::Query(_)
            | Statement::With(_)
            | Statement::Insert(_)
            | Statement::Update(_)
            | Statement::Delete(_)
            | Statement::Explain(_)
            | Statement::Declare(_)
            | Statement::Prepare(_)
            | Statement::Copy(_)
            | Statement::CreateView(_)
            | Statement::CreateMaterializedView(_)
            | Statement::CreateTable(_)
            | Statement::CreateRule(_)
            | Statement::Call(_)
            // These read the Session, and the Session holds Catalog handles,
            // so they need the same Catalog the Session was built against.
            | Statement::Execute(_)
            | Statement::Fetch(_)
            | Statement::Move(_)
        )
}

fn compare(oracle: &OracleOutcome, subject: &SubjectOutcome, shape: Shape) -> Verdict {
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
            } else if shape.reads_session && subject_columns.is_empty() {
                // The Session the harness keeps holds only what the
                // statements it replays made, so it misses a cursor a
                // function or a plpgsql body opened, and one whose columns
                // come from a table that was dropped, since a Catalog
                // Handle names a live relation. Analysis then answers with
                // the empty descriptor `UtilityTupleDescriptor` gives a
                // name it does not find, while the server answers from the
                // entry it holds.
                Verdict::KnownDivergence
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
    /// What the server's session holds: its prepared statements, its open
    /// cursors and its placeholders. The harness keeps it by folding in the
    /// Session Effect of every statement that ran.
    session: AnalysisSession,
    /// A transaction block is open.
    in_transaction: bool,
    /// The held cursors a DECLARE opened inside the transaction block that
    /// is open now. A ROLLBACK closes them as well as the cursors that are
    /// not held, since it rolls back the transaction that made them.
    held_here: Vec<String>,
}

impl Session {
    fn open(server: &Server) -> Self {
        let oracle = server
            .session()
            .unwrap_or_else(|error| panic!("cannot open a session: {error}"));
        Self {
            oracle,
            catalog: None,
            catalog_dirty: true,
            catalog_loads: 0,
            fingerprint: None,
            session: AnalysisSession::new(),
            in_transaction: false,
            held_here: Vec::new(),
        }
    }

    /// Reload the Catalog if a statement since the last load may have
    /// changed it. Done before the Oracle is asked anything, because a Parse
    /// failure inside an open transaction aborts that transaction, and the
    /// loader's queries would fail after it.
    fn ensure_catalog(&mut self) -> Result<(), pgtk_catalog::LoadError> {
        if self.catalog_dirty || self.catalog.is_none() {
            let loaded = self.oracle.catalog()?;
            if let Some(old) = &self.catalog {
                self.session = remap_session(&self.session, old, &loaded.catalog);
            }
            self.catalog = Some(loaded.catalog);
            self.catalog_dirty = false;
            self.catalog_loads += 1;
            self.fingerprint = self.oracle.catalog_fingerprint().ok();
        }
        Ok(())
    }

    /// Run the statement, and say whether the server accepted it. A
    /// statement the harness does not run, or one the server rejected,
    /// changes nothing in the session.
    fn run(&mut self, shape: Shape, sql: &str) -> bool {
        let mut ran = false;
        if shape.runnable {
            match self.oracle.execute(sql) {
                Ok(accepted) => ran = accepted,
                Err(error) => self.recover(&error),
            }
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
        ran
    }

    /// Fold in what the statement did to the session, once it has run.
    /// `rolled_back` says the statement ended the transaction by rolling it
    /// back, which the Session Effect itself does not distinguish.
    fn apply(&mut self, effect: &SessionEffect, rolled_back: bool) {
        match effect {
            SessionEffect::Declare(cursor) => {
                if self.in_transaction && cursor.hold {
                    self.held_here.push(cursor.name.clone());
                }
                self.session.apply(effect);
            }
            SessionEffect::Close(name) => {
                self.held_here.retain(|held| held != name);
                self.session.apply(effect);
            }
            SessionEffect::CloseAll => {
                self.held_here.clear();
                self.session.apply(effect);
            }
            SessionEffect::TransactionEnd => self.end_transaction(rolled_back),
            _ => self.session.apply(effect),
        }
    }

    /// LOAD of an extension deletes every placeholder in the extension's
    /// namespace, so the harness forgets the placeholders it holds.
    fn forget_placeholders(&mut self) {
        let mut session = AnalysisSession::new();
        for prepared in self.session.prepared_statements() {
            session.add_prepared(prepared.clone());
        }
        for cursor in self.session.cursors() {
            session.add_cursor(cursor.clone());
        }
        self.session = session;
    }

    /// End the transaction the way the server ended it.
    fn end_transaction(&mut self, rolled_back: bool) {
        for name in std::mem::take(&mut self.held_here) {
            if rolled_back {
                self.session.apply(&SessionEffect::Close(name));
            }
        }
        self.session.end_transaction();
        self.in_transaction = false;
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
            // A new session holds no prepared statement and no cursor.
            self.session = AnalysisSession::new();
            self.in_transaction = false;
            self.held_here.clear();
        }
        self.catalog_dirty = true;
    }
}

/// Run every runnable statement of `file` on `server`, in a fresh session,
/// without comparing anything.
pub fn execute_file(server: &Server, file: &PlannedFile) {
    let mut session = Session::open(server);
    for item in &file.items {
        if let (Some(sql), Some(shape)) = (&item.sql, item.shape) {
            session.run(shape, sql);
        }
    }
}

/// Replay `file` on `server`, in a fresh session, comparing the Oracle and
/// the Subject on every statement.
pub fn replay_file(server: &Server, file: &PlannedFile, subject: &mut dyn Subject) -> FileResult {
    let mut session = Session::open(server);
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
        // A transaction that ends while it is failed rolls everything back.
        match session.oracle.in_failed_transaction() {
            Ok(false) => session.end_transaction(true),
            Ok(true) => {}
            Err(error) => session.recover(&error),
        }
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
    let answer = subject.analyze(catalog, &search_path, sql, &session.session);
    let subject_outcome = answer.outcome;
    let verdict = compare(&oracle_outcome, &subject_outcome, shape);
    if let SubjectOutcome::Unsupported(reason) = &subject_outcome {
        *unsupported.entry(reason.clone()).or_default() += 1;
    }

    if verdict == Verdict::Disagree {
        let _ = writeln!(
            report,
            "--- {file} item {index}\n{sql}\n  oracle:  {oracle_outcome:?}\n  subject: {subject_outcome:?}\n"
        );
    }

    // The statement changes the session only when the server accepted it.
    if session.run(shape, sql) {
        if shape.begins_transaction {
            session.in_transaction = true;
        }
        if shape.loads_library {
            session.forget_placeholders();
        }
        if let Some(effect) = &answer.effect {
            session.apply(effect, shape.rolls_back);
        }
    }
    verdict
}

/// Move a Session from one Catalog to another.
///
/// A Catalog handle is an index into the Catalog that issued it, so the
/// Session the harness keeps must be rewritten whenever it reloads the
/// Catalog. An entry that names a type or a relation the new Catalog does
/// not have is dropped: whatever dropped that entity made the prepared
/// statement or the cursor useless anyway.
fn remap_session(session: &AnalysisSession, old: &Catalog, new: &Catalog) -> AnalysisSession {
    let mut out = AnalysisSession::new();
    for prepared in session.prepared_statements() {
        let types = prepared
            .param_types
            .iter()
            .map(|&ty| remap_type(ty, old, new))
            .collect::<Option<Vec<_>>>();
        if let (Some(param_types), Some(result)) =
            (types, remap_descriptor(&prepared.result, old, new, true))
        {
            out.add_prepared(PreparedStatement {
                name: prepared.name.clone(),
                param_types,
                result,
                source: prepared.source.clone(),
            });
        }
    }
    for cursor in session.cursors() {
        if let Some(result) = remap_descriptor(&cursor.result, old, new, false) {
            out.add_cursor(Cursor {
                name: cursor.name.clone(),
                result,
                scroll: cursor.scroll,
                hold: cursor.hold,
            });
        }
    }
    for placeholder in session.placeholders() {
        out.add_placeholder(placeholder.clone());
    }
    out
}

/// One Result Descriptor in the terms of another Catalog, or nothing when
/// that Catalog is missing one of the entities it names.
///
/// `forget_dropped_origin` keeps a column whose source table was dropped,
/// without the source. A prepared statement is kept that way, because a
/// Describe of an EXECUTE analyses its query again and the descriptor the
/// PREPARE computed is only compared, column name and type, with the new
/// one. A cursor is dropped instead: its descriptor is the answer, and a
/// Catalog Handle cannot name a table that is gone.
fn remap_descriptor(
    descriptor: &ResultDescriptor,
    old: &Catalog,
    new: &Catalog,
    forget_dropped_origin: bool,
) -> Option<ResultDescriptor> {
    let mut columns = Vec::with_capacity(descriptor.columns.len());
    for column in &descriptor.columns {
        let origin = match column.origin {
            None => None,
            Some((relation, attnum)) => match new.relation_by_oid(old[relation].oid) {
                Some(relation) => Some((relation, attnum)),
                None if forget_dropped_origin => None,
                None => return None,
            },
        };
        columns.push(pgtk_analyze::ResultColumn {
            name: column.name.clone(),
            ty: remap_type(column.ty, old, new)?,
            typmod: column.typmod,
            origin,
        });
    }
    let params = descriptor
        .params
        .iter()
        .map(|&ty| remap_type(ty, old, new))
        .collect::<Option<Vec<_>>>()?;
    Some(ResultDescriptor { columns, params })
}

/// One type handle in the terms of another Catalog.
fn remap_type(ty: TypeRef, old: &Catalog, new: &Catalog) -> Option<TypeRef> {
    new.type_by_oid(old[ty].oid)
}
