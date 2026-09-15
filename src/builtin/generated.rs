// PLACEHOLDER. Work item W1-builtins replaces this file with the real
// generated table. `cargo xtask gen-builtins <path/to/pg_type.dat>` writes
// that file from PostgreSQL 17.9's `src/include/catalog/pg_type.dat`. Never
// edit the generated file by hand.
//
// This placeholder holds only the built-in types that `Typmod::decode`, the
// unit tests, and the live loader test need. The real file holds 196
// variants. The shape of the enum here matches section 4 of `docs/spec.md`,
// so code that uses it keeps working after the swap.
//
// This file is included by `src/builtin.rs`. It may name only `Oid`.

/// Every type with a fixed OID that a SQL expression can name.
///
/// The variants are in OID order. The discriminants run from 0 without a
/// gap, so `Builtin::ALL[b as usize] == b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum Builtin {
    /// `bool`, OID 16.
    Bool = 0,
    /// `bytea`, OID 17.
    Bytea,
    /// `char`, OID 18.
    Char,
    /// `name`, OID 19.
    Name,
    /// `int8`, OID 20.
    Int8,
    /// `int2`, OID 21.
    Int2,
    /// `int4`, OID 23.
    Int4,
    /// `text`, OID 25.
    Text,
    /// `oid`, OID 26.
    Oid,
    /// `float4`, OID 700.
    Float4,
    /// `float8`, OID 701.
    Float8,
    /// `unknown`, OID 705.
    Unknown,
    /// `_bool`, OID 1000.
    BoolArray,
    /// `_int2`, OID 1005.
    Int2Array,
    /// `_int4`, OID 1007.
    Int4Array,
    /// `_text`, OID 1009.
    TextArray,
    /// `_varchar`, OID 1015.
    VarcharArray,
    /// `_int8`, OID 1016.
    Int8Array,
    /// `bpchar`, OID 1042.
    Bpchar,
    /// `varchar`, OID 1043.
    Varchar,
    /// `date`, OID 1082.
    Date,
    /// `time`, OID 1083.
    Time,
    /// `timestamp`, OID 1114.
    Timestamp,
    /// `timestamptz`, OID 1184.
    Timestamptz,
    /// `interval`, OID 1186.
    Interval,
    /// `_numeric`, OID 1231.
    NumericArray,
    /// `timetz`, OID 1266.
    Timetz,
    /// `bit`, OID 1560.
    Bit,
    /// `varbit`, OID 1562.
    Varbit,
    /// `numeric`, OID 1700.
    Numeric,
    /// `record`, OID 2249.
    Record,
    /// `cstring`, OID 2275.
    Cstring,
    /// `any`, OID 2276.
    Any,
    /// `anyarray`, OID 2277.
    AnyArray,
    /// `void`, OID 2278.
    Void,
    /// `internal`, OID 2281.
    Internal,
    /// `anyelement`, OID 2283.
    AnyElement,
    /// `_record`, OID 2287.
    RecordArray,
}

