//! Types: the `pg_type` rows that the model keeps.

use crate::collation::Collation;
use crate::extension::Extension;
use crate::handle::Handle;
use crate::name::Name;
use crate::oid::Oid;
use crate::operator::Operator;
use crate::relation::Relation;
use crate::schema::Schema;
use crate::typmod::Typmod;

/// A PostgreSQL data type: one `pg_type` row.
///
/// A type is a base, composite, domain, enum, range, multirange, array, or
/// pseudo-type. The [`kind`](Type::kind) field holds the detail of each.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Type {
    /// The server's identifier for this type.
    pub oid: Oid,
    /// The name of the type, as `typname` holds it.
    pub name: Name,
    /// The schema that holds this type.
    pub schema: Handle<Schema>,
    /// The `typcategory` of the type.
    pub category: TypeCategory,
    /// True when the type is the preferred type of its category.
    pub is_preferred: bool,
    /// True when the type takes a collation, that is `typcollation <> 0`.
    pub is_collatable: bool,
    /// The collation of the type, from `typcollation`.
    ///
    /// For a collatable base type this is the `default` collation, which
    /// stands for the collation of the database. A domain can name another
    /// one. It is `None` for a type that is not collatable. It is also `None`
    /// when the catalog does not hold the collation; the load then has a
    /// note, and [`is_collatable`](Type::is_collatable) is still true.
    #[cfg_attr(feature = "serde", serde(default))]
    pub collation: Option<Handle<Collation>>,
    /// The array type that has this type as its element.
    ///
    /// This is `typarray`. It is `None` for an array type, and `None` when
    /// the server records `0`.
    pub array: Option<Handle<Type>>,
    /// The kind of the type and the detail that belongs to that kind.
    pub kind: TypeKind,
    /// The comparison operators of the default btree operator class of this
    /// type, when the type has one.
    ///
    /// See [`Catalog::btree_ordering`](crate::Catalog::btree_ordering) for
    /// the rules.
    #[cfg_attr(feature = "serde", serde(default))]
    pub btree: Option<BtreeOperators>,
    /// The `=` operator of the default hash operator class of this type,
    /// when the type has one.
    ///
    /// See [`Catalog::hash_equality`](crate::Catalog::hash_equality) for the
    /// rules.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hash_equality: Option<Handle<Operator>>,
    /// The extension that owns this type, when one does.
    pub extension: Option<Handle<Extension>>,
}

/// The five comparison operators that a default btree operator class gives
/// a type.
///
/// Each operator takes the type on both sides. The strategy numbers are those
/// of `pg_amop.amopstrategy` for a btree. PostgreSQL sorts with `lt` and `gt`
/// and groups with `eq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BtreeOperators {
    /// The `<` operator. Strategy 1.
    pub lt: Handle<Operator>,
    /// The `<=` operator. Strategy 2.
    pub le: Handle<Operator>,
    /// The `=` operator. Strategy 3.
    pub eq: Handle<Operator>,
    /// The `>=` operator. Strategy 4.
    pub ge: Handle<Operator>,
    /// The `>` operator. Strategy 5.
    pub gt: Handle<Operator>,
}

/// The kind of a type, with the detail that belongs to it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum TypeKind {
    /// A base type. The server implements it in C or in SQL.
    Base,
    /// An array type.
    Array {
        /// The type of one element.
        element: Handle<Type>,
    },
    /// A composite type: a row with named fields.
    Composite(CompositeDetail),
    /// A domain: a type that wraps another type and adds constraints.
    Domain(DomainDetail),
    /// An enum type.
    Enum {
        /// The labels, in `enumsortorder`.
        labels: Vec<Name>,
    },
    /// A range type.
    Range(RangeDetail),
    /// A multirange type.
    Multirange {
        /// The range type that this multirange holds.
        range: Handle<Type>,
    },
    /// A pseudo-type.
    Pseudo(PseudoType),
}

