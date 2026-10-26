//! COPY data: the lines that psql reads itself after `COPY ... FROM STDIN`.
//!
//! When psql sends a query buffer, `SendQuery` gets the number of
//! `COPY ... FROM STDIN` commands in the buffer, and for each one
//! `handleCopyIn` (`src/bin/psql/copy.c`) reads lines from the script itself,
//! not from the lexer: the data starts on the line after the line that sent
//! the buffer, and it ends at a line that is exactly `\.` with a LF or CRLF
//! ending, or at end of input. psql reads the data even when the server
//! refuses the COPY, so that the script stays in step (the end of
//! `ExecQueryAndProcessResults`). The rest of the line that sent the buffer
//! is SQL text again after the data.
//!
//! The data can hold any text, so the lexer must not read it. [`frame`] lexes
//! the script in segments, finds each send of a buffer that holds a
//! `COPY ... FROM STDIN` (or a `\copy ... from stdin`), and supplies the data
//! and the `\.` line as the tokens of `crate::ast::CopyData` through
//! `LexBuilder`.

use std::ops::Range;

use crate::substitution::{Variables, Walker};
use crate::{LexBuilder, LexResult, PsqlDocument, PsqlError, TokenKind};

/// Lexes `source` with its COPY data regions.
///
/// The walk of the items applies `\set`, `\unset` and the `\if` blocks with
/// `variables`, as [`crate::render`] does, because psql sends no buffer in an
/// inactive branch and so reads no data there.
pub(crate) fn frame<'source>(
    source: &'source str,
    variables: &Variables,
) -> Result<LexResult<'source>, PsqlError> {
    let mut builder = LexBuilder::new(source);
    let mut walker = Walker::new(source, variables);
    let mut position = 0;
    loop {
        let rest = &source[position..];
        let lexed = lex_segment(rest, position)?;
        let items = parse_segment(&lexed)?;
        // The first item that makes psql read COPY data.
        let start = walker.clone();
        let mut ends = semicolons(&lexed, position);
        let send_end = items.items.iter().find_map(|item| {
            let read = walker.item(item, &mut ends);
            (read.blocks > 0).then_some(read.end)
        });
        let line_end = send_end.and_then(|end| source[end..].find('\n').map(|at| end + at + 1));
        let Some(data_start) = line_end else {
            append(&mut builder, &lexed, position)?;
            break;
        };

        // psql reads the data after the whole line, so the segment ends with
        // the line. Every send on the rest of the line reads its data after
        // the data of the sends before it.
        let segment = lex_segment(&source[position..data_start], position)?;
        let items = parse_segment(&segment)?;
        walker = start;
        let mut semicolons = semicolons(&segment, position);
        let mut blocks = 0;
        for item in &items.items {
            blocks += walker.item(item, &mut semicolons).blocks;
        }
        append(&mut builder, &segment, position)?;

        position = data_start;
        for _ in 0..blocks {
            if position >= source.len() {
                break;
            }
            let (rows, end) = data_block(source, position);
            builder
                .append(TokenKind::CopyRowsText, span(rows.clone()))
                .expect("the segments and the data are in source order");
            position = rows.end;
            if let Some(end) = end {
                builder
                    .append(TokenKind::CopyEndText, span(end.clone()))
                    .expect("the segments and the data are in source order");
                position = end.end;
            }
        }
        if position >= source.len() {
            break;
        }
    }
    Ok(builder.finish())
}

/// The rows of one COPY data block that starts at `start`, and its `\.` line,
/// as `handleCopyIn` reads them in text mode.
///
/// Only a line that is exactly `\.` and a LF or a CRLF ends the data. A `\.`
/// with space around it, or a `\.` at end of input with no line ending, is
/// data.
pub(crate) fn data_block(source: &str, start: usize) -> (Range<usize>, Option<Range<usize>>) {
    let mut line = start;
    while line < source.len() {
        let end = source[line..]
            .find('\n')
            .map_or(source.len(), |at| line + at + 1);
        let text = &source[line..end];
        if text == "\\.\n" || text == "\\.\r\n" {
            return (start..line, Some(line..end));
        }
        line = end;
    }
    (start..source.len(), None)
}

