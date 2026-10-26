//! The state of `psqlscan.l` that decides where a query buffer ends.
//!
//! psql does not end a statement at every `;`. The `;` rule of `psqlscan.l`
//! returns `LEXRES_SEMI` only when the parenthesis depth and the `BEGIN`
//! depth are both zero, and the `{identifier}` rule calls
//! `psqlscan_track_identifier`, which counts `BEGIN ... END` in a
//! `CREATE [OR REPLACE] {FUNCTION|PROCEDURE}` and records the first words of
//! the statement, so that `psqlscan_is_copy_from_stdin` can find
//! `COPY ... FROM STDIN`. [`ScanState`] is that state (`PsqlScanStateSave` in
//! `src/include/fe_utils/psqlscan_int.h`), kept in step with the tokens of a
//! document. The rules are the same in each target version.

use crate::ast::SqlAtom;

/// `lengthof(state->init_idents)`.
const INITIAL_WORDS: usize = 8;

/// The lexer state of one query buffer, as `psqlscan.l` keeps it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScanState {
    /// `paren_depth`: the number of open parentheses.
    paren_depth: u32,
    /// `begin_depth`: the number of open `BEGIN` (and `CASE` in a `BEGIN`)
    /// blocks of a `CREATE FUNCTION` or `CREATE PROCEDURE`.
    begin_depth: u32,
    /// `copy_stdin_count`: the statements of the buffer, ended by `\;`, that
    /// are `COPY ... FROM STDIN`.
    copy_count: u32,
    /// `init_idents_count`: the number of words since the start of the
    /// statement.
    word_count: usize,
    /// `init_idents`: one code for each of the first words of the statement.
    words: [u8; INITIAL_WORDS],
    /// Whether the first statement of the buffer, when a `\;` ends it, is
    /// `COPY ... FROM STDIN`. A send command gives psql only the buffer
    /// text, and `ExecQueryAndProcessResults` counts the COPY commands of the
    /// text with one `psql_scan` call, which stops at the first `;` that `\;`
    /// put in the text.
    first_statement_copies: Option<bool>,
}

impl ScanState {
    /// `psql_scan_reset`: the state of an empty query buffer.
    ///
    /// psql sets `init_idents_count` to zero but keeps the words of
    /// `init_idents`, so a `;` with no word before it reads the words of the
    /// statement before it. The reset here does the same.
    pub(crate) fn reset(&mut self) {
        *self = Self {
            words: self.words,
            ..Self::default()
        };
    }

    /// Whether a `;` here ends the query buffer.
    pub(crate) fn is_top_level(&self) -> bool {
        self.paren_depth == 0 && self.begin_depth == 0
    }

    /// Whether the buffer is inside parentheses or inside a tracked `BEGIN`
    /// block at the end of the document. psql then asks for another line
    /// (`PROMPT_PAREN`, `PROMPT_CONTINUE`).
    pub(crate) fn is_open(&self) -> bool {
        !self.is_top_level()
    }

    /// Reads one atom of SQL text. `after` is the source text that follows
    /// the atom.
    pub(crate) fn atom(&mut self, atom: &SqlAtom<'_>, after: &str) {
        match atom {
            SqlAtom::Punct(token) => {
                // psqlscan.l's `"("` and `")"` rules. A `Punct` run holds
                // each parenthesis that `{self}` would otherwise match.
                for character in token.text().chars() {
                    match character {
                        '(' => self.paren_depth += 1,
                        ')' => self.paren_depth = self.paren_depth.saturating_sub(1),
                        _ => {}
                    }
                }
            }
            SqlAtom::Word(token) => {
                let word = token.text();
                // `{xnstart}` matches `n'` and `{xufailed}` matches `u&`; each
                // gives back all but the first letter, which psql echoes as
                // it stands, not as an `{identifier}`.
                let lone = word.len() == 1;
                if lone && word.eq_ignore_ascii_case("n") && after.starts_with('\'') {
                    return;
                }
                if lone && word.eq_ignore_ascii_case("u") && after.starts_with('&') {
                    return;
                }
                self.identifier(word);
            }
            // psqlscan.l's `"\\"[;:]` rule: at the top level, `\;` ends a
            // statement of the buffer but not the buffer.
            SqlAtom::Escaped(token) if token.text() == "\\;" && self.is_top_level() => {
                // The scan of the buffer text again starts with no words, so a
                // statement with no word is not a COPY there.
                let first = self.word_count > 0 && self.is_copy_from_stdin();
                self.first_statement_copies.get_or_insert(first);
                if self.is_copy_from_stdin() {
                    self.copy_count += 1;
                }
                self.word_count = 0;
            }
            _ => {}
        }
    }

