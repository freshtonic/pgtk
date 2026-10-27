//! The strict PostgreSQL server-language document interface.
//!
//! [`parse_sql`] accepts zero or more semicolon-separated PostgreSQL
//! statements — PostgreSQL 17.11 `RAW_PARSE_DEFAULT` input — with an optional
//! final semicolon and no psql-only syntax. Empty statements remain source
//! and provenance occurrences without entering the semantic statement list.
//! A COPY FROM STDIN header is ordinary SQL; the client payload and control
//! line that follow it in a psql script are not, and they are rejected like
//! every other non-SQL region.
//!
//! psql input is rejected here the way any other invalid input is. This
//! grammar mirrors `gram.y`, which contains no psql construct, so there is
//! nothing left in it that could recognise one: a psql script is text the
//! SQL grammar does not accept, and it selects [`SqlParseError::Rejected`]
//! with the failing statement named. To parse one, render it through the
//! `pgtk-psql` crate first — that is what psql itself does before the server
//! ever sees the text.
//!
//! Success is a [`SqlDocument`]: a fully strict, provenance-bearing
//! partition of the complete source. Ordinary invalid input is rejected with
//! a [`SqlRejection`]: the first failing statement island, its strict parse
//! diagnostics, and the framing cause when the statement itself parsed. A
//! rejection cannot contain or convert into a PostgreSQL statement. Fatal
//! failure is limited to violated framing invariants ([`FrameError`]).
//!
//! [`parse_sql`] is the strict parse: it stops at the first rejected
//! statement. [`parse_sql_recovering`] is the recovering parse: after a
//! rejected statement it continues at the next statement boundary, and it
//! gives a [`RecoveredDocument`] with each statement or each rejection, in
//! source order. On text that the strict parse accepts, the two give the same
//! statements.

use std::fmt;

use recursa::framing::ArenaCompleteFrame;
use recursa::{ArenaParsed, Span};

use crate::TokenKind;
use crate::ast::Statement;
use crate::ast::file::SqlDocumentItem;
use crate::{CompletePart, FrameDiagnostic, FrameError, FrameFailure, FrameRejection};

type SqlDocumentItemFamily =
    <SqlDocumentItem<'static> as recursa::__private::ArenaGeneratedParse<'static>>::Family;

/// Parses one strict PostgreSQL document.
///
/// The generated document-framing adapter partitions the source into
/// semicolon-bounded statement islands and right-owned trivia gaps, parses
/// every island strictly, and fails closed on any ordinary failure. A final
/// statement does not need a trailing semicolon. Psql-only syntax — a
/// directive, a send command, a query-buffer escape, COPY payload text, or
/// an interpolation such as `:name` — is not SQL and selects
/// [`SqlParseError::Rejected`]; use the `pgtk-psql` crate to render it to SQL
/// first.
///
/// # Examples
///
/// Parse a document, and read the location of each statement:
///
/// ```
/// use pgtk_sql::ast::Statement;
///
/// let source = "SELECT 1;\nINSERT INTO t VALUES (2);";
/// let document = pgtk_sql::document::parse_sql(source).expect("PostgreSQL accepts both");
///
/// let statements = document.statements();
/// assert_eq!(statements.len(), 2);
/// assert!(matches!(statements[1], Statement::Insert(_)));
///
/// // `items` gives each statement with its span in the source.
/// let spans = document
///     .items()
///     .map(|item| item.source_bounds().expect("a span").range())
///     .collect::<Vec<_>>();
/// assert_eq!(&source[spans[1].clone()], "INSERT INTO t VALUES (2);");
/// ```
///
/// A rejection names the first statement that PostgreSQL rejects, and the
/// location of each error in it:
///
/// ```
/// use pgtk_sql::document::{SqlParseError, parse_sql};
///
/// let source = "SELECT 1; SELECT FROM WHERE;";
/// let Err(SqlParseError::Rejected(rejection)) = parse_sql(source) else {
///     panic!("PostgreSQL rejects the second statement");
/// };
/// assert_eq!(&source[rejection.island().range()], "SELECT FROM WHERE;");
/// let first = &rejection.diagnostics()[0];
/// assert_eq!(&source[first.span().range()], "WHERE");
/// ```
pub fn parse_sql(source: &str) -> Result<SqlDocument<'_>, SqlParseError<'_>> {
    match SqlDocumentItem::frame(source) {
        Err(FrameFailure::Fatal(fatal)) => Err(SqlParseError::Fatal(fatal)),
        Err(FrameFailure::Rejected(rejection)) => {
            Err(SqlParseError::Rejected(SqlRejection(rejection)))
        }
        Ok(frame) => Ok(SqlDocument { frame }),
    }
}

