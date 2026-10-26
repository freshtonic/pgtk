//! Diagnostics: notes about entities that the loader could not fully keep.

use std::fmt;

use crate::name::Name;
use crate::oid::Oid;

/// A note about an entity that the loader could not fully represent.
///
/// A diagnostic never stops a load. The loader either keeps the entity as it
/// is or skips it, and records the reason here.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Diagnostic {
    /// The `typtype` letter is not one of `b c d e p r m`.
    UnknownTypeKind {
        /// The OID of the type.
        oid: Oid,
        /// The name of the type.
        name: Name,
        /// The letter that the server gave.
        typtype: char,
    },
    /// An entity named an OID that the catalog does not hold.
    ///
    /// The referenced entity was absent, or the loader skipped it.
    MissingReference {
        /// The kind of the entity that holds the reference.
        entity: EntityKind,
        /// The OID of the entity that holds the reference.
        oid: Oid,
        /// The name of the entity that holds the reference.
        name: Name,
        /// The kind of the entity that the reference names.
        referenced: EntityKind,
        /// The OID that the reference names.
        referenced_oid: Oid,
    },
    /// A value in a row had a form that the loader could not read.
    ///
    /// The loader skipped the entity.
    UnparsableValue {
        /// The kind of the entity.
        entity: EntityKind,
        /// The OID of the entity.
        oid: Oid,
        /// The name of the column that held the value.
        column: &'static str,
        /// The text of the value.
        value: String,
    },
    /// The loader skipped an entity for a stated reason.
    Skipped {
        /// The kind of the entity.
        entity: EntityKind,
        /// The OID of the entity.
        oid: Oid,
        /// The name of the entity.
        name: Name,
        /// Why the loader skipped it.
        reason: SkipReason,
    },
}

/// The kind of an entity that a diagnostic names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EntityKind {
    /// A schema.
    Schema,
    /// An extension.
    Extension,
    /// A type.
    Type,
    /// A relation.
    Relation,
    /// A column of a relation.
    Column,
    /// A function.
    Function,
    /// An operator.
    Operator,
    /// A cast.
    Cast,
    /// A default operator class.
    OperatorClass,
    /// A rewrite rule.
    Rule,
    /// A trigger.
    Trigger,
    /// A configuration parameter.
    Parameter,
    /// A collation.
    Collation,
    /// A constraint of a relation.
    Constraint,
}

impl EntityKind {
    /// The name of the kind, in lower case.
    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::Schema => "schema",
            EntityKind::Extension => "extension",
            EntityKind::Type => "type",
            EntityKind::Relation => "relation",
            EntityKind::Column => "column",
            EntityKind::Function => "function",
            EntityKind::Operator => "operator",
            EntityKind::Cast => "cast",
            EntityKind::OperatorClass => "operator class",
            EntityKind::Rule => "rule",
            EntityKind::Trigger => "trigger",
            EntityKind::Parameter => "parameter",
            EntityKind::Collation => "collation",
            EntityKind::Constraint => "constraint",
        }
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why the loader skipped an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SkipReason {
    /// The operator has no function, so it is a shell operator.
    ShellOperator,
    /// The entity depends on an entity that the loader skipped.
    DependsOnSkipped,
    /// The `relkind` is one that this model does not keep.
    UnsupportedRelationKind(char),
    /// A default btree operator class does not give its input type each of
    /// the five comparison operators.
    IncompleteOperatorClass,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkipReason::ShellOperator => f.write_str("it is a shell operator"),
            SkipReason::DependsOnSkipped => f.write_str("it depends on a skipped entity"),
            SkipReason::UnsupportedRelationKind(kind) => {
                write!(f, "relkind {kind} is not supported")
            }
            SkipReason::IncompleteOperatorClass => {
                f.write_str("it does not have each of the five btree operators")
            }
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Diagnostic::UnknownTypeKind { oid, name, typtype } => {
                write!(f, "type {name} (oid {oid}) has unknown typtype {typtype}")
            }
            Diagnostic::MissingReference {
                entity,
                oid,
                name,
                referenced,
                referenced_oid,
            } => write!(
                f,
                "{entity} {name} (oid {oid}) names {referenced} oid {referenced_oid}, which the catalog does not hold"
            ),
            Diagnostic::UnparsableValue {
                entity,
                oid,
                column,
                value,
            } => write!(
                f,
                "{entity} oid {oid} has an unreadable {column}: {value:?}"
            ),
            Diagnostic::Skipped {
                entity,
                oid,
                name,
                reason,
            } => write!(f, "skipped {entity} {name} (oid {oid}): {reason}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, EntityKind, SkipReason};
    use crate::oid::Oid;

    #[test]
    fn display_names_the_entity_and_the_reason() {
        let diagnostic = Diagnostic::Skipped {
            entity: EntityKind::Operator,
            oid: Oid(42),
            name: "+".into(),
            reason: SkipReason::ShellOperator,
        };
        assert_eq!(
            diagnostic.to_string(),
            "skipped operator + (oid 42): it is a shell operator"
        );
    }

    #[test]
    fn display_reports_a_missing_reference() {
        let diagnostic = Diagnostic::MissingReference {
            entity: EntityKind::Function,
            oid: Oid(1),
            name: "f".into(),
            referenced: EntityKind::Type,
            referenced_oid: Oid(2),
        };
        assert_eq!(
            diagnostic.to_string(),
            "function f (oid 1) names type oid 2, which the catalog does not hold"
        );
    }

    #[test]
    fn display_reports_an_unreadable_value() {
        let diagnostic = Diagnostic::UnparsableValue {
            entity: EntityKind::Column,
            oid: Oid(7),
            column: "atttypmod",
            value: "abc".to_owned(),
        };
        assert_eq!(
            diagnostic.to_string(),
            "column oid 7 has an unreadable atttypmod: \"abc\""
        );
    }

    #[test]
    fn display_reports_an_unknown_type_kind() {
        let diagnostic = Diagnostic::UnknownTypeKind {
            oid: Oid(9),
            name: "odd".into(),
            typtype: 'q',
        };
        assert_eq!(
            diagnostic.to_string(),
            "type odd (oid 9) has unknown typtype q"
        );
    }
}
