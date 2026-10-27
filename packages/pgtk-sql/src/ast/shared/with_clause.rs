/// WITH clause (Common Table Expressions) AST.
///
/// Supports `WITH [RECURSIVE] name [(col, ...)] AS [MATERIALIZED|NOT MATERIALIZED] (query)
///   [SEARCH DEPTH|BREADTH FIRST BY col, ... SET col]
///   [CYCLE col, ... SET col [TO val DEFAULT val] USING col]
///   [, ...] SELECT|INSERT|UPDATE|DELETE|MERGE`
use crate::ast::shared::expr::Expr;
use crate::tokens::literal;

recursa::ast_node! {
    /// Materialization option: `MATERIALIZED` or `NOT MATERIALIZED`.
    #[derive(Debug)]
    pub enum MaterializedOption {
        #[tok(NOT, MATERIALIZED)]
        NotMaterialized,
        #[tok(MATERIALIZED)]
        Materialized,
    }
}

recursa::ast_node! {
    /// Required `AS` separator between a CTE name and its optional materialization mode.
    #[derive(Debug)]
    pub enum CteAs {
        #[tok(AS)]
        Value,
    }
}

recursa::ast_node! {
    /// SEARCH direction: DEPTH or BREADTH
    #[derive(Debug)]
    pub enum SearchDirection {
        #[tok(DEPTH)]
        Depth,
        #[tok(BREADTH)]
        Breadth,
    }
}

recursa::ast_node! {
    /// `FIRST BY col, ...` list in a recursive CTE SEARCH clause.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(FIRST, BY, this)]
    pub struct SearchColumnList(
        /// gram.y `columnList`: one or more `ColId`.
        #[sep(COMMA)]
        #[deref]
        pub one_or_many!(crate::tokens::ColId),
    );
}

recursa::ast_node! {
    /// SEARCH clause: `SEARCH DEPTH|BREADTH FIRST BY col, ... SET col`
    #[derive(Debug)]
    pub struct SearchClause {
        #[tok(SEARCH, this)]
        pub direction: SearchDirection,
        pub columns: SearchColumnList,
        #[tok(SET, this)]
        pub set_column: crate::tokens::ColId,
    }
}

recursa::ast_node! {
    /// CYCLE clause: `CYCLE col, ... SET col [TO val DEFAULT val] USING col`
    #[derive(Debug)]
    #[tok(CYCLE, this)]
    pub struct CycleClause {
        #[sep(COMMA)]
        pub columns: one_or_many!(literal::AliasName),
        #[tok(SET, this)]
        pub set_column: CycleSetColumn,
        #[tok(USING, this)]
        pub using_column: literal::AliasName,
    }
}

recursa::ast_node! {
    /// SET column with optional TO/DEFAULT values.
    #[derive(Debug)]
    pub struct CycleSetColumn {
        pub name: literal::AliasName,
        pub to_default: Option<CycleToDefault>,
    }
}

recursa::ast_node! {
    /// `TO value DEFAULT value`. gram.y `opt_cycle_clause` takes an
    /// `AexprConst` for each value, in each target version (REL_14_24 to
    /// REL_19_STABLE; REL_17_11 gram.y 12944), so `TO 1 + 1` and `TO f(x)`
    /// are syntax errors (issue #87).
    #[derive(Debug)]
    pub struct CycleToDefault {
        #[tok(TO, this)]
        pub to_value: CycleMarkValue,
        #[tok(DEFAULT, this)]
        pub default_value: CycleMarkValue,
    }
}

recursa::ast_node! {
    /// gram.y `AexprConst` as the value of a `CYCLE` mark (issue #87).
    #[derive(Debug)]
    pub enum CycleMarkValue {
        /// A constant, a typed literal with a type name, `TRUE`, `FALSE` or
        /// `NULL`: `'Y'`, `1`, `int4 '1'`, `numeric(10, 2) '1'`.
        Const(crate::ast::shared::expr::AexprConst),
        /// `func_name Sconst` and `func_name '(' func_arg_list ')' Sconst`
        /// with a dotted name: `pg_catalog.int4 '1'`, `s.t(3) 'x'`.
        Qualified(boxed!(QualifiedTypedLiteral)),
        /// `func_name '(' func_arg_list ')' Sconst` with a plain name:
        /// `t(3) 'x'`.
        Call(boxed!(CallTypedLiteral)),
    }
}

recursa::ast_node! {
    /// A typed literal whose `func_name` is `ColId indirection` (REL_17_11
    /// gram.y 17223-17262). `check_func_name` refuses a `.*` and a subscript
    /// in the name, so the name is dotted attribute names only.
    #[derive(Debug)]
    pub struct QualifiedTypedLiteral {
        pub table: crate::tokens::ColId,
        pub attrs: one_or_many!(crate::ast::shared::expr::IndirectionField),
        pub value: QualifiedTypedLiteralValue,
    }
}

