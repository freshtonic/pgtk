//! The server's own object identifier.

use std::fmt;

/// A PostgreSQL object identifier.
///
/// An OID is the server's identity for an entity. The model keeps it for
/// display and for correlation with server output. The model never uses an
/// OID as its own identity; a [`crate::Handle`] does that job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Oid(
    /// The raw number.
    pub u32,
);

impl fmt::Display for Oid {
    /// Print the number.
    ///
    /// This passes the format string on to the number, so a width and an
    /// alignment apply to it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<u32> for Oid {
    fn from(value: u32) -> Self {
        Oid(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Oid;

    #[test]
    fn display_prints_the_number() {
        assert_eq!(Oid(23).to_string(), "23");
    }

    #[test]
    fn display_obeys_a_width() {
        assert_eq!(format!("[{:>5}]", Oid(23)), "[   23]");
    }

    #[test]
    fn oids_order_by_number() {
        assert!(Oid(16) < Oid(23));
    }
}
