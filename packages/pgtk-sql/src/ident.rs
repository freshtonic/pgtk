//! Identifier folding: the name PostgreSQL's scanner gives an identifier token.
//!
//! pgtk-sql keeps the source spelling of every identifier, quotes included. A
//! consumer that looks names up in a catalog needs the spelling PostgreSQL
//! stores, and the rules for that live in the scanner, not in the grammar:
//!
//! - `scan.l` `{identifier}`: `downcase_truncate_identifier`.
//! - `scan.l` `<xd>{xdstop}`: the text between the quotes with `""` read as
//!   `"`, then `truncate_identifier`.
//! - `scan.l` `<xui>{dquote}` and `parser.c` `base_yylex` `case UIDENT`: the
//!   same unquoting, then `str_udeescape`, then `truncate_identifier`.
//!
//! The functions work on token text alone, because that is all the flat AST
//! holds. They assume a UTF-8 server encoding: `downcase_identifier`
//! (`scansup.c`) downcases a byte with the high bit set only in a single-byte
//! encoding, so here every non-ASCII character stays as written.
//!
//! The scanner rejects a zero-length quoted identifier (`""`, `U&""`).
//! pgtk-sql's lexer accepts one, so [`fold_identifier`] reports it.

use std::borrow::Cow;
use std::fmt;

/// PostgreSQL's `NAMEDATALEN`. A name holds at most `NAMEDATALEN - 1` bytes.
pub const NAMEDATALEN: usize = 64;

/// The default Unicode escape character of a `U&"..."` identifier.
pub const DEFAULT_UESCAPE: &str = "\\";

/// An identifier as PostgreSQL's scanner reports it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Folded<'a> {
    /// The folded name. It borrows from the source text when folding changed
    /// nothing but the extent.
    pub name: Cow<'a, str>,
    /// The name was longer than `NAMEDATALEN - 1` bytes and was cut.
    /// PostgreSQL raises the NOTICE `identifier "..." will be truncated to
    /// "..."` (`scansup.c` `truncate_identifier`) when this happens.
    pub truncated: bool,
}

/// An identifier that PostgreSQL's scanner rejects. `Display` gives
/// PostgreSQL's message text (`scan.l`, `parser.c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoldError {
    /// `""` or `U&""` (`scan.l` `<xd>{xdstop}` and `<xui>{dquote}`).
    ZeroLength,
    /// The `UESCAPE` value is not one byte, or is a hexadecimal digit, `+`,
    /// `'`, `"` or whitespace (`check_uescapechar`).
    InvalidEscapeCharacter,
    /// The escape character does not start `XXXX`, `+XXXXXX` or a second
    /// escape character.
    InvalidEscape,
    /// The escape names code point 0 or one above U+10FFFF
    /// (`check_unicode_value`).
    InvalidEscapeValue,
    /// A UTF-16 surrogate without its partner.
    InvalidSurrogatePair,
}

impl fmt::Display for FoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ZeroLength => "zero-length delimited identifier",
            Self::InvalidEscapeCharacter => "invalid Unicode escape character",
            Self::InvalidEscape => "invalid Unicode escape",
            Self::InvalidEscapeValue => "invalid Unicode escape value",
            Self::InvalidSurrogatePair => "invalid Unicode surrogate pair",
        })
    }
}

impl std::error::Error for FoldError {}

/// Fold the source text of one identifier token, with the default escape
/// character `\` for the `U&"..."` form.
///
/// `raw` is the text pgtk-sql's identifier nodes hold: an unquoted word (a
/// keyword in an identifier position included), `"..."` or `U&"..."`.
pub fn fold_identifier(raw: &str) -> Result<Folded<'_>, FoldError> {
    fold_identifier_with_uescape(raw, DEFAULT_UESCAPE)
}

