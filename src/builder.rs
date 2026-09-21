//! The builder: the only way to make a [`Catalog`].

use std::collections::HashSet;

use crate::builtin::Builtin;
use crate::cast::{Cast, CastMethod};
use crate::catalog::{Catalog, Metadata};
use crate::collation::Collation;
use crate::extension::Extension;
use crate::function::Function;
use crate::handle::Handle;
use crate::oid::Oid;
use crate::operator::Operator;
use crate::parameter::{self, Parameter};
use crate::relation::Relation;
use crate::schema::Schema;
use crate::types::{Type, TypeKind};

/// The only way to make a [`Catalog`].
///
/// The loader drives a builder from query rows. A test drives one by hand.
/// Each push gives back the handle that the entity will have, so a caller
/// can close a cycle without a second pass.
#[derive(Debug, Clone)]
pub struct CatalogBuilder {
    metadata: Metadata,
    schemas: Vec<Schema>,
    extensions: Vec<Extension>,
    collations: Vec<Collation>,
    types: Vec<Type>,
    relations: Vec<Relation>,
    functions: Vec<Function>,
    operators: Vec<Operator>,
    casts: Vec<Cast>,
    parameters: Vec<Parameter>,
    permit_missing_builtins: bool,
}

/// Why a build failed.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// An entity holds a handle that no arena slot answers.
    #[error("{entity} handle {index} is out of range (arena has {len})")]
    DanglingHandle {
        /// The kind of arena that the handle points into.
        entity: &'static str,
        /// The index that the handle holds.
        index: u32,
        /// The number of entities in the arena.
        len: u32,
    },
    /// Two entities of one kind have the same OID.
    #[error("duplicate OID {oid} in {entity} arena")]
    DuplicateOid {
        /// The kind of arena.
        entity: &'static str,
        /// The OID that repeats.
        oid: Oid,
    },
    /// The catalog does not hold a built-in type.
    #[error("built-in type {builtin:?} (oid {oid}) is missing")]
    MissingBuiltin {
        /// The built-in type that is absent.
        builtin: Builtin,
        /// The fixed OID of that type.
        oid: Oid,
    },
    /// A primary key names an attribute position that no column of the
    /// relation has.
    #[error("primary key of relation {relation} names position {position}, which is not a column")]
    UnknownKeyColumn {
        /// The OID of the relation.
        relation: Oid,
        /// The attribute position that the key names.
        position: i16,
    },
    /// Two parameters have the same name. The comparison does not see the
    /// case of an ASCII letter.
    #[error("duplicate parameter {name}")]
    DuplicateParameter {
        /// The name that repeats, as the second parameter spells it.
        name: String,
    },
    /// A function does not have one default type for each default.
    #[error("function {function} has {default_count} defaults and {default_types} default types")]
    DefaultTypesLength {
        /// The OID of the function.
        function: Oid,
        /// The `default_count` of the function.
        default_count: u16,
        /// The length of the `default_types` of the function.
        default_types: usize,
    },
}

/// Check that one handle points inside an arena.
fn check<T>(handle: Handle<T>, len: usize, entity: &'static str) -> Result<(), BuildError> {
    if (handle.index() as usize) < len {
        Ok(())
    } else {
        Err(BuildError::DanglingHandle {
            entity,
            index: handle.index(),
            len: len as u32,
        })
    }
}

/// Check one optional handle.
fn check_opt<T>(
    handle: Option<Handle<T>>,
    len: usize,
    entity: &'static str,
) -> Result<(), BuildError> {
    match handle {
        Some(handle) => check(handle, len, entity),
        None => Ok(()),
    }
}

/// Check that the OIDs of one arena are unique.
fn check_oids(oids: impl Iterator<Item = Oid>, entity: &'static str) -> Result<(), BuildError> {
    let mut seen = HashSet::new();
    for oid in oids {
        if !seen.insert(oid) {
            return Err(BuildError::DuplicateOid { entity, oid });
        }
    }
    Ok(())
}