recursa::ast_node! {
    /// What follows the dotted name of a [`QualifiedTypedLiteral`].
    #[derive(Debug)]
    pub enum QualifiedTypedLiteralValue {
        /// `func_name Sconst`.
        String(literal::StringLit),
        /// `func_name '(' func_arg_list ')' Sconst`.
        Call(crate::ast::shared::expr::FunctionTypedLiteralTail),
    }
}

recursa::ast_node! {
    /// `func_name '(' func_arg_list ')' Sconst` with a plain name (REL_17_11
    /// gram.y 17236). The form without arguments is a `CastFunc` in
    /// [`CycleMarkValue::Const`].
    #[derive(Debug)]
    pub struct CallTypedLiteral {
        pub name: crate::tokens::type_function_name,
        pub tail: crate::ast::shared::expr::FunctionTypedLiteralTail,
    }
}

recursa::ast_node! {
    /// Optional parenthesized output-column list on a CTE definition.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct CteColumnList(
        #[sep(COMMA)]
        #[deref]
        pub one_or_many!(literal::AliasName),
    );
}

recursa::ast_node! {
    /// A single CTE definition: `name [(col, ...)] AS [MATERIALIZED|NOT MATERIALIZED] (query)
    ///   [SEARCH ...] [CYCLE ...]`
    #[derive(Debug)]
    pub struct CteDefinition {
        pub name: crate::tokens::ColId,
        pub columns: Option<CteColumnList>,
        pub r#as: CteAs,
        pub materialized: Option<MaterializedOption>,
        #[tok(LPAREN, this, RPAREN)]
        pub query: boxed!(crate::ast::tcl::prepared::PreparableStmt),
        pub search: Option<SearchClause>,
        pub cycle: Option<CycleClause>,
    }
}

recursa::ast_node! {
    /// `WITH` with its optional `RECURSIVE` modifier. Keeping the alternatives
    /// explicit lets the LR table encode PostgreSQL's shift preference: an
    /// immediate `RECURSIVE` is always the modifier, never the first CTE name.
    #[derive(Debug)]
    pub enum WithClauseHead {
        Recursive(RecursiveWith),
        #[parse(lr_conflict(
        action = shift,
        against = ast::shared::with_clause::RecursiveWith,
        lookahead = { RECURSIVE },
        expect = 1
    ))]
        Plain(crate::ast::shared::flags::AnyWith),
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub struct RecursiveWith {
        pub with: crate::ast::shared::flags::AnyWith,
        pub recursive: RecursiveKeyword,
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub enum RecursiveKeyword {
        #[tok(RECURSIVE)]
        Value,
    }
}

recursa::ast_node! {
    /// WITH clause: `WITH [RECURSIVE] cte_def, ...` — gram.y `with_clause:
    /// WITH cte_list | WITH_LA cte_list | WITH RECURSIVE cte_list`, whose
    /// `WITH_LA` twin lets the first CTE be named `time` or `ordinality`.
    #[derive(Debug)]
    pub struct WithClause {
        pub head: WithClauseHead,
        #[sep(COMMA)]
        pub ctes: one_or_many!(CteDefinition),
    }
}

impl WithClause<'_> {
    pub fn is_recursive(&self) -> bool {
        matches!(self.head, WithClauseHead::Recursive(_))
    }
}

recursa::ast_node! {
    /// WITH statement: WITH clause followed by a query-shaped body.
    #[derive(Debug)]
    #[validate(
        crate::ast::validation::with_statement,
        flat = crate::ast::validation::with_statement_flat
    )]
    pub struct WithStatement {
        pub with_clause: WithClause,
        pub body: WithBody,
    }
}
recursa::ast_node! {
    /// Query that follows a `WITH` clause. Mirrors `gram.y`: `with_clause` is
    /// followed by `select_clause` (with its set operations), `insert_rest`,
    /// `update`, `delete`, or `merge`. A second `WITH` clause is not admitted, so
    /// the `Statement` FOLLOW set no longer inherits every query continuation.
    ///
    /// Variant order: `Query` leads with `SELECT`, `VALUES`, `TABLE`, or `(`;
    /// the four DML variants have disjoint leading keywords.
    #[derive(Debug)]
    pub enum WithBody {
        /// gram.y `select_no_parens: with_clause select_clause ...`.
        Query(boxed!(crate::ast::dml::values::QueryBody)),
        Insert(boxed!(crate::ast::dml::insert::InsertStmt)),
        Update(boxed!(crate::ast::dml::update::UpdateStmt)),
        Delete(boxed!(crate::ast::dml::delete::DeleteStmt)),
        /// Added in 15: research, PostgreSQL 15, "New statements" (REL_15_19 gram.y
        /// `PreparableStmt`, `ExplainableStmt`, `MergeStmt: opt_with_clause MERGE`).
        #[config(since = pg15)]
        Merge(boxed!(crate::ast::dml::merge::MergeStmt)),
    }
}