impl Builtin {
    /// All variants, in discriminant order.
    pub const ALL: &'static [Builtin] = &[
        Builtin::Bool,
        Builtin::Bytea,
        Builtin::Char,
        Builtin::Name,
        Builtin::Int8,
        Builtin::Int2,
        Builtin::Int4,
        Builtin::Text,
        Builtin::Oid,
        Builtin::Float4,
        Builtin::Float8,
        Builtin::Unknown,
        Builtin::BoolArray,
        Builtin::Int2Array,
        Builtin::Int4Array,
        Builtin::TextArray,
        Builtin::VarcharArray,
        Builtin::Int8Array,
        Builtin::Bpchar,
        Builtin::Varchar,
        Builtin::Date,
        Builtin::Time,
        Builtin::Timestamp,
        Builtin::Timestamptz,
        Builtin::Interval,
        Builtin::NumericArray,
        Builtin::Timetz,
        Builtin::Bit,
        Builtin::Varbit,
        Builtin::Numeric,
        Builtin::Record,
        Builtin::Cstring,
        Builtin::Any,
        Builtin::AnyArray,
        Builtin::Void,
        Builtin::Internal,
        Builtin::AnyElement,
        Builtin::RecordArray,
    ];

    /// The fixed OID of this built-in type.
    pub const fn oid(self) -> Oid {
        match self {
            Builtin::Bool => Oid(16),
            Builtin::Bytea => Oid(17),
            Builtin::Char => Oid(18),
            Builtin::Name => Oid(19),
            Builtin::Int8 => Oid(20),
            Builtin::Int2 => Oid(21),
            Builtin::Int4 => Oid(23),
            Builtin::Text => Oid(25),
            Builtin::Oid => Oid(26),
            Builtin::Float4 => Oid(700),
            Builtin::Float8 => Oid(701),
            Builtin::Unknown => Oid(705),
            Builtin::BoolArray => Oid(1000),
            Builtin::Int2Array => Oid(1005),
            Builtin::Int4Array => Oid(1007),
            Builtin::TextArray => Oid(1009),
            Builtin::VarcharArray => Oid(1015),
            Builtin::Int8Array => Oid(1016),
            Builtin::Bpchar => Oid(1042),
            Builtin::Varchar => Oid(1043),
            Builtin::Date => Oid(1082),
            Builtin::Time => Oid(1083),
            Builtin::Timestamp => Oid(1114),
            Builtin::Timestamptz => Oid(1184),
            Builtin::Interval => Oid(1186),
            Builtin::NumericArray => Oid(1231),
            Builtin::Timetz => Oid(1266),
            Builtin::Bit => Oid(1560),
            Builtin::Varbit => Oid(1562),
            Builtin::Numeric => Oid(1700),
            Builtin::Record => Oid(2249),
            Builtin::Cstring => Oid(2275),
            Builtin::Any => Oid(2276),
            Builtin::AnyArray => Oid(2277),
            Builtin::Void => Oid(2278),
            Builtin::Internal => Oid(2281),
            Builtin::AnyElement => Oid(2283),
            Builtin::RecordArray => Oid(2287),
        }
    }

    /// The `typname` of this built-in type.
    pub const fn name(self) -> &'static str {
        match self {
            Builtin::Bool => "bool",
            Builtin::Bytea => "bytea",
            Builtin::Char => "char",
            Builtin::Name => "name",
            Builtin::Int8 => "int8",
            Builtin::Int2 => "int2",
            Builtin::Int4 => "int4",
            Builtin::Text => "text",
            Builtin::Oid => "oid",
            Builtin::Float4 => "float4",
            Builtin::Float8 => "float8",
            Builtin::Unknown => "unknown",
            Builtin::BoolArray => "_bool",
            Builtin::Int2Array => "_int2",
            Builtin::Int4Array => "_int4",
            Builtin::TextArray => "_text",
            Builtin::VarcharArray => "_varchar",
            Builtin::Int8Array => "_int8",
            Builtin::Bpchar => "bpchar",
            Builtin::Varchar => "varchar",
            Builtin::Date => "date",
            Builtin::Time => "time",
            Builtin::Timestamp => "timestamp",
            Builtin::Timestamptz => "timestamptz",
            Builtin::Interval => "interval",
            Builtin::NumericArray => "_numeric",
            Builtin::Timetz => "timetz",
            Builtin::Bit => "bit",
            Builtin::Varbit => "varbit",
            Builtin::Numeric => "numeric",
            Builtin::Record => "record",
            Builtin::Cstring => "cstring",
            Builtin::Any => "any",
            Builtin::AnyArray => "anyarray",
            Builtin::Void => "void",
            Builtin::Internal => "internal",
            Builtin::AnyElement => "anyelement",
            Builtin::RecordArray => "_record",
        }
    }
}