impl CatalogBuilder {
    /// Start an empty catalog.
    pub fn new(metadata: Metadata) -> Self {
        CatalogBuilder {
            metadata,
            schemas: Vec::new(),
            extensions: Vec::new(),
            collations: Vec::new(),
            types: Vec::new(),
            relations: Vec::new(),
            functions: Vec::new(),
            operators: Vec::new(),
            casts: Vec::new(),
            parameters: Vec::new(),
            permit_missing_builtins: false,
        }
    }

    /// Start from a catalog that exists, and add to it.
    ///
    /// A test usually starts from the fixture. The metadata of the catalog
    /// stays as it is.
    pub fn from_catalog(catalog: Catalog) -> Self {
        let (
            metadata,
            schemas,
            extensions,
            collations,
            types,
            relations,
            functions,
            operators,
            casts,
            parameters,
        ) = catalog.into_parts();
        CatalogBuilder {
            metadata,
            schemas,
            extensions,
            collations,
            types,
            relations,
            functions,
            operators,
            casts,
            parameters,
            permit_missing_builtins: false,
        }
    }

    /// Let [`finish`](CatalogBuilder::finish) pass with built-in types
    /// absent.
    ///
    /// Tests use this. A catalog that a server produced always holds every
    /// built-in type.
    pub fn permit_missing_builtins(&mut self) -> &mut Self {
        self.permit_missing_builtins = true;
        self
    }

    /// Add a schema and give back its handle.
    pub fn push_schema(&mut self, s: Schema) -> Handle<Schema> {
        let handle = Handle::new(self.schemas.len() as u32);
        self.schemas.push(s);
        handle
    }

    /// Add an extension and give back its handle.
    pub fn push_extension(&mut self, e: Extension) -> Handle<Extension> {
        let handle = Handle::new(self.extensions.len() as u32);
        self.extensions.push(e);
        handle
    }

    /// Add a type and give back its handle.
    pub fn push_type(&mut self, t: Type) -> Handle<Type> {
        let handle = Handle::new(self.types.len() as u32);
        self.types.push(t);
        handle
    }

    /// Add a relation and give back its handle.
    pub fn push_relation(&mut self, r: Relation) -> Handle<Relation> {
        let handle = Handle::new(self.relations.len() as u32);
        self.relations.push(r);
        handle
    }

    /// Add a function and give back its handle.
    pub fn push_function(&mut self, f: Function) -> Handle<Function> {
        let handle = Handle::new(self.functions.len() as u32);
        self.functions.push(f);
        handle
    }

    /// Add an operator and give back its handle.
    pub fn push_operator(&mut self, o: Operator) -> Handle<Operator> {
        let handle = Handle::new(self.operators.len() as u32);
        self.operators.push(o);
        handle
    }

    /// Add a collation and give back its handle.
    pub fn push_collation(&mut self, c: Collation) -> Handle<Collation> {
        let handle = Handle::new(self.collations.len() as u32);
        self.collations.push(c);
        handle
    }

    /// Add a configuration parameter.
    ///
    /// A parameter has no handle, because no entity points at one. The
    /// catalog sorts the parameters by name.
    pub fn push_parameter(&mut self, p: Parameter) {
        self.parameters.push(p);
    }

    /// Add a cast and give back its handle.
    pub fn push_cast(&mut self, c: Cast) -> Handle<Cast> {
        let handle = Handle::new(self.casts.len() as u32);
        self.casts.push(c);
        handle
    }

    /// The handle that the next [`push_type`](CatalogBuilder::push_type)
    /// gives back.
    ///
    /// Use this to write a forward reference, such as the array of a type
    /// that the builder does not hold yet.
    pub fn next_type_handle(&self) -> Handle<Type> {
        Handle::new(self.types.len() as u32)
    }

    /// The handle that the next
    /// [`push_relation`](CatalogBuilder::push_relation) gives back.
    pub fn next_relation_handle(&self) -> Handle<Relation> {
        Handle::new(self.relations.len() as u32)
    }

    /// The handle that the next
    /// [`push_operator`](CatalogBuilder::push_operator) gives back.
    pub fn next_operator_handle(&self) -> Handle<Operator> {
        Handle::new(self.operators.len() as u32)
    }