/// A complete, provenance-bearing strict PostgreSQL document.
///
/// Wraps the exact-source [`recursa::framing::ArenaCompleteFrame`] partition:
/// every UTF-8 source byte is owned exactly once by an island extent or a
/// right-owned gap, and
/// [`ArenaCompleteFrame::render_exact`](recursa::framing::ArenaCompleteFrame::render_exact)
/// reproduces the complete source.
#[derive(derive_more::Deref)]
pub struct SqlDocument<'input> {
    /// Exact-source strict partition; [`std::ops::Deref`] target.
    #[deref]
    frame: recursa::framing::ArenaCompleteFrame<'input, SqlDocumentItemFamily>,
}

impl<'input> SqlDocument<'input> {
    /// Iterates every statement item in source order, including empty
    /// statements, with island-bounded occurrence provenance.
    pub fn items(&self) -> impl Iterator<Item = &ArenaParsed<'input, SqlDocumentItemFamily>> {
        self.frame.typed_islands()
    }

    /// Returns the semantic statement list in source order.
    ///
    /// Empty statements stay out of this list; use [`SqlDocument::items`]
    /// for their source and provenance occurrences.
    pub fn statements(&self) -> Vec<&Statement<'_>> {
        self.frame
            .typed_islands()
            .filter_map(|item| item.ast().statement.as_ref())
            .collect()
    }

    /// Iterates the exact source-ownership partition spans in order.
    pub fn part_spans(&self) -> impl Iterator<Item = Span> + '_ {
        self.frame.parts().map(|part| part.span())
    }

    /// Returns end-of-file trivia owned by the document root, if any.
    ///
    /// An empty document owns one zero-width gap; that is not trivia.
    pub fn eof_trivia(&self) -> Option<&'input str> {
        match self.frame.parts().last()? {
            CompletePart::Gap(gap) if !gap.text().is_empty() => Some(gap.text()),
            CompletePart::Gap(_)
            | CompletePart::Island(_)
            | CompletePart::Line(_)
            | CompletePart::Delimited(_)
            | CompletePart::Payload(_) => None,
        }
    }
}

impl fmt::Debug for SqlDocument<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SqlDocument")
            .field("source", &self.frame.source())
            .field("statements", &self.statements())
            .finish_non_exhaustive()
    }
}

/// Rejection of one strict-parse request.
pub enum SqlParseError<'input> {
    /// Ordinary invalid input: the first failing statement island with its
    /// strict diagnostics and optional framing cause.
    Rejected(SqlRejection<'input>),
    /// Violated grammar, framing, partition, plan, or progress invariants.
    Fatal(FrameError<'input>),
}

impl fmt::Display for SqlParseError<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(rejection) => {
                write!(
                    formatter,
                    "invalid SQL document (statement at {}..{}): {}",
                    rejection.island().start(),
                    rejection.island().end(),
                    rejection.0,
                )
            }
            Self::Fatal(fatal) => fatal.fmt(formatter),
        }
    }
}

