//! Substitution: a psql document becomes server SQL text plus a source map.
//!
//! Rendering copies the source verbatim and rewrites only the regions psql
//! itself rewrites. That is what keeps the map exact: everything outside a
//! recorded region is the user's own bytes at a known shift, so a diagnostic
//! over the rendered SQL can always be carried back to the psql source.

use std::collections::BTreeMap;
use std::ops::Range;

use crate::ast::{
    CopyData, Interpolation, MetaCommand, PsqlDocument, PsqlItem, SendCommand, SqlAtom, Terminator,
};
use crate::meta::{is_valid_variable_name, parse_bool};
use crate::scan_state::ScanState;

/// psql variable bindings.
///
/// Names are case-sensitive, as psql's are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Variables(BTreeMap<String, String>);

impl Variables {
    /// An empty binding set: every interpolation will be left verbatim.
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `name`, returning the previous value if there was one.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.0.insert(name.into(), value.into())
    }

    /// Unbinds `name`, returning the previous value if there was one.
    pub fn unset(&mut self, name: &str) -> Option<String> {
        self.0.remove(name)
    }

    /// The value bound to `name`, if any.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Whether `name` is bound. This is what `:{?name}` tests.
    pub fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for Variables {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(bindings: I) -> Self {
        Self(
            bindings
                .into_iter()
                .map(|(name, value)| (name.into(), value.into()))
                .collect(),
        )
    }
}

/// An interpolation left in the rendered SQL because its variable is unbound.
///
/// psql copies such a token through unchanged (`psqlscan.l:744-751` for
/// `:name`, `psqlscan.l:1561-1565` for the quoted forms), and so does this
/// crate: substitution never invents a value. The SQL parse then fails on
/// the surviving colon, which makes an unbound variable a diagnosable
/// condition rather than a silent rewrite — and this list names it directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unbound {
    /// The variable name, without the interpolation's punctuation.
    pub name: String,
    /// The interpolation's extent in the psql source.
    pub source: Range<usize>,
}

/// Where a byte of the rendered SQL came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// The byte is the user's own text, at this psql-source offset.
    Verbatim(usize),
    /// The byte is substituted text. A substituted region has no byte-level
    /// correspondence to the source — the value never appeared there — so
    /// the extent of the interpolation that produced it is the whole truth
    /// available.
    Substituted { start: usize, end: usize },
}

/// Translates offsets in the rendered SQL back to the psql source.
///
/// Outside the recorded regions the two texts differ only by an accumulated
/// shift, so a verbatim offset is exact.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    /// Rewritten regions, disjoint and in ascending rendered order.
    regions: Vec<Region>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Region {
    rendered: Range<usize>,
    source: Range<usize>,
}

impl SourceMap {
    /// Where the byte at `rendered` came from.
    ///
    /// An offset past the end of the rendered SQL continues the final
    /// verbatim run, so `origin(sql.len())` names the end of the source.
    pub fn origin(&self, rendered: usize) -> Origin {
        // The first region that has not already ended at `rendered`.
        let next = self
            .regions
            .partition_point(|region| region.rendered.end <= rendered);
        if let Some(region) = self.regions.get(next)
            && region.rendered.start <= rendered
        {
            return Origin::Substituted {
                start: region.source.start,
                end: region.source.end,
            };
        }
        // Between regions the texts run in step, so the offset is the end of
        // the previous region plus the distance travelled since.
        let (rendered_base, source_base) = match next.checked_sub(1) {
            Some(previous) => {
                let previous = &self.regions[previous];
                (previous.rendered.end, previous.source.end)
            }
            None => (0, 0),
        };
        Origin::Verbatim(source_base + (rendered - rendered_base))
    }

    /// The rewritten regions as `(rendered, source)` pairs, in order.
    pub fn regions(&self) -> impl Iterator<Item = (Range<usize>, Range<usize>)> + '_ {
        self.regions
            .iter()
            .map(|region| (region.rendered.clone(), region.source.clone()))
    }
}

/// A place where psql ends the query buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Boundary {
    /// The extent in the psql source: the `;`, or the backslash and the name
    /// of the command.
    pub source: Range<usize>,
    /// What psql does with the buffer here.
    pub kind: BoundaryKind,
    /// The extent in the rendered SQL: the `;`, the `;` that a send command
    /// renders as, or the empty place of a buffer that a discard command
    /// removes. `None` in an inactive `\if` branch, whose text is not
    /// rendered and whose boundaries send nothing.
    pub rendered: Option<Range<usize>>,
}

