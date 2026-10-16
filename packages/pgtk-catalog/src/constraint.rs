//! Constraints of a relation.

use crate::name::Name;

/// A constraint of a relation: one `pg_constraint` row.
///
/// Parse analysis reads a constraint only by its name, for
/// `ON CONFLICT ON CONSTRAINT name`. The model holds the name, the kind, the
/// deferrable flag, and the columns. It holds no expression of a `CHECK`, no
/// target of a foreign key, and no index.
///
/// A constraint of a domain is not here.
/// [`DomainDetail::check_constraints`](crate::DomainDetail::check_constraints)
/// counts those.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Constraint {
    /// The name of the constraint. It is unique among the constraints of one
    /// relation.
    pub name: Name,
    /// The kind of the constraint.
    pub kind: ConstraintKind,
    /// True when the constraint is `DEFERRABLE`.
    pub deferrable: bool,
    /// The attribute positions of the columns of the constraint, in the
    /// order of `conkey`.
    ///
    /// A position is a [`Column::position`](crate::Column::position). An
    /// exclusion constraint has the position 0 for an element that is an
    /// expression. The list is empty for a constraint with no column, such
    /// as a `CHECK` that names none.
    pub columns: Vec<i16>,
}

/// The kind of a constraint, from `contype`.
///
/// PostgreSQL 18 stores a `NOT NULL` constraint of a relation in
/// `pg_constraint`. PostgreSQL 14 to 17 do not, so there is no kind for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConstraintKind {
    /// `PRIMARY KEY`. The letter is `p`.
    PrimaryKey,
    /// `UNIQUE`. The letter is `u`.
    Unique,
    /// `CHECK`. The letter is `c`.
    Check,
    /// `FOREIGN KEY`. The letter is `f`.
    ForeignKey,
    /// `EXCLUDE`. The letter is `x`.
    Exclusion,
    /// A constraint trigger. The letter is `t`.
    ConstraintTrigger,
}

impl ConstraintKind {
    /// True for a kind that has an index, and thus can be the arbiter of an
    /// `ON CONFLICT` clause: a primary key, a unique constraint, and an
    /// exclusion constraint.
    pub fn has_index(self) -> bool {
        matches!(
            self,
            ConstraintKind::PrimaryKey | ConstraintKind::Unique | ConstraintKind::Exclusion
        )
    }
}
