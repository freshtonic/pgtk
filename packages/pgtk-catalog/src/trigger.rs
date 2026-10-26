//! Triggers.

use crate::name::Name;
use crate::rule::FiresWhen;

/// A trigger of a relation: one `pg_trigger` row.
///
/// The model holds what PostgreSQL's rewriter needs to decide if a statement
/// on a view is permitted. It holds no function, no column list, and no
/// `WHEN` condition of the trigger. An internal trigger, which the server
/// makes for a constraint, is not here.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Trigger {
    /// The name of the trigger. The triggers of one kind fire in name order.
    pub name: Name,
    /// When the trigger fires in relation to the statement.
    pub timing: TriggerTiming,
    /// True when the trigger fires for an `INSERT`.
    pub insert: bool,
    /// True when the trigger fires for an `UPDATE`.
    pub update: bool,
    /// True when the trigger fires for a `DELETE`.
    pub delete: bool,
    /// True when the trigger fires for a `TRUNCATE`.
    pub truncate: bool,
    /// True for a `FOR EACH ROW` trigger. False for a `FOR EACH STATEMENT`
    /// trigger.
    pub row_level: bool,
    /// When the trigger fires, from `tgenabled`.
    ///
    /// PostgreSQL does not read this setting when it decides if a view is
    /// updatable: a disabled `INSTEAD OF` trigger still makes the view
    /// updatable.
    pub fires: FiresWhen,
}

/// The timing of a trigger, from the bits of `tgtype`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum TriggerTiming {
    /// `BEFORE`. Bit 2 is set.
    Before,
    /// `AFTER`. Bit 2 and bit 64 are clear.
    After,
    /// `INSTEAD OF`. Bit 64 is set. Only a view can have one.
    InsteadOf,
}

impl Trigger {
    /// The bit of `tgtype` for `FOR EACH ROW`.
    const ROW: i16 = 1;
    /// The bit of `tgtype` for `BEFORE`.
    const BEFORE: i16 = 2;
    /// The bit of `tgtype` for `INSERT`.
    const INSERT: i16 = 4;
    /// The bit of `tgtype` for `DELETE`.
    const DELETE: i16 = 8;
    /// The bit of `tgtype` for `UPDATE`.
    const UPDATE: i16 = 16;
    /// The bit of `tgtype` for `TRUNCATE`.
    const TRUNCATE: i16 = 32;
    /// The bit of `tgtype` for `INSTEAD OF`.
    const INSTEAD: i16 = 64;

    /// Make a trigger from the `tgtype` bits of the server.
    pub fn from_tgtype(name: Name, tgtype: i16, fires: FiresWhen) -> Self {
        let has = |bit: i16| tgtype & bit != 0;
        let timing = if has(Self::INSTEAD) {
            TriggerTiming::InsteadOf
        } else if has(Self::BEFORE) {
            TriggerTiming::Before
        } else {
            TriggerTiming::After
        };
        Trigger {
            name,
            timing,
            insert: has(Self::INSERT),
            update: has(Self::UPDATE),
            delete: has(Self::DELETE),
            truncate: has(Self::TRUNCATE),
            row_level: has(Self::ROW),
            fires,
        }
    }

    /// True for an `INSTEAD OF` trigger `FOR EACH ROW`.
    ///
    /// A view with such a trigger for a statement kind is updatable for that
    /// kind, whatever its query is.
    pub fn is_instead_of_row(&self) -> bool {
        self.timing == TriggerTiming::InsteadOf && self.row_level
    }
}

#[cfg(test)]
mod tests {
    use super::{Trigger, TriggerTiming};
    use crate::name::Name;
    use crate::rule::FiresWhen;

    #[test]
    fn the_bits_of_tgtype_give_the_timing_and_the_events() {
        // INSTEAD OF INSERT OR UPDATE FOR EACH ROW: 64 + 16 + 4 + 1.
        let trigger = Trigger::from_tgtype(Name::new("t"), 85, FiresWhen::Origin);
        assert_eq!(trigger.timing, TriggerTiming::InsteadOf);
        assert!(trigger.insert && trigger.update && trigger.row_level);
        assert!(!trigger.delete && !trigger.truncate);
        assert!(trigger.is_instead_of_row());

        // BEFORE DELETE FOR EACH ROW: 8 + 2 + 1.
        let trigger = Trigger::from_tgtype(Name::new("t"), 11, FiresWhen::Origin);
        assert_eq!(trigger.timing, TriggerTiming::Before);
        assert!(trigger.delete && trigger.row_level);
        assert!(!trigger.is_instead_of_row());

        // AFTER TRUNCATE FOR EACH STATEMENT: 32.
        let trigger = Trigger::from_tgtype(Name::new("t"), 32, FiresWhen::Origin);
        assert_eq!(trigger.timing, TriggerTiming::After);
        assert!(trigger.truncate && !trigger.row_level);
    }
}