/// What psql does with the query buffer at a [`Boundary`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryKind {
    /// A `;` outside parentheses and outside a tracked `BEGIN ... END` block
    /// of a `CREATE FUNCTION` or `CREATE PROCEDURE` (psqlscan.l's `";"`
    /// rule). psql sends the buffer.
    Semicolon,
    /// A send command, such as `\g`. psql sends the buffer.
    Send,
    /// `\r`, or from 18 a command that ends the buffer and sends none of its
    /// text. psql removes the buffer.
    Discard,
}

/// One query string that psql sends to the server: the text of one query
/// buffer.
///
/// psql ends a buffer at each [`Boundary`] and, when it reads a file, at
/// the end of the input. It sends the buffer at a `;` and at a send
/// command, and sends nothing at a discard command or in an inactive `\if`
/// branch. While the buffer is empty, psql does not put whitespace or a
/// `--` comment into it (psqlscan.l's `{whitespace}` rule), so a submission
/// starts at its first other byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    /// The extent in the rendered SQL: from the first byte that psql keeps
    /// to the `;` that ends the buffer, or the `;` that a send command
    /// renders as, or the end of the input.
    pub rendered: Range<usize>,
    /// The extent in the psql source, from the source of the first rendered
    /// byte to the end of the source of the last one. When the buffer ends
    /// at a send command, the extent includes the command and its
    /// arguments.
    pub source: Range<usize>,
    /// The kind of the boundary that ends the buffer: [`BoundaryKind::Semicolon`]
    /// or [`BoundaryKind::Send`], or `None` for the text that remains at the
    /// end of the input.
    pub end: Option<BoundaryKind>,
}

/// Server SQL rendered from a psql document, with the map back to its source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    sql: String,
    map: SourceMap,
    unbound: Vec<Unbound>,
    unevaluated: Vec<Range<usize>>,
    variables: Variables,
    boundaries: Vec<Boundary>,
    complete: bool,
}

impl Rendered {
    /// The rendered server SQL.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// The map from rendered offsets back to the psql source.
    pub fn map(&self) -> &SourceMap {
        &self.map
    }

    /// Interpolations left verbatim because their variables are unbound.
    pub fn unbound(&self) -> &[Unbound] {
        &self.unbound
    }

    /// The source extent of each `\set`, `\if` or `\elif` whose arguments
    /// hold a backquoted shell command.
    ///
    /// psql runs the shell command; pgtk-psql never does. Such a `\set`
    /// changes no variable, and such an `\if` or `\elif` is false.
    pub fn unevaluated(&self) -> &[Range<usize>] {
        &self.unevaluated
    }

    /// The variables at the end of the document: the variables given to
    /// [`crate::render`], changed by each `\set` and `\unset` in an active
    /// branch.
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    /// Each place where psql ends the query buffer, in source order.
    ///
    /// psql does not end the buffer at a `;` inside parentheses or inside a
    /// `BEGIN ... END` block of a `CREATE FUNCTION` or `CREATE PROCEDURE`
    /// (psqlscan.l's `paren_depth` and `begin_depth`), so such a `;` is not
    /// here. A send command ends the buffer at any depth. The text between
    /// two boundaries is one submission: psql sends it to the server as one
    /// query string, even when it holds more than one statement.
    pub fn boundaries(&self) -> &[Boundary] {
        &self.boundaries
    }

    /// Whether the document ends with a complete query buffer.
    ///
    /// psql asks for another line when the input ends inside a string, a
    /// quoted identifier, a dollar-quoted body or a block comment, inside
    /// parentheses, or inside a tracked `BEGIN ... END` block. When psql reads
    /// a file, it then sends the text that it has, so the text after the last
    /// boundary is a submission all the same.
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Each query string that psql sends to the server, in source order.
    ///
    /// The text between two [`boundaries`](Self::boundaries) is one
    /// submission, when psql sends it. psql sends nothing at a discard
    /// command or in an inactive `\if` branch. At the end of the input psql
    /// sends the text that remains, also when the buffer is not complete
    /// ([`is_complete`](Self::is_complete)). A `;` with an empty buffer is a
    /// submission of its own, `;`, as psql sends it.
    pub fn submissions(&self) -> Vec<Submission> {
        let mut submissions = Vec::new();
        let mut start = 0;
        for boundary in &self.boundaries {
            // A boundary of an inactive branch sends nothing, and the buffer
            // goes on.
            let Some(place) = &boundary.rendered else {
                continue;
            };
            if boundary.kind != BoundaryKind::Discard
                && let Some(submission) = self.submission(start..place.end, Some(boundary.kind))
            {
                submissions.push(submission);
            }
            start = place.end;
        }
        if let Some(submission) = self.submission(start..self.sql.len(), None) {
            submissions.push(submission);
        }
        submissions
    }