impl fmt::Debug for SqlParseError<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(rejection) => formatter
                .debug_struct("Rejected")
                .field("island", &rejection.island())
                .field("diagnostics", &rejection.diagnostics().len())
                .field("framing", &rejection.framing().map(FrameDiagnostic::code))
                .finish_non_exhaustive(),
            Self::Fatal(fatal) => fatal.fmt(formatter),
        }
    }
}

impl std::error::Error for SqlParseError<'_> {}

/// The strict rejection of one document at its first failing statement.
///
/// Wraps the [`FrameRejection`]: the rejected statement extent, that
/// statement's strict parse diagnostics with their stable codes, and the
/// framing cause (`RCA5001` missing COPY terminator, `RCA5002` missing `;`)
/// when the statement itself parsed. No authored value is retained, so a
/// rejection cannot contain or convert into a PostgreSQL statement, parsed
/// statement, or authored document.
#[derive(derive_more::Deref)]
pub struct SqlRejection<'input>(FrameRejection<'input>);

// --- Recovering parse --------------------------------------------------------

/// Parses a PostgreSQL document, and continues after each rejected statement.
///
/// This is the recovering parse. [`parse_sql`] is the strict parse: it stops
/// at the first rejected statement. This function gives one result for each
/// statement of the document, in source order: the statement, or the
/// [`RecoveredRejection`] of that statement. The statements after a
/// rejection are parsed as if the rejected statement was not there.
///
/// On a document that the strict parse accepts, this function gives the
/// same items, with the same spans and the same statements.
///
/// # Statement boundaries
///
/// When the strict parse of a statement fails, the parse continues after
/// the next statement boundary. A boundary is a `;` at the top level. These
/// semicolons are not boundaries:
///
/// - a `;` in parentheses, for example in `CREATE RULE … DO ( … ; … )`;
/// - a `;` in a quoted or dollar-quoted string, in a quoted identifier or
///   in a comment (the lexer makes these one token, so they are never a
///   `;` token);
/// - a `;` in the `BEGIN ATOMIC … END` body of a statement that starts
///   with `CREATE`.
///
/// In `gram.y`, a `;` in a statement is in `'(' RuleActionMulti ')'` or in
/// `BEGIN_P ATOMIC routine_body_stmt_list END_P`, so a statement that
/// PostgreSQL accepts never contains a boundary. The body ends at an `END`
/// that comes immediately after `ATOMIC` or after a `;`, because
/// `routine_body_stmt_list` puts a `;` after each statement; the `END` of a
/// `CASE` expression never comes there. PostgreSQL itself has no recovery:
/// its raw parser stops the whole string at the first syntax error. These
/// boundaries are pgtk's own rule. They are near to what psql does when it
/// splits a script (`psqlscan.l` counts parentheses and `BEGIN` … `END` in
/// `CREATE FUNCTION` and `CREATE PROCEDURE`).
///
/// If a rejected statement opens a parenthesis or a `BEGIN ATOMIC` body that
/// does not close, or a string or comment that does not close, no boundary
/// follows it, and its rejection continues to the end of the document.
///
/// psql syntax is not SQL. A psql directive or a COPY payload is rejected
/// like other invalid text, one rejection for each part between
/// boundaries. To parse a psql script, render it through the `pgtk-psql`
/// crate first.
///
/// # Errors
///
/// [`FrameError`] only when a framing invariant fails, as for
/// [`SqlParseError::Fatal`]. A rejected statement is not an error of this
/// function: it is one item of the result.
///
/// # Examples
///
/// ```
/// use pgtk_sql::document::{RecoveredItem, parse_sql_recovering};
///
/// let source = "SELECT 1; SELECT FROM WHERE; SELECT 2;";
/// let document = parse_sql_recovering(source).expect("no framing invariant fails");
///
/// let items = document.items().collect::<Vec<_>>();
/// assert_eq!(items.len(), 3);
/// assert!(matches!(items[0], RecoveredItem::Statement { .. }));
/// assert!(matches!(items[2], RecoveredItem::Statement { .. }));
/// let RecoveredItem::Rejected(rejection) = items[1] else {
///     panic!("PostgreSQL rejects the second statement");
/// };
/// assert_eq!(&source[rejection.span().range()], "SELECT FROM WHERE;");
///
/// // The spans of the diagnostics are spans in the complete source.
/// let first = &rejection.rejection().diagnostics()[0];
/// assert_eq!(&source[first.span().range()], "WHERE");
///
/// // The statements that PostgreSQL accepts, in source order.
/// assert_eq!(document.statements().len(), 2);
/// ```
///
/// A `;` in a `BEGIN ATOMIC` body is not a boundary, also when the body has
/// an error:
///
/// ```
/// use pgtk_sql::document::{RecoveredItem, parse_sql_recovering};
///
/// let source = "CREATE FUNCTION f() RETURNS int LANGUAGE sql \
///               BEGIN ATOMIC SELECT 1; SELECT FROM WHERE; END; SELECT 2;";
/// let document = parse_sql_recovering(source).expect("no framing invariant fails");
/// let spans = document
///     .items()
///     .map(|item| &source[item.span().range()])
///     .collect::<Vec<_>>();
/// assert_eq!(spans[1], "SELECT 2;");
/// assert!(spans[0].ends_with("END;"));
/// ```
pub fn parse_sql_recovering(source: &str) -> Result<RecoveredDocument<'_>, FrameError<'_>> {
    let records = document_records(source);
    let mut entries = Vec::new();
    // The first record of the next window, and the first byte that the
    // windows before it do not own.
    let mut index = 0_usize;
    let mut byte = 0_usize;
    while index < records.len() {
        let end = window_end(source, &records, index);
        let window_end_byte = if end == records.len() {
            source.len()
        } else {
            records[end - 1].span.end() as usize
        };
        match SqlDocumentItem::frame_window(source, byte..window_end_byte) {
            Ok(frame) => entries.push(Entry::Accepted(frame)),
            Err(FrameFailure::Fatal(fatal)) => return Err(fatal),
            Err(FrameFailure::Rejected(rejection)) => {
                let mut start = rejection.island().start() as usize;
                let first = records[index].span.start() as usize;
                if start > first {
                    // The strict framing found complete statements before the
                    // rejected one in this window. It keeps no partial
                    // partition, so frame the statements before the rejection
                    // again. The same records give the same islands.
                    match SqlDocumentItem::frame_window(source, byte..start) {
                        Ok(frame) => entries.push(Entry::Accepted(frame)),
                        Err(FrameFailure::Fatal(fatal)) => return Err(fatal),
                        // Not expected. Keep the window as one rejection.
                        Err(FrameFailure::Rejected(_)) => start = first,
                    }
                }
                // The strict extent ends at the first `;` token at or after
                // the failure, and never after the end of the window. Take
                // the complete window, so that the extent ends at the
                // boundary even when the strict extent ends at a `;` that
                // is not a boundary.
                let rejected_end = window_end_byte;
                let span = Span::new(start, rejected_end)
                    .expect("a rejection extent is a valid span of the source");
                entries.push(Entry::Rejected(RecoveredRejection {
                    rejection: SqlRejection(rejection),
                    span,
                }));
                byte = rejected_end;
                index = records.partition_point(|record| (record.span.start() as usize) < byte);
                continue;
            }
        }
        byte = window_end_byte;
        index = end;
    }
    Ok(RecoveredDocument {
        source,
        entries: entries.into_boxed_slice(),
    })
}

