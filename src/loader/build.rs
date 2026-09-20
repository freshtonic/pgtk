//! The assembly policy: the loader turns query rows into a catalog here.
//!
//! The policy is section 10 of `docs/spec.md`. The steps are:
//!
//! 1. Read every query into a typed row.
//! 2. Keep every schema. Keep an extension when its schema is loaded.
//! 3. Compute the set of types to keep, to a fixpoint.
//! 4. Compute the set of relations to keep.
//! 5. Give each kept entity a handle, in OID order.
//! 6. Build every entity. A cycle closes through a handle that step 5
//!    computed, so nothing needs a patch afterwards.
//! 7. Push everything into the builder, in handle order, and finish.
//!
//! An entity that the loader cannot keep produces a
//! [`Diagnostic`](crate::Diagnostic). Only a value that makes a query wrong
//! stops the load.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::builder::CatalogBuilder;
use crate::cast::{Cast, CastContext, CastMethod};
use crate::catalog::{Catalog, Metadata, ServerVersion};
use crate::diagnostic::{Diagnostic, EntityKind, SkipReason};
use crate::error::LoadError;
use crate::extension::Extension;
use crate::function::{
    AggregateDetail, AggregateKind, ArgMode, Argument, Function, FunctionKind, Volatility,
};
use crate::handle::Handle;
use crate::loader::parse::Cells;
use crate::loader::queries::QueryId;
use crate::loader::{Loaded, Rows};
use crate::name::Name;
use crate::oid::Oid;
use crate::operator::{Operator, OperatorKind};
use crate::relation::{Column, Relation, RelationKind};
use crate::schema::Schema;
use crate::types::{
    BtreeOperators, CompositeDetail, DomainDetail, Field, PseudoType, RangeDetail, Type,
    TypeCategory, TypeKind,
};
use crate::typmod::Typmod;

// ---------------------------------------------------------------------------
// Raw rows
// ---------------------------------------------------------------------------

/// One row of [`QueryId::Schemas`].
#[derive(Debug, Clone)]
struct SchemaRow {
    /// The OID of the schema.
    oid: Oid,
    /// The name of the schema.
    name: Name,
    /// True for the temporary schema of this session.
    is_temporary: bool,
}

/// One row of [`QueryId::Extensions`].
#[derive(Debug, Clone)]
struct ExtensionRow {
    /// The OID of the extension.
    oid: Oid,
    /// The name of the extension.
    name: Name,
    /// The installed version.
    version: String,
    /// The OID of the schema that holds the extension.
    namespace: Oid,
}

/// One row of [`QueryId::ExtensionMembers`].
#[derive(Debug, Clone)]
struct MemberRow {
    /// The catalog that holds the member object.
    class: MemberClass,
    /// The OID of the member object.
    oid: Oid,
    /// The OID of the extension that owns it.
    extension: Oid,
}

/// The catalog that holds one member of an extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MemberClass {
    /// A row of `pg_type`.
    Type,
    /// A row of `pg_class`.
    Class,
    /// A row of `pg_proc`.
    Proc,
    /// A row of `pg_operator`.
    Operator,
    /// A row of `pg_cast`.
    Cast,
    /// A row of `pg_namespace`.
    Namespace,
    /// A catalog that this model does not hold.
    Other,
}

impl MemberClass {
    /// Read the `class_label` of a member row.
    fn from_label(label: &str) -> Self {
        match label {
            "type" => MemberClass::Type,
            "class" => MemberClass::Class,
            "proc" => MemberClass::Proc,
            "operator" => MemberClass::Operator,
            "cast" => MemberClass::Cast,
            "namespace" => MemberClass::Namespace,
            _ => MemberClass::Other,
        }
    }
}

/// One row of [`QueryId::Types`].
#[derive(Debug, Clone)]
struct TypeRow {
    /// The OID of the type.
    oid: Oid,
    /// The name of the type.
    name: Name,
    /// The OID of the schema that holds the type.
    namespace: Oid,
    /// The `typtype` letter.
    typtype: char,
    /// The `typcategory` letter.
    category: char,
    /// True when the type is preferred in its category.
    is_preferred: bool,
    /// True when the type takes a collation.
    collatable: bool,
    /// The OID of the array of this type, or 0.
    typarray: Oid,
    /// The OID of the element type, or 0.
    typelem: Oid,
    /// True when the type is a true array.
    is_array: bool,
    /// The OID of the class that backs a composite, or 0.
    typrelid: Oid,
    /// The `relkind` of that class.
    relkind: Option<char>,
    /// The OID of the base type of a domain, or 0.
    typbasetype: Oid,
    /// The type modifier that a domain applies.
    typtypmod: i32,
    /// True when a domain declares `NOT NULL`.
    typnotnull: bool,
    /// The OID of the subtype of a range.
    rngsubtype: Option<Oid>,
    /// The OID of the multirange of a range.
    rngmultitypid: Option<Oid>,
    /// The OID of the range that a multirange holds.
    multirange_of: Option<Oid>,
    /// The number of `CHECK` constraints on a domain.
    check_count: u32,
}

/// One row of [`QueryId::Attributes`].
#[derive(Debug, Clone)]
struct AttributeRow {
    /// The OID of the relation or of the composite class.
    relid: Oid,
    /// The attribute position, which starts at 1.
    num: i16,
    /// The name of the attribute.
    name: Name,
    /// The OID of the type of the attribute.
    typid: Oid,
    /// The type modifier.
    typmod: i32,
    /// True when the attribute declares `NOT NULL`.
    not_null: bool,
    /// True when the attribute has a default.
    has_default: bool,
    /// True when the attribute is a generated column.
    generated: bool,
    /// True when the attribute is an identity column.
    identity: bool,
}

/// One row of [`QueryId::Relations`].
#[derive(Debug, Clone)]
struct RelationRow {
    /// The OID of the relation.
    oid: Oid,
    /// The name of the relation.
    name: Name,
    /// The OID of the schema that holds the relation.
    namespace: Oid,
    /// The `relkind` letter.
    relkind: char,
    /// The OID of the row type.
    reltype: Oid,
}

/// One row of [`QueryId::Functions`].
#[derive(Debug, Clone)]
struct FunctionRow {
    /// The OID of the function.
    oid: Oid,
    /// The name of the function.
    name: Name,
    /// The OID of the schema that holds the function.
    namespace: Oid,
    /// The `prokind` letter.
    prokind: char,
    /// The OID of the variadic element type, or 0.
    variadic: Oid,
    /// The number of arguments that have a default.
    default_count: u16,
    /// The OID of the return type.
    rettype: Oid,
    /// True when the function returns a set.
    retset: bool,
    /// True when the function is strict.
    strict: bool,
    /// The `provolatile` letter.
    volatile: char,
}

/// One row of [`QueryId::FunctionArguments`].
#[derive(Debug, Clone)]
struct ArgumentRow {
    /// The OID of the function.
    function: Oid,
    /// The OID of the type of the argument.
    typid: Oid,
    /// The mode letter. `None` means an IN argument.
    mode: Option<char>,
    /// The name of the argument, when it has one.
    name: Option<Name>,
}

/// One row of [`QueryId::Aggregates`].
#[derive(Debug, Clone)]
struct AggregateRow {
    /// The `aggkind` letter.
    kind: char,
    /// The number of direct arguments.
    direct_args: u16,
}

/// One row of [`QueryId::Operators`].
#[derive(Debug, Clone)]
struct OperatorRow {
    /// The OID of the operator.
    oid: Oid,
    /// The symbol of the operator.
    name: Name,
    /// The OID of the schema that holds the operator.
    namespace: Oid,
    /// The `oprkind` letter.
    oprkind: char,
    /// The OID of the left operand type, or 0.
    left: Oid,
    /// The OID of the right operand type.
    right: Oid,
    /// The OID of the result type.
    result: Oid,
    /// The OID of the function that implements the operator, or 0.
    code: Oid,
    /// The OID of the commutator, or 0.
    commutator: Oid,
    /// The OID of the negator, or 0.
    negator: Oid,
}

/// One row of [`QueryId::Casts`].
#[derive(Debug, Clone)]
struct CastRow {
    /// The OID of the cast.
    oid: Oid,
    /// The OID of the source type.
    source: Oid,
    /// The OID of the target type.
    target: Oid,
    /// The OID of the cast function, or 0.
    func: Oid,
    /// The `castcontext` letter.
    context: char,
    /// The `castmethod` letter.
    method: char,
}

// ---------------------------------------------------------------------------
// Reading the rows
// ---------------------------------------------------------------------------

/// The rows of one query, or [`LoadError::MissingRows`].
fn rows_of(rows: &HashMap<QueryId, Rows>, query: QueryId) -> Result<&Rows, LoadError> {
    rows.get(&query).ok_or(LoadError::MissingRows { query })
}

