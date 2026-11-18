/// CREATE INDEX / DROP INDEX statement AST.
pub use crate::ast::shared::flags::{DropBehavior, IfExists, IfNotExists};

use crate::ast::dml::select::{NullsOrder, SortDir, WhereClause};
use crate::ast::session::set_reset::SetValue;
use crate::ast::shared::expr::{Expr, FunctionApplicationExpr};
use crate::tokens::literal;

// ---------------------------------------------------------------------------
// Additional imports for the ALTER/DROP types appended to this file as part
// of the DDL physical-extraction migration. Glob imports keep cross-batch
// type references resolvable regardless of migration order; a polish pass
// will tighten these once the migration completes.
use crate::ast::ddl::database::SetTablespaceClause;
use crate::ast::ddl::statistics::SetStatisticsValue;
use crate::ast::ddl::trigger::DependsOnExtension;
#[allow(unused_imports)]
use crate::ast::shared::expr::*;
#[allow(unused_imports)]
use crate::ast::shared::flags::*;
#[allow(unused_imports)]
use crate::ast::shared::names::*;
#[allow(unused_imports)]
use crate::ast::shared::numbers::*;
recursa::ast_node! {
    #[allow(unused_imports)]
    // ---------------------------------------------------------------------------
    /// Index access method: `USING method_name`.
    ///
    /// The method name can be an identifier or one of the built-in method
    /// keywords (`btree`, `gin`, ...). We accept `literal::AliasName` so both
    /// identifiers and keywords are allowed in this position.
    #[derive(Debug)]
    pub struct UsingMethod {
        #[tok(USING, this)]
        pub method: literal::AliasName,
    }
}

recursa::ast_node! {
    /// A single opclass option: `name = value`, gram.y `reloption_elem:
    /// ColLabel '=' def_arg` (REL_17_11 3041). The value is a `def_arg`, not
    /// an expression, so `(siglen = ARRAY[])` is a syntax error (issue #59).
    #[derive(Debug)]
    pub struct OpclassOption {
        pub name: literal::AliasName,
        #[tok(EQ, this)]
        pub value: crate::ast::ddl::role::DefArg,
    }
}

recursa::ast_node! {
    /// Parenthesized opclass option list: `(name = value, ...)`. gram.y:3026
    /// `reloptions: '(' reloption_list ')'`, and the list is never empty.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct OpclassOptions(
        #[sep(COMMA)]
        #[deref]
        pub one_or_many!(OpclassOption),
    );
}

recursa::ast_node! {
    /// Opclass name plus optional options: `int4_ops [(opt = val, ...)]`.
    #[derive(Debug)]
    pub struct OpclassSpec {
        /// gram.y `opt_qualified_name`: an `any_name`, so `s.text_ops`.
        pub name: crate::ast::shared::names::AnyName,
        pub options: Option<OpclassOptions>,
    }
}

recursa::ast_node! {
    /// A storage parameter entry: `name [= value]`.
    #[derive(Debug)]
    pub struct StorageParam {
        pub name: StorageParamName,
        pub value: Option<StorageParamValue>,
    }
}

recursa::ast_node! {
    /// Storage parameter name: either a bare word or `namespace.word`.
    ///
    /// Parsing the first word unconditionally keeps the optional suffix's first
    /// token (`.`) disjoint from the first word. In particular, a bare name does
    /// not speculatively commit to the qualified form and then require a dot.
    #[derive(Debug)]
    pub struct StorageParamName {
        pub head: literal::AliasName,
        pub qualified_tail: Option<StorageParamQualifiedTail>,
    }
}

recursa::ast_node! {
    /// `.name` suffix on a qualified storage parameter name.
    #[derive(Debug)]
    pub struct StorageParamQualifiedTail {
        #[tok(DOT, this)]
        pub name: literal::AliasName,
    }
}

impl<'input> StorageParamName<'input> {
    /// Namespace of a qualified name, or `None` for a bare name.
    pub fn namespace(&self) -> Option<&literal::AliasName<'input>> {
        self.qualified_tail.as_ref().map(|_| &self.head)
    }

    /// Unqualified name component.
    pub fn name(&self) -> &literal::AliasName<'input> {
        self.qualified_tail
            .as_ref()
            .map_or(&self.head, |tail| &tail.name)
    }
}

