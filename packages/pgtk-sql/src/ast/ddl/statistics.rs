//! STATISTICS DDL statements (CREATE/ALTER/DROP).
#![allow(unused_imports)]

use crate::ast::shared::expr::*;
use crate::ast::shared::flags::*;
use crate::ast::shared::names::*;
use crate::ast::shared::numbers::*;
use crate::tokens::{literal, punct};

recursa::ast_node! {
    /// A single `stats_param`: a parenthesised expression, a bare column
    /// reference, or a window-less function expression.
    ///
    /// Postgres' grammar:
    ///
    /// ```text
    /// stats_param: ColId
    ///            | func_expr_windowless
    ///            | '(' a_expr ')'
    /// ```
    ///
    /// `Paren` starts with `(`. `Func` and `Bare` both begin with an identifier,
    /// and the LR parser distinguishes them when it reaches the function-call
    /// `(`. We
    /// model `func_expr_windowless` by re-using `Expr` and letting any
    /// expression that begins like a function call lex into the `Func` arm.
    ///
    /// `Bare` is a `ColId`, as gram.y writes it. A wider `NonReservedWord`
    /// there put a second word nonterminal beside the function name's `ColId`
    /// in one state, which is an `RCA0401` reduce/reduce conflict on `(`.
    #[derive(Debug)]
    pub enum StatsParam {
        Paren(#[tok(LPAREN, this, RPAREN)] boxed!(Expr)),
        Func(StatsFuncParam),
        Bare(crate::tokens::ColId),
    }
}

recursa::ast_node! {
    /// A bare function call as a `stats_param` — `ident '(' args ')'`. The
    /// argument list is captured as a raw `Expr` so any of PG's `func_expr_*`
    /// shapes round-trip.
    #[derive(Debug)]
    pub struct StatsFuncParam {
        pub name: crate::ast::shared::names::AnyName,
        pub args: StatsFuncArgs,
    }
}

recursa::ast_node! {
    /// Required parentheses around a possibly empty statistics function argument
    /// list. Keeping the delimiters on this owning node prevents the nullable
    /// vector from making the whole suffix disappear.
    #[derive(Debug)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct StatsFuncArgs {
        #[sep(COMMA)]
        pub args: zero_or_many!(boxed!(Expr)),
    }
}

recursa::ast_node! {
    /// `ON stats_param (, stats_param)*` clause on `CREATE STATISTICS`.
    #[derive(Debug)]
    #[tok(ON, this)]
    pub struct StatisticsOnClause {
        #[sep(COMMA)]
        pub params: one_or_many!(StatsParam),
    }
}

recursa::ast_node! {
    /// `FROM table_ref (, table_ref)*` clause on `CREATE STATISTICS`. Re-uses
    /// `select::TableRef` so the full `from_list` grammar (JOIN, subquery,
    /// TABLESAMPLE, function table, XMLTABLE, JSON_TABLE) is accepted — gram.y's
    /// CreateStatsStmt rule explicitly uses `from_list`, even though PG
    /// semantically rejects most of these.
    #[derive(Debug)]
    #[tok(FROM, this)]
    pub struct StatisticsFromClause {
        #[sep(COMMA)]
        pub tables: one_or_many!(crate::ast::dml::select::TableRef),
    }
}

recursa::ast_node! {
    /// Optional parenthesized extended-statistics kind list.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct StatisticsTypeList(
        #[sep(COMMA)]
        #[deref]
        pub one_or_many!(literal::AliasName),
    );
}

recursa::ast_node! {
    /// `CREATE STATISTICS [IF NOT EXISTS] [name] [(stat_type, ...)]
    /// ON stats_params FROM from_list`.
    ///
    /// Both trailers are mandatory: every arm of gram.y `CreateStatsStmt` ends
    /// with `ON stats_params FROM from_list`, so a statement without them is a
    /// raw-parser error in every target version (REL_14_24 gram.y
    /// `CreateStatsStmt`; b73d13c gram.y `CreateStatsStmt`).
    #[derive(Debug)]
    #[tok(CREATE, STATISTICS, this)]
    pub struct CreateStatisticsStmt {
        // The name is optional from 16: research, PostgreSQL 16, "Changes to
        // existing statements" (commit 624aa2a13). REL_15_19 gram.y has
        // `CREATE STATISTICS any_name`.
        //
        // A shape rule, so the requirement belongs to the absence of the name
        // (ADR 0010). REL_15_19 raises no ereport for it: the rule has no
        // alternative without the name, so the parser gives a plain syntax
        // error and the gate carries no message.
        #[config(
            since = pg16,
            on = absent,
            cite = "gram.y REL_15_19:4423-4425 `CreateStatsStmt: CREATE STATISTICS any_name \
                    opt_name_list ON stats_params FROM from_list`; optional from 16, \
                    commit 624aa2a13"
        )]
        pub name: Option<StatisticsName>,
        #[config(before = pg16)]
        pub name: StatisticsName,
        pub stat_types: Option<StatisticsTypeList>,
        pub on: StatisticsOnClause,
        pub from: StatisticsFromClause,
    }
}

