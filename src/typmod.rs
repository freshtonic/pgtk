//! Type modifiers: the per-column integer that refines a type.

use crate::builtin::Builtin;

/// A raw type modifier.
///
/// The server keeps a type modifier as one integer. Its meaning belongs to
/// the type. `-1` means that the column states no modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Typmod(
    /// The raw value, as `atttypmod` or `typtypmod` holds it.
    pub i32,
);

/// The size of a variable-length header, which PostgreSQL calls `VARHDRSZ`.
const VARHDRSZ: i32 = 4;

impl Typmod {
    /// The modifier that states no modifier.
    pub const NONE: Typmod = Typmod(-1);

    /// True when the modifier states no modifier.
    pub fn is_none(self) -> bool {
        self.0 < 0
    }

    /// Read the modifier for a built-in type.
    ///
    /// The result is `None` when the modifier is [`Typmod::NONE`], and
    /// `None` when the type takes no modifier. An array type takes no
    /// modifier here. The modifier of an array belongs to its element, and
    /// the consumer applies it there.
    pub fn decode(self, builtin: Builtin) -> Option<TypmodDetail> {
        if self.is_none() {
            return None;
        }
        let raw = self.0;
        match builtin {
            Builtin::Bpchar | Builtin::Varchar => {
                Some(TypmodDetail::CharLength((raw - VARHDRSZ).max(0) as u32))
            }
            Builtin::Numeric => {
                let value = raw - VARHDRSZ;
                let precision = ((value >> 16) & 0xffff) as u16;
                let scale = (((value & 0x7ff) ^ 1024) - 1024) as i16;
                Some(TypmodDetail::Numeric { precision, scale })
            }
            Builtin::Bit | Builtin::Varbit => Some(TypmodDetail::BitLength(raw as u32)),
            Builtin::Time | Builtin::Timetz | Builtin::Timestamp | Builtin::Timestamptz => {
                Some(TypmodDetail::TimePrecision(raw as u8))
            }
            Builtin::Interval => {
                let precision_bits = (raw & 0xffff) as u16;
                let field_bits = ((raw >> 16) & 0x7fff) as u16;
                Some(TypmodDetail::Interval {
                    fields: (field_bits != 0x7fff).then_some(IntervalFields(field_bits)),
                    precision: (precision_bits != 0xffff).then_some(precision_bits as u8),
                })
            }
            _ => None,
        }
    }
}

/// The meaning of a type modifier, for a type that has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TypmodDetail {
    /// The declared length of a `bpchar` or a `varchar`, in characters.
    CharLength(u32),
    /// The declared precision and scale of a `numeric`.
    Numeric {
        /// The total number of digits.
        precision: u16,
        /// The number of digits after the decimal point.
        ///
        /// PostgreSQL 15 and later accept a negative scale.
        scale: i16,
    },
    /// The declared length of a `bit` or a `varbit`, in bits.
    BitLength(u32),
    /// The declared fractional-second precision of a time or a timestamp.
    TimePrecision(u8),
    /// The declared fields and precision of an `interval`.
    Interval {
        /// The field mask, when the column states one.
        fields: Option<IntervalFields>,
        /// The fractional-second precision, when the column states one.
        precision: Option<u8>,
    },
}

/// The raw field mask of an `interval` type modifier.
///
/// The bits come from `utils/timestamp.h`. Use the constants below to read
/// the mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IntervalFields(
    /// The raw mask.
    pub u16,
);

impl IntervalFields {
    /// The `YEAR` bit.
    pub const YEAR: u16 = 1 << 2;
    /// The `MONTH` bit.
    pub const MONTH: u16 = 1 << 1;
    /// The `DAY` bit.
    pub const DAY: u16 = 1 << 3;
    /// The `HOUR` bit.
    pub const HOUR: u16 = 1 << 10;
    /// The `MINUTE` bit.
    pub const MINUTE: u16 = 1 << 11;
    /// The `SECOND` bit.
    pub const SECOND: u16 = 1 << 12;