/// Fold the source text of one identifier token whose `U&"..."` form is
/// followed by `UESCAPE 'x'`.
///
/// pgtk-sql parses the `UESCAPE` clause as a separate suffix, so the caller
/// supplies it: `uescape` is the value of the clause's string constant, the
/// text between its quotes. It has an effect only on the `U&"..."` form, but
/// it is checked for every form.
pub fn fold_identifier_with_uescape<'a>(
    raw: &'a str,
    uescape: &str,
) -> Result<Folded<'a>, FoldError> {
    let escape = escape_byte(uescape.as_bytes())?;
    let name = if let Some(inner) = delimited(raw, 0) {
        if inner.is_empty() {
            return Err(FoldError::ZeroLength);
        }
        unquote(inner)
    } else if let Some(inner) = unicode_delimited(raw) {
        if inner.is_empty() {
            return Err(FoldError::ZeroLength);
        }
        match unquote(inner) {
            Cow::Borrowed(text) => decode_escapes(text, escape)?,
            Cow::Owned(text) => Cow::Owned(decode_escapes(&text, escape)?.into_owned()),
        }
    } else {
        downcase(raw)
    };
    Ok(truncate(name))
}

/// Fold the source text of one identifier token for a declared
/// classification, which compares the folded name with the names a `gram.y`
/// action reads (`#[classify(...)]`, Recursa `docs/declared-rejections.md`).
///
/// Returns `None` when the scanner rejects the text, because such a token
/// names nothing.
pub fn fold_name(raw: &str) -> Option<Cow<'_, str>> {
    fold_identifier(raw).ok().map(|folded| folded.name)
}

/// Fold the source text of one identifier token as [`fold_name`] does, and
/// then make it lower case, for a `gram.y` action that compares the name with
/// `pg_strcasecmp`.
///
/// Returns `None` when the scanner rejects the text.
pub fn fold_name_caseless(raw: &str) -> Option<String> {
    fold_name(raw).map(|folded| folded.to_ascii_lowercase())
}

/// Fold the source text of one gram.y `RoleSpec` word for the declared
/// classification of `RoleId` and `RoleSpec` (REL_17_11 gram.y 17305-17385).
///
/// The actions read two things: the keyword of a pseudo-role
/// (`CURRENT_ROLE`, `CURRENT_USER`, `SESSION_USER`), and the names `public`
/// and `none`, which they compare as strings. So an unquoted pseudo-role
/// keyword folds to its upper-case spelling, a name that folds to `public` or
/// `none` folds to that name, and every other name folds to the empty text.
/// No identifier folds to the empty text, and a quoted `"CURRENT_USER"` is a
/// plain name, so the classes do not overlap.
///
/// Returns `None` when the scanner rejects the text.
pub fn fold_role_name(raw: &str) -> Option<&'static str> {
    const PSEUDO_ROLES: [&str; 3] = ["CURRENT_ROLE", "CURRENT_USER", "SESSION_USER"];
    if let Some(keyword) = PSEUDO_ROLES
        .into_iter()
        .find(|keyword| raw.eq_ignore_ascii_case(keyword))
    {
        return Some(keyword);
    }
    Some(match fold_name(raw)?.as_ref() {
        "public" => "public",
        "none" => "none",
        _ => "",
    })
}

/// Classify an identifier spelling for the `UESCAPE` rule of
/// `parser.c`: `"unicode"` for a `U&"..."` identifier, `"plain"` for each
/// other spelling.
pub fn unicode_identifier_class(raw: &str) -> Option<&'static str> {
    let bytes = raw.as_bytes();
    let unicode = bytes.len() > 2 && bytes[0].eq_ignore_ascii_case(&b'u') && bytes[1] == b'&';
    Some(if unicode { "unicode" } else { "plain" })
}

