//! Meta-command names, arguments and argument values.
//!
//! A meta-command is one token (`crate::ast::MetaCommand`). This module
//! divides the token into the parts that `psqlscanslash.l` reads: the
//! command name (`xslashcmd`), each argument (`xslashargstart`, `xslasharg`
//! and the quote states), and a trailing `\\` (`xslashend`). The token
//! pattern has already decided where the command ends, so the division here
//! only finds the boundaries inside a token that is known to be complete.
//!
//! [`Argument::value`] computes the value that `psql_scan_slash_option`
//! returns for an `OT_NORMAL` argument. pgtk-psql uses the values of `\set`,
//! `\unset`, `\if` and `\elif` only. For every other command the arguments
//! stay text (`packages/pgtk-sql/docs/adr/0013-keep-most-meta-command-arguments-as-text.md`).

use crate::substitution::{Variables, quote_identifier, quote_literal};

/// How a command reads its arguments, from `exec_command` in
/// `src/bin/psql/command.c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentKind {
    /// The command name is not known. psql drops the rest of the line;
    /// pgtk-psql keeps the name only.
    None,
    /// `OT_NORMAL` arguments, separated by `space`.
    Normal,
    /// One `OT_WHOLE_LINE` argument: the rest of the line.
    WholeLine,
    /// One `OT_FILEPIPE` argument, which is the rest of the line when it
    /// starts with `|`, and then `OT_NORMAL` arguments.
    FilePipe,
    /// The arguments of `\g` and `\gx`: an optional parenthesised option
    /// list, then an `OT_FILEPIPE` argument.
    Send,
}

/// One meta-command, divided as `psqlscanslash.l` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command<'input> {
    text: &'input str,
    kind: ArgumentKind,
}

/// One argument of a meta-command, exactly as it is spelled in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Argument<'input> {
    text: &'input str,
    whole_line: bool,
}

/// `space` of `psqlscanslash.l`, without the line break. From 17 it also
/// holds the vertical tab (docs/research/psql-14-19-syntax-changes.md, item
/// A4).
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\x0c' | b'\n')
        || (cfg!(feature = "since-pg17") && byte == b'\x0b')
}

impl<'input> Command<'input> {
    pub(crate) fn new(text: &'input str, kind: ArgumentKind) -> Self {
        Self { text, kind }
    }

    /// The whole command: the backslash, the name, the arguments and a
    /// trailing `\\`, if there is one.
    pub fn text(&self) -> &'input str {
        self.text
    }

    /// How the command reads its arguments.
    pub fn argument_kind(&self) -> ArgumentKind {
        self.kind
    }

    /// The backslash and the name.
    pub fn head(&self) -> &'input str {
        let bytes = self.text.as_bytes();
        let end = 1 + bytes[1..]
            .iter()
            .position(|byte| is_space(*byte) || *byte == b'\\')
            .unwrap_or(bytes.len() - 1);
        &self.text[..end]
    }

    /// The command name without the backslash, as `psql_scan_slash_command`
    /// returns it.
    pub fn name(&self) -> &'input str {
        &self.head()[1..]
    }

    /// The `\\` that ends the command, if there is one. psql then reads SQL
    /// text again on the same line.
    pub fn separator(&self) -> Option<&'input str> {
        if self.kind == ArgumentKind::None || self.kind == ArgumentKind::WholeLine {
            return None;
        }
        let rest = &self.text[self.head().len()..];
        let consumed = self.scan().1;
        let tail = &rest[consumed..];
        tail.starts_with("\\\\").then(|| &tail[..2])
    }

    /// The arguments, in order.
    pub fn arguments(&self) -> Vec<Argument<'input>> {
        self.scan().0
    }

    /// Reads the arguments after the name. Returns them and the number of
    /// bytes they take after the name, trailing space included.
    fn scan(&self) -> (Vec<Argument<'input>>, usize) {
        let rest = &self.text[self.head().len()..];
        let bytes = rest.as_bytes();
        let mut arguments = Vec::new();
        let mut at = 0;
        match self.kind {
            ArgumentKind::None => return (arguments, 0),
            ArgumentKind::WholeLine => {
                // `xslashwholeline` drops the leading space and keeps the
                // rest of the line.
                while at < bytes.len() && is_space(bytes[at]) {
                    at += 1;
                }
                if at < bytes.len() {
                    arguments.push(Argument {
                        text: &rest[at..],
                        whole_line: true,
                    });
                }
                return (arguments, bytes.len());
            }
            ArgumentKind::Normal | ArgumentKind::FilePipe | ArgumentKind::Send => {}
        }

        // `\g (options)`: the first argument opens the option list, and the
        // list ends with the first argument that ends with `)`
        // (`process_command_g_options`). The file or pipe argument follows.
        let mut file_pipe = matches!(self.kind, ArgumentKind::FilePipe | ArgumentKind::Send);
        let mut in_options = false;
        loop {
            while at < bytes.len() && is_space(bytes[at]) {
                at += 1;
            }
            if at >= bytes.len() || bytes[at] == b'\\' {
                break;
            }
            if file_pipe && !in_options && bytes[at] == b'|' {
                arguments.push(Argument {
                    text: &rest[at..],
                    whole_line: true,
                });
                at = bytes.len();
                break;
            }
            let start = at;
            at = argument_end(bytes, at);
            let argument = &rest[start..at];
            arguments.push(Argument {
                text: argument,
                whole_line: false,
            });
            if self.kind == ArgumentKind::Send && arguments.len() == 1 && argument.starts_with('(')
            {
                in_options = true;
            }
            if in_options {
                if argument.ends_with(')') {
                    in_options = false;
                }
            } else {
                file_pipe = false;
            }
        }
        (arguments, at)
    }
}

