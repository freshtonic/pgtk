//! AGGREGATE DDL statements (CREATE/ALTER/DROP).
#![allow(unused_imports)]

use crate::ast::ddl::role::DefList;
use crate::ast::shared::expr::*;
use crate::ast::shared::flags::*;
use crate::ast::shared::names::*;
use crate::ast::shared::numbers::*;
use crate::tokens::{literal, punct};

recursa::ast_node! {
    /// Argument signature for `CREATE AGGREGATE` — Postgres' `aggr_args`:
    ///
    /// ```text
    /// '(' '*' ')'
    ///   | '(' aggr_args_list ')'
    ///   | '(' ORDER BY aggr_args_list ')'
    ///   | '(' aggr_args_list ORDER BY aggr_args_list ')'
    /// ```
    ///
    /// `aggr_arg` is `func_arg` without the output modes ([`AggregateArg`]): a
    /// type, optionally with a mode keyword (`VARIADIC` / `IN`) and an
    /// argument name.
    ///
    /// Variant ordering: `Star` first (the literal `*` is unambiguous), then
    /// `OrderBy` (leading `ORDER BY`), then `BothArgs` (which has an args list
    /// followed by `ORDER BY`), then `Args` (a bare args list). The peek
    /// regex disambiguates `OrderBy` from `BothArgs`/`Args` by the leading
    /// `ORDER` keyword; `BothArgs` and `Args` are disambiguated at parse time
    /// by the presence of a trailing `ORDER BY`.
    ///
    /// `aggr_args` has no default values and no old-style `name = value`
    /// entries, so `ALTER AGGREGATE a(x = 1)` is a syntax error (issue #54).
    #[derive(Debug)]
    pub enum CreateAggregateArgs {
        #[tok(LPAREN, STAR, RPAREN)]
        Star,
        OrderBy(CreateAggregateOrderBy),
        Args(CreateAggregateArgLists),
    }
}

recursa::ast_node! {
    /// gram.y `old_aggr_definition: '(' old_aggr_list ')'`.
    #[derive(Debug)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct OldAggregateDefinition {
        #[sep(COMMA)]
        pub elems: one_or_many!(OldAggregateElem),
    }
}

recursa::ast_node! {
    /// gram.y `old_aggr_elem: IDENT '=' def_arg`.
    #[derive(Debug)]
    pub struct OldAggregateElem {
        pub name: crate::tokens::literal::IdentOnly,
        #[tok(EQ, this)]
        pub value: crate::ast::ddl::role::DefArg,
    }
}

recursa::ast_node! {
    /// `(ORDER BY aggr_args_list)` — ordered-set aggregate with no plain args.
    #[derive(Debug)]
    pub struct CreateAggregateOrderBy {
        #[tok(LPAREN, this, RPAREN)]
        pub args: CreateAggregateOrderByInner,
    }
}

recursa::ast_node! {
    /// `ORDER BY aggr_args_list` — the `ORDER BY` leads the whole list, so it is
    /// declared on the struct rather than on the repeated field.
    #[derive(Debug)]
    #[tok(ORDER, BY, this)]
    pub struct CreateAggregateOrderByInner {
        #[sep(COMMA)]
        pub args: one_or_many!(AggregateArg),
    }
}

recursa::ast_node! {
    /// One entry of `CREATE AGGREGATE`'s first parenthesized group — an
    /// `aggr_arg`, or an `old_aggr_elem` when it carries a `= value` tail.
    ///
    /// gram.y keeps the two apart in separate productions (`DefineStmt: CREATE
    /// AGGREGATE func_name aggr_args definition` versus `... func_name
    /// old_aggr_definition`), but on the surface both are one comma-separated
    /// group directly after the aggregate name, and they are told apart only by
    /// the `=` that follows the first name. Parse the shared `[mode] [name]
    /// type` prefix once and let the optional tail decide, avoiding two LR
    /// productions with an arbitrarily long shared prefix.
    #[derive(Debug)]
    pub struct CreateAggregateArg {
        /// gram.y `aggr_arg: func_arg`.
        pub arg: AggregateArg,
    }
}

recursa::ast_node! {
    /// gram.y `aggr_arg: func_arg`. Its action rejects the modes `OUT` and
    /// `INOUT` ("aggregates cannot have output arguments", REL_17_11 gram.y
    /// 8529; REL_14_24 gram.y 7742), so the modes are [`AggregateArgMode`]
    /// (issue #55). The shapes are those of
    /// [`FunctionArg`](crate::ast::ddl::function::FunctionArg).
    ///
    /// Variant ordering: `NameMode` and `Named` start with a name; the token
    /// after it decides.
    #[derive(Debug)]
    pub enum AggregateArg {
        /// `param_name arg_class func_type`.
        NameMode(AggregateNameModeArg),
        /// `[arg_class] param_name func_type`.
        Named(AggregateNamedArg),
        /// `[arg_class] func_type`.
        Unnamed(AggregateUnnamedArg),
    }
}

recursa::ast_node! {
    /// The argument modes that gram.y `aggr_arg` admits: `IN` and
    /// `VARIADIC`.
    #[derive(Debug)]
    pub enum AggregateArgMode {
        #[tok(IN)]
        In,
        #[tok(VARIADIC)]
        Variadic,
    }
}

recursa::ast_node! {
    /// `name mode type`.
    #[derive(Debug)]
    pub struct AggregateNameModeArg {
        pub name: crate::tokens::type_function_name,
        pub mode: AggregateArgMode,
        pub type_name: crate::ast::ddl::function::FuncArgType,
    }
}

