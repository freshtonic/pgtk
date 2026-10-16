//! Collations.

use crate::extension::Extension;
use crate::handle::Handle;
use crate::name::Name;
use crate::oid::Oid;
use crate::schema::Schema;

/// A `pg_collation` row.
///
/// Parse analysis compares the identity of two collations and finds a
/// collation by its name. It does not sort text, so the model holds no locale
/// data: no `collcollate`, no `collctype`, and no ICU rules.
///
/// Two collations in one schema can have one name, when each is for a
/// different encoding. [`Catalog::collation_named`](crate::Catalog::collation_named)
/// selects the one that the database can use.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Collation {
    /// The server's identifier for this collation.
    pub oid: Oid,
    /// The name of the collation.
    pub name: Name,
    /// The schema that holds this collation.
    pub schema: Handle<Schema>,
    /// The encoding that the collation is for, as a PostgreSQL encoding
    /// number. It is `None` for a collation that each encoding can use:
    /// `collencoding` is -1.
    pub encoding: Option<i32>,
    /// The provider of the collation.
    pub provider: CollationProvider,
    /// True for a deterministic collation: one in which only byte-equal
    /// strings are equal.
    pub deterministic: bool,
    /// The extension that owns this collation, when one does.
    pub extension: Option<Handle<Extension>>,
}

/// The provider of a collation, from `collprovider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CollationProvider {
    /// The `default` collation, which stands for the collation of the
    /// database. The letter is `d`.
    Default,
    /// The C library. The letter is `c`.
    Libc,
    /// ICU. The letter is `i`.
    Icu,
    /// The provider that is built in. PostgreSQL 17 added it. The letter is
    /// `b`.
    Builtin,
}

/// The encoding of a database.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Encoding {
    /// The PostgreSQL encoding number, from `pg_database.encoding`. `UTF8`
    /// is 6.
    pub id: i32,
    /// The name of the encoding, such as `UTF8`. PostgreSQL gives this name
    /// in the message `collation "x" for encoding "UTF8" does not exist`.
    pub name: String,
}

impl Encoding {
    /// `UTF8`, which is the encoding of most databases.
    pub fn utf8() -> Self {
        Encoding {
            id: 6,
            name: "UTF8".to_owned(),
        }
    }

    /// True when ICU can work with this encoding.
    ///
    /// ICU cannot work with `SQL_ASCII`, `EUC_JIS_2004`, `MULE_INTERNAL`, and
    /// `WIN874`. In a database with one of those encodings, PostgreSQL does
    /// not find an ICU collation by name.
    pub fn is_supported_by_icu(&self) -> bool {
        !matches!(self.id, 0 | 5 | 7 | 21)
    }

    /// True when a database with this encoding can use a collation.
    ///
    /// This is the rule of `lookup_collation` in PostgreSQL. A collation for
    /// one encoding needs that encoding. A collation for each encoding is
    /// usable, but an ICU collation also needs an encoding that ICU can work
    /// with.
    pub fn can_use(&self, collation: &Collation) -> bool {
        match collation.encoding {
            Some(id) => id == self.id,
            None => collation.provider != CollationProvider::Icu || self.is_supported_by_icu(),
        }
    }
}
