/// CREATE VIEW and DROP VIEW statement AST.
///
/// `CREATE [OR REPLACE] [TEMP|TEMPORARY] [RECURSIVE] VIEW name [(cols)] AS query`
/// `DROP VIEW [IF EXISTS] name`
use crate::ast::ddl::table::TempKw;
use crate::ast::dml::values::Subquery;
use crate::ast::shared::flags::DropBehavior;
use crate::ast::shared::names::QualifiedName;
use crate::tokens::literal;
// ---------------------------------------------------------------------------
// Additional imports for the ALTER/DROP types appended to this file as part
// of the DDL physical-extraction migration. Glob imports keep cross-batch
// type references resolvable regardless of migration order; a polish pass
// will tighten these once the migration completes.
use crate::ast::ddl::index::{ResetReloptions, SetReloptions};
#[allow(unused_imports)]
use crate::ast::shared::expr::*;
#[allow(unused_imports)]
use crate::ast::shared::flags::*;
#[allow(unused_imports)]
use crate::ast::shared::names::*;
#[allow(unused_imports)]
use crate::ast::shared::numbers::*;
// ---------------------------------------------------------------------------

recursa::ast_node! {
    /// CREATE VIEW statement.
    /// gram.y `ViewStmt: CREATE OptTemp VIEW ... | CREATE OR REPLACE OptTemp
    /// VIEW ...`. The optional keywords are plain presences so they inline into
    /// this rule, as gram.y spells them; a presence with a fixed token attached
    /// to it lowers to its own nonterminal, which has to be reduced before the
    /// `TEMP` that `CREATE TEMP TABLE` shifts.
    ///
    /// gram.y has no access method on a view (issue #57). A `RECURSIVE` view
    /// has a column list, and its action rejects `WITH CHECK OPTION`
    /// (REL_17_11 gram.y 11206-11243; REL_14_24 gram.y `ViewStmt`), so the
    /// recursive form is its own body (issues #55 and #57).
    #[derive(Debug)]
    #[tok(CREATE, this)]
    pub struct CreateViewStmt {
        #[presence(OR, REPLACE)]
        pub or_replace: bool,
        pub temp: Option<TempKw>,
        pub body: CreateViewBody,
    }
}

recursa::ast_node! {
    /// The part of `CREATE VIEW` after `OptTemp`.
    ///
    /// Variant ordering: `Recursive` starts with `RECURSIVE`, `Plain` with
    /// `VIEW`.
    #[derive(Debug)]
    pub enum CreateViewBody {
        Recursive(RecursiveViewBody),
        Plain(PlainViewBody),
    }
}

recursa::ast_node! {
    /// `RECURSIVE VIEW name (cols) [WITH (options)] AS query`.
    #[derive(Debug)]
    pub struct RecursiveViewBody {
        #[tok(RECURSIVE, VIEW, this)]
        pub name: QualifiedName,
        pub columns: CreateViewColumnList,
        /// Optional `WITH (option [= value], ...)` view options.
        pub with_options: Option<crate::ast::ddl::index::WithStorage>,
        #[tok(AS, this)]
        pub query: Subquery,
    }
}

recursa::ast_node! {
    /// `VIEW name [(cols)] [WITH (options)] AS query [WITH CHECK OPTION]`.
    #[derive(Debug)]
    pub struct PlainViewBody {
        #[tok(VIEW, this)]
        pub name: QualifiedName,
        pub columns: Option<CreateViewColumnList>,
        /// Optional `WITH (option [= value], ...)` view options such as
        /// `security_invoker`, `security_barrier`, `check_option`.
        pub with_options: Option<crate::ast::ddl::index::WithStorage>,
        #[tok(AS, this)]
        pub query: Subquery,
        /// Optional `WITH [CASCADED|LOCAL] CHECK OPTION` trailer, used with
        /// updatable views to cascade predicate checks to underlying rows.
        pub check_option: Option<ViewCheckOption>,
    }
}

impl<'input> CreateViewStmt<'input> {
    /// The name of the view.
    pub fn name(&self) -> &QualifiedName<'input> {
        match &self.body {
            CreateViewBody::Recursive(body) => &body.name,
            CreateViewBody::Plain(body) => &body.name,
        }
    }

    /// Whether the view is `RECURSIVE`.
    pub fn recursive(&self) -> bool {
        matches!(self.body, CreateViewBody::Recursive(_))
    }

    /// The output-column list.
    pub fn columns(&self) -> Option<&CreateViewColumnList<'input>> {
        match &self.body {
            CreateViewBody::Recursive(body) => Some(&body.columns),
            CreateViewBody::Plain(body) => body.columns.as_ref(),
        }
    }

    /// The query.
    pub fn query(&self) -> &Subquery<'input> {
        match &self.body {
            CreateViewBody::Recursive(body) => &body.query,
            CreateViewBody::Plain(body) => &body.query,
        }
    }
}

recursa::ast_node! {
    /// Optional parenthesized, nonempty CREATE VIEW output-column list.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct CreateViewColumnList(
        #[sep(COMMA)]
        #[deref]
        pub one_or_many!(literal::AliasName),
    );
}