/// Returns the exclusive index of the record that ends the statement window
/// that starts at record `start`: the index after the first boundary `;`, or
/// the number of records when no boundary follows.
///
/// See "Statement boundaries" in [`parse_sql_recovering`].
fn window_end(source: &str, records: &[Record], start: usize) -> usize {
    let creates = records
        .get(start)
        .is_some_and(|record| record.kind == TokenKind::CREATE);
    let mut parentheses = 0_usize;
    let mut bodies = 0_usize;
    let mut previous = None;
    for (index, record) in records.iter().enumerate().skip(start) {
        let current = record.kind;
        match current {
            TokenKind::LPAREN => parentheses += 1,
            // psqlscan.l also ignores a `)` that closes nothing.
            TokenKind::RPAREN => parentheses = parentheses.saturating_sub(1),
            TokenKind::SEMI if parentheses == 0 && bodies == 0 => return index + 1,
            // A quote that does not close is one lexical error token, and the
            // lexer continues after it. For PostgreSQL's scan.l the quoted
            // text continues to the end of the input, so no boundary follows.
            // An unclosed dollar quote or comment is already one error token
            // to the end of the input.
            TokenKind::LexicalError if source[record.span.range()].starts_with(['\'', '"']) => {
                return records.len();
            }
            TokenKind::ATOMIC
                if creates && parentheses == 0 && previous == Some(TokenKind::BEGIN) =>
            {
                bodies += 1;
            }
            TokenKind::END
                if parentheses == 0
                    && bodies > 0
                    && matches!(previous, Some(TokenKind::SEMI | TokenKind::ATOMIC)) =>
            {
                bodies -= 1;
            }
            _ => {}
        }
        previous = Some(current);
    }
    records.len()
}

