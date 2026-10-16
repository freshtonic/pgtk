//! Built-in types: the closed set of types with a fixed OID.
//!
//! The [`Builtin`] enum itself comes from `src/builtin/generated.rs`. The
//! `gen-builtins` task writes that file from `pg_type.dat`. This file holds
//! the implementations that a person writes by hand.

use crate::oid::Oid;

include!("builtin/generated.rs");

impl Builtin {
    /// The built-in type with this OID, when there is one.
    ///
    /// The search is a binary search over [`Builtin::ALL`], which the
    /// generator puts in OID order.
    pub fn from_oid(oid: Oid) -> Option<Builtin> {
        Builtin::ALL
            .binary_search_by_key(&oid, |b| b.oid())
            .ok()
            .map(|index| Builtin::ALL[index])
    }
}

#[cfg(test)]
mod tests {
    use super::Builtin;
    use crate::oid::Oid;

    #[test]
    fn all_is_in_discriminant_order() {
        for (index, builtin) in Builtin::ALL.iter().enumerate() {
            assert_eq!(*builtin as usize, index, "builtin {builtin:?}");
        }
    }

    #[test]
    fn all_is_in_oid_order() {
        for pair in Builtin::ALL.windows(2) {
            assert!(pair[0].oid() < pair[1].oid(), "pair {pair:?}");
        }
    }

    #[test]
    fn from_oid_is_the_inverse_of_oid() {
        for builtin in Builtin::ALL {
            assert_eq!(Builtin::from_oid(builtin.oid()), Some(*builtin));
        }
    }

    #[test]
    fn an_unknown_oid_has_no_builtin() {
        assert_eq!(Builtin::from_oid(Oid(999_999)), None);
        assert_eq!(Builtin::from_oid(Oid(0)), None);
    }

    #[test]
    fn names_match_the_server() {
        assert_eq!(Builtin::Int4.name(), "int4");
        assert_eq!(Builtin::Int4.oid(), Oid(23));
        assert_eq!(Builtin::Int4Array.name(), "_int4");
        assert_eq!(Builtin::RecordArray.name(), "_record");
    }
}