    /// The submission of the rendered text `extent`, or `None` when psql
    /// keeps none of it.
    fn submission(&self, extent: Range<usize>, end: Option<BoundaryKind>) -> Option<Submission> {
        let start = extent.start + buffer_skip(&self.sql[extent.clone()]);
        if start >= extent.end {
            return None;
        }
        let first = match self.map.origin(start) {
            Origin::Verbatim(offset) => offset,
            Origin::Substituted { start, .. } => start,
        };
        let last = match self.map.origin(extent.end - 1) {
            Origin::Verbatim(offset) => offset + 1,
            Origin::Substituted { end, .. } => end,
        };
        Some(Submission {
            rendered: start..extent.end,
            source: first..last,
            end,
        })
    }

    /// Parses the rendered SQL as a strict PostgreSQL document.
    ///
    /// This is the last step of the pipeline, and the only place pgtk-sql is
    /// asked to read anything: the SQL grammar never sees psql syntax.
    pub fn parse_sql(&self) -> Result<pgtk_sql::SqlDocument<'_>, pgtk_sql::SqlParseError<'_>> {
        pgtk_sql::document::parse_sql(&self.sql)
    }
}

/// The length of the text that psql keeps out of an empty query buffer at
/// the start of `text`: whitespace and `--` comments (psqlscan.l's
/// `{whitespace}` rule, which ECHOes only when the buffer holds text).
fn buffer_skip(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut offset = 0;
    loop {
        while offset < bytes.len()
            && matches!(
                bytes[offset],
                b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x0b'
            )
        {
            offset += 1;
        }
        if bytes[offset..].starts_with(b"--") {
            // psqlscan.l's `{comment}`: `--` and every byte to the line end.
            while offset < bytes.len() && !matches!(bytes[offset], b'\n' | b'\r') {
                offset += 1;
            }
        } else {
            return offset;
        }
    }
}

/// One rewritten span of the source and the text that replaces it.
#[derive(Clone, Debug)]
struct Rewrite {
    source: Range<usize>,
    text: String,
}

/// The state of one `\if` block: `ifState` in
/// `src/include/fe_utils/conditional.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Branch {
    /// The current branch is active.
    True,
    /// No branch has been active yet, and the current one is not.
    False,
    /// The block is in an inactive branch of an outer block, or an earlier
    /// branch was active.
    Ignored,
    /// The `\else` branch is active.
    ElseTrue,
    /// The `\else` branch is not active.
    ElseFalse,
}

/// One open `\if` block: its state, and the lexer state that psql saves at
/// the start of its current branch (`save_query_text_state` in `command.c`).
#[derive(Clone, Debug)]
struct Frame {
    branch: Branch,
    saved: ScanState,
}

/// Whether the innermost `\if` block, if any, is in an active branch
/// (`conditional_active`).
fn is_active(stack: &[Frame]) -> bool {
    stack
        .last()
        .is_none_or(|frame| matches!(frame.branch, Branch::True | Branch::ElseTrue))
}

/// Evaluates the expression of `\if` or `\elif`, as
/// `is_true_boolean_expression` in `command.c` does: the argument values,
/// with one space between them, read as a Boolean. A value psql does not
/// recognise is false. `None` when an argument holds a backquoted shell
/// command.
fn evaluate(command: &MetaCommand<'_>, variables: &Variables) -> Option<bool> {
    let values = command
        .command()
        .arguments()
        .iter()
        .map(|argument| argument.value(variables))
        .collect::<Option<Vec<_>>>()?;
    Some(parse_bool(&values.join(" ")) == Some(true))
}