    /// True when the mask holds every bit of `mask`.
    pub fn contains(self, mask: u16) -> bool {
        self.0 & mask == mask
    }
}

#[cfg(test)]
mod tests {
    use super::{IntervalFields, Typmod, TypmodDetail};
    use crate::builtin::Builtin;

    #[test]
    fn none_decodes_to_nothing() {
        assert!(Typmod::NONE.is_none());
        assert_eq!(Typmod::NONE.decode(Builtin::Varchar), None);
        assert_eq!(Typmod::NONE.decode(Builtin::Numeric), None);
    }

    #[test]
    fn varchar_ten_has_typmod_fourteen() {
        assert_eq!(
            Typmod(14).decode(Builtin::Varchar),
            Some(TypmodDetail::CharLength(10))
        );
        assert_eq!(
            Typmod(14).decode(Builtin::Bpchar),
            Some(TypmodDetail::CharLength(10))
        );
    }

    #[test]
    fn numeric_keeps_precision_and_scale() {
        let typmod = Typmod(((10 << 16) | 2) + 4);
        assert_eq!(
            typmod.decode(Builtin::Numeric),
            Some(TypmodDetail::Numeric {
                precision: 10,
                scale: 2
            })
        );
    }

    #[test]
    fn numeric_keeps_a_negative_scale() {
        // PostgreSQL 15 accepts `numeric(10, -2)`. The server stores the
        // scale in eleven bits with a bias of 1024.
        let scale_bits = (-2i32) & 0x7ff;
        let typmod = Typmod(((10 << 16) | scale_bits) + 4);
        assert_eq!(
            typmod.decode(Builtin::Numeric),
            Some(TypmodDetail::Numeric {
                precision: 10,
                scale: -2
            })
        );
    }

    #[test]
    fn timestamp_three_keeps_the_precision() {
        assert_eq!(
            Typmod(3).decode(Builtin::Timestamp),
            Some(TypmodDetail::TimePrecision(3))
        );
        assert_eq!(
            Typmod(6).decode(Builtin::Timetz),
            Some(TypmodDetail::TimePrecision(6))
        );
    }

    #[test]
    fn interval_keeps_the_field_mask() {
        let mask = IntervalFields::DAY
            | IntervalFields::HOUR
            | IntervalFields::MINUTE
            | IntervalFields::SECOND;
        let typmod = Typmod(((mask as i32) << 16) | 0xffff);
        let detail = typmod.decode(Builtin::Interval).unwrap();
        let TypmodDetail::Interval { fields, precision } = detail else {
            panic!("expected an interval detail, got {detail:?}");
        };
        assert_eq!(precision, None);
        let fields = fields.unwrap();
        assert_eq!(fields, IntervalFields(mask));
        assert!(fields.contains(IntervalFields::DAY));
        assert!(fields.contains(IntervalFields::SECOND));
        assert!(!fields.contains(IntervalFields::YEAR));
        assert!(!fields.contains(IntervalFields::MONTH));
    }

    #[test]
    fn interval_keeps_a_precision_with_no_fields() {
        let typmod = Typmod((0x7fff << 16) | 2);
        assert_eq!(
            typmod.decode(Builtin::Interval),
            Some(TypmodDetail::Interval {
                fields: None,
                precision: Some(2)
            })
        );
    }

    #[test]
    fn bit_eight_keeps_the_length() {
        assert_eq!(
            Typmod(8).decode(Builtin::Bit),
            Some(TypmodDetail::BitLength(8))
        );
        assert_eq!(
            Typmod(8).decode(Builtin::Varbit),
            Some(TypmodDetail::BitLength(8))
        );
    }

    #[test]
    fn a_type_with_no_modifier_decodes_to_nothing() {
        assert_eq!(Typmod(14).decode(Builtin::Int4), None);
        assert_eq!(Typmod(14).decode(Builtin::Text), None);
        // An array does not decode. The modifier belongs to the element.
        assert_eq!(Typmod(14).decode(Builtin::VarcharArray), None);
    }
}