    /// Reads a `;`. Returns whether it ends the query buffer.
    pub(crate) fn semicolon(&mut self) -> bool {
        if !self.is_top_level() {
            return false;
        }
        if self.is_copy_from_stdin() {
            self.copy_count += 1;
        }
        self.word_count = 0;
        true
    }

    /// The number of `COPY ... FROM STDIN` commands that psql counts when a
    /// `;` sends the buffer (`psql_scan_count_copy_from_stdin`, called after
    /// [`ScanState::semicolon`]).
    pub(crate) fn copy_count(&self) -> u32 {
        self.copy_count
    }

    /// The number of `COPY ... FROM STDIN` commands that psql counts when a
    /// send command, such as `\g`, sends the buffer: `ExecQueryAndProcessResults`
    /// scans the buffer text again, up to its first statement boundary.
    pub(crate) fn send_copy_count(&self) -> u32 {
        let copies = self
            .first_statement_copies
            .unwrap_or_else(|| self.word_count > 0 && self.is_copy_from_stdin());
        u32::from(copies)
    }

    /// `psqlscan_track_identifier`.
    fn identifier(&mut self, word: &str) {
        // None of this happens inside parentheses.
        if self.paren_depth != 0 {
            return;
        }
        if self.word_count == 0 {
            self.words = [0; INITIAL_WORDS];
        }
        // `psqlscan_record_initial_keyword`.
        if self.word_count < INITIAL_WORDS {
            let is = |keyword: &str| word.eq_ignore_ascii_case(keyword);
            let first = word.as_bytes()[0];
            self.words[self.word_count] =
                if is("create") || is("function") || is("procedure") || is("or") || is("replace") {
                    first.to_ascii_lowercase()
                } else if is("copy") || is("from") || is("stdin") || is("stdout") {
                    first.to_ascii_uppercase()
                } else {
                    0
                };
            self.word_count += 1;
        }
        if self.is_create_routine() {
            if word.eq_ignore_ascii_case("begin") {
                self.begin_depth += 1;
            } else if word.eq_ignore_ascii_case("case") {
                // CASE also ends with END, but it only counts inside BEGIN.
                if self.begin_depth >= 1 {
                    self.begin_depth += 1;
                }
            } else if word.eq_ignore_ascii_case("end") && self.begin_depth > 0 {
                self.begin_depth -= 1;
            }
        }
    }

    /// `psqlscan_is_create_routine`: `CREATE [OR REPLACE] {FUNCTION|PROCEDURE}`.
    fn is_create_routine(&self) -> bool {
        let words = &self.words;
        words[0] == b'c'
            && (words[1] == b'f'
                || words[1] == b'p'
                || (words[1] == b'o' && words[2] == b'r' && (words[3] == b'f' || words[3] == b'p')))
    }

    /// `psqlscan_is_copy_from_stdin`: `COPY`, then `FROM` within the next
    /// words, then `STDIN` or `STDOUT`.
    fn is_copy_from_stdin(&self) -> bool {
        let words = &self.words;
        if words[0] != b'C' {
            return false;
        }
        (1..INITIAL_WORDS - 1)
            .find(|&at| words[at] == b'F')
            .is_some_and(|at| words[at + 1] == b'S')
    }
}

/// Whether the text after the last token of a document ends inside a block
/// comment. The comments are ignored trivia, so the token stream does not
/// show them. `trailing` is whitespace and comments only.
pub(crate) fn ends_in_open_comment(trailing: &str) -> bool {
    let bytes = trailing.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at..].starts_with(b"--") {
            match bytes[at..].iter().position(|byte| *byte == b'\n') {
                Some(end) => at += end + 1,
                None => return false,
            }
        } else if bytes[at..].starts_with(b"/*") {
            // psqlscan.l's `xc`: comments nest.
            let mut depth = 1;
            at += 2;
            while depth > 0 {
                if at >= bytes.len() {
                    return true;
                }
                if bytes[at..].starts_with(b"/*") {
                    depth += 1;
                    at += 2;
                } else if bytes[at..].starts_with(b"*/") {
                    depth -= 1;
                    at += 2;
                } else {
                    at += 1;
                }
            }
        } else {
            at += 1;
        }
    }
    false
}