/// Applies one conditional command to `stack`, as `exec_command_if`,
/// `exec_command_elif`, `exec_command_else` and `exec_command_endif` do. A
/// command that psql refuses, such as `\else` with no `\if`, changes nothing.
///
/// psql saves the lexer state at the start of a branch, and gives it back
/// when an inactive branch ends (`discard_query_text`), so that a `(` in an
/// inactive branch does not stay open. `scan` changes in the same way.
fn apply_conditional(
    command: &MetaCommand<'_>,
    stack: &mut Vec<Frame>,
    scan: &mut ScanState,
    variables: &Variables,
    unevaluated: &mut bool,
) {
    let mut test = || match evaluate(command, variables) {
        Some(result) => result,
        None => {
            *unevaluated = true;
            false
        }
    };
    match command {
        MetaCommand::If(_) => {
            let branch = if !is_active(stack) {
                Branch::Ignored
            } else if test() {
                Branch::True
            } else {
                Branch::False
            };
            stack.push(Frame {
                branch,
                saved: scan.clone(),
            });
        }
        MetaCommand::Elif(_) => {
            if let Some(top) = stack.last_mut() {
                match top.branch {
                    Branch::True => {
                        top.saved = scan.clone();
                        top.branch = Branch::Ignored;
                    }
                    Branch::False => {
                        *scan = top.saved.clone();
                        top.branch = if test() { Branch::True } else { Branch::False };
                    }
                    Branch::Ignored => *scan = top.saved.clone(),
                    Branch::ElseTrue | Branch::ElseFalse => {}
                }
            }
        }
        MetaCommand::Else(_) => {
            if let Some(top) = stack.last_mut() {
                match top.branch {
                    Branch::True => {
                        top.saved = scan.clone();
                        top.branch = Branch::ElseFalse;
                    }
                    Branch::Ignored => {
                        *scan = top.saved.clone();
                        top.branch = Branch::ElseFalse;
                    }
                    Branch::False => {
                        *scan = top.saved.clone();
                        top.branch = Branch::ElseTrue;
                    }
                    Branch::ElseTrue | Branch::ElseFalse => {}
                }
            }
        }
        MetaCommand::Endif(_) => {
            if let Some(top) = stack.pop()
                && matches!(
                    top.branch,
                    Branch::False | Branch::Ignored | Branch::ElseFalse
                )
            {
                *scan = top.saved;
            }
        }
        _ => unreachable!("only the four conditional commands come here"),
    }
}

/// Applies `\set` or `\unset` to `variables`, as `exec_command_set` and
/// `exec_command_unset` do. Returns `false` when an argument holds a
/// backquoted shell command, so that pgtk-psql cannot know the value.
///
/// `\set name value ...` sets `name` to the concatenation of the values, and
/// `\set name` sets it to the empty string. `\set` alone lists the
/// variables and changes nothing. A name that psql refuses
/// (`valid_variable_name`) changes nothing. psql's special variables, such
/// as `AUTOCOMMIT`, have hooks that pgtk-psql does not model.
fn apply_assignment(command: &MetaCommand<'_>, variables: &mut Variables) -> bool {
    let arguments = command.command().arguments();
    let Some((first, rest)) = arguments.split_first() else {
        return true;
    };
    let Some(name) = first.value(variables) else {
        return false;
    };
    match command {
        MetaCommand::Set(_) => {
            let Some(values) = rest
                .iter()
                .map(|argument| argument.value(variables))
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            if is_valid_variable_name(&name) {
                variables.set(name, values.concat());
            }
        }
        MetaCommand::Unset(_) => {
            if is_valid_variable_name(&name) {
                variables.unset(&name);
            }
        }
        _ => unreachable!("only `\\set` and `\\unset` come here"),
    }
    true
}

/// What one item makes psql do with COPY data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CopyRead {
    /// Where the item ends in the source. psql reads the data from the line
    /// after the line that holds this offset.
    pub(crate) end: usize,
    /// How many COPY data blocks psql reads.
    pub(crate) blocks: u32,
}

impl CopyRead {
    const NONE: Self = Self { end: 0, blocks: 0 };
}

/// A boundary whose place in the rendered SQL is known only when the
/// rewrites are final.
#[derive(Clone, Debug)]
struct PendingBoundary {
    source: Range<usize>,
    kind: BoundaryKind,
    /// The source extent that marks the place in the rendered SQL: the `;`,
    /// the send command, or the end of the discarded buffer. `None` in an
    /// inactive branch.
    anchor: Option<Range<usize>>,
}