    /// Change a type that the builder holds, to close a cycle.
    ///
    /// # Panics
    ///
    /// This panics when the handle points outside the arena.
    pub fn type_mut(&mut self, h: Handle<Type>) -> &mut Type {
        &mut self.types[h.index() as usize]
    }

    /// Change a relation that the builder holds, to close a cycle.
    ///
    /// # Panics
    ///
    /// This panics when the handle points outside the arena.
    pub fn relation_mut(&mut self, h: Handle<Relation>) -> &mut Relation {
        &mut self.relations[h.index() as usize]
    }

    /// Change an operator that the builder holds, to close a cycle.
    ///
    /// # Panics
    ///
    /// This panics when the handle points outside the arena.
    pub fn operator_mut(&mut self, h: Handle<Operator>) -> &mut Operator {
        &mut self.operators[h.index() as usize]
    }

    /// Check every handle of every entity.
    fn check_handles(&self) -> Result<(), BuildError> {
        let schemas = self.schemas.len();
        let extensions = self.extensions.len();
        let collations = self.collations.len();
        let types = self.types.len();
        let relations = self.relations.len();
        let functions = self.functions.len();
        let operators = self.operators.len();

        for entity in &self.schemas {
            check_opt(entity.extension, extensions, "extension")?;
        }
        for entity in &self.extensions {
            check(entity.schema, schemas, "schema")?;
        }
        for entity in &self.collations {
            check(entity.schema, schemas, "schema")?;
            check_opt(entity.extension, extensions, "extension")?;
        }
        for entity in &self.types {
            check(entity.schema, schemas, "schema")?;
            check_opt(entity.collation, collations, "collation")?;
            check_opt(entity.array, types, "type")?;
            check_opt(entity.extension, extensions, "extension")?;
            if let Some(btree) = entity.btree {
                for operator in [btree.lt, btree.le, btree.eq, btree.ge, btree.gt] {
                    check(operator, operators, "operator")?;
                }
            }
            check_opt(entity.hash_equality, operators, "operator")?;
            match &entity.kind {
                TypeKind::Base | TypeKind::Enum { .. } | TypeKind::Pseudo(_) => {}
                TypeKind::Array { element } => check(*element, types, "type")?,
                TypeKind::Composite(detail) => {
                    for field in &detail.fields {
                        check(field.ty, types, "type")?;
                    }
                    check_opt(detail.relation, relations, "relation")?;
                }
                TypeKind::Domain(detail) => check(detail.base, types, "type")?,
                TypeKind::Range(detail) => {
                    check(detail.subtype, types, "type")?;
                    check(detail.multirange, types, "type")?;
                }
                TypeKind::Multirange { range } => check(*range, types, "type")?,
            }
        }
        for entity in &self.relations {
            check(entity.schema, schemas, "schema")?;
            check_opt(entity.row_type, types, "type")?;
            check_opt(entity.extension, extensions, "extension")?;
            for column in &entity.columns {
                check(column.ty, types, "type")?;
                check_opt(column.collation, collations, "collation")?;
            }
            check_opt(entity.of_type, types, "type")?;
            for parent in &entity.parents {
                check(*parent, relations, "relation")?;
            }
            if let Some(key) = &entity.primary_key {
                for position in &key.columns {
                    if entity.column_at(*position).is_none() {
                        return Err(BuildError::UnknownKeyColumn {
                            relation: entity.oid,
                            position: *position,
                        });
                    }
                }
            }
        }
        for entity in &self.functions {
            check(entity.schema, schemas, "schema")?;
            if entity.default_types.len() != usize::from(entity.default_count) {
                return Err(BuildError::DefaultTypesLength {
                    function: entity.oid,
                    default_count: entity.default_count,
                    default_types: entity.default_types.len(),
                });
            }
            for ty in entity.default_types.iter().flatten() {
                check(*ty, types, "type")?;
            }
            check(entity.returns, types, "type")?;
            check_opt(entity.variadic, types, "type")?;
            check_opt(entity.extension, extensions, "extension")?;
            for argument in &entity.arguments {
                check(argument.ty, types, "type")?;
            }
        }
        for entity in &self.operators {
            check(entity.schema, schemas, "schema")?;
            check_opt(entity.left, types, "type")?;
            check(entity.right, types, "type")?;
            check(entity.result, types, "type")?;
            check(entity.function, functions, "function")?;
            check_opt(entity.commutator, operators, "operator")?;
            check_opt(entity.negator, operators, "operator")?;
            check_opt(entity.extension, extensions, "extension")?;
        }
        for entity in &self.casts {
            check(entity.source, types, "type")?;
            check(entity.target, types, "type")?;
            check_opt(entity.extension, extensions, "extension")?;
            if let CastMethod::Function(function) = entity.method {
                check(function, functions, "function")?;
            }
        }
        Ok(())
    }

