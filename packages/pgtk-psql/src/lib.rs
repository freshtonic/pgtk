#![doc = include_str!("../README.md")]
//!
//! # Source map
//!
//! Rendering keeps a [`SourceMap`] so a diagnostic over the rendered SQL can
//! still name a byte of the user's psql source. Without it, substitution
//! would destroy the tie between a diagnostic and the text the user wrote,
//! which is the whole point of pgtk-sql's occurrence model.
//!
//! # What this crate does not model
//!
//! psql meta-commands (`\set`, `\d`, `\if`, `\copy`, ...) are out of scope
//! and tracked as freshtonic/pg-sql#11, #12 and #13. `psqlscan.l` does not
//! scan them either — it returns `LEXRES_BACKSLASH` and hands off to
//! `psqlscanslash.l` — so a backslash command that is not one of the send
//! commands stays inside an ordinary [`SqlText`] run and is rendered
//! verbatim. A document using one therefore renders to SQL the server
//! cannot parse, which is a diagnosable condition rather than a silent
//! rewrite.
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
pub mod substitution;
pub mod tokens;

pub use ast::{Interpolation, PsqlDocument, PsqlItem, SendCommand, SqlAtom, SqlText, Terminator};
// Added in 18: docs/research/psql-14-19-syntax-changes.md, class (A) item A1.
#[cfg(feature = "since-pg18")]
pub use ast::DiscardCommand;
pub use substitution::{Origin, Rendered, SourceMap, Unbound, Variables};

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

/// Parses `source` as psql, substitutes `variables`, and renders server SQL.
///
/// This is the whole pipeline up to the SQL parse; call
/// [`Rendered::parse_sql`] for the last step.
pub fn render(source: &str, variables: &Variables) -> Result<Rendered, PsqlError> {
    let document = parse(source)?;
    Ok(substitution::render_document(&document, source, variables))
}

/// Parses one complete psql source document.
///
/// The result is the item sequence `psqlscan.l` produces: runs of SQL text,
/// variable interpolations, and the terminators that submit a query buffer.
/// Nothing is substituted — use [`render`] for that.
pub fn parse(source: &str) -> Result<PsqlDocument<'_>, PsqlError> {
    let lexed = lex(source);
    let lexical = lexed.errors().map(|error| error.span()).collect::<Vec<_>>();
    if !lexical.is_empty() {
        return Err(PsqlError::Lexical(lexical));
    }
    let mut input = lexed.input();
    let parsed = PsqlDocument::parse(&mut input).map_err(PsqlError::Syntax)?;
    if !input.is_eof() {
        return Err(PsqlError::TrailingInput);
    }
    Ok(parsed.into_ast())
}