/// Walks the items of a psql document in source order, as psql's main loop
/// reads them: it applies `\set`, `\unset` and the `\if` blocks, keeps the
/// lexer state that ends a query buffer, and records the rewrites of the
/// rendered SQL.
///
/// `crate::copy::frame` walks the items once to find the sends that read
/// COPY data, and [`render_document`] walks the final items again to render
/// them.
#[derive(Clone, Debug)]
pub(crate) struct Walker<'source> {
    source: &'source str,
    variables: Variables,
    rewrites: Vec<Rewrite>,
    unbound: Vec<Unbound>,
    unevaluated: Vec<Range<usize>>,
    /// The `\if` blocks that are open, innermost last.
    stack: Vec<Frame>,
    /// Where the text of the current inactive branch starts. psql discards
    /// the query text of an inactive branch when the branch ends
    /// (`discard_query_text` in `command.c`), so rendering removes it.
    inactive_from: Option<usize>,
    /// Where the current query buffer starts: the end of the previous
    /// boundary. A command that discards the buffer removes the text from
    /// here.
    buffer_start: usize,
    /// The lexer state of the current query buffer.
    scan: ScanState,
    boundaries: Vec<PendingBoundary>,
}

impl<'source> Walker<'source> {
    pub(crate) fn new(source: &'source str, variables: &Variables) -> Self {
        Self {
            source,
            variables: variables.clone(),
            rewrites: Vec::new(),
            unbound: Vec::new(),
            unevaluated: Vec::new(),
            stack: Vec::new(),
            inactive_from: None,
            buffer_start: 0,
            scan: ScanState::default(),
            boundaries: Vec::new(),
        }
    }

    /// Reads one item. `semicolons` gives the extent of each `;` token in
    /// order, because `Terminator::Semi` holds no text.
    pub(crate) fn item(
        &mut self,
        item: &PsqlItem<'_>,
        semicolons: &mut impl Iterator<Item = Range<usize>>,
    ) -> CopyRead {
        let source = self.source;
        if let PsqlItem::Command(command) = item
            && command.is_conditional()
        {
            self.conditional(command);
            return CopyRead::NONE;
        }
        let active = is_active(&self.stack);
        match item {
            PsqlItem::Sql(text) => {
                for atom in text.atoms.iter() {
                    let after = match atom {
                        SqlAtom::Word(token) => &source[span_of(source, token.text()).end..],
                        _ => "",
                    };
                    self.scan.atom(atom, after);
                    if active {
                        self.atom(atom);
                    }
                }
                CopyRead::NONE
            }
            PsqlItem::Terminator(Terminator::Semi) => {
                let semicolon = semicolons
                    .next()
                    .expect("each `;` terminator is one `SEMI` token");
                if !self.scan.semicolon() {
                    // Inside parentheses or a `BEGIN` block the `;` is text.
                    return CopyRead::NONE;
                }
                self.boundaries.push(PendingBoundary {
                    source: semicolon.clone(),
                    kind: BoundaryKind::Semicolon,
                    anchor: active.then(|| semicolon.clone()),
                });
                if !active {
                    // psql sends nothing in an inactive branch, and the
                    // buffer goes on (`MainLoop`).
                    return CopyRead::NONE;
                }
                let blocks = self.scan.copy_count();
                self.scan.reset();
                self.buffer_start = semicolon.end;
                CopyRead {
                    end: semicolon.end,
                    blocks,
                }
            }
            // A send command submits the buffer but is client syntax the
            // server never sees, so the command and its arguments render as
            // the boundary it is.
            PsqlItem::Terminator(Terminator::Send(send)) => {
                let span = span_of(source, send.text());
                self.boundary(
                    &span,
                    send.command().head().len(),
                    BoundaryKind::Send,
                    active,
                );
                if !active {
                    return CopyRead::NONE;
                }
                let blocks = if reads_copy_data(send) {
                    self.scan.send_copy_count()
                } else {
                    0
                };
                self.scan.reset();
                self.buffer_start = span.end;
                self.rewrites.push(Rewrite {
                    source: span.clone(),
                    text: ";".to_owned(),
                });
                CopyRead {
                    end: span.end,
                    blocks,
                }
            }
            // `\r` clears the buffer, and a discard command (18) ends it
            // without sending its text (docs/research/psql-14-19-syntax-changes.md,
            // class (A) item A1). The buffer text and the command render as
            // nothing, and the rewrites and unbound variables inside the
            // buffer go with them. Whitespace before the buffer stays.
            PsqlItem::Terminator(Terminator::Reset(reset)) => {
                let span = span_of(source, reset.text());
                self.discard(&span, reset.command().head().len(), active);
                CopyRead::NONE
            }
            #[cfg(feature = "since-pg18")]
            PsqlItem::Terminator(Terminator::Discard(discard)) => {
                let span = span_of(source, discard.text());
                self.discard(&span, discard.command().head().len(), active);
                CopyRead::NONE
            }
            _ if !active => CopyRead::NONE,
            PsqlItem::Interpolation(interpolation) => {
                let span = span_of(source, interpolation.text());
                match substitute(interpolation, &self.variables) {
                    Some(text) => self.rewrites.push(Rewrite { source: span, text }),
                    // Left verbatim: no rewrite, so the token stays part of
                    // the surrounding copied run.
                    None => self.unbound.push(Unbound {
                        name: interpolation.name().to_owned(),
                        source: span,
                    }),
                }
                CopyRead::NONE
            }
            PsqlItem::Command(command) => {
                let span = span_of(source, command.text());
                let mut read = CopyRead::NONE;
                match command {
                    // psql reports an unknown command as an error. It stays
                    // in the rendered text, so the server SQL shows it.
                    MetaCommand::Unknown(_) => return read,
                    MetaCommand::Set(_) | MetaCommand::Unset(_)
                        if !apply_assignment(command, &mut self.variables) =>
                    {
                        self.unevaluated.push(span.clone());
                    }
                    // `\copy ... from stdin` reads its data from the script,
                    // as `COPY ... FROM STDIN` does (`do_copy` sends the
                    // query with one COPY to read).
                    MetaCommand::WholeLine(_) => {
                        let command = command.command();
                        if command.name().eq_ignore_ascii_case("copy")
                            && command.arguments().first().is_some_and(|arguments| {
                                crate::copy::copy_command_reads_script(arguments.text())
                            })
                        {
                            read = CopyRead {
                                end: span.end,
                                blocks: 1,
                            };
                        }
                    }
                    _ => {}
                }
                // psql runs the command and sends none of it to the server.
                self.rewrites.push(Rewrite {
                    source: span,
                    text: String::new(),
                });
                read
            }
            // psql reads the data itself and sends it as COPY data, not as
            // query text.
            PsqlItem::CopyData(data) => {
                self.rewrites.push(Rewrite {
                    source: copy_data_span(source, data),
                    text: String::new(),
                });
                CopyRead::NONE
            }
        }
    }