recursa::ast_node! {
    /// `= value` suffix for a storage parameter.
    ///
    /// The value is a permissive SetValue (keywords like `off`, `on`, string/numeric
    /// literals, identifiers) rather than a full `Expr` — storage param values are
    /// simple literals and `Expr::ColumnRef` rejects keywords like `off`.
    #[derive(Debug)]
    pub struct StorageParamValue {
        #[tok(EQ, this)]
        pub value: SetValue,
    }
}

recursa::ast_node! {
    /// `WITH (name = value, ...)` storage parameters clause.
    #[derive(Debug)]
    #[tok(WITH, LPAREN, this, RPAREN)]
    pub struct WithStorage {
        #[sep(COMMA)]
        pub params: one_or_many!(StorageParam),
    }
}

recursa::ast_node! {
    /// `INCLUDE (col, ...)` covering-index clause.
    #[derive(Debug)]
    #[tok(INCLUDE, LPAREN, this, RPAREN)]
    pub struct IncludeClause {
        #[sep(COMMA)]
        pub columns: one_or_many!(crate::tokens::ColId),
    }
}

recursa::ast_node! {
    /// Index column target, gram.y:8199 `index_elem: ColId | func_expr_windowless
    /// | '(' a_expr ')'`: a plain column, a parenthesized expression, or a
    /// function-like form with no parentheses around it. `Special` and `Func`
    /// are the two halves of `func_expr_windowless`.
    ///
    /// Variant ordering: `Expr` leads with `(`. `Special`, `Func` and `Col`
    /// can lead with the same word and part on the `(` after it.
    #[derive(Debug)]
    pub enum IndexTarget {
        Expr(#[tok(LPAREN, this, RPAREN)] boxed!(Expr)),
        /// gram.y `func_expr_windowless`'s `func_expr_common_subexpr` half,
        /// as in `ON CONFLICT (coalesce(key, 0))`: `COALESCE`, `CAST`, `TRIM`,
        /// the XML and SQL/JSON functions and the rest. It holds an [`Expr`].
        /// Their words are keywords that `Func` never takes as a name, and a
        /// bare `coalesce` is still `Col`.
        Special(boxed!(crate::ast::shared::expr::FuncExprCommonSubexpr)),
        /// gram.y `func_expr_windowless`: no `WITHIN GROUP`, `FILTER` or `OVER`
        /// suffix, which are also operator class names after the call.
        Func(boxed!(FunctionApplicationExpr)),
        Col(crate::tokens::ColId),
    }
}

recursa::ast_node! {
    /// `COLLATE "name"` on an index element.
    #[derive(Debug)]
    pub struct IndexCollate {
        /// gram.y:8231 `opt_collate: COLLATE any_name`.
        #[tok(COLLATE, this)]
        pub name: crate::ast::shared::names::AnyName,
    }
}

recursa::ast_node! {
    /// An index element:
    /// `column_or_expr [COLLATE "name"] [opclass [(options)]] [ASC|DESC] [NULLS FIRST|LAST]`.
    #[derive(Debug)]
    pub struct IndexElem {
        pub target: IndexTarget,
        pub collate: Option<IndexCollate>,
        pub opclass: Option<OpclassSpec>,
        pub dir: Option<SortDir>,
        pub nulls: Option<NullsOrder>,
    }
}

recursa::ast_node! {
    /// Parenthesized, comma-separated index-element list: gram.y
    /// `'(' index_params ')'`, which is not empty (issue #56). The delimiters
    /// wrap the whole list rather than each element.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct IndexElementList(
        #[sep(COMMA)]
        #[deref]
        pub one_or_many!(IndexElem),
    );
}

recursa::ast_node! {
    /// CREATE INDEX statement.
    ///
    /// ```sql
    /// CREATE [UNIQUE] INDEX [CONCURRENTLY] [IF NOT EXISTS] [name]
    ///        ON table [USING method] (index_elem, ...)
    ///        [INCLUDE (col, ...)]
    ///        [WITH (storage_param = value, ...)]
    ///        [WHERE predicate]
    /// ```
    ///
    /// The index name is optional (Postgres allows it to be omitted).
    #[derive(Debug)]
    pub struct CreateIndexStmt {
        #[tok(CREATE, this, INDEX)]
        #[presence(UNIQUE)]
        pub unique: bool,
        #[presence(CONCURRENTLY)]
        pub concurrently: bool,
        /// gram.y `opt_single_name`, or `IF NOT EXISTS name`: `IF NOT
        /// EXISTS` takes a name (REL_17_11 gram.y 8093-8126; REL_14_24 gram.y
        /// 7310-7340; issue #57).
        pub name: Option<IndexName>,
        /// gram.y `ON relation_expr`. `ONLY` restricts the index to the named
        /// table without descending into its partitions.
        #[tok(ON, this)]
        pub relation: crate::ast::shared::names::RelationExpr,
        pub using: Option<boxed!(UsingMethod)>,
        pub columns: IndexElementList,
        pub include: Option<boxed!(IncludeClause)>,
        /// Added in 15: research, PostgreSQL 15, "Changes to existing statements"
        /// (REL_15_19 gram.y `IndexStmt ... opt_unique_null_treatment`).
        #[config(since = pg15)]
        pub nulls_distinct: Option<NullsDistinctClause>,
        pub with_storage: Option<boxed!(WithStorage)>,
        pub tablespace: Option<crate::ast::ddl::table::TablespaceClause>,
        pub where_clause: Option<boxed!(WhereClause)>,
    }
}

