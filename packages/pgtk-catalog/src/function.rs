//! Functions, procedures, aggregates, and window functions.

use crate::extension::Extension;
use crate::handle::Handle;
use crate::name::Name;
use crate::oid::Oid;
use crate::schema::Schema;
use crate::types::Type;

/// A `pg_proc` row.
///
/// A function is a plain function, a procedure, an aggregate, or a window
/// function. The model keeps no body, no language, and no cost.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Function {
    /// The server's identifier for this function.
    pub oid: Oid,
    /// The name of the function.
    pub name: Name,
    /// The schema that holds this function.
    pub schema: Handle<Schema>,
    /// The kind of the function.
    pub kind: FunctionKind,
    /// The arguments, in `proallargtypes` order, else in `proargtypes` order.
    pub arguments: Vec<Argument>,
    /// The type of the variadic argument, when `provariadic` is not zero.
    pub variadic: Option<Handle<Type>>,
    /// The number of arguments that have a default, from `pronargdefaults`.
    pub default_count: u16,
    /// The type of each default expression, from `proargdefaults`.
    ///
    /// There is one entry for each default, so the length is
    /// [`default_count`](Function::default_count). The defaults belong to
    /// the last input arguments, in order: see
    /// [`default_type_of`](Function::default_type_of).
    ///
    /// An entry is the result type of the expression, which the server gets
    /// from `exprType()`. It can be different from the type of the argument.
    /// For an argument `anyelement = 'World'::text` it is `text`. For
    /// `anyelement = 'World'` it is the pseudo-type `unknown`, because the
    /// server does not coerce a default to a polymorphic type. PostgreSQL
    /// reads these types when a call leaves out a polymorphic argument.
    ///
    /// An entry is `None` when the loader cannot tell the type: the
    /// expression is a node kind that the loader does not know, or its type
    /// is not in the catalog.
    #[cfg_attr(feature = "serde", serde(default))]
    pub default_types: Vec<Option<Handle<Type>>>,
    /// The return type.
    pub returns: Handle<Type>,
    /// True when the function returns a set of rows.
    pub returns_set: bool,
    /// True when the function returns null for a null input.
    pub strict: bool,
    /// The volatility of the function.
    pub volatility: Volatility,
    /// The aggregate detail, when the function is an aggregate.
    pub aggregate: Option<AggregateDetail>,
    /// The extension that owns this function, when one does.
    pub extension: Option<Handle<Extension>>,
}

impl Function {
    /// The type of the default expression of one input argument.
    ///
    /// The index counts the input arguments, as
    /// [`input_arguments`](Function::input_arguments) gives them. The result
    /// is `None` when that argument has no default, and when the loader
    /// cannot tell the type of the default.
    pub fn default_type_of(&self, input_index: usize) -> Option<Handle<Type>> {
        let inputs = self.input_arguments().count();
        let first_default = inputs.checked_sub(self.default_types.len())?;
        let offset = input_index.checked_sub(first_default)?;
        self.default_types.get(offset).copied().flatten()
    }

    /// The arguments with mode `In`, `InOut`, or `Variadic`, in order.
    ///
    /// These are the arguments that a call site supplies.
    pub fn input_arguments(&self) -> impl Iterator<Item = &Argument> {
        self.arguments
            .iter()
            .filter(|a| matches!(a.mode, ArgMode::In | ArgMode::InOut | ArgMode::Variadic))
    }
}

/// The kind of a function, from `prokind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum FunctionKind {
    /// A plain function. `prokind` is `f`.
    Function,
    /// A procedure. `prokind` is `p`.
    Procedure,
    /// An aggregate. `prokind` is `a`.
    Aggregate,
    /// A window function. `prokind` is `w`.
    Window,
}

/// One argument of a function.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Argument {
    /// The name of the argument, when it has one.
    pub name: Option<Name>,
    /// The type of the argument.
    pub ty: Handle<Type>,
    /// The mode of the argument.
    pub mode: ArgMode,
}

/// The mode of an argument, from `proargmodes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ArgMode {
    /// An input argument. The letter is `i`. A missing mode means this.
    In,
    /// An output argument. The letter is `o`.
    Out,
    /// An argument that is both input and output. The letter is `b`.
    InOut,
    /// The variadic argument. The letter is `v`.
    Variadic,
    /// A column of the returned table. The letter is `t`.
    Table,
}

/// How the planner may treat a function, from `provolatile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Volatility {
    /// The result never changes for the same arguments. The letter is `i`.
    Immutable,
    /// The result is stable inside one statement. The letter is `s`.
    Stable,
    /// The result may change at any call. The letter is `v`.
    Volatile,
}

/// The extra detail of an aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AggregateDetail {
    /// The kind of the aggregate.
    pub kind: AggregateKind,
    /// The number of direct arguments, from `aggnumdirectargs`.
    pub direct_args: u16,
}

/// The kind of an aggregate, from `aggkind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum AggregateKind {
    /// An ordinary aggregate. The letter is `n`.
    Normal,
    /// An ordered-set aggregate. The letter is `o`.
    OrderedSet,
    /// A hypothetical-set aggregate. The letter is `h`.
    HypotheticalSet,
}
