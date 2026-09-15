//! The search path: the schemas that resolve an unqualified name.

use crate::catalog::Catalog;
use crate::handle::Handle;
use crate::schema::Schema;

/// An ordered list of schemas that resolves an unqualified name.
///
/// A catalog does not hold a search path. Each lookup takes one as an
/// argument.
///
/// `pg_temp` is not special here. The loader never keeps a temporary schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPath {
    schemas: Vec<Handle<Schema>>,
}

impl SearchPath {
    /// Resolve schema names against a catalog.
    ///
    /// A name that matches no schema is dropped. A name that repeats is kept
    /// once, at its first place. `pg_catalog` goes first, unless the list
    /// names it, in which case it stays where the list puts it.
    pub fn new<'a>(catalog: &Catalog, names: impl IntoIterator<Item = &'a str>) -> Self {
        let names: Vec<&str> = names.into_iter().collect();
        let mut schemas: Vec<Handle<Schema>> = Vec::with_capacity(names.len() + 1);

        if !names.contains(&"pg_catalog")
            && let Some(handle) = catalog.schema_named("pg_catalog")
        {
            schemas.push(handle);
        }

        for name in names {
            if let Some(handle) = catalog.schema_named(name)
                && !schemas.contains(&handle)
            {
                schemas.push(handle);
            }
        }

        SearchPath { schemas }
    }

    /// The search path that the loader recorded.
    ///
    /// The names come from `catalog.metadata().search_path`.
    pub fn from_metadata(catalog: &Catalog) -> Self {
        let names: Vec<&str> = catalog
            .metadata()
            .search_path
            .iter()
            .map(|name| name.as_str())
            .collect();
        SearchPath::new(catalog, names)
    }

    /// The schemas, in the order that a lookup reads them.
    pub fn schemas(&self) -> &[Handle<Schema>] {
        &self.schemas
    }
}
