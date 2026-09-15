//! Identifiers as the server stores them.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

/// An identifier exactly as the server stores it.
///
/// A name is already case-folded and unquoted. Equality is byte equality.
/// Folding and quoting belong to the parser, not to this model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Name(String);

impl Name {
    /// Make a name from text that the server already folded.
    pub fn new(s: impl Into<String>) -> Self {
        Name(s.into())
    }

    /// The text of the name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Name(value.to_owned())
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Name(value)
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A schema name and an entity name together.
///
/// A qualified name points at one entity without a search path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QualifiedName {
    /// The name of the schema that holds the entity.
    pub schema: Name,
    /// The name of the entity inside that schema.
    pub name: Name,
}

impl QualifiedName {
    /// Make a qualified name from a schema name and an entity name.
    pub fn new(schema: impl Into<Name>, name: impl Into<Name>) -> Self {
        QualifiedName {
            schema: schema.into(),
            name: name.into(),
        }
    }
}

impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.schema, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::{Name, QualifiedName};
    use std::collections::HashMap;

    #[test]
    fn display_prints_the_raw_text() {
        assert_eq!(Name::new("Mixed Case").to_string(), "Mixed Case");
        assert_eq!(QualifiedName::new("public", "t").to_string(), "public.t");
    }

    #[test]
    fn a_name_borrows_as_a_string_slice() {
        let mut map = HashMap::new();
        map.insert(Name::new("int4"), 23u32);
        assert_eq!(map.get("int4"), Some(&23));
    }

    #[test]
    fn equality_is_byte_equality() {
        assert_ne!(Name::new("INT4"), Name::new("int4"));
        assert_eq!(Name::from("int4").as_str(), "int4");
    }
}