recursa::ast_node! {
    /// The name of `CREATE STATISTICS`, with `IF NOT EXISTS` before it. gram.y
    /// `CreateStatsStmt: CREATE STATISTICS IF_P NOT EXISTS any_name ...` has a
    /// name in every target version (REL_17_11 gram.y 4607; REL_14_24 gram.y
    /// `CreateStatsStmt`), so `IF NOT EXISTS` goes with the name (issue #57).
    ///
    /// Variant ordering: `IfNotExists` leads with `IF`, which is also a
    /// `ColId`; the token after `IF` parts the two forms.
    #[derive(Debug)]
    pub enum StatisticsName {
        IfNotExists(StatisticsIfNotExistsName),
        Plain(crate::ast::shared::names::AnyName),
    }
}

recursa::ast_node! {
    /// `IF NOT EXISTS any_name`.
    #[derive(Debug)]
    pub struct StatisticsIfNotExistsName {
        #[tok(IF, NOT, EXISTS, this)]
        pub name: crate::ast::shared::names::AnyName,
    }
}

impl<'input> CreateStatisticsStmt<'input> {
    /// Whether the statement has `IF NOT EXISTS`.
    pub fn if_not_exists(&self) -> bool {
        #[cfg(feature = "since-pg16")]
        let name = self.name.as_ref();
        #[cfg(not(feature = "since-pg16"))]
        let name = Some(&self.name);
        matches!(name, Some(StatisticsName::IfNotExists(_)))
    }
}

recursa::ast_node! {
    /// `DROP STATISTICS [IF EXISTS] name [, ...] [CASCADE | RESTRICT]`.
    #[derive(Debug)]
    #[tok(DROP, STATISTICS, this)]
    pub struct DropStatisticsStmt {
        pub if_exists: Option<IfExists>,
        pub names: NameList,
        pub behavior: Option<DropBehavior>,
    }
}

recursa::ast_node! {
    /// `SET STATISTICS set_statistics_value` — Postgres' `AlterStatsStmt`
    /// body. The value is either a (signed) integer literal or `DEFAULT`.
    ///
    /// Variant ordering: `DEFAULT` is a hard keyword (distinct first token
    /// from a numeric literal), so order is for clarity.
    #[derive(Debug)]
    pub enum SetStatisticsValue {
        // Added in 17: research, PostgreSQL 17, "Changes to existing statements"
        // (REL_17_11 gram.y 3093 `set_statistics_value`). REL_16_15 gram.y 2426
        // takes only `SignedIconst`.
        #[config(since = pg17)]
        #[tok(DEFAULT)]
        Default,
        Value(SignedIconst),
    }
}

recursa::ast_node! {
    /// `SET STATISTICS value` clause on `ALTER STATISTICS`.
    #[derive(Debug)]
    pub struct AlterStatisticsSetStatisticsClause {
        #[tok(SET, STATISTICS, this)]
        pub value: SetStatisticsValue,
    }
}

recursa::ast_node! {
    /// One action on `ALTER STATISTICS any_name action` — covers
    /// Postgres' `RenameStmt`, `AlterOwnerStmt`, `AlterObjectSchemaStmt`,
    /// and `AlterStatsStmt` branches for extended statistics.
    ///
    /// Variant ordering: variants beginning with unique keywords
    /// (`RENAME`, `OWNER`) come first. The two `SET ...` variants share the
    /// `SET` token; `SET SCHEMA` (followed by a keyword) and `SET
    /// STATISTICS` (followed by a keyword) disambiguate on the second
    /// token. `SET STATISTICS` is listed before `SET SCHEMA` only for
    /// readability — both have a unique two-token prefix.
    #[derive(Debug)]
    pub enum AlterStatisticsAction {
        Rename(RenameTo),
        Owner(OwnerTo),
        SetStatistics(AlterStatisticsSetStatisticsClause),
        SetSchema(SetSchemaClause),
    }
}

recursa::ast_node! {
    /// `ALTER STATISTICS [IF EXISTS] any_name action` — Postgres'
    /// `AlterStatsStmt` plus the statistics branches of `RenameStmt` /
    /// `AlterOwnerStmt` / `AlterObjectSchemaStmt`.
    ///
    /// gram.y has `IF EXISTS` only on the `SET STATISTICS` form (REL_17_11
    /// gram.y 4662-4680, 9791, 10085, 10434), so the `IF EXISTS` form has no
    /// other action (issue #58).
    #[derive(Debug)]
    #[tok(ALTER, STATISTICS, this)]
    pub struct AlterStatisticsStmt {
        pub form: AlterStatisticsForm,
    }
}

recursa::ast_node! {
    /// The two forms of `ALTER STATISTICS`.
    ///
    /// Variant ordering: `IfExists` leads with `IF`, which is also a
    /// `ColId`; the token after `IF` parts the two forms.
    #[derive(Debug)]
    pub enum AlterStatisticsForm {
        IfExists(AlterStatisticsIfExists),
        Plain(AlterStatisticsPlain),
    }
}

recursa::ast_node! {
    /// `IF EXISTS any_name SET STATISTICS value`.
    #[derive(Debug)]
    pub struct AlterStatisticsIfExists {
        #[tok(IF, EXISTS, this)]
        pub name: crate::ast::shared::names::AnyName,
        pub set_statistics: AlterStatisticsSetStatisticsClause,
    }
}

recursa::ast_node! {
    /// `any_name action`.
    #[derive(Debug)]
    pub struct AlterStatisticsPlain {
        pub name: crate::ast::shared::names::AnyName,
        pub action: AlterStatisticsAction,
    }
}
