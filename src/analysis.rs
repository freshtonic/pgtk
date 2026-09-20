//! What Analysis produces: Annotation Tables keyed by Node Handle, and the
//! Result Descriptor.

use std::collections::HashMap;
use std::hash::Hash;

use pg_catalog::{Function, Handle, Operator, Relation};

use crate::relation::{RangeTableEntry, Var};
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
    /// The function called.
    pub function: Handle<Function>,
    /// The declared type each argument is coerced to, polymorphism resolved.
    pub declared: Vec<TypeRef>,
    /// The call returns a set.
    pub returns_set: bool,
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
/// handles. `E` keys expressions, `S` select statements, `I` target items.
#[derive(Clone, Debug)]
pub struct Analysis<E, S, I>
where
    E: Hash + Eq + Copy,
    S: Hash + Eq + Copy,
    I: Hash + Eq + Copy,
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
    /// `*` and `t.*` items expanded into target entries.
    pub star_expansions: AnnotationTable<I, Vec<TargetEntry>>,
    /// The range table of each query level.
    pub range_tables: AnnotationTable<S, Vec<RangeTableEntry>>,
    /// The target list of each query level.
    pub target_lists: AnnotationTable<S, Vec<TargetEntry>>,
    /// The Result Descriptor of the Statement.
    pub result: ResultDescriptor,
}

impl<E, S, I> Default for Analysis<E, S, I>
where
    E: Hash + Eq + Copy,
    S: Hash + Eq + Copy,
    I: Hash + Eq + Copy,
{
    fn default() -> Self {
        Self {
            expr_types: AnnotationTable::default(),
            coercions: AnnotationTable::default(),
            vars: AnnotationTable::default(),
            operators: AnnotationTable::default(),
            functions: AnnotationTable::default(),
            star_expansions: AnnotationTable::default(),
            range_tables: AnnotationTable::default(),
            target_lists: AnnotationTable::default(),
            result: ResultDescriptor::default(),
        }
    }
}