recursa::ast_node! {
    /// The name of `CREATE INDEX`, with `IF NOT EXISTS` before it.
    ///
    /// Variant ordering: `IfNotExists` leads with `IF`, which is also a
    /// `ColId`; the token after `IF` parts the two forms.
    #[derive(Debug)]
    pub enum IndexName {
        IfNotExists(IndexIfNotExistsName),
        Plain(crate::tokens::ColId),
    }
}

recursa::ast_node! {
    /// `IF NOT EXISTS name`.
    #[derive(Debug)]
    pub struct IndexIfNotExistsName {
        #[tok(IF, NOT, EXISTS, this)]
        pub name: crate::tokens::ColId,
    }
}

impl<'input> CreateIndexStmt<'input> {
    /// Whether the statement has `IF NOT EXISTS`.
    pub fn if_not_exists(&self) -> bool {
        matches!(self.name, Some(IndexName::IfNotExists(_)))
    }

    /// The index name, when written.
    pub fn name(&self) -> Option<&crate::tokens::ColId<'input>> {
        match self.name.as_ref()? {
            IndexName::IfNotExists(name) => Some(&name.name),
            IndexName::Plain(name) => Some(name),
        }
    }
}

// Added in 15: research, PostgreSQL 15, "Changes to existing statements"
// (REL_15_19 gram.y `opt_unique_null_treatment`).
#[cfg(feature = "since-pg15")]
recursa::ast_node! {
    /// `NULLS [NOT] DISTINCT` modifier on a unique index.
    ///
    /// Variant ordering: `NotDistinct` (`NULLS NOT DISTINCT`, longer) before
    /// `Distinct` (`NULLS DISTINCT`, shorter).
    #[derive(Debug)]
    pub enum NullsDistinctClause {
        #[tok(NULLS, NOT, DISTINCT)]
        NotDistinct,
        #[tok(NULLS, DISTINCT)]
        Distinct,
    }
}

recursa::ast_node! {
    /// DROP INDEX statement:
    ///
    /// ```sql
    /// DROP INDEX [CONCURRENTLY] [IF EXISTS] name [, name ...] [CASCADE | RESTRICT]
    /// ```
    #[derive(Debug)]
    #[tok(DROP, INDEX, this)]
    pub struct DropIndexStmt {
        #[presence(CONCURRENTLY)]
        pub concurrently: bool,
        pub if_exists: Option<IfExists>,
        #[sep(COMMA)]
        /// gram.y `any_name_list`: one or more names.
        pub names: one_or_many!(crate::ast::shared::names::AnyName),
        pub behavior: Option<DropBehavior>,
    }
}

// =========================================================================
// ALTER/DROP INDEX — appended from simple_stmts.rs during physical extraction.
// =========================================================================

recursa::ast_node! {
    /// `SET (storage_param = value, ...)` action shared by ALTER INDEX /
    /// ALTER VIEW / ALTER MATERIALIZED VIEW / ALTER TABLE — modifies storage
    /// parameters. Differs from `WithStorage` (`WITH (...)` on CREATE) only in
    /// the leading keyword.
    #[derive(Debug)]
    #[tok(SET, LPAREN, this, RPAREN)]
    pub struct SetReloptions {
        #[sep(COMMA)]
        pub params: one_or_many!(crate::ast::ddl::index::StorageParam),
    }
}