    /// The rewrites of one atom of SQL text in an active branch.
    fn atom(&mut self, atom: &SqlAtom<'_>) {
        // `\;` and `\:` each force their second character into the query
        // buffer (psqlscan.l:697-702); the server receives that one byte, not
        // the two the user wrote.
        if let SqlAtom::Escaped(token) = atom {
            let escaped = token.text();
            self.rewrites.push(Rewrite {
                source: span_of(self.source, escaped),
                text: escaped[1..].to_owned(),
            });
        }
        // Before 15, psql does not send a `--` comment
        // (docs/research/psql-14-19-syntax-changes.md, class (A) item A3;
        // `REL_14_24:psqlscan.l` 388). The line ending after it is
        // whitespace, which psql sends.
        #[cfg(not(feature = "since-pg15"))]
        if let SqlAtom::LineComment(token) = atom {
            let text = token.text();
            let comment = text.trim_end_matches(['\r', '\n']);
            let start = span_of(self.source, text).start;
            self.rewrites.push(Rewrite {
                source: start..start + comment.len(),
                text: String::new(),
            });
        }
    }

    /// `\if`, `\elif`, `\else` or `\endif`.
    fn conditional(&mut self, command: &MetaCommand<'_>) {
        let span = span_of(self.source, command.text());
        let mut unknown = false;
        apply_conditional(
            command,
            &mut self.stack,
            &mut self.scan,
            &self.variables,
            &mut unknown,
        );
        if unknown {
            self.unevaluated.push(span.clone());
        }
        if let Some(from) = self.inactive_from.take()
            && from < span.start
        {
            self.rewrites.push(Rewrite {
                source: from..span.start,
                text: String::new(),
            });
        }
        self.rewrites.push(Rewrite {
            source: span.clone(),
            text: String::new(),
        });
        if !is_active(&self.stack) {
            self.inactive_from = Some(span.end);
        }
    }

    /// Records the boundary of a command whose backslash and name are the
    /// first `head` bytes of `span`.
    fn boundary(&mut self, span: &Range<usize>, head: usize, kind: BoundaryKind, active: bool) {
        self.boundaries.push(PendingBoundary {
            source: span.start..span.start + head,
            kind,
            anchor: active.then(|| span.clone()),
        });
    }

