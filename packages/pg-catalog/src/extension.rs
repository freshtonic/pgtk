//! Extensions: installed packages that own entities.

use crate::handle::Handle;
use crate::name::Name;
use crate::oid::Oid;
use crate::schema::Schema;

/// A `pg_extension` row.
///
/// An extension owns entities. Ownership comes from `pg_depend` rows with
/// dependency type `e`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Extension {
    /// The server's identifier for this extension.
    pub oid: Oid,
    /// The name of the extension.
    pub name: Name,
    /// The installed version, as `extversion` holds it.
    pub version: String,
    /// The schema that holds the objects of the extension.
    pub schema: Handle<Schema>,
}
