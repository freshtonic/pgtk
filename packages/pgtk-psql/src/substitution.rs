//! Substitution: a psql document becomes server SQL text plus a source map.
//!
//! Rendering copies the source verbatim and rewrites only the regions psql
//! itself rewrites. That is what keeps the map exact: everything outside a
//! recorded region is the user's own bytes at a known shift, so a diagnostic
//! over the rendered SQL can always be carried back to the psql source.

use std::collections::BTreeMap;
use std::ops::Range;

use crate::ast::{Interpolation, MetaCommand, PsqlDocument, PsqlItem, SqlAtom, Terminator};
use crate::meta::{is_valid_variable_name, parse_bool};

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

/// Server SQL rendered from a psql document, with the map back to its source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    sql: String,
    map: SourceMap,
    unbound: Vec<Unbound>,
    unevaluated: Vec<Range<usize>>,
    variables: Variables,
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

    /// Parses the rendered SQL as a strict PostgreSQL document.
    ///
    /// This is the last step of the pipeline, and the only place pgtk-sql is
    /// asked to read anything: the SQL grammar never sees psql syntax.
    pub fn parse_sql(&self) -> Result<pgtk_sql::SqlDocument<'_>, pgtk_sql::SqlParseError<'_>> {
        pgtk_sql::document::parse_sql(&self.sql)
    }
}

/// One rewritten span of the source and the text that replaces it.
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

