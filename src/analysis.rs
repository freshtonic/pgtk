//! What Analysis produces: Annotation Tables keyed by Node Handle, and the
//! Result Descriptor.

use std::collections::HashMap;
use std::hash::Hash;

use pg_catalog::{Function, Handle, Operator, Relation};

use crate::relation::{RangeTableEntry, Var};
use crate::session::SessionEffect;
use crate::types::{TypeRef, Typmod};
use crate::value::Coercion;

/// The collection of all Annotations of one kind for one Statement, keyed
/// by Node Handle.
#[derive(Clone, Debug)]
pub struct AnnotationTable<K, V> {
    entries: HashMap<K, V>,
}

impl<K: Hash + Eq + Copy, V> Default for AnnotationTable<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<K: Hash + Eq + Copy, V> AnnotationTable<K, V> {
    /// The Annotation for a node, if it has one of this kind.
    pub fn get(&self, key: K) -> Option<&V> {
        self.entries.get(&key)
    }

    /// Record an Annotation.
    pub fn insert(&mut self, key: K, value: V) {
        self.entries.insert(key, value);
    }

    /// Every Annotation of this kind.
    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        self.entries.iter().map(|(k, v)| (*k, v))
    }

    /// How many nodes carry one.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether none do.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The type an expression has, `exprType` and `exprTypmod`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExprType {
    /// The type.
    pub ty: TypeRef,
    /// The typmod, `-1` for none.
    pub typmod: Typmod,
}

/// A function call resolved to a function, `FuncExpr`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionResolution {
    /// The call has an `OVER` clause: a window function call.
    pub window: bool,
    /// The function called.
    pub function: Handle<Function>,
    /// The declared type each argument is coerced to, polymorphism resolved.
    pub declared: Vec<TypeRef>,
    /// The call returns a set.
    pub returns_set: bool,
}

/// `CollateStrength`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Strength {
    /// A non-collatable expression.
    None,
    /// A collation derived implicitly.
    Implicit,
    /// A conflict of implicit collations.
    Conflict,
    /// A collation given by a COLLATE clause.
    Explicit,
}

/// `assign_collations_context`: the collation state being merged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CollContext {
    pub collation: Option<pg_catalog::Handle<pg_catalog::Collation>>,
    pub strength: Strength,
    pub location: usize,
    /// The second collation of a conflict.
    pub collation2: Option<pg_catalog::Handle<pg_catalog::Collation>>,
    pub location2: usize,
}

impl CollContext {
    pub fn new() -> Self {
        Self {
            collation: None,
            strength: Strength::None,
            location: crate::diagnostic::NO_LOCATION,
            collation2: None,
            location2: crate::diagnostic::NO_LOCATION,
        }
    }

    /// The state of a column whose collation was derived implicitly, or
    /// of a non-collatable one.
    pub fn implicit(collation: Option<Handle<pg_catalog::Collation>>, location: usize) -> Self {
        Self {
            collation,
            strength: if collation.is_some() {
                Strength::Implicit
            } else {
                Strength::None
            },
            location,
            collation2: None,
            location2: crate::diagnostic::NO_LOCATION,
        }
    }

    /// The collation the state resolves to: none for a conflict.
    pub fn resolved(&self) -> Option<Handle<pg_catalog::Collation>> {
        if self.strength == Strength::Conflict {
            None
        } else {
            self.collation
        }
    }
}

impl Default for CollContext {
    fn default() -> Self {
        Self::new()
    }
}

/// One entry of a target list, `TargetEntry`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetEntry {
    /// The column name, `resname`.
    pub name: String,
    /// The entry's type.
    pub ty: TypeRef,
    /// The entry's typmod.
    pub typmod: Typmod,
    /// `resorigtbl` and `resorigcol`: the table column the value comes from
    /// directly, when it does.
    pub origin: Option<(Handle<Relation>, i16)>,
    /// `resjunk`: not part of the result.
    pub junk: bool,
    /// Byte offset of the expression, or of the `*` it was expanded from.
    pub location: usize,
    /// The fields of a `record` entry whose row is known.
    pub(crate) fields: Option<Vec<RecordField>>,
    /// `exprCollation` of the entry's expression, once assigned.
    pub(crate) collation: Option<Handle<pg_catalog::Collation>>,
    /// The collation state of the expression, for a set operation to
    /// merge with its strength.
    pub(crate) collation_state: CollContext,
}

