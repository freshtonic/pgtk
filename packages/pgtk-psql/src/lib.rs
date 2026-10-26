#![doc = include_str!("../README.md")]
//!
//! # Source map
//!
//! Rendering keeps a [`SourceMap`] so a diagnostic over the rendered SQL can
//! still name a byte of the user's psql source. Without it, substitution
//! would destroy the tie between a diagnostic and the text the user wrote,
//! which is the whole point of pgtk-sql's occurrence model.
//!
//! # Meta-commands
//!
//! `psqlscan.l` returns `LEXRES_BACKSLASH` at a backslash and hands off to
//! `psqlscanslash.l`, which reads the command name and the arguments. Each
//! meta-command is one token here, with the argument region that psql reads
//! for that command ([`ast`] gives the rules). [`MetaCommand::command`] and
//! [`Command`] divide the token into the name, the arguments and a trailing
//! `\\`.
//!
//! [`render`] applies `\set`, `\unset` and the `\if` blocks, because they
//! change what psql sends. Every other command keeps its arguments as text
//! and renders as nothing
//! (`packages/pgtk-sql/docs/adr/0013-keep-most-meta-command-arguments-as-text.md`).
//! A name that is not a command of the target version is
//! [`MetaCommand::Unknown`], and it stays in the rendered text, so that the
//! server SQL shows the error.
//!
//! # Query buffers
//!
//! psql ends the query buffer at a `;` only outside parentheses and outside
//! a `BEGIN ... END` block of a `CREATE FUNCTION` or `CREATE PROCEDURE`, and
//! at each send or discard command. [`Rendered::boundaries`] gives these
//! places, and [`Rendered::is_complete`] tells whether the document ends with
//! a complete buffer.
//!
//! # COPY data
//!
//! After psql sends a buffer that holds `COPY ... FROM STDIN`, or runs
//! `\copy ... from stdin`, it reads the data lines from the script itself, up
//! to a line that is exactly `\.`. [`parse`] gives each block of data as a
//! [`PsqlItem::CopyData`], and [`render`] sends none of it as SQL text
//! ([`ast::CopyData`] gives the rules).
//!
//! # What this crate does not model
//!
//! pgtk-psql runs no command: no included file, no shell command, no
//! backquoted argument and no connection.
//!
//! psql tolerates end of input inside any open string, quoted identifier,
//! dollar-quoted body or comment (`psqlscan.l:983`'s single `<<EOF>>` rule).
//! `DollarString` and `BlockComment` are closed matchers
//! (`same_delimiter`/`nested`, `pgtk-psql/src/tokens.rs`) and opt into the same
//! tolerance with `unterminated = to_end_of_input` (freshtonic/recursa#131):
//! an unterminated dollar-quoted body or comment runs to end of input rather
//! than refusing the document, and a colon inside it is still never
//! substituted. The quoted string and quoted-identifier forms are plain
//! `#[lex(pattern = ...)]` tokens, so each also has an open form,
//! `SqlAtom::OpenQuoted`: the same pattern without the closing quote. It wins
//! only at end of input, so an unterminated string or quoted identifier also
//! runs to end of input as one token, and a colon inside it is never
//! substituted (freshtonic/pg-sql#89).

recursa::grammar! {
    module = crate,
    derives(Pretty, Visit),
    keyword_matching = sensitive,
    diagrams,
}

pub mod ast;
pub mod conditional;
mod copy;
pub mod meta;
mod scan_state;
pub mod substitution;
pub mod tokens;

pub use ast::{
    CopyData, CopyEnd, Interpolation, MetaCommand, PsqlDocument, PsqlItem, ResetCommand,
    SendCommand, SqlAtom, SqlText, Terminator,
};
pub use conditional::{ConditionalBlock, ConditionalError, ConditionalErrorKind};
pub use meta::{Argument, ArgumentKind, Command};
// Added in 18: docs/research/psql-14-19-syntax-changes.md, class (A) item A1.
#[cfg(feature = "since-pg18")]
pub use ast::DiscardCommand;
pub use substitution::{Boundary, BoundaryKind, Origin, Rendered, SourceMap, Unbound, Variables};

use std::fmt;

/// Why a psql document could not be parsed.
#[derive(Debug)]
pub enum PsqlError {
    /// One or more source ranges were not recognised by the psql lexer.
    Lexical(Vec<recursa::Span>),
    /// The recognised tokens did not form a psql document.
    Syntax(recursa::ParseError),
    /// A document was parsed but input remained, which the item repetition
    /// covering the whole source makes a grammar defect rather than a user
    /// error.
    TrailingInput,
}

impl fmt::Display for PsqlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lexical(spans) => {
                write!(formatter, "{} psql lexical error(s)", spans.len())
            }
            Self::Syntax(error) => error.fmt(formatter),
            Self::TrailingInput => formatter.write_str("trailing input after the psql document"),
        }
    }
}

impl std::error::Error for PsqlError {}

/// Parses `source` as psql, runs its `\set`, `\unset` and `\if` commands,
/// substitutes the variables, and renders server SQL.
///
/// `variables` are the variables at the start of the script, as `psql -v`
/// gives them. Only the text of the active `\if` branches is rendered.
///
/// This is the whole pipeline up to the SQL parse; call
/// [`Rendered::parse_sql`] for the last step.
pub fn render(source: &str, variables: &Variables) -> Result<Rendered, PsqlError> {
    // The COPY data depends on the `\if` branches, so it is found with the
    // same variables.
    let lexed = copy::frame(source, variables)?;
    let document = parse_lexed(&lexed)?;
    Ok(substitution::render_document(
        &document, &lexed, source, variables,
    ))
}

/// Parses one complete psql source document.
///
/// The result is the item sequence that `psqlscan.l` and `psqlscanslash.l`
/// produce: runs of SQL text, variable interpolations, the terminators that
/// end a query buffer, and the meta-commands, and the COPY data that psql
/// reads itself. Nothing is substituted and no command is applied — use
/// [`render`] for that.
///
/// psql sends no query buffer in an inactive `\if` branch, so it reads no
/// COPY data there. To find the COPY data, `parse` applies the `\set`,
/// `\unset` and `\if` commands of the script with no variables set at the
/// start, as [`render`] does with an empty [`Variables`].
pub fn parse(source: &str) -> Result<PsqlDocument<'_>, PsqlError> {
    let lexed = copy::frame(source, &Variables::new())?;
    parse_lexed(&lexed)
}

fn parse_lexed<'source>(lexed: &LexResult<'source>) -> Result<PsqlDocument<'source>, PsqlError> {
    let mut input = lexed.input();
    let parsed = PsqlDocument::parse(&mut input).map_err(PsqlError::Syntax)?;
    if !input.is_eof() {
        return Err(PsqlError::TrailingInput);
    }
    Ok(parsed.into_ast())
}