/// One token of the document, or one lexical error, at its position in the
/// complete source.
#[derive(Clone, Copy)]
struct Record {
    /// The token kind. A lexical error is [`TokenKind::LexicalError`].
    kind: TokenKind,
    /// The span of the token in the complete source.
    span: Span,
}

/// Lexes the complete document, and gives its tokens and its lexical errors
/// in source order.
///
/// This lex finds only the statement windows. Each window frame lexes the
/// text of its window again, with the same tokens, because each window
/// starts and ends at a token boundary.
fn document_records(source: &str) -> Vec<Record> {
    let lexed = crate::lex(source);
    let mut records = lexed
        .tokens()
        .map(|token| Record {
            kind: token.kind(),
            span: token.span(),
        })
        .chain(lexed.errors().map(|error| Record {
            kind: TokenKind::LexicalError,
            span: error.span(),
        }))
        .collect::<Vec<_>>();
    records.sort_by_key(|record| record.span.start());
    records
}

/// The result of a recovering parse: each statement or each rejection of a
/// document, in source order.
///
/// Made by [`parse_sql_recovering`].
pub struct RecoveredDocument<'input> {
    source: &'input str,
    entries: Box<[Entry<'input>]>,
}

/// One part of a recovering parse.
enum Entry<'input> {
    /// The strict frame of statements that the target version accepts.
    Accepted(ArenaCompleteFrame<'input, SqlDocumentItemFamily>),
    /// One rejected statement.
    Rejected(RecoveredRejection<'input>),
}

impl<'input> RecoveredDocument<'input> {
    /// Returns the complete source.
    pub fn source(&self) -> &'input str {
        self.source
    }

    /// Iterates each statement and each rejection, in source order.
    ///
    /// Empty statements are items, as in [`SqlDocument::items`]. The spans
    /// of the items do not overlap. The text between them is whitespace
    /// and comments.
    pub fn items(&self) -> impl Iterator<Item = RecoveredItem<'_, 'input>> {
        self.entries.iter().flat_map(|entry| {
            let (frame, rejection) = match entry {
                Entry::Accepted(frame) => (Some(frame), None),
                Entry::Rejected(rejection) => (None, Some(RecoveredItem::Rejected(rejection))),
            };
            frame
                .into_iter()
                .flat_map(|frame| frame.parts())
                .filter_map(|part| match part {
                    CompletePart::Island(island) => Some(RecoveredItem::Statement {
                        span: island.span(),
                        parsed: island.parsed(),
                    }),
                    CompletePart::Gap(_)
                    | CompletePart::Line(_)
                    | CompletePart::Delimited(_)
                    | CompletePart::Payload(_) => None,
                })
                .chain(rejection)
        })
    }

    /// Returns the statements that the target version accepts, in source
    /// order.
    ///
    /// Empty statements and rejected statements stay out of this list.
    pub fn statements(&self) -> Vec<&Statement<'_>> {
        self.items()
            .filter_map(|item| match item {
                RecoveredItem::Statement { parsed, .. } => parsed.ast().statement.as_ref(),
                RecoveredItem::Rejected(_) => None,
            })
            .collect()
    }

    /// Iterates the rejected statements, in source order.
    pub fn rejections(&self) -> impl Iterator<Item = &RecoveredRejection<'input>> {
        self.entries.iter().filter_map(|entry| match entry {
            Entry::Rejected(rejection) => Some(rejection),
            Entry::Accepted(_) => None,
        })
    }

    /// Returns `true` when the document has no rejected statement, that is,
    /// when [`parse_sql`] accepts it.
    pub fn is_accepted(&self) -> bool {
        self.rejections().next().is_none()
    }
}