    /// Check that the OIDs of each arena are unique.
    fn check_all_oids(&self) -> Result<(), BuildError> {
        check_oids(self.schemas.iter().map(|e| e.oid), "schema")?;
        check_oids(self.extensions.iter().map(|e| e.oid), "extension")?;
        check_oids(self.collations.iter().map(|e| e.oid), "collation")?;
        check_oids(self.types.iter().map(|e| e.oid), "type")?;
        check_oids(self.relations.iter().map(|e| e.oid), "relation")?;
        check_oids(self.functions.iter().map(|e| e.oid), "function")?;
        check_oids(self.operators.iter().map(|e| e.oid), "operator")?;
        check_oids(self.casts.iter().map(|e| e.oid), "cast")?;
        let mut names = HashSet::new();
        for entity in &self.parameters {
            if !names.insert(parameter::lookup_key(&entity.name)) {
                return Err(BuildError::DuplicateParameter {
                    name: entity.name.clone(),
                });
            }
        }
        Ok(())
    }

    /// Check that the catalog holds every built-in type.
    fn check_builtins(&self) -> Result<(), BuildError> {
        if self.permit_missing_builtins {
            return Ok(());
        }
        let oids: HashSet<Oid> = self.types.iter().map(|t| t.oid).collect();
        for builtin in Builtin::ALL {
            if !oids.contains(&builtin.oid()) {
                return Err(BuildError::MissingBuiltin {
                    builtin: *builtin,
                    oid: builtin.oid(),
                });
            }
        }
        Ok(())
    }