/// Whether a `\copy` command with the whole-line argument `arguments` reads
/// its data from the script, as `parse_slash_copy` in `src/bin/psql/copy.c`
/// reads the argument: `from stdin` (or `from stdout`) after the table, its
/// column list or a parenthesised query. `from pstdin` reads psql's own
/// standard input, which is not the script when psql runs a file.
pub(crate) fn copy_command_reads_script(arguments: &str) -> bool {
    const SPACE: &str = " \t\n\r";
    let mut tokens = Tokens::new(arguments);
    let mut next = |delimiters: &str, quotes: &str, e_strings: bool| {
        tokens.next(SPACE, delimiters, quotes, e_strings)
    };
    let Some(mut token) = next(".,()", "\"", false) else {
        return false;
    };
    if token.eq_ignore_ascii_case("binary") {
        let Some(after) = next(".,()", "\"", false) else {
            return false;
        };
        token = after;
    }
    // `\copy (query) ...`
    if token.starts_with('(') {
        let mut depth = 1;
        while depth > 0 {
            let Some(after) = next("()", "\"'", true) else {
                return false;
            };
            token = after;
            if token.starts_with('(') {
                depth += 1;
            } else if token.starts_with(')') {
                depth -= 1;
            }
        }
    }
    let Some(mut token) = next(".,()", "\"", false) else {
        return false;
    };
    // `schema . table`
    if token.starts_with('.') {
        let (Some(_), Some(after)) = (next(".,()", "\"", false), next(".,()", "\"", false)) else {
            return false;
        };
        token = after;
    }
    // A column list.
    if token.starts_with('(') {
        loop {
            let Some(after) = next("()", "\"", false) else {
                return false;
            };
            if after.starts_with(')') {
                break;
            }
        }
        let Some(after) = next(".,()", "\"", false) else {
            return false;
        };
        token = after;
    }
    if !token.eq_ignore_ascii_case("from") {
        return false;
    }
    next(";", "'", false).is_some_and(|file| {
        file.eq_ignore_ascii_case("stdin") || file.eq_ignore_ascii_case("stdout")
    })
}

/// `strtokx` of `src/bin/psql/stringutils.c`, with no escape character
/// (`standard_conforming_strings` is on) and the quotes kept.
struct Tokens<'text> {
    rest: &'text str,
}

impl<'text> Tokens<'text> {
    fn new(text: &'text str) -> Self {
        Self { rest: text }
    }

    fn next(
        &mut self,
        space: &str,
        delimiters: &str,
        quotes: &str,
        e_strings: bool,
    ) -> Option<&'text str> {
        let text = self
            .rest
            .trim_start_matches(|character| space.contains(character));
        let first = text.chars().next()?;
        let end = if delimiters.contains(first) {
            first.len_utf8()
        } else {
            // `E'...'`: the quote is `'` and a backslash escapes.
            let (start, quotes, escape) =
                if e_strings && (first == 'E' || first == 'e') && text[1..].starts_with('\'') {
                    (1, "'", Some('\\'))
                } else {
                    (0, quotes, None)
                };
            let opening = text[start..].chars().next()?;
            if quotes.contains(opening) {
                quoted_end(text, start, opening, escape)
            } else {
                text.find(|character: char| {
                    space.contains(character)
                        || delimiters.contains(character)
                        || quotes.contains(character)
                })
                .unwrap_or(text.len())
            }
        };
        let (token, after) = text.split_at(end);
        // A space after the token is the end that `strtokx` writes over.
        self.rest = after
            .strip_prefix(|character| space.contains(character))
            .unwrap_or(after);
        Some(token)
    }
}