/// Fold a keyword that stands in an identifier position: its lowercase
/// spelling, which is the text `ScanKeywordLookup` hands the grammar.
///
/// Returns `None` when `kind` is not a keyword: a content token has no fixed
/// spelling, and punctuation has one that is not a word. A keyword token's
/// source text folds to the same name through [`fold_identifier`]; this entry
/// point serves a caller that holds the kind and not the text.
pub fn fold_keyword(kind: crate::TokenKind) -> Option<Folded<'static>> {
    let spelling = kind.static_text()?;
    spelling
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        .then(|| truncate(downcase(spelling)))
}

/// Classify the source text of the string after `UESCAPE` for the
/// `UESCAPE` rule of `parser.c` (`base_yylex`): `"valid"` when the value of
/// the string is one byte that `check_uescapechar` admits, else `"invalid"`.
///
/// The text is a spelling of `crate::ast::shared::expr::UescapeString`; its
/// value is [`sconst_bytes`].
pub fn uescape_class(raw: &str) -> Option<&'static str> {
    let valid = sconst_bytes(raw).is_some_and(|value| escape_byte(&value).is_ok());
    Some(if valid { "valid" } else { "invalid" })
}

/// The value of an `SCONST` token as `scan.l` builds it, in bytes.
///
/// The spellings are the plain `'...'` string, a continued string
/// (`quotecontinue`: more `'...'` parts after a newline), an `E'...'` string
/// (also continued, with the escapes of `xe` in each part) and a
/// dollar-quoted string. The result is `None` for another spelling, and for
/// an `E'...'` escape that `scan.l` refuses (`xeunicodefail`, a code point
/// that is not valid, or a surrogate that is not in a pair).
///
/// pgtk-sql lexes as if `standard_conforming_strings` is on, so a backslash
/// in a plain string is an ordinary character.
pub fn sconst_bytes(raw: &str) -> Option<Vec<u8>> {
    match raw.as_bytes().first()? {
        b'\'' => quoted_parts(raw.as_bytes(), false),
        b'E' | b'e' => quoted_parts(&raw.as_bytes()[1..], true),
        b'$' => {
            let close = raw[1..].find('$')? + 1;
            let tag = &raw[..=close];
            let body = raw[close + 1..].strip_suffix(tag)?;
            Some(body.as_bytes().to_vec())
        }
        _ => None,
    }
}

/// The parts of a (continued) quoted string, joined. `escapes` selects the
/// `xe` rules (`E'...'`); else `''` is the one escape (`xq`).
fn quoted_parts(bytes: &[u8], escapes: bool) -> Option<Vec<u8>> {
    let mut value = Vec::new();
    let mut at = 0;
    loop {
        if bytes.get(at) != Some(&b'\'') {
            return None;
        }
        at += 1;
        loop {
            match *bytes.get(at)? {
                b'\'' if bytes.get(at + 1) == Some(&b'\'') => {
                    value.push(b'\'');
                    at += 2;
                }
                b'\'' => {
                    at += 1;
                    break;
                }
                b'\\' if escapes => at = escape(bytes, at, &mut value)?,
                byte => {
                    value.push(byte);
                    at += 1;
                }
            }
        }
        // The gap before a continuation: white space and `--` comments. The
        // lexer has already found that it holds a newline.
        while at < bytes.len() && bytes[at] != b'\'' {
            if bytes[at..].starts_with(b"--") {
                while at < bytes.len() && !matches!(bytes[at], b'\n' | b'\r') {
                    at += 1;
                }
            } else if is_scanner_space(bytes[at]) {
                at += 1;
            } else {
                return None;
            }
        }
        if at == bytes.len() {
            return Some(value);
        }
    }
}

