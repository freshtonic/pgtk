//! The search path: the schemas that resolve an unqualified name.

use crate::catalog::Catalog;
use crate::handle::Handle;
use crate::schema::Schema;

/// An ordered list of schemas that resolves an unqualified name.
///
/// A catalog does not hold a search path. Each lookup takes one as an
/// argument.
///
/// # The temporary schema
///
/// The loader keeps the temporary schema of its own session, when the session
/// has one. The server names it `pg_temp_N`. A search path accepts that name,
/// and also the alias `pg_temp`, as `SET search_path` does.
///
/// A lookup of a relation or of a type reads the temporary schema. A lookup
/// of a function or of an operator skips it, as PostgreSQL does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPath {
    schemas: Vec<Handle<Schema>>,
}

impl SearchPath {
    /// Resolve schema names against a catalog.
    ///
    /// A name that matches no schema is dropped. A name that repeats is kept
    /// once, at its first place.
    ///
    /// Two schemas are on the path when the list does not name them, in the
    /// order that PostgreSQL uses. The temporary schema goes first, when the
    /// catalog holds one. `pg_catalog` goes next. A schema that the list does
    /// name stays where the list puts it. The list names the temporary schema
    /// with `pg_temp` or with its `pg_temp_N` name.
    pub fn new<'a>(catalog: &Catalog, names: impl IntoIterator<Item = &'a str>) -> Self {
        let temporary = catalog.temporary_schema();
        let resolve = |name: &str| match name {
            "pg_temp" => temporary,
            name => catalog.schema_named(name),
        };
        let named: Vec<Handle<Schema>> = names.into_iter().filter_map(resolve).collect();
        let mut schemas: Vec<Handle<Schema>> = Vec::with_capacity(named.len() + 2);

        let implicit = [temporary, catalog.schema_named("pg_catalog")];
        for handle in implicit.into_iter().flatten() {
            if !named.contains(&handle) {
                schemas.push(handle);
            }
        }

        for handle in named {
            if !schemas.contains(&handle) {
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