/// Read every row of one query with a function that reads one row.
fn read<T, F>(rows: &Rows, query: QueryId, mut one: F) -> Result<Vec<T>, LoadError>
where
    F: FnMut(Cells<'_>) -> Result<T, LoadError>,
{
    rows.iter()
        .enumerate()
        .map(|(index, cells)| one(Cells::new(query, index, cells)))
        .collect()
}

/// The one text value of a query that gives one row and one column.
fn single_text(rows: &HashMap<QueryId, Rows>, query: QueryId) -> Result<String, LoadError> {
    let rows = rows_of(rows, query)?;
    let empty: Vec<Option<String>> = Vec::new();
    let first = rows.first().unwrap_or(&empty);
    Ok(Cells::new(query, 0, first).text(0)?.to_owned())
}

/// The first column of every row of a query.
fn first_column(rows: &HashMap<QueryId, Rows>, query: QueryId) -> Result<Vec<Name>, LoadError> {
    read(rows_of(rows, query)?, query, |row| row.name(0))
}

/// Read [`QueryId::Schemas`].
fn schema_rows(rows: &Rows) -> Result<Vec<SchemaRow>, LoadError> {
    read(rows, QueryId::Schemas, |row| {
        Ok(SchemaRow {
            oid: row.oid(0)?,
            name: row.name(1)?,
            is_temporary: row.bool(2)?,
        })
    })
}

/// Read [`QueryId::Extensions`].
fn extension_rows(rows: &Rows) -> Result<Vec<ExtensionRow>, LoadError> {
    read(rows, QueryId::Extensions, |row| {
        Ok(ExtensionRow {
            oid: row.oid(0)?,
            name: row.name(1)?,
            version: row.text(2)?.to_owned(),
            namespace: row.oid(3)?,
        })
    })
}

/// Read [`QueryId::ExtensionMembers`].
fn member_rows(rows: &Rows) -> Result<Vec<MemberRow>, LoadError> {
    read(rows, QueryId::ExtensionMembers, |row| {
        Ok(MemberRow {
            class: MemberClass::from_label(row.text(0)?),
            oid: row.oid(1)?,
            extension: row.oid(2)?,
        })
    })
}

/// Read [`QueryId::Types`].
fn type_rows(rows: &Rows) -> Result<Vec<TypeRow>, LoadError> {
    read(rows, QueryId::Types, |row| {
        Ok(TypeRow {
            oid: row.oid(0)?,
            name: row.name(1)?,
            namespace: row.oid(2)?,
            typtype: row.char(3)?,
            category: row.char(4)?,
            is_preferred: row.bool(5)?,
            collatable: row.bool(6)?,
            typarray: row.oid(7)?,
            typelem: row.oid(8)?,
            is_array: row.bool(9)?,
            typrelid: row.oid(10)?,
            relkind: row.opt_char(11)?,
            typbasetype: row.oid(12)?,
            typtypmod: row.i32(13)?,
            typnotnull: row.bool(14)?,
            rngsubtype: row.opt_oid(15)?,
            rngmultitypid: row.opt_oid(16)?,
            multirange_of: row.opt_oid(17)?,
            check_count: row.u32(18)?,
        })
    })
}

/// Read [`QueryId::EnumLabels`] into the labels of each enum type.
///
/// The query gives the labels in `enumsortorder`, so the order of the rows
/// is the order of the labels.
fn enum_labels(rows: &Rows) -> Result<HashMap<Oid, Vec<Name>>, LoadError> {
    let mut labels: HashMap<Oid, Vec<Name>> = HashMap::new();
    for (index, cells) in rows.iter().enumerate() {
        let row = Cells::new(QueryId::EnumLabels, index, cells);
        labels.entry(row.oid(0)?).or_default().push(row.name(2)?);
    }
    Ok(labels)
}

/// Read [`QueryId::Relations`].
fn relation_rows(rows: &Rows) -> Result<Vec<RelationRow>, LoadError> {
    read(rows, QueryId::Relations, |row| {
        Ok(RelationRow {
            oid: row.oid(0)?,
            name: row.name(1)?,
            namespace: row.oid(2)?,
            relkind: row.char(3)?,
            reltype: row.oid(4)?,
        })
    })
}

/// Read [`QueryId::Attributes`] into the attributes of each class.
///
/// The query gives the attributes in `attnum`, so the order of the rows is
/// the order of the attributes.
fn attribute_rows(rows: &Rows) -> Result<HashMap<Oid, Vec<AttributeRow>>, LoadError> {
    let mut attributes: HashMap<Oid, Vec<AttributeRow>> = HashMap::new();
    for (index, cells) in rows.iter().enumerate() {
        let row = Cells::new(QueryId::Attributes, index, cells);
        let attribute = AttributeRow {
            relid: row.oid(0)?,
            num: row.i16(1)?,
            name: row.name(2)?,
            typid: row.oid(3)?,
            typmod: row.i32(4)?,
            not_null: row.bool(5)?,
            has_default: row.bool(6)?,
            generated: row.bool(7)?,
            identity: row.bool(8)?,
        };
        attributes
            .entry(attribute.relid)
            .or_default()
            .push(attribute);
    }
    Ok(attributes)
}

/// Read [`QueryId::Functions`].
fn function_rows(rows: &Rows) -> Result<Vec<FunctionRow>, LoadError> {
    read(rows, QueryId::Functions, |row| {
        Ok(FunctionRow {
            oid: row.oid(0)?,
            name: row.name(1)?,
            namespace: row.oid(2)?,
            prokind: row.char(3)?,
            variadic: row.oid(4)?,
            default_count: row.u16(5)?,
            rettype: row.oid(6)?,
            retset: row.bool(7)?,
            strict: row.bool(8)?,
            volatile: row.char(9)?,
        })
    })
}

/// Read [`QueryId::FunctionArguments`] into the arguments of each function.
///
/// The query gives the arguments in argument order, so the order of the rows
/// is the order of the arguments.
fn argument_rows(rows: &Rows) -> Result<HashMap<Oid, Vec<ArgumentRow>>, LoadError> {
    let mut arguments: HashMap<Oid, Vec<ArgumentRow>> = HashMap::new();
    for (index, cells) in rows.iter().enumerate() {
        let row = Cells::new(QueryId::FunctionArguments, index, cells);
        let argument = ArgumentRow {
            function: row.oid(0)?,
            typid: row.oid(2)?,
            mode: row.opt_char(3)?,
            name: row.opt_name(4)?,
        };
        arguments
            .entry(argument.function)
            .or_default()
            .push(argument);
    }
    Ok(arguments)
}

/// Read [`QueryId::Aggregates`] into the detail of each aggregate.
fn aggregate_rows(rows: &Rows) -> Result<HashMap<Oid, AggregateRow>, LoadError> {
    let mut aggregates = HashMap::new();
    for (index, cells) in rows.iter().enumerate() {
        let row = Cells::new(QueryId::Aggregates, index, cells);
        let oid = row.oid(0)?;
        aggregates.insert(
            oid,
            AggregateRow {
                kind: row.char(1)?,
                direct_args: row.u16(2)?,
            },
        );
    }
    Ok(aggregates)
}

/// Read [`QueryId::Operators`].
fn operator_rows(rows: &Rows) -> Result<Vec<OperatorRow>, LoadError> {
    read(rows, QueryId::Operators, |row| {
        Ok(OperatorRow {
            oid: row.oid(0)?,
            name: row.name(1)?,
            namespace: row.oid(2)?,
            oprkind: row.char(3)?,
            left: row.oid(4)?,
            right: row.oid(5)?,
            result: row.oid(6)?,
            code: row.oid(7)?,
            commutator: row.oid(8)?,
            negator: row.oid(9)?,
        })
    })
}

/// Read [`QueryId::Casts`].
fn cast_rows(rows: &Rows) -> Result<Vec<CastRow>, LoadError> {
    read(rows, QueryId::Casts, |row| {
        Ok(CastRow {
            oid: row.oid(0)?,
            source: row.oid(1)?,
            target: row.oid(2)?,
            func: row.oid(3)?,
            context: row.char(4)?,
            method: row.char(5)?,
        })
    })
}

/// One row of [`QueryId::DefaultOperatorClasses`].
#[derive(Debug, Clone)]
struct OperatorClassRow {
    /// The OID of the operator class.
    oid: Oid,
    /// The name of the operator class.
    name: Name,
    /// The name of the access method: `btree` or `hash`.
    method: String,
    /// The OID of the input type of the class.
    intype: Oid,
    /// The strategy number of the operator.
    strategy: i16,
    /// The OID of the operator.
    operator: Oid,
}

/// Read [`QueryId::DefaultOperatorClasses`].
fn operator_class_rows(rows: &Rows) -> Result<Vec<OperatorClassRow>, LoadError> {
    read(rows, QueryId::DefaultOperatorClasses, |row| {
        Ok(OperatorClassRow {
            oid: row.oid(0)?,
            name: row.name(1)?,
            method: row.text(2)?.to_owned(),
            intype: row.oid(3)?,
            strategy: row.i16(4)?,
            operator: row.oid(5)?,
        })
    })
}

/// Every query, read into typed rows.
struct Input {
    /// The name of the database.
    database: Name,
    /// The effective search path at the moment of the load.
    search_path: Vec<Name>,
    /// Every schema.
    schemas: Vec<SchemaRow>,
    /// Every extension.
    extensions: Vec<ExtensionRow>,
    /// Every object that belongs to an extension.
    members: Vec<MemberRow>,
    /// Every type.
    types: Vec<TypeRow>,
    /// The labels of each enum type.
    labels: HashMap<Oid, Vec<Name>>,
    /// Every relation.
    relations: Vec<RelationRow>,
    /// The attributes of each class.
    attributes: HashMap<Oid, Vec<AttributeRow>>,
    /// Every function.
    functions: Vec<FunctionRow>,
    /// The arguments of each function.
    arguments: HashMap<Oid, Vec<ArgumentRow>>,
    /// The detail of each aggregate.
    aggregates: HashMap<Oid, AggregateRow>,
    /// Every operator.
    operators: Vec<OperatorRow>,
    /// Every cast.
    casts: Vec<CastRow>,
    /// The operators of every default btree and hash operator class.
    operator_classes: Vec<OperatorClassRow>,
}

impl Input {
    /// Read every query.
    fn read(rows: &HashMap<QueryId, Rows>) -> Result<Self, LoadError> {
        let mut input = Input {
            database: Name::new(single_text(rows, QueryId::Database)?),
            search_path: first_column(rows, QueryId::SearchPath)?,
            schemas: schema_rows(rows_of(rows, QueryId::Schemas)?)?,
            extensions: extension_rows(rows_of(rows, QueryId::Extensions)?)?,
            members: member_rows(rows_of(rows, QueryId::ExtensionMembers)?)?,
            types: type_rows(rows_of(rows, QueryId::Types)?)?,
            labels: enum_labels(rows_of(rows, QueryId::EnumLabels)?)?,
            relations: relation_rows(rows_of(rows, QueryId::Relations)?)?,
            attributes: attribute_rows(rows_of(rows, QueryId::Attributes)?)?,
            functions: function_rows(rows_of(rows, QueryId::Functions)?)?,
            arguments: argument_rows(rows_of(rows, QueryId::FunctionArguments)?)?,
            aggregates: aggregate_rows(rows_of(rows, QueryId::Aggregates)?)?,
            operators: operator_rows(rows_of(rows, QueryId::Operators)?)?,
            casts: cast_rows(rows_of(rows, QueryId::Casts)?)?,
            operator_classes: operator_class_rows(rows_of(rows, QueryId::DefaultOperatorClasses)?)?,
        };
        // A handle is the position in the arena, and an arena is in OID
        // order. Every query already sorts by OID; the sort here makes the
        // order of the handles certain even when a caller feeds its own
        // rows.
        input.schemas.sort_by_key(|row| row.oid);
        input.extensions.sort_by_key(|row| row.oid);
        input.types.sort_by_key(|row| row.oid);
        input.relations.sort_by_key(|row| row.oid);
        input.functions.sort_by_key(|row| row.oid);
        input.operators.sort_by_key(|row| row.oid);
        input.casts.sort_by_key(|row| row.oid);
        Ok(input)
    }
}

/// The moment of the load, in seconds since the Unix epoch.
fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// The kept set of types
// ---------------------------------------------------------------------------

/// The `typtype` letters that this model keeps.
const KEPT_TYPE_KINDS: &[char] = &['b', 'c', 'd', 'e', 'p', 'r', 'm'];

/// The `relkind` letters that back a composite type that this model keeps.
///
/// A relation with a row type keeps `r v m f p`. A standalone composite type
/// keeps `c`. A sequence, an index, and a TOAST table are not here. The
/// server gives those classes no row type from version 14, so this filter
/// is a guard only, and the type goes away without a diagnostic.
const KEPT_COMPOSITE_RELKINDS: &[char] = &['r', 'v', 'm', 'f', 'p', 'c'];

/// The `relkind` letters of a relation.
///
/// A sequence, `S`, is a relation that has no row type: its `reltype` is 0.
const RELATION_RELKINDS: &[char] = &['r', 'v', 'm', 'f', 'p', 'S'];

/// The `relkind` letter of a sequence.
const SEQUENCE_RELKIND: char = 'S';

/// Which rows of one arena the loader keeps.
///
/// The rows are in OID order, so the position of a kept row is its handle.
#[derive(Debug)]
struct KeptSet {
    /// The position of each OID in the rows.
    index_by_oid: HashMap<Oid, usize>,
    /// True at the position of a row that the loader keeps.
    keep: Vec<bool>,
}

impl KeptSet {
    /// Start with every row kept.
    fn all(oids: impl Iterator<Item = Oid>) -> Self {
        let oids: Vec<Oid> = oids.collect();
        let index_by_oid: HashMap<Oid, usize> = oids
            .iter()
            .enumerate()
            .map(|(index, oid)| (*oid, index))
            .collect();
        // One flag for each row, and not one for each OID that differs.
        // Every caller reads a flag by the position of a row. A hand-fed
        // row set that repeats an OID must not shorten the flags.
        let keep = vec![true; oids.len()];
        KeptSet { index_by_oid, keep }
    }

    /// True when the row with this OID is present and kept.
    fn alive(&self, oid: Oid) -> bool {
        self.index_by_oid
            .get(&oid)
            .is_some_and(|index| self.keep[*index])
    }

    /// Drop the row at one position.
    fn drop_at(&mut self, index: usize) {
        self.keep[index] = false;
    }
}

/// The first type that a type names and the loader does not keep.
///
/// The references are the array element, the array of the type, the base of
/// a domain, the subtype and the multirange of a range, the range of a
/// multirange, and every attribute type of a composite.
fn missing_type_reference(
    row: &TypeRow,
    kept: &KeptSet,
    attributes: &HashMap<Oid, Vec<AttributeRow>>,
) -> Option<Oid> {
    if row.is_array && !kept.alive(row.typelem) {
        return Some(row.typelem);
    }
    if row.typarray != Oid(0) && !kept.alive(row.typarray) {
        return Some(row.typarray);
    }
    match row.typtype {
        'd' => {
            if !kept.alive(row.typbasetype) {
                return Some(row.typbasetype);
            }
        }
        'r' => {
            let subtype = row.rngsubtype.unwrap_or(Oid(0));
            if !kept.alive(subtype) {
                return Some(subtype);
            }
            let multirange = row.rngmultitypid.unwrap_or(Oid(0));
            if !kept.alive(multirange) {
                return Some(multirange);
            }
        }
        'm' => {
            let range = row.multirange_of.unwrap_or(Oid(0));
            if !kept.alive(range) {
                return Some(range);
            }
        }
        'c' => {
            for attribute in attributes.get(&row.typrelid).into_iter().flatten() {
                if !kept.alive(attribute.typid) {
                    return Some(attribute.typid);
                }
            }
        }
        _ => {}
    }
    None
}

/// Compute the set of types to keep.
///
/// A type goes away when its `typtype` is unknown, when its schema is not
/// loaded, when it is the composite type of a class that this model does not
/// keep, or when it names a type that the loader does not keep. The last
/// rule runs to a fixpoint, because one type that goes away can take another
/// with it.
fn kept_types(
    types: &[TypeRow],
    schemas: &HashMap<Oid, Handle<Schema>>,
    attributes: &HashMap<Oid, Vec<AttributeRow>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> KeptSet {
    let mut kept = KeptSet::all(types.iter().map(|row| row.oid));

    for (index, row) in types.iter().enumerate() {
        if !KEPT_TYPE_KINDS.contains(&row.typtype) {
            kept.drop_at(index);
            diagnostics.push(Diagnostic::UnknownTypeKind {
                oid: row.oid,
                name: row.name.clone(),
                typtype: row.typtype,
            });
            continue;
        }
        if !schemas.contains_key(&row.namespace) {
            kept.drop_at(index);
            diagnostics.push(Diagnostic::MissingReference {
                entity: EntityKind::Type,
                oid: row.oid,
                name: row.name.clone(),
                referenced: EntityKind::Schema,
                referenced_oid: row.namespace,
            });
            continue;
        }
        // The composite type of a sequence, of an index, or of a TOAST
        // table goes away without a note. The model gives those classes no
        // row type, so the type has no place.
        if row.typtype == 'c'
            && !row
                .relkind
                .is_some_and(|kind| KEPT_COMPOSITE_RELKINDS.contains(&kind))
        {
            kept.drop_at(index);
        }
    }

    loop {
        let mut changed = false;
        for (index, row) in types.iter().enumerate() {
            if !kept.keep[index] {
                continue;
            }
            if let Some(referenced_oid) = missing_type_reference(row, &kept, attributes) {
                kept.drop_at(index);
                changed = true;
                diagnostics.push(Diagnostic::MissingReference {
                    entity: EntityKind::Type,
                    oid: row.oid,
                    name: row.name.clone(),
                    referenced: EntityKind::Type,
                    referenced_oid,
                });
            }
        }
        if !changed {
            break;
        }
    }

    kept
}

/// Compute the set of relations to keep.
///
/// A relation stays when its schema is loaded and its row type is a type
/// that the loader keeps. A sequence has no row type, so only its schema
/// decides.
fn kept_relations(
    relations: &[RelationRow],
    schemas: &HashMap<Oid, Handle<Schema>>,
    types: &KeptSet,
    diagnostics: &mut Vec<Diagnostic>,
) -> KeptSet {
    let mut kept = KeptSet::all(relations.iter().map(|row| row.oid));
    for (index, row) in relations.iter().enumerate() {
        if !RELATION_RELKINDS.contains(&row.relkind) {
            kept.drop_at(index);
            diagnostics.push(Diagnostic::Skipped {
                entity: EntityKind::Relation,
                oid: row.oid,
                name: row.name.clone(),
                reason: SkipReason::UnsupportedRelationKind(row.relkind),
            });
            continue;
        }
        if !schemas.contains_key(&row.namespace) {
            kept.drop_at(index);
            diagnostics.push(Diagnostic::MissingReference {
                entity: EntityKind::Relation,
                oid: row.oid,
                name: row.name.clone(),
                referenced: EntityKind::Schema,
                referenced_oid: row.namespace,
            });
            continue;
        }
        if row.relkind != SEQUENCE_RELKIND && !types.alive(row.reltype) {
            kept.drop_at(index);
            diagnostics.push(Diagnostic::MissingReference {
                entity: EntityKind::Relation,
                oid: row.oid,
                name: row.name.clone(),
                referenced: EntityKind::Type,
                referenced_oid: row.reltype,
            });
        }
    }
    kept
}

/// Give a handle to every kept row, in OID order.
///
/// The rows are already in OID order, so the handle of a row is the number
/// of kept rows before it.
fn assign_handles<R, E, F>(rows: &[R], kept: &KeptSet, oid_of: F) -> HashMap<Oid, Handle<E>>
where
    F: Fn(&R) -> Oid,
{
    let mut handles = HashMap::new();
    let mut next = 0u32;
    for (index, row) in rows.iter().enumerate() {
        if kept.keep[index] {
            handles.insert(oid_of(row), Handle::new(next));
            next += 1;
        }
    }
    handles
}

/// The kind of a type, and the detail that belongs to the kind.
fn type_kind(
    row: &TypeRow,
    types: &HashMap<Oid, Handle<Type>>,
    relations: &HashMap<Oid, Handle<Relation>>,
    labels: &HashMap<Oid, Vec<Name>>,
    attributes: &HashMap<Oid, Vec<AttributeRow>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> TypeKind {
    // Every reference here survived the fixpoint, so the handle is present.
    // A reference that is absent all the same becomes a note, and the model
    // loses only that one field.
    let mut handle_of = |referenced_oid: Oid| -> Option<Handle<Type>> {
        match types.get(&referenced_oid) {
            Some(handle) => Some(*handle),
            None => {
                diagnostics.push(Diagnostic::MissingReference {
                    entity: EntityKind::Type,
                    oid: row.oid,
                    name: row.name.clone(),
                    referenced: EntityKind::Type,
                    referenced_oid,
                });
                None
            }
        }
    };

    match row.typtype {
        'p' => TypeKind::Pseudo(PseudoType::from_name(row.name.as_str())),
        'e' => TypeKind::Enum {
            labels: labels.get(&row.oid).cloned().unwrap_or_default(),
        },
        'c' => {
            let fields = attributes
                .get(&row.typrelid)
                .into_iter()
                .flatten()
                .filter_map(|attribute| {
                    Some(Field {
                        name: attribute.name.clone(),
                        ty: handle_of(attribute.typid)?,
                        typmod: Typmod(attribute.typmod),
                    })
                })
                .collect();
            let relation = row
                .relkind
                .filter(|kind| RELATION_RELKINDS.contains(kind))
                .and_then(|_| relations.get(&row.typrelid).copied());
            TypeKind::Composite(CompositeDetail { fields, relation })
        }
        'd' => match handle_of(row.typbasetype) {
            Some(base) => TypeKind::Domain(DomainDetail {
                base,
                typmod: Typmod(row.typtypmod),
                not_null: row.typnotnull,
                check_constraints: row.check_count,
            }),
            None => TypeKind::Base,
        },
        'r' => {
            let subtype = row.rngsubtype.and_then(&mut handle_of);
            let multirange = row.rngmultitypid.and_then(&mut handle_of);
            match (subtype, multirange) {
                (Some(subtype), Some(multirange)) => TypeKind::Range(RangeDetail {
                    subtype,
                    multirange,
                }),
                _ => TypeKind::Base,
            }
        }
        'm' => match row.multirange_of.and_then(&mut handle_of) {
            Some(range) => TypeKind::Multirange { range },
            None => TypeKind::Base,
        },
        // The letter is `b`, because the kept set holds no other letter.
        _ if row.is_array => match handle_of(row.typelem) {
            Some(element) => TypeKind::Array { element },
            None => TypeKind::Base,
        },
        _ => TypeKind::Base,
    }
}

/// Turn the rows of every query into a catalog.
///
/// `permit_missing_builtins` passes the same permission to the builder. The
/// loader always gives `false`; a unit test that feeds a few rows by hand
/// gives `true`.
pub(crate) fn assemble(
    version: ServerVersion,
    rows: &HashMap<QueryId, Rows>,
    permit_missing_builtins: bool,
) -> Result<Loaded, LoadError> {
    let input = Input::read(rows)?;
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    let metadata = Metadata {
        server_version: version,
        database: input.database.clone(),
        search_path: input.search_path.clone(),
        loaded_at_unix_secs: now_unix_secs(),
        format_version: Catalog::FORMAT_VERSION,
    };

    // Step 1. Every schema stays.
    let schema_handles: HashMap<Oid, Handle<Schema>> = input
        .schemas
        .iter()
        .enumerate()
        .map(|(index, row)| (row.oid, Handle::new(index as u32)))
        .collect();

    // Step 2. An extension stays when its schema is loaded.
    let mut extension_handles: HashMap<Oid, Handle<Extension>> = HashMap::new();
    let mut extensions: Vec<Extension> = Vec::new();
    for row in &input.extensions {
        match schema_handles.get(&row.namespace) {
            Some(schema) => {
                extension_handles.insert(row.oid, Handle::new(extensions.len() as u32));
                extensions.push(Extension {
                    oid: row.oid,
                    name: row.name.clone(),
                    version: row.version.clone(),
                    schema: *schema,
                });
            }
            None => diagnostics.push(Diagnostic::MissingReference {
                entity: EntityKind::Extension,
                oid: row.oid,
                name: row.name.clone(),
                referenced: EntityKind::Schema,
                referenced_oid: row.namespace,
            }),
        }
    }

    // Step 3. The map from one member object to the extension that owns it.
    // A member of an extension that went away, and a member in a catalog
    // that this model does not hold, are both absent from the map.
    let members: HashMap<(MemberClass, Oid), Handle<Extension>> = input
        .members
        .iter()
        .filter(|row| row.class != MemberClass::Other)
        .filter_map(|row| {
            extension_handles
                .get(&row.extension)
                .map(|handle| ((row.class, row.oid), *handle))
        })
        .collect();
    let owner = |class: MemberClass, oid: Oid| members.get(&(class, oid)).copied();

    // Steps 4 and 5. The kept sets.
    let kept_type_set = kept_types(
        &input.types,
        &schema_handles,
        &input.attributes,
        &mut diagnostics,
    );
    let kept_relation_set = kept_relations(
        &input.relations,
        &schema_handles,
        &kept_type_set,
        &mut diagnostics,
    );

    // Step 6. A handle is the position of the row among the kept rows, and
    // the rows are in OID order.
    let type_handles = assign_handles(&input.types, &kept_type_set, |row| row.oid);
    let relation_handles = assign_handles(&input.relations, &kept_relation_set, |row| row.oid);

    // Step 7. Build the types and the relations. The cycles between a
    // composite type and its relation, and between a type and its array,
    // close through the handles of step 6.
    let mut types: Vec<Type> = Vec::new();
    for (index, row) in input.types.iter().enumerate() {
        if !kept_type_set.keep[index] {
            continue;
        }
        let kind = type_kind(
            row,
            &type_handles,
            &relation_handles,
            &input.labels,
            &input.attributes,
            &mut diagnostics,
        );
        types.push(Type {
            oid: row.oid,
            name: row.name.clone(),
            schema: schema_handles[&row.namespace],
            category: TypeCategory::from_char(row.category),
            is_preferred: row.is_preferred,
            is_collatable: row.collatable,
            btree: None,
            hash_equality: None,
            array: type_handles.get(&row.typarray).copied(),
            kind,
            extension: owner(MemberClass::Type, row.oid),
        });
    }

    let mut relations: Vec<Relation> = Vec::new();
    for (index, row) in input.relations.iter().enumerate() {
        if !kept_relation_set.keep[index] {
            continue;
        }
        let columns = columns_of(row, &input.attributes, &type_handles, &mut diagnostics);
        relations.push(Relation {
            oid: row.oid,
            name: row.name.clone(),
            schema: schema_handles[&row.namespace],
            kind: relation_kind(row.relkind),
            row_type: type_handles.get(&row.reltype).copied(),
            columns,
            extension: owner(MemberClass::Class, row.oid),
        });
    }

    // Step 8. Functions, with the detail of an aggregate.
    let (functions, function_handles) = build_functions(
        &input,
        &schema_handles,
        &type_handles,
        &members,
        &mut diagnostics,
    );

    // Step 9. Operators.
    let (operators, operator_handles) = build_operators(
        &input,
        &schema_handles,
        &type_handles,
        &function_handles,
        &members,
        &mut diagnostics,
    );

    // Step 10. Casts.
    let mut casts: Vec<Cast> = Vec::new();
    for row in &input.casts {
        let extension = owner(MemberClass::Cast, row.oid);
        match one_cast(row, &type_handles, &function_handles, extension) {
            Ok(cast) => casts.push(cast),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    // Step 10a. The operators of the default operator classes. The types
    // are in handle order, so a handle is a position.
    attach_operator_classes(
        &input.operator_classes,
        &type_handles,
        &operator_handles,
        &mut types,
        &mut diagnostics,
    );

    // Step 12. Push in handle order. Step 11, the extension of each entity,
    // is already in place, because the map of step 3 answered every push.
    let mut builder = CatalogBuilder::new(metadata);
    if permit_missing_builtins {
        builder.permit_missing_builtins();
    }
    for row in &input.schemas {
        builder.push_schema(Schema {
            oid: row.oid,
            name: row.name.clone(),
            is_temporary: row.is_temporary,
            extension: owner(MemberClass::Namespace, row.oid),
        });
    }
    for extension in extensions {
        builder.push_extension(extension);
    }
    for ty in types {
        builder.push_type(ty);
    }
    for relation in relations {
        builder.push_relation(relation);
    }
    for function in functions {
        builder.push_function(function);
    }
    for operator in operators {
        builder.push_operator(operator);
    }
    for cast in casts {
        builder.push_cast(cast);
    }

    let catalog = builder.finish()?;
    Ok(Loaded {
        catalog,
        diagnostics,
    })
}

/// The columns of one relation.
fn columns_of(
    row: &RelationRow,
    attributes: &HashMap<Oid, Vec<AttributeRow>>,
    types: &HashMap<Oid, Handle<Type>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Column> {
    attributes
        .get(&row.oid)
        .into_iter()
        .flatten()
        .filter_map(|attribute| {
            let Some(ty) = types.get(&attribute.typid) else {
                // The row type of the relation is a composite, and the
                // fixpoint keeps every attribute type of a composite. This
                // arm is here so that a hand-fed row cannot panic.
                diagnostics.push(Diagnostic::MissingReference {
                    entity: EntityKind::Column,
                    oid: row.oid,
                    name: attribute.name.clone(),
                    referenced: EntityKind::Type,
                    referenced_oid: attribute.typid,
                });
                return None;
            };
            Some(Column {
                name: attribute.name.clone(),
                position: attribute.num,
                ty: *ty,
                typmod: Typmod(attribute.typmod),
                not_null: attribute.not_null,
                has_default: attribute.has_default,
                generated: attribute.generated,
                identity: attribute.identity,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Functions, operators, and casts
// ---------------------------------------------------------------------------

/// Make the note for a letter that the loader does not know.
fn unparsable(entity: EntityKind, oid: Oid, column: &'static str, letter: char) -> Diagnostic {
    Diagnostic::UnparsableValue {
        entity,
        oid,
        column,
        value: letter.to_string(),
    }
}

/// Make the note for a reference that the catalog does not hold.
fn missing(
    entity: EntityKind,
    oid: Oid,
    name: &Name,
    referenced: EntityKind,
    referenced_oid: Oid,
) -> Diagnostic {
    Diagnostic::MissingReference {
        entity,
        oid,
        name: name.clone(),
        referenced,
        referenced_oid,
    }
}

/// The kind of a function, from `prokind`.
fn function_kind(prokind: char) -> Option<FunctionKind> {
    match prokind {
        'f' => Some(FunctionKind::Function),
        'p' => Some(FunctionKind::Procedure),
        'a' => Some(FunctionKind::Aggregate),
        'w' => Some(FunctionKind::Window),
        _ => None,
    }
}

/// The volatility of a function, from `provolatile`.
fn volatility(letter: char) -> Option<Volatility> {
    match letter {
        'i' => Some(Volatility::Immutable),
        's' => Some(Volatility::Stable),
        'v' => Some(Volatility::Volatile),
        _ => None,
    }
}

/// The mode of an argument, from `proargmodes`. A missing letter means `In`.
fn arg_mode(letter: Option<char>) -> Option<ArgMode> {
    match letter {
        None | Some('i') => Some(ArgMode::In),
        Some('o') => Some(ArgMode::Out),
        Some('b') => Some(ArgMode::InOut),
        Some('v') => Some(ArgMode::Variadic),
        Some('t') => Some(ArgMode::Table),
        Some(_) => None,
    }
}

/// The kind of an aggregate, from `aggkind`.
fn aggregate_kind(letter: char) -> Option<AggregateKind> {
    match letter {
        'n' => Some(AggregateKind::Normal),
        'o' => Some(AggregateKind::OrderedSet),
        'h' => Some(AggregateKind::HypotheticalSet),
        _ => None,
    }
}

/// The kind of an operator, from `oprkind`.
fn operator_kind(letter: char) -> Option<OperatorKind> {
    match letter {
        'b' => Some(OperatorKind::Infix),
        'l' => Some(OperatorKind::Prefix),
        _ => None,
    }
}

/// The context of a cast, from `castcontext`.
fn cast_context(letter: char) -> Option<CastContext> {
    match letter {
        'i' => Some(CastContext::Implicit),
        'a' => Some(CastContext::Assignment),
        'e' => Some(CastContext::Explicit),
        _ => None,
    }
}

/// The arguments of one function, or the note that stops the loader keeping
/// it.
fn arguments_of(
    row: &FunctionRow,
    arguments: &HashMap<Oid, Vec<ArgumentRow>>,
    types: &HashMap<Oid, Handle<Type>>,
) -> Result<Vec<Argument>, Diagnostic> {
    let mut built = Vec::new();
    for argument in arguments.get(&row.oid).into_iter().flatten() {
        let Some(mode) = arg_mode(argument.mode) else {
            return Err(unparsable(
                EntityKind::Function,
                row.oid,
                "proargmodes",
                argument.mode.unwrap_or('?'),
            ));
        };
        let Some(ty) = types.get(&argument.typid) else {
            return Err(missing(
                EntityKind::Function,
                row.oid,
                &row.name,
                EntityKind::Type,
                argument.typid,
            ));
        };
        built.push(Argument {
            name: argument.name.clone(),
            ty: *ty,
            mode,
        });
    }
    Ok(built)
}

/// Build one function, or give the note that says why it went away.
fn one_function(
    row: &FunctionRow,
    input: &Input,
    schemas: &HashMap<Oid, Handle<Schema>>,
    types: &HashMap<Oid, Handle<Type>>,
    extension: Option<Handle<Extension>>,
) -> Result<Function, Diagnostic> {
    let Some(schema) = schemas.get(&row.namespace) else {
        return Err(missing(
            EntityKind::Function,
            row.oid,
            &row.name,
            EntityKind::Schema,
            row.namespace,
        ));
    };
    let Some(kind) = function_kind(row.prokind) else {
        return Err(unparsable(
            EntityKind::Function,
            row.oid,
            "prokind",
            row.prokind,
        ));
    };
    let Some(volatility) = volatility(row.volatile) else {
        return Err(unparsable(
            EntityKind::Function,
            row.oid,
            "provolatile",
            row.volatile,
        ));
    };
    let built = arguments_of(row, &input.arguments, types)?;
    let Some(returns) = types.get(&row.rettype) else {
        return Err(missing(
            EntityKind::Function,
            row.oid,
            &row.name,
            EntityKind::Type,
            row.rettype,
        ));
    };
    let variadic = if row.variadic == Oid(0) {
        None
    } else {
        match types.get(&row.variadic) {
            Some(handle) => Some(*handle),
            None => {
                return Err(missing(
                    EntityKind::Function,
                    row.oid,
                    &row.name,
                    EntityKind::Type,
                    row.variadic,
                ));
            }
        }
    };
    let aggregate = match input.aggregates.get(&row.oid) {
        None => None,
        Some(detail) => {
            let Some(kind) = aggregate_kind(detail.kind) else {
                return Err(unparsable(
                    EntityKind::Function,
                    row.oid,
                    "aggkind",
                    detail.kind,
                ));
            };
            Some(AggregateDetail {
                kind,
                direct_args: detail.direct_args,
            })
        }
    };
    Ok(Function {
        oid: row.oid,
        name: row.name.clone(),
        schema: *schema,
        kind,
        arguments: built,
        variadic,
        default_count: row.default_count,
        returns: *returns,
        returns_set: row.retset,
        strict: row.strict,
        volatility,
        aggregate,
        extension,
    })
}

/// Build every function that the loader keeps, with its handle.
fn build_functions(
    input: &Input,
    schemas: &HashMap<Oid, Handle<Schema>>,
    types: &HashMap<Oid, Handle<Type>>,
    members: &HashMap<(MemberClass, Oid), Handle<Extension>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<Function>, HashMap<Oid, Handle<Function>>) {
    let mut functions = Vec::new();
    let mut handles = HashMap::new();
    for row in &input.functions {
        let extension = members.get(&(MemberClass::Proc, row.oid)).copied();
        match one_function(row, input, schemas, types, extension) {
            Ok(function) => {
                handles.insert(row.oid, Handle::new(functions.len() as u32));
                functions.push(function);
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    (functions, handles)
}

/// Say whether one operator stays, and why it does not.
fn operator_note(
    row: &OperatorRow,
    schemas: &HashMap<Oid, Handle<Schema>>,
    types: &HashMap<Oid, Handle<Type>>,
    functions: &HashMap<Oid, Handle<Function>>,
) -> Option<Diagnostic> {
    if !schemas.contains_key(&row.namespace) {
        return Some(missing(
            EntityKind::Operator,
            row.oid,
            &row.name,
            EntityKind::Schema,
            row.namespace,
        ));
    }
    if operator_kind(row.oprkind).is_none() {
        return Some(unparsable(
            EntityKind::Operator,
            row.oid,
            "oprkind",
            row.oprkind,
        ));
    }
    if row.code == Oid(0) {
        return Some(Diagnostic::Skipped {
            entity: EntityKind::Operator,
            oid: row.oid,
            name: row.name.clone(),
            reason: SkipReason::ShellOperator,
        });
    }
    if !functions.contains_key(&row.code) {
        return Some(missing(
            EntityKind::Operator,
            row.oid,
            &row.name,
            EntityKind::Function,
            row.code,
        ));
    }
    // A prefix operator has no left operand, and writes 0 there.
    let operands = if row.left == Oid(0) {
        vec![row.right, row.result]
    } else {
        vec![row.left, row.right, row.result]
    };
    for referenced in operands {
        if !types.contains_key(&referenced) {
            return Some(missing(
                EntityKind::Operator,
                row.oid,
                &row.name,
                EntityKind::Type,
                referenced,
            ));
        }
    }
    None
}

/// Build every operator that the loader keeps.
///
/// The commutator and the negator of an operator are operators, so the
/// kept set comes first and the references resolve against it.
fn build_operators(
    input: &Input,
    schemas: &HashMap<Oid, Handle<Schema>>,
    types: &HashMap<Oid, Handle<Type>>,
    functions: &HashMap<Oid, Handle<Function>>,
    members: &HashMap<(MemberClass, Oid), Handle<Extension>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<Operator>, HashMap<Oid, Handle<Operator>>) {
    let mut kept = KeptSet::all(input.operators.iter().map(|row| row.oid));
    for (index, row) in input.operators.iter().enumerate() {
        if let Some(diagnostic) = operator_note(row, schemas, types, functions) {
            kept.drop_at(index);
            diagnostics.push(diagnostic);
        }
    }
    let handles: HashMap<Oid, Handle<Operator>> =
        assign_handles(&input.operators, &kept, |row| row.oid);

    let mut operators = Vec::new();
    for (index, row) in input.operators.iter().enumerate() {
        if !kept.keep[index] {
            continue;
        }
        let mut sibling = |referenced_oid: Oid| -> Option<Handle<Operator>> {
            if referenced_oid == Oid(0) {
                return None;
            }
            match handles.get(&referenced_oid) {
                Some(handle) => Some(*handle),
                None => {
                    diagnostics.push(missing(
                        EntityKind::Operator,
                        row.oid,
                        &row.name,
                        EntityKind::Operator,
                        referenced_oid,
                    ));
                    None
                }
            }
        };
        let commutator = sibling(row.commutator);
        let negator = sibling(row.negator);
        operators.push(Operator {
            oid: row.oid,
            name: row.name.clone(),
            schema: schemas[&row.namespace],
            kind: operator_kind(row.oprkind).unwrap_or(OperatorKind::Infix),
            left: types.get(&row.left).copied(),
            right: types[&row.right],
            result: types[&row.result],
            function: functions[&row.code],
            commutator,
            negator,
            extension: members.get(&(MemberClass::Operator, row.oid)).copied(),
        });
    }
    (operators, handles)
}

/// Give each type the operators of its default btree and hash operator
/// classes.
///
/// A class goes away, with a note, when its input type or one of its
/// operators is not in the catalog, and when a btree class does not have
/// each of the five strategies. The first class in OID order stays when two
/// classes are the default for one type and one access method.
fn attach_operator_classes(
    rows: &[OperatorClassRow],
    type_handles: &HashMap<Oid, Handle<Type>>,
    operator_handles: &HashMap<Oid, Handle<Operator>>,
    types: &mut [Type],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut rows: Vec<&OperatorClassRow> = rows.iter().collect();
    rows.sort_by_key(|row| (row.oid, row.strategy));

    for class in rows.chunk_by(|a, b| a.oid == b.oid) {
        let first = class[0];
        let Some(ty) = type_handles.get(&first.intype) else {
            diagnostics.push(missing(
                EntityKind::OperatorClass,
                first.oid,
                &first.name,
                EntityKind::Type,
                first.intype,
            ));
            continue;
        };

        // The operator of each strategy. Position 0 is strategy 1.
        let mut by_strategy: [Option<Handle<Operator>>; 5] = [None; 5];
        let mut complete = true;
        for row in class {
            let Some(operator) = operator_handles.get(&row.operator) else {
                diagnostics.push(missing(
                    EntityKind::OperatorClass,
                    row.oid,
                    &row.name,
                    EntityKind::Operator,
                    row.operator,
                ));
                complete = false;
                continue;
            };
            if let Some(slot) = usize::try_from(row.strategy - 1)
                .ok()
                .and_then(|index| by_strategy.get_mut(index))
            {
                *slot = Some(*operator);
            }
        }
        if !complete {
            continue;
        }

        let ty = &mut types[ty.index() as usize];
        match first.method.as_str() {
            "btree" => {
                let [Some(lt), Some(le), Some(eq), Some(ge), Some(gt)] = by_strategy else {
                    diagnostics.push(Diagnostic::Skipped {
                        entity: EntityKind::OperatorClass,
                        oid: first.oid,
                        name: first.name.clone(),
                        reason: SkipReason::IncompleteOperatorClass,
                    });
                    continue;
                };
                ty.btree
                    .get_or_insert(BtreeOperators { lt, le, eq, ge, gt });
            }
            "hash" => {
                if let Some(eq) = by_strategy[0] {
                    ty.hash_equality.get_or_insert(eq);
                }
            }
            _ => {}
        }
    }
}

/// Build one cast, or give the note that says why it went away.
///
/// A cast has no name, so a note about one carries the empty name.
fn one_cast(
    row: &CastRow,
    types: &HashMap<Oid, Handle<Type>>,
    functions: &HashMap<Oid, Handle<Function>>,
    extension: Option<Handle<Extension>>,
) -> Result<Cast, Diagnostic> {
    let nameless = Name::new("");
    let Some(context) = cast_context(row.context) else {
        return Err(unparsable(
            EntityKind::Cast,
            row.oid,
            "castcontext",
            row.context,
        ));
    };
    let method = match row.method {
        'b' => CastMethod::Binary,
        'i' => CastMethod::InOut,
        'f' => match functions.get(&row.func) {
            Some(handle) => CastMethod::Function(*handle),
            None => {
                return Err(missing(
                    EntityKind::Cast,
                    row.oid,
                    &nameless,
                    EntityKind::Function,
                    row.func,
                ));
            }
        },
        other => return Err(unparsable(EntityKind::Cast, row.oid, "castmethod", other)),
    };
    let Some(source) = types.get(&row.source) else {
        return Err(missing(
            EntityKind::Cast,
            row.oid,
            &nameless,
            EntityKind::Type,
            row.source,
        ));
    };
    let Some(target) = types.get(&row.target) else {
        return Err(missing(
            EntityKind::Cast,
            row.oid,
            &nameless,
            EntityKind::Type,
            row.target,
        ));
    };
    Ok(Cast {
        oid: row.oid,
        source: *source,
        target: *target,
        context,
        method,
        extension,
    })
}

/// The kind of a relation.
///
/// The kept set holds only the six letters of a relation, so the last arm
/// never runs.
fn relation_kind(relkind: char) -> RelationKind {
    match relkind {
        'v' => RelationKind::View,
        'm' => RelationKind::MaterializedView,
        'f' => RelationKind::ForeignTable,
        'p' => RelationKind::PartitionedTable,
        SEQUENCE_RELKIND => RelationKind::Sequence,
        _ => RelationKind::Table,
    }
}

#[cfg(test)]
mod tests {
    use super::assemble;
    use crate::cast::{CastContext, CastMethod};
    use crate::catalog::ServerVersion;
    use crate::diagnostic::{Diagnostic, EntityKind, SkipReason};
    use crate::function::{ArgMode, FunctionKind, Volatility};
    use crate::loader::queries::QueryId;
    use crate::loader::{Loaded, Rows};
    use crate::name::{Name, QualifiedName};
    use crate::oid::Oid;
    use crate::relation::RelationKind;
    use crate::types::{TypeCategory, TypeKind};
    use crate::typmod::Typmod;
    use std::collections::HashMap;

    /// The cell text that stands for a SQL NULL in a hand-written row.
    const NULL: &str = "\\N";

    /// One hand-written row.
    fn row(cells: &[&str]) -> Vec<Option<String>> {
        cells
            .iter()
            .map(|cell| {
                if *cell == NULL {
                    None
                } else {
                    Some((*cell).to_owned())
                }
            })
            .collect()
    }

    /// The rows of every query, with every query empty.
    fn empty_rows() -> HashMap<QueryId, Rows> {
        let mut rows: HashMap<QueryId, Rows> =
            QueryId::ALL.iter().map(|id| (*id, Vec::new())).collect();
        rows.insert(QueryId::Database, vec![row(&["testdb"])]);
        rows.insert(
            QueryId::SearchPath,
            vec![row(&["pg_catalog"]), row(&["public"])],
        );
        rows
    }

    /// Assemble a catalog from hand-written rows.
    ///
    /// The rows hold no built-in type, so the builder must permit those to
    /// be absent.
    fn load(rows: HashMap<QueryId, Rows>) -> Loaded {
        assemble(
            ServerVersion {
                major: 17,
                minor: 4,
            },
            &rows,
            true,
        )
        .expect("the rows assemble")
    }

    /// The rows of a small database: two schemas, one extension, four types,
    /// one table, one function, two operators, and one cast.
    fn sample_rows() -> HashMap<QueryId, Rows> {
        let mut rows = empty_rows();
        rows.insert(
            QueryId::Schemas,
            vec![
                row(&["11", "pg_catalog", "f"]),
                row(&["2200", "public", "f"]),
            ],
        );
        rows.insert(
            QueryId::Extensions,
            vec![row(&["100", "myext", "1.0", "2200"])],
        );
        rows.insert(
            QueryId::ExtensionMembers,
            vec![row(&["proc", "40000", "100"])],
        );
        rows.insert(
            QueryId::Types,
            vec![
                // bool, with its array.
                row(&[
                    "16", "bool", "11", "b", "B", "t", "f", "1000", "0", "f", "0", NULL, "0", "-1",
                    "f", NULL, NULL, NULL, "0",
                ]),
                // _bool, the array of bool.
                row(&[
                    "1000", "_bool", "11", "b", "A", "f", "f", "0", "16", "t", "0", NULL, "0",
                    "-1", "f", NULL, NULL, NULL, "0",
                ]),
                // The row type of the table.
                row(&[
                    "16385", "t1", "2200", "c", "C", "f", "f", "0", "0", "f", "16384", "r", "0",
                    "-1", "f", NULL, NULL, NULL, "0",
                ]),
                // An array whose element type is not in the catalog.
                row(&[
                    "16386", "_ghost", "2200", "b", "A", "f", "f", "0", "99999", "t", "0", NULL,
                    "0", "-1", "f", NULL, NULL, NULL, "0",
                ]),
            ],
        );
        rows.insert(
            QueryId::Relations,
            vec![row(&["16384", "t1", "2200", "r", "16385"])],
        );
        rows.insert(
            QueryId::Attributes,
            vec![
                row(&["16384", "1", "flag", "16", "-1", "t", "f", "f", "f"]),
                row(&["16384", "2", "flags", "1000", "-1", "f", "t", "f", "f"]),
            ],
        );
        rows.insert(
            QueryId::Functions,
            vec![row(&[
                "40000", "f1", "2200", "f", "0", "0", "16", "f", "t", "i",
            ])],
        );
        rows.insert(
            QueryId::FunctionArguments,
            vec![row(&["40000", "1", "16", NULL, "x"])],
        );
        rows.insert(
            QueryId::Operators,
            vec![
                row(&[
                    "50000", "===", "2200", "b", "16", "16", "16", "40000", "0", "0",
                ]),
                // A shell operator: it has no function.
                row(&["50001", "?!", "2200", "b", "16", "16", "16", "0", "0", "0"]),
            ],
        );
        rows.insert(
            QueryId::Casts,
            vec![row(&["60000", "16", "16385", "40000", "e", "f"])],
        );
        rows
    }

    #[test]
    fn it_reports_exactly_the_two_entities_that_it_could_not_keep() {
        let loaded = load(sample_rows());
        assert_eq!(
            loaded.diagnostics,
            vec![
                Diagnostic::MissingReference {
                    entity: EntityKind::Type,
                    oid: Oid(16386),
                    name: Name::new("_ghost"),
                    referenced: EntityKind::Type,
                    referenced_oid: Oid(99999),
                },
                Diagnostic::Skipped {
                    entity: EntityKind::Operator,
                    oid: Oid(50001),
                    name: Name::new("?!"),
                    reason: SkipReason::ShellOperator,
                },
            ]
        );
    }

    #[test]
    fn it_keeps_every_other_entity() {
        let loaded = load(sample_rows());
        let catalog = &loaded.catalog;
        assert_eq!(catalog.schemas().count(), 2);
        assert_eq!(catalog.extensions().count(), 1);
        assert_eq!(catalog.types().count(), 3);
        assert_eq!(catalog.relations().count(), 1);
        assert_eq!(catalog.functions().count(), 1);
        assert_eq!(catalog.operators().count(), 1);
        assert_eq!(catalog.casts().count(), 1);
    }

    #[test]
    fn it_records_the_metadata_of_the_load() {
        let loaded = load(sample_rows());
        let metadata = loaded.catalog.metadata();
        assert_eq!(
            metadata.server_version,
            ServerVersion {
                major: 17,
                minor: 4
            }
        );
        assert_eq!(metadata.database, Name::new("testdb"));
        assert_eq!(
            metadata.search_path,
            vec![Name::new("pg_catalog"), Name::new("public")]
        );
        assert!(metadata.loaded_at_unix_secs > 1_600_000_000);
    }

    #[test]
    fn a_handle_is_the_position_of_the_oid_in_order() {
        let loaded = load(sample_rows());
        let catalog = &loaded.catalog;
        let oids: Vec<Oid> = catalog.types().map(|(_, ty)| ty.oid).collect();
        assert_eq!(oids, vec![Oid(16), Oid(1000), Oid(16385)]);
        for (handle, ty) in catalog.types() {
            assert_eq!(catalog.type_by_oid(ty.oid), Some(handle));
        }
    }

    #[test]
    fn the_cycle_between_a_type_and_its_array_closes() {
        let loaded = load(sample_rows());
        let catalog = &loaded.catalog;
        let boolean = catalog.type_by_oid(Oid(16)).unwrap();
        let array = catalog.type_by_oid(Oid(1000)).unwrap();
        assert_eq!(catalog[boolean].array, Some(array));
        assert_eq!(catalog[boolean].kind, TypeKind::Base);
        assert_eq!(catalog[boolean].category, TypeCategory::Boolean);
        assert!(catalog[boolean].is_preferred);
        assert_eq!(catalog[array].array, None);
        assert_eq!(catalog[array].kind, TypeKind::Array { element: boolean });
    }

    #[test]
    fn the_cycle_between_a_composite_and_its_relation_closes() {
        let loaded = load(sample_rows());
        let catalog = &loaded.catalog;
        let boolean = catalog.type_by_oid(Oid(16)).unwrap();
        let array = catalog.type_by_oid(Oid(1000)).unwrap();
        let row_type = catalog.type_by_oid(Oid(16385)).unwrap();
        let relation = catalog.relation_by_oid(Oid(16384)).unwrap();

        let TypeKind::Composite(detail) = &catalog[row_type].kind else {
            panic!("the row type is a composite");
        };
        assert_eq!(detail.relation, Some(relation));
        assert_eq!(detail.fields.len(), 2);
        assert_eq!(detail.fields[0].name, Name::new("flag"));
        assert_eq!(detail.fields[0].ty, boolean);
        assert_eq!(detail.fields[0].typmod, Typmod::NONE);
        assert_eq!(detail.fields[1].ty, array);

        assert_eq!(catalog[relation].row_type, Some(row_type));
        assert_eq!(catalog[relation].kind, RelationKind::Table);
        let column = catalog[relation].column("flag").unwrap();
        assert_eq!(column.position, 1);
        assert_eq!(column.ty, boolean);
        assert!(column.not_null);
        assert!(!column.has_default);
        assert!(catalog[relation].column("flags").unwrap().has_default);
    }

    #[test]
    fn a_function_keeps_its_argument_and_its_extension() {
        let loaded = load(sample_rows());
        let catalog = &loaded.catalog;
        let boolean = catalog.type_by_oid(Oid(16)).unwrap();
        let extension = catalog.extension_named("myext").unwrap();
        let handle = catalog.function_by_oid(Oid(40000)).unwrap();
        let function = &catalog[handle];
        assert_eq!(function.name, Name::new("f1"));
        assert_eq!(function.kind, FunctionKind::Function);
        assert_eq!(function.volatility, Volatility::Immutable);
        assert!(function.strict);
        assert!(!function.returns_set);
        assert_eq!(function.returns, boolean);
        assert_eq!(function.variadic, None);
        assert_eq!(function.aggregate, None);
        assert_eq!(function.extension, Some(extension));
        assert_eq!(function.arguments.len(), 1);
        assert_eq!(function.arguments[0].name, Some(Name::new("x")));
        assert_eq!(function.arguments[0].ty, boolean);
        assert_eq!(function.arguments[0].mode, ArgMode::In);
        assert_eq!(
            catalog.qualified_name_of_function(handle),
            QualifiedName::new("public", "f1")
        );
    }

    #[test]
    fn an_operator_and_a_cast_point_at_the_function() {
        let loaded = load(sample_rows());
        let catalog = &loaded.catalog;
        let boolean = catalog.type_by_oid(Oid(16)).unwrap();
        let row_type = catalog.type_by_oid(Oid(16385)).unwrap();
        let function = catalog.function_by_oid(Oid(40000)).unwrap();

        let handle = catalog.operator_by_oid(Oid(50000)).unwrap();
        let operator = &catalog[handle];
        assert_eq!(operator.name, Name::new("==="));
        assert_eq!(operator.left, Some(boolean));
        assert_eq!(operator.right, boolean);
        assert_eq!(operator.result, boolean);
        assert_eq!(operator.function, function);
        assert_eq!(operator.commutator, None);
        assert_eq!(operator.negator, None);
        assert_eq!(catalog.operator_by_oid(Oid(50001)), None);

        let handle = catalog.cast(boolean, row_type).unwrap();
        let cast = &catalog[handle];
        assert_eq!(cast.context, CastContext::Explicit);
        assert_eq!(cast.method, CastMethod::Function(function));
        assert_eq!(catalog.casts_from(boolean), &[handle]);
    }

    #[test]
    fn a_missing_attribute_type_takes_the_composite_and_the_relation_with_it() {
        let mut rows = empty_rows();
        rows.insert(QueryId::Schemas, vec![row(&["2200", "public", "f"])]);
        rows.insert(
            QueryId::Types,
            vec![
                // The row type of the table. One attribute has a type that
                // the catalog does not hold.
                row(&[
                    "16385", "t1", "2200", "c", "C", "f", "f", "16386", "0", "f", "16384", "r",
                    "0", "-1", "f", NULL, NULL, NULL, "0",
                ]),
                // The array of that row type. It goes when the row type
                // goes.
                row(&[
                    "16386", "_t1", "2200", "b", "A", "f", "f", "0", "16385", "t", "0", NULL, "0",
                    "-1", "f", NULL, NULL, NULL, "0",
                ]),
            ],
        );
        rows.insert(
            QueryId::Relations,
            vec![row(&["16384", "t1", "2200", "r", "16385"])],
        );
        rows.insert(
            QueryId::Attributes,
            vec![row(&[
                "16384", "1", "ghost", "99999", "-1", "f", "f", "f", "f",
            ])],
        );

        let loaded = load(rows);
        assert_eq!(loaded.catalog.types().count(), 0);
        assert_eq!(loaded.catalog.relations().count(), 0);
        assert_eq!(
            loaded.diagnostics,
            vec![
                Diagnostic::MissingReference {
                    entity: EntityKind::Type,
                    oid: Oid(16385),
                    name: Name::new("t1"),
                    referenced: EntityKind::Type,
                    referenced_oid: Oid(99999),
                },
                Diagnostic::MissingReference {
                    entity: EntityKind::Type,
                    oid: Oid(16386),
                    name: Name::new("_t1"),
                    referenced: EntityKind::Type,
                    referenced_oid: Oid(16385),
                },
                Diagnostic::MissingReference {
                    entity: EntityKind::Relation,
                    oid: Oid(16384),
                    name: Name::new("t1"),
                    referenced: EntityKind::Type,
                    referenced_oid: Oid(16385),
                },
            ]
        );
    }

    #[test]
    fn an_unknown_typtype_is_reported_and_the_type_goes() {
        let mut rows = empty_rows();
        rows.insert(QueryId::Schemas, vec![row(&["2200", "public", "f"])]);
        rows.insert(
            QueryId::Types,
            vec![row(&[
                "16400", "odd", "2200", "q", "U", "f", "f", "0", "0", "f", "0", NULL, "0", "-1",
                "f", NULL, NULL, NULL, "0",
            ])],
        );
        let loaded = load(rows);
        assert_eq!(loaded.catalog.types().count(), 0);
        assert_eq!(
            loaded.diagnostics,
            vec![Diagnostic::UnknownTypeKind {
                oid: Oid(16400),
                name: Name::new("odd"),
                typtype: 'q',
            }]
        );
    }

    #[test]
    fn the_composite_of_a_sequence_goes_without_a_note() {
        let mut rows = empty_rows();
        rows.insert(QueryId::Schemas, vec![row(&["2200", "public", "f"])]);
        rows.insert(
            QueryId::Types,
            vec![row(&[
                "16410", "s1", "2200", "c", "C", "f", "f", "0", "0", "f", "16409", "S", "0", "-1",
                "f", NULL, NULL, NULL, "0",
            ])],
        );
        let loaded = load(rows);
        assert_eq!(loaded.catalog.types().count(), 0);
        assert_eq!(loaded.diagnostics, Vec::new());
    }

    #[test]
    fn a_sequence_is_a_relation_with_three_columns_and_no_row_type() {
        let mut rows = empty_rows();
        rows.insert(QueryId::Schemas, vec![row(&["2200", "public", "f"])]);
        rows.insert(
            QueryId::Types,
            vec![
                row(&[
                    "16", "bool", "2200", "b", "B", "t", "f", "0", "0", "f", "0", NULL, "0", "-1",
                    "f", NULL, NULL, NULL, "0",
                ]),
                row(&[
                    "20", "int8", "2200", "b", "N", "f", "f", "0", "0", "f", "0", NULL, "0", "-1",
                    "f", NULL, NULL, NULL, "0",
                ]),
            ],
        );
        // The server gives a sequence a `reltype` of 0.
        rows.insert(
            QueryId::Relations,
            vec![row(&["16409", "s1", "2200", "S", "0"])],
        );
        rows.insert(
            QueryId::Attributes,
            vec![
                row(&["16409", "1", "last_value", "20", "-1", "t", "f", "f", "f"]),
                row(&["16409", "2", "log_cnt", "20", "-1", "t", "f", "f", "f"]),
                row(&["16409", "3", "is_called", "16", "-1", "t", "f", "f", "f"]),
            ],
        );
        let loaded = load(rows);
        assert_eq!(loaded.diagnostics, Vec::new());

        let catalog = &loaded.catalog;
        let int8 = catalog.type_by_oid(Oid(20)).unwrap();
        let boolean = catalog.type_by_oid(Oid(16)).unwrap();
        let relation = catalog.relation_by_oid(Oid(16409)).unwrap();

        assert_eq!(catalog[relation].kind, RelationKind::Sequence);
        assert_eq!(catalog[relation].row_type, None);
        let columns: Vec<_> = catalog[relation]
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.ty))
            .collect();
        assert_eq!(
            columns,
            vec![
                ("last_value", int8),
                ("log_cnt", int8),
                ("is_called", boolean)
            ]
        );
    }

    #[test]
    fn the_temporary_schema_of_the_session_keeps_its_flag() {
        let mut rows = empty_rows();
        rows.insert(
            QueryId::Schemas,
            vec![
                row(&["2200", "public", "f"]),
                row(&["16500", "pg_temp_3", "t"]),
            ],
        );
        let loaded = load(rows);
        assert_eq!(loaded.diagnostics, Vec::new());

        let catalog = &loaded.catalog;
        let temporary = catalog.schema_named("pg_temp_3").unwrap();
        assert!(catalog[temporary].is_temporary);
        assert_eq!(catalog.temporary_schema(), Some(temporary));
        assert!(!catalog[catalog.schema_named("public").unwrap()].is_temporary);
    }

    /// The notes that name an operator class.
    fn operator_class_notes(loaded: &Loaded) -> Vec<Diagnostic> {
        loaded
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic,
                    Diagnostic::MissingReference {
                        entity: EntityKind::OperatorClass,
                        ..
                    } | Diagnostic::Skipped {
                        entity: EntityKind::OperatorClass,
                        ..
                    }
                )
            })
            .cloned()
            .collect()
    }

    #[test]
    fn a_type_takes_the_operators_of_its_default_operator_classes() {
        let mut rows = sample_rows();
        // The sample has one operator, so it stands for each strategy. The
        // rows are out of order, to show that the loader sorts them.
        rows.insert(
            QueryId::DefaultOperatorClasses,
            vec![
                row(&["70001", "bool_hash_ops", "hash", "16", "1", "50000"]),
                row(&["70000", "bool_ops", "btree", "16", "5", "50000"]),
                row(&["70000", "bool_ops", "btree", "16", "1", "50000"]),
                row(&["70000", "bool_ops", "btree", "16", "2", "50000"]),
                row(&["70000", "bool_ops", "btree", "16", "3", "50000"]),
                row(&["70000", "bool_ops", "btree", "16", "4", "50000"]),
            ],
        );
        let loaded = load(rows);
        assert_eq!(operator_class_notes(&loaded), Vec::new());

        let catalog = &loaded.catalog;
        let boolean = catalog.type_by_oid(Oid(16)).unwrap();
        let operator = catalog.operator_by_oid(Oid(50000)).unwrap();
        let btree = catalog.btree_ordering(boolean).unwrap();
        assert_eq!(
            [btree.lt, btree.le, btree.eq, btree.ge, btree.gt],
            [operator; 5]
        );
        assert_eq!(catalog.hash_equality(boolean), Some(operator));

        // A type that no class names has neither.
        let array = catalog.type_by_oid(Oid(1000)).unwrap();
        assert_eq!(catalog.btree_ordering(array), None);
        assert_eq!(catalog.hash_equality(array), None);
    }

    #[test]
    fn a_btree_class_without_each_strategy_goes_with_a_note() {
        let mut rows = sample_rows();
        rows.insert(
            QueryId::DefaultOperatorClasses,
            vec![
                row(&["70000", "bool_ops", "btree", "16", "1", "50000"]),
                row(&["70000", "bool_ops", "btree", "16", "3", "50000"]),
            ],
        );
        let loaded = load(rows);
        let boolean = loaded.catalog.type_by_oid(Oid(16)).unwrap();
        assert_eq!(loaded.catalog.btree_ordering(boolean), None);
        assert_eq!(
            operator_class_notes(&loaded),
            vec![Diagnostic::Skipped {
                entity: EntityKind::OperatorClass,
                oid: Oid(70000),
                name: Name::new("bool_ops"),
                reason: SkipReason::IncompleteOperatorClass,
            }]
        );
    }

    #[test]
    fn an_operator_class_with_a_missing_reference_goes_with_a_note() {
        let mut rows = sample_rows();
        rows.insert(
            QueryId::DefaultOperatorClasses,
            vec![
                // The shell operator 50001 is not in the catalog.
                row(&["70001", "bool_hash_ops", "hash", "16", "1", "50001"]),
                // The type 99999 is not in the catalog.
                row(&["70002", "ghost_ops", "hash", "99999", "1", "50000"]),
            ],
        );
        let loaded = load(rows);
        let boolean = loaded.catalog.type_by_oid(Oid(16)).unwrap();
        assert_eq!(loaded.catalog.hash_equality(boolean), None);
        assert_eq!(
            operator_class_notes(&loaded),
            vec![
                Diagnostic::MissingReference {
                    entity: EntityKind::OperatorClass,
                    oid: Oid(70001),
                    name: Name::new("bool_hash_ops"),
                    referenced: EntityKind::Operator,
                    referenced_oid: Oid(50001),
                },
                Diagnostic::MissingReference {
                    entity: EntityKind::OperatorClass,
                    oid: Oid(70002),
                    name: Name::new("ghost_ops"),
                    referenced: EntityKind::Type,
                    referenced_oid: Oid(99999),
                },
            ]
        );
    }
}