/// The end of the `xslasharg` argument that starts at `at`: the first
/// unquoted `space` or backslash.
fn argument_end(bytes: &[u8], mut at: usize) -> usize {
    while at < bytes.len() {
        match bytes[at] {
            byte if is_space(byte) || byte == b'\\' => break,
            b'\'' => {
                // `xslashquote`: a backslash escapes the next character.
                at += 1;
                while at < bytes.len() && bytes[at] != b'\'' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
                at += 1;
            }
            quote @ (b'"' | b'`') => {
                at += 1;
                while at < bytes.len() && bytes[at] != quote {
                    at += 1;
                }
                at += 1;
            }
            _ => at += 1,
        }
    }
    at.min(bytes.len())
}

/// The characters of a psql variable name, `variable_char` of
/// `psqlscanslash.l`: `[A-Za-z\200-\377_0-9]`.
fn is_variable_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || !character.is_ascii()
}

/// Whether `name` is a name that `\set` accepts (`valid_variable_name` in
/// `src/bin/psql/variables.c`).
pub fn is_valid_variable_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(is_variable_char)
}

impl<'input> Argument<'input> {
    /// The exact source spelling.
    pub fn text(&self) -> &'input str {
        self.text
    }

    /// Whether the argument is the rest of the line (`OT_WHOLE_LINE`, or an
    /// `OT_FILEPIPE` argument that starts with `|`).
    pub fn is_whole_line(&self) -> bool {
        self.whole_line
    }

    /// The value that `psql_scan_slash_option` returns for this argument as
    /// an `OT_NORMAL` option, with `variables` bound.
    ///
    /// - `'...'` is removed, with `''` and the backslash escapes of
    ///   `xslashquote` (`\n`, `\t`, `\b`, `\r`, `\f`, octal, hexadecimal, and
    ///   any other character after a backslash);
    /// - `"..."` stays as it is, quotes included;
    /// - `:name` is the value of the variable, `:'name'` and `:"name"` are
    ///   the quoted value, and `:{?name}` is `TRUE` or `FALSE`. An unset
    ///   variable stays as it is spelled.
    ///
    /// `None` when the argument holds a backquoted shell command, which psql
    /// runs and pgtk-psql never runs. A whole-line argument is returned as
    /// it is spelled.
    pub fn value(&self, variables: &Variables) -> Option<String> {
        if self.whole_line {
            return Some(self.text.to_owned());
        }
        let text = self.text;
        let mut value = Vec::<u8>::with_capacity(text.len());
        let mut rest = text;
        while let Some(character) = rest.chars().next() {
            match character {
                '\'' => {
                    let (quoted, after) = single_quoted(&rest[1..]);
                    value.extend_from_slice(&quoted);
                    rest = after;
                }
                '"' => {
                    let end = rest[1..].find('"').map_or(rest.len(), |at| at + 2);
                    value.extend_from_slice(&rest.as_bytes()[..end]);
                    rest = &rest[end..];
                }
                '`' => return None,
                ':' => {
                    let (substituted, after) = colon(rest, variables);
                    value.extend_from_slice(substituted.as_bytes());
                    rest = after;
                }
                _ => {
                    let width = character.len_utf8();
                    value.extend_from_slice(&rest.as_bytes()[..width]);
                    rest = &rest[width..];
                }
            }
        }
        Some(String::from_utf8_lossy(&value).into_owned())
    }
}

