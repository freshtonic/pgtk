//! Operator classes.

use crate::extension::Extension;
use crate::handle::Handle;
use crate::name::Name;
use crate::oid::Oid;
use crate::schema::Schema;
use crate::types::Type;

/// A `pg_opclass` row.
///
/// Parse analysis finds an operator class by its name and its access method,
/// for `ON CONFLICT (column opclass)` and for an index definition. The model
/// holds no operator family and no member of the class.
/// [`Catalog::btree_ordering`](crate::Catalog::btree_ordering) and
/// [`Catalog::hash_equality`](crate::Catalog::hash_equality) give the
/// operators of a default class.
///
/// A name is unique for one schema and one access method. `int4_ops` is the
/// name of a `btree` class and of a `hash` class.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OperatorClass {
    /// The server's identifier for this operator class.
    pub oid: Oid,
    /// The name of the operator class.
    pub name: Name,
    /// The schema that holds this operator class.
    pub schema: Handle<Schema>,
    /// The name of the access method, such as `btree` or `gist`.
    pub method: Name,
    /// The type of the data that the class indexes, from `opcintype`.
    pub input_type: Handle<Type>,
    /// True when the class is the default for its input type and its access
    /// method.
    pub is_default: bool,
    /// The extension that owns this operator class, when one does.
    pub extension: Option<Handle<Extension>>,
}
