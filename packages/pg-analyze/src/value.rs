//! The typed view of an expression that the resolution algorithms work on.
//! Analysis never builds a tree; each algorithm reads the type facts of its
//! operands and reports what it decided as Annotations.

use pg_catalog::{Cast, Function, Handle};

use crate::types::{TypeRef, Typmod};

/// How PostgreSQL would represent the expression, as far as the algorithms
/// care.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// A `Const`. The text is the literal's own text for a string literal,
    /// so a Literal Validator can check it.
    Const { is_null: bool, text: Option<String> },
    /// A `Param`, with its 1-based number.
    Param { number: usize },
    /// A `Var`.
    Var,
    /// Anything else.
    Other,
}

/// The type facts of one expression.
#[derive(Clone, Debug)]
pub(crate) struct Typed {
    pub ty: TypeRef,
    pub typmod: Typmod,
    pub form: Form,
    /// Byte offset of the expression's location, as `exprLocation` gives it.
    pub location: usize,
    /// `expression_returns_set`.
    pub returns_set: bool,
}

impl Typed {
    pub fn is_const(&self) -> bool {
        matches!(self.form, Form::Const { .. })
    }

    pub fn is_null_const(&self) -> bool {
        matches!(self.form, Form::Const { is_null: true, .. })
    }
}

/// How PostgreSQL would coerce a value, as `find_coercion_pathway` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoercionPath {
    /// `RelabelType`: binary-compatible, no work at run time.
    RelabelType,
    /// A cast function.
    Func(Handle<Function>),
    /// Output as text then input as the target type.
    CoerceViaIO,
    /// Element-wise coercion of an array.
    ArrayCoerce,
    /// An `unknown` literal read directly as the target type.
    UnknownLiteral,
    /// A parameter of unknown type took the target type.
    ParamType,
    /// `CoerceToDomain` around a value of the domain's base type.
    Domain,
    /// Inherited or typed-table row conversion.
    ConvertRowtype,
    /// A `record` value coerced to a named composite type.
    RecordToComplex,
}

/// The coercion context, `CoercionContext`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoercionContext {
    /// Applied silently, as for operator and function arguments.
    Implicit,
    /// Applied on assignment to a column.
    Assignment,
    /// Applied only when written as a cast.
    Explicit,
}

/// One coercion Analysis decided to apply to an expression's result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coercion {
    /// The path taken.
    pub path: CoercionPath,
    /// The `pg_cast` row used, when one was.
    pub cast: Option<Handle<Cast>>,
    /// The type the expression has after this coercion.
    pub target: TypeRef,
    /// The typmod it has after this coercion.
    pub typmod: Typmod,
    /// The context that permitted it.
    pub context: CoercionContext,
}
