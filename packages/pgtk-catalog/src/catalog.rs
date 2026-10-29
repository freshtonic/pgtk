//! The catalog itself: the arenas, the metadata, and every lookup.

use std::collections::HashMap;
use std::fmt;
use std::ops::Index;

use crate::builtin::Builtin;
use crate::cast::Cast;
use crate::collation::{Collation, Encoding};
use crate::extension::Extension;
use crate::function::Function;
use crate::handle::Handle;
use crate::name::{Name, QualifiedName};
use crate::oid::Oid;
use crate::operator::Operator;
use crate::operator_class::OperatorClass;
use crate::parameter::{self, Parameter};
use crate::relation::Relation;
use crate::schema::Schema;
use crate::search_path::SearchPath;
use crate::types::{BtreeOperators, Type};

/// The version of the server that a catalog came from.
///
/// The loader reads `server_version_num`. The major number is that value
/// divided by 10000. The minor number is the remainder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ServerVersion {
    /// The major version, such as 17.
    pub major: u16,
    /// The minor version, such as 2.
    pub minor: u16,
}

impl fmt::Display for ServerVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// What the loader recorded about the database and the moment of the load.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Metadata {
    /// The version of the server.
    pub server_version: ServerVersion,
    /// The name of the database.
    pub database: Name,
    /// The encoding of the database.
    ///
    /// A lookup of a collation by name reads it: see
    /// [`Catalog::collation_named`].
    #[cfg_attr(feature = "serde", serde(default = "Encoding::utf8"))]
    pub encoding: Encoding,
    /// The result of `current_schemas(true)` at the moment of the load.
    pub search_path: Vec<Name>,
    /// The moment of the load, in seconds since the Unix epoch, by the
    /// server's clock. The loader reads no clock of its own, so that it
    /// runs on targets that have none, such as `wasm32-unknown-unknown`.
    pub loaded_at_unix_secs: u64,
    /// The format version of the catalog.
    ///
    /// [`CatalogBuilder::finish`](crate::CatalogBuilder::finish) sets this
    /// field to [`Catalog::FORMAT_VERSION`].
    pub format_version: u32,
}

impl Metadata {
    /// Metadata for a catalog that no server produced.
    ///
    /// Tests use this. The server version is `0.0`, the database is
    /// `placeholder`, the encoding is `UTF8`, the search path is empty, and
    /// the load time is 0.
    pub fn placeholder() -> Self {
        Metadata {
            server_version: ServerVersion { major: 0, minor: 0 },
            database: Name::new("placeholder"),
            encoding: Encoding::utf8(),
            search_path: Vec::new(),
            loaded_at_unix_secs: 0,
            format_version: Catalog::FORMAT_VERSION,
        }
    }
}

/// An immutable picture of one PostgreSQL database's system catalog.
///
/// A catalog holds every entity in an arena, one arena per kind. A
/// [`Handle`] points into an arena. Arenas are in OID order, so iteration is
/// in OID order too.
///
/// [`CatalogBuilder`](crate::CatalogBuilder) is the only way to make a
/// catalog. A catalog never changes after that. A refresh makes a new
/// catalog.
#[derive(Debug, Clone)]
pub struct Catalog {
    metadata: Metadata,
    schemas: Vec<Schema>,
    extensions: Vec<Extension>,
    collations: Vec<Collation>,
    types: Vec<Type>,
    relations: Vec<Relation>,
    functions: Vec<Function>,
    operators: Vec<Operator>,
    casts: Vec<Cast>,
    operator_classes: Vec<OperatorClass>,
    parameters: Vec<Parameter>,
    builtins: Vec<Option<Handle<Type>>>,
    schema_by_oid: HashMap<Oid, Handle<Schema>>,
    extension_by_oid: HashMap<Oid, Handle<Extension>>,
    collation_by_oid: HashMap<Oid, Handle<Collation>>,
    type_by_oid: HashMap<Oid, Handle<Type>>,
    relation_by_oid: HashMap<Oid, Handle<Relation>>,
    function_by_oid: HashMap<Oid, Handle<Function>>,
    operator_by_oid: HashMap<Oid, Handle<Operator>>,
    cast_by_oid: HashMap<Oid, Handle<Cast>>,
    schema_by_name: HashMap<Name, Handle<Schema>>,
    extension_by_name: HashMap<Name, Handle<Extension>>,
    type_by_name: HashMap<(Handle<Schema>, Name), Handle<Type>>,
    relation_by_name: HashMap<(Handle<Schema>, Name), Handle<Relation>>,
    functions_by_name: HashMap<(Handle<Schema>, Name), Vec<Handle<Function>>>,
    operators_by_name: HashMap<(Handle<Schema>, Name), Vec<Handle<Operator>>>,
    cast_by_pair: HashMap<(Handle<Type>, Handle<Type>), Handle<Cast>>,
    casts_by_source: HashMap<Handle<Type>, Vec<Handle<Cast>>>,
    parameter_by_name: HashMap<String, usize>,
    collations_by_name: HashMap<(Handle<Schema>, Name), Vec<Handle<Collation>>>,
    operator_class_by_oid: HashMap<Oid, Handle<OperatorClass>>,
    operator_class_by_name: HashMap<(Handle<Schema>, Name, Name), Handle<OperatorClass>>,
}