/// The detail of a composite type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompositeDetail {
    /// The fields, in attribute order.
    pub fields: Vec<Field>,
    /// The relation that has this composite as its row type, when one does.
    pub relation: Option<Handle<Relation>>,
}

/// One member of a composite type.
///
/// A field belongs to a type. A [`crate::Column`] belongs to a relation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Field {
    /// The name of the field.
    pub name: Name,
    /// The type of the field.
    pub ty: Handle<Type>,
    /// The type modifier of the field.
    pub typmod: Typmod,
}

/// The detail of a domain.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DomainDetail {
    /// The type that the domain wraps.
    pub base: Handle<Type>,
    /// The type modifier that the domain applies to the base type.
    pub typmod: Typmod,
    /// True when the domain declares `NOT NULL`.
    pub not_null: bool,
    /// The number of `CHECK` constraints on the domain.
    ///
    /// The model does not keep the text of a constraint.
    pub check_constraints: u32,
}

/// The detail of a range type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RangeDetail {
    /// The type of the range bounds.
    pub subtype: Handle<Type>,
    /// The multirange type that belongs to this range.
    pub multirange: Handle<Type>,
}

/// The `typcategory` of a type.
///
/// `Other` holds a letter that PostgreSQL does not document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum TypeCategory {
    /// Array types. Letter `A`.
    Array,
    /// Boolean types. Letter `B`.
    Boolean,
    /// Composite types. Letter `C`.
    Composite,
    /// Date and time types. Letter `D`.
    DateTime,
    /// Enum types. Letter `E`.
    Enum,
    /// Geometric types. Letter `G`.
    Geometric,
    /// Network address types. Letter `I`.
    Network,
    /// Numeric types. Letter `N`.
    Numeric,
    /// Pseudo-types. Letter `P`.
    Pseudo,
    /// Range types. Letter `R`.
    Range,
    /// String types. Letter `S`.
    String,
    /// Timespan types. Letter `T`.
    Timespan,
    /// User-defined types. Letter `U`.
    UserDefined,
    /// Bit-string types. Letter `V`.
    BitString,
    /// The `unknown` category. Letter `X`.
    Unknown,
    /// Internal-use types. Letter `Z`.
    Internal,
    /// A letter that PostgreSQL does not document.
    Other(char),
}

impl TypeCategory {
    /// Read a `typcategory` letter.
    pub fn from_char(c: char) -> Self {
        match c {
            'A' => TypeCategory::Array,
            'B' => TypeCategory::Boolean,
            'C' => TypeCategory::Composite,
            'D' => TypeCategory::DateTime,
            'E' => TypeCategory::Enum,
            'G' => TypeCategory::Geometric,
            'I' => TypeCategory::Network,
            'N' => TypeCategory::Numeric,
            'P' => TypeCategory::Pseudo,
            'R' => TypeCategory::Range,
            'S' => TypeCategory::String,
            'T' => TypeCategory::Timespan,
            'U' => TypeCategory::UserDefined,
            'V' => TypeCategory::BitString,
            'X' => TypeCategory::Unknown,
            'Z' => TypeCategory::Internal,
            other => TypeCategory::Other(other),
        }
    }

    /// The `typcategory` letter of this category.
    pub fn as_char(self) -> char {
        match self {
            TypeCategory::Array => 'A',
            TypeCategory::Boolean => 'B',
            TypeCategory::Composite => 'C',
            TypeCategory::DateTime => 'D',
            TypeCategory::Enum => 'E',
            TypeCategory::Geometric => 'G',
            TypeCategory::Network => 'I',
            TypeCategory::Numeric => 'N',
            TypeCategory::Pseudo => 'P',
            TypeCategory::Range => 'R',
            TypeCategory::String => 'S',
            TypeCategory::Timespan => 'T',
            TypeCategory::UserDefined => 'U',
            TypeCategory::BitString => 'V',
            TypeCategory::Unknown => 'X',
            TypeCategory::Internal => 'Z',
            TypeCategory::Other(c) => c,
        }
    }
}