/// Whether the innermost `\if` block, if any, is in an active branch
/// (`conditional_active`).
fn is_active(stack: &[Branch]) -> bool {
    stack
        .last()
        .is_none_or(|branch| matches!(branch, Branch::True | Branch::ElseTrue))
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
fn apply_conditional(
    command: &MetaCommand<'_>,
    stack: &mut Vec<Branch>,
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
            stack.push(branch);
        }
        MetaCommand::Elif(_) => {
            if let Some(top) = stack.last_mut() {
                match *top {
                    Branch::True => *top = Branch::Ignored,
                    Branch::False => *top = if test() { Branch::True } else { Branch::False },
                    Branch::Ignored | Branch::ElseTrue | Branch::ElseFalse => {}
                }
            }
        }
        MetaCommand::Else(_) => {
            if let Some(top) = stack.last_mut() {
                match *top {
                    Branch::True | Branch::Ignored => *top = Branch::ElseFalse,
                    Branch::False => *top = Branch::ElseTrue,
                    Branch::ElseTrue | Branch::ElseFalse => {}
                }
            }
        }
        MetaCommand::Endif(_) => {
            stack.pop();
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

pub(crate) fn render_document(
    document: &PsqlDocument<'_>,
    source: &str,
    variables: &Variables,
) -> Rendered {
    let mut variables = variables.clone();
    let mut rewrites = Vec::new();
    let mut unbound = Vec::new();
    let mut unevaluated = Vec::new();
    // The `\if` blocks that are open, innermost last.
    let mut stack = Vec::new();
    // Where the text of the current inactive branch starts. psql discards
    // the query text of an inactive branch when the branch ends
    // (`discard_query_text` in `command.c`), so rendering removes it.
    let mut inactive_from: Option<usize> = None;
    // Where the current query buffer starts: the end of the previous
    // terminator. A command that discards the buffer removes the text from
    // here.
    let mut buffer_start = 0;
    let mut semicolons = semicolon_ends(source).into_iter();

    for item in &document.items {
        if let PsqlItem::Command(command) = item
            && command.is_conditional()
        {
            let span = span_of(source, command.text());
            let mut unknown = false;
            apply_conditional(command, &mut stack, &variables, &mut unknown);
            if unknown {
                unevaluated.push(span.clone());
            }
            if let Some(from) = inactive_from.take()
                && from < span.start
            {
                rewrites.push(Rewrite {
                    source: from..span.start,
                    text: String::new(),
                });
            }
            rewrites.push(Rewrite {
                source: span.clone(),
                text: String::new(),
            });
            if !is_active(&stack) {
                inactive_from = Some(span.end);
            }
            continue;
        }
        if !is_active(&stack) {
            // The text of an inactive branch goes as a whole when the branch
            // ends; only the position of each `;` is kept in step.
            if matches!(item, PsqlItem::Terminator(Terminator::Semi)) {
                semicolons.next();
            }
            continue;
        }
        match item {
            PsqlItem::Interpolation(interpolation) => {
                let span = span_of(source, interpolation.text());
                match substitute(interpolation, &variables) {
                    Some(text) => rewrites.push(Rewrite { source: span, text }),
                    // Left verbatim: no rewrite, so the token stays part of
                    // the surrounding copied run.
                    None => unbound.push(Unbound {
                        name: interpolation.name().to_owned(),
                        source: span,
                    }),
                }
            }
            // A send command submits the buffer but is client syntax the
            // server never sees, so the command and its arguments render as
            // the boundary it is.
            PsqlItem::Terminator(Terminator::Send(send)) => {
                let span = span_of(source, send.text());
                buffer_start = span.end;
                rewrites.push(Rewrite {
                    source: span,
                    text: ";".to_owned(),
                });
            }
            PsqlItem::Terminator(Terminator::Semi) => {
                buffer_start = semicolons
                    .next()
                    .expect("each `;` terminator is one `SEMI` token");
            }
            // `\r` clears the buffer, and a discard command (18) ends it
            // without sending its text (docs/research/psql-14-19-syntax-changes.md,
            // class (A) item A1). The buffer text and the command render as
            // nothing, and the rewrites and unbound variables inside the
            // buffer go with them. Whitespace before the buffer stays.
            PsqlItem::Terminator(Terminator::Reset(reset)) => {
                let command = span_of(source, reset.text());
                buffer_start =
                    discard_buffer(source, buffer_start, command, &mut rewrites, &mut unbound);
            }
            #[cfg(feature = "since-pg18")]
            PsqlItem::Terminator(Terminator::Discard(discard)) => {
                let command = span_of(source, discard.text());
                buffer_start =
                    discard_buffer(source, buffer_start, command, &mut rewrites, &mut unbound);
            }
            PsqlItem::Command(command) => {
                let span = span_of(source, command.text());
                match command {
                    // psql reports an unknown command as an error. It stays
                    // in the rendered text, so the server SQL shows it.
                    MetaCommand::Unknown(_) => continue,
                    MetaCommand::Set(_) | MetaCommand::Unset(_)
                        if !apply_assignment(command, &mut variables) =>
                    {
                        unevaluated.push(span.clone());
                    }
                    _ => {}
                }
                // psql runs the command and sends none of it to the server.
                rewrites.push(Rewrite {
                    source: span,
                    text: String::new(),
                });
            }
            PsqlItem::Sql(text) => {
                for atom in text.atoms.iter() {
                    // `\;` and `\:` each force their second character into
                    // the query buffer (psqlscan.l:697-702); the server
                    // receives that one byte, not the two the user wrote.
                    if let SqlAtom::Escaped(token) = atom {
                        let escaped = token.text();
                        rewrites.push(Rewrite {
                            source: span_of(source, escaped),
                            text: escaped[1..].to_owned(),
                        });
                    }
                    // Before 15, psql does not send a `--` comment
                    // (docs/research/psql-14-19-syntax-changes.md, class (A)
                    // item A3; `REL_14_24:psqlscan.l` 388). The line ending
                    // after it is whitespace, which psql sends.
                    #[cfg(not(feature = "since-pg15"))]
                    if let SqlAtom::LineComment(token) = atom {
                        let text = token.text();
                        let comment = text.trim_end_matches(['\r', '\n']);
                        let start = span_of(source, text).start;
                        rewrites.push(Rewrite {
                            source: start..start + comment.len(),
                            text: String::new(),
                        });
                    }
                }
            }
        }
    }
    // An `\if` block that is still open at the end of the document: psql
    // sends no text of an inactive branch.
    if let Some(from) = inactive_from
        && from < source.len()
    {
        rewrites.push(Rewrite {
            source: from..source.len(),
            text: String::new(),
        });
    }

    // Items are parsed in source order and each rewrite lies inside its own
    // item or between two conditional commands, so the rewrites are already
    // ascending and disjoint.
    let mut sql = String::with_capacity(source.len());
    let mut regions = Vec::with_capacity(rewrites.len());
    let mut cursor = 0;
    for rewrite in rewrites {
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

    Rendered {
        sql,
        map: SourceMap { regions },
        unbound,
        unevaluated,
        variables,
    }
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

/// The end offset of each `;` terminator in `source`, in order.
///
/// `Terminator::Semi` holds no text, so its position comes from the lexer.
/// Only a terminator lexes as `SEMI`: `\;` is one `Escaped` token, and a
/// `;` inside a string, a quoted identifier, a dollar-quoted body, a comment
/// or a meta-command is part of that token or of trivia.
fn semicolon_ends(source: &str) -> Vec<usize> {
    crate::lex(source)
        .tokens()
        .filter(|token| token.kind() == crate::TokenKind::SEMI)
        .map(|token| token.span().range().end)
        .collect()
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
