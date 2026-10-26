//! Constraints of a relation.

use crate::name::Name;

/// A constraint of a relation: one `pg_constraint` row.
///
/// Parse analysis reads a constraint only by its name, for
/// `ON CONFLICT ON CONSTRAINT name`. The model holds the name, the kind, the
/// flags, and the columns. It holds no expression of a `CHECK`, no target of
/// a foreign key, and no index.
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
    /// False when the constraint is `NOT ENFORCED`.
    ///
    /// This is `conenforced`. PostgreSQL 18 added `NOT ENFORCED` for a
    /// `CHECK` and for a foreign key. PostgreSQL 14 to 17 enforce each
    /// constraint, so the loader gives `true` for each constraint of those
    /// versions.
    pub enforced: bool,
    /// True when the last column of the constraint has `WITHOUT OVERLAPS`
    /// (a primary key or a unique constraint) or `PERIOD` (a foreign key).
    ///
    /// This is `conperiod`. PostgreSQL 18 added these temporal constraints,
    /// so the loader gives `false` for each constraint of PostgreSQL 14 to
    /// 17. The index of a `WITHOUT OVERLAPS` constraint is an exclusion
    /// index.
    pub period: bool,
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
/// A newer PostgreSQL can add a kind, so the enum is `#[non_exhaustive]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
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
    /// `NOT NULL`. The letter is `n`.
    ///
    /// PostgreSQL 18 and later. From version 18, the server keeps each
    /// `NOT NULL` constraint of a relation in `pg_constraint`, with a name,
    /// and [`Constraint::columns`] holds its one column. PostgreSQL 14 to 17
    /// keep only the flag, so a catalog of those versions holds no
    /// constraint of this kind. [`Column::not_null`](crate::Column::not_null)
    /// is correct for each version.
    NotNull,
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