/// Make handles for an arena, then pair each handle with its entity.
fn with_handles<T>(arena: &[T]) -> impl ExactSizeIterator<Item = (Handle<T>, &T)> {
    arena
        .iter()
        .enumerate()
        .map(|(index, entity)| (Handle::new(index as u32), entity))
}

impl Catalog {
    /// The format version of a serialized catalog.
    ///
    /// A fixture with another version is not readable.
    pub const FORMAT_VERSION: u32 = 11;

    /// Make a catalog and build its indexes.
    ///
    /// The builder calls this after it validates the arenas.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build(
        metadata: Metadata,
        schemas: Vec<Schema>,
        extensions: Vec<Extension>,
        collations: Vec<Collation>,
        types: Vec<Type>,
        relations: Vec<Relation>,
        functions: Vec<Function>,
        operators: Vec<Operator>,
        casts: Vec<Cast>,
        operator_classes: Vec<OperatorClass>,
        mut parameters: Vec<Parameter>,
    ) -> Self {
        // One name in one schema can have a collation for each encoding.
        let mut collations_by_name: HashMap<(Handle<Schema>, Name), Vec<Handle<Collation>>> =
            HashMap::new();
        for (handle, collation) in with_handles(&collations) {
            collations_by_name
                .entry((collation.schema, collation.name.clone()))
                .or_default()
                .push(handle);
        }
        for group in collations_by_name.values_mut() {
            group.sort_by_key(|h| collations[h.index() as usize].oid);
        }

        // A parameter has no OID. The order is that of the lookup key, so
        // that two loads of one server give one order.
        parameters.sort_by_key(|entity| parameter::lookup_key(&entity.name));
        let parameter_by_name = parameters
            .iter()
            .enumerate()
            .map(|(index, entity)| (parameter::lookup_key(&entity.name), index))
            .collect();

        let type_by_oid: HashMap<Oid, Handle<Type>> = with_handles(&types)
            .map(|(handle, entity)| (entity.oid, handle))
            .collect();

        let builtins = Builtin::ALL
            .iter()
            .map(|builtin| type_by_oid.get(&builtin.oid()).copied())
            .collect();

        let mut functions_by_name: HashMap<(Handle<Schema>, Name), Vec<Handle<Function>>> =
            HashMap::new();
        for (handle, function) in with_handles(&functions) {
            functions_by_name
                .entry((function.schema, function.name.clone()))
                .or_default()
                .push(handle);
        }
        for overloads in functions_by_name.values_mut() {
            overloads.sort_by_key(|h| functions[h.index() as usize].oid);
        }

        let mut operators_by_name: HashMap<(Handle<Schema>, Name), Vec<Handle<Operator>>> =
            HashMap::new();
        for (handle, operator) in with_handles(&operators) {
            operators_by_name
                .entry((operator.schema, operator.name.clone()))
                .or_default()
                .push(handle);
        }
        for overloads in operators_by_name.values_mut() {
            overloads.sort_by_key(|h| operators[h.index() as usize].oid);
        }

        let mut cast_by_pair = HashMap::new();
        let mut casts_by_source: HashMap<Handle<Type>, Vec<Handle<Cast>>> = HashMap::new();
        for (handle, cast) in with_handles(&casts) {
            cast_by_pair.insert((cast.source, cast.target), handle);
            casts_by_source.entry(cast.source).or_default().push(handle);
        }
        for group in casts_by_source.values_mut() {
            group.sort_by_key(|h| casts[h.index() as usize].oid);
        }

        Catalog {
            schema_by_oid: with_handles(&schemas)
                .map(|(handle, entity)| (entity.oid, handle))
                .collect(),
            extension_by_oid: with_handles(&extensions)
                .map(|(handle, entity)| (entity.oid, handle))
                .collect(),
            collation_by_oid: with_handles(&collations)
                .map(|(handle, entity)| (entity.oid, handle))
                .collect(),
            relation_by_oid: with_handles(&relations)
                .map(|(handle, entity)| (entity.oid, handle))
                .collect(),
            function_by_oid: with_handles(&functions)
                .map(|(handle, entity)| (entity.oid, handle))
                .collect(),
            operator_by_oid: with_handles(&operators)
                .map(|(handle, entity)| (entity.oid, handle))
                .collect(),
            cast_by_oid: with_handles(&casts)
                .map(|(handle, entity)| (entity.oid, handle))
                .collect(),
            schema_by_name: with_handles(&schemas)
                .map(|(handle, entity)| (entity.name.clone(), handle))
                .collect(),
            extension_by_name: with_handles(&extensions)
                .map(|(handle, entity)| (entity.name.clone(), handle))
                .collect(),
            type_by_name: with_handles(&types)
                .map(|(handle, entity)| ((entity.schema, entity.name.clone()), handle))
                .collect(),
            relation_by_name: with_handles(&relations)
                .map(|(handle, entity)| ((entity.schema, entity.name.clone()), handle))
                .collect(),
            type_by_oid,
            functions_by_name,
            operators_by_name,
            cast_by_pair,
            casts_by_source,
            parameter_by_name,
            collations_by_name,
            operator_class_by_oid: with_handles(&operator_classes)
                .map(|(handle, entity)| (entity.oid, handle))
                .collect(),
            operator_class_by_name: with_handles(&operator_classes)
                .map(|(handle, entity)| {
                    let key = (entity.schema, entity.name.clone(), entity.method.clone());
                    (key, handle)
                })
                .collect(),
            operator_classes,
            collations,
            parameters,
            builtins,
            metadata,
            schemas,
            extensions,
            types,
            relations,
            functions,
            operators,
            casts,
        }
    }

    /// Give back the arenas and the metadata, so that a builder can add to
    /// them.
    #[allow(clippy::type_complexity)]
    pub(crate) fn into_parts(
        self,
    ) -> (
        Metadata,
        Vec<Schema>,
        Vec<Extension>,
        Vec<Collation>,
        Vec<Type>,
        Vec<Relation>,
        Vec<Function>,
        Vec<Operator>,
        Vec<Cast>,
        Vec<OperatorClass>,
        Vec<Parameter>,
    ) {
        (
            self.metadata,
            self.schemas,
            self.extensions,
            self.collations,
            self.types,
            self.relations,
            self.functions,
            self.operators,
            self.casts,
            self.operator_classes,
            self.parameters,
        )
    }

    /// What the loader recorded about this catalog.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// The type of a built-in.
    ///
    /// # Panics
    ///
    /// This panics when the catalog does not hold the type. A catalog holds
    /// each built-in of its server version. It does not hold a built-in that
    /// is newer than its server, that is, when
    /// [`Builtin::since`] is more than
    /// [`metadata().server_version.major`](Metadata::server_version). Use
    /// [`try_builtin`](Catalog::try_builtin) for such a built-in. A catalog
    /// that a test built with
    /// [`permit_missing_builtins`](crate::CatalogBuilder::permit_missing_builtins)
    /// can lack any built-in.
    pub fn builtin(&self, b: Builtin) -> Handle<Type> {
        match self.try_builtin(b) {
            Some(handle) => handle,
            None => panic!(
                "built-in type {:?} (oid {}) is not in this catalog",
                b,
                b.oid()
            ),
        }
    }

    /// The type of a built-in, when the catalog holds it.
    pub fn try_builtin(&self, b: Builtin) -> Option<Handle<Type>> {
        self.builtins.get(b as usize).copied().flatten()
    }

    /// The built-in that this type is, when it is one.
    ///
    /// The test is the OID of the type.
    pub fn as_builtin(&self, h: Handle<Type>) -> Option<Builtin> {
        Builtin::from_oid(self[h].oid)
    }

    /// Every schema, in OID order.
    pub fn schemas(&self) -> impl ExactSizeIterator<Item = (Handle<Schema>, &Schema)> {
        with_handles(&self.schemas)
    }

    /// Every extension, in OID order.
    pub fn extensions(&self) -> impl ExactSizeIterator<Item = (Handle<Extension>, &Extension)> {
        with_handles(&self.extensions)
    }

    /// Every collation, in OID order.
    pub fn collations(&self) -> impl ExactSizeIterator<Item = (Handle<Collation>, &Collation)> {
        with_handles(&self.collations)
    }

    /// Every type, in OID order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = (Handle<Type>, &Type)> {
        with_handles(&self.types)
    }

    /// Every relation, in OID order.
    pub fn relations(&self) -> impl ExactSizeIterator<Item = (Handle<Relation>, &Relation)> {
        with_handles(&self.relations)
    }

    /// Every function, in OID order.
    pub fn functions(&self) -> impl ExactSizeIterator<Item = (Handle<Function>, &Function)> {
        with_handles(&self.functions)
    }

    /// Every operator, in OID order.
    pub fn operators(&self) -> impl ExactSizeIterator<Item = (Handle<Operator>, &Operator)> {
        with_handles(&self.operators)
    }

    /// Every cast, in OID order.
    pub fn casts(&self) -> impl ExactSizeIterator<Item = (Handle<Cast>, &Cast)> {
        with_handles(&self.casts)
    }

    /// Every operator class, in OID order.
    pub fn operator_classes(
        &self,
    ) -> impl ExactSizeIterator<Item = (Handle<OperatorClass>, &OperatorClass)> {
        with_handles(&self.operator_classes)
    }

    /// The operator class with this OID.
    pub fn operator_class_by_oid(&self, oid: Oid) -> Option<Handle<OperatorClass>> {
        self.operator_class_by_oid.get(&oid).copied()
    }

    /// The operator class with this qualified name for this access method.
    ///
    /// The name of the access method is in lower case, such as `btree`.
    /// PostgreSQL gives the error `operator class "x" does not exist for
    /// access method "btree"` when it finds nothing.
    pub fn operator_class_named(
        &self,
        name: &QualifiedName,
        method: &str,
    ) -> Option<Handle<OperatorClass>> {
        let (schema, name) = self.key(name)?;
        self.operator_class_by_name
            .get(&(schema, name, Name::new(method)))
            .copied()
    }

    /// The operator class with this name for this access method, in the
    /// first schema of the search path that has one.
    pub fn operator_class_visible(
        &self,
        path: &SearchPath,
        name: &str,
        method: &str,
    ) -> Option<Handle<OperatorClass>> {
        let name = Name::new(name);
        let method = Name::new(method);
        path.schemas().iter().find_map(|schema| {
            self.operator_class_by_name
                .get(&(*schema, name.clone(), method.clone()))
                .copied()
        })
    }

    /// The qualified name of an operator class.
    pub fn qualified_name_of_operator_class(&self, h: Handle<OperatorClass>) -> QualifiedName {
        let entity = &self[h];
        QualifiedName::new(self[entity.schema].name.clone(), entity.name.clone())
    }

    /// Every configuration parameter, in the order of the lower-case name.
    ///
    /// The list is what `pg_settings` gave the session of the loader. It does
    /// not hold a parameter that the role of that session cannot see. It does
    /// not hold a custom parameter, such as `myapp.setting`, which is the
    /// state of a session. It holds a parameter of a loadable module, such as
    /// `plpgsql.variable_conflict`, only when that session had the module in
    /// memory.
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    /// The configuration parameter with this name.
    ///
    /// The comparison does not see the case of an ASCII letter, as in the
    /// server. The result holds the canonical spelling: a lookup of
    /// `timezone` gives the parameter with the name `TimeZone`.
    pub fn parameter(&self, name: &str) -> Option<&Parameter> {
        self.parameter_by_name
            .get(&parameter::lookup_key(name))
            .map(|index| &self.parameters[*index])
    }

    /// The schema with this OID.
    pub fn schema_by_oid(&self, oid: Oid) -> Option<Handle<Schema>> {
        self.schema_by_oid.get(&oid).copied()
    }

    /// The extension with this OID.
    pub fn extension_by_oid(&self, oid: Oid) -> Option<Handle<Extension>> {
        self.extension_by_oid.get(&oid).copied()
    }

    /// The collation with this OID.
    pub fn collation_by_oid(&self, oid: Oid) -> Option<Handle<Collation>> {
        self.collation_by_oid.get(&oid).copied()
    }

    /// The type with this OID.
    pub fn type_by_oid(&self, oid: Oid) -> Option<Handle<Type>> {
        self.type_by_oid.get(&oid).copied()
    }

    /// The relation with this OID.
    pub fn relation_by_oid(&self, oid: Oid) -> Option<Handle<Relation>> {
        self.relation_by_oid.get(&oid).copied()
    }

    /// The function with this OID.
    pub fn function_by_oid(&self, oid: Oid) -> Option<Handle<Function>> {
        self.function_by_oid.get(&oid).copied()
    }

    /// The operator with this OID.
    pub fn operator_by_oid(&self, oid: Oid) -> Option<Handle<Operator>> {
        self.operator_by_oid.get(&oid).copied()
    }

    /// The cast with this OID.
    pub fn cast_by_oid(&self, oid: Oid) -> Option<Handle<Cast>> {
        self.cast_by_oid.get(&oid).copied()
    }

    /// The schema with this name.
    pub fn schema_named(&self, name: &str) -> Option<Handle<Schema>> {
        self.schema_by_name.get(name).copied()
    }

    /// The temporary schema of the session that the loader used.
    ///
    /// This is `None` when that session had no temporary schema.
    pub fn temporary_schema(&self) -> Option<Handle<Schema>> {
        self.schemas()
            .find(|(_, schema)| schema.is_temporary)
            .map(|(handle, _)| handle)
    }

    /// The extension with this name.
    pub fn extension_named(&self, name: &str) -> Option<Handle<Extension>> {
        self.extension_by_name.get(name).copied()
    }

    /// The key that a name-keyed index uses.
    fn key(&self, name: &QualifiedName) -> Option<(Handle<Schema>, Name)> {
        let schema = self.schema_named(name.schema.as_str())?;
        Some((schema, name.name.clone()))
    }

    /// Every collation with this qualified name, for each encoding, in OID
    /// order.
    ///
    /// Use [`collation_named`](Catalog::collation_named) to find the one that
    /// a statement names. This lookup tells a consumer why that one found
    /// nothing: no collation has the name, or one has it for another
    /// encoding.
    pub fn collations_named(&self, name: &QualifiedName) -> &[Handle<Collation>] {
        self.key(name)
            .and_then(|key| self.collations_by_name.get(&key))
            .map_or(&[], Vec::as_slice)
    }

    /// The collation with this qualified name that the database can use.
    ///
    /// This is the rule of `lookup_collation` in PostgreSQL. A collation for
    /// the encoding of the database wins. A collation for each encoding is
    /// next. An ICU collation is not a match when ICU cannot work with the
    /// encoding of the database. The encoding is
    /// `catalog.metadata().encoding`.
    pub fn collation_named(&self, name: &QualifiedName) -> Option<Handle<Collation>> {
        self.usable_collation(self.collations_named(name))
    }

    /// The collation with this name in the first schema of the search path
    /// that has one that the database can use.
    ///
    /// The rule for one schema is that of
    /// [`collation_named`](Catalog::collation_named). A schema that has the
    /// name only for another encoding does not stop the search, as in
    /// `CollationGetCollid` of PostgreSQL.
    pub fn collation_visible(&self, path: &SearchPath, name: &str) -> Option<Handle<Collation>> {
        let key = Name::new(name);
        path.schemas().iter().find_map(|schema| {
            let group = self.collations_by_name.get(&(*schema, key.clone()))?;
            self.usable_collation(group)
        })
    }

    /// The collation of a group that the database can use.
    fn usable_collation(&self, group: &[Handle<Collation>]) -> Option<Handle<Collation>> {
        let encoding = &self.metadata.encoding;
        let usable = |handle: &&Handle<Collation>| encoding.can_use(&self[**handle]);
        group
            .iter()
            .filter(usable)
            .find(|handle| self[**handle].encoding.is_some())
            .or_else(|| group.iter().find(usable))
            .copied()
    }

    /// The type with this qualified name.
    pub fn type_named(&self, name: &QualifiedName) -> Option<Handle<Type>> {
        let key = self.key(name)?;
        self.type_by_name.get(&key).copied()
    }

    /// The relation with this qualified name.
    pub fn relation_named(&self, name: &QualifiedName) -> Option<Handle<Relation>> {
        let key = self.key(name)?;
        self.relation_by_name.get(&key).copied()
    }

    /// Every overload with this qualified name, in OID order.
    pub fn functions_named(&self, name: &QualifiedName) -> &[Handle<Function>] {
        match self.key(name) {
            Some(key) => self
                .functions_by_name
                .get(&key)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            None => &[],
        }
    }

    /// Every overload with this qualified name, in OID order.
    pub fn operators_named(&self, name: &QualifiedName) -> &[Handle<Operator>] {
        match self.key(name) {
            Some(key) => self
                .operators_by_name
                .get(&key)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            None => &[],
        }
    }

    /// The first type with this name along the search path.
    pub fn type_visible(&self, path: &SearchPath, name: &str) -> Option<Handle<Type>> {
        let key = Name::new(name);
        path.schemas()
            .iter()
            .find_map(|schema| self.type_by_name.get(&(*schema, key.clone())).copied())
    }

    /// The first relation with this name along the search path.
    pub fn relation_visible(&self, path: &SearchPath, name: &str) -> Option<Handle<Relation>> {
        let key = Name::new(name);
        path.schemas()
            .iter()
            .find_map(|schema| self.relation_by_name.get(&(*schema, key.clone())).copied())
    }

    /// Every function with this name along the search path.
    ///
    /// The order is search-path order, then OID order inside one schema. The
    /// hiding rule, where a function in an earlier schema hides one with the
    /// same signature, belongs to the consumer.
    ///
    /// The lookup skips a temporary schema. PostgreSQL never searches
    /// `pg_temp` for the name of a function or of an operator, so that a
    /// temporary object cannot take the place of a permanent one. A qualified
    /// name still finds the function, through
    /// [`functions_named`](Catalog::functions_named).
    pub fn functions_visible(&self, path: &SearchPath, name: &str) -> Vec<Handle<Function>> {
        let key = Name::new(name);
        let mut found = Vec::new();
        for schema in self.permanent_schemas(path) {
            if let Some(group) = self.functions_by_name.get(&(*schema, key.clone())) {
                found.extend_from_slice(group);
            }
        }
        found
    }

    /// Every operator with this name along the search path.
    ///
    /// The order is the same as for
    /// [`functions_visible`](Catalog::functions_visible). The lookup skips a
    /// temporary schema, for the same reason.
    pub fn operators_visible(&self, path: &SearchPath, name: &str) -> Vec<Handle<Operator>> {
        let key = Name::new(name);
        let mut found = Vec::new();
        for schema in self.permanent_schemas(path) {
            if let Some(group) = self.operators_by_name.get(&(*schema, key.clone())) {
                found.extend_from_slice(group);
            }
        }
        found
    }

    /// The schemas of a search path, without a temporary schema.
    fn permanent_schemas<'a>(
        &'a self,
        path: &'a SearchPath,
    ) -> impl Iterator<Item = &'a Handle<Schema>> {
        path.schemas()
            .iter()
            .filter(|schema| !self[**schema].is_temporary)
    }

    /// The comparison operators of the default btree operator class of a
    /// type.
    ///
    /// PostgreSQL reads these operators to sort and to group: `ORDER BY`,
    /// `GROUP BY`, `DISTINCT`, a set operation, and a window frame. The
    /// operators are the members of the operator family of the class that
    /// take the type on both sides. A cross-type member, such as
    /// `int4 < int8`, is not here.
    ///
    /// The lookup is for the exact type only. It is `None` when no default
    /// btree operator class names this type as its input type. PostgreSQL
    /// looks further, and the consumer must do the same:
    ///
    /// - A domain uses the class of its base type.
    /// - A type that is binary-coercible to the input type of a class uses
    ///   that class. `varchar` uses the class of `text` in this way.
    /// - An array, an enum, a range, a multirange, and a composite type use
    ///   the class of `anyarray`, `anyenum`, `anyrange`, `anymultirange`,
    ///   and `record`. Those pseudo-types have an answer here. For an array,
    ///   a range, and a composite, PostgreSQL then also examines the element
    ///   type, the subtype, or each column type.
    pub fn btree_ordering(&self, ty: Handle<Type>) -> Option<BtreeOperators> {
        self[ty].btree
    }

    /// The `=` operator of the default hash operator class of a type.
    ///
    /// PostgreSQL uses this operator to group by hashing. It also uses it as
    /// the equality operator of a type that has no btree operator class,
    /// such as `xid`.
    ///
    /// The lookup is for the exact type only, as for
    /// [`btree_ordering`](Catalog::btree_ordering).
    pub fn hash_equality(&self, ty: Handle<Type>) -> Option<Handle<Operator>> {
        self[ty].hash_equality
    }

    /// The stored cast from one type to another, when the catalog holds one.
    pub fn cast(&self, from: Handle<Type>, to: Handle<Type>) -> Option<Handle<Cast>> {
        self.cast_by_pair.get(&(from, to)).copied()
    }

    /// Every stored cast whose source is this type, in OID order.
    pub fn casts_from(&self, from: Handle<Type>) -> &[Handle<Cast>] {
        self.casts_by_source
            .get(&from)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// The qualified name of a collation.
    pub fn qualified_name_of_collation(&self, h: Handle<Collation>) -> QualifiedName {
        let entity = &self[h];
        QualifiedName::new(self[entity.schema].name.clone(), entity.name.clone())
    }

    /// The qualified name of a type.
    pub fn qualified_name_of_type(&self, h: Handle<Type>) -> QualifiedName {
        let entity = &self[h];
        QualifiedName::new(self[entity.schema].name.clone(), entity.name.clone())
    }

    /// The qualified name of a relation.
    pub fn qualified_name_of_relation(&self, h: Handle<Relation>) -> QualifiedName {
        let entity = &self[h];
        QualifiedName::new(self[entity.schema].name.clone(), entity.name.clone())
    }

    /// The qualified name of a function.
    pub fn qualified_name_of_function(&self, h: Handle<Function>) -> QualifiedName {
        let entity = &self[h];
        QualifiedName::new(self[entity.schema].name.clone(), entity.name.clone())
    }

    /// The qualified name of an operator.
    pub fn qualified_name_of_operator(&self, h: Handle<Operator>) -> QualifiedName {
        let entity = &self[h];
        QualifiedName::new(self[entity.schema].name.clone(), entity.name.clone())
    }
}