/// One `xe` escape at `bytes[at]` (a backslash): `xeoctesc`, `xehexesc`,
/// `xeunicode` or `xeescape` (`unescape_single_char`). Returns the offset
/// after it.
fn escape(bytes: &[u8], at: usize, value: &mut Vec<u8>) -> Option<usize> {
    let next = *bytes.get(at + 1)?;
    let digits = |from: usize, max: usize, radix: u32| {
        bytes[from..]
            .iter()
            .take(max)
            .take_while(|byte| char::from(**byte).is_digit(radix))
            .count()
    };
    let number = |from: usize, len: usize, radix: u32| {
        u32::from_str_radix(std::str::from_utf8(&bytes[from..from + len]).ok()?, radix).ok()
    };
    match next {
        b'0'..=b'7' => {
            let len = digits(at + 1, 3, 8);
            // `unsigned char c = strtoul(...)`: the value is cut to one byte.
            value.push(number(at + 1, len, 8)? as u8);
            Some(at + 1 + len)
        }
        b'x' if digits(at + 2, 2, 16) > 0 => {
            let len = digits(at + 2, 2, 16);
            value.push(number(at + 2, len, 16)? as u8);
            Some(at + 2 + len)
        }
        b'u' | b'U' => {
            let len = if next == b'u' { 4 } else { 8 };
            if digits(at + 2, len, 16) != len {
                return None; // `xeunicodefail`
            }
            let mut code = number(at + 2, len, 16)?;
            let mut end = at + 2 + len;
            if (0xD800..0xDC00).contains(&code) {
                // `xeunicode` in state `xeu`: a low surrogate must follow.
                if bytes.get(end) != Some(&b'\\') {
                    return None;
                }
                let len = match bytes.get(end + 1)? {
                    b'u' => 4,
                    b'U' => 8,
                    _ => return None,
                };
                if digits(end + 2, len, 16) != len {
                    return None;
                }
                let low = number(end + 2, len, 16)?;
                if !(0xDC00..0xE000).contains(&low) {
                    return None;
                }
                code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                end += 2 + len;
            }
            let character = char::from_u32(code).filter(|c| *c != '\0')?;
            value.extend_from_slice(character.encode_utf8(&mut [0; 4]).as_bytes());
            Some(end)
        }
        _ => {
            // `unescape_single_char`. `\v` is an escape from 17 (REL_17_11
            // scan.l 1420; REL_16_15 scan.l has no `case 'v'`).
            value.push(match next {
                b'b' => 0x08,
                b'f' => 0x0c,
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                b'v' if cfg!(feature = "since-pg17") => 0x0b,
                other => other,
            });
            Some(at + 2)
        }
    }
}

/// `check_uescapechar` (`parser.c`), after `strlen(escstr) != 1`.
fn escape_byte(uescape: &[u8]) -> Result<u8, FoldError> {
    match uescape {
        [byte]
            if !(byte.is_ascii_hexdigit()
                || matches!(byte, b'+' | b'\'' | b'"')
                || is_scanner_space(*byte)) =>
        {
            Ok(*byte)
        }
        _ => Err(FoldError::InvalidEscapeCharacter),
    }
}

/// `scanner_isspace` (`scansup.c`).
///
/// Vertical tab (0x0b) is scanner space from 17: research
/// (`docs/research/postgres-14-19-sql-syntax-changes.md`), PostgreSQL 17,
/// "Lexical and literal syntax" (commit ae6d06f0968; REL_16_15 scansup.c 117).
fn is_scanner_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
        || (cfg!(feature = "since-pg17") && byte == 0x0b)
}

/// The text between the quotes of `"..."`, which starts at byte `open`.
fn delimited(raw: &str, open: usize) -> Option<&str> {
    let bytes = raw.as_bytes();
    (bytes.len() >= open + 2 && bytes[open] == b'"' && bytes[bytes.len() - 1] == b'"')
        .then(|| &raw[open + 1..raw.len() - 1])
}

/// The text between the quotes of `U&"..."`.
fn unicode_delimited(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    (bytes.len() >= 2 && bytes[0].eq_ignore_ascii_case(&b'u') && bytes[1] == b'&')
        .then(|| delimited(raw, 2))
        .flatten()
}

