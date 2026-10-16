//! Operators.

use crate::extension::Extension;
use crate::function::Function;
use crate::handle::Handle;
use crate::name::Name;
use crate::oid::Oid;
use crate::schema::Schema;
use crate::types::Type;

/// A `pg_operator` row.
///
/// An operator is a named symbol with an optional left operand type, a right
/// operand type, a result type, and a function that does the work. The model
/// holds no operator family. An [`OperatorClass`](crate::OperatorClass) holds
/// no member. The operators that the default btree and hash operator classes
/// give a [`Type`] are in [`Type::btree`] and [`Type::hash_equality`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Operator {
    /// The server's identifier for this operator.
    pub oid: Oid,
    /// The symbol of the operator, such as `+`.
    pub name: Name,
    /// The schema that holds this operator.
    pub schema: Handle<Schema>,
    /// The kind of the operator.
    pub kind: OperatorKind,
    /// The type of the left operand, for an infix operator.
    pub left: Option<Handle<Type>>,
    /// The type of the right operand.
    pub right: Handle<Type>,
    /// The type of the result.
    pub result: Handle<Type>,
    /// The function that implements the operator.
    pub function: Handle<Function>,
    /// The commutator of this operator, when the catalog holds one.
    pub commutator: Option<Handle<Operator>>,
    /// The negator of this operator, when the catalog holds one.
    pub negator: Option<Handle<Operator>>,
    /// The extension that owns this operator, when one does.
    pub extension: Option<Handle<Extension>>,
}

/// The kind of an operator, from `oprkind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OperatorKind {
    /// A prefix operator, which takes one operand. `oprkind` is `l`.
    Prefix,
    /// An infix operator, which takes two operands. `oprkind` is `b`.
    Infix,
}