recursa::ast_node! {
    /// `RESET (param_name [= value], ...)` action shared by ALTER INDEX /
    /// ALTER VIEW / ALTER MATERIALIZED VIEW / ALTER TABLE — removes storage
    /// parameters. Postgres' gram.y `reloption_elem` allows
    /// `ColLabel [. ColLabel] [= def_arg]`, so the syntax accepts `name = value`
    /// in RESET too (PG ignores the value semantically). Modeled via the same
    /// `StorageParam` type used by `WITH (...)`.
    #[derive(Debug)]
    #[tok(RESET, LPAREN, this, RPAREN)]
    pub struct ResetReloptions {
        #[sep(COMMA)]
        pub params: one_or_many!(crate::ast::ddl::index::StorageParam),
    }
}

recursa::ast_node! {
    /// `ATTACH PARTITION qualified_name` — Postgres' `index_partition_cmd` (the
    /// single ALTER INDEX form that takes a partition operation).
    #[derive(Debug)]
    pub struct AttachPartitionClause {
        #[tok(ATTACH, PARTITION, this)]
        pub name: QualifiedName,
    }
}

recursa::ast_node! {
    /// The `Iconst` column number of gram.y `alter_table_cmd: ALTER
    /// opt_column Iconst SET STATISTICS set_statistics_value`. The number has
    /// no sign, and the action rejects a number below 1 or above 32767
    /// (REL_17_11 gram.y 2487; REL_14_24 gram.y 2271). The error is at the
    /// number (`parser_errposition(@3)`).
    #[classify(value, fold = crate::tokens::literal::column_number_class)]
    #[reject(
        one_of(value, "out_of_range"),
        code = "22023",
        message = "column number must be in range from 1 to 32767",
        cite = "REL_17_11 gram.y 2487, alter_table_cmd",
    )]
    #[derive(Debug, derive_more :: Deref)]
    pub struct ColumnNumber {
        #[deref]
        pub value: literal::IntegerLit,
    }
}

recursa::ast_node! {
    /// `SET STATISTICS …` tail of an ALTER COLUMN command.
    #[derive(Debug)]
    pub struct AlterColumnStatisticsAction {
        #[tok(SET, STATISTICS, this)]
        pub value: SetStatisticsValue,
    }
}

recursa::ast_node! {
    /// Action following the shared `ALTER [COLUMN] col_ref` prefix.
    #[derive(Debug)]
    pub enum AlterColumnIndexAction {
        Statistics(AlterColumnStatisticsAction),
        Reloptions(SetReloptions),
    }
}

recursa::ast_node! {
    /// One `ALTER [COLUMN] …` cmd on ALTER INDEX. A column name takes `SET
    /// STATISTICS`, `SET (params)` or `RESET (params)`; a column number takes
    /// only `SET STATISTICS` (REL_17_11 gram.y 2470-2515).
    #[derive(Debug)]
    pub struct AlterColumnIndexCmd {
        #[tok(ALTER, optional(COLUMN), this)]
        pub target: AlterColumnIndexTarget,
    }
}

recursa::ast_node! {
    /// The column of an [`AlterColumnIndexCmd`], with its action.
    #[derive(Debug)]
    pub enum AlterColumnIndexTarget {
        Number(AlterColumnNumberStatistics),
        Name(AlterColumnIndexName),
    }
}

recursa::ast_node! {
    /// `Iconst SET STATISTICS value`.
    #[derive(Debug)]
    pub struct AlterColumnNumberStatistics {
        pub number: ColumnNumber,
        pub statistics: AlterColumnStatisticsAction,
    }
}

recursa::ast_node! {
    /// `ColId action`.
    #[derive(Debug)]
    pub struct AlterColumnIndexName {
        pub name: crate::tokens::ColId,
        pub action: AlterColumnIndexAction,
    }
}

recursa::ast_node! {
    /// One action of `ALTER INDEX IF EXISTS name action`: an `alter_table_cmd`
    /// or `RENAME TO`. gram.y has no `IF EXISTS` form of `index_partition_cmd`
    /// (`ATTACH PARTITION`) or of `AlterObjectDependsStmt` (REL_17_11 gram.y
    /// 2156-2175, 9579, 9932), so those are [`AlterIndexAction`] variants only
    /// (issue #58).
    ///
    /// Variant ordering:
    /// - `SetTablespace` (`SET TABLESPACE`) and `SetReloptions` (`SET (`) and
    ///   `ResetReloptions` (`RESET …`) — second tokens are disjoint, so order
    ///   is for clarity.
    /// - `AlterColumn` starts with the `ALTER` token, distinct from all
    ///   `SET`/`RESET`/`RENAME` first tokens.
    #[derive(Debug)]
    pub enum AlterIndexIfExistsAction {
        SetTablespace(SetTablespaceClause),
        SetReloptions(SetReloptions),
        ResetReloptions(ResetReloptions),
        AlterColumn(AlterColumnIndexCmd),
        Rename(RenameTo),
    }
}