macro_rules! impl_index {
    ($entity:ty, $field:ident) => {
        impl Index<Handle<$entity>> for Catalog {
            type Output = $entity;

            fn index(&self, handle: Handle<$entity>) -> &$entity {
                &self.$field[handle.index() as usize]
            }
        }
    };
}

impl_index!(Schema, schemas);
impl_index!(Extension, extensions);
impl_index!(Collation, collations);
impl_index!(Type, types);
impl_index!(Relation, relations);
impl_index!(Function, functions);
impl_index!(Operator, operators);
impl_index!(Cast, casts);
impl_index!(OperatorClass, operator_classes);

/// The serialized form of a [`Catalog`].
///
/// A catalog keeps its indexes out of the serialized form. The deserializer
/// rebuilds them through the builder.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct CatalogData {
    metadata: Metadata,
    schemas: Vec<Schema>,
    extensions: Vec<Extension>,
    #[serde(default)]
    collations: Vec<Collation>,
    types: Vec<Type>,
    relations: Vec<Relation>,
    functions: Vec<Function>,
    operators: Vec<Operator>,
    casts: Vec<Cast>,
    #[serde(default)]
    operator_classes: Vec<OperatorClass>,
    #[serde(default)]
    parameters: Vec<Parameter>,
}

/// The borrowed twin of `CatalogData`, so that serialization copies nothing.
#[cfg(feature = "serde")]
#[derive(serde::Serialize)]
#[serde(rename = "CatalogData")]
struct CatalogDataRef<'a> {
    metadata: &'a Metadata,
    schemas: &'a [Schema],
    extensions: &'a [Extension],
    collations: &'a [Collation],
    types: &'a [Type],
    relations: &'a [Relation],
    functions: &'a [Function],
    operators: &'a [Operator],
    casts: &'a [Cast],
    operator_classes: &'a [OperatorClass],
    parameters: &'a [Parameter],
}