/// A type that stands for a family of types or for a special value.
///
/// The model keeps the 26 pseudo-types of PostgreSQL 14 through 19. `Other`
/// holds the name of a pseudo-type that a later server adds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum PseudoType {
    /// `any`.
    Any,
    /// `anyelement`.
    AnyElement,
    /// `anyarray`.
    AnyArray,
    /// `anynonarray`.
    AnyNonArray,
    /// `anyenum`.
    AnyEnum,
    /// `anyrange`.
    AnyRange,
    /// `anymultirange`.
    AnyMultirange,
    /// `anycompatible`.
    AnyCompatible,
    /// `anycompatiblearray`.
    AnyCompatibleArray,
    /// `anycompatiblenonarray`.
    AnyCompatibleNonArray,
    /// `anycompatiblerange`.
    AnyCompatibleRange,
    /// `anycompatiblemultirange`.
    AnyCompatibleMultirange,
    /// `record`.
    Record,
    /// `_record`, the array of `record`.
    RecordArray,
    /// `cstring`.
    Cstring,
    /// `internal`.
    Internal,
    /// `void`.
    Void,
    /// `unknown`.
    Unknown,
    /// `trigger`.
    Trigger,
    /// `event_trigger`.
    EventTrigger,
    /// `language_handler`.
    LanguageHandler,
    /// `fdw_handler`.
    FdwHandler,
    /// `index_am_handler`.
    IndexAmHandler,
    /// `tsm_handler`.
    TsmHandler,
    /// `table_am_handler`.
    TableAmHandler,
    /// `pg_ddl_command`.
    PgDdlCommand,
    /// A pseudo-type that this model does not name.
    Other(Name),
}

impl PseudoType {
    /// Read a `typname` of a pseudo-type.
    ///
    /// A name that this model does not know becomes [`PseudoType::Other`].
    pub fn from_name(name: &str) -> Self {
        match name {
            "any" => PseudoType::Any,
            "anyelement" => PseudoType::AnyElement,
            "anyarray" => PseudoType::AnyArray,
            "anynonarray" => PseudoType::AnyNonArray,
            "anyenum" => PseudoType::AnyEnum,
            "anyrange" => PseudoType::AnyRange,
            "anymultirange" => PseudoType::AnyMultirange,
            "anycompatible" => PseudoType::AnyCompatible,
            "anycompatiblearray" => PseudoType::AnyCompatibleArray,
            "anycompatiblenonarray" => PseudoType::AnyCompatibleNonArray,
            "anycompatiblerange" => PseudoType::AnyCompatibleRange,
            "anycompatiblemultirange" => PseudoType::AnyCompatibleMultirange,
            "record" => PseudoType::Record,
            "_record" => PseudoType::RecordArray,
            "cstring" => PseudoType::Cstring,
            "internal" => PseudoType::Internal,
            "void" => PseudoType::Void,
            "unknown" => PseudoType::Unknown,
            "trigger" => PseudoType::Trigger,
            "event_trigger" => PseudoType::EventTrigger,
            "language_handler" => PseudoType::LanguageHandler,
            "fdw_handler" => PseudoType::FdwHandler,
            "index_am_handler" => PseudoType::IndexAmHandler,
            "tsm_handler" => PseudoType::TsmHandler,
            "table_am_handler" => PseudoType::TableAmHandler,
            "pg_ddl_command" => PseudoType::PgDdlCommand,
            other => PseudoType::Other(Name::new(other)),
        }
    }