recursa::ast_node! {
    /// One action on a single-target `ALTER INDEX name action` —
    /// the corpus-exercised subset of `alter_table_cmds` plus `RenameStmt` and
    /// `AlterObjectDependsStmt` and `index_partition_cmd`.
    ///
    /// Variant ordering:
    /// - `SetTablespace` (`SET TABLESPACE`) and `SetReloptions` (`SET (`) and
    ///   `ResetReloptions` (`RESET …`) — second tokens are disjoint, so order
    ///   is for clarity.
    /// - `AlterColumn` starts with the `ALTER` token, distinct from all
    ///   `SET`/`RESET`/`ATTACH`/`RENAME`/`NO`/`DEPENDS` first tokens.
    /// - `Depends` allows a bare `DEPENDS …` (without `NO`), and `NoDepends`
    ///   is reached via the `Depends` arm since both share the
    ///   `DependsOnExtension` type (with `NO` as an `Option`).
    #[derive(Debug)]
    pub enum AlterIndexAction {
        SetTablespace(SetTablespaceClause),
        SetReloptions(SetReloptions),
        ResetReloptions(ResetReloptions),
        Attach(AttachPartitionClause),
        AlterColumn(AlterColumnIndexCmd),
        Depends(DependsOnExtension),
        Rename(RenameTo),
    }
}

recursa::ast_node! {
    /// `ALL IN TABLESPACE name [OWNED BY role_list] SET TABLESPACE new
    /// [NOWAIT]` — Postgres' bulk-relocate action on ALTER INDEX (and ALTER
    /// MATERIALIZED VIEW). Moves every index in the named tablespace to a new
    /// tablespace, optionally filtered by owner role(s).
    #[derive(Debug)]
    pub struct AllInTablespaceBody {
        #[tok(ALL, IN, TABLESPACE, this)]
        pub source: crate::tokens::ColId,
        pub owned_by: Option<OwnedByRoles>,
        pub set_tablespace: SetTablespaceClause,
        #[presence(NOWAIT)]
        pub nowait: bool,
    }
}

recursa::ast_node! {
    /// `OWNED BY role_list` — owner filter on the bulk `ALL IN TABLESPACE`
    /// action.
    #[derive(Debug)]
    pub struct OwnedByRoles {
        #[tok(OWNED, BY, this)]
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    /// `ALTER INDEX [IF EXISTS] name action` plus the bulk `ALTER INDEX ALL IN
    /// TABLESPACE …` form. The two top-level shapes share the leading `ALTER
    /// INDEX` keywords, so they sit on either side of a single enum and share that
    /// LR prefix once.
    ///
    /// Variant ordering: `All` (starts with `ALL`) and `IfExists` (starts
    /// with `IF EXISTS`) before `Single` (starts with a `qualified_name`).
    #[derive(Debug)]
    pub enum AlterIndexBody {
        All(AllInTablespaceBody),
        IfExists(AlterIndexIfExists),
        Single(AlterIndexSingle),
    }
}

recursa::ast_node! {
    /// `IF EXISTS name action`.
    #[derive(Debug)]
    pub struct AlterIndexIfExists {
        #[tok(IF, EXISTS, this)]
        pub name: QualifiedName,
        pub action: AlterIndexIfExistsAction,
    }
}

recursa::ast_node! {
    /// `name action` — the per-index branch of ALTER INDEX.
    #[derive(Debug)]
    pub struct AlterIndexSingle {
        pub name: QualifiedName,
        pub action: AlterIndexAction,
    }
}

recursa::ast_node! {
    /// `ALTER INDEX [IF EXISTS] name action`
    /// `ALTER INDEX ALL IN TABLESPACE name [OWNED BY role_list] SET TABLESPACE
    ///   new [NOWAIT]` — the two top-level shapes of Postgres' `AlterTableStmt`
    /// branches that begin with `ALTER INDEX …`, plus the index branches of
    /// `RenameStmt` / `AlterObjectDependsStmt` (single form).
    #[derive(Debug)]
    pub struct AlterIndexStmt {
        #[tok(ALTER, INDEX, this)]
        pub body: AlterIndexBody,
    }
}