/// The end of the quoted token of `text` whose quote is at `start`.
fn quoted_end(text: &str, start: usize, quote: char, escape: Option<char>) -> usize {
    let mut characters = text[start + 1..].char_indices().peekable();
    while let Some((at, character)) = characters.next() {
        if Some(character) == escape {
            characters.next();
        } else if character == quote {
            if characters.peek().is_some_and(|(_, next)| *next == quote) {
                characters.next();
            } else {
                return start + 1 + at + character.len_utf8();
            }
        }
    }
    text.len()
}

/// The lexed segment of `source` that starts at `offset`. The lexer of
/// pgtk-psql reports no error on any text, but a segment is checked as a
/// whole document is.
fn lex_segment(segment: &str, offset: usize) -> Result<LexResult<'_>, PsqlError> {
    let lexed = crate::lex(segment);
    let errors = lexed
        .errors()
        .map(|error| {
            let range = error.span().range();
            span(range.start + offset..range.end + offset)
        })
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(lexed)
    } else {
        Err(PsqlError::Lexical(errors))
    }
}

fn parse_segment<'source>(lexed: &LexResult<'source>) -> Result<PsqlDocument<'source>, PsqlError> {
    let mut input = lexed.input();
    let parsed = PsqlDocument::parse(&mut input).map_err(PsqlError::Syntax)?;
    if !input.is_eof() {
        return Err(PsqlError::TrailingInput);
    }
    Ok(parsed.into_ast())
}

/// The extent in the whole source of each `;` token of a segment.
fn semicolons(lexed: &LexResult<'_>, offset: usize) -> std::vec::IntoIter<Range<usize>> {
    lexed
        .tokens()
        .filter(|token| token.kind() == TokenKind::SEMI)
        .map(|token| {
            let range = token.span().range();
            range.start + offset..range.end + offset
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// Appends the tokens of a segment to the document at `offset`.
fn append(
    builder: &mut LexBuilder<'_>,
    lexed: &LexResult<'_>,
    offset: usize,
) -> Result<(), PsqlError> {
    for token in lexed.tokens() {
        let range = token.span().range();
        builder
            .append(token.kind(), span(range.start + offset..range.end + offset))
            .expect("the segments and the data are in source order");
    }
    Ok(())
}

fn span(range: Range<usize>) -> recursa::Span {
    recursa::Span::new(range.start, range.end).expect("a psql document fits in a span")
}

/// Whether the lexer of psql is inside a string, a quoted identifier, a
/// dollar-quoted body or a block comment at the end of `lexed`.
///
/// COPY data is not lexed, so the state is the state at the end of the last
/// SQL text before it. A block comment is ignored trivia, so it is found in
/// the text after the last token.
pub(crate) fn ends_in_open_region(lexed: &LexResult<'_>) -> bool {
    let source = lexed.source();
    let tokens = lexed
        .tokens()
        .map(|token| (token.kind(), token.span().range()))
        .collect::<Vec<_>>();
    let is_data =
        |kind: TokenKind| matches!(kind, TokenKind::CopyRowsText | TokenKind::CopyEndText);
    let last = tokens.iter().rposition(|(kind, _)| !is_data(*kind));
    let gap_start = last.map_or(0, |at| tokens[at].1.end);
    let gap_end = tokens[last.map_or(0, |at| at + 1)..]
        .first()
        .map_or(source.len(), |(_, range)| range.start);
    if let Some(at) = last {
        let (kind, range) = &tokens[at];
        let text = &source[range.clone()];
        match kind {
            TokenKind::OpenQuotedText => return true,
            TokenKind::DollarString if !is_closed_dollar_string(text) => return true,
            _ => {}
        }
    }
    crate::scan_state::ends_in_open_comment(&source[gap_start..gap_end])
}

/// Whether a dollar-quoted token holds its closing delimiter.
fn is_closed_dollar_string(text: &str) -> bool {
    let tag = text[1..].find('$').map_or(text.len(), |at| at + 2);
    text.len() >= 2 * tag && text.ends_with(&text[..tag])
}
