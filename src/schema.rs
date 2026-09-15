//! Schemas: the containers that qualify entity names.

use crate::extension::Extension;
use crate::handle::Handle;
use crate::name::Name;
use crate::oid::Oid;

/// A `pg_namespace` row.
///
/// A schema is the container that qualifies the name of an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Schema {
    /// The server's identifier for this schema.
    pub oid: Oid,
    /// The name of the schema.
    pub name: Name,
    /// The extension that owns this schema, when one does.
    pub extension: Option<Handle<Extension>>,
}