    /// `\r`, or a command of 18 that ends the buffer and sends nothing.
    fn discard(&mut self, span: &Range<usize>, head: usize, active: bool) {
        self.boundary(span, head, BoundaryKind::Discard, active);
        if active {
            // `exec_command_reset`, and `MainLoop` after `PSQL_CMD_SEND`.
            self.scan.reset();
            self.buffer_start = discard_buffer(
                self.source,
                self.buffer_start,
                span.clone(),
                &mut self.rewrites,
                &mut self.unbound,
            );
        }
    }

    /// The rendered SQL. `open_region` tells whether the source ends inside
    /// a string, a quoted identifier, a dollar-quoted body or a comment.
    pub(crate) fn finish(mut self, open_region: bool) -> Rendered {
        let source = self.source;
        // An `\if` block that is still open at the end of the document: psql
        // sends no text of an inactive branch.
        if let Some(from) = self.inactive_from
            && from < source.len()
        {
            self.rewrites.push(Rewrite {
                source: from..source.len(),
                text: String::new(),
            });
        }

        // Items are parsed in source order and each rewrite lies inside its
        // own item or between two conditional commands, so the rewrites are
        // already ascending and disjoint.
        let mut sql = String::with_capacity(source.len());
        let mut regions = Vec::with_capacity(self.rewrites.len());
        let mut cursor = 0;
        for rewrite in self.rewrites {
            sql.push_str(&source[cursor..rewrite.source.start]);
            let start = sql.len();
            sql.push_str(&rewrite.text);
            regions.push(Region {
                rendered: start..sql.len(),
                source: rewrite.source.clone(),
            });
            cursor = rewrite.source.end;
        }
        sql.push_str(&source[cursor..]);

        let boundaries = self
            .boundaries
            .into_iter()
            .map(|boundary| Boundary {
                rendered: boundary
                    .anchor
                    .map(|anchor| rendered_place(&regions, boundary.kind, anchor)),
                source: boundary.source,
                kind: boundary.kind,
            })
            .collect();

        Rendered {
            sql,
            map: SourceMap { regions },
            unbound: self.unbound,
            unevaluated: self.unevaluated,
            variables: self.variables,
            boundaries,
            complete: !open_region && !self.scan.is_open(),
        }
    }
}

/// Whether psql reads COPY data after `send`.
///
/// `\gdesc` describes the query and does not run it (`DescribeQuery`).
/// From 18, `\sendpipeline` queues the query in a pipeline, and psql leaves
/// `ExecQueryAndProcessResults` before it counts the COPY commands. Every
/// other send command runs the query in `ExecQueryAndProcessResults`, which
/// reads the data of each `COPY ... FROM STDIN` of the buffer text.
/// `\watch` runs the query again and again; pgtk-psql reads the data once.
fn reads_copy_data(send: &SendCommand<'_>) -> bool {
    match send {
        SendCommand::Gdesc(_) => false,
        #[cfg(feature = "since-pg18")]
        SendCommand::Sendpipeline(_) => false,
        _ => true,
    }
}

/// The source extent of a COPY data block: the rows and the `\.` line.
fn copy_data_span(source: &str, data: &CopyData<'_>) -> Range<usize> {
    let rows = span_of(source, data.rows.text());
    match &data.end {
        Some(end) => rows.start..span_of(source, end.marker.text()).end,
        None => rows,
    }
}

/// The place in the rendered SQL of a boundary whose source extent is
/// `anchor`.
fn rendered_place(regions: &[Region], kind: BoundaryKind, anchor: Range<usize>) -> Range<usize> {
    match kind {
        // The `;` is the user's own text.
        BoundaryKind::Semicolon => {
            let start = verbatim_place(regions, anchor.start);
            start..start + anchor.len()
        }
        // The command renders as `;`.
        BoundaryKind::Send => regions
            .iter()
            .find(|region| region.source == anchor)
            .map(|region| region.rendered.clone())
            .expect("a send command has its own rewrite"),
        // The buffer and the command render as nothing.
        BoundaryKind::Discard => regions
            .iter()
            .find(|region| region.source.end == anchor.end)
            .map(|region| region.rendered.start..region.rendered.start)
            .expect("a discard command has its own rewrite"),
    }
}

/// The rendered offset of the verbatim source offset `offset`.
fn verbatim_place(regions: &[Region], offset: usize) -> usize {
    let before = regions.partition_point(|region| region.source.end <= offset);
    match before.checked_sub(1) {
        Some(previous) => {
            let previous = &regions[previous];
            previous.rendered.end + (offset - previous.source.end)
        }
        None => offset,
    }
}

