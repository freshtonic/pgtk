//! Relations and their columns.

use crate::extension::Extension;
use crate::handle::Handle;
use crate::name::Name;
use crate::oid::Oid;
use crate::schema::Schema;
use crate::types::Type;
use crate::typmod::Typmod;

/// A `pg_class` row that has columns and can be named in a `FROM` clause.
///
/// A sequence, an index, and a TOAST table are not relations in this model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Relation {
    /// The server's identifier for this relation.
    pub oid: Oid,
    /// The name of the relation.
    pub name: Name,
    /// The schema that holds this relation.
    pub schema: Handle<Schema>,
    /// The kind of the relation.
    pub kind: RelationKind,
    /// The composite type of one row of this relation.
    ///
    /// A sequence has no row type in `pg_type`, so this is `None` for a
    /// [`RelationKind::Sequence`]. It is `Some` for each other kind.
    pub row_type: Option<Handle<Type>>,
    /// The live columns, in attribute order.
    pub columns: Vec<Column>,
    /// The extension that owns this relation, when one does.
    pub extension: Option<Handle<Extension>>,
}

impl Relation {
    /// The column with this name, when the relation has one.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name.as_str() == name)
    }
}

/// The kind of a relation, from `relkind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RelationKind {
    /// An ordinary table. `relkind` is `r`.
    Table,
    /// A view. `relkind` is `v`.
    View,
    /// A materialized view. `relkind` is `m`.
    MaterializedView,
    /// A foreign table. `relkind` is `f`.
    ForeignTable,
    /// A partitioned table. `relkind` is `p`.
    PartitionedTable,
    /// A sequence. `relkind` is `S`.
    ///
    /// The columns are `last_value`, `log_cnt`, and `is_called`. The server
    /// gives a sequence no row type, so [`Relation::row_type`] is `None`. The
    /// model holds no parameter of the sequence, such as the start or the
    /// increment.
    Sequence,
}

/// One live attribute of a relation.
///
/// A dropped attribute is not a column.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Column {
    /// The name of the column.
    pub name: Name,
    /// The attribute number, which starts at 1.
    pub position: i16,
    /// The type of the column.
    pub ty: Handle<Type>,
    /// The type modifier of the column.
    pub typmod: Typmod,
    /// True when the column declares `NOT NULL`.
    pub not_null: bool,
    /// True when the column has a default expression.
    ///
    /// The model does not keep the text of the expression.
    pub has_default: bool,
    /// True when the column is a generated column.
    pub generated: bool,
    /// True when the column is an identity column.
    pub identity: bool,
}