/// `scan.l` `<xd,xui>{xddouble}`: `""` stands for one `"`.
fn unquote(inner: &str) -> Cow<'_, str> {
    if inner.contains("\"\"") {
        Cow::Owned(inner.replace("\"\"", "\""))
    } else {
        Cow::Borrowed(inner)
    }
}

/// `downcase_identifier` (`scansup.c`) in a multibyte encoding: ASCII `A`-`Z`
/// only.
fn downcase(raw: &str) -> Cow<'_, str> {
    if raw.bytes().any(|byte| byte.is_ascii_uppercase()) {
        Cow::Owned(raw.to_ascii_lowercase())
    } else {
        Cow::Borrowed(raw)
    }
}

/// `str_udeescape` (`parser.c`).
fn decode_escapes(text: &str, escape: u8) -> Result<Cow<'_, str>, FoldError> {
    let bytes = text.as_bytes();
    if !bytes.contains(&escape) {
        return Ok(Cow::Borrowed(text));
    }
    let mut out = String::with_capacity(text.len());
    let mut pair_first: Option<u32> = None;
    // `escape` is ASCII, so every index this loop reaches is a character
    // boundary of `text`.
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != escape {
            if pair_first.is_some() {
                return Err(FoldError::InvalidSurrogatePair);
            }
            let ch = text[at..].chars().next().expect("index is inside the text");
            out.push(ch);
            at += ch.len_utf8();
        } else if bytes.get(at + 1) == Some(&escape) {
            if pair_first.is_some() {
                return Err(FoldError::InvalidSurrogatePair);
            }
            out.push(char::from(escape));
            at += 2;
        } else {
            let (value, len) = if let Some(value) = hex_value(bytes, at + 1, 4) {
                (value, 5)
            } else if bytes.get(at + 1) == Some(&b'+')
                && let Some(value) = hex_value(bytes, at + 2, 6)
            {
                (value, 8)
            } else {
                return Err(FoldError::InvalidEscape);
            };
            // `is_valid_unicode_codepoint` (`mb/pg_wchar.h`).
            if value == 0 || value > 0x10_FFFF {
                return Err(FoldError::InvalidEscapeValue);
            }
            let is_second = (0xDC00..=0xDFFF).contains(&value);
            let value = match pair_first.take() {
                Some(first) if is_second => ((first & 0x3FF) << 10) + 0x1_0000 + (value & 0x3FF),
                Some(_) => return Err(FoldError::InvalidSurrogatePair),
                None if is_second => return Err(FoldError::InvalidSurrogatePair),
                None => value,
            };
            if (0xD800..=0xDBFF).contains(&value) {
                pair_first = Some(value);
            } else {
                out.push(char::from_u32(value).expect("not a surrogate, not above U+10FFFF"));
            }
            at += len;
        }
    }
    if pair_first.is_some() {
        return Err(FoldError::InvalidSurrogatePair);
    }
    Ok(Cow::Owned(out))
}

/// The value of `digits` hexadecimal digits that start at byte `from`.
fn hex_value(bytes: &[u8], from: usize, digits: usize) -> Option<u32> {
    bytes
        .get(from..from + digits)?
        .iter()
        .try_fold(0u32, |value, byte| {
            Some((value << 4) + char::from(*byte).to_digit(16)?)
        })
}

/// `truncate_identifier` (`scansup.c`): `pg_mbcliplen` to `NAMEDATALEN - 1`
/// bytes, which never cuts inside a character.
fn truncate(name: Cow<'_, str>) -> Folded<'_> {
    if name.len() < NAMEDATALEN {
        return Folded {
            name,
            truncated: false,
        };
    }
    let mut end = NAMEDATALEN - 1;
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    let name = match name {
        Cow::Borrowed(text) => Cow::Borrowed(&text[..end]),
        Cow::Owned(mut text) => {
            text.truncate(end);
            Cow::Owned(text)
        }
    };
    Folded {
        name,
        truncated: true,
    }
}