/// One field of a `record` whose row is known: a row constructor, a
/// whole-row reference to a table-less entry, or OUT parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecordField {
    pub(crate) name: String,
    pub(crate) ty: TypeRef,
    pub(crate) typmod: Typmod,
}

/// One result column, in the terms a Describe message uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultColumn {
    /// The column name.
    pub name: String,
    /// The column type.
    pub ty: TypeRef,
    /// The column typmod.
    pub typmod: Typmod,
    /// The table column the value comes from directly, when it does.
    pub origin: Option<(Handle<Relation>, i16)>,
}

/// What a client would learn from a Describe message.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ResultDescriptor {
    /// The result columns in order.
    pub columns: Vec<ResultColumn>,
    /// The type of every parameter in `$n` order.
    pub params: Vec<TypeRef>,
}

/// Everything Analysis learned about one Statement, keyed by the flat AST's
/// handles. `E` keys expressions, `S` select statements, `I` target items,
/// `F` function calls in `FROM`.
#[derive(Clone, Debug)]
pub struct Analysis<E, S, I, F>
where
    E: Hash + Eq + Copy,
    S: Hash + Eq + Copy,
    I: Hash + Eq + Copy,
    F: Hash + Eq + Copy,
{
    /// `exprType`/`exprTypmod` of every expression.
    pub expr_types: AnnotationTable<E, ExprType>,
    /// Coercions applied to an expression's result, in order.
    pub coercions: AnnotationTable<E, Vec<Coercion>>,
    /// Column references resolved to `Var`s.
    pub vars: AnnotationTable<E, Var>,
    /// Operator expressions resolved to `pg_operator` rows.
    pub operators: AnnotationTable<E, Handle<Operator>>,
    /// Function calls resolved to `pg_proc` rows.
    pub functions: AnnotationTable<E, FunctionResolution>,
    /// Function calls in `FROM` resolved to `pg_proc` rows.
    pub from_functions: AnnotationTable<F, FunctionResolution>,
    /// `*` and `t.*` items expanded into target entries.
    pub star_expansions: AnnotationTable<I, Vec<TargetEntry>>,
    /// The range table of each query level.
    pub range_tables: AnnotationTable<S, Vec<RangeTableEntry>>,
    /// The target list of each query level.
    pub target_lists: AnnotationTable<S, Vec<TargetEntry>>,
    /// The Result Descriptor of the Statement.
    pub result: ResultDescriptor,
    /// The Statement is a Utility Statement: nothing in it was analysed
    /// and the Result Descriptor is empty.
    pub utility: bool,
    /// What the Statement would add to the Session or remove from it when
    /// it runs. Analysis reports it and never applies it.
    pub session_effect: Option<SessionEffect>,
}

impl<E, S, I, F> Default for Analysis<E, S, I, F>
where
    E: Hash + Eq + Copy,
    S: Hash + Eq + Copy,
    I: Hash + Eq + Copy,
    F: Hash + Eq + Copy,
{
    fn default() -> Self {
        Self {
            expr_types: AnnotationTable::default(),
            coercions: AnnotationTable::default(),
            vars: AnnotationTable::default(),
            operators: AnnotationTable::default(),
            functions: AnnotationTable::default(),
            from_functions: AnnotationTable::default(),
            star_expansions: AnnotationTable::default(),
            range_tables: AnnotationTable::default(),
            target_lists: AnnotationTable::default(),
            result: ResultDescriptor::default(),
            utility: false,
            session_effect: None,
        }
    }
}