recursa::ast_node! {
    /// `WITH [CASCADED | LOCAL] CHECK OPTION` trailer on CREATE VIEW.
    #[derive(Debug)]
    #[tok(WITH, this, CHECK, OPTION)]
    pub struct ViewCheckOption {
        pub mode: Option<ViewCheckMode>,
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub enum ViewCheckMode {
        #[tok(CASCADED)]
        Cascaded,
        #[tok(LOCAL)]
        Local,
    }
}

recursa::ast_node! {
    /// DROP VIEW statement:
    ///
    /// ```sql
    /// DROP VIEW [IF EXISTS] name [, name ...] [CASCADE | RESTRICT]
    /// ```
    #[derive(Debug)]
    #[tok(DROP, VIEW, this)]
    pub struct DropViewStmt {
        #[presence(IF, EXISTS)]
        pub if_exists: bool,
        #[sep(COMMA)]
        /// gram.y `any_name_list`: one or more names.
        pub names: one_or_many!(crate::ast::shared::names::AnyName),
        pub behavior: Option<DropBehavior>,
    }
}

// =========================================================================
// ALTER/DROP VIEW — appended from simple_stmts.rs during physical extraction.
// =========================================================================

recursa::ast_node! {
    /// `ALTER [COLUMN] name SET DEFAULT expr` — Postgres' alter_table_cmd
    /// branch for setting a column default. Used by ALTER VIEW (the only
    /// alter-table-cmd subset exercised by the corpus for views).
    #[derive(Debug)]
    pub struct AlterColumnSetDefault {
        #[tok(ALTER, optional(COLUMN), this)]
        pub name: literal::Ident,
        #[tok(SET, DEFAULT, this)]
        pub expr: boxed!(Expr),
    }
}

recursa::ast_node! {
    /// `ALTER [COLUMN] name DROP DEFAULT` — Postgres' alter_table_cmd branch
    /// for dropping a column default. Used by ALTER VIEW (sister of
    /// `AlterColumnSetDefault`).
    #[derive(Debug)]
    pub struct AlterColumnDropDefault {
        #[tok(ALTER, optional(COLUMN), this, DROP, DEFAULT)]
        pub name: literal::Ident,
    }
}

recursa::ast_node! {
    /// One `ALTER COLUMN …` cmd on ALTER VIEW. Both forms start with `ALTER
    /// [COLUMN] name`; the disambiguation token after the column name is
    /// `SET`/`DROP`.
    #[derive(Debug)]
    pub enum AlterColumnViewCmd {
        SetDefault(AlterColumnSetDefault),
        DropDefault(AlterColumnDropDefault),
    }
}

recursa::ast_node! {
    /// `RENAME [COLUMN] old TO new` — Postgres' RenameStmt branch for renaming
    /// a view column. Used by ALTER VIEW / ALTER MATERIALIZED VIEW.
    #[derive(Debug)]
    pub struct RenameColumnClause {
        #[tok(RENAME, optional(COLUMN), this)]
        pub old_name: literal::Ident,
        #[tok(TO, this)]
        pub new_name: literal::Ident,
    }
}

recursa::ast_node! {
    /// One action on `ALTER VIEW [IF EXISTS] name action` — Postgres'
    /// `alter_table_cmds` (view subset: `OWNER TO`, `SET (...)`, `RESET (...)`,
    /// `ALTER COLUMN … SET/DROP DEFAULT`), plus the view branches of
    /// `RenameStmt` / `AlterObjectSchemaStmt`.
    ///
    /// Variant ordering:
    /// - `RenameColumn` (`RENAME [COLUMN] …`) before `Rename` (`RENAME TO …`)
    ///   — both start with `RENAME`; `RenameColumn` is the longer match.
    /// - `SetSchema` (`SET SCHEMA`) and `SetReloptions` (`SET (`) — disjoint
    ///   second tokens.
    /// - `AlterColumn` (`ALTER`) keyword-disjoint from `SET`/`RESET`/`RENAME`/
    ///   `OWNER`.
    #[derive(Debug)]
    pub enum AlterViewAction {
        SetSchema(SetSchemaClause),
        SetReloptions(SetReloptions),
        ResetReloptions(ResetReloptions),
        AlterColumn(AlterColumnViewCmd),
        RenameColumn(RenameColumnClause),
        Rename(RenameTo),
        Owner(OwnerTo),
    }
}

recursa::ast_node! {
    /// `ALTER VIEW [IF EXISTS] name action` — Postgres' `AlterTableStmt`
    /// branches that begin with `ALTER VIEW …`, plus the view branches of
    /// `RenameStmt` / `AlterObjectSchemaStmt` / `AlterOwnerStmt`.
    #[derive(Debug)]
    #[tok(ALTER, VIEW, this)]
    pub struct AlterViewStmt {
        pub if_exists: Option<IfExists>,
        pub name: QualifiedName,
        pub action: AlterViewAction,
    }
}
