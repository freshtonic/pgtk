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
    /// The direct parents of the relation, in `inhseqno` order.
    ///
    /// This is `pg_inherits`. A table that inherits names its parents here,
    /// and a partition names its partitioned table. The list is empty for a
    /// relation with no parent. It holds only the direct parents: a consumer
    /// that needs each ancestor follows the chain.
    ///
    /// PostgreSQL accepts the row type of a relation where it expects the row
    /// type of an ancestor of that relation.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parents: Vec<Handle<Relation>>,
    /// The primary key of the relation, when it has one.
    ///
    /// No other constraint is in the model.
    #[cfg_attr(feature = "serde", serde(default))]
    pub primary_key: Option<PrimaryKey>,
    /// The extension that owns this relation, when one does.
    pub extension: Option<Handle<Extension>>,
}

impl Relation {
    /// The column with this name, when the relation has one.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name.as_str() == name)
    }

    /// The column at this attribute position, when the relation has one.
    ///
    /// The position is [`Column::position`]. It is not an index into
    /// [`columns`](Relation::columns), because a dropped attribute leaves a
    /// gap in the positions.
    pub fn column_at(&self, position: i16) -> Option<&Column> {
        self.columns.iter().find(|c| c.position == position)
    }
}

/// The primary key of a relation: a `pg_constraint` row with `contype` `p`.
///
/// PostgreSQL reads the primary key in its check of an ungrouped column.
/// When each column of the key is in `GROUP BY`, each other column of the
/// relation is functionally dependent on the key, and thus is grouped too.
/// PostgreSQL applies this rule only when the key is not deferrable. See
/// [`grouping_columns`](PrimaryKey::grouping_columns).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PrimaryKey {
    /// The attribute positions of the key columns, in key order.
    ///
    /// This is `conkey`. Each position is the [`Column::position`] of a
    /// column of the relation. Key order can be different from column order.
    pub columns: Vec<i16>,
    /// True when the constraint is `DEFERRABLE`.
    ///
    /// A deferrable key can hold duplicates inside a transaction, so
    /// PostgreSQL does not use it to prove a functional dependency.
    pub deferrable: bool,
}

impl PrimaryKey {
    /// The key columns that prove a functional dependency in `GROUP BY`.
    ///
    /// This is `None` for a deferrable key, as in `check_functional_grouping`
    /// of PostgreSQL.
    pub fn grouping_columns(&self) -> Option<&[i16]> {
        (!self.deferrable).then_some(self.columns.as_slice())
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
    /// The kind of identity column, when the column is one.
    ///
    /// This is `attidentity`. It is `None` for a column that is not an
    /// identity column.
    pub identity: Option<Identity>,
}

/// The kind of an identity column, from `attidentity`.
///
/// The two kinds are different for an `INSERT` and for an `UPDATE`.
/// PostgreSQL accepts only `DEFAULT` for a [`Identity::Always`] column, unless
/// the statement says `OVERRIDING SYSTEM VALUE`. It gives SQLSTATE `428C9`
/// for another value. A [`Identity::ByDefault`] column accepts each value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Identity {
    /// `GENERATED ALWAYS AS IDENTITY`. `attidentity` is `a`.
    Always,
    /// `GENERATED BY DEFAULT AS IDENTITY`. `attidentity` is `d`.
    ByDefault,
}