    /// True for the `any` family.
    ///
    /// A polymorphic pseudo-type stands for a type that the call site
    /// supplies. The other pseudo-types stand for a special value.
    pub fn is_polymorphic(&self) -> bool {
        matches!(
            self,
            PseudoType::Any
                | PseudoType::AnyElement
                | PseudoType::AnyArray
                | PseudoType::AnyNonArray
                | PseudoType::AnyEnum
                | PseudoType::AnyRange
                | PseudoType::AnyMultirange
                | PseudoType::AnyCompatible
                | PseudoType::AnyCompatibleArray
                | PseudoType::AnyCompatibleNonArray
                | PseudoType::AnyCompatibleRange
                | PseudoType::AnyCompatibleMultirange
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{PseudoType, TypeCategory};

    const DOCUMENTED: &[(char, TypeCategory)] = &[
        ('A', TypeCategory::Array),
        ('B', TypeCategory::Boolean),
        ('C', TypeCategory::Composite),
        ('D', TypeCategory::DateTime),
        ('E', TypeCategory::Enum),
        ('G', TypeCategory::Geometric),
        ('I', TypeCategory::Network),
        ('N', TypeCategory::Numeric),
        ('P', TypeCategory::Pseudo),
        ('R', TypeCategory::Range),
        ('S', TypeCategory::String),
        ('T', TypeCategory::Timespan),
        ('U', TypeCategory::UserDefined),
        ('V', TypeCategory::BitString),
        ('X', TypeCategory::Unknown),
        ('Z', TypeCategory::Internal),
    ];

    #[test]
    fn every_documented_letter_makes_a_round_trip() {
        for (letter, category) in DOCUMENTED {
            assert_eq!(TypeCategory::from_char(*letter), *category);
            assert_eq!(category.as_char(), *letter);
        }
    }

    #[test]
    fn an_unknown_letter_stays_as_it_is() {
        assert_eq!(TypeCategory::from_char('Q'), TypeCategory::Other('Q'));
        assert_eq!(TypeCategory::Other('Q').as_char(), 'Q');
    }

    const PSEUDO_NAMES: &[(&str, PseudoType)] = &[
        ("any", PseudoType::Any),
        ("anyelement", PseudoType::AnyElement),
        ("anyarray", PseudoType::AnyArray),
        ("anynonarray", PseudoType::AnyNonArray),
        ("anyenum", PseudoType::AnyEnum),
        ("anyrange", PseudoType::AnyRange),
        ("anymultirange", PseudoType::AnyMultirange),
        ("anycompatible", PseudoType::AnyCompatible),
        ("anycompatiblearray", PseudoType::AnyCompatibleArray),
        ("anycompatiblenonarray", PseudoType::AnyCompatibleNonArray),
        ("anycompatiblerange", PseudoType::AnyCompatibleRange),
        (
            "anycompatiblemultirange",
            PseudoType::AnyCompatibleMultirange,
        ),
        ("record", PseudoType::Record),
        ("_record", PseudoType::RecordArray),
        ("cstring", PseudoType::Cstring),
        ("internal", PseudoType::Internal),
        ("void", PseudoType::Void),
        ("unknown", PseudoType::Unknown),
        ("trigger", PseudoType::Trigger),
        ("event_trigger", PseudoType::EventTrigger),
        ("language_handler", PseudoType::LanguageHandler),
        ("fdw_handler", PseudoType::FdwHandler),
        ("index_am_handler", PseudoType::IndexAmHandler),
        ("tsm_handler", PseudoType::TsmHandler),
        ("table_am_handler", PseudoType::TableAmHandler),
        ("pg_ddl_command", PseudoType::PgDdlCommand),
    ];

    #[test]
    fn all_twenty_six_pseudo_type_names_map() {
        assert_eq!(PSEUDO_NAMES.len(), 26);
        for (name, pseudo) in PSEUDO_NAMES {
            assert_eq!(&PseudoType::from_name(name), pseudo, "name {name}");
        }
    }

    #[test]
    fn an_unknown_pseudo_type_name_is_kept() {
        assert_eq!(
            PseudoType::from_name("future_thing"),
            PseudoType::Other("future_thing".into())
        );
    }

    #[test]
    fn only_the_any_family_is_polymorphic() {
        for (name, pseudo) in PSEUDO_NAMES {
            assert_eq!(
                pseudo.is_polymorphic(),
                name.starts_with("any"),
                "name {name}"
            );
        }
        assert!(!PseudoType::Other("future_thing".into()).is_polymorphic());
    }
}