impl fmt::Debug for RecoveredDocument<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecoveredDocument")
            .field("source", &self.source)
            .field("items", &self.items().collect::<Vec<_>>())
            .finish()
    }
}

/// One item of a [`RecoveredDocument`].
#[derive(Clone, Copy)]
pub enum RecoveredItem<'doc, 'input> {
    /// A statement that the target version accepts, or an empty statement.
    Statement {
        /// The extent of the statement in the source, with its `;`.
        span: Span,
        /// The statement and its provenance, as [`SqlDocument::items`]
        /// gives them.
        parsed: &'doc ArenaParsed<'input, SqlDocumentItemFamily>,
    },
    /// A statement that the target version rejects.
    Rejected(&'doc RecoveredRejection<'input>),
}

impl RecoveredItem<'_, '_> {
    /// Returns the extent of the item in the source.
    pub fn span(&self) -> Span {
        match self {
            Self::Statement { span, .. } => *span,
            Self::Rejected(rejection) => rejection.span(),
        }
    }
}

impl fmt::Debug for RecoveredItem<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Statement { span, parsed } => formatter
                .debug_struct("Statement")
                .field("span", span)
                .field("statement", &parsed.ast().statement)
                .finish(),
            Self::Rejected(rejection) => rejection.fmt(formatter),
        }
    }
}

/// One rejected statement of a recovering parse.
///
/// The [`SqlRejection`] is the rejection that the strict parse gives for a
/// document that starts at this statement. Its spans are spans in the
/// complete source.
pub struct RecoveredRejection<'input> {
    rejection: SqlRejection<'input>,
    span: Span,
}

impl<'input> RecoveredRejection<'input> {
    /// Returns the extent of the rejected statement: from its first token to
    /// the end of the boundary `;` after it, or to the end of the source
    /// when no boundary follows.
    ///
    /// The extent contains the [`island`](FrameRejection::island) of the
    /// strict rejection. The two are different when the strict extent ends
    /// at a `;` that is not a boundary, for example a `;` in parentheses.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Returns the strict rejection: its diagnostics, and its framing cause.
    pub fn rejection(&self) -> &SqlRejection<'input> {
        &self.rejection
    }
}

impl fmt::Display for RecoveredRejection<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "rejected statement at {}..{}: {}",
            self.span.start(),
            self.span.end(),
            self.rejection.0,
        )
    }
}

impl fmt::Debug for RecoveredRejection<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Rejected")
            .field("span", &self.span)
            .field("island", &self.rejection.island())
            .field("diagnostics", &self.rejection.diagnostics().len())
            .field(
                "framing",
                &self.rejection.framing().map(FrameDiagnostic::code),
            )
            .finish_non_exhaustive()
    }
}