    /// Validate the entities, then build the catalog and its indexes.
    ///
    /// The check runs in three steps. First, every handle must point inside
    /// its arena, and every column of a primary key must be a column of its
    /// relation. Second, the OIDs of each arena must be unique, and so must
    /// the names of the parameters. Third, the
    /// catalog must hold every built-in type, unless
    /// [`permit_missing_builtins`](CatalogBuilder::permit_missing_builtins)
    /// says otherwise.
    ///
    /// The metadata takes the current [`Catalog::FORMAT_VERSION`].
    pub fn finish(mut self) -> Result<Catalog, BuildError> {
        self.check_handles()?;
        self.check_all_oids()?;
        self.check_builtins()?;
        self.metadata.format_version = Catalog::FORMAT_VERSION;
        Ok(Catalog::build(
            self.metadata,
            self.schemas,
            self.extensions,
            self.collations,
            self.types,
            self.relations,
            self.functions,
            self.operators,
            self.casts,
            self.parameters,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildError, CatalogBuilder};
    use crate::builtin::Builtin;
    use crate::catalog::Metadata;
    use crate::handle::Handle;
    use crate::name::Name;
    use crate::oid::Oid;
    use crate::schema::Schema;
    use crate::types::{Type, TypeCategory, TypeKind};

    fn schema(oid: u32, name: &str) -> Schema {
        Schema {
            oid: Oid(oid),
            name: Name::new(name),
            is_temporary: false,
            extension: None,
        }
    }

    fn base_type(oid: u32, name: &str, schema: Handle<Schema>) -> Type {
        Type {
            oid: Oid(oid),
            name: Name::new(name),
            schema,
            category: TypeCategory::Numeric,
            is_preferred: false,
            is_collatable: false,
            collation: None,
            array: None,
            kind: TypeKind::Base,
            btree: None,
            hash_equality: None,
            extension: None,
        }
    }

    #[test]
    fn an_empty_catalog_needs_permission() {
        let error = CatalogBuilder::new(Metadata::placeholder())
            .finish()
            .unwrap_err();
        assert!(matches!(error, BuildError::MissingBuiltin { .. }));
    }

    #[test]
    fn permission_lets_an_empty_catalog_build() {
        let mut builder = CatalogBuilder::new(Metadata::placeholder());
        builder.permit_missing_builtins();
        let catalog = builder.finish().unwrap();
        assert_eq!(catalog.types().len(), 0);
        assert_eq!(catalog.try_builtin(Builtin::Int4), None);
    }

    #[test]
    fn a_dangling_handle_fails_the_build() {
        let mut builder = CatalogBuilder::new(Metadata::placeholder());
        builder.permit_missing_builtins();
        builder.push_type(base_type(23, "int4", Handle::new(0)));
        let error = builder.finish().unwrap_err();
        match error {
            BuildError::DanglingHandle { entity, index, len } => {
                assert_eq!(entity, "schema");
                assert_eq!(index, 0);
                assert_eq!(len, 0);
            }
            other => panic!("expected a dangling handle, got {other:?}"),
        }
    }

    #[test]
    fn a_dangling_operator_class_handle_fails_the_build() {
        let mut builder = CatalogBuilder::new(Metadata::placeholder());
        builder.permit_missing_builtins();
        let pg_catalog = builder.push_schema(schema(11, "pg_catalog"));
        let int4 = builder.push_type(base_type(23, "int4", pg_catalog));
        builder.type_mut(int4).hash_equality = Some(Handle::new(7));
        let error = builder.finish().unwrap_err();
        match error {
            BuildError::DanglingHandle { entity, index, len } => {
                assert_eq!(entity, "operator");
                assert_eq!(index, 7);
                assert_eq!(len, 0);
            }
            other => panic!("expected a dangling handle, got {other:?}"),
        }
    }

    #[test]
    fn a_repeated_oid_fails_the_build() {
        let mut builder = CatalogBuilder::new(Metadata::placeholder());
        builder.permit_missing_builtins();
        let pg_catalog = builder.push_schema(schema(11, "pg_catalog"));
        builder.push_type(base_type(23, "int4", pg_catalog));
        builder.push_type(base_type(23, "other", pg_catalog));
        let error = builder.finish().unwrap_err();
        match error {
            BuildError::DuplicateOid { entity, oid } => {
                assert_eq!(entity, "type");
                assert_eq!(oid, Oid(23));
            }
            other => panic!("expected a duplicate OID, got {other:?}"),
        }
    }

    #[test]
    fn finish_stamps_the_format_version() {
        let mut metadata = Metadata::placeholder();
        metadata.format_version = 99;
        let mut builder = CatalogBuilder::new(metadata);
        builder.permit_missing_builtins();
        let catalog = builder.finish().unwrap();
        assert_eq!(
            catalog.metadata().format_version,
            crate::catalog::Catalog::FORMAT_VERSION
        );
    }

    #[test]
    fn from_catalog_reopens_the_arenas() {
        let mut builder = CatalogBuilder::new(Metadata::placeholder());
        builder.permit_missing_builtins();
        let pg_catalog = builder.push_schema(schema(11, "pg_catalog"));
        builder.push_type(base_type(23, "int4", pg_catalog));
        let catalog = builder.finish().unwrap();

        let mut again = CatalogBuilder::from_catalog(catalog);
        again.permit_missing_builtins();
        assert_eq!(again.next_type_handle(), Handle::new(1));
        again.push_type(base_type(25, "text", pg_catalog));
        let catalog = again.finish().unwrap();
        assert_eq!(catalog.types().len(), 2);
        assert_eq!(catalog.type_by_oid(Oid(25)), Some(Handle::new(1)));
    }
}