pub(crate) fn render_document(
    document: &PsqlDocument<'_>,
    lexed: &crate::LexResult<'_>,
    source: &str,
    variables: &Variables,
) -> Rendered {
    let mut walker = Walker::new(source, variables);
    let mut semicolons = lexed
        .tokens()
        .filter(|token| token.kind() == crate::TokenKind::SEMI)
        .map(|token| token.span().range());
    for item in &document.items {
        walker.item(item, &mut semicolons);
    }
    walker.finish(crate::copy::ends_in_open_region(lexed))
}

/// Removes the query buffer that `command` ends without sending it: the
/// text from `buffer_start`, without its leading whitespace, to the end of
/// the command. Returns where the next buffer starts.
fn discard_buffer(
    source: &str,
    buffer_start: usize,
    command: Range<usize>,
    rewrites: &mut Vec<Rewrite>,
    unbound: &mut Vec<Unbound>,
) -> usize {
    // The leading whitespace of the buffer stays. A meta-command that renders
    // as nothing is not text of the buffer, so the whitespace after it stays
    // too.
    let mut start = buffer_start;
    loop {
        let gap = &source[start..command.start];
        start += gap.len() - gap.trim_start().len();
        match rewrites
            .iter()
            .find(|rewrite| rewrite.source.start == start && rewrite.text.is_empty())
        {
            Some(rewrite) if rewrite.source.end < command.start => start = rewrite.source.end,
            _ => break,
        }
    }
    rewrites.retain(|rewrite| rewrite.source.start < start);
    unbound.retain(|variable| variable.source.start < start);
    rewrites.push(Rewrite {
        source: start..command.end,
        text: String::new(),
    });
    command.end
}

/// The text a bound interpolation renders to, or `None` when it is unbound.
fn substitute(interpolation: &Interpolation<'_>, variables: &Variables) -> Option<String> {
    let name = interpolation.name();
    match interpolation {
        Interpolation::Literal(_) => variables.get(name).map(quote_literal),
        Interpolation::Identifier(_) => variables.get(name).map(quote_identifier),
        Interpolation::Raw(_) => variables.get(name).map(str::to_owned),
        // The existence test is the one form with no unbound case: psql
        // answers it rather than passing it through (psqlscan.l:1568-1591).
        Interpolation::Test(_) => Some(
            if variables.contains(name) {
                "TRUE"
            } else {
                "FALSE"
            }
            .to_owned(),
        ),
    }
}

/// Quotes a value as a SQL string literal, as libpq's `PQescapeLiteral` does.
///
/// `fe-exec.c:4371-4375` doubles every `'` and every `\`. When the value
/// contains a backslash, `fe-exec.c:4334-4345` emits ` E'...'` instead of
/// `'...'`, so the literal means the same under either setting of
/// `standard_conforming_strings`; the leading space guards against the
/// result landing immediately after an identifier.
pub(crate) fn quote_literal(value: &str) -> String {
    let backslashes = value.bytes().filter(|byte| *byte == b'\\').count();
    let mut quoted = String::with_capacity(value.len() + backslashes + 4);
    if backslashes > 0 {
        quoted.push_str(" E");
    }
    quoted.push('\'');
    for character in value.chars() {
        if character == '\'' || character == '\\' {
            quoted.push(character);
        }
        quoted.push(character);
    }
    quoted.push('\'');
    quoted
}

/// Quotes a value as a quoted identifier, as libpq's `PQescapeIdentifier`
/// does: `fe-exec.c:4371-4375` doubles every `"`, and a backslash is an
/// ordinary character inside an identifier.
pub(crate) fn quote_identifier(value: &str) -> String {
    let quotes = value.bytes().filter(|byte| *byte == b'"').count();
    let mut quoted = String::with_capacity(value.len() + quotes + 2);
    quoted.push('"');
    for character in value.chars() {
        if character == '"' {
            quoted.push(character);
        }
        quoted.push(character);
    }
    quoted.push('"');
    quoted
}

/// The extent of `part` inside `source`.
///
/// Every token this crate reads is a subslice of the parsed source, so the
/// distance between the two pointers is the offset. recursa's own span
/// capture is per node and reached through generated views; a token's source
/// slice carries the same fact directly and without a traversal, and the
/// assertion keeps the two honest.
fn span_of(source: &str, part: &str) -> Range<usize> {
    let base = source.as_ptr() as usize;
    let start = part.as_ptr() as usize;
    assert!(
        start >= base && start + part.len() <= base + source.len(),
        "token text is not a subslice of the parsed source",
    );
    (start - base)..(start - base + part.len())
}