/// The content of an `xslashquote` string whose opening quote is gone, and
/// the text after its closing quote.
fn single_quoted(text: &str) -> (Vec<u8>, &str) {
    let bytes = text.as_bytes();
    let mut value = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\'' if bytes.get(at + 1) == Some(&b'\'') => {
                value.push(b'\'');
                at += 2;
            }
            b'\'' => return (value, &text[at + 1..]),
            b'\\' if at + 1 < bytes.len() => {
                let next = bytes[at + 1];
                match next {
                    b'n' => value.push(b'\n'),
                    b't' => value.push(b'\t'),
                    b'b' => value.push(b'\x08'),
                    b'r' => value.push(b'\r'),
                    b'f' => value.push(b'\x0c'),
                    b'0'..=b'7' => {
                        // `{xeoctesc}`: one to three octal digits.
                        let digits = bytes[at + 1..]
                            .iter()
                            .take(3)
                            .take_while(|byte| (b'0'..=b'7').contains(*byte))
                            .count();
                        let number = &text[at + 1..at + 1 + digits];
                        value.push(u32::from_str_radix(number, 8).unwrap_or(0) as u8);
                        at += 1 + digits;
                        continue;
                    }
                    b'x' if bytes.get(at + 2).is_some_and(u8::is_ascii_hexdigit) => {
                        // `{xehexesc}`: one or two hexadecimal digits.
                        let digits = bytes[at + 2..]
                            .iter()
                            .take(2)
                            .take_while(|byte| byte.is_ascii_hexdigit())
                            .count();
                        let number = &text[at + 2..at + 2 + digits];
                        value.push(u8::from_str_radix(number, 16).unwrap_or(0));
                        at += 2 + digits;
                        continue;
                    }
                    _ => {
                        // `"\\".`: the character itself, which can be a
                        // multibyte one.
                        let width = text[at + 1..].chars().next().map_or(1, char::len_utf8);
                        value.extend_from_slice(&bytes[at + 1..at + 1 + width]);
                        at += 1 + width;
                        continue;
                    }
                }
                at += 2;
            }
            byte => {
                value.push(byte);
                at += 1;
            }
        }
    }
    (value, "")
}

/// An `xslasharg` text that starts with `:`: an interpolation, or a colon
/// that starts none. Returns the text it adds and the text after it.
fn colon<'text>(text: &'text str, variables: &Variables) -> (String, &'text str) {
    let after = &text[1..];
    let name_length = |text: &str| {
        text.char_indices()
            .find(|(_, character)| !is_variable_char(*character))
            .map_or(text.len(), |(at, _)| at)
    };
    // `:'name'` and `:"name"`.
    for (quote, escape) in [
        ('\'', quote_literal as fn(&str) -> String),
        ('"', quote_identifier as fn(&str) -> String),
    ] {
        if let Some(inner) = after.strip_prefix(quote) {
            let length = name_length(inner);
            if length > 0 && inner[length..].starts_with(quote) {
                let spelling = &text[..length + 3];
                let value = variables
                    .get(&inner[..length])
                    .map_or_else(|| spelling.to_owned(), escape);
                return (value, &text[length + 3..]);
            }
            // An incomplete form gives back everything but the colon.
            return (":".to_owned(), after);
        }
    }
    // `:{?name}`.
    if let Some(inner) = after.strip_prefix("{?") {
        let length = name_length(inner);
        if length > 0 && inner[length..].starts_with('}') {
            let set = variables.contains(&inner[..length]);
            let value = if set { "TRUE" } else { "FALSE" };
            return (value.to_owned(), &text[length + 4..]);
        }
        return (":".to_owned(), after);
    }
    // `:name`.
    let length = name_length(after);
    if length > 0 {
        let spelling = &text[..length + 1];
        let value = variables
            .get(&after[..length])
            .map_or_else(|| spelling.to_owned(), str::to_owned);
        return (value, &text[length + 1..]);
    }
    (":".to_owned(), after)
}

/// Reads a psql Boolean, as `ParseVariableBool` in
/// `src/bin/psql/variables.c` does: any prefix of `true`, `false`, `yes` or
/// `no`, a prefix of at least two characters of `on` or `off`, or `1` or
/// `0`, without regard to case. `None` when the value is none of these.
pub fn parse_bool(value: &str) -> Option<bool> {
    let lower = value.to_ascii_lowercase();
    let prefix_of =
        |word: &str, minimum: usize| lower.len() >= minimum && word.starts_with(lower.as_str());
    if prefix_of("true", 1) || prefix_of("yes", 1) {
        Some(true)
    } else if prefix_of("false", 1) || prefix_of("no", 1) {
        Some(false)
    } else if prefix_of("on", 2) || lower == "1" {
        Some(true)
    } else if prefix_of("off", 2) || lower == "0" {
        Some(false)
    } else {
        None
    }
}