recursa::ast_node! {
    /// `[mode] name type`.
    #[derive(Debug)]
    pub struct AggregateNamedArg {
        pub mode: Option<AggregateArgMode>,
        pub name: crate::tokens::type_function_name,
        pub type_name: crate::ast::ddl::function::FuncArgType,
    }
}

recursa::ast_node! {
    /// `[mode] type`.
    #[derive(Debug)]
    pub struct AggregateUnnamedArg {
        pub mode: Option<AggregateArgMode>,
        pub type_name: crate::ast::ddl::function::FuncArgType,
    }
}

recursa::ast_node! {
    /// `(aggr_args_list [ORDER BY aggr_args_list])` — regular arguments with an
    /// optional ordered-set tail, or the old-style `(name = value, ...)`
    /// definition list, which occupies the same position.
    #[derive(Debug)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct CreateAggregateArgLists {
        #[sep(COMMA)]
        pub direct: one_or_many!(CreateAggregateArg),
        pub ordered: Option<CreateAggregateOrderedTail>,
    }
}

recursa::ast_node! {
    /// `ORDER BY aggr_args_list` tail of an ordered-set aggregate signature.
    ///
    /// `ORDER BY` leads the whole list, so it is declared on the struct.
    #[derive(Debug)]
    #[tok(ORDER, BY, this)]
    pub struct CreateAggregateOrderedTail {
        #[sep(COMMA)]
        pub ordered: one_or_many!(AggregateArg),
    }
}

recursa::ast_node! {
    /// `CREATE [OR REPLACE] AGGREGATE func_name { aggr_args definition |
    /// old_aggr_definition }` — gram.y `DefineStmt` (REL_17_11 gram.y
    /// 6235-6256).
    #[derive(Debug)]
    pub struct AggregateSig {
        /// gram.y `DefineStmt: CREATE … AGGREGATE func_name …`.
        pub name: crate::ast::shared::expr::FuncCallName,
        pub form: AggregateForm,
    }
}

recursa::ast_node! {
    /// The two forms of `CREATE AGGREGATE` after the name.
    ///
    /// The old form is one `(name = value, ...)` group. The modern form is
    /// the argument signature and then the definition list. The old group
    /// takes no definition list after it, and the modern signature takes no
    /// `name = value` entry, because an `aggr_arg` has no default (issue
    /// #54).
    ///
    /// Variant ordering: both open with `(`; `name =` decides for `Old`.
    #[derive(Debug)]
    pub enum AggregateForm {
        /// gram.y `old_aggr_definition`.
        Old(OldAggregateDefinition),
        /// gram.y `aggr_args definition`.
        Modern(ModernAggregateDefinition),
    }
}

recursa::ast_node! {
    /// gram.y `aggr_args definition`: the modern `CREATE AGGREGATE` form.
    #[derive(Debug)]
    pub struct ModernAggregateDefinition {
        pub args: CreateAggregateArgs,
        pub definition: DefList,
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub struct CreateAggregateStmt {
        #[tok(CREATE, this, AGGREGATE)]
        #[presence(OR, REPLACE)]
        pub or_replace: bool,
        pub signature: AggregateSig,
    }
}

recursa::ast_node! {
    /// A single `DROP AGGREGATE` target: a qualified name plus its `(...)`
    /// argument signature — Postgres' `aggregate_with_argtypes`.
    #[derive(Debug)]
    pub struct DropAggregateTarget {
        /// gram.y `aggregate_with_argtypes: func_name aggr_args`.
        pub name: crate::ast::shared::expr::FuncCallName,
        pub args: AggregateArgs,
    }
}

recursa::ast_node! {
    /// `DROP AGGREGATE [IF EXISTS] name(args) [, ...] [CASCADE | RESTRICT]`.
    #[derive(Debug)]
    #[tok(DROP, AGGREGATE, this)]
    pub struct DropAggregateStmt {
        pub if_exists: Option<IfExists>,
        #[sep(COMMA)]
        pub targets: one_or_many!(DropAggregateTarget),
        pub behavior: Option<DropBehavior>,
    }
}

recursa::ast_node! {
    /// One action on `ALTER AGGREGATE name(args) action` — Postgres'
    /// `RenameStmt`, `AlterOwnerStmt`, and `AlterObjectSchemaStmt` branches
    /// for aggregates. Aggregates have no `[NO] DEPENDS ON EXTENSION` and no
    /// `alterfunc_opt_list` form in gram.y.
    ///
    /// Variant ordering: each variant has a distinct leading keyword
    /// (`RENAME`, `OWNER`, `SET`), so order is for clarity only.
    #[derive(Debug)]
    pub enum AlterAggregateAction {
        Rename(RenameTo),
        Owner(OwnerTo),
        SetSchema(SetSchemaClause),
    }
}

recursa::ast_node! {
    /// `ALTER AGGREGATE aggregate_with_argtypes { RENAME TO new | OWNER TO
    /// role | SET SCHEMA new }` — Postgres' `RenameStmt`, `AlterOwnerStmt`,
    /// and `AlterObjectSchemaStmt` branches for aggregates.
    ///
    /// The argument signature uses Postgres' full `aggr_args` shape (covers
    /// `(*)`, plain type lists, and the ordered-set `(... ORDER BY ...)`
    /// forms) — reuses [`CreateAggregateArgs`] from the CREATE AGGREGATE
    /// path.
    #[derive(Debug)]
    pub struct AlterAggregateStmt {
        /// gram.y `aggregate_with_argtypes: func_name aggr_args`.
        #[tok(ALTER, AGGREGATE, this)]
        pub name: crate::ast::shared::expr::FuncCallName,
        pub args: CreateAggregateArgs,
        pub action: AlterAggregateAction,
    }
}
