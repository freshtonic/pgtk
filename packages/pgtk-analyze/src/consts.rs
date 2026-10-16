//! `make_const`: the type of a literal, as `parse_node.c` decides it.

use pgtk_catalog::Builtin;

use crate::types::Types;
use crate::value::{Form, Typed};

/// The literal kinds `gram.y` produces as `A_Const`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiteralKind {
    /// An integer literal in any of PostgreSQL's radix forms, with a
    /// possible leading minus from `doNegate`.
    Integer,
    /// A numeric literal with a fraction or an exponent.
    Float,
    /// A string literal, of type `unknown` until coerced.
    String,
    /// `TRUE` or `FALSE`.
    Boolean,
    /// `B'...'` or `X'...'`.
    BitString,
    /// `NULL`.
    Null,
}

/// The type `make_const` assigns. `text` is the literal's text without its
/// quotes for a string, and with radix prefix and underscores for a number.
pub(crate) fn make_const(
    types: &Types<'_>,
    kind: LiteralKind,
    text: &str,
    location: usize,
) -> Typed {
    let (ty, form) = match kind {
        LiteralKind::Null => (
            types.builtin(Builtin::Unknown),
            Form::Const {
                is_null: true,
                text: None,
            },
        ),
        LiteralKind::Integer | LiteralKind::Float => {
            let ty = match integer_value(text) {
                Some(v) if i32::try_from(v).is_ok() => types.builtin(Builtin::Int4),
                Some(_) => types.builtin(Builtin::Int8),
                None => types.builtin(Builtin::Numeric),
            };
            (
                ty,
                Form::Const {
                    is_null: false,
                    text: Some(text.to_owned()),
                },
            )
        }
        LiteralKind::Boolean => (
            types.builtin(Builtin::Bool),
            Form::Const {
                is_null: false,
                text: Some(text.to_owned()),
            },
        ),
        LiteralKind::String => (
            types.builtin(Builtin::Unknown),
            Form::Const {
                is_null: false,
                text: Some(text.to_owned()),
            },
        ),
        LiteralKind::BitString => (
            types.builtin(Builtin::Bit),
            Form::Const {
                is_null: false,
                text: Some(text.to_owned()),
            },
        ),
    };
    Typed {
        ty,
        typmod: -1,
        form,
        location,
        returns_set: false,
    }
}

/// `pg_strtoint64_safe` over the scanner's integer forms: decimal, `0x`,
/// `0o`, `0b`, with `_` separators, and an optional leading minus. `None`
/// when the text is not an integer or does not fit in 64 bits.
pub(crate) fn integer_value(text: &str) -> Option<i64> {
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (radix, digits) = match rest.get(..2).map(str::to_ascii_lowercase).as_deref() {
        Some("0x") => (16, &rest[2..]),
        Some("0o") => (8, &rest[2..]),
        Some("0b") => (2, &rest[2..]),
        _ => (10, rest),
    };
    if digits.is_empty() {
        return None;
    }
    let mut value: i128 = 0;
    for c in digits.chars() {
        if c == '_' {
            continue;
        }
        let d = c.to_digit(radix)? as i128;
        value = value * radix as i128 + d;
        if value > i64::MAX as i128 + 1 {
            return None;
        }
    }
    let value = if negative { -value } else { value };
    i64::try_from(value).ok()
}