#[cfg(feature = "serde")]
impl serde::Serialize for Catalog {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        CatalogDataRef {
            metadata: &self.metadata,
            schemas: &self.schemas,
            extensions: &self.extensions,
            collations: &self.collations,
            types: &self.types,
            relations: &self.relations,
            functions: &self.functions,
            operators: &self.operators,
            casts: &self.casts,
            operator_classes: &self.operator_classes,
            parameters: &self.parameters,
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Catalog {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;

        let data = CatalogData::deserialize(deserializer)?;
        if data.metadata.format_version != Catalog::FORMAT_VERSION {
            return Err(D::Error::custom(format!(
                "catalog format version {} does not match {}",
                data.metadata.format_version,
                Catalog::FORMAT_VERSION
            )));
        }

        let mut builder = crate::builder::CatalogBuilder::new(data.metadata);
        for entity in data.schemas {
            builder.push_schema(entity);
        }
        for entity in data.extensions {
            builder.push_extension(entity);
        }
        for entity in data.collations {
            builder.push_collation(entity);
        }
        for entity in data.types {
            builder.push_type(entity);
        }
        for entity in data.relations {
            builder.push_relation(entity);
        }
        for entity in data.functions {
            builder.push_function(entity);
        }
        for entity in data.operators {
            builder.push_operator(entity);
        }
        for entity in data.casts {
            builder.push_cast(entity);
        }
        for entity in data.operator_classes {
            builder.push_operator_class(entity);
        }
        for entity in data.parameters {
            builder.push_parameter(entity);
        }
        builder.finish().map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::{Catalog, Metadata, ServerVersion};

    #[test]
    fn server_version_prints_major_and_minor() {
        let version = ServerVersion {
            major: 17,
            minor: 2,
        };
        assert_eq!(version.to_string(), "17.2");
    }

    #[test]
    fn placeholder_metadata_states_no_server() {
        let metadata = Metadata::placeholder();
        assert_eq!(metadata.server_version.major, 0);
        assert_eq!(metadata.database.as_str(), "placeholder");
        assert!(metadata.search_path.is_empty());
        assert_eq!(metadata.loaded_at_unix_secs, 0);
        assert_eq!(metadata.format_version, Catalog::FORMAT_VERSION);
    }

    #[test]
    fn a_catalog_moves_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Catalog>();
    }
}
