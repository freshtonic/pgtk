//! Configuration parameters: the names that `SHOW` and `SET` accept.

/// A configuration parameter of the server: one `pg_settings` row.
///
/// The model holds the name and the kind of value. It holds no value, because
/// a value is the state of a session and not a fact of the catalog.
///
/// PostgreSQL gives the result column of `SHOW name` the canonical spelling
/// of the parameter, so `SHOW timezone` gives a column with the name
/// `TimeZone`. An unknown name is the error `unrecognized configuration
/// parameter` (SQLSTATE `42704`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Parameter {
    /// The name, in the canonical spelling of the server, such as `TimeZone`.
    pub name: String,
    /// The kind of value that the parameter holds.
    pub kind: ParameterKind,
}

/// The kind of value of a parameter, from `pg_settings.vartype`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParameterKind {
    /// `bool`.
    Bool,
    /// `integer`.
    Integer,
    /// `real`.
    Real,
    /// `string`.
    String,
    /// `enum`.
    Enum,
}

impl ParameterKind {
    /// The kind for a `vartype` value of the server.
    pub fn from_vartype(vartype: &str) -> Option<Self> {
        match vartype {
            "bool" => Some(ParameterKind::Bool),
            "integer" => Some(ParameterKind::Integer),
            "real" => Some(ParameterKind::Real),
            "string" => Some(ParameterKind::String),
            "enum" => Some(ParameterKind::Enum),
            _ => None,
        }
    }
}

/// The key of a parameter name in a lookup.
///
/// The server compares parameter names without regard to the case of an
/// ASCII letter.
pub(crate) fn lookup_key(name: &str) -> String {
    name.to_ascii_lowercase()
}
