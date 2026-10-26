//! The Analyzer: the walk over `pgtk-sql`'s flat AST that ties the resolution
//! algorithms together and records what they decide as Annotations.

use pgtk_catalog::{Builtin, Catalog};
use pgtk_sql::FlatAst;
use pgtk_sql::ast::FlatStatement;
use pgtk_sql::ast::ddl::table::FlatCreateTableBody;
use pgtk_sql::ast::dml::select::{
    FlatAlias, FlatColNameTableTail, FlatFromClause, FlatNamedTableRefTail, FlatPlainTableAlias,
    FlatSelectHead, FlatSelectItem, FlatSelectStmt, FlatSelectTargets, FlatSimpleTableRef,
    FlatTableFunctionName, FlatWhereCondition,
};
use pgtk_sql::ast::dml::select::{
    FlatFetchFirstValue, FlatGroupByItem, FlatLimitOffsetClause, FlatLimitingClause,
    FlatOffsetClause, FlatOrderByClause,
};
#[cfg(feature = "since-pg17")]
use pgtk_sql::ast::dml::select::{
    FlatJsonTableColumn, FlatJsonTableColumnKind, FlatJsonTableColumnList, FlatJsonTableFormat,
    FlatJsonTableInner, FlatJsonTableRef, FlatJsonTableTypedTail,
};
use pgtk_sql::ast::dml::values::{FlatSelectClause, FlatSetQuantifier};
use pgtk_sql::ast::session::set_reset::{FlatShowStmt, FlatShowTarget};

mod collate;
use crate::analysis::{CollContext, Strength};
use collate::SublinkInfo;
use pgtk_catalog::Collation;
use pgtk_sql::ast::shared::expr::FlatIsNormalizedTail;
use pgtk_sql::ast::shared::expr::{
    FlatArrayElements, FlatArrayExpr, FlatCaseBody, FlatExtractField, FlatInContent,
    FlatSubstringTail, FlatTrimDir, FlatTrimTail, FlatTrimValuesRest,
};
use pgtk_sql::ast::shared::expr::{
    FlatBitType, FlatBoolTestKind, FlatCastTypeHead, FlatCharacterType, FlatCharacterTypeName,
    FlatDateTimeCastTypeName, FlatExpr, FlatFixedTypeCastFuncName, FlatFloatPrecision, FlatFuncArg,
    FlatFunctionCallBody, FlatFunctionCallTail, FlatGenericTypeName, FlatIconstPrecision,
    FlatIntervalCastTypeModifier, FlatIntervalQualifier, FlatNumericType,
    FlatPrecisionSqlValueFuncName, FlatSqlValueFunction, FlatStringLitSeq0, FlatTimeZoneQualifier,
    FlatTypeCastFunc, FlatTypeModifierSign,
};
#[cfg(feature = "since-pg16")]
use pgtk_sql::ast::shared::expr::{
    FlatCastType, FlatIntervalLitForm, FlatJsonEncoding, FlatTypeArray,
};
use pgtk_sql::ast::shared::names::FlatQualifiedOperatorName;
use pgtk_sql::ast::shared::numbers::FlatUnsignedNumberLit;
use pgtk_sql::tokens::FlatColId;
use pgtk_sql::tokens::literal::FlatAliasName;
use recursa::{Handle, List, List1, Span, Text};

use crate::analysis::RecordField;
use crate::analysis::{
    Analysis, ExprType, FunctionResolution, ResultColumn, ResultDescriptor, TargetEntry,
};
use crate::coerce::Coercer;
use crate::consts::{LiteralKind, make_const};
use crate::diagnostic::{AnalyzeError, Diagnostic, NO_LOCATION, Result, unsupported};
use crate::func::{Decoration, FuncResolver, ObjectName, Projection, ResolvedFunc};
use crate::relation::{
    CteColumn, CteEntry, JoinType, NamespaceItem, Relations, RteKind, Scope, Var,
};
use crate::session::{Cursor, Placeholder, PreparedStatement, Session, SessionEffect};
use crate::settings::Settings;
use crate::typename::{TypeName, TypeNames, TypmodArg};
use crate::types::Typmod;
use crate::types::{TypeRef, Types};
use crate::value::{Coercion, CoercionContext, Form, Typed};
use pgtk_catalog::RelationKind;
use pgtk_catalog::{ArgMode, FunctionKind, PseudoType, RuleEvent, TypeCategory, TypeKind};
use pgtk_sql::TargetVersion;
use pgtk_sql::ast::cursor::declare::{FlatCursorHold, FlatCursorOption, FlatDeclareStmt};
use pgtk_sql::ast::cursor::fetch::{FlatCloseStmt, FlatCloseTarget, FlatFetchStmt, FlatMoveStmt};
use pgtk_sql::ast::ddl::index::FlatIndexTarget;
use pgtk_sql::ast::ddl::materialized_view::FlatCreateMaterializedViewStmt;
use pgtk_sql::ast::ddl::procedure::FlatCallStmt;
use pgtk_sql::ast::ddl::table::FlatCtasSource;
use pgtk_sql::ast::dml::delete::{FlatDeleteStmt, FlatDeleteTableAlias};
use pgtk_sql::ast::dml::insert::{FlatConflictAction, FlatConflictTarget, FlatOnConflictClause};
use pgtk_sql::ast::dml::insert::{FlatInsertRest, FlatInsertStmt, FlatOverridingKind};
#[cfg(feature = "since-pg15")]
use pgtk_sql::ast::dml::merge::{
    FlatInsertAction, FlatMatchedAction, FlatMatchedKind, FlatMergeStmt, FlatMergeTableAlias,
    FlatNotMatchedAction, FlatUpdateAction, FlatWhenClause,
};
use pgtk_sql::ast::dml::select::{
    FlatAsAliasName, FlatValuesRow, FlatWhereClause, FlatWhereOrCurrentClause,
};
use pgtk_sql::ast::dml::select::{
    FlatCubeItem, FlatFetchFirstBody, FlatFetchFirstClause, FlatFetchMode, FlatForUpdateWait,
    FlatGroupByModifier, FlatOnlyRelationName, FlatRollupItem, FlatSortOrder, FlatUsingClause,
    FlatUsingOp,
};
use pgtk_sql::ast::dml::select::{
    FlatForLockingClause, FlatForUpdateClause, FlatLimitLockingClause, FlatLockingMode,
    FlatTableSampleClause,
};
use pgtk_sql::ast::dml::select::{
    FlatFuncTableAlias, FlatFuncTableAliasAfterAs, FlatFuncTableAliasColumnList,
    FlatFuncTableAliasColumns, FlatFuncTableColumnDef, FlatRowsFromColDef, FlatRowsFromFunc,
};
use pgtk_sql::ast::dml::select::{
    FlatJoinCondition, FlatJoinSuffix, FlatJoinType, FlatJoinedTable, FlatLateralBody,
    FlatParenJoinGroup, FlatParenTableRef, FlatTableRef, FlatUnqualifiedJoinKind,
};
use pgtk_sql::ast::dml::select::{FlatSelectIntoClause, FlatValuesBody};
use pgtk_sql::ast::dml::select::{
    FlatTableAlias, FlatXmlNamespaceItem, FlatXmlTableColumnKind, FlatXmlTableColumnOption,
    FlatXmlTableRef,
};
use pgtk_sql::ast::dml::update::{
    FlatReturningClause, FlatSetAssignment, FlatUpdateStmt, FlatUpdateTableAlias,
};
use pgtk_sql::ast::dml::values::{FlatQueryBody, FlatSubquery, FlatTableStmt};
use pgtk_sql::ast::session::set_reset::{
    FlatGenericSetRest, FlatGenericSetValue, FlatVariableSetRest, FlatVariableSetStmt,
};
use pgtk_sql::ast::shared::expr::{
    FlatBetweenModifier, FlatIndirectionEl, FlatXmlElement, FlatXmlElementTail, FlatXmlExists,
    FlatXmlExistsDocument, FlatXmlForest, FlatXmlNamedArg, FlatXmlParse, FlatXmlPi, FlatXmlRoot,
    FlatXmlRootVersion, FlatXmlSerialize, FlatXmlVersionValue,
};
use pgtk_sql::ast::shared::expr::{FlatBracketSubscript, FlatBracketSubscriptValue};
use pgtk_sql::ast::shared::expr::{FlatFuncCallName, FlatFunctionApplicationExpr};
use pgtk_sql::ast::shared::expr::{
    FlatFunctionCallApplication, FlatInlineWindowSpec, FlatQuantifiedComparisonOperand,
    FlatQuantifiedComparisonTail, FlatQuantifiedLikeOperator, FlatWindowFrameBody,
    FlatWindowFrameClause, FlatWindowFrameUnit, FlatWindowSpec, FlatWindowSpecBody,
    FlatWithinGroupClause,
};
#[cfg(feature = "since-pg16")]
use pgtk_sql::ast::shared::expr::{
    FlatJsonArray, FlatJsonArrayAgg, FlatJsonArrayArgs, FlatJsonFormat, FlatJsonKeyValue,
    FlatJsonObject, FlatJsonObjectAgg, FlatJsonObjectLegacyArg, FlatJsonReturning,
    FlatNullOrAbsent, FlatWithOrWithout,
};
#[cfg(feature = "since-pg17")]
use pgtk_sql::ast::shared::expr::{
    FlatJsonBehavior, FlatJsonBehaviorClause, FlatJsonConstructor, FlatJsonExists, FlatJsonPassing,
    FlatJsonQuery, FlatJsonQuotes, FlatJsonScalar, FlatJsonSerialize, FlatJsonValue,
    FlatJsonWrapper, FlatKeepOrOmit,
};
use pgtk_sql::ast::shared::names::{FlatAnyName, FlatQualifiedName};
use pgtk_sql::ast::shared::names::{FlatOnlyRelation, FlatRelationExpr};
use pgtk_sql::ast::shared::with_clause::{
    FlatCteDefinition, FlatWithBody, FlatWithClause, FlatWithClauseHead, FlatWithStatement,
};
use pgtk_sql::ast::shared::with_clause::{FlatCycleClause, FlatSearchDirection};
use pgtk_sql::ast::tcl::prepared::FlatPreparableStmt;
use pgtk_sql::ast::tcl::prepared::{
    FlatDeallocateStmt, FlatDeallocateTarget, FlatExecuteStmt, FlatPrepareStmt, FlatPrepareStmtBody,
};
use pgtk_sql::ast::tcl::transaction::{FlatRollbackAfterWork, FlatRollbackBody, FlatRollbackStmt};
use pgtk_sql::ast::utility::explain::{
    FlatExplainInput, FlatExplainOptValue, FlatExplainStmt, FlatExplainableStmt,
};
use pgtk_sql::tokens::Flattype_function_name;
use pgtk_sql::tokens::literal::FlatWindowRefNameIdent;
use pgtk_sql::tokens::literal::{FlatRelationAliasName, FlatUtilityOptionName};

/// The Analysis of one Statement, keyed by `pgtk-sql`'s flat handles.
pub type FlatAnalysis = Analysis<
    Handle<FlatExpr>,
    Handle<FlatSelectStmt>,
    Handle<FlatSelectItem>,
    Handle<FlatFunctionCallApplication>,
>;

/// Where a function call is written: an expression, or a `FROM` item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CallSite {
    Expr(Handle<FlatExpr>),
    From(Handle<FlatFunctionCallApplication>),
    /// The procedure of a CALL statement.
    Statement,
}

/// The alias of a subquery in FROM, which the grammar requires before 16.
#[cfg(feature = "since-pg16")]
macro_rules! subquery_alias {
    ($alias:expr) => {
        $alias
    };
}
#[cfg(not(feature = "since-pg16"))]
macro_rules! subquery_alias {
    ($alias:expr) => {
        Some($alias)
    };
}

/// The parsed statement Analysis reads.
pub type Statement<'input> = FlatAst<'input, FlatStatement>;

/// What a caller builds once and asks to analyse Statements.
///
/// An Analyzer holds a Catalog and the [`Settings`]. Make it once, then give
/// it each Statement with [`Analyzer::analyze_sql`] (text) or
/// [`Analyzer::analyze`] (a Statement that `pgtk-sql` parsed).
///
/// ```
/// use pgtk_analyze::{Analyzer, GRAMMAR_VERSION, Session, Settings};
///
/// let catalog = pgtk_catalog::fixture::pg17();
/// let settings = Settings::new(&catalog, GRAMMAR_VERSION, ["public"])?;
/// let analyzer = Analyzer::new(&catalog, settings);
///
/// let analysis = analyzer
///     .analyze_sql("SELECT 1 AS one", &[], &Session::new())
///     .expect("PostgreSQL accepts it");
/// assert_eq!(analysis.result.columns[0].name, "one");
/// # Ok::<(), pgtk_analyze::TargetVersionError>(())
/// ```
pub struct Analyzer<'c> {
    catalog: &'c Catalog,
    settings: Settings,
}

impl<'c> Analyzer<'c> {
    /// An Analyzer over a Catalog with the given Settings.
    pub fn new(catalog: &'c Catalog, settings: Settings) -> Self {
        Self { catalog, settings }
    }

    /// The Catalog.
    pub fn catalog(&self) -> &'c Catalog {
        self.catalog
    }

    /// The Settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Analyse one parsed Statement. `source` is the text it was parsed
    /// from, for cursor positions. It does not yet reject a construct newer
    /// than the Target Version; [`Self::analyze_sql`] does. `declared_params` gives a Declared
    /// Parameter Type per positional parameter, in `$n` order; a missing or
    /// `None` entry is inferred. `session` is the Session snapshot that
    /// EXECUTE, FETCH and SHOW of a placeholder read; use
    /// [`Session::new`] when there is none.
    ///
    /// ```
    /// use pgtk_analyze::{Analyzer, GRAMMAR_VERSION, Session, Settings};
    /// use pgtk_catalog::Builtin;
    ///
    /// let catalog = pgtk_catalog::fixture::pg17();
    /// let settings = Settings::new(&catalog, GRAMMAR_VERSION, ["public"]).unwrap();
    /// let analyzer = Analyzer::new(&catalog, settings);
    ///
    /// // Analysis reads the Flat AST of pgtk-sql.
    /// let sql = "SELECT $1 + 1";
    /// let lexed = pgtk_sql::lex(sql);
    /// let mut input = lexed.input();
    /// let statement = pgtk_sql::ast::Statement::parse_flat(&mut input).expect("a statement");
    ///
    /// // Declare `$1` as int8. The result of `int8 + int4` is int8.
    /// let int8 = catalog.builtin(Builtin::Int8);
    /// let analysis = analyzer
    ///     .analyze(sql, &statement, &[Some(int8)], &Session::new())
    ///     .expect("PostgreSQL accepts it");
    /// assert_eq!(analysis.result.params, [int8]);
    /// assert_eq!(analysis.result.columns[0].ty, int8);
    /// ```
    pub fn analyze(
        &self,
        source: &str,
        statement: &Statement<'_>,
        declared_params: &[Option<TypeRef>],
        session: &Session,
    ) -> Result<FlatAnalysis> {
        let mut walker = self.walker(source, statement, session);
        *walker.resolver.coercer.params.borrow_mut() = declared_params.to_vec();
        match walker.statement() {
            Ok(()) => Ok(walker.analysis),
            Err(AnalyzeError::Diagnostic(mut d)) => {
                d.resolve(source);
                Err(AnalyzeError::Diagnostic(d))
            }
            Err(other) => Err(other),
        }
    }

    /// What the rewriter needs to know about a view's query, from its
    /// definition text as `pg_get_viewdef` prints it.
    fn view_info(&self, sql: &str) -> Result<ViewInfo> {
        let statement = parse_definition(sql)
            .ok_or_else(|| unsupported("view definition does not parse", None))?;
        let mut walker = self.walker(sql, &statement, &crate::session::EMPTY_SESSION);
        walker.statement()?;
        walker.view_shape()
    }

    /// A Walker over one parsed statement.
    fn walker<'a, 'input: 'a>(
        &'a self,
        source: &'a str,
        statement: &'a Statement<'input>,
        session: &'a Session,
    ) -> Walker<'a, 'input, 'a> {
        let coercer = Coercer::new(self.catalog);
        let resolver = FuncResolver {
            coercer,
            search_path: &self.settings.search_path,
        };
        Walker {
            ast: statement,
            source,
            resolver,
            relations: Relations {
                types: Types::new(self.catalog),
                search_path: &self.settings.search_path,
            },
            typenames: TypeNames {
                types: Types::new(self.catalog),
                search_path: &self.settings.search_path,
            },
            settings: &self.settings,
            session,
            analysis: FlatAnalysis::default(),
            expr_kind: ExprKind::SelectTarget,
            in_agg_args: 0,
            has_aggs: false,
            agg_locations: Vec::new(),
            outer: Vec::new(),
            outer_kinds: Vec::new(),
            outer_vars: Vec::new(),
            sublinks: std::collections::HashSet::new(),
            sublink_names: std::collections::HashMap::new(),
            last_srf: None,
            resolve_unknowns: true,
            last_target_exprs: Vec::new(),
            allow_into: false,
            into_consumed: false,
            set_op_member: false,
            pending_ctes: Vec::new(),
            defer_to: None,
            recursive_top: None,
            recursion: None,
            outer_recursions: Vec::new(),
            recursion_context: RecursionContext::Ok,
            subquery_depth: 0,
            windows: Vec::new(),
            has_window_funcs: false,
            has_target_srfs: false,
            grouping_calls: Vec::new(),
            lock_refusal: None,
            statement_function: None,
            agg_level: None,
            outer_agg_levels: Vec::new(),
            agg_refs: Vec::new(),
            outer_grouping_calls: Vec::new(),
            collate_exprs: std::collections::HashMap::new(),
            expr_collations: std::collections::HashMap::new(),
            sublink_info: std::collections::HashMap::new(),
            clause_exprs: Vec::new(),
            expr_depth: 0,
            extra_sortgroup_refs: Vec::new(),
            cte_refs: Vec::new(),
            window_locations: Vec::new(),
            record_fields: std::collections::HashMap::new(),
            element_typed: std::collections::HashMap::new(),
            whole_rows: std::collections::HashMap::new(),
        }
    }

    /// Lex, parse and analyse one statement's text. A statement `pgtk-sql`
    /// rejects is Unsupported here, because this crate does not reproduce
    /// PostgreSQL's syntax errors. A construct newer than the Target Version
    /// is the Diagnostic that version's parser gives.
    ///
    /// ```
    /// use pgtk_analyze::{AnalyzeError, Analyzer, GRAMMAR_VERSION, Session, Settings};
    ///
    /// let catalog = pgtk_catalog::fixture::pg17();
    /// let settings = Settings::new(&catalog, GRAMMAR_VERSION, ["public"]).unwrap();
    /// let analyzer = Analyzer::new(&catalog, settings);
    /// let session = Session::new();
    ///
    /// // The Result Descriptor: the name, type and source column of each
    /// // result column, and the type of each parameter.
    /// let analysis = analyzer
    ///     .analyze_sql("SELECT relname FROM pg_class WHERE oid = $1", &[], &session)
    ///     .expect("PostgreSQL accepts it");
    /// let column = &analysis.result.columns[0];
    /// assert_eq!(column.name, "relname");
    /// assert_eq!(catalog[column.ty].name.as_str(), "name");
    /// assert!(column.origin.is_some());
    /// assert_eq!(catalog[analysis.result.params[0]].name.as_str(), "oid");
    ///
    /// // The error that PostgreSQL gives, with its SQLSTATE and position.
    /// let Err(AnalyzeError::Diagnostic(error)) =
    ///     analyzer.analyze_sql("SELECT relnmae FROM pg_class", &[], &session)
    /// else {
    ///     panic!("PostgreSQL rejects it");
    /// };
    /// assert_eq!(error.sqlstate, "42703");
    /// assert_eq!(error.message, r#"column "relnmae" does not exist"#);
    /// assert_eq!(error.position, Some(8));
    /// ```
    pub fn analyze_sql(
        &self,
        sql: &str,
        declared_params: &[Option<TypeRef>],
        session: &Session,
    ) -> Result<FlatAnalysis> {
        self.parsed(sql, |statement| {
            self.analyze(sql, statement, declared_params, session)
        })
    }

    /// The Result Descriptor of the query of a PREPARE, from the text of
    /// the PREPARE statement.
    ///
    /// PostgreSQL's plan cache keeps the query's raw parse tree, not its
    /// analysed form, and analyses it again whenever the cache
    /// revalidates. A Describe of an EXECUTE revalidates, so it answers
    /// from the query as the Catalog is now.
    pub(crate) fn prepared_query(&self, source: &str) -> Result<ResultDescriptor> {
        self.parsed(source, |statement| {
            let mut walker = self.walker(source, statement, &crate::session::EMPTY_SESSION);
            match walker.prepared_query() {
                Ok(()) => Ok(walker.analysis.result),
                Err(AnalyzeError::Diagnostic(mut d)) => {
                    d.resolve(source);
                    Err(AnalyzeError::Diagnostic(d))
                }
                Err(other) => Err(other),
            }
        })
    }

    /// The first construct, in source order, that the Target Version cannot
    /// lex or parse, with the error that server reports (ADR 0005). `None`
    /// when the Target Version is the Grammar Version.
    ///
    /// pgtk-sql answers the Minimum Version on its arena parse only, so this
    /// parses the statement a second time.
    fn too_new_for_target(&self, sql: &str, lexed: &pgtk_sql::LexResult<'_>) -> Option<Diagnostic> {
        use pgtk_sql::MinimumVersion;
        use pgtk_sql::minimum_version::lexical_minimum_version_above;
        let target = self.settings.target_version();
        if target >= pgtk_sql::TARGET_VERSION {
            return None;
        }
        let lexical = lexical_minimum_version_above(lexed.tokens(), target);
        let syntactic = pgtk_sql::ast::Statement::parse(&mut lexed.input())
            .ok()
            .and_then(|parsed| parsed.minimum_version_above(target));
        let found = [lexical, syntactic]
            .into_iter()
            .flatten()
            .min_by_key(|found| found.span().map_or(0, |span| span.start()))?;
        let at = found.span().map_or(0, |span| span.start() as usize);
        let mut d = match found.message() {
            Some(message) => Diagnostic::new(found.sqlstate().unwrap_or("42601"), message),
            None => {
                let token = lexed
                    .tokens()
                    .find(|token| token.span().start() as usize >= at)
                    .map_or("", |token| token.text());
                Diagnostic::new("42601", format!("syntax error at or near \"{token}\""))
            }
        };
        if let Some(hint) = found.hint() {
            d = d.hint(hint);
        }
        let mut d = d.at(at);
        d.resolve(sql);
        Some(d)
    }

    /// Lex and parse one statement's text, and hand the parse to `f`.
    fn parsed<T>(&self, sql: &str, f: impl FnOnce(&Statement<'_>) -> Result<T>) -> Result<T> {
        let lexed = pgtk_sql::lex(sql);
        if lexed.errors().next().is_some() {
            return Err(unsupported("statement does not lex", None));
        }
        // A trailing semicolon is the document's terminator, not part of
        // the statement.
        let terminator = lexed
            .tokens()
            .last()
            .filter(|token| token.kind() == pgtk_sql::TokenKind::SEMI)
            .map(|token| token.span());
        let lexed = match terminator {
            None => lexed,
            Some(terminator) => {
                let mut builder = pgtk_sql::LexBuilder::new(sql);
                for token in lexed.tokens() {
                    if token.span() != terminator
                        && builder.append(token.kind(), token.span()).is_err()
                    {
                        return Err(unsupported("statement does not lex", None));
                    }
                }
                builder.finish()
            }
        };
        if let Some(d) = self.too_new_for_target(sql, &lexed) {
            return Err(d.into());
        }
        let mut input = lexed.input();
        let Ok(statement) = pgtk_sql::ast::Statement::parse_flat(&mut input) else {
            return Err(unsupported("statement does not parse", None));
        };
        if !input.is_eof() {
            return Err(unsupported("statement does not parse", None));
        }
        f(&statement)
    }
}

/// The walk over one Statement.
/// One element of an `ARRAY[...]` constructor or of one of its sub-lists.
#[derive(Clone, Copy)]
enum ArrayItem {
    Expr(Handle<FlatExpr>),
    Nested(Handle<pgtk_sql::ast::shared::expr::FlatNestedArrayElements>),
}

struct Walker<'a, 'input, 'c> {
    ast: &'a Statement<'input>,
    source: &'a str,
    resolver: FuncResolver<'c>,
    relations: Relations<'c>,
    typenames: TypeNames<'c>,
    settings: &'a Settings,
    /// The Session snapshot: what EXECUTE, FETCH and SHOW read.
    session: &'a Session,
    analysis: FlatAnalysis,
    /// `p_expr_kind`: which clause is being transformed.
    expr_kind: ExprKind,
    /// Depth of aggregate argument lists being transformed, for the
    /// nesting check.
    in_agg_args: usize,
    /// `p_hasAggs` of the query level being transformed.
    has_aggs: bool,
    /// Locations of the aggregates of the query level, in source order.
    agg_locations: Vec<usize>,
    /// The scopes of the enclosing query levels, outermost first.
    outer: Vec<Scope>,
    /// `p_expr_kind` of each enclosing level when it entered its subquery.
    outer_kinds: Vec<ExprKind>,
    /// Column references from inner levels to enclosing ones, for the
    /// enclosing level's ungrouped-column check.
    outer_vars: Vec<OuterRef>,
    /// Expressions that are `SubLink`s: the expressions of the subquery
    /// inside them belong to another level.
    sublinks: std::collections::HashSet<Handle<FlatExpr>>,
    /// The name of the first target column of each scalar `SubLink`, for
    /// `FigureColname`.
    sublink_names: std::collections::HashMap<Handle<FlatExpr>, String>,
    /// `p_last_srf`: the location of the last set-returning call resolved.
    last_srf: Option<usize>,
    /// `p_resolve_unknowns`: unknown-typed target entries become text.
    resolve_unknowns: bool,
    /// The expressions of the target entries of the last SELECT analysed.
    last_target_exprs: Vec<Option<Handle<FlatExpr>>>,
    /// `transformOptionalSelectInto`: the next SELECT may carry INTO, which
    /// makes the statement a CREATE TABLE AS.
    allow_into: bool,
    /// A SELECT with INTO was accepted: the statement creates a table.
    into_consumed: bool,
    /// The SELECT being analysed is a member of a set operation other than
    /// the first.
    set_op_member: bool,
    /// The WITH queries analysed for the query level about to start; its
    /// Scope takes them.
    pending_ctes: Vec<CteEntry>,
    /// A forward reference to a WITH item of a RECURSIVE WITH that is not
    /// analysed yet: the level and index of its entry. `with_item_ordered`
    /// analyses that item first, as `TopologicalSort` orders the items.
    defer_to: Option<(usize, usize)>,
    /// A recursive WITH query's body is about to be analysed: where its
    /// entry lives (outer level, index), for `determineRecursiveColTypes`.
    recursive_top: Option<(usize, usize)>,
    /// The state of the recursive WITH query under analysis.
    recursion: Option<RecursionState>,
    /// The recursive WITH queries enclosing the one being analysed, with
    /// the recursion context each was in when a nested WITH query began:
    /// `checkWellFormedRecursion` walks a nested WITH query as part of
    /// the enclosing term.
    outer_recursions: Vec<(RecursionState, RecursionContext)>,
    /// `checkWellFormedRecursion`'s context for a recursive reference.
    recursion_context: RecursionContext,
    /// Depth of sublinks and FROM subqueries below the statement.
    subquery_depth: usize,
    /// `p_windowdefs`: the WINDOW clause definitions of the query level,
    /// then each `OVER (...)` written in it.
    windows: Vec<WindowDef>,
    /// `p_hasWindowFuncs`.
    has_window_funcs: bool,
    /// `p_hasTargetSRFs`: a set-returning call in the target list or in
    /// a clause that is effectively part of it.
    has_target_srfs: bool,
    /// The `GROUPING(...)` calls of this level with their arguments, for
    /// `finalize_grouping_exprs`.
    grouping_calls: Vec<(Handle<FlatExpr>, Vec<Handle<FlatExpr>>)>,
    /// What `CheckSelectLocking` would refuse for the query level last
    /// analysed, were a locking clause pushed down into it.
    lock_refusal: Option<&'static str>,
    /// The procedure a CALL statement resolved to.
    statement_function: Option<FunctionResolution>,
    /// `agglevelsup` of the aggregate being resolved when it belongs to an
    /// outer level: that level's index.
    agg_level: Option<usize>,
    /// The levels an inner query's aggregate was found to belong to.
    outer_agg_levels: Vec<usize>,
    /// Every aggregate call of the statement so far, with the level it
    /// belongs to: what `check_agg_arguments` finds for a nesting check.
    agg_refs: Vec<AggRef>,
    /// GROUPING calls that belong to an outer level, checked against that
    /// level's grouping expressions when it finishes.
    outer_grouping_calls: Vec<OuterGrouping>,
    /// Every WITH query reference of the statement so far, with the level
    /// the query is defined at.
    cte_refs: Vec<CteRef>,
    /// Each COLLATE expression's collation and the clause's location.
    collate_exprs:
        std::collections::HashMap<Handle<FlatExpr>, (pgtk_catalog::Handle<Collation>, usize)>,
    /// `exprCollation` of each expression once assigned.
    expr_collations:
        std::collections::HashMap<Handle<FlatExpr>, Option<pgtk_catalog::Handle<Collation>>>,
    /// What each sublink contributes to collation assignment.
    sublink_info: std::collections::HashMap<Handle<FlatExpr>, SublinkInfo>,
    /// The top-level expressions of the query level's clauses, for
    /// collation assignment.
    clause_exprs: Vec<Handle<FlatExpr>>,
    /// Depth of expression transformation, to know a clause's top.
    expr_depth: usize,
    /// Target entries that are sort or group keys of a window or of
    /// DISTINCT ON, besides ORDER BY and GROUP BY.
    extra_sortgroup_refs: Vec<usize>,
    /// Locations of the window function calls of the query level.
    window_locations: Vec<usize>,
    /// The fields of expressions of type `record` whose shape is known: a
    /// row constructor, or a whole-row reference to a non-table entry.
    record_fields: std::collections::HashMap<Handle<FlatExpr>, Vec<RecordField>>,
    /// The analysed form of each row constructor element, for comparisons.
    element_typed: std::collections::HashMap<Handle<FlatExpr>, Typed>,
    /// Whole-row references by expression, for field selection on them.
    whole_rows: std::collections::HashMap<Handle<FlatExpr>, (usize, NamespaceItem)>,
}

/// The lower and upper expressions of one subscript, `[lower:upper]`.
type SubscriptBounds = (Option<Handle<FlatExpr>>, Option<Handle<FlatExpr>>);

/// `WindowDef`: a window specification as written.
#[derive(Clone, Debug)]
struct WindowDef {
    name: Option<String>,
    refname: Option<String>,
    partition: Vec<Handle<FlatExpr>>,
    order_by: Option<Handle<FlatOrderByClause>>,
    frame: Option<Handle<FlatWindowFrameClause>>,
    /// Byte offset of the specification, or of the name in `OVER name`.
    location: usize,
}

/// A frame bound as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameBound {
    UnboundedPreceding,
    OffsetPreceding(Handle<FlatExpr>),
    CurrentRow,
    OffsetFollowing(Handle<FlatExpr>),
    UnboundedFollowing,
}

/// `RecursionContext`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecursionContext {
    Ok,
    OuterJoin,
    Intersect,
    Except,
}

/// The recursive WITH query being analysed.
#[derive(Clone, Debug)]
struct RecursionState {
    /// Where its entry lives: outer level index and CTE index.
    level: usize,
    index: usize,
    name: String,
    location: usize,
    /// The top of its body was UNION [ALL].
    form_ok: bool,
    /// Locations of an ORDER BY, OFFSET or LIMIT on the body's top level.
    order_by: Option<usize>,
    offset: Option<usize>,
    limit: Option<usize>,
    /// The recursive query has a locking clause.
    locking: bool,
    /// The columns fixed from the non-recursive term.
    fixed: Option<Vec<CteColumn>>,
    /// Whether the left and the right side of the UNION are plain
    /// SELECTs, which SEARCH and CYCLE require.
    sides_leaf: (bool, bool),
    /// The subquery depth of the recursive term.
    term_depth: usize,
    /// The query's own WITH clause is being analysed, which
    /// `checkWellFormedRecursion` walks as a subquery.
    in_with: bool,
    /// The SEARCH and CYCLE columns, which `addRangeTableEntryForCTE`
    /// adds to the self-reference.
    extra_columns: Vec<CteColumn>,
    /// References to the query inside its recursive term.
    references: usize,
}

/// What a `FROM` item calls. gram.y's `func_table` takes a
/// `func_expr_windowless`, which is a `func_application` or one of the
/// special expressions of `func_expr_common_subexpr`.
#[derive(Clone, Copy)]
enum FromFuncCall {
    /// A call, whose arguments and name resolve a function.
    Application(Handle<FlatFunctionCallApplication>),
    /// A special expression, which the ordinary expression walker
    /// analyses.
    Special(Handle<FlatExpr>),
}

/// One function of a `FROM` item before it is transformed.
struct FromFunc {
    call: FromFuncCall,
    /// The name of a `func_application`, or the name `FigureColname`
    /// gives a special expression, which is the name of its form.
    name: ObjectName,
    coldef: Option<Vec<(String, Handle<pgtk_sql::ast::shared::expr::FlatCastType>)>>,
    coldef_location: Option<usize>,
    location: usize,
}

/// One function of a `FROM` item after it is transformed.
struct ResolvedFromFunc {
    ty: TypeRef,
    typmod: Typmod,
    funcname: String,
    coldef: Option<Vec<(String, Handle<pgtk_sql::ast::shared::expr::FlatCastType>)>>,
    coldef_location: Option<usize>,
    func: Option<pgtk_catalog::Handle<pgtk_catalog::Function>>,
    /// `get_call_expr_argtype` for each input argument: the argument's
    /// type once an unknown literal has taken its declared type.
    actual: Vec<TypeRef>,
    location: usize,
}

/// The `PATH` and `DEFAULT` expressions of an XMLTABLE column.
type XmlColumnExprs = (Option<Handle<FlatExpr>>, Option<Handle<FlatExpr>>);

/// The action of one MERGE WHEN clause that assigns values.
#[cfg(feature = "since-pg15")]
#[derive(Clone, Copy, Debug)]
enum MergeAction {
    None,
    Update(Handle<FlatUpdateAction>),
    Insert(Handle<FlatInsertAction>),
}

/// What the rewriter needs to know about a view's query.
struct ViewInfo {
    /// Why the view is not automatically updatable, before the check that
    /// it has an updatable column.
    detail: Option<&'static str>,
    /// The single base relation the view selects from.
    base: Option<pgtk_catalog::Handle<pgtk_catalog::Relation>>,
    /// Per view column: the base column it is, or why it is not updatable.
    columns: Vec<std::result::Result<i16, &'static str>>,
}

impl ViewInfo {
    /// `view_query_is_auto_updatable`.
    fn auto_update_detail(&self, check_cols: bool) -> Option<&'static str> {
        if let Some(d) = self.detail {
            return Some(d);
        }
        if check_cols && self.columns.iter().all(|c| c.is_err()) {
            return Some("Views that have no updatable columns are not automatically updatable.");
        }
        None
    }
}

/// The command the rewriter sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DmlEvent {
    Insert,
    Update,
    Delete,
    #[cfg_attr(not(feature = "since-pg15"), allow(dead_code))]
    Merge,
}

/// The command of one MERGE action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MergeKind {
    Insert,
    Update,
    Delete,
    Nothing,
}

/// One `rewriteTargetListIU` check: the columns a command assigns, its
/// OVERRIDING clause, and whether it inserts.
struct AssignedSet {
    assigned: Vec<(i16, bool)>,
    override_kind: Option<FlatOverridingKind>,
    insert: bool,
}

/// One INSERT target column: name, type, typmod, location, and the
/// indirection written after the name.
type InsertColumn = (
    String,
    TypeRef,
    Typmod,
    usize,
    Vec<Handle<FlatIndirectionEl>>,
);

/// How an assignment reached its target through indirection.
#[derive(Clone, Debug)]
enum AssignKind {
    Plain,
    Subscripted(String),
    Subfield(String),
}

/// One value assigned to a column by INSERT or UPDATE.
#[derive(Clone, Debug)]
enum InsertValue {
    /// The `DEFAULT` keyword, at its location.
    Default(usize),
    /// An expression of the statement.
    Expr(Handle<FlatExpr>, Typed),
    /// A column of the INSERT's SELECT.
    Column(Typed),
}

impl InsertValue {
    fn location(&self) -> usize {
        match self {
            Self::Default(at) => *at,
            Self::Expr(_, t) | Self::Column(t) => t.location,
        }
    }
}

/// One member of a set operation chain before it is analysed.
#[derive(Clone, Copy, Debug)]
enum SetLeaf {
    Select(Handle<FlatSelectStmt>),
    Values(Handle<FlatValuesBody>),
    Parens(Handle<FlatSubquery>),
    Table(Handle<FlatTableStmt>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SetOpKind {
    Union,
    Intersect,
    Except,
}

/// A set operation chain grouped by precedence.
#[derive(Debug)]
enum SetTree {
    Leaf(usize),
    Op {
        op: SetOpKind,
        all: bool,
        left: Box<SetTree>,
        right: Box<SetTree>,
    },
}

/// One result column of a set operation node.
#[derive(Clone, Debug)]
struct SetColumn {
    name: String,
    ty: TypeRef,
    typmod: Typmod,
    /// The column's collation state: a leaf's expression as assigned,
    /// a set operation's as `select_common_collation` merges it.
    state: CollContext,
    origin: Option<(pgtk_catalog::Handle<pgtk_catalog::Relation>, i16)>,
    /// `exprLocation` of the node's entry: the leaf's expression, or the
    /// expression that decided the merged type.
    location: usize,
    /// The location of the leftmost leaf's expression.
    leftmost_location: usize,
}

/// `TypeFuncClass`.
enum ResultClass {
    Composite(Vec<(String, TypeRef, Typmod)>),
    Scalar,
    Record,
    Other,
}

/// A `Var` with `varlevelsup > 0`, as seen from the level it refers to.
/// The argument items of a function call body, its ORDER BY, and whether
/// it is `(*)`, `DISTINCT` or `VARIADIC`.
type CallBodyItems = (
    Vec<Handle<FlatFuncArg>>,
    Option<Handle<FlatOrderByClause>>,
    (bool, bool, bool),
);

/// The ORDER BY expressions and the FILTER expression of an aggregate call.
type AggregateClauses = (Vec<Handle<FlatExpr>>, Option<Handle<FlatExpr>>);

/// The text of a view or rule definition, as `pg_get_viewdef` or
/// `pg_get_ruledef` gives it, parsed as one Statement without its
/// terminator.
fn parse_definition(sql: &str) -> Option<Statement<'_>> {
    let lexed = pgtk_sql::lex(sql);
    if lexed.errors().next().is_some() {
        return None;
    }
    let terminator = lexed
        .tokens()
        .last()
        .filter(|token| token.kind() == pgtk_sql::TokenKind::SEMI)
        .map(|token| token.span());
    let lexed = match terminator {
        None => lexed,
        Some(terminator) => {
            let mut builder = pgtk_sql::LexBuilder::new(sql);
            for token in lexed.tokens() {
                if token.span() != terminator && builder.append(token.kind(), token.span()).is_err()
                {
                    return None;
                }
            }
            builder.finish()
        }
    };
    let mut input = lexed.input();
    let statement = pgtk_sql::ast::Statement::parse_flat(&mut input).ok()?;
    if !input.is_eof() {
        return None;
    }
    Some(statement)
}

/// Whether a CREATE RULE statement has an action with a RETURNING list.
fn rule_action_returns(statement: &Statement<'_>) -> bool {
    use pgtk_sql::ast::ddl::rule::{FlatRuleActionStmt, FlatRuleActions, FlatRuleWithBody};
    fn node<'s, N>(st: &'s Statement<'_>, h: Handle<N>) -> &'s N
    where
        N: recursa::FlatNode,
        for<'i> Statement<'i>: recursa::FlatStorage<N>,
    {
        st.get(h).expect("handle came from this AST")
    }
    let FlatStatement::CreateRule(r) = node(statement, statement.root()) else {
        return false;
    };
    let actions = node(
        statement,
        node(statement, node(statement, *r).do_clause).actions,
    );
    let list: Vec<Handle<FlatRuleActionStmt>> = match actions {
        FlatRuleActions::Nothing => Vec::new(),
        FlatRuleActions::Single(a) => vec![*a],
        FlatRuleActions::Multi(l) => statement.lists().get(node(statement, *l).actions).to_vec(),
    };
    list.into_iter().any(|a| match node(statement, a) {
        FlatRuleActionStmt::Insert(i) => node(statement, *i).returning.is_some(),
        FlatRuleActionStmt::Update(u) => node(statement, *u).returning.is_some(),
        FlatRuleActionStmt::Delete(d) => node(statement, *d).returning.is_some(),
        FlatRuleActionStmt::With(w) => match node(statement, node(statement, *w).body) {
            FlatRuleWithBody::Insert(i) => node(statement, *i).returning.is_some(),
            FlatRuleWithBody::Update(u) => node(statement, *u).returning.is_some(),
            FlatRuleWithBody::Delete(d) => node(statement, *d).returning.is_some(),
            FlatRuleWithBody::Query(_) => false,
        },
        FlatRuleActionStmt::Query(_) | FlatRuleActionStmt::Notify(_) => false,
    })
}

/// The items of one WITH clause while they are analysed.
struct WithItems {
    defs: Vec<Handle<FlatCteDefinition>>,
    entries: Vec<CteEntry>,
    /// The column aliases written for each item.
    aliases: Vec<Vec<String>>,
    /// The WITH queries visible: the enclosing ones, then, under
    /// RECURSIVE, every item of this clause.
    known: Vec<CteEntry>,
    /// The index of this clause's first item in `known`.
    base: usize,
    recursive: bool,
    /// The query level the clause belongs to.
    level: usize,
    done: Vec<bool>,
    /// The items being analysed, outermost first.
    in_progress: Vec<usize>,
}

/// A GROUPING call inside a subquery whose arguments name an outer level.
#[derive(Clone, Debug)]
struct OuterGrouping {
    /// The level the call belongs to.
    level: usize,
    /// Each argument with, when it is a plain column of that level, its
    /// `Var` there.
    args: Vec<(Handle<FlatExpr>, Option<Var>)>,
}

/// An aggregate call and the query level it belongs to.
#[derive(Clone, Copy, Debug)]
struct AggRef {
    /// The level, 0 being the statement.
    level: usize,
    location: usize,
}

/// A WITH query reference and the level the query is defined at.
#[derive(Clone, Debug)]
struct CteRef {
    level: usize,
    name: String,
    location: usize,
}

#[derive(Clone, Copy, Debug)]
struct OuterRef {
    /// The referenced level, 0 being the statement.
    level: usize,
    /// The clause of that level the reference is inside.
    kind: ExprKind,
    /// The `Var` with `levelsup` 0.
    var: Var,
    location: usize,
}

/// `ParseExprKind`, with `ParseExprKindName`'s spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExprKind {
    SelectTarget,
    Where,
    Having,
    GroupBy,
    OrderBy,
    DistinctOn,
    Limit,
    Offset,
    Filter,
    JoinOn,
    JoinUsing,
    FromFunction,
    FromSubselect,
    Values,
    ValuesSingle,
    Returning,
    UpdateSource,
    WindowPartition,
    WindowOrder,
    WindowFrameRows,
    WindowFrameRange,
    WindowFrameGroups,
    MergeWhen,
    MergeReturning,
    IndexExpression,
    IndexPredicate,
    CallArgument,
    CycleMark,
}

impl ExprKind {
    fn name(self) -> &'static str {
        match self {
            Self::SelectTarget => "SELECT",
            Self::Where => "WHERE",
            Self::Having => "HAVING",
            Self::GroupBy => "GROUP BY",
            Self::OrderBy => "ORDER BY",
            Self::DistinctOn => "DISTINCT ON",
            Self::Limit => "LIMIT",
            Self::Offset => "OFFSET",
            Self::Filter => "FILTER",
            Self::JoinOn => "JOIN/ON",
            Self::JoinUsing => "JOIN/USING",
            Self::FromFunction => "function in FROM",
            Self::FromSubselect => "sub-SELECT in FROM",
            Self::Values | Self::ValuesSingle => "VALUES",
            Self::Returning | Self::MergeReturning => "RETURNING",
            Self::UpdateSource => "UPDATE",
            Self::WindowPartition => "window PARTITION BY",
            Self::WindowOrder => "window ORDER BY",
            Self::WindowFrameRows => "window ROWS",
            Self::WindowFrameRange => "window RANGE",
            Self::WindowFrameGroups => "window GROUPS",
            Self::MergeWhen => "MERGE WHEN",
            Self::IndexExpression => "index expression",
            Self::IndexPredicate => "index predicate",
            Self::CallArgument => "CALL",
            Self::CycleMark => "CYCLE",
        }
    }

    /// `check_srf_call_placement`: where a set-returning call is refused.
    fn refuses_srf(self) -> bool {
        matches!(
            self,
            Self::Where
                | Self::Having
                | Self::Filter
                | Self::Limit
                | Self::Offset
                | Self::JoinOn
                | Self::JoinUsing
                | Self::FromSubselect
                | Self::Values
                | Self::Returning
                | Self::MergeReturning
                | Self::IndexExpression
                | Self::IndexPredicate
                | Self::CallArgument
                | Self::CycleMark
                | Self::UpdateSource
        )
    }

    /// `check_agglevels_and_constraints`: where an aggregate is refused.
    fn refuses_aggregate(self) -> bool {
        matches!(
            self,
            Self::Where
                | Self::Filter
                | Self::GroupBy
                | Self::Limit
                | Self::Offset
                | Self::JoinOn
                | Self::JoinUsing
                | Self::FromFunction
                | Self::FromSubselect
                | Self::Values
                | Self::ValuesSingle
                | Self::Returning
                | Self::MergeReturning
                | Self::IndexExpression
                | Self::IndexPredicate
                | Self::CallArgument
                | Self::CycleMark
                | Self::UpdateSource
                | Self::WindowFrameRows
                | Self::WindowFrameRange
                | Self::WindowFrameGroups
                | Self::MergeWhen
        )
    }

    /// `transformWindowFuncCall`: where a window function is refused, with
    /// its message.
    fn window_refusal(self) -> Option<String> {
        match self {
            Self::JoinOn | Self::JoinUsing => {
                Some("window functions are not allowed in JOIN conditions".to_owned())
            }
            Self::FromFunction => {
                Some("window functions are not allowed in functions in FROM".to_owned())
            }
            Self::WindowPartition
            | Self::WindowOrder
            | Self::WindowFrameRows
            | Self::WindowFrameRange
            | Self::WindowFrameGroups => {
                Some("window functions are not allowed in window definitions".to_owned())
            }
            Self::MergeWhen => {
                Some("window functions are not allowed in MERGE WHEN conditions".to_owned())
            }
            Self::IndexExpression => {
                Some("window functions are not allowed in index expressions".to_owned())
            }
            Self::IndexPredicate => {
                Some("window functions are not allowed in index predicates".to_owned())
            }
            Self::SelectTarget | Self::OrderBy | Self::DistinctOn => None,
            other => Some(format!(
                "window functions are not allowed in {}",
                other.name()
            )),
        }
    }

    /// The message `check_agglevels_and_constraints` raises here.
    fn aggregate_refusal(self) -> String {
        match self {
            Self::FromFunction => {
                "aggregate functions are not allowed in functions in FROM".to_owned()
            }
            Self::JoinOn | Self::JoinUsing => {
                "aggregate functions are not allowed in JOIN conditions".to_owned()
            }
            Self::FromSubselect => {
                "aggregate functions are not allowed in FROM clause of their own query level"
                    .to_owned()
            }
            Self::MergeWhen => {
                "aggregate functions are not allowed in MERGE WHEN conditions".to_owned()
            }
            Self::IndexExpression => {
                "aggregate functions are not allowed in index expressions".to_owned()
            }
            Self::IndexPredicate => {
                "aggregate functions are not allowed in index predicates".to_owned()
            }
            Self::WindowFrameRows => {
                "aggregate functions are not allowed in window ROWS".to_owned()
            }
            Self::WindowFrameRange => {
                "aggregate functions are not allowed in window RANGE".to_owned()
            }
            Self::WindowFrameGroups => {
                "aggregate functions are not allowed in window GROUPS".to_owned()
            }
            other => format!("aggregate functions are not allowed in {}", other.name()),
        }
    }
}

/// `JsonFormatType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "since-pg16")]
enum JsonFormatKind {
    Default,
    Json,
    Jsonb,
}

/// `JsonReturning`: the RETURNING clause of a SQL/JSON construct.
#[derive(Clone, Debug)]
#[cfg(feature = "since-pg16")]
struct JsonReturning {
    ty: Option<TypeRef>,
    typmod: Typmod,
    format: JsonFormatKind,
    format_location: Option<usize>,
}

/// Which SQL/JSON query function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "since-pg17")]
enum JsonOp {
    Exists,
    Query,
    Value,
}

/// The context item of a SQL/JSON query function.
#[cfg(feature = "since-pg17")]
enum JsonContext {
    /// An expression with its optional FORMAT clause.
    Expr(Handle<FlatExpr>, Option<Handle<FlatJsonFormat>>),
    /// A JSON_TABLE column's context: the row of the document, already
    /// json or jsonb.
    Typed,
}

/// The RETURNING part of a SQL/JSON query function.
#[cfg(feature = "since-pg17")]
enum JsonReturningSpec {
    None,
    Clause(Handle<FlatJsonReturning>),
    /// A JSON_TABLE column's type and its optional FORMAT clause.
    Column(Handle<FlatCastType>, Option<Handle<FlatJsonTableFormat>>),
}

/// The parts of a SQL/JSON query function call.
#[cfg(feature = "since-pg17")]
struct JsonFuncParts {
    context: JsonContext,
    /// The path expression; `None` for a JSON_TABLE column's constant path.
    path: Option<Handle<FlatExpr>>,
    passing: Option<Handle<FlatJsonPassing>>,
    returning: JsonReturningSpec,
    wrapper: Option<Handle<FlatJsonWrapper>>,
    quotes: Option<Handle<FlatJsonQuotes>>,
    /// The ON EMPTY behavior and the start of its clause.
    on_empty: Option<(Handle<FlatJsonBehavior>, usize)>,
    /// The ON ERROR behavior and the start of its clause.
    on_error: Option<(Handle<FlatJsonBehavior>, usize)>,
    /// The JSON_TABLE column the call stands for, which changes the
    /// behavior diagnostics.
    column: Option<String>,
}

/// The GROUP BY clause of one level: its grouping expressions and, with
/// grouping sets, the expanded sets as lists of target indexes.
#[derive(Default)]
struct Grouping {
    entries: Vec<GroupEntry>,
    /// The target list index of each entry.
    refs: Vec<usize>,
    /// The expanded grouping sets, when the clause has any.
    sets: Option<Vec<Vec<usize>>>,
}

impl Grouping {
    /// `groupClauseCommonVars`: the entries in every grouping set.
    fn common(&self) -> Vec<GroupEntry> {
        let Some(sets) = &self.sets else {
            return self.entries.clone();
        };
        self.entries
            .iter()
            .zip(&self.refs)
            .filter(|(_, r)| sets.iter().all(|set| set.contains(r)))
            .map(|(e, _)| *e)
            .collect()
    }
}

/// The cross product of two lists of grouping sets, each pair united.
fn cross_union(left: &[Vec<usize>], right: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut out = Vec::with_capacity(left.len() * right.len());
    for l in left {
        for r in right {
            let mut u = l.clone();
            for x in r {
                if !u.contains(x) {
                    u.push(*x);
                }
            }
            out.push(u);
        }
    }
    out
}

/// One GROUP BY entry, for the ungrouped-column check.
#[derive(Clone, Copy, Debug)]
enum GroupEntry {
    /// A plain column reference.
    Var(crate::relation::Var),
    /// Any other expression, compared structurally.
    Expr(Handle<FlatExpr>),
}

/// How PostgreSQL's `FigureColname` names an expression: the name and its
/// strength.
struct Colname {
    name: Option<String>,
    strength: u8,
}

impl<'a, 'input, 'c> Walker<'a, 'input, 'c> {
    fn types(&self) -> &Types<'c> {
        &self.resolver.coercer.types
    }

    fn catalog(&self) -> &'c Catalog {
        self.resolver.coercer.types.catalog
    }

    fn coercer(&self) -> &Coercer<'c> {
        &self.resolver.coercer
    }

    /// A node's span as a byte range, or an empty range at 0 when spans are
    /// off.
    fn span<N>(&self, handle: Handle<N>) -> std::ops::Range<usize>
    where
        N: recursa::FlatNode,
        Statement<'input>: recursa::FlatStorage<N>,
    {
        self.ast.span(handle).map_or(0..0, span_range)
    }

    fn text<K>(&self, handle: Text<K>) -> &'a str {
        let ast: &'a Statement<'input> = self.ast;
        ast.text(handle).map(|t| t.as_str()).unwrap_or("")
    }

    fn text_span<K>(&self, handle: Text<K>) -> std::ops::Range<usize> {
        self.ast.text_span(handle).map_or(0..0, span_range)
    }

    fn get<N>(&self, handle: Handle<N>) -> &'a N
    where
        N: recursa::FlatNode,
        Statement<'input>: recursa::FlatStorage<N>,
    {
        let ast: &'a Statement<'input> = self.ast;
        ast.get(handle).expect("handle came from this AST")
    }

    fn list<N: recursa::FlatNode>(&self, list: List<Handle<N>>) -> &'a [Handle<N>] {
        let ast: &'a Statement<'input> = self.ast;
        ast.lists().get(list)
    }

    fn list1<N: recursa::FlatNode>(&self, list: List1<Handle<N>>) -> &'a [Handle<N>] {
        let ast: &'a Statement<'input> = self.ast;
        ast.lists().get1(list)
    }

    /// Fold an identifier's spelling.
    fn fold(&self, raw: &str, location: usize) -> Result<String> {
        match pgtk_sql::ident::fold_identifier(raw) {
            Ok(folded) => Ok(folded.name.into_owned()),
            Err(error) => Err(unsupported(
                format!("identifier {raw:?}: {error}"),
                Some(location..location),
            )),
        }
    }

    fn col_id(&self, handle: Handle<FlatColId>) -> Result<(String, usize)> {
        let FlatColId::Text(t) = self.get(handle);
        let span = self.text_span(*t);
        Ok((self.fold(self.text(*t), span.start)?, span.start))
    }

    /// The bare alias of a DELETE, UPDATE or MERGE target.
    fn relation_alias_name(&self, handle: Handle<FlatRelationAliasName>) -> Result<String> {
        let FlatRelationAliasName::Text(t) = self.get(handle);
        let span = self.text_span(*t);
        self.fold(self.text(*t), span.start)
    }

    fn alias_name(&self, handle: Handle<FlatAliasName>) -> Result<(String, usize)> {
        let FlatAliasName::Text(t) = self.get(handle);
        let span = self.text_span(*t);
        Ok((self.fold(self.text(*t), span.start)?, span.start))
    }

    fn record_type(&mut self, handle: Handle<FlatExpr>, typed: &Typed) {
        self.analysis.expr_types.insert(
            handle,
            ExprType {
                ty: typed.ty,
                typmod: typed.typmod,
            },
        );
    }

    /// Record coercions applied to a node's result, after any earlier ones.
    fn record_coercions(&mut self, handle: Handle<FlatExpr>, steps: Vec<Coercion>) {
        if steps.is_empty() {
            return;
        }
        let mut all = self
            .analysis
            .coercions
            .get(handle)
            .cloned()
            .unwrap_or_default();
        all.extend(steps);
        self.analysis.coercions.insert(handle, all);
    }

    // ----- statements -----

    fn statement(&mut self) -> Result<()> {
        let root = self.ast.root();
        match self.get(root) {
            FlatStatement::Query(body) => {
                self.allow_into = true;
                let target_list = self.query_body(*body, None, None, Vec::new())?;
                self.allow_into = false;
                if self.into_consumed {
                    // SELECT ... INTO is a CREATE TABLE AS.
                    self.set_result(&[])
                } else {
                    self.set_result(&target_list)
                }
            }
            FlatStatement::Declare(d) => self.declare(*d),
            FlatStatement::CreateMaterializedView(m) => self.create_matview(*m),
            FlatStatement::Insert(i) => {
                let returning = self.insert(*i)?;
                self.set_result(&returning)
            }
            FlatStatement::Update(u) => {
                let returning = self.update(*u)?;
                self.set_result(&returning)
            }
            FlatStatement::Delete(d) => {
                let returning = self.delete(*d)?;
                self.set_result(&returning)
            }
            FlatStatement::Explain(e) => self.explain(*e),
            FlatStatement::With(w) => self.with_statement(*w),
            #[cfg(feature = "since-pg15")]
            FlatStatement::Merge(m) => {
                let returning = self.merge(*m)?;
                self.set_result(&returning)
            }
            // transformStmt rewrites these at parse analysis.
            FlatStatement::Call(c) => self.call_statement(*c),
            FlatStatement::CreateTable(t) => match self.get(self.get(*t).body) {
                FlatCreateTableBody::AsQuery(a) => self.create_table_as(self.get(*a).source),
                _ => self.utility(),
            },
            // Describe answers these from the Session.
            FlatStatement::Execute(e) => self.execute_statement(*e),
            FlatStatement::Fetch(f) => self.fetch_statement(*f),
            FlatStatement::Move(m) => self.move_statement(*m),
            FlatStatement::Show(s) => self.show_statement(*s),
            // Utility Statements that change the Session. Each answers with
            // an empty Result Descriptor and reports its Session Effect.
            FlatStatement::Prepare(p) => {
                self.utility()?;
                self.prepare_effect(*p);
                Ok(())
            }
            FlatStatement::Deallocate(d) => {
                self.utility()?;
                self.deallocate_effect(*d)
            }
            FlatStatement::Close(c) => {
                self.utility()?;
                self.close_effect(*c)
            }
            FlatStatement::Set(v) => {
                self.utility()?;
                self.set_effect(*v)
            }
            FlatStatement::Commit(_) | FlatStatement::End(_) | FlatStatement::Abort(_) => {
                self.utility()?;
                self.analysis.session_effect = Some(SessionEffect::TransactionEnd);
                Ok(())
            }
            FlatStatement::Rollback(r) => {
                self.utility()?;
                if self.rollback_ends_transaction(*r) {
                    self.analysis.session_effect = Some(SessionEffect::TransactionEnd);
                }
                Ok(())
            }
            // Utility Statements with a query inside them, which parse
            // analysis leaves for execution: CREATE VIEW, CREATE RULE,
            // PREPARE and COPY (query) among them.
            _ => self.utility(),
        }
    }

    /// One query, at any level: the SELECT with its ORDER BY and LIMIT,
    /// looking through parentheses. `extra_order_by` and `extra_limit` are
    /// clauses written outside the parentheses, which `insertSelectOptions`
    /// pushes into the SELECT.
    fn query_body(
        &mut self,
        body: Handle<FlatQueryBody>,
        extra_order_by: Option<Handle<FlatOrderByClause>>,
        extra_limit: Option<Handle<FlatLimitOffsetClause>>,
        extra_locking: Vec<Handle<FlatForUpdateClause>>,
    ) -> Result<Vec<TargetEntry>> {
        let body = self.get(body);
        // The LIMIT and locking clauses, in either written order; FOR READ
        // ONLY is no locking at all.
        let (own_limit, own_locking) = self.limit_locking(body.limit_locking);
        // insertSelectOptions appends the outer locking clauses to the
        // inner ones.
        let mut locking = own_locking;
        locking.extend(extra_locking);
        let order_by = match (body.order_by, extra_order_by) {
            (Some(_), Some(extra)) => {
                return Err(
                    Diagnostic::new("42601", "multiple ORDER BY clauses not allowed")
                        .at(self.span(extra).start)
                        .into(),
                );
            }
            (inner, extra) => inner.or(extra),
        };
        let limit_offset = match (own_limit, extra_limit) {
            (Some(_), Some(extra)) => {
                return Err(
                    Diagnostic::new("42601", "multiple LIMIT clauses not allowed")
                        .at(self.span(extra).start)
                        .into(),
                );
            }
            (inner, extra) => inner.or(extra),
        };
        // insertSelectOptions: WITH TIES needs an ORDER BY and no SKIP LOCKED.
        if let Some(lo) = limit_offset
            && let Some(limit) = match self.get(lo) {
                FlatLimitOffsetClause::LimitOffset(x) => Some(self.get(*x).limit),
                FlatLimitOffsetClause::OffsetLimit(x) => self.get(*x).limit,
            }
            && let FlatLimitingClause::FetchFirst(f) = self.get(limit)
            && self.fetch_with_ties(*f)
        {
            if order_by.is_none() {
                return Err(Diagnostic::new(
                    "42601",
                    "WITH TIES cannot be specified without ORDER BY clause",
                )
                .into());
            }
            for &l in &locking {
                if self
                    .get(l)
                    .wait
                    .is_some_and(|w| matches!(self.get(w), FlatForUpdateWait::SkipLocked))
                {
                    return Err(Diagnostic::new(
                        "42601",
                        "SKIP LOCKED and WITH TIES options cannot be used together",
                    )
                    .into());
                }
            }
        }
        match self.get(body.clause) {
            FlatSelectClause::Parens(p) => {
                let inner = self.get(self.get(*p).inner);
                if let Some(with) = inner.with {
                    self.with_clause(with)?;
                }
                self.query_body(inner.body, order_by, limit_offset, locking)
            }
            FlatSelectClause::Select(select) => {
                let select = *select;
                let target_list = self.select(select, order_by, limit_offset, &locking)?;
                self.analysis
                    .target_lists
                    .insert(select, target_list.clone());
                Ok(target_list)
            }
            FlatSelectClause::Values(v) => self.values_query(*v, order_by, limit_offset, &locking),
            FlatSelectClause::Table(t) => self.table_query(*t, order_by, limit_offset, &locking),
            FlatSelectClause::Union(..)
            | FlatSelectClause::Except(..)
            | FlatSelectClause::Intersect(..) => {
                self.set_operation(body.clause, order_by, limit_offset, &locking)
            }
        }
    }

    /// `parse_sub_analyze`: one query level below the current one, which
    /// sees the current level's names as outer references.
    fn subquery(
        &mut self,
        scope: &Scope,
        sub: Handle<FlatSubquery>,
        lateral: bool,
    ) -> Result<Vec<TargetEntry>> {
        self.subquery_with(scope, sub, lateral, true)
    }

    /// `parse_sub_analyze` with `p_resolve_unknowns` chosen by the caller.
    fn subquery_with(
        &mut self,
        scope: &Scope,
        sub: Handle<FlatSubquery>,
        lateral: bool,
        resolve_unknowns: bool,
    ) -> Result<Vec<TargetEntry>> {
        let sub = self.get(sub);
        self.sub_level(scope, lateral, resolve_unknowns, |w| {
            if let Some(with) = sub.with {
                w.with_clause(with)?;
            }
            w.query_body(sub.body, None, None, Vec::new())
        })
    }

    /// One query level below the current one: `scope` becomes an outer
    /// level, the per-level state is fresh, and `f` analyses the query.
    fn sub_level<F>(
        &mut self,
        scope: &Scope,
        lateral: bool,
        resolve_unknowns: bool,
        f: F,
    ) -> Result<Vec<TargetEntry>>
    where
        F: FnOnce(&mut Self) -> Result<Vec<TargetEntry>>,
    {
        let mut snapshot = scope.clone();
        snapshot.lateral_active = lateral;
        self.outer.push(snapshot);
        self.outer_kinds.push(self.expr_kind);
        let saved = (
            self.expr_kind,
            (self.has_aggs, self.has_target_srfs),
            (
                std::mem::take(&mut self.agg_locations),
                std::mem::take(&mut self.grouping_calls),
            ),
            self.in_agg_args,
            self.resolve_unknowns,
            self.allow_into,
            self.set_op_member,
        );
        let saved_windows = (
            std::mem::take(&mut self.windows),
            self.has_window_funcs,
            std::mem::take(&mut self.window_locations),
        );
        let saved_srf = self.last_srf.take();
        let saved_collate = (
            std::mem::take(&mut self.clause_exprs),
            std::mem::replace(&mut self.expr_depth, 0),
            std::mem::take(&mut self.extra_sortgroup_refs),
        );
        self.has_aggs = false;
        self.has_target_srfs = false;
        self.in_agg_args = 0;
        self.resolve_unknowns = resolve_unknowns;
        self.allow_into = false;
        self.set_op_member = false;
        self.has_window_funcs = false;
        let result = f(self);
        self.outer.pop();
        self.outer_kinds.pop();
        (
            self.expr_kind,
            (self.has_aggs, self.has_target_srfs),
            (self.agg_locations, self.grouping_calls),
            self.in_agg_args,
            self.resolve_unknowns,
            self.allow_into,
            self.set_op_member,
        ) = saved;
        (self.windows, self.has_window_funcs, self.window_locations) = saved_windows;
        self.last_srf = saved_srf;
        (
            self.clause_exprs,
            self.expr_depth,
            self.extra_sortgroup_refs,
        ) = saved_collate;
        let here = self.level();
        if self.outer_agg_levels.contains(&here) {
            self.has_aggs = true;
        }
        self.outer_agg_levels.retain(|&l| l < here);
        result
    }

    /// `transformSetOperationStmt` over a set operation tree.
    fn set_operation(
        &mut self,
        clause: Handle<FlatSelectClause>,
        order_by: Option<Handle<FlatOrderByClause>>,
        limit_offset: Option<Handle<FlatLimitOffsetClause>>,
        locking: &[Handle<FlatForUpdateClause>],
    ) -> Result<Vec<TargetEntry>> {
        let mut leaves: Vec<SetLeaf> = Vec::new();
        let tree = self.set_tree(clause, &mut leaves);
        // The set operation's own level: its WITH queries, and nothing else.
        let level_scope = self.level_scope();
        // determineRecursiveColTypes: a recursive WITH query's body ends
        // here, and its columns come from the non-recursive term.
        let recursive = self.recursive_top.take();
        if recursive.is_some() && self.recursion.is_some() {
            let form_ok = matches!(
                tree,
                SetTree::Op {
                    op: SetOpKind::Union,
                    ..
                }
            );
            let plain = |t: &SetTree| match t {
                SetTree::Leaf(i) => self.leaf_is_plain(&leaves[*i]),
                SetTree::Op { .. } => false,
            };
            let sides_leaf = match &tree {
                SetTree::Op { left, right, .. } => (plain(left), plain(right)),
                SetTree::Leaf(_) => (false, false),
            };
            let order_by_at = order_by.map(|o| {
                let clause = self.get(o);
                self.list1(clause.items)
                    .first()
                    .map_or(NO_LOCATION, |i| self.span(self.get(*i).expr).start)
            });
            let mut limit_at = None;
            let mut offset_at = None;
            if let Some(lo) = limit_offset {
                let (limit, offset) = match self.get(lo) {
                    FlatLimitOffsetClause::LimitOffset(x) => {
                        let x = self.get(*x);
                        (Some(x.limit), x.offset)
                    }
                    FlatLimitOffsetClause::OffsetLimit(x) => {
                        let x = self.get(*x);
                        (x.limit, Some(x.offset))
                    }
                };
                limit_at = limit.map(|l| match self.get(l) {
                    FlatLimitingClause::Limit(x) => self.span(self.get(*x).count).start,
                    FlatLimitingClause::FetchFirst(_) => self.span(l).start,
                });
                offset_at = offset.map(|o| match self.get(o) {
                    FlatOffsetClause::Count(c) => self.span(self.get(*c).count).start,
                    FlatOffsetClause::Rows(r) => self.span(self.get(*r).count).start,
                });
            }
            if let Some(state) = self.recursion.as_mut() {
                state.form_ok = form_ok;
                state.order_by = order_by_at;
                state.limit = limit_at;
                state.offset = offset_at;
                state.locking = !locking.is_empty();
                state.sides_leaf = sides_leaf;
            }
        }
        let columns = self.set_node(&tree, &leaves, &level_scope, recursive.is_some())?;
        self.allow_into = false;
        // The result: the leftmost query's names with the merged types.
        // transformSetOperationStmt marks no origins.
        let mut target_list: Vec<TargetEntry> = columns
            .iter()
            .map(|c| TargetEntry {
                collation_state: c.state,
                collation: c.state.resolved(),
                name: c.name.clone(),
                ty: c.ty,
                typmod: c.typmod,
                origin: None,
                junk: false,
                location: c.leftmost_location,
                fields: None,
            })
            .collect();
        let count = target_list.len();
        // ORDER BY sees the result columns only, through a join-like entry.
        let mut scope = level_scope;
        scope.rtable.push(crate::relation::RangeTableEntry {
            kind: RteKind::SetOperation {
                columns: columns.iter().map(|c| (c.ty, c.typmod)).collect(),
                collations: columns.iter().map(|c| c.state.resolved()).collect(),
            },
            alias: None,
            eref_name: "unnamed_join".to_owned(),
            colnames: columns.iter().map(|c| c.name.clone()).collect(),
            location: NO_LOCATION,
            composite_only: false,
        });
        scope.namespace.push(NamespaceItem {
            rel_visible: false,
            ..NamespaceItem::plain(1)
        });
        let mut target_exprs: Vec<Option<Handle<FlatExpr>>> = vec![None; count];
        self.sort_clause(&scope, order_by, &mut target_list, &mut target_exprs)?;
        if target_list.len() != count {
            return Err(Diagnostic::new(
                "0A000",
                "invalid UNION/INTERSECT/EXCEPT ORDER BY clause",
            )
            .detail("Only result column names can be used, not expressions or functions.")
            .hint("Add the expression/function to every SELECT, or move the UNION into a FROM clause.")
            .at(target_list[count].location)
            .into());
        }
        self.limit_clause(&scope, limit_offset)?;
        for &l in locking {
            self.locking_clause(&scope, l, Some("UNION/INTERSECT/EXCEPT"))?;
        }
        self.lock_refusal = Some("UNION/INTERSECT/EXCEPT");
        Ok(target_list)
    }

    /// Whether a set operation member is one query rather than a
    /// parenthesised set operation of its own.
    fn leaf_is_plain(&self, leaf: &SetLeaf) -> bool {
        let SetLeaf::Parens(sub) = leaf else {
            return true;
        };
        let mut body = self.get(*sub).body;
        loop {
            match self.get(self.get(body).clause) {
                FlatSelectClause::Parens(p) => body = self.get(self.get(*p).inner).body,
                FlatSelectClause::Select(_)
                | FlatSelectClause::Values(_)
                | FlatSelectClause::Table(_) => return true,
                FlatSelectClause::Union(..)
                | FlatSelectClause::Except(..)
                | FlatSelectClause::Intersect(..) => return false,
            }
        }
    }

    /// The tree of a set operation clause, with its member queries
    /// collected in order.
    fn set_tree(&self, clause: Handle<FlatSelectClause>, leaves: &mut Vec<SetLeaf>) -> SetTree {
        let leaf = |leaves: &mut Vec<SetLeaf>, l: SetLeaf| {
            leaves.push(l);
            SetTree::Leaf(leaves.len() - 1)
        };
        let all = |q: Option<Handle<FlatSetQuantifier>>| {
            matches!(q.map(|q| self.get(q)), Some(FlatSetQuantifier::All))
        };
        match self.get(clause) {
            FlatSelectClause::Union(l, op, r) => {
                let left = self.set_tree(*l, leaves);
                let right = self.set_tree(*r, leaves);
                SetTree::Op {
                    op: SetOpKind::Union,
                    all: all(self.get(*op).quantifier),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            FlatSelectClause::Except(l, op, r) => {
                let left = self.set_tree(*l, leaves);
                let right = self.set_tree(*r, leaves);
                SetTree::Op {
                    op: SetOpKind::Except,
                    all: all(self.get(*op).quantifier),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            FlatSelectClause::Intersect(l, op, r) => {
                let left = self.set_tree(*l, leaves);
                let right = self.set_tree(*r, leaves);
                SetTree::Op {
                    op: SetOpKind::Intersect,
                    all: all(self.get(*op).quantifier),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            FlatSelectClause::Select(h) => leaf(leaves, SetLeaf::Select(*h)),
            FlatSelectClause::Values(v) => leaf(leaves, SetLeaf::Values(*v)),
            FlatSelectClause::Table(t) => leaf(leaves, SetLeaf::Table(*t)),
            FlatSelectClause::Parens(p) => leaf(leaves, SetLeaf::Parens(self.get(*p).inner)),
        }
    }

    /// `transformSetOperationTree` over one node of the grouped chain.
    /// `top` is the top node of a recursive WITH query's body.
    fn set_node(
        &mut self,
        tree: &SetTree,
        leaves: &[SetLeaf],
        level_scope: &Scope,
        top: bool,
    ) -> Result<Vec<SetColumn>> {
        match tree {
            SetTree::Leaf(i) => self.set_leaf(leaves[*i], *i == 0, level_scope),
            SetTree::Op {
                op,
                all,
                left,
                right,
            } => {
                // checkWellFormedRecursion: where a recursive reference may
                // not appear.
                let saved_context = self.recursion_context;
                if *op == SetOpKind::Intersect && *all {
                    self.recursion_context = RecursionContext::Intersect;
                } else if *op == SetOpKind::Except && *all {
                    self.recursion_context = RecursionContext::Except;
                }
                let l = self.set_node(left, leaves, level_scope, false);
                self.recursion_context = saved_context;
                let l = l?;
                if top {
                    self.fix_recursive_columns(&l)?;
                }
                if *op == SetOpKind::Except {
                    self.recursion_context = RecursionContext::Except;
                } else if *op == SetOpKind::Intersect && *all {
                    self.recursion_context = RecursionContext::Intersect;
                }
                let r = self.set_node(right, leaves, level_scope, false);
                self.recursion_context = saved_context;
                let r = r?;
                let context = match op {
                    SetOpKind::Union => "UNION",
                    SetOpKind::Intersect => "INTERSECT",
                    SetOpKind::Except => "EXCEPT",
                };
                if l.len() != r.len() {
                    let at = r
                        .iter()
                        .map(|c| c.location)
                        .filter(|l| *l != NO_LOCATION)
                        .min()
                        .unwrap_or(NO_LOCATION);
                    return Err(Diagnostic::new(
                        "42601",
                        format!("each {context} query must have the same number of columns"),
                    )
                    .at(at)
                    .into());
                }
                let mut out = Vec::with_capacity(l.len());
                for (lc, rc) in l.iter().zip(&r) {
                    let types = self.types();
                    let rescoltype = match self.coercer().common_type(&[lc.ty, rc.ty]) {
                        Ok(t) => t,
                        Err((p, n)) => {
                            let at = if n == rc.ty { rc.location } else { lc.location };
                            return Err(Diagnostic::new(
                                "42804",
                                format!(
                                    "{context} types {} and {} cannot be matched",
                                    types.format(p),
                                    types.format(n)
                                ),
                            )
                            .at(at)
                            .into());
                        }
                    };
                    for c in [lc, rc] {
                        if !types.is_unknown(c.ty) && c.ty != rescoltype {
                            let typed = Typed {
                                ty: c.ty,
                                typmod: c.typmod,
                                form: Form::Other,
                                location: c.location,
                                returns_set: false,
                            };
                            self.coercer()
                                .coerce_to_common_type(&typed, rescoltype, context)?;
                        }
                    }
                    let typmod =
                        if lc.ty == rescoltype && rc.ty == rescoltype && lc.typmod == rc.typmod {
                            lc.typmod
                        } else {
                            -1
                        };
                    let best = if (types.is_unknown(lc.ty) && !types.is_unknown(rc.ty))
                        || (lc.ty != rescoltype && rc.ty == rescoltype)
                    {
                        rc.location
                    } else {
                        lc.location
                    };
                    if !(*op == SetOpKind::Union && *all)
                        && types.equality_operator(rescoltype).is_none()
                    {
                        return Err(Diagnostic::new(
                            "42883",
                            format!(
                                "could not identify an equality operator for type {}",
                                types.format(rescoltype)
                            ),
                        )
                        .at(best)
                        .into());
                    }
                    // select_common_collation over the two columns, each
                    // with its strength; a UNION ALL takes a conflict.
                    let mut ctx = CollContext::new();
                    for side in [lc, rc] {
                        self.merge_collation_state(
                            side.state.collation,
                            side.state.strength,
                            side.state.location,
                            side.state.collation2,
                            side.state.location2,
                            &mut ctx,
                        )?;
                    }
                    let collation = if ctx.strength == Strength::Conflict {
                        if *op == SetOpKind::Union && *all {
                            None
                        } else {
                            return Err(self.implicit_mismatch(&ctx).into());
                        }
                    } else {
                        ctx.collation
                    };
                    let state = CollContext {
                        collation,
                        strength: if collation.is_some() {
                            Strength::Implicit
                        } else {
                            Strength::None
                        },
                        location: best,
                        collation2: None,
                        location2: NO_LOCATION,
                    };
                    out.push(SetColumn {
                        name: lc.name.clone(),
                        ty: rescoltype,
                        typmod,
                        state,
                        origin: lc.origin,
                        location: best,
                        leftmost_location: lc.leftmost_location,
                    });
                }
                Ok(out)
            }
        }
    }

    /// `determineRecursiveColTypes`: the non-recursive term's columns
    /// become the recursive WITH query's columns, so its recursive term
    /// can refer to it.
    fn fix_recursive_columns(&mut self, left: &[SetColumn]) -> Result<()> {
        let Some(state) = self.recursion.clone() else {
            return Ok(());
        };
        let text = self.types().builtin(Builtin::Text);
        let entry = &self.outer[state.level].ctes[state.index];
        let aliases: Vec<String> = entry.columns.iter().map(|c| c.name.clone()).collect();
        let mut columns = Vec::with_capacity(left.len());
        for (i, c) in left.iter().enumerate() {
            // An unknown column reads as text with the default collation.
            let (ty, typmod, collation) = if self.types().is_unknown(c.ty) {
                (text, -1, self.type_collation(text))
            } else {
                (c.ty, c.typmod, c.state.resolved())
            };
            columns.push(CteColumn {
                collation,
                name: aliases.get(i).cloned().unwrap_or_else(|| c.name.clone()),
                ty,
                typmod,
                origin: c.origin,
                fields: None,
            });
        }
        if columns.len() < aliases.len() {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "WITH query \"{}\" has {} columns available but {} columns specified",
                    state.name,
                    columns.len(),
                    aliases.len()
                ),
            )
            .at(state.location)
            .into());
        }
        let entry = &mut self.outer[state.level].ctes[state.index];
        entry.columns = columns.clone();
        entry.columns_known = true;
        if let Some(state) = self.recursion.as_mut() {
            state.fixed = Some(columns);
            state.term_depth = self.subquery_depth;
        }
        Ok(())
    }

    /// One member query of a set operation, analysed one level down with
    /// its unknown literals kept.
    fn set_leaf(
        &mut self,
        leaf: SetLeaf,
        first: bool,
        level_scope: &Scope,
    ) -> Result<Vec<SetColumn>> {
        let allow_into = first && self.allow_into;
        let scope = level_scope.clone();
        let entries = self.sub_level(&scope, false, false, |w| {
            w.allow_into = allow_into;
            w.set_op_member = !first;
            match leaf {
                SetLeaf::Select(h) => w.select(h, None, None, &[]),
                SetLeaf::Values(v) => w.values_query(v, None, None, &[]),
                SetLeaf::Table(t) => w.table_query(t, None, None, &[]),
                SetLeaf::Parens(sub) => {
                    let sub = w.get(sub);
                    if let Some(with) = sub.with {
                        w.with_clause(with)?;
                    }
                    w.query_body(sub.body, None, None, Vec::new())
                }
            }
        })?;
        Ok(entries
            .into_iter()
            .filter(|t| !t.junk)
            .map(|t| SetColumn {
                name: t.name,
                ty: t.ty,
                typmod: t.typmod,
                state: t.collation_state,
                origin: t.origin,
                location: t.location,
                leftmost_location: t.location,
            })
            .collect())
    }

    /// `transformValuesClause`.
    fn values_query(
        &mut self,
        v: Handle<FlatValuesBody>,
        order_by: Option<Handle<FlatOrderByClause>>,
        limit_offset: Option<Handle<FlatLimitOffsetClause>>,
        locking: &[Handle<FlatForUpdateClause>],
    ) -> Result<Vec<TargetEntry>> {
        let mut scope = self.level_scope();
        let saved_kind = self.expr_kind;
        self.expr_kind = ExprKind::Values;
        let mut columns: Vec<Vec<(Handle<FlatExpr>, Typed)>> = Vec::new();
        for row in self.list1(self.get(v).rows) {
            let written = self.list1(self.get(*row).values).to_vec();
            // transformExpressionList: `t.*` expands to the columns.
            let mut exprs: Vec<(Handle<FlatExpr>, Option<TargetEntry>)> = Vec::new();
            for e in written {
                if let FlatExpr::QualRef(q_handle) = self.get(e) {
                    let q_handle = *q_handle;
                    if self.qref_star(q_handle) {
                        for entry in self.expand_qualified_star(&scope, q_handle)? {
                            exprs.push((e, Some(entry)));
                        }
                        continue;
                    }
                }
                exprs.push((e, None));
            }
            if !columns.is_empty() && columns.len() != exprs.len() {
                let at = exprs
                    .first()
                    .map_or(self.span(*row).start, |(e, _)| self.span(*e).start);
                return Err(
                    Diagnostic::new("42601", "VALUES lists must all be the same length")
                        .at(at)
                        .into(),
                );
            }
            if columns.is_empty() {
                columns.resize_with(exprs.len(), Vec::new);
            }
            for (i, (e, expanded)) in exprs.iter().enumerate() {
                let t = match expanded {
                    Some(entry) => Typed {
                        ty: entry.ty,
                        typmod: entry.typmod,
                        form: Form::Var,
                        location: entry.location,
                        returns_set: false,
                    },
                    None => self.expr(&scope, *e)?,
                };
                columns[i].push((*e, t));
            }
        }
        self.expr_kind = saved_kind;
        let mut coltypes: Vec<(TypeRef, Typmod)> = Vec::with_capacity(columns.len());
        let mut colcollations: Vec<Option<pgtk_catalog::Handle<Collation>>> =
            Vec::with_capacity(columns.len());
        for col in &columns {
            let types_of: Vec<TypeRef> = col.iter().map(|(_, t)| t.ty).collect();
            let coltype = match self.coercer().common_type(&types_of) {
                Ok(t) => t,
                Err((p, n)) => {
                    let at = col
                        .iter()
                        .find(|(_, t)| t.ty == n)
                        .map_or(NO_LOCATION, |(_, t)| t.location);
                    return Err(Diagnostic::new(
                        "42804",
                        format!(
                            "VALUES types {} and {} cannot be matched",
                            self.types().format(p),
                            self.types().format(n)
                        ),
                    )
                    .at(at)
                    .into());
                }
            };
            for (e, t) in col {
                if t.ty != coltype {
                    let steps = self.coercer().coerce_to_common_type(t, coltype, "VALUES")?;
                    self.record_coercions(*e, steps);
                }
            }
            let typmod = if col.iter().all(|(_, t)| t.ty == coltype)
                && col.windows(2).all(|w| w[0].1.typmod == w[1].1.typmod)
            {
                col.first().map_or(-1, |(_, t)| t.typmod)
            } else {
                -1
            };
            coltypes.push((coltype, typmod));
            let exprs: Vec<Handle<FlatExpr>> = col.iter().map(|(e, _)| *e).collect();
            colcollations.push(self.select_common_collation(&scope, &exprs, true)?);
        }
        let count = coltypes.len();
        scope.rtable.push(crate::relation::RangeTableEntry {
            kind: RteKind::Values {
                columns: coltypes.clone(),
                collations: colcollations.clone(),
            },
            alias: None,
            eref_name: "*VALUES*".to_owned(),
            colnames: (1..=count).map(|i| format!("column{i}")).collect(),
            location: self.span(v).start,
            composite_only: false,
        });
        scope.namespace.push(NamespaceItem::plain(1));
        let mut target_list: Vec<TargetEntry> = coltypes
            .iter()
            .enumerate()
            .map(|(i, (ty, typmod))| TargetEntry {
                collation_state: CollContext::implicit(
                    colcollations.get(i).copied().flatten(),
                    NO_LOCATION,
                ),
                collation: colcollations.get(i).copied().flatten(),
                name: format!("column{}", i + 1),
                ty: *ty,
                typmod: *typmod,
                origin: None,
                junk: false,
                location: NO_LOCATION,
                fields: None,
            })
            .collect();
        let mut target_exprs: Vec<Option<Handle<FlatExpr>>> = vec![None; count];
        self.sort_clause(&scope, order_by, &mut target_list, &mut target_exprs)?;
        self.limit_clause(&scope, limit_offset)?;
        for &l in locking {
            self.locking_clause(&scope, l, None)?;
        }
        self.lock_refusal = None;
        Ok(target_list)
    }

    /// `TABLE name`: `SELECT * FROM name`.
    fn table_query(
        &mut self,
        t: Handle<FlatTableStmt>,
        order_by: Option<Handle<FlatOrderByClause>>,
        limit_offset: Option<Handle<FlatLimitOffsetClause>>,
        locking: &[Handle<FlatForUpdateClause>],
    ) -> Result<Vec<TargetEntry>> {
        let mut scope = self.level_scope();
        let (name, _) = self.relation_expr(self.get(t).relation);
        let location = self.span(name).start;
        let (schema, relname) = self.qualified_name(name, location)?;
        let rt_index = self.relations.add_range_table_entry(
            &mut scope,
            schema.as_deref(),
            &relname,
            None,
            &[],
            true,
            location,
        )?;
        scope.namespace.push(NamespaceItem::plain(rt_index));
        self.expr_kind = ExprKind::SelectTarget;
        let mut target_list = self.expand_all_tables(&scope, location)?;
        for e in &mut target_list {
            e.location = location;
        }
        let mut target_exprs: Vec<Option<Handle<FlatExpr>>> = vec![None; target_list.len()];
        self.sort_clause(&scope, order_by, &mut target_list, &mut target_exprs)?;
        self.limit_clause(&scope, limit_offset)?;
        for &l in locking {
            self.locking_clause(&scope, l, None)?;
        }
        self.lock_refusal = None;
        Ok(target_list)
    }

    /// The name a SHOW or a SET writes, folded and joined with dots.
    fn parameter_name(&self, q: Handle<FlatAnyName>) -> Result<String> {
        Ok(self.any_name_parts(q)?.join("."))
    }

    /// The folded parts of a `qualified_name`, the first one first.
    fn qualified_name_parts(&self, qn: Handle<FlatQualifiedName>) -> Result<Vec<String>> {
        let qn = self.get(qn);
        let (first, _) = self.col_id(qn.first)?;
        let mut parts = vec![first];
        if let Some(rest) = qn.rest {
            let rest = self.get(rest);
            parts.push(self.col_label(self.get(rest.second).name)?);
            if let Some(third) = rest.third {
                parts.push(self.col_label(self.get(third).name)?);
            }
        }
        Ok(parts)
    }

    /// The folded parts of an `any_name`, the first one first.
    fn any_name_parts(&self, name: Handle<FlatAnyName>) -> Result<Vec<String>> {
        match self.get(name) {
            FlatAnyName::Short(qn) => self.qualified_name_parts(*qn),
            FlatAnyName::Long(long) => {
                let long = self.get(*long);
                let (first, _) = self.col_id(long.first)?;
                let mut parts = vec![
                    first,
                    self.col_label(self.get(long.second).name)?,
                    self.col_label(self.get(long.third).name)?,
                ];
                for part in self.list1(long.more) {
                    parts.push(self.col_label(self.get(*part).name)?);
                }
                Ok(parts)
            }
        }
    }

    /// An identifier token, folded.
    fn ident(&self, handle: Handle<pgtk_sql::tokens::literal::FlatIdent>) -> Result<String> {
        let pgtk_sql::tokens::literal::FlatIdent::Text(t) = self.get(handle);
        let span = self.text_span(*t);
        self.fold(self.text(*t), span.start)
    }

    /// `UtilityTupleDescriptor` for an EXECUTE: the Result Descriptor of
    /// the prepared statement the Session holds. The argument expressions
    /// are not analysed and their count is not checked, because PostgreSQL
    /// does both only when the Statement runs.
    ///
    /// A name the Session does not hold answers with no columns, as
    /// `UtilityTupleDescriptor` does: raising `26000` there is "not our
    /// business", and PostgreSQL raises it when the Statement runs.
    fn execute_statement(&mut self, e: Handle<FlatExecuteStmt>) -> Result<()> {
        let stmt = self.get(e);
        let (name, _) = self.alias_name(stmt.name)?;
        let session = self.session;
        let Some(prepared) = session.prepared(&name) else {
            return self.utility();
        };
        let columns = self.revalidate(prepared)?;
        self.analysis.result = ResultDescriptor {
            columns,
            params: self.finish_params()?,
        };
        Ok(())
    }

    /// `RevalidateCachedQuery`: the prepared query is analysed again
    /// against the Catalog as it is now, so an EXECUTE answers with the
    /// Diagnostic the query raises now, and with the tables the columns
    /// come from now. The plan cache refuses a prepared statement whose
    /// result columns changed, because a client is not ready for that.
    ///
    /// A prepared statement the caller built without the text of its
    /// PREPARE answers from the Result Descriptor it holds.
    fn revalidate(&self, prepared: &PreparedStatement) -> Result<Vec<ResultColumn>> {
        if prepared.source.is_empty() {
            return Ok(prepared.result.columns.clone());
        }
        let analyzer = Analyzer::new(self.catalog(), self.settings.clone());
        let result = match analyzer.prepared_query(&prepared.source) {
            Ok(result) => result,
            // The Diagnostic is about the text of the PREPARE, so it keeps
            // the position it has there and loses the spans.
            Err(AnalyzeError::Diagnostic(mut d)) => {
                d.location = None;
                d.span = None;
                d.labels.clear();
                return Err(AnalyzeError::Diagnostic(d));
            }
            Err(other) => return Err(other),
        };
        let same = result.columns.len() == prepared.result.columns.len()
            && result
                .columns
                .iter()
                .zip(&prepared.result.columns)
                .all(|(a, b)| a.name == b.name && a.ty == b.ty && a.typmod == b.typmod);
        if !same {
            return Err(Diagnostic::new("0A000", "cached plan must not change result type").into());
        }
        Ok(result.columns)
    }

    /// `UtilityTupleDescriptor` for a FETCH: the Result Descriptor of the
    /// cursor the Session holds. The direction is not checked, because
    /// PostgreSQL checks it only when the Statement runs.
    fn fetch_statement(&mut self, f: Handle<FlatFetchStmt>) -> Result<()> {
        let cursor = self.get(f).cursor;
        let columns = self.cursor_columns(cursor)?;
        self.analysis.result = ResultDescriptor {
            columns,
            params: self.finish_params()?,
        };
        Ok(())
    }

    /// A MOVE reads the same cursor as a FETCH and answers with no columns.
    fn move_statement(&mut self, m: Handle<FlatMoveStmt>) -> Result<()> {
        let cursor = self.get(m).cursor;
        self.cursor_columns(cursor)?;
        self.utility()
    }

    /// The result columns of the cursor a FETCH or a MOVE names.
    ///
    /// A name the Session does not hold answers with no columns, as
    /// `UtilityTupleDescriptor` does: raising `34000` there is "not our
    /// business", and PostgreSQL raises it when the Statement runs.
    fn cursor_columns(&mut self, cursor: Handle<FlatColId>) -> Result<Vec<ResultColumn>> {
        let (name, _) = self.col_id(cursor)?;
        let session = self.session;
        Ok(session
            .cursor(&name)
            .map_or_else(Vec::new, |c| c.result.columns.clone()))
    }

    /// The query of a PREPARE, analysed with the parameter types the
    /// PREPARE declares. `transformStmt` does this when the Statement
    /// runs, and the plan cache does it again whenever it revalidates.
    fn prepared_query(&mut self) -> Result<()> {
        let FlatStatement::Prepare(p) = self.get(self.ast.root()) else {
            return Err(unsupported("not a PREPARE statement", None));
        };
        let FlatPrepareStmtBody::Standard(body) = self.get(self.get(*p).body) else {
            return Err(unsupported("PREPARE TRANSACTION has no query", None));
        };
        let body = self.get(*body);
        let mut declared: Vec<Option<TypeRef>> = Vec::new();
        if let Some(types) = body.types {
            for cast_type in self.list1(self.get(self.get(types).types).types) {
                let type_name = self.type_name(*cast_type)?;
                let (ty, _) = self.typenames.resolve(&type_name)?;
                declared.push(Some(ty));
            }
        }
        *self.coercer().params.borrow_mut() = declared;
        match self.get(body.body) {
            FlatPreparableStmt::Query(b) => {
                let target_list = self.query_body(*b, None, None, Vec::new())?;
                self.set_result(&target_list)
            }
            FlatPreparableStmt::With(w) => self.with_statement(*w),
            FlatPreparableStmt::Insert(i) => {
                let returning = self.insert(*i)?;
                self.set_result(&returning)
            }
            FlatPreparableStmt::Update(u) => {
                let returning = self.update(*u)?;
                self.set_result(&returning)
            }
            FlatPreparableStmt::Delete(d) => {
                let returning = self.delete(*d)?;
                self.set_result(&returning)
            }
            #[cfg(feature = "since-pg15")]
            FlatPreparableStmt::Merge(m) => {
                let returning = self.merge(*m)?;
                self.set_result(&returning)
            }
        }
    }

    /// The Session Effect of a PREPARE: the prepared statement it adds.
    ///
    /// PREPARE is a Utility Statement, so its own Analysis leaves the query
    /// unanalysed. The Effect needs the query's Result Descriptor, so the
    /// query is analysed here on the side, with the declared parameter
    /// types. A side analysis that fails reports no Effect at all, because
    /// PostgreSQL would fail when the Statement runs and the prepared
    /// statement would not exist.
    fn prepare_effect(&mut self, handle: Handle<FlatPrepareStmt>) {
        let FlatPrepareStmtBody::Standard(body) = self.get(self.get(handle).body) else {
            return;
        };
        let Ok((name, _)) = self.alias_name(self.get(*body).name) else {
            return;
        };
        // The Session keeps the text of the PREPARE, because a Describe of
        // an EXECUTE analyses the query again.
        let span = self.span(self.ast.root());
        let source = match self.source.get(span) {
            Some(text) if !text.is_empty() => text,
            _ => self.source,
        };
        let analyzer = Analyzer::new(self.catalog(), self.settings.clone());
        let Ok(result) = analyzer.prepared_query(source) else {
            return;
        };
        self.analysis.session_effect = Some(SessionEffect::Prepare(PreparedStatement {
            name,
            param_types: result.params.clone(),
            result,
            source: source.to_owned(),
        }));
    }

    /// The Session Effect of a DEALLOCATE.
    fn deallocate_effect(&mut self, handle: Handle<FlatDeallocateStmt>) -> Result<()> {
        let effect = match self.get(self.get(handle).target) {
            FlatDeallocateTarget::All => SessionEffect::DeallocateAll,
            FlatDeallocateTarget::Name(n) => SessionEffect::Deallocate(self.ident(*n)?),
        };
        self.analysis.session_effect = Some(effect);
        Ok(())
    }

    /// The Session Effect of a CLOSE.
    fn close_effect(&mut self, handle: Handle<FlatCloseStmt>) -> Result<()> {
        let effect = match self.get(self.get(handle).target) {
            FlatCloseTarget::All => SessionEffect::CloseAll,
            FlatCloseTarget::Cursor(c) => SessionEffect::Close(self.ident(*c)?),
        };
        self.analysis.session_effect = Some(effect);
        Ok(())
    }

    /// The Session Effect of a SET: a placeholder, when the parameter name
    /// has a dot. SET LOCAL counts too, because SHOW answers from the
    /// placeholder it makes until the transaction ends.
    fn set_effect(&mut self, handle: Handle<FlatVariableSetStmt>) -> Result<()> {
        let rest = match self.get(handle) {
            FlatVariableSetStmt::Unscoped(x) => self.get(*x).rest,
            FlatVariableSetStmt::Local(x) => self.get(*x).rest,
            FlatVariableSetStmt::Session(x) => self.get(*x).rest,
        };
        let FlatVariableSetRest::Generic(g) = self.get(rest) else {
            return Ok(());
        };
        let g: &FlatGenericSetRest = self.get(*g);
        let name = self.parameter_name(g.param)?;
        if !name.contains('.') {
            return Ok(());
        }
        let value = match self.get(g.value) {
            // A placeholder's default is the empty string.
            FlatGenericSetValue::Default => String::new(),
            FlatGenericSetValue::List(list) => {
                let values = self.list1(self.get(*list).0);
                let from = self.span(values[0]).start;
                let to = self.span(values[values.len() - 1]).end;
                self.source.get(from..to).unwrap_or_default().to_owned()
            }
        };
        self.analysis.session_effect = Some(SessionEffect::Set(Placeholder { name, value }));
        Ok(())
    }

    /// Does this ROLLBACK end the transaction? ROLLBACK TO SAVEPOINT does
    /// not, so it closes no cursor.
    fn rollback_ends_transaction(&self, handle: Handle<FlatRollbackStmt>) -> bool {
        match self.get(handle).body.map(|b| self.get(b)) {
            None | Some(FlatRollbackBody::Prepared(_)) | Some(FlatRollbackBody::Chain(_)) => true,
            Some(FlatRollbackBody::To(_)) => false,
            Some(FlatRollbackBody::WithWork(w)) => !matches!(
                self.get(*w).after.map(|a| self.get(a)),
                Some(FlatRollbackAfterWork::To(_))
            ),
        }
    }

    /// `GetPGVariableResultDesc`: SHOW ALL is three text columns, SHOW
    /// name one text column named with the parameter's canonical
    /// spelling.
    fn show_statement(&mut self, s: Handle<FlatShowStmt>) -> Result<()> {
        let text = self.types().builtin(Builtin::Text);
        let column = |name: String| TargetEntry {
            collation_state: CollContext::new(),
            collation: None,
            name,
            ty: text,
            typmod: -1,
            origin: None,
            junk: false,
            location: 0,
            fields: None,
        };
        let name = match self.get(self.get(s).target) {
            FlatShowTarget::All => {
                let columns: Vec<TargetEntry> = ["name", "setting", "description"]
                    .into_iter()
                    .map(|n| column(n.to_owned()))
                    .collect();
                return self.set_result(&columns);
            }
            FlatShowTarget::TransactionIsolationLevel => "transaction_isolation".to_owned(),
            FlatShowTarget::SessionAuthorization => "session_authorization".to_owned(),
            FlatShowTarget::TimeZone => "timezone".to_owned(),
            FlatShowTarget::Param(q) => self.parameter_name(*q)?,
        };
        // A placeholder (`custom.x`) exists only when the Session set it.
        if name.contains('.') {
            let session = self.session;
            let Some(placeholder) = session.placeholder(&name) else {
                return Err(Diagnostic::new(
                    "42704",
                    format!("unrecognized configuration parameter \"{name}\""),
                )
                .into());
            };
            let columns = vec![column(placeholder.name.clone())];
            return self.set_result(&columns);
        }
        // GUC_NO_SHOW_ALL parameters are absent from pg_settings, so
        // from the Catalog, but SHOW answers them.
        const HIDDEN: [&str; 6] = [
            "default_with_oids",
            "is_superuser",
            "role",
            "seed",
            "session_authorization",
            "ssl_renegotiation_limit",
        ];
        let lower = name.to_ascii_lowercase();
        let canonical = match self.catalog().parameter(&name) {
            Some(parameter) => parameter.name.clone(),
            None if HIDDEN.contains(&lower.as_str()) => lower,
            None => {
                return Err(Diagnostic::new(
                    "42704",
                    format!("unrecognized configuration parameter \"{name}\""),
                )
                .into());
            }
        };
        let columns = vec![column(canonical)];
        self.set_result(&columns)
    }

    /// `transformCallStmt`: the procedure and its arguments; the result
    /// columns are its OUT arguments, as `CallStmtResultDesc` describes.
    fn call_statement(&mut self, c: Handle<FlatCallStmt>) -> Result<()> {
        let c = self.get(c);
        let app = self.get(c.call);
        let location = self.span(app.name).start;
        let name = self.func_call_name(app.name, location)?;
        let application = self.get(app.application);
        let scope = self.level_scope();
        self.expr_kind = ExprKind::CallArgument;
        self.call(
            &scope,
            CallSite::Statement,
            name,
            application.body,
            None,
            None,
            None,
            location,
        )?;
        // The argument count and names, for the parameter order.
        let mut nargs = 0;
        let mut argnames: Vec<String> = Vec::new();
        let mut arg_handles: Vec<Handle<FlatExpr>> = Vec::new();
        if let Some(body) = application.body {
            let (items, _, _) = self.call_body_items(body);
            nargs = items.len();
            for item in items {
                match self.get(item) {
                    FlatFuncArg::Plain(e) => arg_handles.push(*e),
                    FlatFuncArg::Named(n) => {
                        let n = self.get(*n);
                        let Flattype_function_name::Text(t) = self.get(n.name);
                        let span = self.text_span(*t);
                        argnames.push(self.fold(self.text(*t), span.start)?);
                        arg_handles.push(n.value);
                    }
                }
            }
        }
        let Some(resolution) = self.statement_function.clone() else {
            return Err(unsupported("CALL without a procedure", None));
        };
        let proc = &self.catalog()[resolution.function];
        // CallStmtResultDesc: a polymorphic output column takes the type
        // of its argument, which expand_function_arguments put in
        // parameter order.
        let params: Vec<&pgtk_catalog::Argument> = proc
            .arguments
            .iter()
            .filter(|a| a.mode != ArgMode::Table)
            .collect();
        let argnumbers = if argnames.is_empty() {
            None
        } else {
            FuncResolver::match_named_call(proc, nargs, &argnames, params.len(), true)
        };
        let mut columns = Vec::new();
        for (i, (j, a)) in params
            .iter()
            .enumerate()
            .filter(|(_, a)| matches!(a.mode, ArgMode::Out | ArgMode::InOut))
            .enumerate()
        {
            let position = argnumbers
                .as_ref()
                .map_or(Some(j), |numbers| numbers.iter().position(|&n| n == j));
            // A polymorphic parameter's argument keeps its own type when
            // that is known; an unknown one took the resolved type.
            let ty = position
                .and_then(|p| {
                    let declared = *resolution.declared.get(p)?;
                    if self.types().is_polymorphic(declared)
                        && let Some(t) = arg_handles
                            .get(p)
                            .and_then(|h| self.analysis.expr_types.get(*h))
                    {
                        Some(t.ty)
                    } else {
                        Some(declared)
                    }
                })
                .unwrap_or(a.ty);
            columns.push(TargetEntry {
                collation_state: CollContext::new(),
                collation: None,
                name: a
                    .name
                    .as_ref()
                    .map_or_else(|| format!("column{}", i + 1), ToString::to_string),
                ty,
                typmod: -1,
                origin: None,
                junk: false,
                location,
                fields: None,
            });
        }
        self.set_result(&columns)
    }

    /// `transformWithClause`: each WITH query is analysed one level down
    /// and becomes visible to the queries after it, or to all of them
    /// under RECURSIVE.
    fn with_clause(&mut self, w: Handle<FlatWithClause>) -> Result<()> {
        let clause = self.get(w);
        let recursive = matches!(self.get(clause.head), FlatWithClauseHead::Recursive(_));
        let defs: Vec<Handle<FlatCteDefinition>> = self.list1(clause.ctes).to_vec();
        let mut entries: Vec<CteEntry> = Vec::new();
        let mut aliases: Vec<Vec<String>> = Vec::new();
        for (i, d) in defs.iter().enumerate() {
            let def = self.get(*d);
            let (name, location) = self.col_id(def.name)?;
            for later in &defs[i + 1..] {
                let later = self.get(*later);
                let (other, at) = self.col_id(later.name)?;
                if other == name {
                    return Err(Diagnostic::new(
                        "42712",
                        format!("WITH query name \"{other}\" specified more than once"),
                    )
                    .at(at)
                    .into());
                }
            }
            let mut cols = Vec::new();
            if let Some(c) = def.columns {
                for a in self.list1(self.get(c).0) {
                    cols.push(self.alias_name(*a)?.0);
                }
            }
            let modifying = match self.get(def.query) {
                FlatPreparableStmt::Query(_) => false,
                FlatPreparableStmt::With(ws) => {
                    !matches!(self.get(self.get(*ws).body), FlatWithBody::Query(_))
                }
                _ => true,
            };
            entries.push(CteEntry {
                name,
                columns: cols
                    .iter()
                    .map(|n| CteColumn {
                        collation: None,
                        name: n.clone(),
                        ty: self.types().builtin(Builtin::Text),
                        typmod: -1,
                        origin: None,
                        fields: None,
                    })
                    .collect(),
                columns_known: false,
                analysing: false,
                modifying,
                has_returning: false,
                location,
                hidden: 0,
            });
            aliases.push(cols);
        }
        // Under RECURSIVE every query sees every other; otherwise only the
        // earlier ones.
        let mut known: Vec<CteEntry> = std::mem::take(&mut self.pending_ctes);
        let base = known.len();
        if recursive {
            known.extend(entries.iter().cloned());
        }
        let mut items = WithItems {
            done: vec![false; defs.len()],
            in_progress: Vec::new(),
            level: self.level(),
            defs,
            entries,
            aliases,
            known,
            base,
            recursive,
        };
        for i in 0..items.defs.len() {
            self.with_item_ordered(&mut items, i)?;
        }
        self.pending_ctes = items.known;
        Ok(())
    }

    /// `TopologicalSort`: a RECURSIVE WITH item that refers to a later
    /// item is analysed after it; a cycle of such references is refused.
    fn with_item_ordered(&mut self, items: &mut WithItems, i: usize) -> Result<()> {
        if items.done[i] {
            return Ok(());
        }
        if items.in_progress.contains(&i) {
            let first = items.in_progress[0];
            return Err(Diagnostic::new(
                "0A000",
                "mutual recursion between WITH items is not implemented",
            )
            .at(items.entries[first].location)
            .into());
        }
        items.in_progress.push(i);
        loop {
            self.defer_to = None;
            self.pending_ctes = Vec::new();
            let result = self.with_item(items, i);
            let Err(error) = result else {
                break;
            };
            match self.defer_to {
                Some((level, index))
                    if items.recursive
                        && level == items.level
                        && index >= items.base
                        && !items.done[index - items.base] =>
                {
                    self.defer_to = None;
                    self.with_item_ordered(items, index - items.base)?;
                }
                _ => return Err(error),
            }
        }
        items.in_progress.pop();
        items.done[i] = true;
        Ok(())
    }

    /// `analyzeCTE` for one WITH item.
    fn with_item(&mut self, items: &mut WithItems, i: usize) -> Result<()> {
        let d = items.defs[i];
        let def = self.get(d);
        let entry = items.entries[i].clone();
        let entry = &entry;
        let mut with_scope = Scope::new();
        with_scope.ctes = items.known.clone();
        if items.recursive {
            with_scope.ctes[items.base + i].analysing = true;
        }
        let level = self.level();
        let saved_top = self.recursive_top.take();
        let saved_context = self.recursion_context;
        let enclosing = self.recursion.take();
        let pushed = enclosing.is_some();
        if let Some(state) = enclosing {
            self.outer_recursions.push((state, saved_context));
        }
        self.recursion_context = RecursionContext::Ok;
        if items.recursive {
            self.recursion = Some(RecursionState {
                level,
                index: items.base + i,
                name: entry.name.clone(),
                location: entry.location,
                form_ok: false,
                order_by: None,
                offset: None,
                limit: None,
                locking: false,
                fixed: None,
                sides_leaf: (false, false),
                term_depth: 0,
                in_with: false,
                extra_columns: Vec::new(),
                references: 0,
            });
            self.recursive_top = Some((level, items.base + i));
        }
        // analyzeCTE: the CYCLE mark values first, at this level.
        let cycle_mark = match def.cycle {
            Some(c) => Some(self.cycle_mark(&with_scope, c)?),
            None => None,
        };
        // addRangeTableEntryForCTE adds the SEARCH and CYCLE columns to
        // every reference, the self-reference included.
        let mut extra_columns: Vec<CteColumn> = Vec::new();
        if let Some(sc) = def.search {
            let sc = self.get(sc);
            let (seq, _) = self.col_id(sc.set_column)?;
            let depth = matches!(self.get(sc.direction), FlatSearchDirection::Depth);
            extra_columns.push(CteColumn {
                collation: None,
                name: seq,
                ty: self.types().builtin(if depth {
                    Builtin::RecordArray
                } else {
                    Builtin::Record
                }),
                typmod: -1,
                origin: None,
                fields: None,
            });
        }
        if let (Some(cc), Some((mark_ty, mark_typmod))) = (def.cycle, cycle_mark) {
            let cc = self.get(cc);
            let (mark, _) = self.alias_name(self.get(cc.set_column).name)?;
            let (path, _) = self.alias_name(cc.using_column)?;
            extra_columns.push(CteColumn {
                collation: None,
                name: mark,
                ty: mark_ty,
                typmod: mark_typmod,
                origin: None,
                fields: None,
            });
            extra_columns.push(CteColumn {
                collation: None,
                name: path,
                ty: self.types().builtin(Builtin::RecordArray),
                typmod: -1,
                origin: None,
                fields: None,
            });
        }
        let hidden = extra_columns.len();
        if let Some(state) = self.recursion.as_mut() {
            state.extra_columns = extra_columns;
        }
        let query = def.query;
        let result = self.sub_level(&with_scope, false, true, |w| match w.get(query) {
            FlatPreparableStmt::Query(b) => w.query_body(*b, None, None, Vec::new()),
            FlatPreparableStmt::With(ws) => {
                let ws = w.get(*ws);
                if let Some(state) = w.recursion.as_mut() {
                    state.in_with = true;
                }
                let with = w.with_clause(ws.with_clause);
                if let Some(state) = w.recursion.as_mut() {
                    state.in_with = false;
                }
                with?;
                match w.get(ws.body) {
                    FlatWithBody::Query(b) => w.query_body(*b, None, None, Vec::new()),
                    FlatWithBody::Insert(i) => w.insert(*i),
                    FlatWithBody::Update(u) => w.update(*u),
                    FlatWithBody::Delete(d) => w.delete(*d),
                    #[cfg(feature = "since-pg15")]
                    FlatWithBody::Merge(m) => w.merge(*m),
                }
            }
            FlatPreparableStmt::Insert(i) => w.insert(*i),
            FlatPreparableStmt::Update(u) => w.update(*u),
            FlatPreparableStmt::Delete(d) => w.delete(*d),
            #[cfg(feature = "since-pg15")]
            FlatPreparableStmt::Merge(m) => w.merge(*m),
        });
        self.recursive_top = saved_top;
        let state = self.recursion.take();
        let sides_leaf = state.as_ref().map_or((false, false), |s| s.sides_leaf);
        self.recursion = if pushed {
            self.outer_recursions.pop().map(|(s, _)| s)
        } else {
            None
        };
        self.recursion_context = saved_context;
        let target_list = result?;
        if entry.modifying && self.level() > 0 {
            return Err(Diagnostic::new(
                "0A000",
                "WITH clause containing a data-modifying statement must be at the top level",
            )
            .at(entry.location)
            .into());
        }
        // analyzeCTETargetList
        let outputs: Vec<&TargetEntry> = target_list.iter().filter(|t| !t.junk).collect();
        let mut columns = Vec::with_capacity(outputs.len());
        for (j, t) in outputs.iter().enumerate() {
            columns.push(CteColumn {
                collation: t.collation,
                name: items.aliases[i]
                    .get(j)
                    .cloned()
                    .unwrap_or_else(|| t.name.clone()),
                ty: t.ty,
                typmod: t.typmod,
                origin: t.origin,
                fields: t.fields.clone(),
            });
        }
        if columns.len() < items.aliases[i].len() {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "WITH query \"{}\" has {} columns available but {} columns specified",
                    entry.name,
                    columns.len(),
                    items.aliases[i].len()
                ),
            )
            .at(entry.location)
            .into());
        }
        // A recursive query's columns were fixed from its
        // non-recursive term; the whole must agree.
        if let Some(state) = state
            && state.references > 0
            && let Some(fixed) = state.fixed
        {
            for (j, (c, f)) in columns.iter().zip(&fixed).enumerate() {
                if c.ty == f.ty && c.typmod == f.typmod && c.collation != f.collation {
                    let name = |w: &Self, c: Option<pgtk_catalog::Handle<Collation>>| {
                        c.map_or_else(String::new, |c| w.collation_name(c))
                    };
                    return Err(Diagnostic::new(
                            "42P21",
                            format!(
                                "recursive query \"{}\" column {} has collation \"{}\" in non-recursive term but collation \"{}\" overall",
                                entry.name,
                                j + 1,
                                name(self, f.collation),
                                name(self, c.collation)
                            ),
                        )
                        .hint("Use the COLLATE clause to set the collation of the non-recursive term.")
                        .at(outputs[j].location)
                        .into());
                }
                if c.ty != f.ty || c.typmod != f.typmod {
                    return Err(Diagnostic::new(
                            "42804",
                            format!(
                                "recursive query \"{}\" column {} has type {} in non-recursive term but type {} overall",
                                entry.name,
                                j + 1,
                                self.types().format_with_typmod(f.ty, Some(f.typmod)),
                                self.types().format_with_typmod(c.ty, Some(c.typmod))
                            ),
                        )
                        .hint("Cast the output of the non-recursive term to the correct type.")
                        .at(outputs[j].location)
                        .into());
                }
            }
        }
        // SEARCH and CYCLE: their checks, then their columns.
        if def.search.is_some() || def.cycle.is_some() {
            let sides = sides_leaf;
            if !sides.0 {
                return Err(Diagnostic::new(
                    "0A000",
                    "with a SEARCH or CYCLE clause, the left side of the UNION must be a SELECT",
                )
                .into());
            }
            if !sides.1 {
                return Err(Diagnostic::new(
                    "42601",
                    "with a SEARCH or CYCLE clause, the right side of the UNION must be a SELECT",
                )
                .into());
            }
            let colnames: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
            let mut search_seq: Option<String> = None;
            if let Some(sc) = def.search {
                let at = self.span(sc).start;
                let sc = self.get(sc);
                let mut seen: Vec<String> = Vec::new();
                for c in self.list1(self.get(sc.columns).0) {
                    let (colname, _) = self.col_id(*c)?;
                    if !colnames.contains(&colname) {
                        return Err(Diagnostic::new(
                            "42601",
                            format!("search column \"{colname}\" not in WITH query column list"),
                        )
                        .at(at)
                        .into());
                    }
                    if seen.contains(&colname) {
                        return Err(Diagnostic::new(
                            "42701",
                            format!("search column \"{colname}\" specified more than once"),
                        )
                        .at(at)
                        .into());
                    }
                    seen.push(colname);
                }
                let (seq, _) = self.col_id(sc.set_column)?;
                if colnames.contains(&seq) {
                    return Err(Diagnostic::new(
                            "42601",
                            format!("search sequence column name \"{seq}\" already used in WITH query column list"),
                        )
                        .at(at)
                        .into());
                }
                search_seq = Some(seq);
            }
            let mut cycle_names: Option<(String, String)> = None;
            if let Some(cc) = def.cycle {
                let at = self.span(cc).start;
                let cc = self.get(cc);
                let mut seen: Vec<String> = Vec::new();
                for c in self.list1(cc.columns) {
                    let (colname, _) = self.alias_name(*c)?;
                    if !colnames.contains(&colname) {
                        return Err(Diagnostic::new(
                            "42601",
                            format!("cycle column \"{colname}\" not in WITH query column list"),
                        )
                        .at(at)
                        .into());
                    }
                    if seen.contains(&colname) {
                        return Err(Diagnostic::new(
                            "42701",
                            format!("cycle column \"{colname}\" specified more than once"),
                        )
                        .at(at)
                        .into());
                    }
                    seen.push(colname);
                }
                let (mark, _) = self.alias_name(self.get(cc.set_column).name)?;
                let (path, _) = self.alias_name(cc.using_column)?;
                if colnames.contains(&mark) {
                    return Err(Diagnostic::new(
                            "42601",
                            format!("cycle mark column name \"{mark}\" already used in WITH query column list"),
                        )
                        .at(at)
                        .into());
                }
                if colnames.contains(&path) {
                    return Err(Diagnostic::new(
                            "42601",
                            format!("cycle path column name \"{path}\" already used in WITH query column list"),
                        )
                        .at(at)
                        .into());
                }
                if mark == path {
                    return Err(Diagnostic::new(
                        "42601",
                        "cycle mark column name and cycle path column name are the same",
                    )
                    .at(at)
                    .into());
                }
                if let Some(seq) = &search_seq {
                    let search_at = def.search.map_or(at, |sc| self.span(sc).start);
                    if seq == &mark {
                        return Err(Diagnostic::new(
                            "42601",
                            "search sequence column name and cycle mark column name are the same",
                        )
                        .at(search_at)
                        .into());
                    }
                    if seq == &path {
                        return Err(Diagnostic::new(
                            "42601",
                            "search sequence column name and cycle path column name are the same",
                        )
                        .at(search_at)
                        .into());
                    }
                }
                cycle_names = Some((mark, path));
            }
            // rewriteSearchAndCycle adds the columns in this order.
            let record = self.types().builtin(Builtin::Record);
            let record_array = self.types().builtin(Builtin::RecordArray);
            if let (Some(seq), Some(sc)) = (search_seq, def.search) {
                let depth = matches!(self.get(self.get(sc).direction), FlatSearchDirection::Depth);
                columns.push(CteColumn {
                    collation: None,
                    name: seq,
                    ty: if depth { record_array } else { record },
                    typmod: -1,
                    origin: None,
                    fields: None,
                });
            }
            if let (Some((mark, path)), Some((mark_ty, mark_typmod))) = (cycle_names, cycle_mark) {
                columns.push(CteColumn {
                    collation: None,
                    name: mark,
                    ty: mark_ty,
                    typmod: mark_typmod,
                    origin: None,
                    fields: None,
                });
                columns.push(CteColumn {
                    collation: None,
                    name: path,
                    ty: record_array,
                    typmod: -1,
                    origin: None,
                    fields: None,
                });
            }
        }
        let finished = CteEntry {
            columns,
            columns_known: true,
            analysing: false,
            has_returning: entry.modifying && !target_list.is_empty(),
            hidden,
            ..entry.clone()
        };
        if items.recursive {
            items.known[items.base + i] = finished;
        } else {
            items.known.push(finished);
        }
        Ok(())
    }

    /// The expression of a CYCLE mark value. A typed literal with a
    /// function name (`pg_catalog.int4 '1'`, `t(3) 'x'`) has no expression
    /// node.
    fn cycle_mark_value(
        &self,
        v: Handle<pgtk_sql::ast::shared::with_clause::FlatCycleMarkValue>,
    ) -> Result<Handle<FlatExpr>> {
        use pgtk_sql::ast::shared::with_clause::FlatCycleMarkValue;
        match self.get(v) {
            FlatCycleMarkValue::Const(e) => Ok(*e),
            FlatCycleMarkValue::Qualified(_) | FlatCycleMarkValue::Call(_) => {
                let at = self.span(v).start;
                Err(unsupported(
                    "typed literal with a function name in CYCLE",
                    Some(at..at),
                ))
            }
        }
    }

    /// analyzeCTE for a CYCLE clause: the mark's type from its TO and
    /// DEFAULT values, which must have an equality operator.
    fn cycle_mark(
        &mut self,
        scope: &Scope,
        cc: Handle<FlatCycleClause>,
    ) -> Result<(TypeRef, Typmod)> {
        let set = self.get(self.get(cc).set_column);
        let Some(td) = set.to_default else {
            return Ok((self.types().builtin(Builtin::Bool), -1));
        };
        let td = self.get(td);
        let to_handle = self.cycle_mark_value(td.to_value)?;
        let default_handle = self.cycle_mark_value(td.default_value)?;
        let saved = self.expr_kind;
        self.expr_kind = ExprKind::CycleMark;
        let to_value = self.expr(scope, to_handle)?;
        let default_value = self.expr(scope, default_handle)?;
        self.expr_kind = saved;
        let ty = match self.coercer().common_type(&[to_value.ty, default_value.ty]) {
            Ok(t) => t,
            Err((p, n)) => {
                return Err(Diagnostic::new(
                    "42804",
                    format!(
                        "CYCLE types {} and {} cannot be matched",
                        self.types().format(p),
                        self.types().format(n)
                    ),
                )
                .at(if default_value.ty == n {
                    default_value.location
                } else {
                    to_value.location
                })
                .into());
            }
        };
        for (h, t) in [(to_handle, &to_value), (default_handle, &default_value)] {
            if t.ty != ty
                && let Some((_, steps)) =
                    self.coercer()
                        .coerce_to_target_type(t, ty, -1, CoercionContext::Implicit)?
            {
                self.record_coercions(h, steps);
            }
        }
        let typmod = if to_value.ty == ty
            && default_value.ty == ty
            && to_value.typmod == default_value.typmod
        {
            to_value.typmod
        } else {
            -1
        };
        if self.types().equality_operator(ty).is_none() {
            return Err(Diagnostic::new(
                "42883",
                format!(
                    "could not identify an equality operator for type {}",
                    self.types().format(ty)
                ),
            )
            .into());
        }
        Ok((ty, typmod))
    }

    /// A `WITH ... <statement>` at the top level.
    fn with_statement(&mut self, w: Handle<FlatWithStatement>) -> Result<()> {
        let ws = self.get(w);
        #[cfg(feature = "since-pg15")]
        if matches!(self.get(ws.body), FlatWithBody::Merge(_))
            && matches!(
                self.get(self.get(ws.with_clause).head),
                FlatWithClauseHead::Recursive(_)
            )
        {
            return Err(Diagnostic::new(
                "42601",
                "WITH RECURSIVE is not supported for MERGE statement",
            )
            .into());
        }
        self.with_clause(ws.with_clause)?;
        match self.get(ws.body) {
            FlatWithBody::Query(b) => {
                self.allow_into = true;
                let target_list = self.query_body(*b, None, None, Vec::new())?;
                self.allow_into = false;
                if self.into_consumed {
                    self.set_result(&[])
                } else {
                    self.set_result(&target_list)
                }
            }
            FlatWithBody::Insert(i) => {
                let returning = self.insert(*i)?;
                self.set_result(&returning)
            }
            FlatWithBody::Update(u) => {
                let returning = self.update(*u)?;
                self.set_result(&returning)
            }
            FlatWithBody::Delete(d) => {
                let returning = self.delete(*d)?;
                self.set_result(&returning)
            }
            #[cfg(feature = "since-pg15")]
            FlatWithBody::Merge(m) => {
                let returning = self.merge(*m)?;
                self.set_result(&returning)
            }
        }
    }

    /// `scanNameSpaceForCTE`: the WITH query a name means, with how many
    /// levels up it lives and its index there.
    fn scan_cte(&self, scope: &Scope, name: &str) -> Option<(usize, usize)> {
        for (levelsup, level) in std::iter::once(scope)
            .chain(self.outer.iter().rev())
            .enumerate()
        {
            if let Some(index) = level.ctes.iter().position(|c| c.name == name) {
                return Some((levelsup, index));
            }
        }
        None
    }

    /// `addRangeTableEntryForCTE`, with the recursion checks of
    /// `checkWellFormedRecursion` for a reference to the query being
    /// analysed.
    fn cte_ref(
        &mut self,
        scope: &mut Scope,
        levelsup: usize,
        index: usize,
        alias_name: Option<String>,
        column_aliases: Vec<String>,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let entry = self.scope_at(scope, levelsup).ctes[index].clone();
        let entry_level = self.level() - levelsup;
        self.cte_refs.push(CteRef {
            level: entry_level,
            name: entry.name.clone(),
            location,
        });
        let mut self_reference = false;
        if entry.analysing && entry.modifying {
            return Err(Diagnostic::new(
                "42P19",
                format!(
                    "recursive query \"{}\" must not contain data-modifying statements",
                    entry.name
                ),
            )
            .at(entry.location)
            .into());
        }
        if !entry.columns_known {
            if !entry.analysing {
                // A later item of a RECURSIVE WITH, analysed first.
                self.defer_to = Some((entry_level, index));
                return Err(unsupported(
                    "forward reference to a WITH query",
                    Some(location..location),
                ));
            }
            let subquery = || {
                Diagnostic::new(
                    "42P19",
                    format!(
                        "recursive reference to query \"{}\" must not appear within a subquery",
                        entry.name
                    ),
                )
                .at(location)
            };
            let Some((state, _)) = self.recursion_state(entry_level, index) else {
                return Err(subquery().into());
            };
            let state = state.clone();
            if state.in_with {
                return Err(subquery().into());
            }
            if !state.form_ok {
                return Err(Diagnostic::new(
                    "42P19",
                    format!(
                        "recursive query \"{}\" does not have the form non-recursive-term UNION [ALL] recursive-term",
                        entry.name
                    ),
                )
                .at(state.location)
                .into());
            }
            return Err(Diagnostic::new(
                "42P19",
                format!(
                    "recursive reference to query \"{}\" must not appear within its non-recursive term",
                    entry.name
                ),
            )
            .at(location)
            .into());
        }
        if entry.analysing {
            // The recursive reference inside the recursive term.
            self_reference = true;
            let subquery_depth = self.subquery_depth;
            let Some((state, context)) = self.recursion_state(entry_level, index) else {
                return Err(Diagnostic::new(
                    "42P19",
                    format!(
                        "recursive reference to query \"{}\" must not appear within a subquery",
                        entry.name
                    ),
                )
                .at(location)
                .into());
            };
            if !state.form_ok {
                return Err(Diagnostic::new(
                    "42P19",
                    format!(
                        "recursive query \"{}\" does not have the form non-recursive-term UNION [ALL] recursive-term",
                        entry.name
                    ),
                )
                .at(state.location)
                .into());
            }
            if let Some(at) = state.order_by {
                return Err(Diagnostic::new(
                    "0A000",
                    "ORDER BY in a recursive query is not implemented",
                )
                .at(at)
                .into());
            }
            if let Some(at) = state.offset {
                return Err(Diagnostic::new(
                    "0A000",
                    "OFFSET in a recursive query is not implemented",
                )
                .at(at)
                .into());
            }
            if let Some(at) = state.limit {
                return Err(Diagnostic::new(
                    "0A000",
                    "LIMIT in a recursive query is not implemented",
                )
                .at(at)
                .into());
            }
            if state.locking {
                return Err(Diagnostic::new(
                    "0A000",
                    "FOR UPDATE/SHARE in a recursive query is not implemented",
                )
                .into());
            }
            let context_error = match context {
                _ if state.in_with || subquery_depth > state.term_depth => {
                    Some("must not appear within a subquery")
                }
                RecursionContext::Ok => None,
                RecursionContext::OuterJoin => Some("must not appear within an outer join"),
                RecursionContext::Intersect => Some("must not appear within INTERSECT"),
                RecursionContext::Except => Some("must not appear within EXCEPT"),
            };
            if let Some(what) = context_error {
                return Err(Diagnostic::new(
                    "42P19",
                    format!("recursive reference to query \"{}\" {what}", entry.name),
                )
                .at(location)
                .into());
            }
            state.references += 1;
            if state.references > 1 {
                return Err(Diagnostic::new(
                    "42P19",
                    format!(
                        "recursive reference to query \"{}\" must not appear more than once",
                        entry.name
                    ),
                )
                .at(location)
                .into());
            }
        } else if entry.modifying && !entry.has_returning {
            return Err(Diagnostic::new(
                "0A000",
                format!(
                    "WITH query \"{}\" does not have a RETURNING clause",
                    entry.name
                ),
            )
            .at(location)
            .into());
        }
        let eref_name = alias_name.clone().unwrap_or_else(|| entry.name.clone());
        if column_aliases.len() > entry.columns.len() {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "table \"{eref_name}\" has {} columns available but {} columns specified",
                    entry.columns.len(),
                    column_aliases.len()
                ),
            )
            .into());
        }
        // The SEARCH and CYCLE columns follow; `*` inside the query does
        // not expand them.
        let mut columns = entry.columns.clone();
        let hidden = if self_reference {
            let extra = self
                .recursion_state(entry_level, index)
                .map(|(s, _)| s.extra_columns.clone())
                .unwrap_or_default();
            let n = extra.len();
            columns.extend(extra);
            n
        } else if levelsup > 0 {
            entry.hidden
        } else {
            0
        };
        let mut colnames: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
        for (i, a) in column_aliases.into_iter().enumerate() {
            colnames[i] = a;
        }
        scope.rtable.push(crate::relation::RangeTableEntry {
            kind: RteKind::Cte {
                columns,
                self_reference,
                hidden,
            },
            alias: alias_name,
            eref_name,
            colnames,
            location,
            composite_only: false,
        });
        let item = NamespaceItem::plain(scope.rtable.len());
        Ok((item.clone(), vec![item]))
    }

    /// The recursion state of the recursive WITH query at `level` and
    /// `index`, the one being analysed or an enclosing one, with the
    /// recursion context a reference to it is in.
    fn recursion_state(
        &mut self,
        level: usize,
        index: usize,
    ) -> Option<(&mut RecursionState, RecursionContext)> {
        let own = self
            .recursion
            .as_ref()
            .is_some_and(|s| s.level == level && s.index == index);
        if own {
            let context = self.recursion_context;
            return self.recursion.as_mut().map(|s| (s, context));
        }
        let at = self
            .outer_recursions
            .iter()
            .rposition(|(s, _)| s.level == level && s.index == index)?;
        let (state, context) = &mut self.outer_recursions[at];
        Some((state, *context))
    }

    /// A SELECT's INTO clause: accepted once for the statement's own
    /// SELECT, an error anywhere else.
    fn select_into(&mut self, into: Handle<FlatSelectIntoClause>) -> Result<()> {
        if self.allow_into {
            self.allow_into = false;
            self.into_consumed = true;
            return Ok(());
        }
        let at = self.span(self.get(into).target).start;
        let message = if self.set_op_member {
            "INTO is only allowed on first SELECT of UNION/INTERSECT/EXCEPT"
        } else {
            "SELECT ... INTO is not allowed here"
        };
        Err(Diagnostic::new("42601", message).at(at).into())
    }

    /// `transformDeclareCursorStmt`.
    fn declare(&mut self, handle: Handle<FlatDeclareStmt>) -> Result<()> {
        let stmt = self.get(handle);
        let options: Vec<&FlatCursorOption> = self
            .list(stmt.options)
            .iter()
            .map(|o| self.get(*o))
            .collect();
        let has = |o: FlatCursorOption| {
            options
                .iter()
                .any(|x| std::mem::discriminant(*x) == std::mem::discriminant(&o))
        };
        if has(FlatCursorOption::Scroll) && has(FlatCursorOption::NoScroll) {
            return Err(
                Diagnostic::new("42P11", "cannot specify both SCROLL and NO SCROLL").into(),
            );
        }
        if has(FlatCursorOption::Asensitive) && has(FlatCursorOption::Insensitive) {
            return Err(
                Diagnostic::new("42P11", "cannot specify both ASENSITIVE and INSENSITIVE").into(),
            );
        }
        let sub = self.get(stmt.query);
        if let Some(with) = sub.with {
            self.with_clause(with)?;
        }
        let target_list = self.query_body(sub.body, None, None, Vec::new())?;
        let (name, _) = self.alias_name(stmt.name)?;
        let result = ResultDescriptor {
            columns: result_columns(&target_list),
            params: self.finish_params()?,
        };
        self.set_result(&[])?;
        self.analysis.session_effect = Some(SessionEffect::Declare(Cursor {
            name,
            result,
            scroll: has(FlatCursorOption::Scroll),
            hold: matches!(stmt.hold.map(|h| self.get(h)), Some(FlatCursorHold::With)),
        }));
        Ok(())
    }

    /// `transformCreateTableAsStmt` for a table.
    fn create_table_as(&mut self, source: Handle<FlatCtasSource>) -> Result<()> {
        match self.get(source) {
            // The prepared statement is looked up when the Statement runs.
            FlatCtasSource::Execute(_) => self.set_result(&[]),
            FlatCtasSource::Query(sub) => {
                let sub = self.get(*sub);
                if let Some(with) = sub.with {
                    self.with_clause(with)?;
                }
                self.query_body(sub.body, None, None, Vec::new())?;
                self.set_result(&[])
            }
        }
    }

    /// `transformCreateTableAsStmt` for a materialized view.
    fn create_matview(&mut self, handle: Handle<FlatCreateMaterializedViewStmt>) -> Result<()> {
        let stmt = self.get(handle);
        let sub = self.get(stmt.query);
        if let Some(with) = sub.with {
            self.with_clause(with)?;
        }
        self.query_body(sub.body, None, None, Vec::new())?;
        let params = self.finish_params()?;
        if !params.is_empty() {
            return Err(Diagnostic::new(
                "0A000",
                "materialized views may not be defined using bound parameters",
            )
            .into());
        }
        if stmt.unlogged {
            return Err(Diagnostic::new("0A000", "materialized views cannot be unlogged").into());
        }
        self.analysis.result = ResultDescriptor {
            columns: Vec::new(),
            params,
        };
        Ok(())
    }

    /// `transformSortClause` with the SQL92 rules; returns the index of
    /// each sort item's target entry.
    fn sort_clause(
        &mut self,
        scope: &Scope,
        order_by: Option<Handle<FlatOrderByClause>>,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
    ) -> Result<Vec<usize>> {
        let mut sort_refs: Vec<usize> = Vec::new();
        self.expr_kind = ExprKind::OrderBy;
        if let Some(order_by) = order_by {
            let clause = self.get(order_by);
            for item in self.list1(clause.items) {
                let item = self.get(*item);
                let index = self.find_targetlist_entry_sql92(
                    scope,
                    item.expr,
                    target_list,
                    target_exprs,
                    ExprKind::OrderBy,
                )?;
                sort_refs.push(index);
                let entry = &mut target_list[index];
                // addTargetToSortList: unknown becomes text, then the type
                // must have an ordering operator.
                if self.types().is_unknown(entry.ty) {
                    entry.ty = self.types().builtin(Builtin::Text);
                    entry.typmod = -1;
                }
                let location = self.span(item.expr).start;
                if let Some(FlatSortOrder::Using(using)) = item.order.map(|o| self.get(o)) {
                    let ty = entry.ty;
                    self.using_operator(*using, ty, location)?;
                    continue;
                }
                if self.types().btree_ordering(entry.ty).is_none() {
                    return Err(Diagnostic::new(
                        "42883",
                        format!(
                            "could not identify an ordering operator for type {}",
                            self.types().format(entry.ty)
                        ),
                    )
                    .hint("Use an explicit ordering operator or modify the query.")
                    .at(location)
                    .into());
                }
            }
        }
        Ok(sort_refs)
    }

    /// `transformLimitClause` for LIMIT and OFFSET.
    fn limit_clause(
        &mut self,
        scope: &Scope,
        limit_offset: Option<Handle<FlatLimitOffsetClause>>,
    ) -> Result<()> {
        let Some(lo) = limit_offset else {
            return Ok(());
        };
        let (limit, offset) = match self.get(lo) {
            FlatLimitOffsetClause::LimitOffset(x) => {
                let x = self.get(*x);
                (Some(x.limit), x.offset)
            }
            FlatLimitOffsetClause::OffsetLimit(x) => {
                let x = self.get(*x);
                (x.limit, Some(x.offset))
            }
        };
        if let Some(limit) = limit {
            match self.get(limit) {
                FlatLimitingClause::Limit(l) => {
                    let count = self.get(*l).count;
                    self.expr_kind = ExprKind::Limit;
                    self.limit_expr(scope, count, "LIMIT")?;
                }
                FlatLimitingClause::FetchFirst(f) => {
                    // A count of 1 when none is written.
                    if let FlatFetchFirstBody::WithCount(c) = self.get(self.get(*f).body) {
                        let count = self.get(*c).count;
                        self.expr_kind = ExprKind::Limit;
                        self.fetch_first_value(scope, count, "LIMIT")?;
                    }
                }
            }
        }
        if let Some(offset) = offset {
            self.expr_kind = ExprKind::Offset;
            match self.get(offset) {
                FlatOffsetClause::Count(c) => {
                    let count = self.get(*c).count;
                    self.limit_expr(scope, count, "OFFSET")?;
                }
                FlatOffsetClause::Rows(r) => {
                    let count = self.get(*r).count;
                    self.fetch_first_value(scope, count, "OFFSET")?;
                }
            }
        }
        Ok(())
    }

    /// gram.y `select_fetch_first_value`: a `c_expr` is a count like any
    /// other. A signed constant is an `int4` or a `numeric` constant, which
    /// both coerce to `int8` by assignment, so it needs no check.
    fn fetch_first_value(
        &mut self,
        scope: &Scope,
        value: Handle<FlatFetchFirstValue>,
        construct: &str,
    ) -> Result<()> {
        match self.get(value) {
            FlatFetchFirstValue::Value(e) => self.limit_expr(scope, *e, construct),
            FlatFetchFirstValue::Signed(_) => Ok(()),
        }
    }

    /// The number of levels above the current one.
    fn level(&self) -> usize {
        self.outer.len()
    }

    /// A new Scope for the query level about to start, holding the WITH
    /// queries analysed for it.
    fn level_scope(&mut self) -> Scope {
        let mut scope = Scope::new();
        scope.ctes = std::mem::take(&mut self.pending_ctes);
        scope
    }

    /// The scope `levelsup` levels above `scope`.
    fn scope_at<'s>(&'s self, scope: &'s Scope, levelsup: usize) -> &'s Scope {
        if levelsup == 0 {
            scope
        } else {
            &self.outer[self.outer.len() - levelsup]
        }
    }

    /// `markTargetListOrigin` for one `Var`: the table column it comes
    /// from, through subqueries.
    fn origin_of(
        &self,
        scope: &Scope,
        var: &Var,
    ) -> Option<(pgtk_catalog::Handle<pgtk_catalog::Relation>, i16)> {
        let level = self.scope_at(scope, var.levelsup);
        let rte = &level.rtable[var.varno - 1];
        match &rte.kind {
            RteKind::Relation { relation, .. } => Some((*relation, var.varattno)),
            RteKind::Subquery { columns, .. } => {
                columns[usize::try_from(var.varattno - 1).ok()?].origin
            }
            RteKind::Cte {
                columns,
                self_reference,
                ..
            } => {
                if *self_reference {
                    None
                } else {
                    columns[usize::try_from(var.varattno - 1).ok()?].origin
                }
            }
            RteKind::Join { .. }
            | RteKind::Function { .. }
            | RteKind::Values { .. }
            | RteKind::SetOperation { .. } => None,
        }
    }

    /// The Result Descriptor of a query: its non-junk target entries and the
    /// parameter types.
    fn set_result(&mut self, target_list: &[TargetEntry]) -> Result<()> {
        self.analysis.result = ResultDescriptor {
            columns: result_columns(target_list),
            params: self.finish_params()?,
        };
        Ok(())
    }

    /// Whether a FETCH FIRST clause says WITH TIES.
    fn fetch_with_ties(&self, f: Handle<FlatFetchFirstClause>) -> bool {
        let mode = match self.get(self.get(f).body) {
            FlatFetchFirstBody::NoCount(n) => self.get(*n).mode,
            FlatFetchFirstBody::WithCount(c) => self.get(*c).mode,
        };
        matches!(self.get(mode), FlatFetchMode::WithTies)
    }

    /// The LIMIT/OFFSET clause and the locking clauses a query body
    /// wrote, in their order.
    fn limit_locking(
        &self,
        clause: Option<Handle<FlatLimitLockingClause>>,
    ) -> (
        Option<Handle<FlatLimitOffsetClause>>,
        Vec<Handle<FlatForUpdateClause>>,
    ) {
        let Some(c) = clause else {
            return (None, Vec::new());
        };
        let (limit, locking) = match self.get(c) {
            FlatLimitLockingClause::LockingFirst(x) => {
                let x = self.get(*x);
                (x.limit_offset, Some(x.locking))
            }
            FlatLimitLockingClause::LimitFirst(x) => {
                let x = self.get(*x);
                (Some(x.limit_offset), x.locking)
            }
        };
        let items = match locking.map(|l| self.get(l)) {
            Some(FlatForLockingClause::Items(items)) => self.list1(*items).to_vec(),
            Some(FlatForLockingClause::ReadOnly) | None => Vec::new(),
        };
        (limit, items)
    }

    /// `transformLockingClause` for one clause. `own` is what
    /// `CheckSelectLocking` refuses at this level, if anything.
    fn locking_clause(
        &self,
        scope: &Scope,
        clause: Handle<FlatForUpdateClause>,
        own: Option<&'static str>,
    ) -> Result<()> {
        let c = self.get(clause);
        let strength = match self.get(c.mode) {
            FlatLockingMode::Update => "FOR UPDATE",
            FlatLockingMode::NoKeyUpdate => "FOR NO KEY UPDATE",
            FlatLockingMode::Share => "FOR SHARE",
            FlatLockingMode::KeyShare => "FOR KEY SHARE",
        };
        let refused = |what: &str| -> Diagnostic {
            Diagnostic::new("0A000", format!("{strength} is not allowed with {what}"))
        };
        if let Some(what) = own {
            return Err(refused(what).into());
        }
        let Some(of) = c.of else {
            // Every relation and subquery in the query, and the subqueries'
            // relations in turn.
            for rte in &scope.rtable {
                if let RteKind::Subquery {
                    lock_refusal: Some(what),
                    ..
                } = &rte.kind
                {
                    return Err(refused(what).into());
                }
            }
            return Ok(());
        };
        for n in self.list1(self.get(of).names) {
            let at = self.span(*n).start;
            let (schema, name) = self.qualified_name(*n, at)?;
            // For simplicity PostgreSQL insists on unqualified alias names.
            if schema.is_some() {
                return Err(Diagnostic::new(
                    "42601",
                    format!("{strength} must specify unqualified relation names"),
                )
                .at(at)
                .into());
            }
            let mut found = false;
            for (i, rte) in scope.rtable.iter().enumerate() {
                let rtename: &str = match (&rte.alias, &rte.kind) {
                    // A join without an alias is visible through its USING
                    // alias only; a subquery or VALUES without one never.
                    (None, RteKind::Join { .. }) => {
                        match scope
                            .namespace
                            .iter()
                            .find(|item| item.rt_index == i + 1)
                            .and_then(|item| item.using_alias.as_ref())
                        {
                            Some((using, _)) => using.as_str(),
                            None => continue,
                        }
                    }
                    (None, RteKind::Subquery { .. } | RteKind::Values { .. }) => continue,
                    _ => rte.eref_name.as_str(),
                };
                if rtename != name {
                    continue;
                }
                found = true;
                let applied_to = match &rte.kind {
                    RteKind::Relation { .. } => break,
                    RteKind::Subquery { lock_refusal, .. } => {
                        if let Some(what) = lock_refusal {
                            return Err(refused(what).into());
                        }
                        break;
                    }
                    RteKind::Join { .. } => "a join",
                    RteKind::Function { .. } => "a function",
                    RteKind::Values { .. } => "VALUES",
                    RteKind::Cte { .. } => "a WITH query",
                    RteKind::SetOperation { .. } => break,
                };
                return Err(Diagnostic::new(
                    "0A000",
                    format!("{strength} cannot be applied to {applied_to}"),
                )
                .at(at)
                .into());
            }
            if !found {
                return Err(Diagnostic::new(
                    "42P01",
                    format!("relation \"{name}\" in {strength} clause not found in FROM clause"),
                )
                .at(at)
                .into());
            }
        }
        Ok(())
    }

    /// What a locking clause pushed down into this level's subqueries
    /// would be refused with, if anything.
    fn pushed_down_refusal(scope: &Scope) -> Option<&'static str> {
        scope.rtable.iter().find_map(|rte| match &rte.kind {
            RteKind::Subquery { lock_refusal, .. } => *lock_refusal,
            _ => None,
        })
    }

    /// `transformRangeTableSample`, after the relation is in the range
    /// table at `rt_index`.
    fn tablesample(
        &mut self,
        scope: &Scope,
        rt_index: usize,
        ts: Handle<FlatTableSampleClause>,
    ) -> Result<()> {
        let rte = &scope.rtable[rt_index - 1];
        let sampleable = match &rte.kind {
            RteKind::Relation { relation, .. } => matches!(
                self.catalog()[*relation].kind,
                RelationKind::Table
                    | RelationKind::MaterializedView
                    | RelationKind::PartitionedTable
            ),
            _ => false,
        };
        if !sampleable {
            return Err(Diagnostic::new(
                "0A000",
                "TABLESAMPLE clause can only be applied to tables and materialized views",
            )
            .at(rte.location)
            .into());
        }
        let clause = self.get(ts);
        let at = self.span(clause.method).start;
        let name = self.func_call_name(clause.method, at)?;
        let method = match &name.schema {
            Some(schema) => format!("{schema}.{}", name.name),
            None => name.name.clone(),
        };
        // LookupFuncName: the handler has the method's name and one
        // `internal` argument.
        let catalog = self.catalog();
        let handler = self
            .resolver
            .functions_named(&name, at)?
            .into_iter()
            .map(|(f, _)| f)
            .find(|f| {
                let f = &catalog[*f];
                f.arguments.len() == 1 && catalog[f.arguments[0].ty].name == "internal"
            });
        let Some(handler) = handler else {
            return Err(Diagnostic::new(
                "42704",
                format!("tablesample method {method} does not exist"),
            )
            .at(at)
            .into());
        };
        if !matches!(
            self.types().pseudo(catalog[handler].returns),
            Some(PseudoType::TsmHandler)
        ) {
            return Err(Diagnostic::new(
                "42809",
                format!("function {method} must return type tsm_handler"),
            )
            .at(at)
            .into());
        }
        // The handler's TsmRoutine, for the methods this slice knows.
        let (params, repeatable): (Vec<Builtin>, bool) = match name.name.as_str() {
            "bernoulli" | "system" => (vec![Builtin::Float4], true),
            "system_rows" => (vec![Builtin::Int4], false),
            "system_time" => (vec![Builtin::Float8], false),
            _ => return Err(unsupported("tablesample method", Some(at..at))),
        };
        let args = self.list1(self.get(clause.args).0).to_vec();
        if args.len() != params.len() {
            let plural = if params.len() == 1 {
                "argument"
            } else {
                "arguments"
            };
            return Err(Diagnostic::new(
                "2202H",
                format!(
                    "tablesample method {method} requires {} {plural}, not {}",
                    params.len(),
                    args.len()
                ),
            )
            .at(at)
            .into());
        }
        let saved = self.expr_kind;
        self.expr_kind = ExprKind::FromFunction;
        for (arg, param) in args.into_iter().zip(params) {
            let typed = self.expr(scope, arg)?;
            let target = self.types().builtin(param);
            self.coerce_to_specific_type(arg, &typed, target, "TABLESAMPLE")?;
        }
        if let Some(r) = clause.repeatable {
            if !repeatable {
                return Err(Diagnostic::new(
                    "0A000",
                    format!("tablesample method {method} does not support REPEATABLE"),
                )
                .at(at)
                .into());
            }
            let seed = self.get(r).seed;
            let typed = self.expr(scope, seed)?;
            let target = self.types().builtin(Builtin::Float8);
            self.coerce_to_specific_type(seed, &typed, target, "REPEATABLE")?;
        }
        self.expr_kind = saved;
        Ok(())
    }

    /// `coerce_to_specific_type`: assignment coercion to `target`, which
    /// must succeed, of a value that must not be a set.
    fn coerce_to_specific_type(
        &mut self,
        expr: Handle<FlatExpr>,
        typed: &Typed,
        target: TypeRef,
        construct: &str,
    ) -> Result<()> {
        if typed.ty != target {
            match self.coercer().coerce_to_target_type(
                typed,
                target,
                -1,
                CoercionContext::Assignment,
            )? {
                Some((_, steps)) => self.record_coercions(expr, steps),
                None => {
                    return Err(Diagnostic::new(
                        "42804",
                        format!(
                            "argument of {construct} must be type {}, not type {}",
                            self.types().format(target),
                            self.types().format(typed.ty)
                        ),
                    )
                    .at(typed.location)
                    .into());
                }
            }
        }
        if typed.returns_set {
            return Err(Diagnostic::new(
                "42804",
                format!("argument of {construct} must not return a set"),
            )
            .at(typed.location)
            .into());
        }
        Ok(())
    }

    /// `transformWhereClause` for a `WHERE`.
    fn where_clause(
        &mut self,
        scope: &Scope,
        clause: Option<Handle<FlatWhereClause>>,
    ) -> Result<()> {
        self.where_clause_kind(scope, clause, ExprKind::Where)
    }

    /// `transformWhereClause` with the kind the clause is analysed as.
    fn where_clause_kind(
        &mut self,
        scope: &Scope,
        clause: Option<Handle<FlatWhereClause>>,
        kind: ExprKind,
    ) -> Result<()> {
        let Some(where_clause) = clause else {
            return Ok(());
        };
        let expr = self.get(where_clause).condition;
        self.where_expr(scope, expr, kind)
    }

    /// `transformWhereClause` for the `WHERE` of UPDATE and DELETE, which
    /// can be `CURRENT OF`.
    fn where_or_current_clause(
        &mut self,
        scope: &Scope,
        clause: Option<Handle<FlatWhereOrCurrentClause>>,
    ) -> Result<()> {
        let Some(where_clause) = clause else {
            return Ok(());
        };
        let clause = self.get(where_clause);
        // transformCurrentOfExpr: the cursor is looked up at execution.
        let FlatWhereCondition::Expr(expr) = self.get(clause.condition) else {
            return Ok(());
        };
        self.where_expr(scope, *expr, ExprKind::Where)
    }

    /// The condition of a `WHERE`, coerced to boolean.
    fn where_expr(&mut self, scope: &Scope, expr: Handle<FlatExpr>, kind: ExprKind) -> Result<()> {
        self.expr_kind = kind;
        let typed = self.expr(scope, expr)?;
        let (_, steps) = self.coercer().coerce_to_boolean(&typed, "WHERE")?;
        self.record_coercions(expr, steps);
        Ok(())
    }

    /// A `qualified_name`: schema and relation name.
    fn qualified_name(
        &self,
        qn: Handle<FlatQualifiedName>,
        location: usize,
    ) -> Result<(Option<String>, String)> {
        let mut parts = self.qualified_name_parts(qn)?;
        if parts.len() > 2 {
            return Err(unsupported(
                "catalog-qualified table name",
                Some(location..location),
            ));
        }
        let name = parts.pop().expect("at least one part");
        Ok((parts.pop(), name))
    }

    /// A `relation_expr`: the name, and whether inheritance children are
    /// included (`ONLY` excludes them; a trailing `*` is the default).
    fn relation_expr(&self, r: Handle<FlatRelationExpr>) -> (Handle<FlatQualifiedName>, bool) {
        match self.get(r) {
            FlatRelationExpr::Only(o) => match self.get(*o) {
                FlatOnlyRelation::Parens(p) => (self.get(*p).name, false),
                FlatOnlyRelation::Plain(q) => (*q, false),
            },
            FlatRelationExpr::Named(n) => (self.get(*n).name, true),
        }
    }

    /// `setTargetTable`: the target relation of a DML statement, added to
    /// the namespace when it is also a source of values.
    fn dml_target(
        &mut self,
        scope: &mut Scope,
        table: Handle<FlatQualifiedName>,
        alias: Option<String>,
        inh: bool,
        also_source: bool,
    ) -> Result<usize> {
        let location = self.span(table).start;
        let (schema, name) = self.qualified_name(table, location)?;
        let rt_index = self.relations.add_range_table_entry(
            scope,
            schema.as_deref(),
            &name,
            alias.as_deref(),
            &[],
            inh,
            location,
        )?;
        if also_source {
            scope.namespace.push(NamespaceItem::plain(rt_index));
        }
        Ok(rt_index)
    }

    /// `rewriteTargetListIU`'s checks on generated and identity columns,
    /// which the rewriter raises at parse time. `assigned` holds, for each
    /// column of the relation, whether every value written for it is
    /// `DEFAULT`, and is absent for a column not written.
    fn generated_column_checks(
        &self,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        assigned: &[(i16, bool)],
        override_kind: Option<FlatOverridingKind>,
        insert: bool,
    ) -> Result<()> {
        let rel = &self.catalog()[relation];
        for column in &rel.columns {
            let entry = assigned
                .iter()
                .find(|(attnum, _)| *attnum == column.position);
            let mut apply_default = match entry {
                None => insert,
                Some((_, all_default)) => *all_default,
            };
            let name = column.name.to_string();
            if insert {
                if column.identity == Some(pgtk_catalog::Identity::Always) && !apply_default {
                    match override_kind {
                        Some(FlatOverridingKind::User) => apply_default = true,
                        Some(FlatOverridingKind::System) => {}
                        None => {
                            return Err(Diagnostic::new(
                                "428C9",
                                format!("cannot insert a non-DEFAULT value into column \"{name}\""),
                            )
                            .detail(format!(
                                "Column \"{name}\" is an identity column defined as GENERATED ALWAYS."
                            ))
                            .hint("Use OVERRIDING SYSTEM VALUE to override.")
                            .into());
                        }
                    }
                }
                if column.identity == Some(pgtk_catalog::Identity::ByDefault)
                    && matches!(override_kind, Some(FlatOverridingKind::User))
                {
                    apply_default = true;
                }
                if column.is_generated() && !apply_default {
                    return Err(Diagnostic::new(
                        "428C9",
                        format!("cannot insert a non-DEFAULT value into column \"{name}\""),
                    )
                    .detail(format!("Column \"{name}\" is a generated column."))
                    .into());
                }
            } else if entry.is_some() && !apply_default {
                if column.identity == Some(pgtk_catalog::Identity::Always) {
                    return Err(Diagnostic::new(
                        "428C9",
                        format!("column \"{name}\" can only be updated to DEFAULT"),
                    )
                    .detail(format!(
                        "Column \"{name}\" is an identity column defined as GENERATED ALWAYS."
                    ))
                    .into());
                }
                if column.is_generated() {
                    return Err(Diagnostic::new(
                        "428C9",
                        format!("column \"{name}\" can only be updated to DEFAULT"),
                    )
                    .detail(format!("Column \"{name}\" is a generated column."))
                    .into());
                }
            }
        }
        Ok(())
    }

    /// `attnameAttNum`: a column of the target relation by name, with the
    /// system columns when `sys_ok`.
    fn target_attnum(
        &self,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        name: &str,
        sys_ok: bool,
    ) -> Option<(i16, TypeRef, Typmod)> {
        let rel = &self.catalog()[relation];
        if let Some(c) = rel.columns.iter().find(|c| c.name == name) {
            return Some((c.position, c.ty, c.typmod.0));
        }
        if sys_ok && let Some((attnum, ty)) = crate::relation::system_attribute(self.types(), name)
        {
            return Some((attnum, ty, -1));
        }
        None
    }

    /// `transformAssignedExpr` without indirection: the value is coerced
    /// to the column in assignment context.
    #[allow(clippy::too_many_arguments)]
    fn assign(
        &mut self,
        scope: &Scope,
        value: &InsertValue,
        colname: &str,
        ty: TypeRef,
        typmod: Typmod,
        parts: &[Handle<FlatIndirectionEl>],
        location: usize,
    ) -> Result<()> {
        let (handle, typed) = match value {
            InsertValue::Default(_) => {
                if let Some(first) = parts.first() {
                    let message = match self.get(*first) {
                        FlatIndirectionEl::Subscript(_) => "cannot set an array element to DEFAULT",
                        FlatIndirectionEl::Field(_) => "cannot set a subfield to DEFAULT",
                    };
                    return Err(Diagnostic::new("0A000", message).at(location).into());
                }
                return Ok(());
            }
            InsertValue::Expr(h, t) => (Some(*h), t.clone()),
            InsertValue::Column(t) => (None, t.clone()),
        };
        // transformAssignmentIndirection: the target is the element, the
        // slice or the field the indirection names.
        let (ty, typmod, kind) =
            self.assignment_target(scope, colname, ty, typmod, parts, location)?;
        let coerced = self.coercer().coerce_to_target_type(
            &typed,
            ty,
            typmod,
            CoercionContext::Assignment,
        )?;
        match coerced {
            Some((_, steps)) => {
                if let Some(h) = handle {
                    self.record_coercions(h, steps);
                }
                Ok(())
            }
            None => {
                let (message, at) = match kind {
                    AssignKind::Plain => (
                        format!(
                            "column \"{colname}\" is of type {} but expression is of type {}",
                            self.types().format(ty),
                            self.types().format(typed.ty)
                        ),
                        typed.location,
                    ),
                    AssignKind::Subscripted(name) => (
                        format!(
                            "subscripted assignment to \"{name}\" requires type {} but expression is of type {}",
                            self.types().format(ty),
                            self.types().format(typed.ty)
                        ),
                        location,
                    ),
                    AssignKind::Subfield(name) => (
                        format!(
                            "subfield \"{name}\" is of type {} but expression is of type {}",
                            self.types().format(ty),
                            self.types().format(typed.ty)
                        ),
                        location,
                    ),
                };
                Err(Diagnostic::new("42804", message)
                    .hint("You will need to rewrite or cast the expression.")
                    .at(at)
                    .into())
            }
        }
    }

    /// `transformAssignmentIndirection` down to the value's own target:
    /// its type and typmod, and how the target was reached.
    #[allow(clippy::too_many_arguments)]
    fn assignment_target(
        &mut self,
        scope: &Scope,
        colname: &str,
        ty: TypeRef,
        typmod: Typmod,
        parts: &[Handle<FlatIndirectionEl>],
        location: usize,
    ) -> Result<(TypeRef, Typmod, AssignKind)> {
        let mut ty = ty;
        let mut typmod = typmod;
        let mut name = colname.to_owned();
        let mut kind = AssignKind::Plain;
        let mut subs: Vec<Handle<FlatBracketSubscript>> = Vec::new();
        for part in parts {
            match self.get(*part) {
                FlatIndirectionEl::Subscript(b) => subs.push(*b),
                FlatIndirectionEl::Field(f) => {
                    if !subs.is_empty() {
                        (ty, typmod) = self.subscript_target(scope, ty, typmod, &subs, location)?;
                        subs.clear();
                    }
                    let (fname, _) = self.alias_name(self.get(*f).name)?;
                    let (base, base_typmod) = self.types().base_type_and_typmod(ty, typmod);
                    let _ = base_typmod;
                    let TypeKind::Composite(detail) = &self.catalog()[base].kind else {
                        return Err(Diagnostic::new(
                            "42804",
                            format!(
                                "cannot assign to field \"{fname}\" of column \"{name}\" because its type {} is not a composite type",
                                self.types().format(ty)
                            ),
                        )
                        .at(location)
                        .into());
                    };
                    let Some(field) = detail.fields.iter().find(|fl| fl.name.as_str() == fname)
                    else {
                        if detail.relation.is_some()
                            && crate::relation::system_attribute(self.types(), &fname).is_some()
                        {
                            return Err(Diagnostic::new(
                                "42703",
                                format!("cannot assign to system column \"{fname}\""),
                            )
                            .at(location)
                            .into());
                        }
                        return Err(Diagnostic::new(
                            "42703",
                            format!(
                                "cannot assign to field \"{fname}\" of column \"{name}\" because there is no such column in data type {}",
                                self.types().format(ty)
                            ),
                        )
                        .at(location)
                        .into());
                    };
                    ty = field.ty;
                    typmod = field.typmod.0;
                    name = fname;
                    kind = AssignKind::Subfield(name.clone());
                }
            }
        }
        if !subs.is_empty() {
            (ty, typmod) = self.subscript_target(scope, ty, typmod, &subs, location)?;
            kind = AssignKind::Subscripted(name);
        }
        Ok((ty, typmod, kind))
    }

    /// `transformAssignmentSubscripts`: the type a subscripted assignment
    /// needs, the element for an index and the container for a slice.
    fn subscript_target(
        &mut self,
        scope: &Scope,
        ty: TypeRef,
        typmod: Typmod,
        subs: &[Handle<FlatBracketSubscript>],
        location: usize,
    ) -> Result<(TypeRef, Typmod)> {
        let base = Typed {
            ty,
            typmod,
            form: Form::Other,
            location,
            returns_set: false,
        };
        let t = self.subscripts(scope, &base, subs, location)?;
        Ok((t.ty, t.typmod))
    }

    /// `transformReturningList`.
    fn returning(
        &mut self,
        scope: &Scope,
        clause: Option<Handle<FlatReturningClause>>,
        kind: ExprKind,
    ) -> Result<Vec<TargetEntry>> {
        let Some(r) = clause else {
            return Ok(Vec::new());
        };
        #[cfg(feature = "since-pg18")]
        if let Some(with) = self.get(r).with {
            return Err(unsupported(
                "RETURNING WITH",
                Some(self.span(with).start..self.span(with).end),
            ));
        }
        let saved = self.expr_kind;
        self.expr_kind = kind;
        let mut list = Vec::new();
        for item in self.list1(self.get(r).items) {
            self.target_item(scope, *item, &mut list)?;
        }
        self.expr_kind = saved;
        if list.is_empty() {
            return Err(
                Diagnostic::new("42601", "RETURNING must have at least one column")
                    .at(self.span(r).start)
                    .into(),
            );
        }
        let text = self.types().builtin(Builtin::Text);
        for entry in &mut list {
            if self.types().is_unknown(entry.ty) {
                entry.ty = text;
                entry.typmod = -1;
            }
        }
        let exprs: Vec<Option<Handle<FlatExpr>>> = vec![None; list.len()];
        self.assign_query_collations(scope, &mut list, &exprs, &[])?;
        Ok(list)
    }

    /// `transformInsertStmt`, returning the RETURNING list.
    fn insert(&mut self, handle: Handle<FlatInsertStmt>) -> Result<Vec<TargetEntry>> {
        let stmt = self.get(handle);
        let mut scope = self.level_scope();
        let alias = match stmt.alias {
            Some(a) => Some(self.col_id(self.get(a).name)?.0),
            None => None,
        };
        let rt_index = self.dml_target(&mut scope, stmt.table_name, alias, false, false)?;
        let relation = scope.rtable[rt_index - 1]
            .relation()
            .expect("the target is a relation");
        let relname = self.catalog()[relation].name.to_string();
        // checkInsertTargets
        let mut icolumns: Vec<InsertColumn> = Vec::new();
        let mut attnums: Vec<i16> = Vec::new();
        let override_kind: Option<FlatOverridingKind>;
        let (cols_given, source) = match self.get(stmt.rest) {
            FlatInsertRest::Columns(c) => {
                let c = self.get(*c);
                override_kind = c.overriding.map(|o| *self.get(self.get(o).which));
                // checkInsertTargets: a column written whole once, or in
                // parts any number of times.
                let mut wholecols = std::collections::HashSet::new();
                let mut partialcols = std::collections::HashSet::new();
                for item in self.list1(self.get(c.columns).items) {
                    let it = self.get(*item);
                    let (name, loc) = self.col_id(it.name)?;
                    let parts = self.assignment_indirection(it.indirection)?;
                    let Some((attnum, ty, typmod)) = self.target_attnum(relation, &name, false)
                    else {
                        return Err(Diagnostic::new(
                            "42703",
                            format!("column \"{name}\" of relation \"{relname}\" does not exist"),
                        )
                        .at(loc)
                        .into());
                    };
                    let duplicate = if parts.is_empty() {
                        !wholecols.insert(attnum) || partialcols.contains(&attnum)
                    } else {
                        partialcols.insert(attnum);
                        wholecols.contains(&attnum)
                    };
                    if duplicate {
                        return Err(Diagnostic::new(
                            "42701",
                            format!("column \"{name}\" specified more than once"),
                        )
                        .at(loc)
                        .into());
                    }
                    icolumns.push((name, ty, typmod, loc, parts));
                    attnums.push(attnum);
                }
                (true, Some(c.source))
            }
            FlatInsertRest::Plain(_) | FlatInsertRest::Default => {
                let source = match self.get(stmt.rest) {
                    FlatInsertRest::Plain(p) => {
                        let p = self.get(*p);
                        override_kind = p.overriding.map(|o| *self.get(self.get(o).which));
                        Some(p.source)
                    }
                    _ => {
                        override_kind = None;
                        None
                    }
                };
                let rel = &self.catalog()[relation];
                let mut cols: Vec<_> = rel.columns.iter().collect();
                cols.sort_by_key(|c| c.position);
                for c in cols {
                    icolumns.push((
                        c.name.to_string(),
                        c.ty,
                        c.typmod.0,
                        NO_LOCATION,
                        Vec::new(),
                    ));
                    attnums.push(c.position);
                }
                (false, source)
            }
        };
        let mut rows: Vec<Vec<InsertValue>> = Vec::new();
        match source {
            None => {}
            Some(sub_handle) => {
                let sub = self.get(sub_handle);
                let body = self.get(sub.body);
                let mut value_rows: Option<Vec<Handle<FlatValuesRow>>> = None;
                if sub.with.is_none()
                    && body.order_by.is_none()
                    && body.limit_locking.is_none()
                    && let FlatSelectClause::Values(v) = self.get(body.clause)
                {
                    value_rows = Some(self.list1(self.get(*v).rows).to_vec());
                }
                match value_rows {
                    Some(value_rows) => {
                        let saved = self.expr_kind;
                        self.expr_kind = if value_rows.len() > 1 {
                            ExprKind::Values
                        } else {
                            ExprKind::ValuesSingle
                        };
                        let mut width: Option<usize> = None;
                        for row in value_rows {
                            let exprs = self.list1(self.get(row).values).to_vec();
                            let mut vals = Vec::with_capacity(exprs.len());
                            for e in &exprs {
                                if let FlatExpr::Default = self.get(*e) {
                                    vals.push(InsertValue::Default(self.span(*e).start));
                                } else {
                                    let t = self.expr(&scope, *e)?;
                                    vals.push(InsertValue::Expr(*e, t));
                                }
                            }
                            if let Some(w) = width
                                && w != vals.len()
                            {
                                let at = exprs
                                    .first()
                                    .map_or(self.span(row).start, |e| self.span(*e).start);
                                return Err(Diagnostic::new(
                                    "42601",
                                    "VALUES lists must all be the same length",
                                )
                                .at(at)
                                .into());
                            }
                            width = Some(vals.len());
                            rows.push(vals);
                        }
                        self.expr_kind = saved;
                    }
                    None => {
                        // INSERT ... SELECT: the query keeps its unknown
                        // literals, which are coerced to the columns.
                        let target_list = self.subquery_with(&scope, sub_handle, false, false)?;
                        let exprs = std::mem::take(&mut self.last_target_exprs);
                        let mut vals = Vec::new();
                        for (i, t) in target_list.iter().enumerate() {
                            if t.junk {
                                continue;
                            }
                            let form = if self.types().is_unknown(t.ty) {
                                match exprs.get(i).copied().flatten().map(|e| self.get(e)) {
                                    Some(FlatExpr::PositionalParam(p)) => {
                                        let text = self.text(self.get(*p).param);
                                        let number: usize =
                                            text.trim_start_matches('$').parse().unwrap_or(0);
                                        Form::Param { number }
                                    }
                                    _ => Form::Const {
                                        is_null: false,
                                        text: None,
                                    },
                                }
                            } else {
                                Form::Var
                            };
                            vals.push(InsertValue::Column(Typed {
                                ty: t.ty,
                                typmod: t.typmod,
                                form,
                                location: t.location,
                                returns_set: false,
                            }));
                        }
                        rows.push(vals);
                    }
                }
            }
        }
        // transformInsertRow
        for vals in &rows {
            if vals.len() > icolumns.len() {
                return Err(Diagnostic::new(
                    "42601",
                    "INSERT has more expressions than target columns",
                )
                .at(vals[icolumns.len()].location())
                .into());
            }
            if cols_given && vals.len() < icolumns.len() {
                return Err(Diagnostic::new(
                    "42601",
                    "INSERT has more target columns than expressions",
                )
                .at(icolumns[vals.len()].3)
                .into());
            }
            for (v, col) in vals.iter().zip(&icolumns) {
                self.assign(&scope, v, &col.0, col.1, col.2, &col.4, col.3)?;
            }
        }
        scope.namespace = vec![NamespaceItem::plain(rt_index)];
        let conflict_assigned = match stmt.on_conflict {
            Some(oc) => self.on_conflict(&mut scope, oc, relation, &relname)?,
            None => None,
        };
        let returning = self.returning(&scope, stmt.returning, ExprKind::Returning)?;
        // The rewriter's generated and identity column checks.
        let width = rows.iter().map(Vec::len).max().unwrap_or(0);
        let assigned: Vec<(i16, bool)> = attnums
            .iter()
            .take(width)
            .enumerate()
            .map(|(j, attnum)| {
                let all_default = rows
                    .iter()
                    .all(|vals| matches!(vals.get(j), Some(InsertValue::Default(_))));
                (*attnum, all_default)
            })
            .collect();
        let mut sets = vec![AssignedSet {
            assigned,
            override_kind,
            insert: true,
        }];
        if let Some(assigned) = conflict_assigned {
            sets.push(AssignedSet {
                assigned,
                override_kind: None,
                insert: false,
            });
        }
        self.rewrite_target(
            relation,
            DmlEvent::Insert,
            sets,
            stmt.returning.is_some(),
            stmt.on_conflict.is_some(),
            &[],
        )?;
        Ok(returning)
    }

    /// `transformOnConflictClause`: the arbiter, then for DO UPDATE the
    /// SET list and WHERE with `excluded` in the namespace. Returns the
    /// columns DO UPDATE assigns, for the rewriter's checks.
    fn on_conflict(
        &mut self,
        scope: &mut Scope,
        oc: Handle<FlatOnConflictClause>,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        relname: &str,
    ) -> Result<Option<Vec<(i16, bool)>>> {
        let clause = self.get(oc);
        let location = self.span(oc).start;
        let update = match self.get(clause.action) {
            FlatConflictAction::DoUpdate(u) => Some(*u),
            FlatConflictAction::DoNothing => None,
            #[cfg(feature = "since-pg19")]
            FlatConflictAction::DoSelect(d) => {
                let span = self.span(*d);
                return Err(unsupported(
                    "ON CONFLICT DO SELECT",
                    Some(span.start..span.end),
                ));
            }
        };
        let schema_name = {
            let rel = &self.catalog()[relation];
            self.catalog()[rel.schema].name.to_string()
        };
        // The `excluded` pseudo-relation joins the range table first.
        let excluded = match update {
            Some(_) => Some(self.relations.add_range_table_entry(
                scope,
                Some(&schema_name),
                relname,
                Some("excluded"),
                &[],
                false,
                location,
            )?),
            None => None,
        };
        if let Some(i) = excluded {
            // The pseudo-relation has the relkind of a composite type: no
            // system columns.
            scope.rtable[i - 1].composite_only = true;
        }
        // transformOnConflictArbiter
        if update.is_some() && clause.target.is_none() {
            return Err(Diagnostic::new(
                "42601",
                "ON CONFLICT DO UPDATE requires inference specification or constraint name",
            )
            .hint("For example, ON CONFLICT (column_name).")
            .at(location)
            .into());
        }
        if schema_name == "pg_catalog" || schema_name == "pg_toast" {
            return Err(Diagnostic::new(
                "0A000",
                "ON CONFLICT is not supported with system catalog tables",
            )
            .at(location)
            .into());
        }
        if let Some(target) = clause.target {
            match self.get(target) {
                FlatConflictTarget::Constraint(c) => {
                    // get_relation_constraint_attnos: the constraint must be
                    // one of the target table's.
                    let (name, _) = self.col_id(self.get(*c).name)?;
                    if self.catalog()[relation].constraint(&name).is_none() {
                        return Err(Diagnostic::new(
                            "42704",
                            format!("constraint \"{name}\" for table \"{relname}\" does not exist"),
                        )
                        .into());
                    }
                }
                FlatConflictTarget::Index(index) => {
                    let index = self.get(*index);
                    let list = index.columns;
                    // gram.y locates the inference clause at its parenthesis.
                    let start = self.span(list).start;
                    let infer_location = if self.source[start..].starts_with('(') {
                        start
                    } else {
                        self.source[..start].rfind('(').unwrap_or(start)
                    };
                    let items = self.list1(self.get(list).0).to_vec();
                    let saved = self.expr_kind;
                    self.expr_kind = ExprKind::IndexExpression;
                    for item in items {
                        let it = self.get(item);
                        // resolve_unique_index_expr: the element, then its
                        // collation, then its btree operator class.
                        let expr_location = match self.get(it.target) {
                            FlatIndexTarget::Col(c) => {
                                let (name, _) = self.col_id(*c)?;
                                if self
                                    .relations
                                    .col_name_to_var(scope, &self.outer, &name, infer_location)?
                                    .is_none()
                                {
                                    return Err(self
                                        .relations
                                        .error_missing_column(None, &name, infer_location)
                                        .into());
                                }
                                infer_location
                            }
                            // A parenthesized expression and a special
                            // expression are both an `a_expr` here.
                            FlatIndexTarget::Expr(e) | FlatIndexTarget::Special(e) => {
                                self.expr(scope, *e)?;
                                self.span(*e).start
                            }
                            FlatIndexTarget::Func(f) => {
                                let application = self.get(*f).application;
                                let func = self.function_item(*f, None)?;
                                let app = self.get(application);
                                self.call(
                                    scope,
                                    CallSite::From(application),
                                    func.name.clone(),
                                    app.body,
                                    None,
                                    None,
                                    None,
                                    func.location,
                                )?;
                                func.location
                            }
                        };
                        if let Some(cl) = it.collate {
                            self.lookup_collation(self.get(cl).name, expr_location)?;
                        }
                        if let Some(oc) = it.opclass {
                            let (opcname, _) = self.col_id(oc)?;
                            self.lookup_operator_class_parts(None, opcname, "btree")?;
                        }
                    }
                    self.expr_kind = saved;
                    self.where_clause_kind(scope, index.where_clause, ExprKind::IndexPredicate)?;
                }
            }
        }
        let Some(u) = update else {
            return Ok(None);
        };
        let excl_index = excluded.expect("excluded joins DO UPDATE");
        let u = self.get(u);
        scope.namespace.push(NamespaceItem::plain(excl_index));
        let assignments = self.list1(u.assignments).to_vec();
        let assigned = self.set_assignments(scope, relation, relname, &assignments)?;
        self.where_clause(scope, u.where_clause)?;
        scope.namespace.pop();
        Ok(Some(assigned))
    }

    /// `transformUpdateStmt`, returning the RETURNING list.
    fn update(&mut self, handle: Handle<FlatUpdateStmt>) -> Result<Vec<TargetEntry>> {
        let stmt = self.get(handle);
        let mut scope = self.level_scope();
        let alias = match stmt.alias.map(|a| self.get(a)) {
            None => None,
            Some(FlatUpdateTableAlias::WithAs(w)) => Some(self.col_id(self.get(*w).name)?.0),
            Some(FlatUpdateTableAlias::Bare(b)) => Some(self.relation_alias_name(*b)?),
        };
        let (table_name, inh) = self.relation_expr(stmt.relation);
        let rt_index = self.dml_target(&mut scope, table_name, alias, inh, true)?;
        let relation = scope.rtable[rt_index - 1]
            .relation()
            .expect("the target is a relation");
        let relname = self.catalog()[relation].name.to_string();
        scope.namespace[0].lateral_only = true;
        scope.namespace[0].lateral_ok = false;
        if let Some(from) = stmt.from_clause {
            self.transform_from_clause(&mut scope, from)?;
        }
        scope.namespace[0].lateral_only = false;
        scope.namespace[0].lateral_ok = true;
        self.where_or_current_clause(&scope, stmt.where_clause)?;
        let returning = self.returning(&scope, stmt.returning, ExprKind::Returning)?;
        // transformUpdateTargetList: every value first, then each column.
        let assignments = self.list1(self.get(stmt.assignments).0).to_vec();
        let assigned = self.set_assignments(&scope, relation, &relname, &assignments)?;
        self.rewrite_target(
            relation,
            DmlEvent::Update,
            vec![AssignedSet {
                assigned,
                override_kind: None,
                insert: false,
            }],
            stmt.returning.is_some(),
            false,
            &[],
        )?;
        Ok(returning)
    }

    /// `RewriteQuery` for the target relation of a data-modifying
    /// statement: the rewriter's generated-column checks, its rules, and
    /// the rewriting of an automatically updatable view onto its base
    /// relation, repeated for that relation.
    fn rewrite_target(
        &mut self,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        event: DmlEvent,
        sets: Vec<AssignedSet>,
        has_returning: bool,
        has_on_conflict: bool,
        merge_kinds: &[MergeKind],
    ) -> Result<()> {
        let mut relation = relation;
        let mut sets = sets;
        for depth in 0.. {
            if depth > 32 {
                return Err(unsupported("views nested too deeply", None));
            }
            // rewriteTargetListIU
            for set in &sets {
                self.generated_column_checks(
                    relation,
                    &set.assigned,
                    set.override_kind,
                    set.insert,
                )?;
            }
            let rel = &self.catalog()[relation];
            let relname = rel.name.to_string();
            // matchLocks
            let has_update_rule = rel.rules.iter().any(|r| r.event == RuleEvent::Update);
            let firing: Vec<&pgtk_catalog::Rule> =
                rel.rules.iter().filter(|r| r.fires.in_origin()).collect();
            if event == DmlEvent::Merge && !firing.is_empty() {
                return Err(Diagnostic::new(
                    "0A000",
                    format!("cannot execute MERGE on relation \"{relname}\""),
                )
                .detail("MERGE is not supported for relations with rules.")
                .into());
            }
            let rule_event = match event {
                DmlEvent::Insert => Some(RuleEvent::Insert),
                DmlEvent::Update => Some(RuleEvent::Update),
                DmlEvent::Delete => Some(RuleEvent::Delete),
                DmlEvent::Merge => None,
            };
            let matching: Vec<&pgtk_catalog::Rule> = firing
                .iter()
                .copied()
                .filter(|r| Some(r.event) == rule_event)
                .collect();
            let instead = matching.iter().any(|r| r.instead && r.unconditional);
            let qual_instead = matching.iter().any(|r| r.instead && !r.unconditional);
            let product = !matching.is_empty();
            let mut updatable_view = false;
            if !instead
                && rel.kind == RelationKind::View
                && !self.view_has_instead_trigger(relation, event, merge_kinds)
            {
                if qual_instead {
                    return Err(self.view_not_updatable(
                        relation,
                        event,
                        merge_kinds,
                        Some("Views with conditional DO INSTEAD rules are not automatically updatable."),
                    ));
                }
                // rewriteTargetView
                let info = self.view_info_of(relation)?;
                let insert_or_update = match event {
                    DmlEvent::Insert | DmlEvent::Update => true,
                    DmlEvent::Delete => false,
                    DmlEvent::Merge => merge_kinds
                        .iter()
                        .any(|k| matches!(k, MergeKind::Insert | MergeKind::Update)),
                };
                if let Some(detail) = info.auto_update_detail(event != DmlEvent::Delete) {
                    return Err(self.view_not_updatable(
                        relation,
                        event,
                        merge_kinds,
                        Some(detail),
                    ));
                }
                if insert_or_update {
                    // view_cols_are_auto_updatable: the first modified column
                    // that is not updatable, in the view's column order.
                    let modified: std::collections::HashSet<i16> = sets
                        .iter()
                        .flat_map(|set| set.assigned.iter().map(|(a, _)| *a))
                        .collect();
                    for (i, col) in info.columns.iter().enumerate() {
                        let attnum = i16::try_from(i + 1).expect("column count");
                        if let Err(reason) = col
                            && modified.contains(&attnum)
                        {
                            let colname = rel
                                .column_at(attnum)
                                .map_or_else(String::new, |c| c.name.to_string());
                            let message = match event {
                                DmlEvent::Insert => {
                                    format!(
                                        "cannot insert into column \"{colname}\" of view \"{relname}\""
                                    )
                                }
                                DmlEvent::Update => {
                                    format!(
                                        "cannot update column \"{colname}\" of view \"{relname}\""
                                    )
                                }
                                _ => format!(
                                    "cannot merge into column \"{colname}\" of view \"{relname}\""
                                ),
                            };
                            return Err(Diagnostic::new("0A000", message).detail(*reason).into());
                        }
                    }
                }
                if event == DmlEvent::Merge
                    && merge_kinds
                        .iter()
                        .any(|k| *k != MergeKind::Nothing && self.instead_trigger_for(relation, *k))
                {
                    return Err(Diagnostic::new(
                        "0A000",
                        format!("cannot merge into view \"{relname}\""),
                    )
                    .detail("MERGE is not supported for views with INSTEAD OF triggers for some actions but not all.")
                    .hint("To enable merging into the view, either provide a full set of INSTEAD OF triggers or drop the existing INSTEAD OF triggers.")
                    .into());
                }
                // The base relation takes the view's place.
                sets = sets
                    .into_iter()
                    .map(|set| AssignedSet {
                        assigned: set
                            .assigned
                            .iter()
                            .filter_map(|(a, d)| {
                                // rewriteTargetListIU replaces DEFAULT on a
                                // view column that has a default with that
                                // default; otherwise DEFAULT reaches the base.
                                let has_default = rel.column_at(*a).is_some_and(|c| c.has_default);
                                info.columns
                                    .get(usize::try_from(*a - 1).ok()?)
                                    .and_then(|c| c.ok())
                                    .map(|b| (b, *d && !has_default))
                            })
                            .collect(),
                        override_kind: set.override_kind,
                        insert: set.insert,
                    })
                    .collect();
                relation = info.base.expect("an updatable view has a base");
                updatable_view = true;
            } else if (instead || qual_instead) && has_returning {
                // The rewriter keeps the Statement's own RETURNING list, with
                // the view's columns replaced by the rule action's RETURNING
                // expressions, so the Result Descriptor is the list as written
                // against the view; without such an action it refuses.
                if !self.rule_returns(relation, event) {
                    let what = match event {
                        DmlEvent::Insert => "INSERT",
                        DmlEvent::Update => "UPDATE",
                        DmlEvent::Delete => "DELETE",
                        DmlEvent::Merge => "MERGE",
                    };
                    return Err(Diagnostic::new(
                        "0A000",
                        format!("cannot perform {what} RETURNING on relation \"{relname}\""),
                    )
                    .hint(format!(
                        "You need an unconditional ON {what} DO INSTEAD rule with a RETURNING clause."
                    ))
                    .into());
                }
            }
            if has_on_conflict && (product || has_update_rule) && !updatable_view {
                return Err(Diagnostic::new(
                    "0A000",
                    "INSERT with ON CONFLICT clause cannot be used with table that has INSERT or UPDATE rules",
                )
                .into());
            }
            if !updatable_view {
                return Ok(());
            }
        }
        Ok(())
    }

    /// `view_has_instead_trigger`.
    /// The parts of a possibly qualified name: at most a schema and a
    /// name, as `DeconstructQualifiedName` allows.
    fn schema_and_name(
        &self,
        qn: Handle<FlatAnyName>,
        at: usize,
    ) -> Result<(Option<String>, String)> {
        let mut parts = self.any_name_parts(qn)?;
        if parts.len() > 2 {
            return Err(Diagnostic::new(
                "42601",
                format!(
                    "improper qualified name (too many dotted names): {}",
                    parts.join(".")
                ),
            )
            .at(at)
            .into());
        }
        let name = parts.pop().expect("at least one part");
        Ok((parts.pop(), name))
    }

    /// `LookupCollation`: by name through the Search Path, for the
    /// database encoding.
    fn lookup_collation(
        &self,
        qn: Handle<FlatAnyName>,
        at: usize,
    ) -> Result<pgtk_catalog::Handle<Collation>> {
        let (schema, name) = self.schema_and_name(qn, at)?;
        let catalog = self.catalog();
        let found = match &schema {
            Some(schema) => catalog.collation_named(&pgtk_catalog::QualifiedName::new(
                schema.as_str(),
                name.as_str(),
            )),
            None => catalog.collation_visible(self.resolver.search_path, &name),
        };
        found.ok_or_else(|| {
            let written = match &schema {
                Some(schema) => format!("{schema}.{name}"),
                None => name.clone(),
            };
            Diagnostic::new(
                "42704",
                format!(
                    "collation \"{written}\" for encoding \"{}\" does not exist",
                    catalog.metadata().encoding.name
                ),
            )
            .at(at)
            .into()
        })
    }

    /// `get_opclass_oid` for an access method, without a position.
    fn lookup_operator_class_parts(
        &self,
        schema: Option<String>,
        name: String,
        method: &str,
    ) -> Result<pgtk_catalog::Handle<pgtk_catalog::OperatorClass>> {
        let catalog = self.catalog();
        let found = match &schema {
            Some(schema) => catalog.operator_class_named(
                &pgtk_catalog::QualifiedName::new(schema.as_str(), name.as_str()),
                method,
            ),
            None => catalog.operator_class_visible(self.resolver.search_path, &name, method),
        };
        found.ok_or_else(|| {
            let written = match &schema {
                Some(schema) => format!("{schema}.{name}"),
                None => name.clone(),
            };
            Diagnostic::new(
                "42704",
                format!(
                    "operator class \"{written}\" does not exist for access method \"{method}\""
                ),
            )
            .into()
        })
    }

    /// Whether a DO INSTEAD rule for the event has an action with a
    /// RETURNING list, read from the rule's definition.
    fn rule_returns(
        &self,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        event: DmlEvent,
    ) -> bool {
        let rule_event = match event {
            DmlEvent::Insert => RuleEvent::Insert,
            DmlEvent::Update => RuleEvent::Update,
            DmlEvent::Delete => RuleEvent::Delete,
            DmlEvent::Merge => return false,
        };
        self.catalog()[relation].rules.iter().any(|r| {
            r.event == rule_event
                && r.instead
                && parse_definition(&r.definition).is_some_and(|st| rule_action_returns(&st))
        })
    }

    fn view_has_instead_trigger(
        &self,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        event: DmlEvent,
        merge_kinds: &[MergeKind],
    ) -> bool {
        match event {
            DmlEvent::Insert => self.instead_trigger_for(relation, MergeKind::Insert),
            DmlEvent::Update => self.instead_trigger_for(relation, MergeKind::Update),
            DmlEvent::Delete => self.instead_trigger_for(relation, MergeKind::Delete),
            DmlEvent::Merge => merge_kinds
                .iter()
                .all(|k| *k == MergeKind::Nothing || self.instead_trigger_for(relation, *k)),
        }
    }

    /// Whether the relation has an INSTEAD OF row trigger for the command.
    fn instead_trigger_for(
        &self,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        kind: MergeKind,
    ) -> bool {
        self.catalog()[relation].triggers.iter().any(|t| {
            t.is_instead_of_row()
                && match kind {
                    MergeKind::Insert => t.insert,
                    MergeKind::Update => t.update,
                    MergeKind::Delete => t.delete,
                    MergeKind::Nothing => false,
                }
        })
    }

    /// `error_view_not_updatable`.
    fn view_not_updatable(
        &self,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        event: DmlEvent,
        merge_kinds: &[MergeKind],
        detail: Option<&'static str>,
    ) -> AnalyzeError {
        let relname = self.catalog()[relation].name.to_string();
        let (message, hint) = match event {
            DmlEvent::Insert => (
                format!("cannot insert into view \"{relname}\""),
                "To enable inserting into the view, provide an INSTEAD OF INSERT trigger or an unconditional ON INSERT DO INSTEAD rule.".to_owned(),
            ),
            DmlEvent::Update => (
                format!("cannot update view \"{relname}\""),
                "To enable updating the view, provide an INSTEAD OF UPDATE trigger or an unconditional ON UPDATE DO INSTEAD rule.".to_owned(),
            ),
            DmlEvent::Delete => (
                format!("cannot delete from view \"{relname}\""),
                "To enable deleting from the view, provide an INSTEAD OF DELETE trigger or an unconditional ON DELETE DO INSTEAD rule.".to_owned(),
            ),
            DmlEvent::Merge => {
                let missing = merge_kinds
                    .iter()
                    .find(|k| **k != MergeKind::Nothing && !self.instead_trigger_for(relation, **k))
                    .copied()
                    .unwrap_or(MergeKind::Nothing);
                match missing {
                    MergeKind::Update => (
                        format!("cannot update view \"{relname}\""),
                        "To enable updating the view using MERGE, provide an INSTEAD OF UPDATE trigger.".to_owned(),
                    ),
                    MergeKind::Delete => (
                        format!("cannot delete from view \"{relname}\""),
                        "To enable deleting from the view using MERGE, provide an INSTEAD OF DELETE trigger.".to_owned(),
                    ),
                    _ => (
                        format!("cannot insert into view \"{relname}\""),
                        "To enable inserting into the view using MERGE, provide an INSTEAD OF INSERT trigger.".to_owned(),
                    ),
                }
            }
        };
        let mut d = Diagnostic::new("55000", message).hint(hint);
        if let Some(detail) = detail {
            d = d.detail(detail);
        }
        d.into()
    }

    /// The view's query analysed on its own, with the search path the
    /// Catalog was loaded under, as `pg_get_viewdef` wrote it.
    fn view_info_of(
        &self,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
    ) -> Result<ViewInfo> {
        let catalog = self.catalog();
        let Some(text) = catalog[relation].view_definition.as_deref() else {
            return Err(unsupported("a view without a definition", None));
        };
        let mut settings = self.settings.clone();
        settings.search_path = pgtk_catalog::SearchPath::new(
            catalog,
            catalog.metadata().search_path.iter().map(|n| n.as_str()),
        );
        let analyzer = Analyzer::new(catalog, settings);
        analyzer.view_info(text).map_err(|e| match e {
            AnalyzeError::Diagnostic(d) => {
                unsupported(format!("view definition: {}", d.message), None)
            }
            other => other,
        })
    }

    /// The shape of this Walker's statement as a view query, after
    /// `statement()` analysed it.
    fn view_shape(&self) -> Result<ViewInfo> {
        let root = self.ast.root();
        let (mut cte, mut body) = match self.get(root) {
            FlatStatement::Query(b) => (false, *b),
            FlatStatement::With(w) => {
                let ws = self.get(*w);
                match self.get(ws.body) {
                    FlatWithBody::Query(b) => (true, *b),
                    _ => return Err(unsupported("view definition form", None)),
                }
            }
            _ => return Err(unsupported("view definition form", None)),
        };
        let mut limit = false;
        let mut setop = false;
        let select = loop {
            let qb = self.get(body);
            let (own_limit, _) = self.limit_locking(qb.limit_locking);
            limit |= own_limit.is_some();
            match self.get(qb.clause) {
                FlatSelectClause::Parens(p) => {
                    let inner = self.get(self.get(*p).inner);
                    cte |= inner.with.is_some();
                    body = inner.body;
                }
                FlatSelectClause::Select(s) => break Some(*s),
                FlatSelectClause::Union(..)
                | FlatSelectClause::Except(..)
                | FlatSelectClause::Intersect(..) => {
                    setop = true;
                    break None;
                }
                FlatSelectClause::Values(_) | FlatSelectClause::Table(_) => break None,
            }
        };
        let single_table =
            "Views that do not select from a single table or view are not automatically updatable.";
        let Some(select) = select else {
            let detail = if setop {
                "Views containing UNION, INTERSECT, or EXCEPT are not automatically updatable."
            } else if cte {
                "Views containing WITH are not automatically updatable."
            } else if limit {
                "Views containing LIMIT or OFFSET are not automatically updatable."
            } else {
                single_table
            };
            return Ok(ViewInfo {
                detail: Some(detail),
                base: None,
                columns: Vec::new(),
            });
        };
        let stmt = self.get(select);
        let (distinct, items, from) = match stmt.head.map(|h| self.get(h)) {
            None => (false, Vec::new(), None),
            Some(FlatSelectHead::Distinct(d)) => {
                let l = self.get(self.get(*d).targets);
                (true, self.list1(l.items).to_vec(), l.from_clause)
            }
            Some(FlatSelectHead::DistinctOn(d)) => {
                let l = self.get(self.get(*d).targets);
                (true, self.list1(l.items).to_vec(), l.from_clause)
            }
            Some(FlatSelectHead::Plain(t)) => self.view_targets(*t),
            Some(FlatSelectHead::All(a)) => match self.get(*a).targets {
                Some(t) => self.view_targets(t),
                None => (false, Vec::new(), None),
            },
        };
        let detail = if distinct {
            Some("Views containing DISTINCT are not automatically updatable.")
        } else if stmt.group_by.is_some() {
            Some("Views containing GROUP BY are not automatically updatable.")
        } else if stmt.having.is_some() {
            Some("Views containing HAVING are not automatically updatable.")
        } else if cte {
            Some("Views containing WITH are not automatically updatable.")
        } else if limit {
            Some("Views containing LIMIT or OFFSET are not automatically updatable.")
        } else if self.has_aggs {
            Some("Views that return aggregate functions are not automatically updatable.")
        } else if self.has_window_funcs {
            Some("Views that return window functions are not automatically updatable.")
        } else if self.has_target_srfs {
            Some("Views that return set-returning functions are not automatically updatable.")
        } else {
            None
        };
        if let Some(detail) = detail {
            return Ok(ViewInfo {
                detail: Some(detail),
                base: None,
                columns: Vec::new(),
            });
        }
        // One FROM item that is a table or view, without joins.
        let tables: Vec<Handle<FlatTableRef>> = from
            .map(|f| self.list1(self.get(f).tables).to_vec())
            .unwrap_or_default();
        let single = match tables.as_slice() {
            [t] => {
                let tr = self.get(*t);
                self.list(tr.joins).is_empty()
                    && match self.get(tr.base) {
                        FlatSimpleTableRef::Named(n) => !matches!(
                            self.get(*n).tail.map(|t| self.get(t)),
                            Some(FlatNamedTableRefTail::Function(_))
                        ),
                        FlatSimpleTableRef::Only(_) | FlatSimpleTableRef::ColName(_) => true,
                        _ => false,
                    }
            }
            _ => false,
        };
        let rtable = self.analysis.range_tables.get(select);
        let base = if single {
            rtable
                .and_then(|r| r.first())
                .and_then(crate::relation::RangeTableEntry::relation)
                .filter(|b| {
                    matches!(
                        self.catalog()[*b].kind,
                        RelationKind::Table
                            | RelationKind::ForeignTable
                            | RelationKind::View
                            | RelationKind::PartitionedTable
                    )
                })
        } else {
            None
        };
        let Some(base) = base else {
            return Ok(ViewInfo {
                detail: Some(single_table),
                base: None,
                columns: Vec::new(),
            });
        };
        if self.base_has_tablesample(self.get(tables[0]).base) {
            return Ok(ViewInfo {
                detail: Some("Views containing TABLESAMPLE are not automatically updatable."),
                base: Some(base),
                columns: Vec::new(),
            });
        }
        // view_col_is_auto_updatable per output column.
        let mut columns = Vec::new();
        for item in items {
            match self.get(item) {
                FlatSelectItem::Star(_) => {
                    let entries = self
                        .analysis
                        .star_expansions
                        .get(item)
                        .cloned()
                        .unwrap_or_default();
                    for e in entries {
                        columns.push(match e.origin {
                            Some((r, attnum)) if r == base && attnum > 0 => Ok(attnum),
                            _ => Err("View columns that are not columns of their base relation are not updatable."),
                        });
                    }
                }
                FlatSelectItem::Expr(e) => {
                    let expr = self.get(*e).expr;
                    columns.push(match self.plain_var(expr) {
                        Some(var) if var.varno == 1 && var.levelsup == 0 => {
                            if var.varattno < 0 {
                                Err("View columns that refer to system columns are not updatable.")
                            } else if var.varattno == 0 {
                                Err("View columns that return whole-row references are not updatable.")
                            } else {
                                Ok(var.varattno)
                            }
                        }
                        _ => Err("View columns that are not columns of their base relation are not updatable."),
                    });
                }
            }
        }
        Ok(ViewInfo {
            detail: None,
            base: Some(base),
            columns,
        })
    }

    /// Whether a relation form in FROM carries a TABLESAMPLE clause.
    fn base_has_tablesample(&self, base: Handle<FlatSimpleTableRef>) -> bool {
        match self.get(base) {
            FlatSimpleTableRef::Named(n) => match self.get(*n).tail.map(|t| self.get(t)) {
                Some(FlatNamedTableRefTail::Alias(a)) => self.get(*a).tablesample.is_some(),
                Some(FlatNamedTableRefTail::Inherited(i)) => self.get(*i).tablesample.is_some(),
                Some(FlatNamedTableRefTail::Sample(_)) => true,
                _ => false,
            },
            FlatSimpleTableRef::ColName(c) => match self.get(*c).tail.map(|t| self.get(t)) {
                Some(FlatColNameTableTail::Alias(a)) => self.get(*a).tablesample.is_some(),
                Some(FlatColNameTableTail::Inherited(i)) => self.get(*i).tablesample.is_some(),
                Some(FlatColNameTableTail::Sample(_)) => true,
                _ => false,
            },
            FlatSimpleTableRef::Only(o) => self.get(*o).tablesample.is_some(),
            _ => false,
        }
    }

    /// The items and FROM clause of a plain or ALL target list.
    fn view_targets(
        &self,
        targets: Handle<FlatSelectTargets>,
    ) -> (
        bool,
        Vec<Handle<FlatSelectItem>>,
        Option<Handle<FlatFromClause>>,
    ) {
        match self.get(targets) {
            FlatSelectTargets::Items(l) => {
                let l = self.get(*l);
                (false, self.list1(l.items).to_vec(), l.from_clause)
            }
            FlatSelectTargets::Into(i) => (false, Vec::new(), self.get(*i).from_clause),
            FlatSelectTargets::Empty(f) => (false, Vec::new(), Some(*f)),
        }
    }

    /// `transformUpdateTargetList`: every value first, then each column;
    /// returns each assigned column with whether its value is `DEFAULT`.
    fn set_assignments(
        &mut self,
        scope: &Scope,
        relation: pgtk_catalog::Handle<pgtk_catalog::Relation>,
        relname: &str,
        assignments: &[Handle<FlatSetAssignment>],
    ) -> Result<Vec<(i16, bool)>> {
        // Each target: its column handle, its indirection, and its value.
        let mut values: Vec<(
            Handle<pgtk_sql::tokens::literal::FlatIdent>,
            Vec<Handle<FlatIndirectionEl>>,
            InsertValue,
        )> = Vec::new();
        self.expr_kind = ExprKind::UpdateSource;
        for a in assignments {
            match self.get(*a) {
                FlatSetAssignment::Tuple(t) => {
                    // transformMultiAssignRef: a ROW() or a sub-SELECT with
                    // one value per target.
                    let t = self.get(*t);
                    let targets = self.list1(self.get(t.columns).0).to_vec();
                    let source = t.values;
                    let at = self.span(source).start;
                    let sources: Vec<InsertValue> = if let Some(elements) =
                        self.row_elements(source)
                    {
                        // transformRowExpr with DEFAULT allowed and `t.*`
                        // expanded to the columns.
                        let mut values = Vec::new();
                        for e in elements {
                            if let FlatExpr::Default = self.get(e) {
                                values.push(InsertValue::Default(self.span(e).start));
                            } else if let Some(entries) = self.star_columns(scope, e)? {
                                for entry in entries {
                                    values.push(InsertValue::Column(Typed {
                                        ty: entry.ty,
                                        typmod: entry.typmod,
                                        form: Form::Var,
                                        location: entry.location,
                                        returns_set: false,
                                    }));
                                }
                            } else {
                                let typed = self.expr(scope, e)?;
                                values.push(InsertValue::Expr(e, typed));
                            }
                        }
                        if values.len() != targets.len() {
                            return Err(Diagnostic::new(
                                "42601",
                                "number of columns does not match number of values",
                            )
                            .at(at)
                            .into());
                        }
                        values
                    } else if let FlatExpr::SubSelect(q) = self.get(source) {
                        let sub = self.get(*q).inner;
                        self.sublinks.insert(source);
                        let columns = self.sublink_columns(scope, source, sub)?;
                        if columns.len() != targets.len() {
                            return Err(Diagnostic::new(
                                "42601",
                                "number of columns does not match number of values",
                            )
                            .at(at)
                            .into());
                        }
                        columns
                            .iter()
                            .map(|c| {
                                InsertValue::Column(Typed {
                                    ty: c.ty,
                                    typmod: c.typmod,
                                    form: Form::Other,
                                    location: at,
                                    returns_set: false,
                                })
                            })
                            .collect()
                    } else {
                        return Err(Diagnostic::new(
                            "0A000",
                            "source for a multiple-column UPDATE item must be a sub-SELECT or ROW() expression",
                        )
                        .at(self.span(self.unparen(source)).start)
                        .into());
                    };
                    for (target, value) in targets.into_iter().zip(sources) {
                        let target = self.get(target);
                        values.push((
                            target.column,
                            self.assignment_indirection(target.indirection)?,
                            value,
                        ));
                    }
                }
                FlatSetAssignment::Single(single) => {
                    let single = self.get(*single);
                    let value = if let FlatExpr::Default = self.get(single.value) {
                        InsertValue::Default(self.span(single.value).start)
                    } else {
                        InsertValue::Expr(single.value, self.expr(scope, single.value)?)
                    };
                    values.push((
                        single.column,
                        self.assignment_indirection(single.indirection)?,
                        value,
                    ));
                }
            }
        }
        let mut assigned: Vec<(i16, bool)> = Vec::new();
        for (column, parts, value) in values {
            let span = self.span(column);
            let name = self.fold(&self.source[span.clone()], span.start)?;
            let Some((attnum, ty, typmod)) = self.target_attnum(relation, &name, true) else {
                return Err(Diagnostic::new(
                    "42703",
                    format!("column \"{name}\" of relation \"{relname}\" does not exist"),
                )
                .at(span.start)
                .into());
            };
            if attnum <= 0 {
                return Err(Diagnostic::new(
                    "0A000",
                    format!("cannot assign to system column \"{name}\""),
                )
                .at(span.start)
                .into());
            }
            self.assign(scope, &value, &name, ty, typmod, &parts, span.start)?;
            assigned.push((attnum, matches!(value, InsertValue::Default(_))));
        }
        Ok(assigned)
    }

    /// `transformMergeStmt`, returning the RETURNING list.
    #[cfg(feature = "since-pg15")]
    fn merge(&mut self, handle: Handle<FlatMergeStmt>) -> Result<Vec<TargetEntry>> {
        let stmt = self.get(handle);
        // Which kinds of WHEN clause are possible, and terminal ones.
        let mut terminal = [false; 3];
        let whens: Vec<Handle<FlatWhenClause>> = self.list1(stmt.when_clauses).to_vec();
        for w in &whens {
            // gram.y groups WHEN NOT MATCHED BY SOURCE with WHEN MATCHED,
            // because its actions need a target row.
            let (kind, unconditional) = match self.get(*w) {
                FlatWhenClause::Matched(m) => {
                    let m = self.get(*m);
                    let kind = match self.get(m.kind) {
                        FlatMatchedKind::Matched => 0usize,
                        #[cfg(feature = "since-pg17")]
                        FlatMatchedKind::NotMatchedBySource => 1,
                    };
                    (kind, m.and.is_none())
                }
                FlatWhenClause::NotMatched(n) => (2usize, self.get(*n).and.is_none()),
            };
            if terminal[kind] {
                return Err(Diagnostic::new(
                    "42601",
                    "unreachable WHEN clause specified after unconditional WHEN clause",
                )
                .into());
            }
            if unconditional {
                terminal[kind] = true;
            }
        }
        let mut scope = self.level_scope();
        let target = self.get(stmt.target);
        let alias_name = match target.alias.map(|a| self.get(a)) {
            None => None,
            Some(FlatMergeTableAlias::WithAs(w)) => Some(self.col_id(self.get(*w).name)?.0),
            Some(FlatMergeTableAlias::Bare(b)) => Some(self.relation_alias_name(*b)?),
        };
        let (target_name, inh) = self.relation_expr(target.relation);
        let rt_index = self.dml_target(&mut scope, target_name, alias_name, inh, false)?;
        let relation = scope.rtable[rt_index - 1]
            .relation()
            .expect("the target is a relation");
        let relname = self.catalog()[relation].name.to_string();
        // The source, then the target, in the namespace.
        self.transform_from_items(&mut scope, &[stmt.source])?;
        let source_index = scope.rtable.len();
        let target_name = scope.rtable[rt_index - 1].eref_name.clone();
        if scope.rtable[source_index - 1].eref_name == target_name {
            return Err(Diagnostic::new(
                "42712",
                format!("name \"{target_name}\" specified more than once"),
            )
            .detail("The name is used both as MERGE target table and data source.")
            .into());
        }
        scope.namespace.push(NamespaceItem::plain(rt_index));
        self.expr_kind = ExprKind::JoinOn;
        self.expr(&scope, stmt.condition)?;
        // MERGE ... RETURNING is from 17.
        #[cfg(feature = "since-pg17")]
        let (returning, has_returning) = (
            self.returning(&scope, stmt.returning, ExprKind::MergeReturning)?,
            stmt.returning.is_some(),
        );
        #[cfg(not(feature = "since-pg17"))]
        let (returning, has_returning) = (Vec::new(), false);
        // Each WHEN clause sees the target, the source, or both.
        let mut sets: Vec<AssignedSet> = Vec::new();
        let mut kinds: Vec<MergeKind> = Vec::new();
        for w in whens {
            kinds.push(match self.get(w) {
                FlatWhenClause::Matched(m) => match self.get(self.get(*m).action) {
                    FlatMatchedAction::Update(_) => MergeKind::Update,
                    FlatMatchedAction::Delete => MergeKind::Delete,
                    FlatMatchedAction::DoNothing => MergeKind::Nothing,
                },
                FlatWhenClause::NotMatched(n) => match self.get(self.get(*n).action) {
                    FlatNotMatchedAction::Insert(_) => MergeKind::Insert,
                    FlatNotMatchedAction::DoNothing => MergeKind::Nothing,
                },
            });
            let (target_visible, source_visible, condition, action) = match self.get(w) {
                FlatWhenClause::Matched(m) => {
                    let m = self.get(*m);
                    // WHEN NOT MATCHED BY SOURCE sees the target only.
                    #[cfg(feature = "since-pg17")]
                    let by_source = matches!(self.get(m.kind), FlatMatchedKind::NotMatchedBySource);
                    #[cfg(not(feature = "since-pg17"))]
                    let by_source = false;
                    let action = match self.get(m.action) {
                        FlatMatchedAction::Update(u) => MergeAction::Update(*u),
                        FlatMatchedAction::Delete | FlatMatchedAction::DoNothing => {
                            MergeAction::None
                        }
                    };
                    (true, !by_source, m.and, action)
                }
                FlatWhenClause::NotMatched(n) => {
                    let n = self.get(*n);
                    let action = match self.get(n.action) {
                        FlatNotMatchedAction::Insert(i) => MergeAction::Insert(*i),
                        FlatNotMatchedAction::DoNothing => MergeAction::None,
                    };
                    (false, true, n.and, action)
                }
            };
            // setNamespaceVisibilityForRTE on the target and the source.
            for item in &mut scope.namespace {
                let visible = if item.rt_index == rt_index {
                    target_visible
                } else if item.rt_index == source_index {
                    source_visible
                } else {
                    continue;
                };
                item.rel_visible = visible;
                item.cols_visible = visible;
            }
            if let Some(c) = condition {
                let expr = self.get(c).condition;
                self.expr_kind = ExprKind::MergeWhen;
                let t = self.expr(&scope, expr)?;
                let (_, steps) = self.coercer().coerce_to_boolean(&t, "WHEN")?;
                self.record_coercions(expr, steps);
            }
            match action {
                MergeAction::None => {}
                MergeAction::Update(u) => {
                    let assignments = self.list1(self.get(u).assignments).to_vec();
                    let assigned =
                        self.set_assignments(&scope, relation, &relname, &assignments)?;
                    sets.push(AssignedSet {
                        assigned,
                        override_kind: None,
                        insert: false,
                    });
                }
                MergeAction::Insert(i) => {
                    // `INSERT DEFAULT VALUES` takes no column list, no
                    // OVERRIDING and no values.
                    let (columns, overriding, values) = match self.get(i) {
                        FlatInsertAction::Default => (None, None, None),
                        FlatInsertAction::Values(v) => {
                            let v = self.get(*v);
                            (v.columns, v.overriding, Some(v.values))
                        }
                    };
                    let override_kind = overriding.map(|o| *self.get(self.get(o).which));
                    // checkInsertTargets
                    let mut icolumns: Vec<InsertColumn> = Vec::new();
                    let mut attnums: Vec<i16> = Vec::new();
                    let cols_given = columns.is_some();
                    match columns {
                        Some(c) => {
                            let mut seen = std::collections::HashSet::new();
                            for a in self.list1(self.get(c).0) {
                                let (name, loc) = self.alias_name(*a)?;
                                let Some((attnum, ty, typmod)) =
                                    self.target_attnum(relation, &name, false)
                                else {
                                    return Err(Diagnostic::new(
                                        "42703",
                                        format!("column \"{name}\" of relation \"{relname}\" does not exist"),
                                    )
                                    .at(loc)
                                    .into());
                                };
                                if !seen.insert(attnum) {
                                    return Err(Diagnostic::new(
                                        "42701",
                                        format!("column \"{name}\" specified more than once"),
                                    )
                                    .at(loc)
                                    .into());
                                }
                                icolumns.push((name, ty, typmod, loc, Vec::new()));
                                attnums.push(attnum);
                            }
                        }
                        None => {
                            let rel = &self.catalog()[relation];
                            let mut cols: Vec<_> = rel.columns.iter().collect();
                            cols.sort_by_key(|c| c.position);
                            for c in cols {
                                icolumns.push((
                                    c.name.to_string(),
                                    c.ty,
                                    c.typmod.0,
                                    NO_LOCATION,
                                    Vec::new(),
                                ));
                                attnums.push(c.position);
                            }
                        }
                    }
                    let mut vals: Vec<InsertValue> = Vec::new();
                    if let Some(values) = values {
                        // gram.y's `merge_values_clause` is one row.
                        self.expr_kind = ExprKind::ValuesSingle;
                        for e in self.list1(self.get(values).values).to_vec() {
                            if let FlatExpr::Default = self.get(e) {
                                vals.push(InsertValue::Default(self.span(e).start));
                            } else {
                                let t = self.expr(&scope, e)?;
                                vals.push(InsertValue::Expr(e, t));
                            }
                        }
                    }
                    if vals.len() > icolumns.len() {
                        return Err(Diagnostic::new(
                            "42601",
                            "INSERT has more expressions than target columns",
                        )
                        .at(vals[icolumns.len()].location())
                        .into());
                    }
                    if cols_given && vals.len() < icolumns.len() {
                        return Err(Diagnostic::new(
                            "42601",
                            "INSERT has more target columns than expressions",
                        )
                        .at(icolumns[vals.len()].3)
                        .into());
                    }
                    for (v, col) in vals.iter().zip(&icolumns) {
                        self.assign(&scope, v, &col.0, col.1, col.2, &col.4, col.3)?;
                    }
                    let assigned: Vec<(i16, bool)> = attnums
                        .iter()
                        .take(vals.len())
                        .enumerate()
                        .map(|(j, attnum)| (*attnum, matches!(vals[j], InsertValue::Default(_))))
                        .collect();
                    sets.push(AssignedSet {
                        assigned,
                        override_kind,
                        insert: true,
                    });
                }
            }
        }
        self.rewrite_target(
            relation,
            DmlEvent::Merge,
            sets,
            has_returning,
            false,
            &kinds,
        )?;
        Ok(returning)
    }

    /// `transformDeleteStmt`, returning the RETURNING list.
    fn delete(&mut self, handle: Handle<FlatDeleteStmt>) -> Result<Vec<TargetEntry>> {
        let stmt = self.get(handle);
        let mut scope = self.level_scope();
        let alias = match stmt.alias.map(|a| self.get(a)) {
            None => None,
            Some(FlatDeleteTableAlias::WithAs(w)) => Some(self.col_id(self.get(*w).name)?.0),
            Some(FlatDeleteTableAlias::Bare(b)) => Some(self.relation_alias_name(*b)?),
        };
        let (table_name, inh) = self.relation_expr(stmt.relation);
        let rt_index = self.dml_target(&mut scope, table_name, alias, inh, true)?;
        let relation = scope.rtable[rt_index - 1]
            .relation()
            .expect("the target is a relation");
        scope.namespace[0].lateral_only = true;
        scope.namespace[0].lateral_ok = false;
        if let Some(using) = stmt.using_clause {
            let tables = self.list1(self.get(using).tables).to_vec();
            self.transform_from_items(&mut scope, &tables)?;
        }
        scope.namespace[0].lateral_only = false;
        scope.namespace[0].lateral_ok = true;
        self.where_or_current_clause(&scope, stmt.where_clause)?;
        let returning = self.returning(&scope, stmt.returning, ExprKind::Returning)?;
        self.rewrite_target(
            relation,
            DmlEvent::Delete,
            Vec::new(),
            stmt.returning.is_some(),
            false,
            &[],
        )?;
        Ok(returning)
    }

    /// `transformExplainStmt` plus `ExplainResultDesc`.
    fn explain(&mut self, handle: Handle<FlatExplainStmt>) -> Result<()> {
        let stmt = self.get(handle);
        let (options, statement) = match self.get(stmt.input) {
            FlatExplainInput::WithOptions(o) => {
                let o = self.get(*o);
                (Some(o.options), o.statement)
            }
            FlatExplainInput::Statement(s) => (None, *s),
        };
        let mut result_type = self.types().builtin(Builtin::Text);
        if let Some(opts) = options {
            for opt in self.list1(self.get(opts).options) {
                let opt = self.get(*opt);
                let is_format = match self.get(opt.name) {
                    FlatUtilityOptionName::Text(t) => self.text(*t).eq_ignore_ascii_case("format"),
                    // `FORMAT` before `JSON` is the merged kind `FORMAT_LA`.
                    #[cfg(feature = "since-pg16")]
                    FlatUtilityOptionName::Format => true,
                };
                if !is_format {
                    continue;
                }
                let value = match opt.value.map(|v| self.get(v)) {
                    Some(FlatExplainOptValue::Ident(c)) => self.col_id(*c)?.0,
                    Some(FlatExplainOptValue::String(t)) => string_literal_body(self.text(*t)),
                    _ => String::new(),
                };
                result_type = match value.as_str() {
                    "xml" => self.types().builtin(Builtin::Xml),
                    "json" => self.types().builtin(Builtin::Json),
                    _ => self.types().builtin(Builtin::Text),
                };
            }
        }
        match self.get(statement) {
            FlatExplainableStmt::Query(b) => {
                self.allow_into = true;
                self.query_body(*b, None, None, Vec::new())?;
                self.allow_into = false;
            }
            FlatExplainableStmt::Insert(i) => {
                self.insert(*i)?;
            }
            FlatExplainableStmt::Update(u) => {
                self.update(*u)?;
            }
            FlatExplainableStmt::Delete(d) => {
                self.delete(*d)?;
            }
            #[cfg(feature = "since-pg15")]
            FlatExplainableStmt::Merge(m) => {
                self.merge(*m)?;
            }
            FlatExplainableStmt::With(w) => {
                self.with_statement(*w)?;
            }
            // EXPLAIN EXECUTE looks the name up only when it runs.
            FlatExplainableStmt::Execute(_) => {}
            FlatExplainableStmt::Declare(d) => {
                self.declare(*d)?;
            }
            FlatExplainableStmt::CreateTable(t) => {
                self.create_table_as(self.get(self.get(*t).body).source)?;
            }
            FlatExplainableStmt::CreateMaterializedView(m) => {
                self.create_matview(*m)?;
            }
            FlatExplainableStmt::Refresh(_) => return Err(unsupported("REFRESH", None)),
        }
        // EXPLAIN runs nothing, so a DECLARE inside it opens no cursor.
        self.analysis.session_effect = None;
        self.analysis.result = ResultDescriptor {
            columns: vec![ResultColumn {
                name: "QUERY PLAN".to_owned(),
                ty: result_type,
                typmod: -1,
                origin: None,
            }],
            params: self.finish_params()?,
        };
        Ok(())
    }

    /// A Utility Statement: parse analysis leaves it as it is, and a
    /// Describe message answers with no columns.
    fn utility(&mut self) -> Result<()> {
        self.analysis.utility = true;
        self.analysis.result = ResultDescriptor {
            columns: Vec::new(),
            params: self.finish_params()?,
        };
        Ok(())
    }

    /// `check_variable_parameters`.
    fn finish_params(&self) -> Result<Vec<TypeRef>> {
        let params = self.coercer().params.borrow();
        let mut out = Vec::with_capacity(params.len());
        for (i, p) in params.iter().enumerate() {
            match p {
                Some(t) if !self.types().is_unknown(*t) => out.push(*t),
                _ => {
                    return Err(Diagnostic::new(
                        "42P18",
                        format!("could not determine data type of parameter ${}", i + 1),
                    )
                    .into());
                }
            }
        }
        Ok(out)
    }

    /// `transformSelectStmt` for one plain SELECT. Returns the target list.
    fn select(
        &mut self,
        handle: Handle<FlatSelectStmt>,
        order_by: Option<Handle<FlatOrderByClause>>,
        limit_offset: Option<Handle<FlatLimitOffsetClause>>,
        locking: &[Handle<FlatForUpdateClause>],
    ) -> Result<Vec<TargetEntry>> {
        let stmt = self.get(handle);
        let mut scope = self.level_scope();
        self.windows.clear();
        self.has_window_funcs = false;
        self.has_target_srfs = false;
        self.grouping_calls.clear();
        self.window_locations.clear();
        if let Some(w) = stmt.window {
            for def in self.list1(self.get(w).defs) {
                let from = self.span(*def).start;
                let def = self.get(*def);
                let (name, _) = self.col_id(def.name)?;
                let mut wd = self.window_spec(def.spec, from)?;
                wd.name = Some(name);
                self.windows.push(wd);
            }
        }
        let mut distinct = false;
        let mut distinct_on: Option<Vec<Handle<FlatExpr>>> = None;
        let (items, from) = match stmt.head {
            None => (None, None),
            Some(head) => match self.get(head) {
                FlatSelectHead::DistinctOn(d) => {
                    let d = self.get(*d);
                    distinct_on = Some(self.list1(self.get(d.qualifier).exprs).to_vec());
                    let list = self.get(d.targets);
                    if let Some(into) = list.into {
                        self.select_into(into)?;
                    }
                    (Some(list.items), list.from_clause)
                }
                FlatSelectHead::Distinct(d) => {
                    distinct = true;
                    let list = self.get(self.get(*d).targets);
                    if let Some(into) = list.into {
                        self.select_into(into)?;
                    }
                    (Some(list.items), list.from_clause)
                }
                FlatSelectHead::All(a) => match self.get(*a).targets {
                    None => (None, None),
                    Some(targets) => match self.get(targets) {
                        FlatSelectTargets::Items(list) => {
                            let list = self.get(*list);
                            if let Some(into) = list.into {
                                self.select_into(into)?;
                            }
                            (Some(list.items), list.from_clause)
                        }
                        FlatSelectTargets::Empty(from) => (None, Some(*from)),
                        FlatSelectTargets::Into(i) => {
                            let i = self.get(*i);
                            self.select_into(i.into)?;
                            (None, i.from_clause)
                        }
                    },
                },
                FlatSelectHead::Plain(targets) => match self.get(*targets) {
                    FlatSelectTargets::Items(list) => {
                        let list = self.get(*list);
                        if let Some(into) = list.into {
                            self.select_into(into)?;
                        }
                        (Some(list.items), list.from_clause)
                    }
                    FlatSelectTargets::Empty(from) => (None, Some(*from)),
                    FlatSelectTargets::Into(i) => {
                        let i = self.get(*i);
                        self.select_into(i.into)?;
                        (None, i.from_clause)
                    }
                },
            },
        };
        if let Some(from) = from {
            self.transform_from_clause(&mut scope, from)?;
        }
        let mut target_list = Vec::new();
        let mut target_exprs: Vec<Option<Handle<FlatExpr>>> = Vec::new();
        self.expr_kind = ExprKind::SelectTarget;
        if let Some(items) = items {
            for item in self.list1(items) {
                let before = target_list.len();
                self.target_item(&scope, *item, &mut target_list)?;
                let expr = match self.get(*item) {
                    FlatSelectItem::Expr(e) if target_list.len() == before + 1 => {
                        Some(self.get(*e).expr)
                    }
                    _ => None,
                };
                target_exprs.resize(before, None);
                target_exprs.push(expr);
                target_exprs.resize(target_list.len(), None);
            }
        }
        // markTargetListOrigins happened as entries were made.
        self.where_clause(&scope, stmt.where_clause)?;
        // HAVING, then the sort and group clauses, as transformSelectStmt orders them.
        let has_distinct = distinct || distinct_on.is_some();
        let mut having_expr = None;
        if let Some(having) = stmt.having {
            let expr = self.get(having).condition;
            self.expr_kind = ExprKind::Having;
            let typed = self.expr(&scope, expr)?;
            let (_, steps) = self.coercer().coerce_to_boolean(&typed, "HAVING")?;
            self.record_coercions(expr, steps);
            having_expr = Some(expr);
        }
        let sort_refs = self.sort_clause(&scope, order_by, &mut target_list, &mut target_exprs)?;
        // transformGroupClause, with grouping sets expanded as
        // expand_grouping_sets does.
        let has_group_by = stmt.group_by.is_some();
        let mut grouping = Grouping::default();
        if let Some(group_by) = stmt.group_by {
            let clause = self.get(group_by);
            let _ = clause
                .modifier
                .map(|m| matches!(self.get(m), FlatGroupByModifier::Distinct));
            self.expr_kind = ExprKind::GroupBy;
            let items = self.list1(clause.items).to_vec();
            let mut sets: Vec<Vec<usize>> = vec![Vec::new()];
            let mut numsets: f64 = 1.0;
            let mut has_sets = false;
            for item in items {
                let expanded = match self.get(item) {
                    FlatGroupByItem::Expr(e) => {
                        let refs = self.group_element(
                            &scope,
                            *e,
                            &mut target_list,
                            &mut target_exprs,
                            &mut grouping,
                        )?;
                        vec![refs]
                    }
                    _ => {
                        has_sets = true;
                        self.grouping_set(
                            &scope,
                            item,
                            &mut target_list,
                            &mut target_exprs,
                            &mut grouping,
                        )?
                    }
                };
                numsets *= expanded.len() as f64;
                if numsets <= 4096.0 {
                    sets = cross_union(&sets, &expanded);
                }
            }
            if numsets > 4096.0 {
                return Err(Diagnostic::new(
                    "54001",
                    "too many grouping sets present (maximum 4096)",
                )
                .into());
            }
            if has_sets {
                grouping.sets = Some(sets);
            }
        }
        let group_entries: Vec<GroupEntry> = grouping.entries.clone();
        let common_entries: Vec<GroupEntry> = grouping.common();
        // transformDistinctOnClause.
        if let Some(exprs) = distinct_on {
            self.expr_kind = ExprKind::DistinctOn;
            let mut refs: Vec<usize> = Vec::new();
            for &expr in &exprs {
                let index = self.find_targetlist_entry_sql92(
                    &scope,
                    expr,
                    &mut target_list,
                    &mut target_exprs,
                    ExprKind::DistinctOn,
                )?;
                refs.push(index);
            }
            // ORDER BY items that are DISTINCT ON items lead the result;
            // once one is skipped no more may follow.
            let mut result: Vec<usize> = Vec::new();
            let mut skipped = false;
            for &sort in &sort_refs {
                if let Some(pos) = refs.iter().position(|&r| r == sort) {
                    if skipped {
                        return Err(Diagnostic::new(
                            "42P10",
                            "SELECT DISTINCT ON expressions must match initial ORDER BY expressions",
                        )
                        .at(self.span(exprs[pos]).start)
                        .into());
                    }
                    if !result.contains(&sort) {
                        result.push(sort);
                    }
                } else {
                    skipped = true;
                }
            }
            for (pos, &index) in refs.iter().enumerate() {
                if result.contains(&index) {
                    continue;
                }
                if skipped {
                    return Err(Diagnostic::new(
                        "42P10",
                        "SELECT DISTINCT ON expressions must match initial ORDER BY expressions",
                    )
                    .at(self.span(exprs[pos]).start)
                    .into());
                }
                // addTargetToGroupList
                let entry = &mut target_list[index];
                if self.types().is_unknown(entry.ty) {
                    entry.ty = self.types().builtin(Builtin::Text);
                    entry.typmod = -1;
                }
                let ty = entry.ty;
                if self.types().equality_operator(ty).is_none() {
                    return Err(Diagnostic::new(
                        "42883",
                        format!(
                            "could not identify an equality operator for type {}",
                            self.types().format(ty)
                        ),
                    )
                    .at(self.span(exprs[pos]).start)
                    .into());
                }
                result.push(index);
            }
            self.extra_sortgroup_refs.extend(result.iter().copied());
        }
        // transformDistinctClause.
        if distinct {
            for &index in &sort_refs {
                if target_list[index].junk {
                    let location = target_exprs[index].map_or(0, |e| self.span(e).start);
                    return Err(Diagnostic::new(
                        "42P10",
                        "for SELECT DISTINCT, ORDER BY expressions must appear in select list",
                    )
                    .at(location)
                    .into());
                }
            }
            let mut any = false;
            for (index, entry) in target_list.iter_mut().enumerate() {
                if entry.junk {
                    continue;
                }
                any = true;
                if self.types().is_unknown(entry.ty) {
                    entry.ty = self.types().builtin(Builtin::Text);
                    entry.typmod = -1;
                }
                let ty = entry.ty;
                if self.types().equality_operator(ty).is_none() {
                    let location = target_exprs[index].map_or(0, |e| self.span(e).start);
                    return Err(Diagnostic::new(
                        "42883",
                        format!(
                            "could not identify an equality operator for type {}",
                            self.types().format(ty)
                        ),
                    )
                    .at(location)
                    .into());
                }
            }
            if !any {
                return Err(Diagnostic::new(
                    "42601",
                    "SELECT DISTINCT must have at least one column",
                )
                .into());
            }
        }
        self.limit_clause(&scope, limit_offset)?;
        // transformWindowDefinitions
        self.window_definitions(&scope, &mut target_list, &mut target_exprs)?;
        // Per spec, aggregates can't appear in a recursive term.
        if self.has_aggs
            && scope.rtable.iter().any(|rte| {
                matches!(
                    rte.kind,
                    RteKind::Cte {
                        self_reference: true,
                        ..
                    }
                )
            })
        {
            return Err(Diagnostic::new(
                "42P19",
                "aggregate functions are not allowed in a recursive query's recursive term",
            )
            .at(self
                .agg_locations
                .iter()
                .copied()
                .min()
                .unwrap_or(NO_LOCATION))
            .into());
        }
        // parseCheckAggregates, including references to this level from
        // subqueries in the target list and HAVING.
        let level = self.level();
        let outer_refs: Vec<OuterRef> = self
            .outer_vars
            .iter()
            .filter(|r| r.level == level)
            .copied()
            .collect();
        self.outer_vars.retain(|r| r.level != level);
        if self.has_aggs || has_group_by || having_expr.is_some() {
            // finalize_grouping_exprs: every GROUPING argument is a
            // grouping expression of this level.
            let have_non_var = group_entries
                .iter()
                .any(|g| matches!(g, GroupEntry::Expr(_)));
            let not_grouping = |at: usize| {
                Diagnostic::new(
                    "42803",
                    "arguments to GROUPING must be grouping expressions of the associated query level",
                )
                .at(at)
            };
            for (_, args) in std::mem::take(&mut self.grouping_calls) {
                for arg in args {
                    let grouped = if let Some(var) = self.analysis.vars.get(arg) {
                        self.grouped_var(&scope, *var, &group_entries)
                    } else {
                        have_non_var
                            && group_entries.iter().any(
                                |g| matches!(g, GroupEntry::Expr(ge) if self.expr_equal(ge.to_owned(), arg)),
                            )
                    };
                    if !grouped {
                        return Err(not_grouping(self.span(arg).start).into());
                    }
                }
            }
            // The GROUPING calls of subqueries that belong to this level:
            // only a plain column can match from there.
            let outer_groupings: Vec<OuterGrouping> = self
                .outer_grouping_calls
                .iter()
                .filter(|g| g.level == level)
                .cloned()
                .collect();
            self.outer_grouping_calls.retain(|g| g.level != level);
            for grouping in outer_groupings {
                for (arg, var) in grouping.args {
                    let grouped = var.is_some_and(|v| self.grouped_var(&scope, v, &group_entries));
                    if !grouped {
                        return Err(not_grouping(self.span(arg).start).into());
                    }
                }
            }
            let mut checked: Vec<Handle<FlatExpr>> =
                target_exprs.iter().flatten().copied().collect();
            checked.extend(having_expr);
            for expr in checked {
                self.check_ungrouped(&scope, expr, &group_entries, &common_entries, false)?;
            }
            for r in outer_refs {
                if matches!(
                    r.kind,
                    ExprKind::SelectTarget
                        | ExprKind::OrderBy
                        | ExprKind::DistinctOn
                        | ExprKind::GroupBy
                        | ExprKind::Having
                ) {
                    self.check_ungrouped_var(
                        &scope,
                        r.var,
                        &group_entries,
                        &common_entries,
                        r.location,
                        false,
                    )?;
                }
            }
        }
        // CheckSelectLocking, for this level's own clause and for one
        // pushed down from above.
        let own = if has_distinct {
            Some("DISTINCT clause")
        } else if has_group_by {
            Some("GROUP BY clause")
        } else if having_expr.is_some() {
            Some("HAVING clause")
        } else if self.has_aggs {
            Some("aggregate functions")
        } else if self.has_window_funcs {
            Some("window functions")
        } else if self.has_target_srfs {
            Some("set-returning functions in the target list")
        } else {
            None
        };
        for &l in locking {
            self.locking_clause(&scope, l, own)?;
        }
        self.lock_refusal = own.or_else(|| Self::pushed_down_refusal(&scope));
        // resolveTargetListUnknowns
        if self.resolve_unknowns {
            let text = self.types().builtin(Builtin::Text);
            for entry in target_list.iter_mut() {
                if self.types().is_unknown(entry.ty) {
                    entry.ty = text;
                    entry.typmod = -1;
                }
            }
        }
        // assign_query_collations: DISTINCT makes every output column a
        // group key.
        let mut sortgroup: Vec<usize> = sort_refs.clone();
        sortgroup.extend(grouping.refs.iter().copied());
        sortgroup.extend(std::mem::take(&mut self.extra_sortgroup_refs));
        if distinct {
            sortgroup.extend(
                target_list
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| !t.junk)
                    .map(|(i, _)| i),
            );
        }
        self.assign_query_collations(&scope, &mut target_list, &target_exprs, &sortgroup)?;
        self.last_target_exprs = target_exprs;
        self.analysis
            .range_tables
            .insert(handle, scope.rtable.clone());
        Ok(target_list)
    }

    /// `transformLimitClause`: the count is coerced to bigint and must not
    /// reference a column.
    fn limit_expr(&mut self, scope: &Scope, expr: Handle<FlatExpr>, construct: &str) -> Result<()> {
        let typed = match self.get(expr) {
            // LIMIT ALL is a null constant.
            FlatExpr::ColumnRef(_)
                if self.source[self.span(expr)].eq_ignore_ascii_case("all")
                    && construct == "LIMIT" =>
            {
                make_const(self.types(), LiteralKind::Null, "", self.span(expr).start)
            }
            _ => self.expr(scope, expr)?,
        };
        if typed.returns_set {
            // check_srf_call_placement for EXPR_KIND_LIMIT / OFFSET.
            return Err(Diagnostic::new(
                "0A000",
                format!("set-returning functions are not allowed in {construct}"),
            )
            .at(typed.location)
            .into());
        }
        let int8 = self.types().builtin(Builtin::Int8);
        if typed.ty != int8 {
            match self.coercer().coerce_to_target_type(
                &typed,
                int8,
                -1,
                CoercionContext::Assignment,
            )? {
                Some((_, steps)) => self.record_coercions(expr, steps),
                None => {
                    return Err(Diagnostic::new(
                        "42804",
                        format!(
                            "argument of {construct} must be type {}, not type {}",
                            "bigint",
                            self.types().format(typed.ty)
                        ),
                    )
                    .at(typed.location)
                    .into());
                }
            }
        }
        if typed.returns_set {
            return Err(Diagnostic::new(
                "42804",
                format!("argument of {construct} must not return a set"),
            )
            .at(typed.location)
            .into());
        }
        if self.contains_var(expr) {
            return Err(Diagnostic::new(
                "42P10",
                format!("argument of {construct} must not contain variables"),
            )
            .at(typed.location)
            .into());
        }
        Ok(())
    }

    /// Whether any column reference was resolved inside the expression.
    /// Every resolved reference is recorded in `vars` as the walk goes, so
    /// the check is a scan of the subtree's handles by span containment.
    /// `contain_vars_of_level(expr, 0)`: a column of this query level,
    /// written directly or as an outer reference inside a subquery.
    fn contains_var(&self, expr: Handle<FlatExpr>) -> bool {
        let span = self.span(expr);
        let inside = |s: &std::ops::Range<usize>| s.start >= span.start && s.end <= span.end;
        let sublinks: Vec<std::ops::Range<usize>> = self
            .sublinks
            .iter()
            .map(|h| self.span(*h))
            .filter(|s| inside(s) && *s != span)
            .collect();
        let direct = self.analysis.vars.iter().any(|(h, v)| {
            let s = self.span(h);
            v.levelsup == 0
                && inside(&s)
                && !sublinks
                    .iter()
                    .any(|sl| s.start >= sl.start && s.end <= sl.end)
        });
        let level = self.level();
        direct
            || self
                .outer_vars
                .iter()
                .any(|r| r.level == level && r.location >= span.start && r.location < span.end)
    }

    /// `findTargetlistEntrySQL92`: a bare name matches an output column
    /// name, an integer is an ordinal, anything else is an expression that
    /// becomes a resjunk entry unless one already equals it.
    fn find_targetlist_entry_sql92(
        &mut self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
        kind: ExprKind,
    ) -> Result<usize> {
        let index =
            self.find_targetlist_entry_sql92_inner(scope, expr, target_list, target_exprs, kind)?;
        // checkTargetlistEntrySQL92: a GROUP BY item that names an output
        // column may not hold an aggregate or a window function.
        if kind == ExprKind::GroupBy
            && let Some(e) = target_exprs.get(index).copied().flatten()
        {
            let span = self.span(e);
            if let Some(&at) = self
                .agg_locations
                .iter()
                .find(|&&l| l >= span.start && l < span.end)
            {
                return Err(Diagnostic::new(
                    "42803",
                    "aggregate functions are not allowed in GROUP BY",
                )
                .at(at)
                .into());
            }
            if let Some(&at) = self
                .window_locations
                .iter()
                .find(|&&l| l >= span.start && l < span.end)
            {
                return Err(Diagnostic::new(
                    "42P20",
                    "window functions are not allowed in GROUP BY",
                )
                .at(at)
                .into());
            }
        }
        Ok(index)
    }

    fn find_targetlist_entry_sql92_inner(
        &mut self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
        kind: ExprKind,
    ) -> Result<usize> {
        let construct = kind.name();
        let location = self.span(expr).start;
        if let FlatExpr::ColumnRef(c) = self.get(expr) {
            let c = self.get(*c);
            if c.indirection.is_none() {
                let (name, _) = self.col_id(c.name)?;
                // In GROUP BY a name that is a column of the FROM clause is
                // not matched against output names.
                let is_column = kind == ExprKind::GroupBy
                    && self
                        .relations
                        .col_name_to_var(scope, &self.outer, &name, location)?
                        .is_some();
                let mut found: Option<usize> = None;
                let _ = is_column;
                for (i, entry) in target_list.iter().enumerate() {
                    if !is_column && !entry.junk && entry.name == name {
                        if let Some(first) = found {
                            // Two output columns with the name: ambiguous
                            // unless their expressions are equal.
                            let same = match (target_exprs[first], target_exprs[i]) {
                                (Some(a), Some(b)) => self.expr_equal(a, b),
                                _ => false,
                            };
                            if !same {
                                return Err(Diagnostic::new(
                                    "42702",
                                    format!("{construct} \"{name}\" is ambiguous"),
                                )
                                .at(location)
                                .into());
                            }
                            continue;
                        }
                        found = Some(i);
                    }
                }
                if let Some(i) = found {
                    return Ok(i);
                }
            }
        }
        if let FlatExpr::IntegerLit(t) = self.get(expr) {
            let text = self.text(*t);
            let Some(position) = crate::consts::integer_value(text) else {
                return Err(unsupported("ordinal", Some(location..location)));
            };
            let mut n = 0;
            for (i, entry) in target_list.iter().enumerate() {
                if !entry.junk {
                    n += 1;
                    if n == position {
                        return Ok(i);
                    }
                }
            }
            return Err(Diagnostic::new(
                "42P10",
                format!("{construct} position {position} is not in select list"),
            )
            .at(location)
            .into());
        }
        if let FlatExpr::NumericLit(_) = self.get(expr) {
            return Err(
                Diagnostic::new("42601", format!("non-integer constant in {construct}"))
                    .at(location)
                    .into(),
            );
        }
        // findTargetlistEntrySQL99: an expression. An existing entry that
        // equals it (implicit coercions stripped) is reused; otherwise a
        // resjunk entry is added.
        let typed = self.expr(scope, expr)?;
        for (i, e) in target_exprs.iter().enumerate() {
            if let Some(e) = e
                && self.expr_equal(*e, expr)
            {
                return Ok(i);
            }
        }
        target_list.push(TargetEntry {
            collation_state: CollContext::new(),
            collation: None,
            name: String::new(),
            ty: typed.ty,
            typmod: typed.typmod,
            origin: None,
            junk: true,
            location: self.span(expr).start,
            fields: None,
        });
        target_exprs.push(Some(expr));
        Ok(target_list.len() - 1)
    }

    /// The `Var` an expression is, looking through parentheses, when it is
    /// a bare column reference.
    fn plain_var(&self, expr: Handle<FlatExpr>) -> Option<crate::relation::Var> {
        if let Some(v) = self.analysis.vars.get(expr) {
            return Some(*v);
        }
        if let FlatExpr::Parenthesized(p) = self.get(expr) {
            let p = self.get(*p);
            if p.indirection.is_none() {
                return self.plain_var(p.expr);
            }
        }
        None
    }

    /// `equal()` over two analysed expressions: the same resolution and the
    /// same structure. Parentheses are transparent, as `gram.y` keeps no
    /// node for them.
    fn expr_equal(&self, a: Handle<FlatExpr>, b: Handle<FlatExpr>) -> bool {
        let a = self.unparen(a);
        let b = self.unparen(b);
        if a == b {
            return true;
        }
        match (self.get(a), self.get(b)) {
            (
                FlatExpr::ColumnRef(_) | FlatExpr::QualRef(_),
                FlatExpr::ColumnRef(_) | FlatExpr::QualRef(_),
            ) => {
                let same_var = match (self.analysis.vars.get(a), self.analysis.vars.get(b)) {
                    (Some(x), Some(y)) => x == y,
                    _ => false,
                };
                // A subscripted or field-selected reference is compared as
                // written.
                let indirect = |w: &Self, h: Handle<FlatExpr>| match w.get(h) {
                    FlatExpr::ColumnRef(c) => w.get(*c).indirection.is_some(),
                    FlatExpr::QualRef(q) => w.qref_indirect(*q),
                    _ => false,
                };
                match (indirect(self, a), indirect(self, b)) {
                    (false, false) => same_var,
                    (true, true) => {
                        same_var
                            && self.source[self.span(a)].trim() == self.source[self.span(b)].trim()
                    }
                    _ => false,
                }
            }
            (FlatExpr::IntegerLit(x), FlatExpr::IntegerLit(y)) => self.text(*x) == self.text(*y),
            (FlatExpr::NumericLit(x), FlatExpr::NumericLit(y)) => self.text(*x) == self.text(*y),
            (FlatExpr::StringLit(x), FlatExpr::StringLit(y)) => {
                self.source[self.span(*x)] == self.source[self.span(*y)]
            }
            (FlatExpr::BoolTrue, FlatExpr::BoolTrue)
            | (FlatExpr::BoolFalse, FlatExpr::BoolFalse)
            | (FlatExpr::Null, FlatExpr::Null) => true,
            (FlatExpr::Func(_), FlatExpr::Func(_)) => {
                let same_function = match (
                    self.analysis.functions.get(a),
                    self.analysis.functions.get(b),
                ) {
                    (Some(x), Some(y)) => x.function == y.function,
                    _ => false,
                };
                same_function && self.children_equal(a, b)
            }
            _ => {
                let same_variant =
                    std::mem::discriminant(self.get(a)) == std::mem::discriminant(self.get(b));
                let same_operator = match (
                    self.analysis.operators.get(a),
                    self.analysis.operators.get(b),
                ) {
                    (Some(x), Some(y)) => x == y,
                    (None, None) => true,
                    _ => false,
                };
                let same_type = self.analysis.expr_types.get(a) == self.analysis.expr_types.get(b);
                same_variant && same_operator && same_type && self.children_equal(a, b)
            }
        }
    }

    /// Whether two expressions of the same variant have pairwise equal
    /// child expressions, found through their spans in source order.
    fn children_equal(&self, a: Handle<FlatExpr>, b: Handle<FlatExpr>) -> bool {
        let ca = self.child_exprs(a);
        let cb = self.child_exprs(b);
        if ca.len() != cb.len() {
            return false;
        }
        // Non-expression content (operator spelling, a cast's type name)
        // must match textually once child expressions are blanked.
        let ta = self.skeleton(a, &ca);
        let tb = self.skeleton(b, &cb);
        if ta != tb {
            return false;
        }
        ca.iter().zip(&cb).all(|(&x, &y)| self.expr_equal(x, y))
    }

    /// The direct child expressions of an expression, in source order: the
    /// analysed expressions whose spans lie inside it and inside no other
    /// analysed descendant.
    fn child_exprs(&self, e: Handle<FlatExpr>) -> Vec<Handle<FlatExpr>> {
        let span = self.span(e);
        let mut inside: Vec<(std::ops::Range<usize>, Handle<FlatExpr>)> = self
            .analysis
            .expr_types
            .iter()
            .map(|(h, _)| (self.span(h), h))
            .filter(|(s, h)| {
                *h != e
                    && s.start >= span.start
                    && s.end <= span.end
                    && !(s.start == span.start && s.end == span.end)
            })
            .collect();
        inside.sort_by_key(|(s, _)| (s.start, std::cmp::Reverse(s.end)));
        let mut out: Vec<(std::ops::Range<usize>, Handle<FlatExpr>)> = Vec::new();
        for (s, h) in inside {
            if out
                .last()
                .is_some_and(|(last, _)| s.start >= last.start && s.end <= last.end)
            {
                continue;
            }
            out.push((s, h));
        }
        out.into_iter().map(|(_, h)| h).collect()
    }

    /// The source text of an expression with its child expressions blanked
    /// and whitespace collapsed.
    fn skeleton(&self, e: Handle<FlatExpr>, children: &[Handle<FlatExpr>]) -> String {
        let span = self.span(e);
        let mut text = String::new();
        let mut pos = span.start;
        for &c in children {
            let cs = self.span(c);
            text.push_str(&self.source[pos..cs.start]);
            text.push('\u{0}');
            pos = cs.end;
        }
        text.push_str(&self.source[pos..span.end]);
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    }

    fn unparen(&self, mut e: Handle<FlatExpr>) -> Handle<FlatExpr> {
        loop {
            let FlatExpr::Parenthesized(p) = self.get(e) else {
                return e;
            };
            let p = self.get(*p);
            if !p.indirection.is_none() {
                return e;
            }
            e = p.expr;
        }
    }

    /// `check_ungrouped_columns_walker` at a `Var`. A merged column of a
    /// join stands for the input columns it is computed from, as
    /// `flatten_join_alias_vars` expands it before the check; those carry
    /// no location.
    fn check_ungrouped_var(
        &self,
        scope: &Scope,
        var: Var,
        groups: &[GroupEntry],
        common: &[GroupEntry],
        location: usize,
        direct: bool,
    ) -> Result<()> {
        if var.levelsup != 0 {
            return Ok(());
        }
        let grouped = groups.iter().any(
            |g| matches!(g, GroupEntry::Var(gv) if gv.varno == var.varno && gv.varattno == var.varattno),
        );
        if grouped {
            return Ok(());
        }
        let rte = &scope.rtable[var.varno - 1];
        if let RteKind::Join { inputs, .. } = &rte.kind {
            let inputs = &inputs[usize::try_from(var.varattno - 1).expect("join attnum")];
            for input in inputs {
                self.check_ungrouped_var(scope, *input, groups, common, NO_LOCATION, direct)?;
            }
            return Ok(());
        }
        // check_functional_grouping: every column of a non-deferrable
        // primary key of the same range table entry is grouped, in every
        // grouping set.
        let key = rte
            .relation()
            .and_then(|r| self.catalog()[r].primary_key.as_ref())
            .and_then(pgtk_catalog::PrimaryKey::grouping_columns);
        if let Some(key) = key
            && !key.is_empty()
            && key.iter().all(|&k| {
                common.iter().any(|g| {
                    matches!(g, GroupEntry::Var(gv) if gv.varno == var.varno && gv.varattno == k)
                })
            })
        {
            return Ok(());
        }
        let attname = if var.varattno > 0 {
            rte.colnames[usize::try_from(var.varattno - 1).expect("attnum")].clone()
        } else {
            match var.varattno {
                -1 => "ctid",
                -2 => "xmin",
                -3 => "cmin",
                -4 => "xmax",
                -5 => "cmax",
                _ => "tableoid",
            }
            .to_owned()
        };
        let mut diagnostic = Diagnostic::new(
            "42803",
            format!(
                "column \"{}.{}\" must appear in the GROUP BY clause or be used in an aggregate function",
                rte.eref_name, attname
            ),
        );
        if direct {
            diagnostic = diagnostic.detail(
                "Direct arguments of an ordered-set aggregate must use only grouped columns.",
            );
        }
        Err(diagnostic.at(location).into())
    }

    /// `check_ungrouped_columns` over one expression: every column
    /// reference must be grouped, be inside an aggregate, or be part of an
    /// expression equal to a GROUP BY expression.
    fn check_ungrouped(
        &self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
        groups: &[GroupEntry],
        common: &[GroupEntry],
        direct: bool,
    ) -> Result<()> {
        // GROUPING arguments are checked by finalize_grouping_exprs.
        if matches!(self.get(expr), FlatExpr::Grouping(_)) {
            return Ok(());
        }
        let have_non_var = groups.iter().any(|g| matches!(g, GroupEntry::Expr(_)));
        if have_non_var {
            for g in groups {
                if let GroupEntry::Expr(ge) = g
                    && self.expr_equal(*ge, expr)
                {
                    return Ok(());
                }
            }
        }
        if let Some(var) = self.analysis.vars.get(expr) {
            return self.check_ungrouped_var(
                scope,
                *var,
                groups,
                common,
                self.span(expr).start,
                direct,
            );
        }
        // The subquery's own expressions belong to its level; its
        // references to this level are checked through `outer_vars`.
        if self.sublinks.contains(&expr) {
            return Ok(());
        }
        // An aggregate's arguments are not checked, except the direct
        // arguments of an ordered-set aggregate of this level, which are
        // checked as though they were outside it.
        if self
            .analysis
            .functions
            .get(expr)
            .is_some_and(|f| self.catalog()[f.function].aggregate.is_some())
        {
            let start = self.span(expr).start;
            let level = self.level();
            let local = self
                .agg_refs
                .iter()
                .any(|r| r.location == start && r.level == level);
            if local && !direct {
                for arg in self.direct_arg_handles(expr) {
                    self.check_ungrouped(scope, arg, groups, common, true)?;
                }
            }
            return Ok(());
        }
        for child in self.child_exprs(expr) {
            self.check_ungrouped(scope, child, groups, common, direct)?;
        }
        Ok(())
    }

    /// The direct arguments of an ordered-set aggregate call: its
    /// arguments before WITHIN GROUP.
    fn direct_arg_handles(&self, expr: Handle<FlatExpr>) -> Vec<Handle<FlatExpr>> {
        let FlatExpr::Func(fc) = self.get(expr) else {
            return Vec::new();
        };
        let FlatFunctionCallTail::Call(suffix) = self.get(self.get(*fc).tail) else {
            return Vec::new();
        };
        let suffix = self.get(*suffix);
        let (Some(_), Some(body)) = (suffix.within_group, suffix.body) else {
            return Vec::new();
        };
        self.call_body_items(body)
            .0
            .into_iter()
            .map(|item| match self.get(item) {
                FlatFuncArg::Plain(e) => *e,
                FlatFuncArg::Named(n) => self.get(*n).value,
            })
            .collect()
    }

    /// The argument items of a function call body, its ORDER BY, and
    /// whether it is `(*)`, `DISTINCT` or `VARIADIC`.
    fn call_body_items(&self, body: Handle<FlatFunctionCallBody>) -> CallBodyItems {
        let mut items: Vec<Handle<FlatFuncArg>> = Vec::new();
        let mut order_by = None;
        let (mut star, mut distinct, mut variadic) = (false, false, false);
        match self.get(body) {
            FlatFunctionCallBody::Star(_) => star = true,
            FlatFunctionCallBody::Args(args) => {
                let args = self.get(*args);
                items.extend(self.list1(args.args).iter().copied());
                if let Some(tv) = args.trailing_variadic {
                    items.push(self.get(self.get(tv).variadic).argument);
                    variadic = true;
                }
                order_by = args.order_by;
            }
            FlatFunctionCallBody::All(all) => {
                let all = self.get(*all);
                items.extend(self.list1(self.get(all.args).0).iter().copied());
                order_by = all.order_by;
            }
            FlatFunctionCallBody::Distinct(d) => {
                let d = self.get(*d);
                distinct = true;
                items.extend(self.list1(self.get(d.args).0).iter().copied());
                order_by = d.order_by;
            }
            FlatFunctionCallBody::LeadingVariadic(l) => {
                let l = self.get(*l);
                items.push(self.get(l.variadic).argument);
                variadic = true;
                order_by = l.order_by;
            }
        }
        (items, order_by, (star, distinct, variadic))
    }

    /// Whether `var`, through join aliases, is one of the grouping
    /// columns.
    fn grouped_var(&self, scope: &Scope, var: Var, groups: &[GroupEntry]) -> bool {
        if var.levelsup != 0 {
            return false;
        }
        if groups.iter().any(
            |g| matches!(g, GroupEntry::Var(gv) if gv.varno == var.varno && gv.varattno == var.varattno),
        ) {
            return true;
        }
        if let RteKind::Join { inputs, .. } = &scope.rtable[var.varno - 1].kind
            && var.varattno > 0
        {
            let inputs = &inputs[usize::try_from(var.varattno - 1).expect("join attnum")];
            return !inputs.is_empty()
                && inputs.iter().all(|i| self.grouped_var(scope, *i, groups));
        }
        false
    }

    /// `transformGroupClauseExpr`: the target list index the expression
    /// groups by, added to the grouping entries when new.
    fn group_expr(
        &mut self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
        grouping: &mut Grouping,
    ) -> Result<usize> {
        let index = self.find_targetlist_entry_sql92(
            scope,
            expr,
            target_list,
            target_exprs,
            ExprKind::GroupBy,
        )?;
        if grouping.refs.contains(&index) {
            return Ok(index);
        }
        // addTargetToGroupList: unknown becomes text; an equality
        // operator must exist.
        let entry = &mut target_list[index];
        if self.types().is_unknown(entry.ty) {
            entry.ty = self.types().builtin(Builtin::Text);
            entry.typmod = -1;
        }
        let ty = entry.ty;
        if self.types().equality_operator(ty).is_none() {
            return Err(Diagnostic::new(
                "42883",
                format!(
                    "could not identify an equality operator for type {}",
                    self.types().format(ty)
                ),
            )
            .at(self.span(expr).start)
            .into());
        }
        let entry_expr = target_exprs[index];
        let var = entry_expr.and_then(|e| self.plain_var(e));
        grouping.entries.push(match var {
            Some(v) => GroupEntry::Var(v),
            None => GroupEntry::Expr(entry_expr.unwrap_or(expr)),
        });
        grouping.refs.push(index);
        Ok(index)
    }

    /// One element of a grouping set: an implicit row is its elements
    /// (`flatten_grouping_sets`), anything else one expression.
    fn group_element(
        &mut self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
        grouping: &mut Grouping,
    ) -> Result<Vec<usize>> {
        let elements = match self.get(expr) {
            FlatExpr::ImplicitRow(row) => self.implicit_row_values(*row),
            _ => vec![expr],
        };
        let mut refs = Vec::new();
        for e in elements {
            let r = self.group_expr(scope, e, target_list, target_exprs, grouping)?;
            if !refs.contains(&r) {
                refs.push(r);
            }
        }
        Ok(refs)
    }

    /// The elements of a ROLLUP or CUBE list, each a list of target indexes.
    fn group_elements(
        &mut self,
        scope: &Scope,
        exprs: Vec<Handle<FlatExpr>>,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
        grouping: &mut Grouping,
    ) -> Result<Vec<Vec<usize>>> {
        let mut lists = Vec::with_capacity(exprs.len());
        for e in exprs {
            lists.push(self.group_element(scope, e, target_list, target_exprs, grouping)?);
        }
        Ok(lists)
    }

    /// `transformGroupingSet` and `expand_groupingset_node`: the grouping
    /// sets one item stands for, each a list of target indexes.
    fn grouping_set(
        &mut self,
        scope: &Scope,
        item: Handle<FlatGroupByItem>,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
        grouping: &mut Grouping,
    ) -> Result<Vec<Vec<usize>>> {
        let location = self.span(item).start;
        match self.get(item) {
            FlatGroupByItem::Expr(e) => {
                let refs = self.group_element(scope, *e, target_list, target_exprs, grouping)?;
                Ok(vec![refs])
            }
            FlatGroupByItem::Empty(_) => Ok(vec![Vec::new()]),
            FlatGroupByItem::Rollup(r) => {
                let r: &FlatRollupItem = self.get(*r);
                let exprs = self.list1(r.items).to_vec();
                let lists =
                    self.group_elements(scope, exprs, target_list, target_exprs, grouping)?;
                let mut out = vec![Vec::new()];
                let mut acc: Vec<usize> = Vec::new();
                for list in lists {
                    for r in list {
                        if !acc.contains(&r) {
                            acc.push(r);
                        }
                    }
                    out.push(acc.clone());
                }
                Ok(out)
            }
            FlatGroupByItem::Cube(c) => {
                let c: &FlatCubeItem = self.get(*c);
                let exprs = self.list1(c.items).to_vec();
                let lists =
                    self.group_elements(scope, exprs, target_list, target_exprs, grouping)?;
                if lists.len() > 12 {
                    return Err(Diagnostic::new("54011", "CUBE is limited to 12 elements")
                        .at(location)
                        .into());
                }
                let mut out: Vec<Vec<usize>> = vec![Vec::new()];
                for list in lists {
                    let mut next = out.clone();
                    for set in &out {
                        let mut u = set.clone();
                        for &r in &list {
                            if !u.contains(&r) {
                                u.push(r);
                            }
                        }
                        next.push(u);
                    }
                    out = next;
                }
                Ok(out)
            }
            FlatGroupByItem::GroupingSets(g) => {
                let subs = self.list1(self.get(*g).groups).to_vec();
                let mut out = Vec::new();
                for sub in subs {
                    out.extend(self.grouping_set(
                        scope,
                        sub,
                        target_list,
                        target_exprs,
                        grouping,
                    )?);
                }
                Ok(out)
            }
        }
    }

    // ----- FROM -----

    /// `transformFromClause`.
    fn transform_from_clause(
        &mut self,
        scope: &mut Scope,
        from: Handle<FlatFromClause>,
    ) -> Result<()> {
        let from = self.get(from);
        let tables = self.list1(from.tables).to_vec();
        self.transform_from_items(scope, &tables)
    }

    /// `transformFromClause` over its items.
    fn transform_from_items(
        &mut self,
        scope: &mut Scope,
        tables: &[Handle<FlatTableRef>],
    ) -> Result<()> {
        for table in tables {
            let (_, mut items) = self.table_ref(scope, *table)?;
            self.relations
                .check_namespace_conflicts(scope, &scope.namespace, &items)?;
            // Visible only to LATERAL items until the FROM clause is done.
            for item in &mut items {
                item.lateral_only = true;
                item.lateral_ok = true;
            }
            scope.namespace.extend(items);
        }
        for item in &mut scope.namespace {
            item.lateral_only = false;
            item.lateral_ok = true;
        }
        Ok(())
    }

    /// `transformFromClauseItem` for a table reference with its chain of
    /// joins.
    fn table_ref(
        &mut self,
        scope: &mut Scope,
        handle: Handle<FlatTableRef>,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let table_ref = self.get(handle);
        let location = self.span(handle).start;
        let joins = self.list(table_ref.joins);
        self.join_chain(scope, table_ref.base, joins, None, location)
    }

    /// `transformFromClauseItem` for a parenthesized join, possibly in more
    /// parentheses. `alias` is the alias after the parentheses, which
    /// PostgreSQL hangs on the outermost join.
    fn paren_join_group(
        &mut self,
        scope: &mut Scope,
        group: Handle<FlatParenJoinGroup>,
        alias: Option<Handle<FlatPlainTableAlias>>,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let group = self.get(group);
        match self.get(group.joined) {
            FlatJoinedTable::Chain(chain) => {
                let location = self.span(*chain).start;
                let chain = self.get(*chain);
                let joins = self.list1(chain.joins);
                self.join_chain(scope, chain.base, joins, alias, location)
            }
            FlatJoinedTable::Group(inner) => self.paren_join_group(scope, *inner, alias),
        }
    }

    /// A base table reference and its joins.
    fn join_chain(
        &mut self,
        scope: &mut Scope,
        base: Handle<FlatSimpleTableRef>,
        joins: &[Handle<FlatJoinSuffix>],
        alias: Option<Handle<FlatPlainTableAlias>>,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        // checkWellFormedRecursion: an item on the nullable side of a later
        // outer join may not hold the recursive reference.
        let nullable_after = |w: &Self, from: usize| {
            joins[from..].iter().any(|j| match w.get(*j) {
                FlatJoinSuffix::Qualified(q) => {
                    matches!(
                        w.join_type(w.get(*q).join_type),
                        JoinType::Right | JoinType::Full
                    )
                }
                FlatJoinSuffix::Unqualified(u) => match w.get(w.get(*u).kind) {
                    FlatUnqualifiedJoinKind::Natural(n) => matches!(
                        w.join_type(w.get(*n).join_type),
                        JoinType::Right | JoinType::Full
                    ),
                    FlatUnqualifiedJoinKind::Cross => false,
                },
            })
        };
        let saved_context = self.recursion_context;
        if nullable_after(self, 0) {
            self.recursion_context = RecursionContext::OuterJoin;
        }
        let current = self.simple_table_ref(scope, base, location);
        self.recursion_context = saved_context;
        let mut current = current?;
        for (i, join) in joins.iter().enumerate() {
            let join_alias = if i + 1 == joins.len() { alias } else { None };
            let join_location = self.span(*join).start;
            match self.get(*join) {
                FlatJoinSuffix::Unqualified(u) => {
                    let u = self.get(*u);
                    let (join_type, natural) = match self.get(u.kind) {
                        FlatUnqualifiedJoinKind::Cross => (JoinType::Inner, false),
                        FlatUnqualifiedJoinKind::Natural(n) => {
                            (self.join_type(self.get(*n).join_type), true)
                        }
                    };
                    let right_location = self.span(u.table).start;
                    let saved_len = self.expose_left_side(scope, &current.1, join_type);
                    if matches!(join_type, JoinType::Left | JoinType::Full)
                        || nullable_after(self, i + 1)
                    {
                        self.recursion_context = RecursionContext::OuterJoin;
                    }
                    let right = self.simple_table_ref(scope, u.table, right_location);
                    self.recursion_context = saved_context;
                    scope.namespace.truncate(saved_len);
                    let right = right?;
                    current = self.join_tables(
                        scope,
                        current,
                        right,
                        join_type,
                        natural,
                        None,
                        join_alias,
                        join_location,
                    )?;
                }
                FlatJoinSuffix::Qualified(q) => {
                    let q = self.get(*q);
                    let join_type = self.join_type(q.join_type);
                    let saved_len = self.expose_left_side(scope, &current.1, join_type);
                    if matches!(join_type, JoinType::Left | JoinType::Full)
                        || nullable_after(self, i + 1)
                    {
                        self.recursion_context = RecursionContext::OuterJoin;
                    }
                    let right = self.table_ref(scope, q.table);
                    self.recursion_context = saved_context;
                    scope.namespace.truncate(saved_len);
                    let right = right?;
                    current = self.join_tables(
                        scope,
                        current,
                        right,
                        join_type,
                        false,
                        Some(q.condition),
                        join_alias,
                        join_location,
                    )?;
                }
            }
        }
        Ok(current)
    }

    /// Make the left side of a join visible to a LATERAL item on its right
    /// side, as an error unless the join is INNER or LEFT. Returns the
    /// namespace length to truncate back to.
    fn expose_left_side(
        &self,
        scope: &mut Scope,
        left: &[NamespaceItem],
        join_type: JoinType,
    ) -> usize {
        let lateral_ok = matches!(join_type, JoinType::Inner | JoinType::Left);
        let saved_len = scope.namespace.len();
        scope
            .namespace
            .extend(left.iter().map(|item| NamespaceItem {
                lateral_only: true,
                lateral_ok,
                ..item.clone()
            }));
        saved_len
    }

    fn join_type(&self, handle: Handle<FlatJoinType>) -> JoinType {
        match self.get(handle) {
            FlatJoinType::LeftOuter | FlatJoinType::Left => JoinType::Left,
            FlatJoinType::RightOuter | FlatJoinType::Right => JoinType::Right,
            FlatJoinType::FullOuter | FlatJoinType::Full => JoinType::Full,
            FlatJoinType::Inner | FlatJoinType::Plain => JoinType::Inner,
        }
    }

    /// The name and column names of a table alias.
    fn plain_alias(
        &self,
        alias: Option<Handle<FlatPlainTableAlias>>,
    ) -> Result<(Option<String>, Vec<String>)> {
        let Some(a) = alias else {
            return Ok((None, Vec::new()));
        };
        let (name, columns) = match self.get(a) {
            FlatPlainTableAlias::WithAs(w) => {
                let w = self.get(*w);
                (w.name, w.columns)
            }
            FlatPlainTableAlias::Bare(b) => {
                let b = self.get(*b);
                (b.name, b.columns)
            }
        };
        let (name, _) = self.col_id(name)?;
        let mut cols = Vec::new();
        if let Some(columns) = columns {
            let columns = self.get(columns);
            for c in self.list1(columns.0) {
                cols.push(self.col_id(*c)?.0);
            }
        }
        Ok((Some(name), cols))
    }

    /// One `JoinExpr` of `transformFromClauseItem`: both sides are already
    /// transformed.
    #[allow(clippy::too_many_arguments)]
    fn join_tables(
        &mut self,
        scope: &mut Scope,
        left: (NamespaceItem, Vec<NamespaceItem>),
        right: (NamespaceItem, Vec<NamespaceItem>),
        join_type: JoinType,
        natural: bool,
        condition: Option<Handle<FlatJoinCondition>>,
        alias: Option<Handle<FlatPlainTableAlias>>,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let (l_top, l_ns) = left;
        let (r_top, r_ns) = right;
        self.relations
            .check_namespace_conflicts(scope, &l_ns, &r_ns)?;
        let mut my_ns = l_ns;
        my_ns.extend(r_ns);
        let l_colnames = scope.rtable[l_top.rt_index - 1].colnames.clone();
        let r_colnames = scope.rtable[r_top.rt_index - 1].colnames.clone();
        // The USING columns, or for NATURAL the names common to both sides
        // in left order.
        let mut using: Vec<String> = Vec::new();
        let mut using_alias: Option<String> = None;
        if natural {
            for l in &l_colnames {
                if !l.is_empty() && r_colnames.iter().any(|r| r == l) {
                    using.push(l.clone());
                }
            }
        } else if let Some(cond) = condition
            && let FlatJoinCondition::Using(u) = self.get(cond)
        {
            let u = self.get(*u);
            for c in self.list1(self.get(u.columns).0) {
                using.push(self.col_id(*c)?.0);
            }
            if let Some(a) = u.alias {
                using_alias = Some(self.col_id(self.get(a).name)?.0);
            }
        }
        let rtindex = scope.rtable.len() + 1;
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        let mut res_colnames: Vec<String> = Vec::new();
        for u in &using {
            if res_colnames.iter().any(|c| c == u) {
                return Err(Diagnostic::new(
                    "42701",
                    format!("column name \"{u}\" appears more than once in USING clause"),
                )
                .into());
            }
            let li = find_using_column(&l_colnames, u, "left")?;
            let ri = find_using_column(&r_colnames, u, "right")?;
            pairs.push((li, ri));
            res_colnames.push(u.clone());
        }
        if !pairs.is_empty() {
            // transformJoinUsingClause: `l = r` for each column, ANDed.
            let eq = ObjectName {
                schema: None,
                name: "=".to_owned(),
            };
            let construct = if pairs.len() == 1 {
                "JOIN/USING"
            } else {
                "AND"
            };
            let saved_kind = self.expr_kind;
            self.expr_kind = ExprKind::JoinUsing;
            for &(li, ri) in &pairs {
                let lt = typed_var(&self.side_var(scope, &l_top, li));
                let rt = typed_var(&self.side_var(scope, &r_top, ri));
                let resolved = self.resolver.make_op(&eq, Some(&lt), &rt, NO_LOCATION)?;
                self.coercer()
                    .coerce_to_boolean(&resolved.result, construct)?;
            }
            self.expr_kind = saved_kind;
        } else if let Some(cond) = condition
            && let FlatJoinCondition::On(o) = self.get(cond)
        {
            // transformJoinOnClause: only the two sides are visible.
            let expr = self.get(*o).condition;
            let on_ns: Vec<NamespaceItem> = my_ns
                .iter()
                .map(|item| NamespaceItem {
                    lateral_only: false,
                    lateral_ok: true,
                    ..item.clone()
                })
                .collect();
            let saved_ns = std::mem::replace(&mut scope.namespace, on_ns);
            let saved_kind = self.expr_kind;
            self.expr_kind = ExprKind::JoinOn;
            let result = self
                .expr(scope, expr)
                .and_then(|t| self.coercer().coerce_to_boolean(&t, "JOIN/ON"));
            scope.namespace = saved_ns;
            self.expr_kind = saved_kind;
            let (_, steps) = result?;
            self.record_coercions(expr, steps);
        }
        // buildMergedJoinVar for each USING column.
        let mut res_vars: Vec<Var> = Vec::new();
        let mut res_inputs: Vec<Vec<Var>> = Vec::new();
        for &(li, ri) in &pairs {
            let lv = self.side_var(scope, &l_top, li);
            let rv = self.side_var(scope, &r_top, ri);
            let out_ty = self
                .coercer()
                .common_type(&[lv.ty, rv.ty])
                .map_err(|(p, n)| {
                    Diagnostic::new(
                        "42804",
                        format!(
                            "JOIN/USING types {} and {} cannot be matched",
                            self.types().format(p),
                            self.types().format(n)
                        ),
                    )
                })?;
            let out_typmod = if lv.ty == out_ty && rv.ty == out_ty && lv.typmod == rv.typmod {
                lv.typmod
            } else {
                -1
            };
            let l_same = lv.ty == out_ty && lv.typmod == out_typmod;
            let r_same = rv.ty == out_ty && rv.typmod == out_typmod;
            if lv.ty != out_ty {
                self.coercer().coerce_type(
                    &typed_var(&lv),
                    out_ty,
                    out_typmod,
                    CoercionContext::Implicit,
                )?;
            }
            if rv.ty != out_ty {
                self.coercer().coerce_type(
                    &typed_var(&rv),
                    out_ty,
                    out_typmod,
                    CoercionContext::Implicit,
                )?;
            }
            let join_var = Var {
                varno: rtindex,
                varattno: i16::try_from(res_vars.len() + 1).expect("column count"),
                levelsup: 0,
                ty: out_ty,
                typmod: out_typmod,
            };
            let (var, inputs) = match join_type {
                JoinType::Inner => {
                    if l_same {
                        (lv, Vec::new())
                    } else if r_same {
                        (rv, Vec::new())
                    } else {
                        (join_var, vec![lv])
                    }
                }
                JoinType::Left => {
                    if l_same {
                        (lv, Vec::new())
                    } else {
                        (join_var, vec![lv])
                    }
                }
                JoinType::Right => {
                    if r_same {
                        (rv, Vec::new())
                    } else {
                        (join_var, vec![rv])
                    }
                }
                JoinType::Full => (join_var, vec![lv, rv]),
            };
            res_vars.push(var);
            res_inputs.push(inputs);
        }
        // extractRemainingColumns for each side.
        for (i, name) in l_colnames.iter().enumerate() {
            if name.is_empty() || pairs.iter().any(|&(li, _)| li == i + 1) {
                continue;
            }
            res_colnames.push(name.clone());
            res_vars.push(self.side_var(scope, &l_top, i + 1));
            res_inputs.push(Vec::new());
        }
        for (i, name) in r_colnames.iter().enumerate() {
            if name.is_empty() || pairs.iter().any(|&(_, ri)| ri == i + 1) {
                continue;
            }
            res_colnames.push(name.clone());
            res_vars.push(self.side_var(scope, &r_top, i + 1));
            res_inputs.push(Vec::new());
        }
        // addRangeTableEntryForJoin: the alias renames the leading columns.
        let (alias_name, column_aliases) = self.plain_alias(alias)?;
        if column_aliases.len() > res_colnames.len() {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "join expression \"{}\" has {} columns available but {} columns specified",
                    alias_name.as_deref().unwrap_or("unnamed_join"),
                    res_colnames.len(),
                    column_aliases.len()
                ),
            )
            .into());
        }
        for (i, a) in column_aliases.into_iter().enumerate() {
            res_colnames[i] = a;
        }
        let eref_name = alias_name
            .clone()
            .unwrap_or_else(|| "unnamed_join".to_owned());
        scope.rtable.push(crate::relation::RangeTableEntry {
            kind: RteKind::Join {
                join_type,
                columns: res_vars,
                inputs: res_inputs,
            },
            alias: alias_name.clone(),
            eref_name,
            colnames: res_colnames,
            location,
            composite_only: false,
        });
        if let Some(name) = using_alias {
            let item = NamespaceItem {
                rt_index: rtindex,
                rel_visible: true,
                cols_visible: true,
                using_alias: Some((name, using.len())),
                lateral_only: false,
                lateral_ok: true,
            };
            self.relations
                .check_namespace_conflicts(scope, &my_ns, std::slice::from_ref(&item))?;
            my_ns.push(item);
        }
        // An alias hides the inputs; otherwise only their column names are
        // hidden.
        if alias_name.is_some() {
            my_ns.clear();
        } else {
            for item in &mut my_ns {
                item.cols_visible = false;
            }
        }
        let top = NamespaceItem {
            rt_index: rtindex,
            rel_visible: alias_name.is_some(),
            cols_visible: true,
            using_alias: None,
            lateral_only: false,
            lateral_ok: true,
        };
        my_ns.push(top.clone());
        Ok((top, my_ns))
    }

    /// The `Var` of one column of a join input.
    fn side_var(&self, scope: &Scope, item: &NamespaceItem, attnum: usize) -> Var {
        self.relations.make_var(
            item.rt_index,
            &scope.rtable[item.rt_index - 1],
            i16::try_from(attnum).expect("column count"),
            0,
        )
    }

    /// A function call's name, schema-qualified or not.
    fn func_call_name(
        &self,
        name: Handle<FlatFuncCallName>,
        location: usize,
    ) -> Result<ObjectName> {
        Ok(match self.get(name) {
            FlatFuncCallName::Name(n) => {
                let pgtk_sql::tokens::Flattype_function_name::Text(t) = self.get(*n);
                let span = self.text_span(*t);
                ObjectName {
                    schema: None,
                    name: self.fold(self.text(*t), span.start)?,
                }
            }
            FlatFuncCallName::Qualified(q) => {
                let q = self.get(*q);
                let (first, _) = self.col_id(q.first)?;
                let mut parts = vec![first];
                for part in self.list1(q.tail) {
                    parts.push(self.name_part(*part)?);
                }
                if parts.len() > 2 {
                    return Err(unsupported(
                        "catalog-qualified function name",
                        Some(location..location),
                    ));
                }
                let name = parts.pop().expect("at least two parts");
                ObjectName {
                    schema: parts.pop(),
                    name,
                }
            }
        })
    }

    /// One function of a `FROM` item, from its application expression.
    fn function_item(
        &self,
        f: Handle<FlatFunctionApplicationExpr>,
        coldef: Option<Handle<FlatRowsFromColDef>>,
    ) -> Result<FromFunc> {
        let location = self.span(f).start;
        let f = self.get(f);
        let name = self.func_call_name(f.name, location)?;
        let (coldef, coldef_location) = self.rows_from_coldef(coldef)?;
        Ok(FromFunc {
            call: FromFuncCall::Application(f.application),
            name,
            coldef,
            coldef_location,
            location,
        })
    }

    /// One special expression of a `FROM` item, as
    /// `transformRangeFunction` takes it: an expression the ordinary
    /// walker analyses, named by `FigureColname`, which gives the name of
    /// its form (`coalesce`, `nullif`, `trim`, and so on).
    fn special_function_item(
        &self,
        e: Handle<FlatExpr>,
        coldef: Option<Handle<FlatRowsFromColDef>>,
    ) -> Result<FromFunc> {
        let (coldef, coldef_location) = self.rows_from_coldef(coldef)?;
        Ok(FromFunc {
            call: FromFuncCall::Special(e),
            name: ObjectName {
                schema: None,
                name: self
                    .figure_colname(e)
                    .name
                    .unwrap_or_else(|| "?column?".to_owned()),
            },
            coldef,
            coldef_location,
            location: self.span(e).start,
        })
    }

    /// The column definition list of one `ROWS FROM()` item, with the
    /// location of its first entry.
    #[allow(clippy::type_complexity)]
    fn rows_from_coldef(
        &self,
        coldef: Option<Handle<FlatRowsFromColDef>>,
    ) -> Result<(
        Option<Vec<(String, Handle<pgtk_sql::ast::shared::expr::FlatCastType>)>>,
        Option<usize>,
    )> {
        let Some(cd) = coldef else {
            return Ok((None, None));
        };
        let cd = self.get(cd);
        let mut cols = Vec::new();
        let mut first = None;
        for c in self.list1(cd.columns) {
            if first.is_none() {
                first = Some(self.span(*c).start);
            }
            let c = self.get(*c);
            cols.push((self.col_id(c.name)?.0, c.type_name));
        }
        Ok((Some(cols), first))
    }

    /// A `func_alias_clause`: the alias name, plain column aliases, and a
    /// column definition list with the location of its first entry.
    #[allow(clippy::type_complexity)]
    fn func_alias(
        &self,
        alias: Option<Handle<FlatFuncTableAlias>>,
    ) -> Result<(
        Option<String>,
        Vec<String>,
        Option<Vec<(String, Handle<pgtk_sql::ast::shared::expr::FlatCastType>)>>,
        Option<usize>,
    )> {
        let Some(alias) = alias else {
            return Ok((None, Vec::new(), None, None));
        };
        // The alias is a name with an optional column list, which names
        // the columns or defines them, or a bare definition list after AS.
        enum Columns {
            Names(Vec<Handle<FlatColId>>),
            Definitions(Vec<Handle<FlatFuncTableColumnDef>>),
        }
        let list_of = |w: &Self, c: Option<Handle<FlatFuncTableAliasColumns>>| match c {
            None => Columns::Names(Vec::new()),
            Some(c) => match w.get(w.get(c).columns) {
                FlatFuncTableAliasColumnList::Names(l) => Columns::Names(w.list1(*l).to_vec()),
                FlatFuncTableAliasColumnList::Definitions(l) => {
                    Columns::Definitions(w.list1(*l).to_vec())
                }
            },
        };
        let (name, columns) = match self.get(alias) {
            FlatFuncTableAlias::WithAs(w) => match self.get(self.get(*w).body) {
                FlatFuncTableAliasAfterAs::Named(n) => {
                    let n = self.get(*n);
                    (Some(n.name), list_of(self, n.columns))
                }
                FlatFuncTableAliasAfterAs::Definitions(d) => (
                    None,
                    Columns::Definitions(self.list1(self.get(*d).columns).to_vec()),
                ),
            },
            FlatFuncTableAlias::Named(n) => {
                let n = self.get(*n);
                (Some(n.name), list_of(self, n.columns))
            }
        };
        let name = match name {
            Some(n) => Some(self.col_id(n)?.0),
            None => None,
        };
        match columns {
            Columns::Names(names) => {
                let mut plain = Vec::with_capacity(names.len());
                for c in names {
                    plain.push(self.col_id(c)?.0);
                }
                Ok((name, plain, None, None))
            }
            Columns::Definitions(list) => {
                let first = list.first().map(|c| self.span(*c).start);
                let mut defs = Vec::with_capacity(list.len());
                for c in list {
                    let c = self.get(c);
                    if let Some(cl) = c.collate {
                        let at = self.span(cl).start;
                        self.lookup_collation(self.get(cl).name, at)?;
                    }
                    defs.push((self.col_id(c.name)?.0, c.type_name));
                }
                Ok((name, Vec::new(), Some(defs), first))
            }
        }
    }

    /// `transformRangeFunction` and `addRangeTableEntryForFunction`.
    fn function_ref(
        &mut self,
        scope: &mut Scope,
        funcs: Vec<FromFunc>,
        ordinality: bool,
        alias: Option<Handle<FlatFuncTableAlias>>,
        is_rowsfrom: bool,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let saved_kind = self.expr_kind;
        self.expr_kind = ExprKind::FromFunction;
        scope.lateral_active = true;
        let result =
            self.function_ref_inner(scope, funcs, ordinality, alias, is_rowsfrom, location);
        scope.lateral_active = false;
        self.expr_kind = saved_kind;
        result
    }

    fn function_ref_inner(
        &mut self,
        scope: &mut Scope,
        funcs: Vec<FromFunc>,
        ordinality: bool,
        alias: Option<Handle<FlatFuncTableAlias>>,
        is_rowsfrom: bool,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let (alias_name, column_aliases, alias_coldef, alias_coldef_location) =
            self.func_alias(alias)?;
        // Each function: its result, its name for the column, its column
        // definition list, and the pg_proc row when it is one.
        let mut resolved: Vec<ResolvedFromFunc> = Vec::new();
        for f in &funcs {
            // A special expression is not a call: `transformRangeFunction`
            // analyses it with the ordinary expression walker, and
            // `FigureColname` has already named it.
            let site = match f.call {
                FromFuncCall::Application(site) => site,
                FromFuncCall::Special(e) => {
                    let srf_before = self.last_srf;
                    let typed = self.expr(scope, e)?;
                    if self.last_srf != srf_before && self.last_srf != Some(f.location) {
                        return Err(Diagnostic::new(
                            "0A000",
                            "set-returning functions must appear at top level of FROM",
                        )
                        .at(self.last_srf.unwrap_or(f.location))
                        .into());
                    }
                    if f.coldef.is_some() && alias_coldef.is_some() {
                        return Err(Diagnostic::new(
                            "42601",
                            "multiple column definition lists are not allowed for the same function",
                        )
                        .at(alias_coldef_location.unwrap_or(location))
                        .into());
                    }
                    resolved.push(ResolvedFromFunc {
                        ty: typed.ty,
                        typmod: typed.typmod,
                        funcname: f.name.name.clone(),
                        coldef: f.coldef.clone(),
                        coldef_location: f.coldef_location,
                        func: None,
                        actual: Vec::new(),
                        location: f.location,
                    });
                    continue;
                }
            };
            let app = self.get(site);
            // UNNEST(a, b, ...) is ROWS FROM(unnest(a), unnest(b), ...).
            if f.name.schema.is_none()
                && f.name.name == "unnest"
                && f.coldef.is_none()
                && let Some(body) = app.body
                && let FlatFunctionCallBody::Args(args) = self.get(body)
                && let args = self.get(*args)
                && args.order_by.is_none()
                && args.trailing_variadic.is_none()
                && self.list1(args.args).len() > 1
            {
                for arg in self.list1(args.args) {
                    let FlatFuncArg::Plain(e) = self.get(*arg) else {
                        return Err(unsupported("named argument", Some(f.location..f.location)));
                    };
                    let srf_before = self.last_srf;
                    let typed = self.expr(scope, *e)?;
                    let unnest = ObjectName {
                        schema: Some("pg_catalog".to_owned()),
                        name: "unnest".to_owned(),
                    };
                    let r = self.resolver.parse_func(
                        &unnest,
                        std::slice::from_ref(&typed),
                        &[],
                        f.location,
                        None,
                        Decoration::default(),
                    )?;
                    let ResolvedFunc::Function {
                        func,
                        rettype,
                        retset,
                        declared,
                    } = r
                    else {
                        return Err(unsupported("UNNEST form", Some(f.location..f.location)));
                    };
                    if typed.ty != declared[0] {
                        let (_, steps) = self.coercer().coerce_type(
                            &typed,
                            declared[0],
                            -1,
                            CoercionContext::Implicit,
                        )?;
                        self.record_coercions(*e, steps);
                    }
                    if retset {
                        if self.last_srf != srf_before {
                            return Err(Diagnostic::new(
                                "0A000",
                                "set-returning functions must appear at top level of FROM",
                            )
                            .at(self.last_srf.unwrap_or(f.location))
                            .into());
                        }
                        self.last_srf = Some(f.location);
                    }
                    let actual = if self.types().is_unknown(typed.ty) {
                        declared[0]
                    } else {
                        typed.ty
                    };
                    resolved.push(ResolvedFromFunc {
                        ty: rettype,
                        typmod: -1,
                        funcname: "unnest".to_owned(),
                        coldef: None,
                        coldef_location: None,
                        func: Some(func),
                        actual: vec![actual],
                        location: f.location,
                    });
                }
                continue;
            }
            let srf_before = self.last_srf;
            let typed = self.call(
                scope,
                CallSite::From(site),
                f.name.clone(),
                app.body,
                None,
                None,
                None,
                f.location,
            )?;
            if self.last_srf != srf_before && self.last_srf != Some(f.location) {
                return Err(Diagnostic::new(
                    "0A000",
                    "set-returning functions must appear at top level of FROM",
                )
                .at(self.last_srf.unwrap_or(f.location))
                .into());
            }
            if f.coldef.is_some() && alias_coldef.is_some() {
                return Err(Diagnostic::new(
                    "42601",
                    "multiple column definition lists are not allowed for the same function",
                )
                .at(alias_coldef_location.unwrap_or(location))
                .into());
            }
            let (func, declared) = self
                .resolved_function(CallSite::From(site))
                .map_or((None, Vec::new()), |r| {
                    (Some(r.function), r.declared.clone())
                });
            let mut actual = Vec::new();
            for (i, arg) in self.call_arg_types(app.body).into_iter().enumerate() {
                match arg {
                    Some(t) if !self.types().is_unknown(t) => actual.push(t),
                    _ => match declared.get(i) {
                        Some(d) => actual.push(*d),
                        None => break,
                    },
                }
            }
            // Defaulted arguments take their declared types.
            actual.extend(declared.iter().skip(actual.len()).copied());
            resolved.push(ResolvedFromFunc {
                ty: typed.ty,
                typmod: typed.typmod,
                funcname: f.name.name.clone(),
                coldef: f.coldef.clone(),
                coldef_location: f.coldef_location,
                func,
                actual,
                location: f.location,
            });
        }
        if let Some(cd) = alias_coldef {
            let at = alias_coldef_location.unwrap_or(location);
            if resolved.len() != 1 {
                return Err(if is_rowsfrom {
                    Diagnostic::new(
                        "42601",
                        "ROWS FROM() with multiple functions cannot have a column definition list",
                    )
                    .hint("Put a separate column definition list for each function inside ROWS FROM().")
                    .at(at)
                    .into()
                } else {
                    Diagnostic::new(
                        "42601",
                        "UNNEST() with multiple arguments cannot have a column definition list",
                    )
                    .hint("Use separate UNNEST() calls inside ROWS FROM(), and attach a column definition list to each one.")
                    .at(at)
                    .into()
                });
            }
            if ordinality {
                return Err(Diagnostic::new(
                    "42601",
                    "WITH ORDINALITY cannot be used with a column definition list",
                )
                .hint("Put the column definition list inside ROWS FROM().")
                .at(at)
                .into());
            }
            resolved[0].coldef = Some(cd);
            resolved[0].coldef_location = alias_coldef_location;
        }
        let nfuncs = resolved.len();
        let mut columns: Vec<(String, TypeRef, Typmod)> = Vec::new();
        for r in &resolved {
            let class = self.result_class(r);
            let coldef_at = r.coldef_location.unwrap_or(r.location);
            if let Some(cd) = &r.coldef {
                match class {
                    ResultClass::Record => {
                        for (name, cast_type) in cd {
                            let tn = self.type_name(*cast_type)?;
                            let (ty, typmod) = self.typenames.resolve(&tn)?;
                            columns.push((name.clone(), ty, typmod));
                        }
                    }
                    ResultClass::Composite(_) => {
                        let message = if self.types().pseudo(r.ty) == Some(&PseudoType::Record) {
                            "a column definition list is redundant for a function with OUT parameters"
                        } else {
                            "a column definition list is redundant for a function returning a named composite type"
                        };
                        return Err(Diagnostic::new("42601", message).at(coldef_at).into());
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            "42601",
                            "a column definition list is only allowed for functions returning \"record\"",
                        )
                        .at(coldef_at)
                        .into());
                    }
                }
                continue;
            }
            match class {
                ResultClass::Record => {
                    return Err(Diagnostic::new(
                        "42601",
                        "a column definition list is required for functions returning \"record\"",
                    )
                    .at(r.location)
                    .into());
                }
                ResultClass::Composite(fields) => columns.extend(fields),
                ResultClass::Scalar => {
                    // chooseScalarFunctionAlias
                    let out_name = r.func.and_then(|f| {
                        let proc = &self.catalog()[f];
                        let outs: Vec<_> = proc
                            .arguments
                            .iter()
                            .filter(|a| {
                                matches!(a.mode, ArgMode::Out | ArgMode::InOut | ArgMode::Table)
                            })
                            .collect();
                        match outs.as_slice() {
                            [only] => only
                                .name
                                .as_ref()
                                .map(|n| n.to_string())
                                .filter(|n| !n.is_empty()),
                            _ => None,
                        }
                    });
                    let name = match (out_name, &alias_name) {
                        (Some(n), _) => n,
                        (None, Some(a)) if nfuncs == 1 => a.clone(),
                        _ => r.funcname.clone(),
                    };
                    columns.push((name, r.ty, r.typmod));
                }
                ResultClass::Other => {
                    return Err(Diagnostic::new(
                        "42804",
                        format!(
                            "function \"{}\" in FROM has unsupported return type {}",
                            r.funcname,
                            self.types().format(r.ty)
                        ),
                    )
                    .at(r.location)
                    .into());
                }
            }
        }
        if ordinality {
            columns.push((
                "ordinality".to_owned(),
                self.types().builtin(Builtin::Int8),
                -1,
            ));
        }
        let eref_name = alias_name
            .clone()
            .unwrap_or_else(|| resolved[0].funcname.clone());
        // buildRelationAliases: aliases rename the live columns in order.
        let live = columns.iter().filter(|c| !c.0.is_empty()).count();
        if column_aliases.len() > live {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "table \"{eref_name}\" has {live} columns available but {} columns specified",
                    column_aliases.len()
                ),
            )
            .into());
        }
        let mut aliases = column_aliases.into_iter();
        let mut colnames = Vec::with_capacity(columns.len());
        let mut coltypes = Vec::with_capacity(columns.len());
        for (name, ty, typmod) in columns {
            if name.is_empty() {
                colnames.push(name);
            } else {
                colnames.push(aliases.next().unwrap_or(name));
            }
            coltypes.push((ty, typmod));
        }
        // makeWholeRowVar: a single composite-returning function gives its
        // type to a whole-row reference.
        let row_type = if nfuncs == 1 && !ordinality {
            let r = &resolved[0];
            self.types()
                .pseudo(r.ty)
                .is_none()
                .then(|| self.types().base_type(r.ty))
                .filter(|t| matches!(self.catalog()[*t].kind, TypeKind::Composite(_)))
                .map(|_| r.ty)
        } else {
            None
        };
        scope.rtable.push(crate::relation::RangeTableEntry {
            kind: RteKind::Function {
                columns: coltypes,
                row_type,
            },
            alias: alias_name,
            eref_name,
            colnames,
            location,
            composite_only: false,
        });
        let item = NamespaceItem::plain(scope.rtable.len());
        Ok((item.clone(), vec![item]))
    }

    /// A `table_alias` with its optional column list.
    fn table_alias(
        &self,
        alias: Option<Handle<FlatTableAlias>>,
    ) -> Result<(Option<String>, Vec<String>)> {
        let Some(a) = alias else {
            return Ok((None, Vec::new()));
        };
        let (name, columns) = match self.get(a) {
            FlatTableAlias::WithAs(w) => {
                let w = self.get(*w);
                (w.name, w.columns)
            }
            FlatTableAlias::Bare(b) => {
                let b = self.get(*b);
                (b.name, b.columns)
            }
        };
        let (name, _) = self.col_id(name)?;
        let mut cols = Vec::new();
        if let Some(c) = columns {
            for id in self.list1(self.get(c).0) {
                cols.push(self.col_id(*id)?.0);
            }
        }
        Ok((Some(name), cols))
    }

    /// `transformRangeTableFunc` for XMLTABLE: the row and document
    /// expressions, the columns with their types, paths and defaults, and
    /// the namespaces; then `addRangeTableEntryForTableFunc`.
    /// `transformJsonTable`: the document expression, the path, the
    /// PASSING arguments and each column as the SQL/JSON query function it
    /// stands for; the columns of a level come before those of its NESTED
    /// paths.
    #[cfg(feature = "since-pg17")]
    fn json_table_ref(
        &mut self,
        scope: &mut Scope,
        j: Handle<FlatJsonTableRef>,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let j = self.get(j);
        let inner = self.get(self.get(j.table).inner);
        // The top-level ON ERROR takes EMPTY [ARRAY] or ERROR only.
        if let Some(b) = inner.on_error
            && !matches!(
                self.get(self.get(b).behavior),
                FlatJsonBehavior::Error | FlatJsonBehavior::Empty | FlatJsonBehavior::EmptyArray
            )
        {
            return Err(Diagnostic::new("42601", "invalid ON ERROR behavior")
                .detail(
                    "Only EMPTY [ ARRAY ] or ERROR is allowed in the top-level ON ERROR clause.",
                )
                .at(self.span(b).start)
                .into());
        }
        // CheckDuplicateColumnOrPathNames, with the root path name first.
        let mut names: Vec<String> = Vec::new();
        match inner.path_name {
            Some(n) => names.push(self.alias_name(self.get(n).name)?.0),
            None => names.push("json_table_path_0".to_owned()),
        }
        self.json_table_names(inner.column_list, &mut names)?;
        let mut lscope = scope.clone();
        lscope.lateral_active = true;
        let saved_kind = self.expr_kind;
        self.expr_kind = ExprKind::FromFunction;
        let result = self.json_table_body(&lscope, inner);
        self.expr_kind = saved_kind;
        let columns = result?;
        let (alias_name, column_aliases) = self.table_alias(j.alias)?;
        let eref_name = alias_name
            .clone()
            .unwrap_or_else(|| "json_table".to_owned());
        if column_aliases.len() > columns.len() {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "JSON_TABLE function has {} columns available but {} columns specified",
                    columns.len(),
                    column_aliases.len()
                ),
            )
            .into());
        }
        let mut aliases = column_aliases.into_iter();
        let mut colnames = Vec::with_capacity(columns.len());
        let mut coltypes = Vec::with_capacity(columns.len());
        for (name, ty, typmod) in columns {
            colnames.push(aliases.next().unwrap_or(name));
            coltypes.push((ty, typmod));
        }
        scope.rtable.push(crate::relation::RangeTableEntry {
            kind: RteKind::Function {
                columns: coltypes,
                row_type: None,
            },
            alias: alias_name,
            eref_name,
            colnames,
            location,
            composite_only: false,
        });
        let item = NamespaceItem::plain(scope.rtable.len());
        Ok((item.clone(), vec![item]))
    }

    /// Every column name and written path name of a JSON_TABLE, each once.
    #[cfg(feature = "since-pg17")]
    fn json_table_names(
        &self,
        list: Handle<FlatJsonTableColumnList>,
        names: &mut Vec<String>,
    ) -> Result<()> {
        for col in self.list1(self.get(list).list).to_vec() {
            match self.get(col) {
                FlatJsonTableColumn::Nested(n) => {
                    let n = self.get(*n);
                    if let Some(pn) = n.as_name {
                        let (name, at) = self.alias_name(self.get(pn).name)?;
                        if names.contains(&name) {
                            return Err(Diagnostic::new(
                                "42712",
                                format!("duplicate JSON_TABLE column or path name: {name}"),
                            )
                            .at(at)
                            .into());
                        }
                        names.push(name);
                    }
                    self.json_table_names(n.columns, names)?;
                }
                FlatJsonTableColumn::Valued(v) => {
                    let (name, _) = self.alias_name(self.get(*v).name)?;
                    if names.contains(&name) {
                        return Err(Diagnostic::new(
                            "42712",
                            format!("duplicate JSON_TABLE column or path name: {name}"),
                        )
                        .at(self.span(col).start)
                        .into());
                    }
                    names.push(name);
                }
            }
        }
        Ok(())
    }

    /// The document, root path and PASSING arguments of a JSON_TABLE, then
    /// its columns.
    #[cfg(feature = "since-pg17")]
    fn json_table_body(
        &mut self,
        lscope: &Scope,
        inner: &FlatJsonTableInner,
    ) -> Result<Vec<(String, TypeRef, Typmod)>> {
        // The path is a string constant: the grammar takes nothing else.
        let path = {
            use pgtk_sql::ast::dml::select::FlatJsonTablePath;
            let mut path = inner.path;
            loop {
                match self.get(path) {
                    FlatJsonTablePath::Const(e) => break *e,
                    FlatJsonTablePath::Parens(p) => path = self.get(*p).path,
                }
            }
        };
        let jsonb = self.types().builtin(Builtin::Jsonb);
        let context_format = self.json_table_format(inner.context_format)?;
        self.json_value_expr_formatted(
            lscope,
            "JSON_TABLE",
            inner.context,
            context_format,
            JsonFormatKind::Jsonb,
            Some(jsonb),
            false,
        )?;
        let path_t = self.expr(lscope, path)?;
        let jsonpath = self.types().builtin(Builtin::Jsonpath);
        if let Some((_, steps)) = self.coercer().coerce_to_target_type(
            &path_t,
            jsonpath,
            -1,
            CoercionContext::Explicit,
        )? {
            self.record_coercions(path, steps);
        }
        if let Some(p) = inner.passing {
            for arg in self.list1(self.get(p).args).to_vec() {
                let arg = self.get(arg);
                self.json_value_expr(
                    lscope,
                    "JSON_TABLE",
                    arg.value,
                    None,
                    JsonFormatKind::Jsonb,
                    None,
                    true,
                )?;
            }
        }
        let mut columns = Vec::new();
        self.json_table_columns(lscope, inner.column_list, inner.passing, &mut columns)?;
        Ok(columns)
    }

    /// `transformJsonTableColumns` for one level: its own columns, then
    /// the columns of each NESTED path.
    #[cfg(feature = "since-pg17")]
    fn json_table_columns(
        &mut self,
        lscope: &Scope,
        list: Handle<FlatJsonTableColumnList>,
        passing: Option<Handle<FlatJsonPassing>>,
        columns: &mut Vec<(String, TypeRef, Typmod)>,
    ) -> Result<()> {
        let cols = self.list1(self.get(list).list).to_vec();
        let mut ordinality_found = false;
        for col in &cols {
            let FlatJsonTableColumn::Valued(v) = self.get(*col) else {
                continue;
            };
            let v = self.get(*v);
            let (name, _) = self.alias_name(v.name)?;
            let at = self.span(*col).start;
            match self.get(v.kind) {
                FlatJsonTableColumnKind::Ordinality(_) => {
                    if ordinality_found {
                        return Err(Diagnostic::new(
                            "42601",
                            "only one FOR ORDINALITY column is allowed",
                        )
                        .at(at)
                        .into());
                    }
                    ordinality_found = true;
                    columns.push((name, self.types().builtin(Builtin::Int4), -1));
                }
                FlatJsonTableColumnKind::Typed(tc) => {
                    let tc = self.get(*tc);
                    let (exists, format, wrapper, quotes, on_empty, on_error) =
                        match self.get(tc.tail) {
                            FlatJsonTableTypedTail::Exists(e) => {
                                let e = self.get(*e);
                                let on_error = e.on_error.map(|e| self.json_on_error(e));
                                (true, None, None, None, None, on_error)
                            }
                            FlatJsonTableTypedTail::Regular(r) => {
                                let r = self.get(*r);
                                let (on_empty, on_error) = self.json_behaviors(r.on_behavior);
                                (false, r.format, r.wrapper, r.quotes, on_empty, on_error)
                            }
                        };
                    let op = if exists {
                        JsonOp::Exists
                    } else if format.is_some() || quotes.is_some() || wrapper.is_some() {
                        JsonOp::Query
                    } else {
                        // A regular column of a composite type is formatted.
                        let tn = self.type_name(tc.ty)?;
                        let (ty, _) = self.typenames.resolve(&tn)?;
                        if self.json_composite_type(ty) {
                            JsonOp::Query
                        } else {
                            JsonOp::Value
                        }
                    };
                    let parts = JsonFuncParts {
                        context: JsonContext::Typed,
                        path: None,
                        passing,
                        returning: JsonReturningSpec::Column(tc.ty, format),
                        wrapper,
                        quotes,
                        on_empty,
                        on_error,
                        column: Some(name.clone()),
                    };
                    let t = self.json_func(lscope, op, &parts, at)?;
                    columns.push((name, t.ty, t.typmod));
                }
            }
        }
        for col in &cols {
            if let FlatJsonTableColumn::Nested(n) = self.get(*col) {
                let n = self.get(*n);
                self.json_table_columns(lscope, n.columns, passing, columns)?;
            }
        }
        Ok(())
    }

    /// `isCompositeType` of parse_jsontable.c: json, jsonb, record, an
    /// array, a composite type, or a domain over one of those.
    #[cfg(feature = "since-pg17")]
    fn json_composite_type(&self, ty: TypeRef) -> bool {
        let types = self.types();
        let base = types.base_type(ty);
        types.is(base, Builtin::Json)
            || types.is(base, Builtin::Jsonb)
            || types.is(base, Builtin::Record)
            || types.element_type(base).is_some()
            || matches!(
                self.catalog()[base].kind,
                pgtk_catalog::TypeKind::Composite(_)
            )
    }

    fn xml_table_ref(
        &mut self,
        scope: &mut Scope,
        x: Handle<FlatXmlTableRef>,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let x = self.get(x);
        let inner = self.get(self.get(x.table).inner);
        let mut lscope = scope.clone();
        lscope.lateral_active = true;
        let saved_kind = self.expr_kind;
        self.expr_kind = ExprKind::FromFunction;
        let text = self.types().builtin(Builtin::Text);
        let xml = self.types().builtin(Builtin::Xml);
        let t = self.expr(&lscope, inner.row_expr)?;
        self.coerce_to_specific_type(inner.row_expr, &t, text, "XMLTABLE")?;
        let doc = self.get(inner.passing).doc;
        let t = self.expr(&lscope, doc)?;
        self.coerce_to_specific_type(doc, &t, xml, "XMLTABLE")?;
        let mut columns: Vec<(String, TypeRef, Typmod)> = Vec::new();
        let mut ordinality_seen = false;
        for col in self.list1(self.get(inner.column_list).0).to_vec() {
            let c = self.get(col);
            let (name, at) = self.alias_name(c.name)?;
            let (ty, typmod) = match self.get(c.kind) {
                FlatXmlTableColumnKind::Ordinality(_) => {
                    if ordinality_seen {
                        return Err(Diagnostic::new(
                            "42601",
                            "only one FOR ORDINALITY column is allowed",
                        )
                        .at(at)
                        .into());
                    }
                    ordinality_seen = true;
                    (self.types().builtin(Builtin::Int4), -1)
                }
                FlatXmlTableColumnKind::Typed(tc) => {
                    let tc = self.get(*tc);
                    let tn = self.type_name(tc.ty)?;
                    let (ty, typmod) = self.typenames.resolve(&tn)?;
                    let (path, default) = self.xmltable_column_options(tc.options, &name)?;
                    if let Some(e) = path {
                        let t = self.expr(&lscope, e)?;
                        self.coerce_to_specific_type(e, &t, text, "XMLTABLE")?;
                    }
                    if let Some(e) = default {
                        let t = self.expr(&lscope, e)?;
                        // coerce_to_specific_type_typmod
                        match self.coercer().coerce_to_target_type(
                            &t,
                            ty,
                            typmod,
                            CoercionContext::Assignment,
                        )? {
                            Some((_, steps)) => self.record_coercions(e, steps),
                            None => {
                                return Err(Diagnostic::new(
                                    "42804",
                                    format!(
                                        "argument of XMLTABLE must be type {}, not type {}",
                                        self.types().format(ty),
                                        self.types().format(t.ty)
                                    ),
                                )
                                .at(t.location)
                                .into());
                            }
                        }
                    }
                    (ty, typmod)
                }
            };
            if columns.iter().any(|(n, _, _)| *n == name) {
                return Err(Diagnostic::new(
                    "42601",
                    format!("column name \"{name}\" is not unique"),
                )
                .at(at)
                .into());
            }
            columns.push((name, ty, typmod));
        }
        if let Some(ns) = inner.namespaces {
            let mut names: Vec<String> = Vec::new();
            let mut default_seen = false;
            for item in self.list1(self.get(ns).items).to_vec() {
                let at = self.span(item).start;
                match self.get(item) {
                    FlatXmlNamespaceItem::Default(d) => {
                        let uri = self.get(*d).uri;
                        let t = self.expr(&lscope, uri)?;
                        self.coerce_to_specific_type(uri, &t, text, "XMLTABLE")?;
                        if default_seen {
                            return Err(Diagnostic::new(
                                "42601",
                                "only one default namespace is allowed",
                            )
                            .at(at)
                            .into());
                        }
                        default_seen = true;
                    }
                    FlatXmlNamespaceItem::Named(n) => {
                        let n = self.get(*n);
                        let t = self.expr(&lscope, n.uri)?;
                        self.coerce_to_specific_type(n.uri, &t, text, "XMLTABLE")?;
                        let (prefix, _) = self.alias_name(n.prefix)?;
                        if names.contains(&prefix) {
                            return Err(Diagnostic::new(
                                "42601",
                                format!("namespace name \"{prefix}\" is not unique"),
                            )
                            .at(at)
                            .into());
                        }
                        names.push(prefix);
                    }
                }
            }
        }
        self.expr_kind = saved_kind;
        let (alias_name, column_aliases) = self.table_alias(x.alias)?;
        let eref_name = alias_name.clone().unwrap_or_else(|| "xmltable".to_owned());
        if column_aliases.len() > columns.len() {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "XMLTABLE function has {} columns available but {} columns specified",
                    columns.len(),
                    column_aliases.len()
                ),
            )
            .into());
        }
        let mut aliases = column_aliases.into_iter();
        let mut colnames = Vec::with_capacity(columns.len());
        let mut coltypes = Vec::with_capacity(columns.len());
        for (name, ty, typmod) in columns {
            colnames.push(aliases.next().unwrap_or(name));
            coltypes.push((ty, typmod));
        }
        scope.rtable.push(crate::relation::RangeTableEntry {
            kind: RteKind::Function {
                columns: coltypes,
                row_type: None,
            },
            alias: alias_name,
            eref_name,
            colnames,
            location,
            composite_only: false,
        });
        let item = NamespaceItem::plain(scope.rtable.len());
        Ok((item.clone(), vec![item]))
    }

    /// The recorded type of each plain argument of a call body.
    fn call_arg_types(&self, body: Option<Handle<FlatFunctionCallBody>>) -> Vec<Option<TypeRef>> {
        let Some(body) = body else {
            return Vec::new();
        };
        let mut items: Vec<Handle<FlatFuncArg>> = Vec::new();
        match self.get(body) {
            FlatFunctionCallBody::Args(args) => {
                let args = self.get(*args);
                items.extend(self.list1(args.args).iter().copied());
                if let Some(tv) = args.trailing_variadic {
                    items.push(self.get(self.get(tv).variadic).argument);
                }
            }
            FlatFunctionCallBody::All(all) => {
                items.extend(self.list1(self.get(self.get(*all).args).0).iter().copied());
            }
            FlatFunctionCallBody::Distinct(d) => {
                items.extend(self.list1(self.get(self.get(*d).args).0).iter().copied());
            }
            FlatFunctionCallBody::LeadingVariadic(l) => {
                items.push(self.get(self.get(*l).variadic).argument);
            }
            FlatFunctionCallBody::Star(_) => {}
        }
        items
            .iter()
            .map(|a| {
                let e = match self.get(*a) {
                    FlatFuncArg::Plain(e) => *e,
                    FlatFuncArg::Named(n) => self.get(*n).value,
                };
                self.analysis.expr_types.get(e).map(|t| t.ty)
            })
            .collect()
    }

    /// `get_expr_result_type` for a function in FROM.
    fn result_class(&self, r: &ResolvedFromFunc) -> ResultClass {
        let catalog = self.catalog();
        let types = self.types();
        if let Some(func) = r.func {
            let proc = &catalog[func];
            let outs: Vec<&pgtk_catalog::Argument> = proc
                .arguments
                .iter()
                .filter(|a| matches!(a.mode, ArgMode::Out | ArgMode::InOut | ArgMode::Table))
                .collect();
            // build_function_result_tupdesc_t: OUT parameters make the
            // columns of a function declared to return record.
            if types.pseudo(proc.returns) == Some(&PseudoType::Record)
                && (outs.len() >= 2 || proc.kind == FunctionKind::Procedure)
            {
                let raw: Vec<TypeRef> = proc.input_arguments().map(|a| a.ty).collect();
                let actual: Vec<TypeRef> = if r.actual.len() == raw.len() {
                    r.actual.clone()
                } else {
                    raw.clone()
                };
                let mut fields = Vec::new();
                for (i, a) in outs.iter().enumerate() {
                    let name = a
                        .name
                        .as_ref()
                        .map(|n| n.to_string())
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| format!("column{}", i + 1));
                    let ty = if types.is_polymorphic(a.ty) {
                        // resolve_polymorphic_tupdesc
                        match self
                            .coercer()
                            .enforce_generic_type_consistency(&actual, &raw, a.ty, false)
                        {
                            Ok((resolved, _)) if !types.is_polymorphic(resolved) => resolved,
                            _ => return ResultClass::Record,
                        }
                    } else {
                        a.ty
                    };
                    fields.push((name, ty, -1));
                }
                return ResultClass::Composite(fields);
            }
        }
        let ty = r.ty;
        if let Some(p) = types.pseudo(ty) {
            return match p {
                PseudoType::Record => ResultClass::Record,
                PseudoType::Void | PseudoType::Cstring => ResultClass::Scalar,
                _ => ResultClass::Other,
            };
        }
        let base = types.base_type(ty);
        if let TypeKind::Composite(c) = &catalog[base].kind {
            let mut fields = Vec::new();
            match c.relation {
                Some(rel) => {
                    let rel = &catalog[rel];
                    let max = rel.columns.iter().map(|c| c.position).max().unwrap_or(0);
                    let placeholder = types.builtin(Builtin::Int4);
                    let mut all =
                        vec![(String::new(), placeholder, -1); usize::try_from(max).unwrap_or(0)];
                    for col in &rel.columns {
                        let i = usize::try_from(col.position - 1).expect("attnum >= 1");
                        all[i] = (col.name.to_string(), col.ty, col.typmod.0);
                    }
                    fields = all;
                }
                None => {
                    for f in &c.fields {
                        fields.push((f.name.to_string(), f.ty, f.typmod.0));
                    }
                }
            }
            return ResultClass::Composite(fields);
        }
        ResultClass::Scalar
    }

    /// `transformRangeSubselect` and `addRangeTableEntryForSubquery`.
    fn subquery_ref(
        &mut self,
        scope: &mut Scope,
        sub: Handle<FlatSubquery>,
        alias: Option<Handle<FlatPlainTableAlias>>,
        lateral: bool,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let (alias_name, column_aliases) = self.plain_alias(alias)?;
        if alias_name.is_none() && self.settings.target_version() < TargetVersion::Pg16 {
            return Err(
                Diagnostic::new("42601", "subquery in FROM must have an alias")
                    .hint("For example, FROM (SELECT ...) [AS] foo.")
                    .at(location)
                    .into(),
            );
        }
        let saved_kind = self.expr_kind;
        self.expr_kind = ExprKind::FromSubselect;
        let target_list = self.subquery(scope, sub, lateral);
        self.expr_kind = saved_kind;
        let target_list = target_list?;
        let lock_refusal = self.lock_refusal.take();
        let columns: Vec<TargetEntry> = target_list.into_iter().filter(|t| !t.junk).collect();
        let eref_name = alias_name
            .clone()
            .unwrap_or_else(|| "unnamed_subquery".to_owned());
        if column_aliases.len() > columns.len() {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "table \"{eref_name}\" has {} columns available but {} columns specified",
                    columns.len(),
                    column_aliases.len()
                ),
            )
            .into());
        }
        let mut colnames: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
        for (i, a) in column_aliases.into_iter().enumerate() {
            colnames[i] = a;
        }
        scope.rtable.push(crate::relation::RangeTableEntry {
            kind: RteKind::Subquery {
                columns,
                lock_refusal,
            },
            alias: alias_name.clone(),
            eref_name,
            colnames,
            location,
            composite_only: false,
        });
        let item = NamespaceItem {
            rel_visible: alias_name.is_some(),
            ..NamespaceItem::plain(scope.rtable.len())
        };
        Ok((item.clone(), vec![item]))
    }

    /// `transformFromClauseItem` for one table reference without joins.
    fn simple_table_ref(
        &mut self,
        scope: &mut Scope,
        base: Handle<FlatSimpleTableRef>,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        {
            let (schema, name, inh, alias, tablesample) = match self.get(base) {
                FlatSimpleTableRef::Named(named) => {
                    let named = self.get(*named);
                    let (schema, name) = match self.get(named.name) {
                        FlatTableFunctionName::Name(n) => {
                            let pgtk_sql::tokens::Flattable_function_name::Text(t) = self.get(*n);
                            let span = self.text_span(*t);
                            (None, self.fold(self.text(*t), span.start)?)
                        }
                        FlatTableFunctionName::Qualified(q) => {
                            let q = self.get(*q);
                            let (first, _) = self.col_id(q.first)?;
                            let mut parts = vec![first, self.name_part(q.second)?];
                            if let Some(third) = q.third {
                                parts.push(self.name_part(third)?);
                            }
                            if parts.len() > 2 {
                                return Err(unsupported(
                                    "catalog-qualified table name",
                                    Some(location..location),
                                ));
                            }
                            let name = parts.pop().expect("at least two parts");
                            (parts.pop(), name)
                        }
                    };
                    let (inh, alias, tablesample) = match named.tail.map(|t| self.get(t)) {
                        None => (true, None, None),
                        Some(FlatNamedTableRefTail::Alias(a)) => {
                            let a = self.get(*a);
                            (true, Some(a.alias), a.tablesample)
                        }
                        Some(FlatNamedTableRefTail::Sample(ts)) => (true, None, Some(*ts)),
                        Some(FlatNamedTableRefTail::Inherited(i)) => {
                            let i = self.get(*i);
                            (true, i.alias, i.tablesample)
                        }
                        Some(FlatNamedTableRefTail::Function(t)) => {
                            let t = self.get(*t);
                            let func = FromFunc {
                                call: FromFuncCall::Application(t.application),
                                name: ObjectName { schema, name },
                                coldef: None,
                                coldef_location: None,
                                location,
                            };
                            return self.function_ref(
                                scope,
                                vec![func],
                                t.ordinality,
                                t.alias,
                                false,
                                location,
                            );
                        }
                    };
                    (schema, name, inh, alias, tablesample)
                }
                FlatSimpleTableRef::Only(only) => {
                    let only = self.get(*only);
                    let qn = match self.get(only.name) {
                        FlatOnlyRelationName::Parens(p) => self.get(*p).name,
                        FlatOnlyRelationName::Plain(q) => *q,
                    };
                    let mut parts = self.qualified_name_parts(qn)?;
                    if parts.len() > 2 {
                        return Err(unsupported(
                            "catalog-qualified table name",
                            Some(location..location),
                        ));
                    }
                    let name = parts.pop().expect("at least one part");
                    (parts.pop(), name, false, only.alias, only.tablesample)
                }
                FlatSimpleTableRef::ColName(c) => {
                    let c = self.get(*c);
                    // A keyword usable as a table name; its spelling is the
                    // source text, folded.
                    let span = self.span(c.name);
                    let raw = &self.source[span.clone()];
                    let name = self.fold(raw, span.start)?;
                    let (inh, alias, tablesample) = match c.tail.map(|t| self.get(t)) {
                        None => (true, None, None),
                        Some(FlatColNameTableTail::Alias(a)) => {
                            let a = self.get(*a);
                            (true, Some(a.alias), a.tablesample)
                        }
                        Some(FlatColNameTableTail::Sample(ts)) => (true, None, Some(*ts)),
                        Some(FlatColNameTableTail::Inherited(i)) => {
                            let i = self.get(*i);
                            (true, i.alias, i.tablesample)
                        }
                    };
                    (None, name, inh, alias, tablesample)
                }
                FlatSimpleTableRef::Paren(p) => {
                    return match self.get(*p) {
                        FlatParenTableRef::Query(q) => {
                            let q = self.get(*q);
                            let paren = self.span(q.query).start;
                            let sub = self.get(q.query).inner;
                            self.subquery_ref(scope, sub, subquery_alias!(q.alias), false, paren)
                        }
                        FlatParenTableRef::Join(j) => {
                            let j = self.get(*j);
                            self.paren_join_group(scope, j.group, j.alias)
                        }
                    };
                }
                FlatSimpleTableRef::Lateral(l) => {
                    return match self.get(self.get(*l).body) {
                        FlatLateralBody::Subquery(s) => {
                            let paren = self.span(*s).start;
                            let s = self.get(*s);
                            self.subquery_ref(scope, s.query, subquery_alias!(s.alias), true, paren)
                        }
                        FlatLateralBody::Func(f) => {
                            let f = self.get(*f);
                            let func = self.function_item(f.func, None)?;
                            self.function_ref(
                                scope,
                                vec![func],
                                f.ordinality,
                                f.alias,
                                false,
                                location,
                            )
                        }
                        // XMLTABLE is LATERAL whether or not it says so.
                        FlatLateralBody::XmlTable(x) => self.xml_table_ref(scope, *x, location),
                        // JSON_TABLE is LATERAL whether or not it says so.
                        #[cfg(feature = "since-pg17")]
                        FlatLateralBody::JsonTable(j) => self.json_table_ref(scope, *j, location),
                    };
                }
                FlatSimpleTableRef::XmlTable(x) => {
                    return self.xml_table_ref(scope, *x, location);
                }
                #[cfg(feature = "since-pg17")]
                FlatSimpleTableRef::JsonTable(j) => {
                    return self.json_table_ref(scope, *j, location);
                }
                FlatSimpleTableRef::RowsFrom(r) => {
                    let r = self.get(*r);
                    let mut funcs = Vec::new();
                    for item in self.list1(self.get(r.items).0) {
                        let item = self.get(*item);
                        funcs.push(match self.get(item.func) {
                            FlatRowsFromFunc::Func(f) => self.function_item(*f, item.coldef)?,
                            FlatRowsFromFunc::Special(e) => {
                                self.special_function_item(*e, item.coldef)?
                            }
                        });
                    }
                    return self.function_ref(scope, funcs, r.ordinality, r.alias, true, location);
                }
                FlatSimpleTableRef::LongFunction(r) => {
                    // `DeconstructQualifiedName`: a function name has at most
                    // three parts.
                    let name = self.get(self.get(*r).name);
                    let (first, _) = self.col_id(name.first)?;
                    let mut parts = vec![
                        first,
                        self.name_part(name.second)?,
                        self.name_part(name.third)?,
                        self.name_part(name.fourth)?,
                    ];
                    for part in self.list(name.more) {
                        parts.push(self.name_part(*part)?);
                    }
                    return Err(Diagnostic::new(
                        "42601",
                        format!(
                            "improper qualified name (too many dotted names): {}",
                            parts.join(".")
                        ),
                    )
                    .at(location)
                    .into());
                }
                FlatSimpleTableRef::SpecialFunc(r) => {
                    let r = self.get(*r);
                    let func = self.special_function_item(r.func, None)?;
                    return self.function_ref(
                        scope,
                        vec![func],
                        r.ordinality,
                        r.alias,
                        false,
                        location,
                    );
                }
            };
            let (alias_name, column_aliases) = self.plain_alias(alias)?;
            // getNSItemForSpecialRelationTypes: an unqualified name may be a
            // WITH query.
            let result = if schema.is_none()
                && let Some((levelsup, index)) = self.scan_cte(scope, &name)
            {
                self.cte_ref(scope, levelsup, index, alias_name, column_aliases, location)?
            } else {
                let rt_index = self.relations.add_range_table_entry(
                    scope,
                    schema.as_deref(),
                    &name,
                    alias_name.as_deref(),
                    &column_aliases,
                    inh,
                    location,
                )?;
                let item = NamespaceItem::plain(rt_index);
                (item.clone(), vec![item])
            };
            if let Some(ts) = tablesample {
                self.tablesample(scope, result.0.rt_index, ts)?;
            }
            Ok(result)
        }
    }

    fn name_part(
        &self,
        handle: Handle<pgtk_sql::ast::shared::expr::FlatFuncCallNamePart>,
    ) -> Result<String> {
        let span = self.span(handle);
        let raw = self.source[span.clone()].trim_start_matches('.').trim();
        self.fold(raw, span.start)
    }

    /// gram.y `xmltable_column_el`: the `PATH` and `DEFAULT` expressions of
    /// a typed column, with its checks for a repeated option. An option
    /// named by an identifier counts when its folded name is `path` or
    /// `default`; pgtk-sql rejects any other name.
    fn xmltable_column_options(
        &self,
        options: List<Handle<FlatXmlTableColumnOption>>,
        column: &str,
    ) -> Result<XmlColumnExprs> {
        let mut path = None;
        let mut default = None;
        let mut nullability_seen = false;
        for option in self.list(options) {
            let at = self.span(*option).start;
            let (is_path, value) = match self.get(*option) {
                #[cfg(feature = "since-pg17")]
                FlatXmlTableColumnOption::Path(e) => (true, *e),
                FlatXmlTableColumnOption::Default(e) => (false, *e),
                FlatXmlTableColumnOption::Named(n) => {
                    let n = self.get(*n);
                    let pgtk_sql::tokens::literal::FlatIdentOnly::Text(t) = self.get(n.name);
                    let name = pgtk_sql::ident::fold_name(self.text(*t));
                    (name.as_deref() == Some("path"), n.value)
                }
                FlatXmlTableColumnOption::NotNull | FlatXmlTableColumnOption::Null => {
                    if nullability_seen {
                        return Err(Diagnostic::new(
                            "42601",
                            format!(
                                "conflicting or redundant NULL / NOT NULL declarations for column \"{column}\""
                            ),
                        )
                        .at(at)
                        .into());
                    }
                    nullability_seen = true;
                    continue;
                }
            };
            let (slot, message) = if is_path {
                (&mut path, "only one PATH value per column is allowed")
            } else {
                (&mut default, "only one DEFAULT value is allowed")
            };
            if slot.is_some() {
                return Err(Diagnostic::new("42601", message).at(at).into());
            }
            *slot = Some(value);
        }
        Ok((path, default))
    }

    fn col_label(&self, handle: Handle<pgtk_sql::tokens::FlatColLabel>) -> Result<String> {
        let pgtk_sql::tokens::FlatColLabel::Text(t) = self.get(handle);
        let span = self.text_span(*t);
        self.fold(self.text(*t), span.start)
    }

    // ----- target list -----

    fn target_item(
        &mut self,
        scope: &Scope,
        item: Handle<FlatSelectItem>,
        out: &mut Vec<TargetEntry>,
    ) -> Result<()> {
        match self.get(item) {
            FlatSelectItem::Star(_) => {
                let location = self.span(item).start;
                let mut entries = self.expand_all_tables(scope, location)?;
                for e in &mut entries {
                    e.location = location;
                }
                self.analysis.star_expansions.insert(item, entries.clone());
                out.extend(entries);
                Ok(())
            }
            FlatSelectItem::Expr(e) => {
                let e = self.get(*e);
                // `t.*` inside an expression item.
                if let FlatExpr::QualRef(q_handle) = self.get(e.expr) {
                    let q_handle = *q_handle;
                    if self.qref_star(q_handle) && e.alias.is_none() {
                        let mut entries = self.expand_qualified_star(scope, q_handle)?;
                        let star_location = self.span(item).start;
                        for e in &mut entries {
                            e.location = star_location;
                        }
                        self.analysis.star_expansions.insert(item, entries.clone());
                        out.extend(entries);
                        return Ok(());
                    }
                }
                // `(SELECT ...).*`: ExpandRowReference of a sublink.
                if e.alias.is_none()
                    && let FlatExpr::SubSelectIndirection(q) = self.get(e.expr)
                    && matches!(
                        self.indirection_parts(self.get(*q).indirection).last(),
                        Some(IndirPart::Star)
                    )
                {
                    let at = self.span(e.expr).start;
                    return Err(unsupported("row expansion of a subquery", Some(at..at)));
                }
                // `a[1].*` and `$1.*`: ExpandIndirectionStar.
                if e.alias.is_none() {
                    let parts = match self.get(e.expr) {
                        FlatExpr::ColumnRef(c) => self.column_ref_parts(self.get(*c).indirection),
                        FlatExpr::PositionalParam(p) => {
                            self.opt_indirection_parts(self.get(*p).indirection)
                        }
                        _ => Vec::new(),
                    };
                    if let Some(IndirPart::Star) = parts.last() {
                        let at = self.span(e.expr).start;
                        return Err(unsupported(
                            "row expansion of a subscripted column or a parameter",
                            Some(at..at),
                        ));
                    }
                }
                // `(expr).*`: ExpandRowReference.
                if e.alias.is_none()
                    && let FlatExpr::Parenthesized(p) = self.get(e.expr)
                {
                    let p = self.get(*p);
                    let parts = self.opt_indirection_parts(p.indirection);
                    if let Some(IndirPart::Star) = parts.last() {
                        let mut entries = self.expand_row_reference(
                            scope,
                            e.expr,
                            p.expr,
                            &parts[..parts.len() - 1],
                        )?;
                        let star_location = self.span(item).start;
                        for e in &mut entries {
                            e.location = star_location;
                        }
                        self.analysis.star_expansions.insert(item, entries.clone());
                        out.extend(entries);
                        return Ok(());
                    }
                }
                let typed = self.expr(scope, e.expr)?;
                let name = match e.alias {
                    Some(alias) => match self.get(alias) {
                        FlatAlias::WithAs(w) => {
                            let w = self.get(*w);
                            match self.get(w.name) {
                                FlatAsAliasName::Plain(name) => self.alias_name(*name)?.0,
                                FlatAsAliasName::Uescaped(u) => {
                                    let u = self.get(*u);
                                    let FlatAliasName::Text(t) = self.get(u.name);
                                    let raw = self.text(*t);
                                    let escape = string_literal_body(
                                        self.text(self.get(u.uescape).escape_char),
                                    );
                                    match pgtk_sql::ident::fold_identifier_with_uescape(
                                        raw, &escape,
                                    ) {
                                        Ok(folded) => folded.name.into_owned(),
                                        Err(error) => {
                                            return Err(unsupported(
                                                format!("identifier {raw:?}: {error}"),
                                                None,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        FlatAlias::Bare(b) => {
                            let pgtk_sql::tokens::literal::FlatSelectBareAliasName::Text(t) =
                                self.get(*b);
                            let span = self.text_span(*t);
                            self.fold(self.text(*t), span.start)?
                        }
                    },
                    None => self
                        .figure_colname(e.expr)
                        .name
                        .unwrap_or_else(|| "?column?".to_owned()),
                };
                let origin = if matches!(typed.form, Form::Var) {
                    self.var_origin(scope, e.expr)
                } else {
                    None
                };
                let fields = if self.types().is_record(typed.ty) {
                    self.record_shape(scope, e.expr)
                } else {
                    None
                };
                out.push(TargetEntry {
                    collation_state: CollContext::new(),
                    collation: None,
                    name,
                    ty: typed.ty,
                    typmod: typed.typmod,
                    origin,
                    junk: false,
                    location: self.span(e.expr).start,
                    fields,
                });
                Ok(())
            }
        }
    }

    /// `markTargetListOrigin`: the table column a target expression is,
    /// looking through parentheses and casts to the column's own type,
    /// which PostgreSQL leaves as the bare `Var`.
    fn var_origin(
        &self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
    ) -> Option<(pgtk_catalog::Handle<pgtk_catalog::Relation>, i16)> {
        if let Some(var) = self.analysis.vars.get(expr) {
            return self.origin_of(scope, var);
        }
        let unchanged = self
            .analysis
            .coercions
            .get(expr)
            .is_none_or(|steps| steps.is_empty());
        match self.get(expr) {
            FlatExpr::Cast(inner, _) if unchanged => {
                let inner = *inner;
                let inner_unchanged = self
                    .analysis
                    .coercions
                    .get(inner)
                    .is_none_or(|s| s.is_empty());
                inner_unchanged
                    .then(|| self.var_origin(scope, inner))
                    .flatten()
            }
            FlatExpr::CastCall(c) if unchanged => {
                let value = self.get(self.get(*c).inner).value;
                let inner_unchanged = self
                    .analysis
                    .coercions
                    .get(value)
                    .is_none_or(|s| s.is_empty());
                inner_unchanged
                    .then(|| self.var_origin(scope, value))
                    .flatten()
            }
            FlatExpr::Parenthesized(p) => {
                let p = self.get(*p);
                if !p.indirection.is_none() {
                    return None;
                }
                self.var_origin(scope, p.expr)
            }
            _ => None,
        }
    }

    /// `ExpandAllTables`.
    fn expand_all_tables(&self, scope: &Scope, location: usize) -> Result<Vec<TargetEntry>> {
        let mut out = Vec::new();
        let mut found = false;
        for item in &scope.namespace {
            if !item.cols_visible {
                continue;
            }
            found = true;
            out.extend(self.expand_ns_item(scope, item, 0));
        }
        if !found {
            return Err(
                Diagnostic::new("42601", "SELECT * with no tables specified is not valid")
                    .at(location)
                    .into(),
            );
        }
        Ok(out)
    }

    /// `transformColumnRef` for `rel.col`: a column of the entry, or a
    /// field or function applied to its whole row.
    fn item_column(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        schema: Option<&str>,
        relname: &str,
        colname: &str,
        location: usize,
    ) -> Result<Typed> {
        let item =
            self.relations
                .refname_namespace_item(scope, &self.outer, schema, relname, location)?;
        let Some((levelsup, item)) = item else {
            return Err(self
                .relations
                .error_missing_rte(scope, &self.outer, schema, relname, location)
                .into());
        };
        let level = self.scope_at(scope, levelsup);
        if let Some(var) = self
            .relations
            .scan_ns_item_for_column(level, &item, levelsup, colname, location)?
        {
            if self.expr_kind == ExprKind::MergeWhen && var.varattno < 0 && var.varattno != -6 {
                return Err(Diagnostic::new(
                    "42P10",
                    format!("cannot use system column \"{colname}\" in MERGE WHEN condition"),
                )
                .at(location)
                .into());
            }
            self.analysis.vars.insert(handle, var);
            if var.levelsup > 0 {
                let outer_level = self.level() - var.levelsup;
                self.outer_vars.push(OuterRef {
                    level: outer_level,
                    kind: self.outer_kinds[outer_level],
                    var: Var { levelsup: 0, ..var },
                    location,
                });
            }
            return Ok(Typed {
                ty: var.ty,
                typmod: var.typmod,
                form: Form::Var,
                location,
                returns_set: false,
            });
        }
        let row = self.whole_row(scope, levelsup, &item, None, location)?;
        self.field_select(
            scope,
            handle,
            &row,
            None,
            Some((levelsup, item)),
            colname,
            location,
        )
    }

    /// `transformWholeRowRef`: the row of an entry as one value. A table's
    /// row has its row type; any other is a `record` whose fields are the
    /// entry's columns.
    fn whole_row(
        &mut self,
        scope: &Scope,
        levelsup: usize,
        item: &NamespaceItem,
        handle: Option<Handle<FlatExpr>>,
        location: usize,
    ) -> Result<Typed> {
        if let Some(h) = handle {
            self.whole_rows.insert(h, (levelsup, item.clone()));
        }
        let level = self.scope_at(scope, levelsup);
        let rte = &level.rtable[item.rt_index - 1];
        let record = self.types().builtin(Builtin::Record);
        let row_type = if item.using_alias.is_some() {
            None
        } else {
            match &rte.kind {
                RteKind::Relation { relation, .. } => self.catalog()[*relation].row_type,
                RteKind::Function { row_type, .. } => *row_type,
                _ => None,
            }
        };
        if let Some(ty) = row_type {
            if let RteKind::Relation { .. } = rte.kind {
                let var = Var {
                    varno: item.rt_index,
                    varattno: 0,
                    levelsup,
                    ty,
                    typmod: -1,
                };
                if let Some(h) = handle {
                    self.analysis.vars.insert(h, var);
                }
                if levelsup > 0 {
                    let outer_level = self.level() - levelsup;
                    self.outer_vars.push(OuterRef {
                        level: outer_level,
                        kind: self.outer_kinds[outer_level],
                        var: Var { levelsup: 0, ..var },
                        location,
                    });
                }
                return Ok(Typed {
                    ty,
                    typmod: -1,
                    form: Form::Var,
                    location,
                    returns_set: false,
                });
            }
            return Ok(Typed {
                ty,
                typmod: -1,
                form: Form::Other,
                location,
                returns_set: false,
            });
        }
        // makeWholeRowVar: a function returning a scalar gives its one
        // column instead of a row.
        if let RteKind::Function { columns, .. } = &rte.kind
            && columns.len() == 1
            && rte.colnames.len() == 1
            && item.using_alias.is_none()
        {
            let var = self.relations.make_var(item.rt_index, rte, 1, levelsup);
            if let Some(h) = handle {
                self.analysis.vars.insert(h, var);
            }
            if levelsup > 0 {
                let outer_level = self.level() - levelsup;
                self.outer_vars.push(OuterRef {
                    level: outer_level,
                    kind: self.outer_kinds[outer_level],
                    var: Var { levelsup: 0, ..var },
                    location,
                });
            }
            return Ok(Typed {
                ty: var.ty,
                typmod: var.typmod,
                form: Form::Var,
                location,
                returns_set: false,
            });
        }
        // A RowExpr of the entry's columns.
        let ncols = item
            .using_alias
            .as_ref()
            .map_or(rte.colnames.len(), |(_, n)| *n);
        let mut fields = Vec::new();
        for (i, name) in rte.colnames[..ncols].iter().enumerate() {
            if name.is_empty() {
                continue;
            }
            let var = self
                .relations
                .make_var(item.rt_index, rte, i as i16 + 1, levelsup);
            fields.push(RecordField {
                name: name.clone(),
                ty: var.ty,
                typmod: var.typmod,
            });
        }
        if let Some(h) = handle {
            self.record_fields.insert(h, fields);
        }
        Ok(Typed {
            ty: record,
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `ParseFuncOrColumn` for `.name` applied to a value: a column of a
    /// whole row, a field of a composite or shaped record, or else a
    /// function taking the value; `unknown_attribute` when none applies.
    #[allow(clippy::too_many_arguments)]
    fn field_select(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        base: &Typed,
        base_handle: Option<Handle<FlatExpr>>,
        base_item: Option<(usize, NamespaceItem)>,
        name: &str,
        location: usize,
    ) -> Result<Typed> {
        let item = base_item.or_else(|| base_handle.and_then(|h| self.whole_rows.get(&h).cloned()));
        // ParseComplexProjection on a whole-row reference: a column.
        if let Some((levelsup, item)) = &item {
            let level = self.scope_at(scope, *levelsup);
            if let Some(var) = self
                .relations
                .scan_ns_item_for_column(level, item, *levelsup, name, location)?
            {
                self.analysis.vars.insert(handle, var);
                if var.levelsup > 0 {
                    let outer_level = self.level() - var.levelsup;
                    self.outer_vars.push(OuterRef {
                        level: outer_level,
                        kind: self.outer_kinds[outer_level],
                        var: Var { levelsup: 0, ..var },
                        location,
                    });
                }
                return Ok(Typed {
                    ty: var.ty,
                    typmod: var.typmod,
                    form: Form::Var,
                    location,
                    returns_set: false,
                });
            }
        } else if let Some(p) = self.projection_field(self.types().base_type(base.ty), name) {
            return Ok(Typed {
                ty: p.ty,
                typmod: p.typmod,
                form: Form::Other,
                location,
                returns_set: false,
            });
        } else if let Some(fields) = base_handle.and_then(|h| self.record_shape(scope, h))
            && let Some(f) = fields.iter().find(|f| f.name == name)
        {
            return Ok(Typed {
                ty: f.ty,
                typmod: f.typmod,
                form: Form::Other,
                location,
                returns_set: false,
            });
        }
        // A function taking the value.
        let fname = ObjectName {
            schema: None,
            name: name.to_owned(),
        };
        let found = self.resolver.func_get_detail(
            &fname,
            std::slice::from_ref(base),
            &[],
            true,
            true,
            false,
            location,
        )?;
        if found.detail == crate::func::FuncDetail::Normal {
            // A value without a node of its own is annotated on the call.
            let base_handle = base_handle.unwrap_or(handle);
            return self.resolve_call(
                handle,
                fname,
                &[base_handle],
                std::slice::from_ref(base),
                location,
                None,
            );
        }
        // unknown_attribute
        let types = self.types();
        let message = if let Some((levelsup, item)) = &item {
            let level = self.scope_at(scope, *levelsup);
            format!(
                "column {}.{name} does not exist",
                self.relations.item_name(level, item)
            )
        } else if types.is_complex(base.ty) {
            return Err(Diagnostic::new(
                "42703",
                format!(
                    "column \"{name}\" not found in data type {}",
                    types.format(base.ty)
                ),
            )
            .at(location)
            .into());
        } else if types.pseudo(base.ty) == Some(&PseudoType::Record) {
            format!("could not identify column \"{name}\" in record data type")
        } else {
            return Err(Diagnostic::new(
                "42809",
                format!(
                    "column notation .{name} applied to type {}, which is not a composite type",
                    types.format(base.ty)
                ),
            )
            .at(location)
            .into());
        };
        Err(Diagnostic::new("42703", message).at(location).into())
    }

    // ----- SQL/JSON -----

    /// A `FORMAT JSON [ENCODING name]` clause: its format kind, position,
    /// and encoding name.
    #[cfg(feature = "since-pg16")]
    fn json_format(
        &self,
        format: Option<Handle<FlatJsonFormat>>,
    ) -> Result<(JsonFormatKind, Option<usize>, Option<String>)> {
        let Some(f) = format else {
            return Ok((JsonFormatKind::Default, None, None));
        };
        self.json_format_of(self.get(f).encoding, self.span(f).start)
    }

    /// The FORMAT clause of a JSON_TABLE context item or column.
    #[cfg(feature = "since-pg17")]
    fn json_table_format(
        &self,
        format: Option<Handle<FlatJsonTableFormat>>,
    ) -> Result<(JsonFormatKind, Option<usize>, Option<String>)> {
        let Some(f) = format else {
            return Ok((JsonFormatKind::Default, None, None));
        };
        self.json_format_of(self.get(f).encoding, self.span(f).start)
    }

    #[cfg(feature = "since-pg16")]
    fn json_format_of(
        &self,
        encoding: Option<Handle<FlatJsonEncoding>>,
        location: usize,
    ) -> Result<(JsonFormatKind, Option<usize>, Option<String>)> {
        let encoding = match encoding {
            None => None,
            Some(e) => {
                let (name, _) = self.col_id(self.get(e).name)?;
                if !matches!(name.as_str(), "utf8" | "utf16" | "utf32") {
                    return Err(Diagnostic::new(
                        "22023",
                        format!("unrecognized JSON encoding: {name}"),
                    )
                    .into());
                }
                Some(name)
            }
        };
        Ok((JsonFormatKind::Json, Some(location), encoding))
    }

    /// `transformJsonValueExpr`: a value with its optional FORMAT clause,
    /// coerced to json or jsonb when the format or `target` asks for it.
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "since-pg16")]
    fn json_value_expr(
        &mut self,
        scope: &Scope,
        construct: &str,
        expr: Handle<FlatExpr>,
        format: Option<Handle<FlatJsonFormat>>,
        default_format: JsonFormatKind,
        target: Option<TypeRef>,
        isarg: bool,
    ) -> Result<Typed> {
        let format = self.json_format(format)?;
        self.json_value_expr_formatted(
            scope,
            construct,
            expr,
            format,
            default_format,
            target,
            isarg,
        )
    }

    /// `transformJsonValueExpr` with the FORMAT clause already read.
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "since-pg16")]
    fn json_value_expr_formatted(
        &mut self,
        scope: &Scope,
        construct: &str,
        expr: Handle<FlatExpr>,
        format: (JsonFormatKind, Option<usize>, Option<String>),
        default_format: JsonFormatKind,
        target: Option<TypeRef>,
        isarg: bool,
    ) -> Result<Typed> {
        let mut t = self.expr(scope, expr)?;
        if self.types().is_unknown(t.ty) {
            let text = self.types().builtin(Builtin::Text);
            self.coerce_to_specific_type(expr, &t, text, construct)?;
            t.ty = text;
            t.typmod = -1;
        }
        let exprtype = t.ty;
        let location = t.location;
        let (given, format_location, encoding) = format;
        let types = self.types();
        let category = types.category(exprtype);
        let is_json = types.is(exprtype, Builtin::Json) || types.is(exprtype, Builtin::Jsonb);
        let is_bytea = types.is(exprtype, Builtin::Bytea);
        let format = if given != JsonFormatKind::Default {
            if encoding.is_some() && !is_bytea {
                return Err(Diagnostic::new(
                    "42804",
                    "JSON ENCODING clause is only allowed for bytea input type",
                )
                .at(format_location.unwrap_or(location))
                .into());
            }
            if is_json {
                JsonFormatKind::Default
            } else {
                given
            }
        } else if isarg {
            let plain = [
                Builtin::Bool,
                Builtin::Numeric,
                Builtin::Int2,
                Builtin::Int4,
                Builtin::Int8,
                Builtin::Float4,
                Builtin::Float8,
                Builtin::Text,
                Builtin::Varchar,
                Builtin::Date,
                Builtin::Time,
                Builtin::Timetz,
                Builtin::Timestamp,
                Builtin::Timestamptz,
            ]
            .into_iter()
            .any(|b| types.is(exprtype, b));
            if plain || category == TypeCategory::String {
                return Ok(t);
            }
            default_format
        } else if is_json {
            JsonFormatKind::Default
        } else {
            default_format
        };
        if format == JsonFormatKind::Default && target.is_none_or(|tt| tt == exprtype) {
            return Ok(t);
        }
        let only_allow_cast = target.is_some();
        if !isarg && !only_allow_cast && !is_bytea && category != TypeCategory::String {
            let message = if given == JsonFormatKind::Default {
                "cannot use non-string types with implicit FORMAT JSON clause"
            } else {
                "cannot use non-string types with explicit FORMAT JSON clause"
            };
            return Err(Diagnostic::new("42804", message)
                .at(format_location.unwrap_or(location))
                .into());
        }
        let mut source = t.clone();
        if format == JsonFormatKind::Json && is_bytea {
            source.ty = types.builtin(Builtin::Text);
        }
        let target = target.unwrap_or_else(|| {
            types.builtin(if format == JsonFormatKind::Jsonb {
                Builtin::Jsonb
            } else {
                Builtin::Json
            })
        });
        let result = Typed {
            ty: target,
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: t.returns_set,
        };
        match self.coercer().coerce_to_target_type(
            &source,
            target,
            -1,
            CoercionContext::Explicit,
        )? {
            Some((_, steps)) => {
                self.record_coercions(expr, steps);
                Ok(result)
            }
            None if only_allow_cast => Err(Diagnostic::new(
                "42846",
                format!(
                    "cannot cast type {} to {}",
                    self.types().format(source.ty),
                    self.types().format(target)
                ),
            )
            .at(location)
            .into()),
            // to_json / to_jsonb takes any type.
            None => Ok(result),
        }
    }

    /// `transformJsonOutput`: a RETURNING clause.
    #[cfg(feature = "since-pg16")]
    fn json_output(
        &mut self,
        output: Option<Handle<FlatJsonReturning>>,
        allow_format: bool,
    ) -> Result<JsonReturning> {
        let Some(o) = output else {
            return Ok(JsonReturning {
                ty: None,
                typmod: -1,
                format: JsonFormatKind::Default,
                format_location: None,
            });
        };
        let o = self.get(o);
        let format = self.json_format(o.format)?;
        self.json_output_of(o.ty, format, allow_format)
    }

    /// `transformJsonOutput` for a type and a FORMAT clause already read.
    #[cfg(feature = "since-pg16")]
    fn json_output_of(
        &mut self,
        type_handle: Handle<FlatCastType>,
        format: (JsonFormatKind, Option<usize>, Option<String>),
        allow_format: bool,
    ) -> Result<JsonReturning> {
        let tn = self.type_name(type_handle)?;
        let (ty, typmod) = self.typenames.resolve(&tn)?;
        if self.types().pseudo(ty).is_some() {
            return Err(Diagnostic::new(
                "0A000",
                "returning pseudo-types is not supported in SQL/JSON functions",
            )
            .into());
        }
        let (given, format_location, encoding) = format;
        let types = self.types();
        let is_jsonb = types.is(ty, Builtin::Jsonb);
        let format = if given == JsonFormatKind::Default {
            if is_jsonb {
                JsonFormatKind::Jsonb
            } else {
                JsonFormatKind::Json
            }
        } else {
            // checkJsonOutputFormat
            let is_bytea = types.is(ty, Builtin::Bytea);
            let stringish = is_bytea || is_jsonb || types.is(ty, Builtin::Json);
            if !allow_format && !stringish && types.category(ty) != TypeCategory::String {
                return Err(Diagnostic::new(
                    "0A000",
                    "cannot use JSON format with non-string output types",
                )
                .at(format_location.unwrap_or(NO_LOCATION))
                .into());
            }
            if given == JsonFormatKind::Json {
                if !is_bytea && encoding.is_some() {
                    return Err(Diagnostic::new(
                        "0A000",
                        "cannot set JSON encoding for non-bytea output types",
                    )
                    .at(format_location.unwrap_or(NO_LOCATION))
                    .into());
                }
                if encoding.as_deref().is_some_and(|e| e != "utf8") {
                    return Err(Diagnostic::new("0A000", "unsupported JSON encoding")
                        .hint("Only UTF8 JSON encoding is supported.")
                        .at(format_location.unwrap_or(NO_LOCATION))
                        .into());
                }
            }
            given
        };
        Ok(JsonReturning {
            ty: Some(ty),
            typmod,
            format,
            format_location,
        })
    }

    /// `transformJsonConstructorOutput`: json unless RETURNING says
    /// otherwise or an argument is jsonb.
    #[cfg(feature = "since-pg16")]
    fn json_constructor_output(
        &mut self,
        output: Option<Handle<FlatJsonReturning>>,
        args: &[Typed],
    ) -> Result<JsonReturning> {
        let mut returning = self.json_output(output, true)?;
        if returning.ty.is_none() {
            let jsonb = self.types().builtin(Builtin::Jsonb);
            if args.iter().any(|a| a.ty == jsonb) {
                returning.ty = Some(jsonb);
                returning.format = JsonFormatKind::Jsonb;
            } else {
                returning.ty = Some(self.types().builtin(Builtin::Json));
                returning.format = JsonFormatKind::Json;
            }
            returning.typmod = -1;
        }
        Ok(returning)
    }

    /// `coerceJsonFuncExpr`: the result type once a json or jsonb value
    /// of type `placeholder` is coerced to the RETURNING type.
    #[cfg(feature = "since-pg16")]
    fn coerce_json_func(
        &self,
        placeholder: TypeRef,
        returning: &JsonReturning,
    ) -> Result<(TypeRef, Typmod)> {
        let Some(ty) = returning.ty else {
            return Ok((placeholder, -1));
        };
        if ty == placeholder {
            return Ok((placeholder, -1));
        }
        if returning.format == JsonFormatKind::Json && self.types().is(ty, Builtin::Bytea) {
            return Ok((ty, -1));
        }
        let node = Typed {
            ty: placeholder,
            typmod: -1,
            form: Form::Other,
            location: NO_LOCATION,
            returns_set: false,
        };
        if self
            .coercer()
            .coerce_to_target_type(&node, ty, returning.typmod, CoercionContext::Assignment)?
            .is_none()
        {
            return Err(Diagnostic::new(
                "42846",
                format!(
                    "cannot cast type {} to {}",
                    self.types().format(placeholder),
                    self.types().format(ty)
                ),
            )
            .at(returning.format_location.unwrap_or(NO_LOCATION))
            .into());
        }
        Ok((ty, returning.typmod))
    }

    /// `makeJsonConstructorExpr` without a function: the RETURNING type
    /// coerced from the format's own type.
    #[cfg(feature = "since-pg16")]
    fn json_constructor_result(&self, returning: &JsonReturning, location: usize) -> Result<Typed> {
        let placeholder = self
            .types()
            .builtin(if returning.format == JsonFormatKind::Jsonb {
                Builtin::Jsonb
            } else {
                Builtin::Json
            });
        let (ty, typmod) = self.coerce_json_func(placeholder, returning)?;
        Ok(Typed {
            ty,
            typmod,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `transformJsonObjectConstructor`; `JSON_OBJECT(a, b)` without
    /// key-value pairs is the plain function.
    #[cfg(feature = "since-pg16")]
    fn json_object(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        o: Handle<FlatJsonObject>,
        location: usize,
    ) -> Result<Typed> {
        let (entries, on_null, unique, output) = match self.get(o) {
            FlatJsonObject::Empty => (Vec::new(), None, None, None),
            FlatJsonObject::Returning(r) => (Vec::new(), None, None, Some(*r)),
            FlatJsonObject::Legacy(l) => {
                let mut handles = Vec::new();
                for arg in self.list1(self.get(*l).args).to_vec() {
                    match self.get(arg) {
                        FlatJsonObjectLegacyArg::Positional(x) => handles.push(*x),
                        FlatJsonObjectLegacyArg::Named(_) => {
                            return Err(unsupported(
                                "a named argument of json_object",
                                Some(location..location),
                            ));
                        }
                    }
                }
                let mut args = Vec::with_capacity(handles.len());
                for &h in &handles {
                    args.push(self.expr(scope, h)?);
                }
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "json_object".to_owned(),
                };
                return self.resolve_call(handle, name, &handles, &args, location, None);
            }
            FlatJsonObject::Entries(a) => {
                let a = self.get(*a);
                (
                    self.list1(a.entries).to_vec(),
                    a.on_null,
                    a.unique,
                    a.returning,
                )
            }
        };
        let _ = (on_null, unique);
        let mut args = Vec::with_capacity(entries.len() * 2);
        for e in entries {
            let (key, value, format) = self.json_key_value(e);
            args.push(self.expr(scope, key)?);
            args.push(self.json_value_expr(
                scope,
                "JSON_OBJECT()",
                value,
                format,
                JsonFormatKind::Default,
                None,
                false,
            )?);
        }
        let returning = self.json_constructor_output(output, &args)?;
        self.json_constructor_result(&returning, location)
    }

    /// `transformJsonArrayConstructor` and
    /// `transformJsonArrayQueryConstructor`.
    #[cfg(feature = "since-pg16")]
    fn json_array(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        a: Handle<FlatJsonArray>,
        location: usize,
    ) -> Result<Typed> {
        let a: &FlatJsonArray = self.get(a);
        match self.get(a.args) {
            FlatJsonArrayArgs::Empty(e) => {
                let returning = self.json_constructor_output(self.get(*e).returning, &[])?;
                self.json_constructor_result(&returning, location)
            }
            FlatJsonArrayArgs::Elements(el) => {
                let el = self.get(*el);
                let mut args = Vec::new();
                for item in self.list1(el.elements).to_vec() {
                    let item = self.get(item);
                    args.push(self.json_value_expr(
                        scope,
                        "JSON_ARRAY()",
                        item.value,
                        item.format,
                        JsonFormatKind::Default,
                        None,
                        false,
                    )?);
                }
                let returning = self.json_constructor_output(el.returning, &args)?;
                self.json_constructor_result(&returning, location)
            }
            FlatJsonArrayArgs::Query(q) => {
                let q = self.get(*q);
                self.sublinks.insert(handle);
                let columns = self.sublink_columns(scope, handle, q.query)?;
                if columns.len() != 1 {
                    return Err(
                        Diagnostic::new("42601", "subquery must return only one column")
                            .at(location)
                            .into(),
                    );
                }
                // JSON_ARRAYAGG over the column: json unless it is jsonb.
                let mut col = Typed {
                    ty: columns[0].ty,
                    typmod: columns[0].typmod,
                    form: Form::Other,
                    location,
                    returns_set: false,
                };
                if self.types().is_unknown(col.ty) {
                    col.ty = self.types().builtin(Builtin::Text);
                }
                let returning = self.json_constructor_output(q.returning, &[col])?;
                self.json_constructor_result(&returning, location)
            }
        }
    }

    /// `transformJsonAggConstructor`: the aggregate `fname` over the
    /// arguments, with FILTER, ORDER BY and OVER as an ordinary aggregate.
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "since-pg16")]
    fn json_agg(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        fname: &str,
        arg_handles: &[Handle<FlatExpr>],
        args: &[Typed],
        order_by: Option<Handle<FlatOrderByClause>>,
        filter: Option<Handle<pgtk_sql::ast::shared::expr::FlatFilterClause>>,
        window: Option<Handle<FlatWindowSpec>>,
        returning: &JsonReturning,
        outer_before: usize,
        location: usize,
    ) -> Result<Typed> {
        if window.is_some() && order_by.is_some() {
            return Err(Diagnostic::new(
                "0A000",
                "aggregate ORDER BY is not implemented for window functions",
            )
            .at(location)
            .into());
        }
        let deco = Decoration {
            filter: filter.is_some(),
            over: window.is_some(),
            order_by: order_by.is_some(),
            ..Decoration::default()
        };
        let site = CallSite::Expr(handle);
        let name = ObjectName {
            schema: Some("pg_catalog".to_owned()),
            name: fname.to_owned(),
        };
        let srf_before = self.last_srf;
        let typed = self.resolve_call_decorated(
            site,
            name,
            arg_handles,
            args,
            &[],
            location,
            None,
            deco,
            srf_before,
        )?;
        if let Some(over) = window {
            self.window_call(
                scope,
                over,
                arg_handles,
                order_by,
                filter,
                deco,
                true,
                location,
            )?;
        } else {
            let level = self.level();
            if self.outer_vars[outer_before..]
                .iter()
                .any(|r| r.level < level)
            {
                return Err(unsupported(
                    "aggregate over an outer-level column",
                    Some(location..location),
                ));
            }
            let kept: Vec<OuterRef> = self.outer_vars[outer_before..]
                .iter()
                .filter(|r| r.level != level)
                .copied()
                .collect();
            self.outer_vars.truncate(outer_before);
            self.outer_vars.extend(kept);
            self.aggregate_clauses(scope, arg_handles, args, order_by, filter, deco)?;
        }
        let (ty, typmod) = self.coerce_json_func(typed.ty, returning)?;
        Ok(Typed {
            ty,
            typmod,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// The key, the value and the value's format of one `json_name_and_value`.
    #[cfg(feature = "since-pg16")]
    fn json_key_value(
        &self,
        entry: Handle<FlatJsonKeyValue>,
    ) -> (
        Handle<FlatExpr>,
        Handle<FlatExpr>,
        Option<Handle<FlatJsonFormat>>,
    ) {
        match self.get(entry) {
            FlatJsonKeyValue::Value(v) => {
                let v = self.get(*v);
                (v.key, v.value, v.format)
            }
            FlatJsonKeyValue::Colon(c) => {
                let c = self.get(*c);
                (c.key, c.value, c.format)
            }
        }
    }

    /// `transformJsonObjectAgg`.
    #[cfg(feature = "since-pg16")]
    fn json_object_agg(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        a: Handle<FlatJsonObjectAgg>,
        location: usize,
    ) -> Result<Typed> {
        let a: &FlatJsonObjectAgg = self.get(a);
        let inner = self.get(a.inner);
        let (key_handle, value_handle, format) = self.json_key_value(inner.entry);
        let outer_before = self.outer_vars.len();
        let key = self.expr(scope, key_handle)?;
        let val = self.json_value_expr(
            scope,
            "JSON_OBJECTAGG()",
            value_handle,
            format,
            JsonFormatKind::Default,
            None,
            false,
        )?;
        let returning =
            self.json_constructor_output(inner.returning, &[key.clone(), val.clone()])?;
        let absent = inner
            .on_null
            .is_some_and(|n| matches!(self.get(self.get(n).which), FlatNullOrAbsent::Absent));
        let unique = inner.unique.is_some_and(|u| {
            matches!(
                self.get(self.get(u).with_or_without),
                FlatWithOrWithout::With
            )
        });
        let fname = match (returning.format == JsonFormatKind::Jsonb, absent, unique) {
            (true, true, true) => "jsonb_object_agg_unique_strict",
            (true, true, false) => "jsonb_object_agg_strict",
            (true, false, true) => "jsonb_object_agg_unique",
            (true, false, false) => "jsonb_object_agg",
            (false, true, true) => "json_object_agg_unique_strict",
            (false, true, false) => "json_object_agg_strict",
            (false, false, true) => "json_object_agg_unique",
            (false, false, false) => "json_object_agg",
        };
        self.json_agg(
            scope,
            handle,
            fname,
            &[key_handle, value_handle],
            &[key, val],
            None,
            a.filter,
            a.window,
            &returning,
            outer_before,
            location,
        )
    }

    /// `transformJsonArrayAgg`.
    #[cfg(feature = "since-pg16")]
    fn json_array_agg(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        a: Handle<FlatJsonArrayAgg>,
        location: usize,
    ) -> Result<Typed> {
        let a: &FlatJsonArrayAgg = self.get(a);
        let inner = self.get(a.inner);
        let outer_before = self.outer_vars.len();
        let val = self.json_value_expr(
            scope,
            "JSON_ARRAYAGG()",
            inner.value,
            inner.format,
            JsonFormatKind::Default,
            None,
            false,
        )?;
        let returning =
            self.json_constructor_output(inner.returning, std::slice::from_ref(&val))?;
        let absent = inner
            .on_null
            .is_some_and(|n| matches!(self.get(self.get(n).which), FlatNullOrAbsent::Absent));
        let fname = match (returning.format == JsonFormatKind::Jsonb, absent) {
            (true, true) => "jsonb_agg_strict",
            (true, false) => "jsonb_agg",
            (false, true) => "json_agg_strict",
            (false, false) => "json_agg",
        };
        self.json_agg(
            scope,
            handle,
            fname,
            &[inner.value],
            &[val],
            inner.order_by,
            a.filter,
            a.window,
            &returning,
            outer_before,
            location,
        )
    }

    /// `transformJsonParseArg`: the type an IS JSON or WITH UNIQUE KEYS
    /// argument has once bytea and string types become text.
    #[cfg(feature = "since-pg16")]
    fn json_parse_arg(
        &mut self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
        format: Option<Handle<FlatJsonFormat>>,
    ) -> Result<(TypeRef, Typed)> {
        let t = self.expr(scope, expr)?;
        let types = self.types();
        let text = types.builtin(Builtin::Text);
        let (_, format_location, encoding) = self.json_format(format)?;
        if self.types().is(t.ty, Builtin::Bytea) {
            return Ok((text, t));
        }
        if encoding.is_some() {
            return Err(Diagnostic::new(
                "0A000",
                "cannot use JSON FORMAT ENCODING clause for non-bytea input types",
            )
            .at(format_location.unwrap_or(t.location))
            .into());
        }
        if self.types().is_unknown(t.ty) || self.types().category(t.ty) == TypeCategory::String {
            if let Some((_, steps)) =
                self.coercer()
                    .coerce_to_target_type(&t, text, -1, CoercionContext::Implicit)?
            {
                self.record_coercions(expr, steps);
            }
            return Ok((text, t));
        }
        Ok((t.ty, t))
    }

    /// `transformJsonParseExpr`: `JSON(value)`.
    #[cfg(feature = "since-pg17")]
    fn json_parse(
        &mut self,
        scope: &Scope,
        c: Handle<FlatJsonConstructor>,
        location: usize,
    ) -> Result<Typed> {
        let c: &FlatJsonConstructor = self.get(c);
        let inner = self.get(c.inner);
        let json = self.types().builtin(Builtin::Json);
        let unique = inner.unique.is_some_and(|u| {
            matches!(
                self.get(self.get(u).with_or_without),
                FlatWithOrWithout::With
            )
        });
        if unique {
            let (ty, _) = self.json_parse_arg(scope, inner.value, inner.format)?;
            if !self.types().is(ty, Builtin::Text) {
                return Err(Diagnostic::new(
                    "42804",
                    "cannot use non-string types with WITH UNIQUE KEYS clause",
                )
                .at(location)
                .into());
            }
        } else {
            self.json_value_expr(
                scope,
                "JSON()",
                inner.value,
                inner.format,
                JsonFormatKind::Json,
                Some(json),
                false,
            )?;
        }
        Ok(Typed {
            ty: json,
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `transformJsonSerializeExpr`.
    #[cfg(feature = "since-pg17")]
    fn json_serialize(
        &mut self,
        scope: &Scope,
        j: Handle<FlatJsonSerialize>,
        location: usize,
    ) -> Result<Typed> {
        let j: &FlatJsonSerialize = self.get(j);
        let inner = self.get(j.inner);
        self.json_value_expr(
            scope,
            "JSON_SERIALIZE()",
            inner.value,
            inner.format,
            JsonFormatKind::Json,
            None,
            false,
        )?;
        let returning = match inner.returning {
            Some(r) => {
                let returning = self.json_output(Some(r), true)?;
                let ty = returning.ty.expect("a RETURNING type");
                if !self.types().is(ty, Builtin::Bytea)
                    && self.types().category(ty) != TypeCategory::String
                {
                    return Err(Diagnostic::new(
                        "42804",
                        format!(
                            "cannot use type {} in RETURNING clause of JSON_SERIALIZE()",
                            self.types().format(ty)
                        ),
                    )
                    .hint("Try returning a string type or bytea.")
                    .into());
                }
                returning
            }
            None => JsonReturning {
                ty: Some(self.types().builtin(Builtin::Text)),
                typmod: -1,
                format: JsonFormatKind::Json,
                format_location: None,
            },
        };
        let json = self.types().builtin(Builtin::Json);
        let (ty, typmod) = self.coerce_json_func(json, &returning)?;
        Ok(Typed {
            ty,
            typmod,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `ValidJsonBehaviorDefaultExpr`: a constant, a plain function or
    /// operator call, or a coercion of one of those.
    #[cfg(feature = "since-pg17")]
    fn valid_json_default(&self, e: Handle<FlatExpr>) -> bool {
        let e = self.unparen(e);
        match self.get(e) {
            FlatExpr::IntegerLit(_)
            | FlatExpr::NumericLit(_)
            | FlatExpr::StringLit(_)
            | FlatExpr::BoolTrue
            | FlatExpr::BoolFalse
            | FlatExpr::Null
            | FlatExpr::CastFunc(_)
            | FlatExpr::PositionalParam(_) => true,
            FlatExpr::Cast(inner, _) => self.valid_json_default(*inner),
            FlatExpr::CastCall(c) => self.valid_json_default(self.get(self.get(*c).inner).value),
            FlatExpr::Collate(inner, _) => self.valid_json_default(*inner),
            // doNegate folds the sign into the constant.
            FlatExpr::Neg(inner) if self.negatable_literal(*inner).is_some() => true,
            FlatExpr::Parenthesized(_) | FlatExpr::ImplicitRow(_) => false,
            node if matches!(fixed_operands(node), Some((None, r)) if self.negatable_literal(r).is_some()) => {
                true
            }
            _ => {
                if let Some(f) = self.analysis.functions.get(e) {
                    return !f.window && self.catalog()[f.function].aggregate.is_none();
                }
                self.analysis.operators.get(e).is_some()
            }
        }
    }

    /// The ON EMPTY and the ON ERROR behaviors of a clause, each with the
    /// start of its clause.
    #[cfg(feature = "since-pg17")]
    #[allow(clippy::type_complexity)]
    fn json_behaviors(
        &self,
        clause: Option<Handle<FlatJsonBehaviorClause>>,
    ) -> (
        Option<(Handle<FlatJsonBehavior>, usize)>,
        Option<(Handle<FlatJsonBehavior>, usize)>,
    ) {
        match clause.map(|c| self.get(c)) {
            None => (None, None),
            Some(FlatJsonBehaviorClause::Error(e)) => (None, Some(self.json_on_error(*e))),
            Some(FlatJsonBehaviorClause::Empty(c)) => {
                let c = self.get(*c);
                let on_empty = (self.get(c.on_empty).behavior, self.span(c.on_empty).start);
                (Some(on_empty), c.on_error.map(|e| self.json_on_error(e)))
            }
        }
    }

    /// The behavior of an ON ERROR clause, with the start of the clause.
    #[cfg(feature = "since-pg17")]
    fn json_on_error(
        &self,
        e: Handle<pgtk_sql::ast::shared::expr::FlatJsonOnError>,
    ) -> (Handle<FlatJsonBehavior>, usize) {
        (self.get(e).behavior, self.span(e).start)
    }

    /// `transformJsonFuncExpr`: JSON_EXISTS, JSON_QUERY and JSON_VALUE.
    #[cfg(feature = "since-pg17")]
    fn json_func(
        &mut self,
        scope: &Scope,
        op: JsonOp,
        parts: &JsonFuncParts,
        location: usize,
    ) -> Result<Typed> {
        let (func_name, default_format) = match op {
            JsonOp::Exists => ("JSON_EXISTS", JsonFormatKind::Default),
            JsonOp::Query => ("JSON_QUERY", JsonFormatKind::Jsonb),
            JsonOp::Value => ("JSON_VALUE", JsonFormatKind::Default),
        };
        if op != JsonOp::Query
            && let JsonReturningSpec::Clause(r) = parts.returning
            && let Some(f) = self.get(r).format
        {
            return Err(Diagnostic::new(
                "42601",
                format!("cannot specify FORMAT JSON in RETURNING clause of {func_name}()"),
            )
            .at(self.span(f).start)
            .into());
        }
        if op == JsonOp::Query {
            let omit = parts.quotes.is_some_and(|q| {
                matches!(self.get(self.get(q).keep_or_omit), FlatKeepOrOmit::Omit)
            });
            let wrapped = parts
                .wrapper
                .is_some_and(|w| matches!(self.get(w), FlatJsonWrapper::With(_)));
            if omit && wrapped {
                return Err(Diagnostic::new(
                    "42601",
                    "SQL/JSON QUOTES behavior must not be specified when WITH WRAPPER is used",
                )
                .at(location)
                .into());
            }
        }
        // The behaviors each function accepts.
        let (on_empty, on_error) = (parts.on_empty, parts.on_error);
        let check = |w: &Self,
                     b: Option<(Handle<FlatJsonBehavior>, usize)>,
                     which: &str|
         -> Result<()> {
            let Some((b, start)) = b else {
                return Ok(());
            };
            let behavior = w.get(b);
            let allowed = match op {
                JsonOp::Query => matches!(
                    behavior,
                    FlatJsonBehavior::Error
                        | FlatJsonBehavior::Null
                        | FlatJsonBehavior::Empty
                        | FlatJsonBehavior::EmptyArray
                        | FlatJsonBehavior::EmptyObject
                        | FlatJsonBehavior::Default(_)
                ),
                JsonOp::Exists => matches!(
                    behavior,
                    FlatJsonBehavior::Error
                        | FlatJsonBehavior::True
                        | FlatJsonBehavior::False
                        | FlatJsonBehavior::Unknown
                ),
                JsonOp::Value => matches!(
                    behavior,
                    FlatJsonBehavior::Error | FlatJsonBehavior::Null | FlatJsonBehavior::Default(_)
                ),
            };
            if allowed {
                return Ok(());
            }
            // A JSON_TABLE column names itself instead of the function.
            let (what, message) = match &parts.column {
                Some(column) => (
                    match op {
                        JsonOp::Query => "formatted columns".to_owned(),
                        JsonOp::Exists => "EXISTS columns".to_owned(),
                        JsonOp::Value => "scalar columns".to_owned(),
                    },
                    format!("invalid {which} behavior for column \"{column}\""),
                ),
                None => (
                    format!("{func_name}()"),
                    format!("invalid {which} behavior"),
                ),
            };
            let detail = match op {
                JsonOp::Query => format!(
                    "Only ERROR, NULL, EMPTY ARRAY, EMPTY OBJECT, or DEFAULT expression is allowed in {which} for {what}."
                ),
                JsonOp::Exists => {
                    format!("Only ERROR, TRUE, FALSE, or UNKNOWN is allowed in {which} for {what}.")
                }
                JsonOp::Value => {
                    format!(
                        "Only ERROR, NULL, or DEFAULT expression is allowed in {which} for {what}."
                    )
                }
            };
            Err(Diagnostic::new("42601", message)
                .detail(detail)
                .at(start)
                .into())
        };
        match op {
            JsonOp::Query | JsonOp::Value => {
                check(self, on_empty, "ON EMPTY")?;
                check(self, on_error, "ON ERROR")?;
            }
            JsonOp::Exists => check(self, on_error, "ON ERROR")?,
        }
        let jsonb = self.types().builtin(Builtin::Jsonb);
        if let JsonContext::Expr(context, context_format) = parts.context {
            self.json_value_expr(
                scope,
                func_name,
                context,
                context_format,
                default_format,
                Some(jsonb),
                false,
            )?;
        }
        if let Some(path_expr) = parts.path {
            let path = self.expr(scope, path_expr)?;
            let jsonpath = self.types().builtin(Builtin::Jsonpath);
            match self.coercer().coerce_to_target_type(
                &path,
                jsonpath,
                -1,
                CoercionContext::Explicit,
            )? {
                Some((_, steps)) => self.record_coercions(path_expr, steps),
                None => {
                    return Err(Diagnostic::new(
                        "42804",
                        format!(
                            "JSON path expression must be of type jsonpath, not of type {}",
                            self.types().format(path.ty)
                        ),
                    )
                    .at(path.location)
                    .into());
                }
            }
        }
        if let Some(p) = parts.passing {
            for arg in self.list1(self.get(p).args).to_vec() {
                let arg = self.get(arg);
                self.json_value_expr(
                    scope,
                    func_name,
                    arg.value,
                    None,
                    JsonFormatKind::Jsonb,
                    None,
                    true,
                )?;
            }
        }
        let mut returning = match parts.returning {
            JsonReturningSpec::None => self.json_output(None, false)?,
            JsonReturningSpec::Clause(r) => self.json_output(Some(r), false)?,
            JsonReturningSpec::Column(ty, format) => {
                let format = self.json_table_format(format)?;
                self.json_output_of(ty, format, false)?
            }
        };
        if returning.ty.is_none() {
            returning.ty = Some(self.types().builtin(match op {
                JsonOp::Exists => Builtin::Bool,
                JsonOp::Query => Builtin::Jsonb,
                JsonOp::Value => Builtin::Text,
            }));
            returning.typmod = -1;
        }
        let ret_ty = returning.ty.expect("a result type");
        // transformJsonBehavior for DEFAULT expressions.
        for (b, _) in parts.on_empty.into_iter().chain(parts.on_error) {
            let FlatJsonBehavior::Default(d) = self.get(b) else {
                continue;
            };
            let e = self.get(*d).value;
            let t = self.expr(scope, e)?;
            if !self.valid_json_default(e) {
                return Err(Diagnostic::new(
                    "42804",
                    "can only specify a constant, non-aggregate function, or operator expression for DEFAULT",
                )
                .at(t.location)
                .into());
            }
            if self.contains_var(e) {
                return Err(Diagnostic::new(
                    "42804",
                    "DEFAULT expression must not contain column references",
                )
                .at(t.location)
                .into());
            }
            if t.returns_set {
                return Err(
                    Diagnostic::new("42804", "DEFAULT expression must not return a set")
                        .at(t.location)
                        .into(),
                );
            }
            let exprcoll = self
                .assign_expr_collations(scope, e)?
                .or_else(|| self.type_collation(t.ty));
            if let (Some(tc), Some(ec)) = (self.type_collation(ret_ty), exprcoll)
                && tc != ec
            {
                return Err(Diagnostic::new(
                    "42P21",
                    "collation of DEFAULT expression conflicts with RETURNING clause",
                )
                .detail(format!(
                    "\"{}\" versus \"{}\"",
                    self.collation_name(ec),
                    self.collation_name(tc)
                ))
                .at(t.location)
                .into());
            }
            if t.ty == ret_ty {
                continue;
            }
            let types = self.types();
            let is_null = matches!(t.form, Form::Const { is_null: true, .. });
            let int4 = types.builtin(Builtin::Int4);
            if is_null
                || types.is(t.ty, Builtin::Jsonb)
                || (types.is(t.ty, Builtin::Bool) && types.base_type(ret_ty) != int4)
            {
                continue;
            }
            let category = types.category(ret_ty);
            let context = if matches!(category, TypeCategory::String | TypeCategory::BitString) {
                CoercionContext::Assignment
            } else {
                CoercionContext::Explicit
            };
            match self
                .coercer()
                .coerce_to_target_type(&t, ret_ty, returning.typmod, context)?
            {
                Some((_, steps)) => self.record_coercions(e, steps),
                None => {
                    return Err(Diagnostic::new(
                        "42846",
                        format!(
                            "cannot cast behavior expression of type {} to {}",
                            self.types().format(t.ty),
                            self.types().format(ret_ty)
                        ),
                    )
                    .hint(format!(
                        "You will need to explicitly cast the expression to type {}.",
                        self.types().format(ret_ty)
                    ))
                    .at(t.location)
                    .into());
                }
            }
        }
        Ok(Typed {
            ty: ret_ty,
            typmod: returning.typmod,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `get_expr_result_tupdesc` for a `record`-typed expression: the
    /// fields a row constructor, a whole-row reference, a column that
    /// carries a known row, or a function with OUT parameters gives it.
    fn record_shape(&self, scope: &Scope, h: Handle<FlatExpr>) -> Option<Vec<RecordField>> {
        if let Some(fields) = self.record_fields.get(&h) {
            return Some(fields.clone());
        }
        if let Some((levelsup, item)) = self.whole_rows.get(&h) {
            let level = self.scope_at(scope, *levelsup);
            return Some(
                self.expand_ns_item(level, item, *levelsup)
                    .into_iter()
                    .map(|e| RecordField {
                        name: e.name,
                        ty: e.ty,
                        typmod: e.typmod,
                    })
                    .collect(),
            );
        }
        if let Some(var) = self.analysis.vars.get(h)
            && var.varattno > 0
        {
            let level = self.scope_at(scope, var.levelsup);
            let index = usize::try_from(var.varattno - 1).expect("attnum");
            return match &level.rtable[var.varno - 1].kind {
                RteKind::Subquery { columns, .. } => {
                    columns.get(index).and_then(|c| c.fields.clone())
                }
                RteKind::Cte { columns, .. } => columns.get(index).and_then(|c| c.fields.clone()),
                _ => None,
            };
        }
        if let Some(resolution) = self.analysis.functions.get(h) {
            // build_function_result_tupdesc_t: two or more OUT parameters.
            let proc = &self.catalog()[resolution.function];
            let outs: Vec<RecordField> = proc
                .arguments
                .iter()
                .filter(|a| matches!(a.mode, ArgMode::Out | ArgMode::InOut | ArgMode::Table))
                .map(|a| RecordField {
                    name: a.name.as_ref().map(ToString::to_string).unwrap_or_default(),
                    ty: a.ty,
                    typmod: -1,
                })
                .collect();
            if outs.len() >= 2 {
                return Some(outs);
            }
        }
        if let FlatExpr::Parenthesized(p) = self.get(h) {
            let p = self.get(*p);
            if p.indirection.is_none() {
                return self.record_shape(scope, p.expr);
            }
        }
        None
    }

    /// The values of an operand of `OVERLAPS`, gram.y `row`.
    fn overlaps_row_values(
        &self,
        row: Handle<pgtk_sql::ast::shared::expr::FlatRow>,
    ) -> Vec<Handle<FlatExpr>> {
        match self.get(row) {
            pgtk_sql::ast::shared::expr::FlatRow::Explicit(r) => self
                .get(*r)
                .values
                .map_or_else(Vec::new, |v| self.list1(v).to_vec()),
            pgtk_sql::ast::shared::expr::FlatRow::Implicit(r) => self.implicit_row_values(*r),
        }
    }

    /// The values of an implicit row, `(a, b, ...)`: the first value and the
    /// rest, in order.
    fn implicit_row_values(
        &self,
        row: Handle<pgtk_sql::ast::shared::expr::FlatImplicitRow>,
    ) -> Vec<Handle<FlatExpr>> {
        let row = self.get(row);
        let mut values = vec![row.first];
        values.extend_from_slice(self.list1(row.rest));
        values
    }

    /// `ExpandRowReference` for `(expr).*` in a target list: one entry per
    /// column of the row.
    fn expand_row_reference(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        inner: Handle<FlatExpr>,
        parts: &[IndirPart],
    ) -> Result<Vec<TargetEntry>> {
        let location = self.span(handle).start;
        let (base, base_handle) = (self.expr(scope, inner)?, Some(inner));
        let (base, base_handle) = if parts.is_empty() {
            (base, base_handle)
        } else {
            (
                self.indirection(scope, handle, base, base_handle, parts, location)?,
                None,
            )
        };
        // A whole-row reference expands to the columns themselves.
        if let Some(h) = base_handle
            && let Some((levelsup, item)) = self.whole_rows.get(&h).cloned()
        {
            let level = self.scope_at(scope, levelsup);
            return Ok(self.expand_ns_item(level, &item, levelsup));
        }
        let ty = self.types().base_type(base.ty);
        let fields: Vec<RecordField> = if let TypeKind::Composite(detail) = &self.catalog()[ty].kind
        {
            detail
                .fields
                .iter()
                .map(|f| RecordField {
                    name: f.name.to_string(),
                    ty: f.ty,
                    typmod: f.typmod.0,
                })
                .collect()
        } else if self.types().is_record(ty) {
            match base_handle.and_then(|h| self.record_shape(scope, h)) {
                Some(fields) => fields,
                None => {
                    return Err(unsupported(
                        "row expansion of a record of unknown shape",
                        Some(location..location),
                    ));
                }
            }
        } else {
            return Err(Diagnostic::new(
                "42809",
                format!("type {} is not composite", self.types().format(base.ty)),
            )
            .at(location)
            .into());
        };
        Ok(fields
            .into_iter()
            .map(|f| TargetEntry {
                collation_state: CollContext::new(),
                collation: None,
                name: f.name,
                ty: f.ty,
                typmod: f.typmod,
                origin: None,
                junk: false,
                location,
                fields: None,
            })
            .collect())
    }

    /// An `xml` value.
    fn xml_result(&self, location: usize) -> Typed {
        Typed {
            ty: self.types().builtin(Builtin::Xml),
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: false,
        }
    }

    /// One named argument of XMLELEMENT's attributes or of XMLFOREST: the
    /// value, and its name from the alias or the column reference.
    fn xml_named_arg(
        &mut self,
        scope: &Scope,
        arg: Handle<FlatXmlNamedArg>,
        attribute: bool,
    ) -> Result<String> {
        let a = self.get(arg);
        self.expr(scope, a.value)?;
        if let Some(alias) = a.alias {
            return Ok(self.alias_name(self.get(alias).name)?.0);
        }
        let is_column = match self.get(a.value) {
            FlatExpr::ColumnRef(_) => true,
            FlatExpr::QualRef(q) => !self.qref_function(*q),
            _ => false,
        };
        if !is_column {
            let message = if attribute {
                "unnamed XML attribute value must be a column reference"
            } else {
                "unnamed XML element value must be a column reference"
            };
            return Err(Diagnostic::new("42601", message)
                .at(self.span(a.value).start)
                .into());
        }
        Ok(self.figure_colname(a.value).name.unwrap_or_default())
    }

    /// `ExpandColumnRefStar` for a `t.*` written as an expression list
    /// item: the entry's columns, or `None` for any other expression.
    fn star_columns(&self, scope: &Scope, e: Handle<FlatExpr>) -> Result<Option<Vec<TargetEntry>>> {
        let FlatExpr::QualRef(q_handle) = self.get(e) else {
            return Ok(None);
        };
        if !self.qref_star(*q_handle) {
            return Ok(None);
        }
        Ok(Some(self.expand_qualified_star(scope, *q_handle)?))
    }

    /// `transformRowExpr`: a `record` whose fields are `f1`, `f2`, ...;
    /// a `t.*` element is the entry's columns.
    fn row_expr(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        elements: &[Handle<FlatExpr>],
        location: usize,
    ) -> Result<Typed> {
        let mut fields = Vec::with_capacity(elements.len());
        for &e in elements {
            if let Some(entries) = self.star_columns(scope, e)? {
                for entry in entries {
                    fields.push(RecordField {
                        name: format!("f{}", fields.len() + 1),
                        ty: entry.ty,
                        typmod: entry.typmod,
                    });
                }
                continue;
            }
            let t = self.expr(scope, e)?;
            self.element_typed.insert(e, t.clone());
            fields.push(RecordField {
                name: format!("f{}", fields.len() + 1),
                ty: t.ty,
                typmod: t.typmod,
            });
        }
        self.record_fields.insert(handle, fields);
        Ok(Typed {
            ty: self.types().builtin(Builtin::Record),
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// The elements of a row constructor: `ROW(...)`, or a parenthesised
    /// list of two or more expressions.
    fn row_elements(&self, e: Handle<FlatExpr>) -> Option<Vec<Handle<FlatExpr>>> {
        match self.get(e) {
            FlatExpr::RowExpr(r) => Some(
                self.get(*r)
                    .values
                    .map_or_else(Vec::new, |v| self.list1(v).to_vec()),
            ),
            FlatExpr::ImplicitRow(row) => Some(self.implicit_row_values(*row)),
            _ => None,
        }
    }

    /// `make_row_comparison_op` for two row constructors: the operator
    /// applied to each pair of elements must yield boolean.
    #[allow(clippy::too_many_arguments)]
    fn row_comparison(
        &mut self,
        scope: &Scope,
        name: &ObjectName,
        left: &[Handle<FlatExpr>],
        right: &[Handle<FlatExpr>],
        location: usize,
    ) -> Result<Typed> {
        if left.len() != right.len() {
            return Err(
                Diagnostic::new("42601", "unequal number of entries in row expressions")
                    .at(location)
                    .into(),
            );
        }
        if left.is_empty() {
            return Err(
                Diagnostic::new("0A000", "cannot compare rows of zero length")
                    .at(location)
                    .into(),
            );
        }
        let bool_type = self.types().builtin(Builtin::Bool);
        for (&l, &r) in left.iter().zip(right) {
            let lt = match self.element_typed.get(&l) {
                Some(t) => t.clone(),
                None => self.expr(scope, l)?,
            };
            let rt = match self.element_typed.get(&r) {
                Some(t) => t.clone(),
                None => self.expr(scope, r)?,
            };
            let resolved = self.resolver.make_op(name, Some(&lt), &rt, location)?;
            if resolved.result.ty != bool_type {
                return Err(Diagnostic::new(
                    "42804",
                    format!(
                        "row comparison operator must yield type boolean, not type {}",
                        self.types().format(resolved.result.ty)
                    ),
                )
                .at(location)
                .into());
            }
            if resolved.result.returns_set {
                return Err(Diagnostic::new(
                    "42804",
                    "row comparison operator must not return a set",
                )
                .at(location)
                .into());
            }
            self.record_coercions(l, resolved.left_coercions);
            self.record_coercions(r, resolved.right_coercions);
        }
        Ok(self.bool_result(location))
    }

    /// `transformContainerSubscripts`: subscripts applied to an array,
    /// a vector or a jsonb value.
    fn subscripts(
        &mut self,
        scope: &Scope,
        base: &Typed,
        subs: &[Handle<FlatBracketSubscript>],
        location: usize,
    ) -> Result<Typed> {
        let _ = location;
        // transformContainerType looks through domains; the types with
        // raw_array_subscript_handler have a fixed element type.
        let (container, container_typmod) = self.types().base_type_and_typmod(base.ty, base.typmod);
        let raw_element = [
            (Builtin::Int2vector, Builtin::Int2),
            (Builtin::Oidvector, Builtin::Oid),
            (Builtin::Name, Builtin::Char),
            (Builtin::Point, Builtin::Float8),
            (Builtin::Box, Builtin::Point),
        ]
        .into_iter()
        .find(|(raw, _)| self.types().is(container, *raw))
        .map(|(_, element)| self.types().builtin(element));
        let mut is_slice = false;
        let mut indexes: Vec<SubscriptBounds> = Vec::new();
        for sub in subs {
            let sub = self.get(*sub);
            match self.get(sub.content) {
                FlatBracketSubscriptValue::LowerUnbounded(slice) => {
                    is_slice = true;
                    indexes.push((None, self.get(*slice).upper));
                }
                FlatBracketSubscriptValue::Bounded(b) => {
                    let b = self.get(*b);
                    match b.slice {
                        Some(sl) => {
                            is_slice = true;
                            indexes.push((Some(b.lower), self.get(sl).upper));
                        }
                        None => indexes.push((None, Some(b.lower))),
                    }
                }
            }
        }
        if self
            .types()
            .is(self.types().base_type(container), Builtin::Jsonb)
        {
            let int4 = self.types().builtin(Builtin::Int4);
            let text = self.types().builtin(Builtin::Text);
            for (lower, upper) in &indexes {
                if is_slice {
                    let at = upper.or(*lower).map_or(NO_LOCATION, |e| self.span(e).start);
                    return Err(Diagnostic::new(
                        "42804",
                        "jsonb subscript does not support slices",
                    )
                    .at(at)
                    .into());
                }
                let Some(e) = upper else { continue };
                let t = self.expr(scope, *e)?;
                let target = if self.types().is_unknown(t.ty) {
                    text
                } else {
                    let to_int =
                        self.coercer()
                            .can_coerce_type(&[t.ty], &[int4], CoercionContext::Implicit);
                    let to_text =
                        self.coercer()
                            .can_coerce_type(&[t.ty], &[text], CoercionContext::Implicit);
                    match (to_int, to_text) {
                        (true, true) => {
                            return Err(Diagnostic::new(
                                "42804",
                                format!("subscript type {} is not supported", self.types().format(t.ty)),
                            )
                            .hint("jsonb subscript must be coercible to only one type, integer or text.")
                            .at(t.location)
                            .into());
                        }
                        (false, false) => {
                            return Err(Diagnostic::new(
                                "42804",
                                format!(
                                    "subscript type {} is not supported",
                                    self.types().format(t.ty)
                                ),
                            )
                            .hint("jsonb subscript must be coercible to either integer or text.")
                            .at(t.location)
                            .into());
                        }
                        (true, false) => int4,
                        (false, true) => text,
                    }
                };
                if t.ty != target {
                    let (_, steps) =
                        self.coercer()
                            .coerce_type(&t, target, -1, CoercionContext::Implicit)?;
                    self.record_coercions(*e, steps);
                }
            }
            return Ok(Typed {
                ty: self.types().builtin(Builtin::Jsonb),
                typmod: -1,
                form: Form::Other,
                location: base.location,
                returns_set: false,
            });
        }
        let Some(element) = raw_element.or_else(|| self.types().element_type(container)) else {
            return Err(Diagnostic::new(
                "42804",
                format!(
                    "cannot subscript type {} because it does not support subscripting",
                    self.types().format(base.ty)
                ),
            )
            .at(base.location)
            .into());
        };
        if indexes.len() > 6 {
            return Err(Diagnostic::new(
                "54000",
                format!(
                    "number of array dimensions ({}) exceeds the maximum allowed (6)",
                    indexes.len()
                ),
            )
            .into());
        }
        let int4 = self.types().builtin(Builtin::Int4);
        for (lower, upper) in &indexes {
            for e in [lower, upper].into_iter().flatten() {
                let t = self.expr(scope, *e)?;
                match self.coercer().coerce_to_target_type(
                    &t,
                    int4,
                    -1,
                    CoercionContext::Assignment,
                )? {
                    Some((_, steps)) => self.record_coercions(*e, steps),
                    None => {
                        return Err(Diagnostic::new(
                            "42804",
                            "array subscript must have type integer",
                        )
                        .at(t.location)
                        .into());
                    }
                }
            }
        }
        Ok(Typed {
            ty: if is_slice { container } else { element },
            typmod: container_typmod,
            form: Form::Other,
            location: base.location,
            returns_set: false,
        })
    }

    /// `transformIndirection` over the parts after a base value: subscripts
    /// group until a field name applies to their result.
    fn indirection(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        mut base: Typed,
        mut base_handle: Option<Handle<FlatExpr>>,
        parts: &[IndirPart],
        location: usize,
    ) -> Result<Typed> {
        let mut subs: Vec<Handle<FlatBracketSubscript>> = Vec::new();
        for part in parts {
            match *part {
                IndirPart::Subscript(b) => subs.push(b),
                IndirPart::Star => {
                    return Err(Diagnostic::new(
                        "0A000",
                        "row expansion via \"*\" is not supported here",
                    )
                    .at(base.location)
                    .into());
                }
                IndirPart::Field(f) => {
                    if !subs.is_empty() {
                        base = self.subscripts(scope, &base, &subs, location)?;
                        base_handle = None;
                        subs.clear();
                    }
                    let (name, _) = self.alias_name(self.get(f).name)?;
                    base = self.field_select(
                        scope,
                        handle,
                        &base,
                        base_handle,
                        None,
                        &name,
                        location,
                    )?;
                    base_handle = None;
                }
            }
        }
        if !subs.is_empty() {
            base = self.subscripts(scope, &base, &subs, location)?;
        }
        Ok(base)
    }

    /// `expandNSItemAttrs`. `scope` is the level the item belongs to,
    /// `levelsup` levels above the current one.
    fn expand_ns_item(
        &self,
        scope: &Scope,
        item: &NamespaceItem,
        levelsup: usize,
    ) -> Vec<TargetEntry> {
        let rte = &scope.rtable[item.rt_index - 1];
        let mut out = Vec::new();
        let mut ncols = item
            .using_alias
            .as_ref()
            .map_or(rte.colnames.len(), |(_, n)| *n);
        if let RteKind::Cte { hidden, .. } = &rte.kind {
            ncols -= hidden;
        }
        for (i, name) in rte.colnames[..ncols].iter().enumerate() {
            if name.is_empty() {
                continue;
            }
            let var = self
                .relations
                .make_var(item.rt_index, rte, i as i16 + 1, levelsup);
            let fields = match &rte.kind {
                RteKind::Subquery { columns, .. } => columns.get(i).and_then(|c| c.fields.clone()),
                RteKind::Cte { columns, .. } => columns.get(i).and_then(|c| c.fields.clone()),
                _ => None,
            };
            out.push(TargetEntry {
                collation_state: CollContext::new(),
                collation: self
                    .var_collation_at(scope, &Var { levelsup: 0, ..var })
                    .unwrap_or(None),
                name: name.clone(),
                ty: var.ty,
                typmod: var.typmod,
                origin: self.origin_of(scope, &var),
                junk: false,
                location: NO_LOCATION,
                fields,
            });
        }
        out
    }

    /// `ExpandColumnRefStar` for `t.*` and `s.t.*`.
    fn expand_qualified_star(
        &self,
        scope: &Scope,
        q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>,
    ) -> Result<Vec<TargetEntry>> {
        let location = self.span(q).start;
        let (names, star_at_end) = self.qualified_names(q)?;
        if !star_at_end {
            return Err(unsupported(
                "indirection after .*",
                Some(location..location),
            ));
        }
        let (schema, relname) = match names.len() {
            1 => (None, names[0].as_str()),
            2 => (Some(names[0].as_str()), names[1].as_str()),
            _ => {
                return Err(Diagnostic::new(
                    "42601",
                    format!(
                        "improper qualified name (too many dotted names): {}.*",
                        names.join(".")
                    ),
                )
                .at(location)
                .into());
            }
        };
        let item =
            self.relations
                .refname_namespace_item(scope, &self.outer, schema, relname, location)?;
        let Some((levelsup, item)) = item else {
            return Err(self
                .relations
                .error_missing_rte(scope, &self.outer, schema, relname, location)
                .into());
        };
        let level = self.scope_at(scope, levelsup);
        Ok(self.expand_ns_item(level, &item, levelsup))
    }

    /// The dotted names of a qualified reference, and whether it ends in `*`.
    fn qualified_names(
        &self,
        q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>,
    ) -> Result<(Vec<String>, bool)> {
        let (names, star, rest) = self.qualified_parts(q)?;
        if !rest.is_empty() {
            return Err(unsupported("subscript", None));
        }
        Ok((names, star))
    }

    /// The dotted names of a qualified reference up to its first
    /// subscript, whether they end in `*`, and the indirection after them.
    fn qualified_parts(
        &self,
        q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>,
    ) -> Result<(Vec<String>, bool, Vec<IndirPart>)> {
        use pgtk_sql::ast::shared::expr::{FlatQualifiedPath, FlatQualifiedTail};
        let q = self.get(q);
        let (table, _) = self.col_id(q.table)?;
        let mut names = vec![table];
        let mut star = false;
        let mut rest = Vec::new();
        match self.get(q.path) {
            FlatQualifiedPath::Star(_) => star = true,
            FlatQualifiedPath::Attrs(a) => {
                let a = self.get(*a);
                for f in self.list1(a.attrs) {
                    names.push(self.alias_name(self.get(*f).name)?.0);
                }
                match a.tail.map(|t| self.get(t)) {
                    None
                    | Some(FlatQualifiedTail::Call(_))
                    | Some(FlatQualifiedTail::TypedLiteral(_)) => {}
                    Some(FlatQualifiedTail::Star(_)) => star = true,
                    Some(FlatQualifiedTail::Subscripted(sub)) => {
                        let sub = self.get(*sub);
                        rest.push(IndirPart::Subscript(sub.subscript));
                        if let Some(more) = sub.rest {
                            rest.extend(self.indirection_parts(more));
                        }
                    }
                }
            }
        }
        Ok((names, star, rest))
    }

    /// The tail of a qualified reference after its dotted names, if any.
    fn qref_tail(
        &self,
        q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>,
    ) -> Option<&pgtk_sql::ast::shared::expr::FlatQualifiedTail> {
        use pgtk_sql::ast::shared::expr::FlatQualifiedPath;
        match self.get(self.get(q).path) {
            FlatQualifiedPath::Star(_) => None,
            FlatQualifiedPath::Attrs(a) => self.get(*a).tail.map(|t| self.get(t)),
        }
    }

    /// The call of a dotted function name: `s.f(x)` or `s.t(10) 'v'`.
    fn qref_call(
        &self,
        q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>,
    ) -> Option<Handle<pgtk_sql::ast::shared::expr::FlatFunctionCallTail>> {
        match self.qref_tail(q) {
            Some(pgtk_sql::ast::shared::expr::FlatQualifiedTail::Call(call)) => Some(*call),
            _ => None,
        }
    }

    /// Whether a qualified reference is `func_name Sconst`: `s.t 'v'`.
    fn qref_typed_literal(&self, q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>) -> bool {
        matches!(
            self.qref_tail(q),
            Some(pgtk_sql::ast::shared::expr::FlatQualifiedTail::TypedLiteral(_))
        )
    }

    /// Whether a qualified reference is a function call or a typed literal,
    /// not a column reference.
    fn qref_function(&self, q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>) -> bool {
        self.qref_call(q).is_some() || self.qref_typed_literal(q)
    }

    /// Whether a qualified reference ends in `.*`.
    fn qref_star(&self, q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>) -> bool {
        use pgtk_sql::ast::shared::expr::{FlatIndirection, FlatQualifiedPath, FlatQualifiedTail};
        if matches!(self.get(self.get(q).path), FlatQualifiedPath::Star(_)) {
            return true;
        }
        match self.qref_tail(q) {
            Some(FlatQualifiedTail::Star(_)) => true,
            Some(FlatQualifiedTail::Subscripted(sub)) => self.get(*sub).rest.is_some_and(|more| {
                matches!(
                    self.get(more),
                    FlatIndirection::Star(_) | FlatIndirection::ElementsStar(..)
                )
            }),
            _ => false,
        }
    }

    /// Whether a qualified reference selects a field or a subscript past
    /// `t.a`: `t.a.b`, `t.a[1]`.
    fn qref_indirect(&self, q: Handle<pgtk_sql::ast::shared::expr::FlatQualifiedRef>) -> bool {
        use pgtk_sql::ast::shared::expr::FlatQualifiedPath;
        match self.get(self.get(q).path) {
            FlatQualifiedPath::Star(_) => false,
            FlatQualifiedPath::Attrs(a) => {
                let a = self.get(*a);
                self.list1(a.attrs).len() > 1 || a.tail.is_some()
            }
        }
    }

    /// The parts of an [`Indirection`](pgtk_sql::ast::shared::expr::Indirection).
    fn indirection_parts(
        &self,
        i: Handle<pgtk_sql::ast::shared::expr::FlatIndirection>,
    ) -> Vec<IndirPart> {
        use pgtk_sql::ast::shared::expr::{FlatIndirection, FlatIndirectionEl};
        let elements = |list| {
            self.list1(list)
                .iter()
                .map(|el| match self.get(*el) {
                    FlatIndirectionEl::Subscript(b) => IndirPart::Subscript(*b),
                    FlatIndirectionEl::Field(f) => IndirPart::Field(*f),
                })
                .collect::<Vec<_>>()
        };
        match self.get(i) {
            FlatIndirection::Star(_) => vec![IndirPart::Star],
            FlatIndirection::Elements(list) => elements(*list),
            FlatIndirection::ElementsStar(list, _) => {
                let mut parts = elements(*list);
                parts.push(IndirPart::Star);
                parts
            }
        }
    }

    /// The elements of the indirection of an assignment target: `SET a[1].b
    /// = ...`, `INSERT INTO t (a[1].b)`. `transformAssignmentIndirection`
    /// refuses a `.*`.
    fn assignment_indirection(
        &self,
        i: Option<Handle<pgtk_sql::ast::shared::expr::FlatIndirection>>,
    ) -> Result<Vec<Handle<FlatIndirectionEl>>> {
        use pgtk_sql::ast::shared::expr::FlatIndirection;
        let Some(i) = i else {
            return Ok(Vec::new());
        };
        match self.get(i) {
            FlatIndirection::Elements(list) => Ok(self.list1(*list).to_vec()),
            FlatIndirection::Star(_) | FlatIndirection::ElementsStar(..) => Err(Diagnostic::new(
                "0A000",
                "row expansion via \"*\" is not supported here",
            )
            .at(self.span(i).start)
            .into()),
        }
    }

    /// The parts of the indirection of an unqualified column reference:
    /// its subscript and the elements after it.
    fn column_ref_parts(
        &self,
        i: Option<Handle<pgtk_sql::ast::shared::expr::FlatSubscriptedIndirection>>,
    ) -> Vec<IndirPart> {
        let Some(i) = i else {
            return Vec::new();
        };
        let i = self.get(i);
        let mut parts = vec![IndirPart::Subscript(i.subscript)];
        parts.extend(self.opt_indirection_parts(i.rest));
        parts
    }

    /// The parts of an optional indirection chain; none when it is absent.
    fn opt_indirection_parts(
        &self,
        i: Option<Handle<pgtk_sql::ast::shared::expr::FlatIndirection>>,
    ) -> Vec<IndirPart> {
        i.map_or_else(Vec::new, |i| self.indirection_parts(i))
    }

    /// `FigureColnameInternal`.
    fn figure_colname(&self, handle: Handle<FlatExpr>) -> Colname {
        let none = Colname {
            name: None,
            strength: 0,
        };
        match self.get(handle) {
            FlatExpr::ColumnRef(c) => {
                let c = self.get(*c);
                // A_Indirection: the last field name after a subscript.
                let mut name = self.last_field_name(&self.column_ref_parts(c.indirection));
                if name.is_none() {
                    name = self.col_id(c.name).ok().map(|(n, _)| n);
                }
                Colname { name, strength: 2 }
            }
            FlatExpr::QualRef(q) => {
                let q_handle = *q;
                if self.qref_function(q_handle) {
                    return Colname {
                        name: self
                            .qualified_names(q_handle)
                            .ok()
                            .and_then(|(n, _)| n.last().cloned()),
                        strength: 2,
                    };
                }
                match self.qualified_parts(q_handle) {
                    Ok((names, _, rest)) => {
                        // The last field name, through any subscripts.
                        let mut name = None;
                        for part in rest {
                            if let IndirPart::Field(f) = part {
                                name = self.alias_name(self.get(f).name).ok().map(|(n, _)| n);
                            }
                        }
                        Colname {
                            name: name.or_else(|| names.last().cloned()),
                            strength: 2,
                        }
                    }
                    Err(_) => none,
                }
            }
            FlatExpr::Overlaps(..) => Colname {
                name: Some("overlaps".to_owned()),
                strength: 2,
            },
            FlatExpr::Grouping(_) => Colname {
                name: Some("grouping".to_owned()),
                strength: 2,
            },
            FlatExpr::XmlElement(_) => Colname {
                name: Some("xmlelement".to_owned()),
                strength: 2,
            },
            FlatExpr::XmlForest(_) => Colname {
                name: Some("xmlforest".to_owned()),
                strength: 2,
            },
            FlatExpr::XmlParse(_) => Colname {
                name: Some("xmlparse".to_owned()),
                strength: 2,
            },
            FlatExpr::XmlPi(_) => Colname {
                name: Some("xmlpi".to_owned()),
                strength: 2,
            },
            FlatExpr::XmlRoot(_) => Colname {
                name: Some("xmlroot".to_owned()),
                strength: 2,
            },
            FlatExpr::XmlSerialize(_) => Colname {
                name: Some("xmlserialize".to_owned()),
                strength: 2,
            },
            FlatExpr::XmlExists(_) => Colname {
                name: Some("xmlexists".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg16")]
            FlatExpr::JsonObject(o) => {
                // JSON_OBJECT(a, b), the plain function, is named the same.
                let _ = o;
                Colname {
                    name: Some("json_object".to_owned()),
                    strength: 2,
                }
            }
            #[cfg(feature = "since-pg16")]
            FlatExpr::JsonArray(_) => Colname {
                name: Some("json_array".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg16")]
            FlatExpr::JsonObjectAgg(_) => Colname {
                name: Some("json_objectagg".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg16")]
            FlatExpr::JsonArrayAgg(_) => Colname {
                name: Some("json_arrayagg".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonCtor(_) => Colname {
                name: Some("json".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonScalar(_) => Colname {
                name: Some("json_scalar".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonSerialize(_) => Colname {
                name: Some("json_serialize".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg17")]
            FlatExpr::MergeAction(_) => Colname {
                name: Some("merge_action".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonExists(_) => Colname {
                name: Some("json_exists".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonQuery(_) => Colname {
                name: Some("json_query".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonValue(_) => Colname {
                name: Some("json_value".to_owned()),
                strength: 2,
            },
            FlatExpr::Func(f) => {
                let f = self.get(*f);
                let pgtk_sql::tokens::Flattype_function_name::Text(t) = self.get(f.name);
                let name = pgtk_sql::ident::fold_identifier(self.text(*t))
                    .ok()
                    .map(|f| f.name.into_owned());
                Colname { name, strength: 2 }
            }
            FlatExpr::NullIf(_) => Colname {
                name: Some("nullif".to_owned()),
                strength: 2,
            },
            FlatExpr::AtTimeZone(..) => Colname {
                name: Some("timezone".to_owned()),
                strength: 2,
            },
            #[cfg(feature = "since-pg17")]
            FlatExpr::AtLocal(_) => Colname {
                name: Some("timezone".to_owned()),
                strength: 2,
            },
            FlatExpr::XmlConcat(_) => Colname {
                name: Some("xmlconcat".to_owned()),
                strength: 2,
            },
            FlatExpr::Extract(_) => Colname {
                name: Some("extract".to_owned()),
                strength: 2,
            },
            FlatExpr::Position(_) => Colname {
                name: Some("position".to_owned()),
                strength: 2,
            },
            FlatExpr::Overlay(_) => Colname {
                name: Some("overlay".to_owned()),
                strength: 2,
            },
            FlatExpr::Substring(_) => Colname {
                name: Some("substring".to_owned()),
                strength: 2,
            },
            FlatExpr::Trim(t) => {
                let inner = self.get(self.get(*t).inner);
                let n = match inner.dir.map(|d| self.get(d)) {
                    Some(FlatTrimDir::Leading) => "ltrim",
                    Some(FlatTrimDir::Trailing) => "rtrim",
                    _ => "btrim",
                };
                Colname {
                    name: Some(n.to_owned()),
                    strength: 2,
                }
            }
            FlatExpr::TimestampLit(t) => {
                let tz = matches!(
                    self.get(*t).tz.map(|z| self.get(z)),
                    Some(FlatTimeZoneQualifier::With)
                );
                Colname {
                    name: Some(if tz { "timestamptz" } else { "timestamp" }.to_owned()),
                    strength: 1,
                }
            }
            FlatExpr::TimeLit(t) => {
                let tz = matches!(
                    self.get(*t).tz.map(|z| self.get(z)),
                    Some(FlatTimeZoneQualifier::With)
                );
                Colname {
                    name: Some(if tz { "timetz" } else { "time" }.to_owned()),
                    strength: 1,
                }
            }
            FlatExpr::IntervalLit(_) => Colname {
                name: Some("interval".to_owned()),
                strength: 1,
            },
            FlatExpr::Normalize(_) => Colname {
                name: Some("normalize".to_owned()),
                strength: 2,
            },
            FlatExpr::CollationFor(_) => Colname {
                name: Some("pg_collation_for".to_owned()),
                strength: 2,
            },
            FlatExpr::Coalesce(_) => Colname {
                name: Some("coalesce".to_owned()),
                strength: 2,
            },
            FlatExpr::Greatest(_) => Colname {
                name: Some("greatest".to_owned()),
                strength: 2,
            },
            FlatExpr::Least(_) => Colname {
                name: Some("least".to_owned()),
                strength: 2,
            },
            FlatExpr::Array(_) => Colname {
                name: Some("array".to_owned()),
                strength: 2,
            },
            FlatExpr::RowExpr(_) => Colname {
                name: Some("row".to_owned()),
                strength: 2,
            },
            FlatExpr::Exists(_) => Colname {
                name: Some("exists".to_owned()),
                strength: 2,
            },
            FlatExpr::Case(c) => {
                // FigureColname looks at the ELSE result; "case" otherwise.
                let c = self.get(*c);
                let else_expr = self.case_else(c.body);
                let inner = else_expr.map(|e| self.figure_colname(e));
                match inner {
                    Some(inner) if inner.strength > 1 => inner,
                    _ => Colname {
                        name: Some("case".to_owned()),
                        strength: 1,
                    },
                }
            }
            FlatExpr::Cast(inner, cast_type) => {
                let inner = self.figure_colname(*inner);
                if inner.strength > 1 {
                    return inner;
                }
                Colname {
                    name: self.cast_type_last_name(*cast_type),
                    strength: 1,
                }
            }
            FlatExpr::CastCall(c) => {
                let c = self.get(*c);
                let inner = self.get(c.inner);
                let name = self.figure_colname(inner.value);
                if name.strength > 1 {
                    return name;
                }
                Colname {
                    name: self.cast_type_last_name(inner.target),
                    strength: 1,
                }
            }
            // TREAT is a FuncCall named after the type, so `FigureColname`
            // sees the name of that type.
            FlatExpr::Treat(t) => Colname {
                name: self.cast_type_last_name(self.get(self.get(*t).inner).target),
                strength: 2,
            },
            FlatExpr::CastFunc(f) => Colname {
                name: self.typed_literal_type_name(*f),
                strength: 1,
            },
            FlatExpr::Collate(inner, _) => self.figure_colname(*inner),
            FlatExpr::Parenthesized(p) => {
                let p = self.get(*p);
                // A_Indirection: the last field name, if any.
                let name = self.last_field_name(&self.opt_indirection_parts(p.indirection));
                if name.is_some() {
                    return Colname { name, strength: 2 };
                }
                self.figure_colname(p.expr)
            }
            FlatExpr::ImplicitRow(_) => Colname {
                name: Some("row".to_owned()),
                strength: 2,
            },
            FlatExpr::User => Colname {
                name: Some("user".to_owned()),
                strength: 2,
            },
            FlatExpr::SqlValue(v) => Colname {
                name: Some(self.sql_value_keyword(*v).0.to_owned()),
                strength: 2,
            },
            // EXPR_SUBLINK: the name of the subquery's single column.
            FlatExpr::SubSelect(_) => match self.sublink_names.get(&handle) {
                Some(name) => Colname {
                    name: Some(name.clone()),
                    strength: 2,
                },
                None => none,
            },
            FlatExpr::SubSelectIndirection(q) => {
                // A_Indirection: the last field name, else the sublink's.
                let name = self
                    .last_field_name(&self.indirection_parts(self.get(*q).indirection))
                    .or_else(|| self.sublink_names.get(&handle).cloned());
                if name.is_some() {
                    Colname { name, strength: 2 }
                } else {
                    none
                }
            }
            FlatExpr::PositionalParam(_) => none,
            _ => none,
        }
    }

    /// The last field name of an `A_Indirection`, if any.
    fn last_field_name(&self, parts: &[IndirPart]) -> Option<String> {
        parts.iter().rev().find_map(|ind| match *ind {
            IndirPart::Field(f) => self.alias_name(self.get(f).name).ok().map(|(n, _)| n),
            _ => None,
        })
    }

    fn case_else(
        &self,
        body: Handle<pgtk_sql::ast::shared::expr::FlatCaseBody>,
    ) -> Option<Handle<FlatExpr>> {
        let else_clause = match self.get(body) {
            FlatCaseBody::Searched(s) => self.get(*s).else_clause,
            FlatCaseBody::Simple(s) => self.get(*s).else_clause,
        };
        else_clause.map(|e| self.get(e).result)
    }

    /// The last name of a cast's type name, as `FigureColname` uses it.
    fn cast_type_last_name(
        &self,
        cast_type: Handle<pgtk_sql::ast::shared::expr::FlatCastType>,
    ) -> Option<String> {
        let tn = self.type_name(cast_type).ok()?;
        Some(tn.name)
    }

    fn typed_literal_type_name(&self, f: Handle<FlatTypeCastFunc>) -> Option<String> {
        match self.get(f) {
            FlatTypeCastFunc::Fixed(fixed) => {
                let fixed = self.get(*fixed);
                let (name, _) = self.fixed_type(fixed.type_name).ok()?;
                Some(name.to_owned())
            }
            #[cfg(feature = "since-pg17")]
            FlatTypeCastFunc::Json(_) => Some("json".to_owned()),
            FlatTypeCastFunc::Named(named) => {
                let named = self.get(*named);
                let pgtk_sql::tokens::Flattype_function_name::Text(t) = self.get(named.type_name);
                pgtk_sql::ident::fold_identifier(self.text(*t))
                    .ok()
                    .map(|f| f.name.into_owned())
            }
        }
    }

    // ----- type names -----

    /// The `TypeName` a cast's type syntax means, with `gram.y`'s
    /// `SystemTypeName` mapping for the SQL-standard spellings.
    fn type_name(
        &self,
        cast_type: Handle<pgtk_sql::ast::shared::expr::FlatCastType>,
    ) -> Result<TypeName> {
        let ct = self.get(cast_type);
        let location = self.span(cast_type).start;
        let array_dims = match ct.array.map(|array| self.get(array)) {
            None => 0,
            Some(FlatTypeArray::Bounds(bounds)) => self.list1(*bounds).len(),
            Some(FlatTypeArray::Keyword(_)) => 1,
        };
        self.simple_type_name(ct.head, array_dims, location)
    }

    /// The `TypeName` of a `SimpleTypename`, with `array_dims` array
    /// dimensions.
    fn simple_type_name(
        &self,
        head: Handle<FlatCastTypeHead>,
        array_dims: usize,
        location: usize,
    ) -> Result<TypeName> {
        let (schema, name, typmods) = match self.get(head) {
            FlatCastTypeHead::Numeric(n) => {
                let (name, typmods) = self.numeric_type(*n)?;
                (Some("pg_catalog"), name.to_owned(), typmods)
            }
            FlatCastTypeHead::Bit(b) => {
                let (name, typmods) = self.bit_type(*b)?;
                (Some("pg_catalog"), name.to_owned(), typmods)
            }
            FlatCastTypeHead::Character(c) => {
                let (name, typmods) = self.character_type(*c);
                (Some("pg_catalog"), name.to_owned(), typmods)
            }
            #[cfg(feature = "since-pg17")]
            FlatCastTypeHead::Json => (Some("pg_catalog"), "json".to_owned(), Vec::new()),
            FlatCastTypeHead::Generic(g) => {
                let g = self.get(*g);
                let typmods = match g.modifiers {
                    Some(p) => self.typmod_args(p)?,
                    None => Vec::new(),
                };
                let (schema, name) = match self.get(g.name) {
                    FlatGenericTypeName::Bool => (Some("pg_catalog"), "bool".to_owned()),
                    FlatGenericTypeName::Text => (Some("pg_catalog"), "text".to_owned()),
                    FlatGenericTypeName::Serial => (None, "serial".to_owned()),
                    FlatGenericTypeName::Unknown => (Some("pg_catalog"), "unknown".to_owned()),
                    FlatGenericTypeName::Ident(ident) => {
                        let ident = self.get(*ident);
                        let pgtk_sql::tokens::Flattype_name_ident::Text(t) = self.get(ident.first);
                        let mut parts = vec![self.fold(self.text(*t), self.text_span(*t).start)?];
                        for attr in self.list(ident.rest) {
                            parts.push(self.col_label(self.get(*attr).name)?);
                        }
                        if parts.len() > 2 {
                            return Err(unsupported(
                                "catalog-qualified type name",
                                Some(location..location),
                            ));
                        }
                        let name = parts.pop().expect("at least one part");
                        return Ok(TypeName {
                            schema: parts.pop(),
                            name,
                            typmods,
                            array_dims,
                            location,
                        });
                    }
                };
                (schema, name, typmods)
            }
            FlatCastTypeHead::DateTime(d) => {
                let d = self.get(*d);
                let tz = matches!(d.tz.map(|t| self.get(t)), Some(FlatTimeZoneQualifier::With));
                let name = match (self.get(d.base), tz) {
                    (FlatDateTimeCastTypeName::Timestamp, false) => "timestamp",
                    (FlatDateTimeCastTypeName::Timestamp, true) => "timestamptz",
                    (FlatDateTimeCastTypeName::Time, false) => "time",
                    (FlatDateTimeCastTypeName::Time, true) => "timetz",
                };
                let typmods = d
                    .precision
                    .map(|p| vec![TypmodArg::Integer(self.iconst_precision(p))])
                    .unwrap_or_default();
                (Some("pg_catalog"), name.to_owned(), typmods)
            }
            FlatCastTypeHead::Interval(i) => {
                let i = self.get(*i);
                let typmods = match i.modifier.map(|m| self.get(m)) {
                    None => Vec::new(),
                    Some(FlatIntervalCastTypeModifier::Precision(p)) => vec![
                        TypmodArg::Integer(0x7fff),
                        TypmodArg::Integer(self.iconst_precision(*p)),
                    ],
                    Some(FlatIntervalCastTypeModifier::Qualifier(q)) => {
                        let (fields, precision) = self.interval_qualifier(*q);
                        let mut v = vec![TypmodArg::Integer(i64::from(fields))];
                        if let Some(p) = precision {
                            v.push(TypmodArg::Integer(p));
                        }
                        v
                    }
                };
                (Some("pg_catalog"), "interval".to_owned(), typmods)
            }
        };
        Ok(TypeName {
            schema: schema.map(str::to_owned),
            name,
            typmods,
            array_dims,
            location,
        })
    }

    /// `SQLValueFunction`: the type of a bare keyword such as
    /// `current_user`, or `None` for an ordinary name.
    fn sql_value_function(&self, name: &str) -> Option<TypeRef> {
        let b = match name {
            "current_date" => Builtin::Date,
            "current_time" => Builtin::Timetz,
            "current_timestamp" => Builtin::Timestamptz,
            "localtime" => Builtin::Time,
            "localtimestamp" => Builtin::Timestamp,
            "current_role" | "current_user" | "session_user" | "user" | "current_catalog"
            | "current_schema" => Builtin::Name,
            _ => return None,
        };
        Some(self.types().builtin(b))
    }

    /// The name of a `SQLValueFunction` keyword, as `FigureColname` gives
    /// it, and its precision, if written.
    fn sql_value_keyword(&self, v: Handle<FlatSqlValueFunction>) -> (&'static str, Option<i64>) {
        match self.get(v) {
            FlatSqlValueFunction::CurrentCatalog => ("current_catalog", None),
            FlatSqlValueFunction::CurrentDate => ("current_date", None),
            FlatSqlValueFunction::CurrentRole => ("current_role", None),
            FlatSqlValueFunction::CurrentUser => ("current_user", None),
            FlatSqlValueFunction::SessionUser => ("session_user", None),
            #[cfg(feature = "since-pg16")]
            FlatSqlValueFunction::SystemUser => ("system_user", None),
            FlatSqlValueFunction::Precision(p) => {
                let p = self.get(*p);
                let name = match self.get(p.name) {
                    FlatPrecisionSqlValueFuncName::CurrentTime => "current_time",
                    FlatPrecisionSqlValueFuncName::CurrentTimestamp => "current_timestamp",
                    FlatPrecisionSqlValueFuncName::LocalTime => "localtime",
                    FlatPrecisionSqlValueFuncName::LocalTimestamp => "localtimestamp",
                };
                let precision = p.precision.map(|q| {
                    let raw = self.text(self.get(q).value);
                    crate::consts::integer_value(raw).expect("an Iconst is an int32")
                });
                (name, precision)
            }
        }
    }

    /// `transformSQLValueFunction`. `SYSTEM_USER` is not one: gram.y makes
    /// it a call of `pg_catalog.system_user()`.
    fn sql_value(
        &mut self,
        handle: Handle<FlatExpr>,
        name: &str,
        precision: Option<i64>,
        location: usize,
    ) -> Result<Typed> {
        if name == "system_user" {
            let name = ObjectName {
                schema: Some("pg_catalog".to_owned()),
                name: name.to_owned(),
            };
            return self.resolve_call(handle, name, &[], &[], location, None);
        }
        let ty = self
            .sql_value_function(name)
            .expect("every SQLValueFunction keyword has a type");
        // anytime_typmod_check: a precision above 6 is reduced to 6.
        let typmod = precision.map_or(-1, |p| p.min(6) as i32);
        Ok(Typed {
            ty,
            typmod,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// gram.y `Numeric`: the `pg_catalog` type name and its modifiers, with
    /// `opt_float`'s precision rule for `FLOAT(p)`.
    fn numeric_type(&self, n: Handle<FlatNumericType>) -> Result<(&'static str, Vec<TypmodArg>)> {
        Ok(match self.get(n) {
            FlatNumericType::Int | FlatNumericType::Integer => ("int4", Vec::new()),
            FlatNumericType::Smallint => ("int2", Vec::new()),
            FlatNumericType::Bigint => ("int8", Vec::new()),
            FlatNumericType::Real => ("float4", Vec::new()),
            FlatNumericType::DoublePrecision => ("float8", Vec::new()),
            FlatNumericType::Boolean => ("bool", Vec::new()),
            FlatNumericType::Decimal(d) => {
                let typmods = match self.get(*d).modifiers {
                    Some(p) => self.typmod_args(p)?,
                    None => Vec::new(),
                };
                ("numeric", typmods)
            }
            // opt_float: pgtk-sql refuses a precision outside 1..53.
            FlatNumericType::Float(f) => match self.get(*f).precision {
                Some(p) if self.float_precision(p) <= 24 => ("float4", Vec::new()),
                _ => ("float8", Vec::new()),
            },
        })
    }

    /// gram.y `Bit`: the `pg_catalog` type name and its modifiers.
    fn bit_type(&self, b: Handle<FlatBitType>) -> Result<(&'static str, Vec<TypmodArg>)> {
        let b = self.get(b);
        let typmods = match b.modifiers {
            Some(p) => self.typmod_args(p)?,
            None => Vec::new(),
        };
        Ok((if b.varying { "varbit" } else { "bit" }, typmods))
    }

    /// gram.y `Character`: the `pg_catalog` type name and its length.
    fn character_type(&self, c: Handle<FlatCharacterType>) -> (&'static str, Vec<TypmodArg>) {
        let c = self.get(c);
        let name = match self.get(c.name) {
            FlatCharacterTypeName::NationalCharacterVarying
            | FlatCharacterTypeName::NationalCharVarying
            | FlatCharacterTypeName::CharacterVarying
            | FlatCharacterTypeName::CharVarying
            | FlatCharacterTypeName::NcharVarying
            | FlatCharacterTypeName::Varchar => "varchar",
            FlatCharacterTypeName::NationalCharacter
            | FlatCharacterTypeName::NationalChar
            | FlatCharacterTypeName::Character
            | FlatCharacterTypeName::Char
            | FlatCharacterTypeName::Nchar => "bpchar",
        };
        let typmods = c
            .length
            .map(|p| vec![TypmodArg::Integer(self.iconst_precision(p))])
            .unwrap_or_default();
        (name, typmods)
    }

    /// gram.y `ConstTypename` for a fixed-keyword typed literal.
    fn fixed_type(
        &self,
        f: Handle<FlatFixedTypeCastFuncName>,
    ) -> Result<(&'static str, Vec<TypmodArg>)> {
        match self.get(f) {
            FlatFixedTypeCastFuncName::Numeric(n) => self.numeric_type(*n),
            FlatFixedTypeCastFuncName::Bit(b) => self.bit_type(*b),
            FlatFixedTypeCastFuncName::Character(c) => Ok(self.character_type(*c)),
        }
    }

    /// gram.y `opt_float`'s `'(' Iconst ')'`.
    fn float_precision(&self, p: Handle<FlatFloatPrecision>) -> i64 {
        let raw = self.text(self.get(p).value);
        crate::consts::integer_value(raw).expect("an Iconst is an int32")
    }

    /// gram.y `'(' Iconst ')'`. The grammar admits only an unsigned
    /// integer that fits in `int32`.
    fn iconst_precision(&self, p: Handle<FlatIconstPrecision>) -> i64 {
        let raw = self.text(self.get(p).value);
        crate::consts::integer_value(raw).expect("an Iconst is an int32")
    }

    fn interval_qualifier(&self, q: Handle<FlatIntervalQualifier>) -> (i32, Option<i64>) {
        const YEAR: i32 = 4;
        const MONTH: i32 = 2;
        const DAY: i32 = 8;
        const HOUR: i32 = 1024;
        const MINUTE: i32 = 2048;
        const SECOND: i32 = 4096;
        let precision =
            |h: &Handle<pgtk_sql::ast::shared::expr::FlatSecondWithPrecision>| -> Option<i64> {
                self.get(*h).precision.map(|p| self.iconst_precision(p))
            };
        match self.get(q) {
            FlatIntervalQualifier::Year => (YEAR, None),
            FlatIntervalQualifier::Month => (MONTH, None),
            FlatIntervalQualifier::Day => (DAY, None),
            FlatIntervalQualifier::Hour => (HOUR, None),
            FlatIntervalQualifier::Minute => (MINUTE, None),
            FlatIntervalQualifier::Second(p) => (SECOND, precision(p)),
            FlatIntervalQualifier::YearToMonth => (YEAR | MONTH, None),
            FlatIntervalQualifier::DayToHour => (DAY | HOUR, None),
            FlatIntervalQualifier::DayToMinute => (DAY | HOUR | MINUTE, None),
            FlatIntervalQualifier::DayToSecond(p) => (DAY | HOUR | MINUTE | SECOND, precision(p)),
            FlatIntervalQualifier::HourToMinute => (HOUR | MINUTE, None),
            FlatIntervalQualifier::HourToSecond(p) => (HOUR | MINUTE | SECOND, precision(p)),
            FlatIntervalQualifier::MinuteToSecond(p) => (MINUTE | SECOND, precision(p)),
        }
    }

    /// `typenameTypeMod`: a modifier written as an expression must be an
    /// integer or string constant or a plain identifier.
    fn typmod_of_expr(&self, e: Handle<FlatExpr>) -> Result<TypmodArg> {
        let at = self.span(e).start;
        let arg = match self.get(e) {
            FlatExpr::IntegerLit(t) => {
                crate::consts::integer_value(self.text(*t)).map(TypmodArg::Integer)
            }
            FlatExpr::Neg(inner) => match self.get(*inner) {
                FlatExpr::IntegerLit(t) => {
                    crate::consts::integer_value(self.text(*t)).map(|v| TypmodArg::Integer(-v))
                }
                _ => None,
            },
            FlatExpr::StringLit(s) => {
                let text = match self.get(*s) {
                    FlatStringLitSeq0::Single(t) => self.text(*t),
                    FlatStringLitSeq0::Sequence(t) => self.text(*t),
                };
                Some(TypmodArg::Ident(string_literal_body(text)))
            }
            FlatExpr::ColumnRef(c) => {
                let c = self.get(*c);
                if c.indirection.is_none() {
                    Some(TypmodArg::Ident(self.col_id(c.name)?.0))
                } else {
                    None
                }
            }
            _ => None,
        };
        arg.ok_or_else(|| {
            Diagnostic::new(
                "42601",
                "type modifiers must be simple constants or identifiers",
            )
            .at(at)
            .into()
        })
    }

    fn typmod_args(
        &self,
        p: Handle<pgtk_sql::ast::shared::expr::FlatTypePrecision>,
    ) -> Result<Vec<TypmodArg>> {
        let p = self.get(p);
        let mut out = Vec::new();
        for arg in self.list1(p.args) {
            let arg = self.get(*arg);
            let negative = matches!(
                arg.sign.map(|s| self.get(s)),
                Some(FlatTypeModifierSign::Neg)
            );
            // `typenameTypeMod` passes a float constant, an integer above
            // int32 included, as its text.
            let raw = match self.get(arg.value) {
                FlatUnsignedNumberLit::Integer(t) => {
                    let raw = self.text(*t);
                    if let Some(v) = crate::consts::integer_value(raw) {
                        out.push(TypmodArg::Integer(if negative { -v } else { v }));
                        continue;
                    }
                    raw
                }
                FlatUnsignedNumberLit::Numeric(t) => self.text(*t),
            };
            let sign = if negative { "-" } else { "" };
            out.push(TypmodArg::Ident(format!("{sign}{raw}")));
        }
        Ok(out)
    }

    // ----- expressions -----

    /// An EXPR_SUBLINK: the type of the subquery's single column.
    fn expr_sublink(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        sub: Handle<FlatSubquery>,
        location: usize,
    ) -> Result<Typed> {
        self.sublinks.insert(handle);
        let columns = self.sublink_columns(scope, handle, sub)?;
        if let Some(first) = columns.first() {
            self.sublink_names.insert(handle, first.name.clone());
        }
        if columns.len() != 1 {
            return Err(
                Diagnostic::new("42601", "subquery must return only one column")
                    .at(location)
                    .into(),
            );
        }
        Ok(Typed {
            ty: columns[0].ty,
            typmod: columns[0].typmod,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `transformMergeSupportFunc`: `MERGE_ACTION()` is `text`, and only the
    /// RETURNING list of a MERGE may use it.
    fn merge_action(&self, location: usize) -> Result<Typed> {
        if self.expr_kind != ExprKind::MergeReturning
            && !self.outer_kinds.contains(&ExprKind::MergeReturning)
        {
            return Err(Diagnostic::new(
                "42601",
                "MERGE_ACTION() can only be used in the RETURNING list of a MERGE command",
            )
            .at(location)
            .into());
        }
        Ok(Typed {
            ty: self.types().builtin(Builtin::Text),
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `transformExprRecurse`. Records the expression's type and returns
    /// its type facts.
    fn expr(&mut self, scope: &Scope, handle: Handle<FlatExpr>) -> Result<Typed> {
        let top = self.expr_depth == 0;
        self.expr_depth += 1;
        let typed = self.expr_inner(scope, handle);
        self.expr_depth -= 1;
        let typed = typed?;
        self.record_type(handle, &typed);
        if top {
            self.clause_exprs.push(handle);
        }
        Ok(typed)
    }

    fn expr_inner(&mut self, scope: &Scope, handle: Handle<FlatExpr>) -> Result<Typed> {
        let location = self.span(handle).start;
        let node = self.get(handle);
        match node {
            FlatExpr::IntegerLit(t) => {
                let text = self.text(*t);
                Ok(make_const(
                    self.types(),
                    LiteralKind::Integer,
                    text,
                    location,
                ))
            }
            FlatExpr::NumericLit(t) => {
                let text = self.text(*t);
                Ok(make_const(self.types(), LiteralKind::Float, text, location))
            }
            FlatExpr::StringLit(s) => {
                let text = match self.get(*s) {
                    FlatStringLitSeq0::Single(t) => self.text(*t),
                    FlatStringLitSeq0::Sequence(t) => self.text(*t),
                };
                let inner = string_literal_body(text);
                Ok(make_const(
                    self.types(),
                    LiteralKind::String,
                    &inner,
                    location,
                ))
            }
            FlatExpr::EscapeStringLit(t) => {
                let text = self.text(*t);
                Ok(make_const(
                    self.types(),
                    LiteralKind::String,
                    text,
                    location,
                ))
            }
            FlatExpr::DollarStringLit(t) => {
                let text = self.text(*t);
                Ok(make_const(
                    self.types(),
                    LiteralKind::String,
                    text,
                    location,
                ))
            }
            FlatExpr::UnicodeStringLit(u) => {
                let u = self.get(*u);
                let text = self.text(u.lit);
                Ok(make_const(
                    self.types(),
                    LiteralKind::String,
                    text,
                    location,
                ))
            }
            FlatExpr::BitStringLit(t) => {
                let text = self.text(*t);
                Ok(make_const(
                    self.types(),
                    LiteralKind::BitString,
                    text,
                    location,
                ))
            }
            FlatExpr::HexStringLit(t) => {
                let text = self.text(*t);
                Ok(make_const(
                    self.types(),
                    LiteralKind::BitString,
                    text,
                    location,
                ))
            }
            FlatExpr::BoolTrue => Ok(make_const(
                self.types(),
                LiteralKind::Boolean,
                "t",
                location,
            )),
            FlatExpr::BoolFalse => Ok(make_const(
                self.types(),
                LiteralKind::Boolean,
                "f",
                location,
            )),
            FlatExpr::Null => Ok(make_const(self.types(), LiteralKind::Null, "", location)),
            FlatExpr::Default => Err(Diagnostic::new(
                "42601",
                "DEFAULT is not allowed in this context",
            )
            .at(location)
            .into()),
            FlatExpr::PositionalParam(p) if self.get(*p).indirection.is_some() => Err(unsupported(
                "parameter with indirection",
                Some(location..location),
            )),
            FlatExpr::PositionalParam(p) => {
                let p = self.get(*p);
                let text = self.text(p.param);
                let number: usize = text
                    .trim_start_matches('$')
                    .parse()
                    .map_err(|_| unsupported("parameter number", None))?;
                if number == 0 {
                    return Err(Diagnostic::new("42P02", "there is no parameter $0")
                        .at(location)
                        .into());
                }
                let ty = {
                    let mut params = self.coercer().params.borrow_mut();
                    if params.len() < number {
                        params.resize(number, None);
                    }
                    params[number - 1]
                };
                let ty = ty.unwrap_or_else(|| self.types().builtin(Builtin::Unknown));
                Ok(Typed {
                    ty,
                    typmod: -1,
                    form: Form::Param { number },
                    location,
                    returns_set: false,
                })
            }
            FlatExpr::Neg(inner) => {
                if let Some((kind, text)) = self.negatable_literal(*inner) {
                    // doNegate folds the sign into the constant.
                    return Ok(make_const(
                        self.types(),
                        kind,
                        &format!("-{text}"),
                        location,
                    ));
                }
                let inner_handle = *inner;
                let arg = self.expr(scope, inner_handle)?;
                self.prefix_op(handle, "-", location, inner_handle, &arg)
            }
            FlatExpr::Pos(inner) => {
                let inner_handle = *inner;
                let arg = self.expr(scope, inner_handle)?;
                self.prefix_op(handle, "+", location, inner_handle, &arg)
            }
            FlatExpr::ColumnRef(c) => {
                let c = self.get(*c);
                if c.indirection.is_some() {
                    let (name, _) = self.col_id(c.name)?;
                    let base = self.column_ref(scope, handle, &[name], location)?;
                    let parts = self.column_ref_parts(c.indirection);
                    return self.indirection(scope, handle, base, Some(handle), &parts, location);
                }
                let FlatColId::Text(raw) = self.get(c.name);
                let raw = self.text(*raw);
                let (name, _) = self.col_id(c.name)?;
                // gram.y turns these unquoted keywords into SQLValueFunction
                // nodes before any column lookup.
                if !raw.starts_with('"')
                    && let Some(ty) = self.sql_value_function(&name)
                {
                    return Ok(Typed {
                        ty,
                        typmod: -1,
                        form: Form::Other,
                        location,
                        returns_set: false,
                    });
                }
                self.column_ref(scope, handle, &[name], location)
            }
            FlatExpr::QualRef(q) => {
                let q_handle = *q;
                if self.qref_typed_literal(q_handle) {
                    return Err(unsupported(
                        "qualified typed literal",
                        Some(location..location),
                    ));
                }
                if let Some(call) = self.qref_call(q_handle) {
                    // A qualified function call: s.f(...).
                    let (names, star) = self.qualified_names(q_handle)?;
                    if star {
                        return Err(unsupported(
                            "qualified star in expression",
                            Some(location..location),
                        ));
                    }
                    if names.len() > 2 {
                        return Err(unsupported(
                            "catalog-qualified function name",
                            Some(location..location),
                        ));
                    }
                    let name = ObjectName {
                        schema: Some(names[0].clone()),
                        name: names[1].clone(),
                    };
                    return self.func_call(scope, handle, name, call, location);
                }
                let (names, star, rest) = self.qualified_parts(q_handle)?;
                if star {
                    // t.* as a value: the whole row.
                    let (schema, relname) = match names.as_slice() {
                        [r] => (None, r.as_str()),
                        [s, r] => (Some(s.as_str()), r.as_str()),
                        _ => {
                            return Err(Diagnostic::new(
                                "42601",
                                format!(
                                    "improper qualified name (too many dotted names): {}.*",
                                    names.join(".")
                                ),
                            )
                            .at(location)
                            .into());
                        }
                    };
                    let item = self.relations.refname_namespace_item(
                        scope,
                        &self.outer,
                        schema,
                        relname,
                        location,
                    )?;
                    let Some((levelsup, item)) = item else {
                        return Err(self
                            .relations
                            .error_missing_rte(scope, &self.outer, schema, relname, location)
                            .into());
                    };
                    return self.whole_row(scope, levelsup, &item, Some(handle), location);
                }
                let base = self.column_ref(scope, handle, &names, location)?;
                if rest.is_empty() {
                    return Ok(base);
                }
                self.indirection(scope, handle, base, Some(handle), &rest, location)
            }
            FlatExpr::Func(f) => {
                let f = self.get(*f);
                let pgtk_sql::tokens::Flattype_function_name::Text(t) = self.get(f.name);
                let name = ObjectName {
                    schema: None,
                    name: self.fold(self.text(*t), location)?,
                };
                self.func_call(scope, handle, name, f.tail, location)
            }
            FlatExpr::Parenthesized(p) => {
                let p = self.get(*p);
                let parts = self.opt_indirection_parts(p.indirection);
                if !parts.is_empty() {
                    let base = self.expr(scope, p.expr)?;
                    // transformIndirection locates its errors at the base.
                    let at = base.location;
                    return self.indirection(scope, handle, base, Some(p.expr), &parts, at);
                }
                self.expr(scope, p.expr)
            }
            FlatExpr::ImplicitRow(row) => {
                let elements = self.implicit_row_values(*row);
                self.row_expr(scope, handle, &elements, location)
            }
            FlatExpr::SubSelect(q) => {
                let sub = self.get(*q).inner;
                self.expr_sublink(scope, handle, sub, location)
            }
            FlatExpr::User => self.sql_value(handle, "user", None, location),
            FlatExpr::SqlValue(v) => {
                let (name, precision) = self.sql_value_keyword(*v);
                self.sql_value(handle, name, precision, location)
            }
            FlatExpr::SubSelectIndirection(q) => {
                let q = self.get(*q);
                let sub = self.get(q.query).inner;
                let parts = self.indirection_parts(q.indirection);
                let base = self.expr_sublink(scope, handle, sub, location)?;
                // transformIndirection locates its errors at the base.
                let at = base.location;
                self.indirection(scope, handle, base, None, &parts, at)
            }
            FlatExpr::RowExpr(r) => {
                let elements = self
                    .get(*r)
                    .values
                    .map_or_else(Vec::new, |v| self.list1(v).to_vec());
                self.row_expr(scope, handle, &elements, location)
            }
            FlatExpr::Exists(e) => {
                self.sublinks.insert(handle);
                let sub = self.get(*e).subquery;
                self.sublink_columns(scope, handle, sub)?;
                Ok(Typed {
                    ty: self.types().builtin(Builtin::Bool),
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            FlatExpr::QuantifiedComparisonCmp(l, suffix) => {
                let (l, suffix) = (*l, *suffix);
                let suffix = self.get(suffix);
                self.quantified(
                    scope,
                    handle,
                    l,
                    self.span(suffix.operator),
                    suffix.tail,
                    location,
                )
            }
            FlatExpr::QuantifiedComparisonOp(l, suffix) => {
                let (l, suffix) = (*l, *suffix);
                let suffix = self.get(suffix);
                self.quantified(
                    scope,
                    handle,
                    l,
                    self.span(suffix.operator),
                    suffix.tail,
                    location,
                )
            }
            FlatExpr::QuantifiedComparisonAdd(l, suffix) => {
                let (l, suffix) = (*l, *suffix);
                let suffix = self.get(suffix);
                self.quantified(
                    scope,
                    handle,
                    l,
                    self.span(suffix.operator),
                    suffix.tail,
                    location,
                )
            }
            FlatExpr::QuantifiedComparisonMul(l, suffix) => {
                let (l, suffix) = (*l, *suffix);
                let suffix = self.get(suffix);
                self.quantified(
                    scope,
                    handle,
                    l,
                    self.span(suffix.operator),
                    suffix.tail,
                    location,
                )
            }
            FlatExpr::QuantifiedComparisonPow(l, suffix) => {
                let (l, suffix) = (*l, *suffix);
                let suffix = self.get(suffix);
                self.quantified(
                    scope,
                    handle,
                    l,
                    self.span(suffix.operator),
                    suffix.tail,
                    location,
                )
            }
            FlatExpr::QuantifiedComparisonLike(l, suffix) => {
                let (l, suffix) = (*l, *suffix);
                let suffix = self.get(suffix);
                let name = match self.get(suffix.operator) {
                    FlatQuantifiedLikeOperator::Like => "~~",
                    FlatQuantifiedLikeOperator::NotLike => "!~~",
                    FlatQuantifiedLikeOperator::Ilike => "~~*",
                    FlatQuantifiedLikeOperator::NotIlike => "!~~*",
                };
                let op_location = self.span(suffix.operator).start;
                let name = ObjectName {
                    schema: None,
                    name: name.to_owned(),
                };
                self.quantified_op(scope, handle, l, name, op_location, suffix.tail)
            }
            FlatExpr::Cast(inner, cast_type) => {
                let (inner, cast_type) = (*inner, *cast_type);
                let arg = self.cast_operand(scope, inner, cast_type)?;
                let inner_end = self.span(inner).end;
                let cast_location = self.next_token_start(inner_end);
                self.type_cast(handle, inner, &arg, cast_type, cast_location)
            }
            FlatExpr::CastCall(c) => {
                let c = self.get(*c);
                let inner = self.get(c.inner);
                let (value, target) = (inner.value, inner.target);
                let arg = self.cast_operand(scope, value, target)?;
                self.type_cast(handle, value, &arg, target, location)
            }
            FlatExpr::Treat(t) => {
                // gram.y: TREAT(expr AS type) is a FuncCall named after the
                // type, through `SystemFuncName` of the last part of the
                // type name, with the expression as its only argument.
                let inner = self.get(self.get(*t).inner);
                let (value, target) = (inner.value, inner.target);
                let arg = self.expr(scope, value)?;
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: self.type_name(target)?.name,
                };
                self.resolve_call(handle, name, &[value], &[arg], location, None)
            }
            FlatExpr::CastFunc(f) => self.typed_literal(handle, *f, location),
            FlatExpr::TimestampLit(t) => {
                let t = self.get(*t);
                let tz = matches!(t.tz.map(|z| self.get(z)), Some(FlatTimeZoneQualifier::With));
                let typmods = t
                    .precision
                    .map(|p| vec![TypmodArg::Integer(self.iconst_precision(p))])
                    .unwrap_or_default();
                let tn = TypeName {
                    schema: Some("pg_catalog".to_owned()),
                    name: if tz { "timestamptz" } else { "timestamp" }.to_owned(),
                    typmods,
                    array_dims: 0,
                    location,
                };
                self.string_const_cast(handle, tn, t.value, location)
            }
            FlatExpr::TimeLit(t) => {
                let t = self.get(*t);
                let tz = matches!(t.tz.map(|z| self.get(z)), Some(FlatTimeZoneQualifier::With));
                let typmods = t
                    .precision
                    .map(|p| vec![TypmodArg::Integer(self.iconst_precision(p))])
                    .unwrap_or_default();
                let tn = TypeName {
                    schema: Some("pg_catalog".to_owned()),
                    name: if tz { "timetz" } else { "time" }.to_owned(),
                    typmods,
                    array_dims: 0,
                    location,
                };
                self.string_const_cast(handle, tn, t.value, location)
            }
            FlatExpr::IntervalLit(i) => {
                let i = self.get(*i);
                // gram.y: INTERVAL '(' Iconst ')' Sconst carries
                // [FULL_RANGE, precision]; INTERVAL Sconst opt_interval carries
                // the qualifier's [fields] or [fields, precision].
                let mut typmods = Vec::new();
                let value = match self.get(i.form) {
                    FlatIntervalLitForm::Precision(p) => {
                        let p = self.get(*p);
                        typmods.push(TypmodArg::Integer(0x7fff));
                        typmods.push(TypmodArg::Integer(self.iconst_precision(p.precision)));
                        p.value
                    }
                    FlatIntervalLitForm::Qualified(q) => {
                        let q = self.get(*q);
                        if let Some(qualifier) = q.qualifier {
                            let (fields, precision) = self.interval_qualifier(qualifier);
                            typmods.push(TypmodArg::Integer(i64::from(fields)));
                            if let Some(p) = precision {
                                typmods.push(TypmodArg::Integer(p));
                            }
                        }
                        q.value
                    }
                };
                let tn = TypeName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "interval".to_owned(),
                    typmods,
                    array_dims: 0,
                    location,
                };
                self.string_const_cast(handle, tn, value, location)
            }
            FlatExpr::Array(a) if matches!(self.get(*a), FlatArrayExpr::Subquery(_)) => {
                let FlatArrayExpr::Subquery(sub) = self.get(*a) else {
                    unreachable!("matched above");
                };
                self.sublinks.insert(handle);
                let sub = self.get(*sub).subquery;
                let columns = self.sublink_columns(scope, handle, sub)?;
                if columns.len() != 1 {
                    return Err(
                        Diagnostic::new("42601", "subquery must return only one column")
                            .at(location)
                            .into(),
                    );
                }
                // get_promoted_array_type: an array stays its own type.
                let element = columns[0].ty;
                let promoted = self
                    .types()
                    .array_type(element)
                    .or_else(|| self.types().element_type(element).map(|_| element));
                let Some(array) = promoted else {
                    return Err(Diagnostic::new(
                        "42704",
                        format!(
                            "could not find array type for data type {}",
                            self.types().format(element)
                        ),
                    )
                    .into());
                };
                Ok(Typed {
                    ty: array,
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            FlatExpr::Array(a) => match self.get(*a) {
                FlatArrayExpr::Subquery(_) => {
                    Err(unsupported("ARRAY(subquery)", Some(location..location)))
                }
                FlatArrayExpr::Bracket(b) => {
                    let elements = self.array_items(self.get(*b).elements);
                    self.array_expr(scope, &elements, location, None)
                }
            },
            FlatExpr::Case(c) => {
                let body = self.get(*c).body;
                self.case_expr(scope, body, location)
            }
            FlatExpr::InExpr(l, list) => {
                let (l, list) = (*l, *list);
                let content = self.get(list).0;
                let op_location = self.next_token_start(self.span(l).end);
                let items = match self.get(content) {
                    FlatInContent::Subquery(sub) => {
                        let eq = ObjectName {
                            schema: None,
                            name: "=".to_owned(),
                        };
                        return self.sublink_comparison(scope, handle, l, eq, op_location, *sub);
                    }
                    FlatInContent::Exprs(list) => self.list1(*list).to_vec(),
                };
                self.in_list(scope, handle, l, &items, false, op_location)
            }
            FlatExpr::NotInExpr(l, suffix) => {
                let (l, suffix) = (*l, *suffix);
                let content = self.get(self.get(suffix).list).0;
                let op_location = self.next_token_start(self.span(l).end);
                let items = match self.get(content) {
                    FlatInContent::Subquery(sub) => {
                        let eq = ObjectName {
                            schema: None,
                            name: "=".to_owned(),
                        };
                        return self.sublink_comparison(scope, handle, l, eq, op_location, *sub);
                    }
                    FlatInContent::Exprs(list) => self.list1(*list).to_vec(),
                };
                self.in_list(scope, handle, l, &items, true, op_location)
            }
            FlatExpr::Extract(e) => {
                let inner = self.get(self.get(*e).inner);
                let field = match self.get(inner.field) {
                    FlatExtractField::StringLit(s) => match self.get(*s) {
                        FlatStringLitSeq0::Single(t) => string_literal_body(self.text(*t)),
                        FlatStringLitSeq0::Sequence(t) => string_literal_body(self.text(*t)),
                    },
                    FlatExtractField::Ident(i) => {
                        let FlatAliasName::Text(t) = self.get(*i);
                        let raw = self.text(*t);
                        // extract_arg: IDENT keeps its spelling; the keywords
                        // arrive lower-cased.
                        if raw.starts_with('"') {
                            self.fold(raw, location)?
                        } else {
                            raw.to_owned()
                        }
                    }
                };
                let source = inner.source;
                let st = self.expr(scope, source)?;
                let field_const = make_const(self.types(), LiteralKind::String, &field, location);
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "extract".to_owned(),
                };
                self.resolve_call(
                    handle,
                    name,
                    &[handle, source],
                    &[field_const, st],
                    location,
                    None,
                )
            }
            FlatExpr::Position(p) => {
                let inner = self.get(self.get(*p).inner);
                let (needle, haystack) = (inner.needle, inner.haystack);
                let nt = self.expr(scope, needle)?;
                let ht = self.expr(scope, haystack)?;
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "position".to_owned(),
                };
                // position_list: b_expr IN b_expr -> (haystack, needle).
                self.resolve_call(handle, name, &[haystack, needle], &[ht, nt], location, None)
            }
            FlatExpr::Overlay(o) => {
                let inner = self.get(self.get(*o).inner);
                let mut handles = vec![inner.source, inner.new, inner.start];
                if let Some(f) = inner.for_count {
                    handles.push(self.get(f).count);
                }
                let mut args = Vec::new();
                for &h in &handles {
                    args.push(self.expr(scope, h)?);
                }
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "overlay".to_owned(),
                };
                self.resolve_call(handle, name, &handles, &args, location, None)
            }
            FlatExpr::Substring(s) => {
                let inner = self.get(self.get(*s).inner);
                let source = inner.source;
                let mut handles = vec![source];
                let mut extra_int4_cast: Option<usize> = None;
                match self.get(inner.tail) {
                    FlatSubstringTail::FromFor(ff) => {
                        let ff = self.get(*ff);
                        handles.push(ff.start);
                        if let Some(f) = ff.for_count {
                            handles.push(self.get(f).count);
                        }
                    }
                    FlatSubstringTail::For(f) => {
                        // substr_list: a_expr FOR a_expr -> (a, 1, cast(b as int4)).
                        let count = self.get(*f).count;
                        handles.push(count);
                        extra_int4_cast = Some(1);
                    }
                    FlatSubstringTail::Similar(sim) => {
                        let sim = self.get(*sim);
                        handles.push(sim.pattern);
                        handles.push(sim.escape);
                    }
                    FlatSubstringTail::Args(list) => {
                        for a in self.list1(*list) {
                            handles.push(self.get(*a).value);
                        }
                    }
                }
                let mut args = Vec::new();
                for &h in &handles {
                    args.push(self.expr(scope, h)?);
                }
                if let Some(i) = extra_int4_cast {
                    let int4 = self.types().builtin(Builtin::Int4);
                    let count = args.remove(i);
                    let coerced = self.coercer().coerce_to_target_type(
                        &count,
                        int4,
                        -1,
                        CoercionContext::Explicit,
                    )?;
                    let Some((count_typed, steps)) = coerced else {
                        return Err(Diagnostic::new(
                            "42846",
                            format!(
                                "cannot cast type {} to {}",
                                self.types().format(count.ty),
                                "integer"
                            ),
                        )
                        .at(count.location)
                        .into());
                    };
                    self.record_coercions(handles[i], steps);
                    let one = make_const(self.types(), LiteralKind::Integer, "1", location);
                    args.insert(1, one);
                    args.push(count_typed);
                    handles.insert(1, handle);
                }
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "substring".to_owned(),
                };
                self.resolve_call(handle, name, &handles, &args, location, None)
            }
            FlatExpr::Trim(t) => {
                let inner = self.get(self.get(*t).inner);
                let fname = match inner.dir.map(|d| self.get(d)) {
                    Some(FlatTrimDir::Leading) => "ltrim",
                    Some(FlatTrimDir::Trailing) => "rtrim",
                    _ => "btrim",
                };
                // trim_list: `a FROM list` -> list ++ [a]; `FROM list` -> list; `list`.
                let mut handles: Vec<Handle<FlatExpr>> = Vec::new();
                match self.get(inner.tail) {
                    FlatTrimTail::FromArgs(f) => handles.extend(self.list1(self.get(*f).args)),
                    FlatTrimTail::Values(v) => {
                        let v = self.get(*v);
                        match v.rest.map(|r| self.get(r)) {
                            None => handles.push(v.first),
                            Some(FlatTrimValuesRest::From(f)) => {
                                handles.extend(self.list1(self.get(*f).args));
                                handles.push(v.first);
                            }
                            Some(FlatTrimValuesRest::More(m)) => {
                                handles.push(v.first);
                                for a in self.list1(self.get(*m).0) {
                                    handles.push(self.get(*a).value);
                                }
                            }
                        }
                    }
                }
                let mut args = Vec::new();
                for &h in &handles {
                    args.push(self.expr(scope, h)?);
                }
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: fname.to_owned(),
                };
                self.resolve_call(handle, name, &handles, &args, location, None)
            }
            FlatExpr::And(l, r) | FlatExpr::Or(l, r) => {
                let (l, r) = (*l, *r);
                let construct = if matches!(node, FlatExpr::And(..)) {
                    "AND"
                } else {
                    "OR"
                };
                let lt = self.expr(scope, l)?;
                let (_, steps) = self.coercer().coerce_to_boolean(&lt, construct)?;
                self.record_coercions(l, steps);
                let rt = self.expr(scope, r)?;
                let (_, steps) = self.coercer().coerce_to_boolean(&rt, construct)?;
                self.record_coercions(r, steps);
                Ok(self.bool_result(location))
            }
            FlatExpr::Not(inner) => {
                let inner = *inner;
                let t = self.expr(scope, inner)?;
                let (_, steps) = self.coercer().coerce_to_boolean(&t, "NOT")?;
                self.record_coercions(inner, steps);
                Ok(self.bool_result(location))
            }
            // transformAExprBetween: `a BETWEEN b AND c` is `a >= b AND a <= c`;
            // `a NOT BETWEEN b AND c` is `a < b OR a > c`. Both operators take
            // the position of the keyword after `a`.
            FlatExpr::BetweenExpr(a, b, c) | FlatExpr::NotBetweenExpr(a, b, c) => {
                let (a, low, c) = (*a, self.get(*b), *c);
                let b = low.low;
                // SYMMETRIC also compares against the bounds swapped.
                let symmetric = low
                    .modifier
                    .is_some_and(|m| matches!(self.get(m), FlatBetweenModifier::Symmetric));
                let negated = matches!(node, FlatExpr::NotBetweenExpr(..));
                let at = self.expr(scope, a)?;
                let bt = self.expr(scope, b)?;
                let ct = self.expr(scope, c)?;
                let (_, op_location) = self.operator_text(self.span(a).end, self.span(b).start);
                let (first, second, construct) = if negated {
                    ("<", ">", "OR")
                } else {
                    (">=", "<=", "AND")
                };
                let name = |n: &str| ObjectName {
                    schema: None,
                    name: n.to_owned(),
                };
                let t1 = self.infix_op(handle, name(first), op_location, a, &at, b, &bt)?;
                self.coercer().coerce_to_boolean(&t1, construct)?;
                let t2 = self.infix_op(handle, name(second), op_location, a, &at, c, &ct)?;
                self.coercer().coerce_to_boolean(&t2, construct)?;
                if symmetric {
                    let t3 = self.infix_op(handle, name(first), op_location, a, &at, c, &ct)?;
                    self.coercer().coerce_to_boolean(&t3, construct)?;
                    let t4 = self.infix_op(handle, name(second), op_location, a, &at, b, &bt)?;
                    self.coercer().coerce_to_boolean(&t4, construct)?;
                }
                Ok(self.bool_result(location))
            }
            #[cfg(feature = "since-pg16")]
            FlatExpr::JsonObject(o) => self.json_object(scope, handle, *o, location),
            #[cfg(feature = "since-pg16")]
            FlatExpr::JsonArray(a) => self.json_array(scope, handle, *a, location),
            #[cfg(feature = "since-pg16")]
            FlatExpr::JsonObjectAgg(a) => self.json_object_agg(scope, handle, *a, location),
            #[cfg(feature = "since-pg16")]
            FlatExpr::JsonArrayAgg(a) => self.json_array_agg(scope, handle, *a, location),
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonCtor(c) => self.json_parse(scope, *c, location),
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonScalar(j) => {
                let j: &FlatJsonScalar = self.get(*j);
                let t = self.expr(scope, j.inner)?;
                if self.types().is_unknown(t.ty) {
                    let text = self.types().builtin(Builtin::Text);
                    self.coerce_to_specific_type(j.inner, &t, text, "JSON_SCALAR")?;
                }
                Ok(Typed {
                    ty: self.types().builtin(Builtin::Json),
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonSerialize(j) => self.json_serialize(scope, *j, location),
            #[cfg(feature = "since-pg17")]
            FlatExpr::MergeAction(_) => self.merge_action(location),
            #[cfg(feature = "since-pg16")]
            FlatExpr::IsJson(e, _) => {
                let e = *e;
                let (ty, _) = self.json_parse_arg(scope, e, None)?;
                let types = self.types();
                if !(types.is(ty, Builtin::Text)
                    || types.is(ty, Builtin::Json)
                    || types.is(ty, Builtin::Jsonb))
                {
                    return Err(Diagnostic::new(
                        "42804",
                        format!("cannot use type {} in IS JSON predicate", types.format(ty)),
                    )
                    .into());
                }
                Ok(self.bool_result(location))
            }
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonExists(x) => {
                let x: &FlatJsonExists = self.get(*x);
                let i = self.get(x.inner);
                let parts = JsonFuncParts {
                    context: JsonContext::Expr(i.context, i.context_format),
                    path: Some(i.path),
                    passing: i.passing,
                    returning: JsonReturningSpec::None,
                    wrapper: None,
                    quotes: None,
                    on_empty: None,
                    on_error: i.on_error.map(|e| self.json_on_error(e)),
                    column: None,
                };
                self.json_func(scope, JsonOp::Exists, &parts, location)
            }
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonQuery(x) => {
                let x: &FlatJsonQuery = self.get(*x);
                let i = self.get(x.inner);
                let parts = JsonFuncParts {
                    context: JsonContext::Expr(i.context, i.context_format),
                    path: Some(i.path),
                    passing: i.passing,
                    returning: i
                        .returning
                        .map_or(JsonReturningSpec::None, JsonReturningSpec::Clause),
                    wrapper: i.wrapper,
                    quotes: i.quotes,
                    on_empty: self.json_behaviors(i.on_behavior).0,
                    on_error: self.json_behaviors(i.on_behavior).1,
                    column: None,
                };
                self.json_func(scope, JsonOp::Query, &parts, location)
            }
            #[cfg(feature = "since-pg17")]
            FlatExpr::JsonValue(x) => {
                let x: &FlatJsonValue = self.get(*x);
                let i = self.get(x.inner);
                let parts = JsonFuncParts {
                    context: JsonContext::Expr(i.context, i.context_format),
                    path: Some(i.path),
                    passing: i.passing,
                    returning: i
                        .returning
                        .map_or(JsonReturningSpec::None, JsonReturningSpec::Clause),
                    wrapper: None,
                    quotes: None,
                    on_empty: self.json_behaviors(i.on_behavior).0,
                    on_error: self.json_behaviors(i.on_behavior).1,
                    column: None,
                };
                self.json_func(scope, JsonOp::Value, &parts, location)
            }
            FlatExpr::XmlParse(x) => {
                let x: &FlatXmlParse = self.get(*x);
                let inner = self.get(x.inner);
                let t = self.expr(scope, inner.value)?;
                let text = self.types().builtin(Builtin::Text);
                self.coerce_to_specific_type(inner.value, &t, text, "XMLPARSE")?;
                Ok(self.xml_result(location))
            }
            FlatExpr::XmlPi(x) => {
                let x: &FlatXmlPi = self.get(*x);
                let inner = self.get(x.inner);
                if let Some(c) = inner.content {
                    let e = self.get(c).expr;
                    let t = self.expr(scope, e)?;
                    let text = self.types().builtin(Builtin::Text);
                    self.coerce_to_specific_type(e, &t, text, "XMLPI")?;
                }
                Ok(self.xml_result(location))
            }
            FlatExpr::XmlRoot(x) => {
                let x: &FlatXmlRoot = self.get(*x);
                let inner = self.get(x.inner);
                let t = self.expr(scope, inner.value)?;
                let xml = self.types().builtin(Builtin::Xml);
                self.coerce_to_specific_type(inner.value, &t, xml, "XMLROOT")?;
                let version: &FlatXmlRootVersion = self.get(inner.version);
                if let FlatXmlVersionValue::Expr(e) = self.get(version.value) {
                    let t = self.expr(scope, *e)?;
                    let text = self.types().builtin(Builtin::Text);
                    self.coerce_to_specific_type(*e, &t, text, "XMLROOT")?;
                }
                Ok(self.xml_result(location))
            }
            FlatExpr::XmlElement(x) => {
                let x: &FlatXmlElement = self.get(*x);
                let inner = self.get(x.inner);
                let (attrs, content) = match inner.tail.map(|t| self.get(t)) {
                    None => (None, None),
                    Some(FlatXmlElementTail::Attrs(t)) => {
                        let t = self.get(*t);
                        (Some(t.attrs), t.content)
                    }
                    Some(FlatXmlElementTail::Content(c)) => (None, Some(*c)),
                };
                let mut names: Vec<String> = Vec::new();
                if let Some(attrs) = attrs {
                    for arg in self.list1(self.get(attrs).args).to_vec() {
                        let name = self.xml_named_arg(scope, arg, true)?;
                        if names.contains(&name) {
                            return Err(Diagnostic::new(
                                "42601",
                                format!("XML attribute name \"{name}\" appears more than once"),
                            )
                            .at(self.span(arg).start)
                            .into());
                        }
                        names.push(name);
                    }
                }
                let exprs = content
                    .map(|c| self.list1(self.get(c).exprs).to_vec())
                    .unwrap_or_default();
                for e in exprs {
                    // Content is converted at execution, whatever its type.
                    self.expr(scope, e)?;
                }
                Ok(self.xml_result(location))
            }
            FlatExpr::XmlForest(x) => {
                let x: &FlatXmlForest = self.get(*x);
                for arg in self.list1(x.args).to_vec() {
                    self.xml_named_arg(scope, arg, false)?;
                }
                Ok(self.xml_result(location))
            }
            FlatExpr::XmlExists(x) => {
                // gram.y: XMLEXISTS is the function xmlexists(text, xml).
                let x: &FlatXmlExists = self.get(*x);
                let inner = self.get(x.inner);
                let passing = self.get(inner.passing);
                let doc = match self.get(passing.document) {
                    FlatXmlExistsDocument::Plain(d) => *d,
                    FlatXmlExistsDocument::Qualified(q) => self.get(*q).doc,
                };
                let handles = [inner.xpath, doc];
                let mut args = Vec::with_capacity(2);
                for &h in &handles {
                    args.push(self.expr(scope, h)?);
                }
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "xmlexists".to_owned(),
                };
                self.resolve_call(handle, name, &handles, &args, location, None)
            }
            FlatExpr::XmlSerialize(x) => {
                let x: &FlatXmlSerialize = self.get(*x);
                let inner = self.get(x.inner);
                let t = self.expr(scope, inner.value)?;
                let xml = self.types().builtin(Builtin::Xml);
                self.coerce_to_specific_type(inner.value, &t, xml, "XMLSERIALIZE")?;
                let tn = self.simple_type_name(inner.ty, 0, self.span(inner.ty).start)?;
                let (target, target_typmod) = self.typenames.resolve(&tn)?;
                let text_value = Typed {
                    ty: self.types().builtin(Builtin::Text),
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                };
                if self
                    .coercer()
                    .coerce_to_target_type(
                        &text_value,
                        target,
                        target_typmod,
                        CoercionContext::Implicit,
                    )?
                    .is_none()
                {
                    return Err(Diagnostic::new(
                        "42846",
                        format!(
                            "cannot cast XMLSERIALIZE result to {}",
                            self.types().format(target)
                        ),
                    )
                    .at(location)
                    .into());
                }
                Ok(Typed {
                    ty: target,
                    typmod: target_typmod,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            FlatExpr::IsDocument(e, _) => {
                let e = *e;
                let t = self.expr(scope, e)?;
                let xml = self.types().builtin(Builtin::Xml);
                self.coerce_to_specific_type(e, &t, xml, "IS DOCUMENT")?;
                Ok(self.bool_result(location))
            }
            // transformGroupingFunc: like an aggregate of this level.
            FlatExpr::Grouping(g) => {
                let args = self.list1(self.get(*g).args).to_vec();
                if args.len() > 31 {
                    return Err(Diagnostic::new(
                        "54023",
                        "GROUPING must have fewer than 32 arguments",
                    )
                    .at(location)
                    .into());
                }
                let outer_before = self.outer_vars.len();
                let agg_before = self.agg_refs.len();
                let cte_before = self.cte_refs.len();
                for &a in &args {
                    self.expr(scope, a)?;
                }
                // check_agglevels_and_constraints: the level of the
                // columns in the arguments.
                let level = self.level();
                let agg_level = self.check_agg_arguments(
                    &[],
                    &args,
                    outer_before,
                    agg_before,
                    cte_before,
                    location,
                )?;
                if agg_level < level {
                    let outer_args: Vec<(Handle<FlatExpr>, Option<Var>)> = args
                        .iter()
                        .map(|&a| {
                            let at = self.span(a).start;
                            let var = self.outer_vars[outer_before..]
                                .iter()
                                .find(|r| r.level == agg_level && r.location == at)
                                .map(|r| r.var);
                            (a, var)
                        })
                        .collect();
                    self.outer_grouping_calls.push(OuterGrouping {
                        level: agg_level,
                        args: outer_args,
                    });
                }
                let kept: Vec<OuterRef> = self.outer_vars[outer_before..]
                    .iter()
                    .filter(|r| r.level != agg_level)
                    .copied()
                    .collect();
                self.outer_vars.truncate(outer_before);
                self.outer_vars.extend(kept);
                self.agg_level = (agg_level < level).then_some(agg_level);
                let registered = self.register_aggregate(location, true);
                self.agg_level = None;
                registered?;
                self.agg_refs.push(AggRef {
                    level: agg_level,
                    location,
                });
                if agg_level == level {
                    self.grouping_calls.push((handle, args));
                }
                Ok(Typed {
                    ty: self.types().builtin(Builtin::Int4),
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            // gram.y: `a SIMILAR TO b [ESCAPE e]` is `a ~ similar_to_escape(b[, e])`.
            FlatExpr::SimilarTo(a, b, esc) | FlatExpr::NotSimilarTo(a, b, esc) => {
                let negated = matches!(node, FlatExpr::NotSimilarTo(..));
                let (a, b, esc) = (*a, *b, *esc);
                let at = self.expr(scope, a)?;
                let mut handles = vec![b];
                handles.extend(esc);
                let mut args = Vec::with_capacity(handles.len());
                for &h in &handles {
                    args.push(self.expr(scope, h)?);
                }
                let (_, op_location) = self.operator_text(self.span(a).end, self.span(b).start);
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "similar_to_escape".to_owned(),
                };
                let pattern =
                    self.resolve_call(handle, name, &handles, &args, op_location, None)?;
                let op = ObjectName {
                    schema: None,
                    name: if negated { "!~" } else { "~" }.to_owned(),
                };
                let resolved = self
                    .resolver
                    .make_op(&op, Some(&at), &pattern, op_location)?;
                self.record_coercions(a, resolved.left_coercions);
                Ok(Typed {
                    location: at.location,
                    ..resolved.result
                })
            }
            // gram.y: `row OVERLAPS row` is the function overlaps(l1, l2, r1, r2).
            FlatExpr::Overlaps(l, r) => {
                let (l, r) = (*l, *r);
                let left = self.overlaps_row_values(l);
                let right = self.overlaps_row_values(r);
                if left.len() != 2 {
                    return Err(Diagnostic::new(
                        "42601",
                        "wrong number of parameters on left side of OVERLAPS expression",
                    )
                    .at(self.span(l).start)
                    .into());
                }
                if right.len() != 2 {
                    return Err(Diagnostic::new(
                        "42601",
                        "wrong number of parameters on right side of OVERLAPS expression",
                    )
                    .at(self.span(r).start)
                    .into());
                }
                let handles = [left[0], left[1], right[0], right[1]];
                let mut args = Vec::with_capacity(4);
                for &h in &handles {
                    args.push(self.expr(scope, h)?);
                }
                let (_, op_location) = self.operator_text(self.span(l).end, self.span(r).start);
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "overlaps".to_owned(),
                };
                self.resolve_call(handle, name, &handles, &args, op_location, None)
            }
            FlatExpr::Isnull(inner) | FlatExpr::Notnull(inner) => {
                let inner = *inner;
                let t = self.expr(scope, inner)?;
                if t.returns_set {
                    return Err(unsupported(
                        "set-returning NullTest argument",
                        Some(location..location),
                    ));
                }
                Ok(self.bool_result(location))
            }
            FlatExpr::BoolTest(inner, kind) => {
                let (inner, kind) = (*inner, *kind);
                let t = self.expr(scope, inner)?;
                let construct = match self.get(kind) {
                    FlatBoolTestKind::IsTrue => "IS TRUE",
                    FlatBoolTestKind::IsNotTrue => "IS NOT TRUE",
                    FlatBoolTestKind::IsFalse => "IS FALSE",
                    FlatBoolTestKind::IsNotFalse => "IS NOT FALSE",
                    FlatBoolTestKind::IsUnknown => "IS UNKNOWN",
                    FlatBoolTestKind::IsNotUnknown => "IS NOT UNKNOWN",
                    FlatBoolTestKind::IsNull | FlatBoolTestKind::IsNotNull => {
                        return Ok(self.bool_result(location));
                    }
                };
                let (_, steps) = self.coercer().coerce_to_boolean(&t, construct)?;
                self.record_coercions(inner, steps);
                Ok(self.bool_result(location))
            }
            FlatExpr::Like(l, r, esc)
            | FlatExpr::Ilike(l, r, esc)
            | FlatExpr::NotLike(l, r, esc)
            | FlatExpr::NotIlike(l, r, esc) => {
                let (l, r, esc) = (*l, *r, *esc);
                let op = match node {
                    FlatExpr::Like(..) => "~~",
                    FlatExpr::Ilike(..) => "~~*",
                    FlatExpr::NotLike(..) => "!~~",
                    _ => "!~~*",
                };
                let lt = self.expr(scope, l)?;
                let rt = self.expr(scope, r)?;
                let op_location = self.next_token_start(self.span(l).end);
                let rt = match esc {
                    None => rt,
                    Some(esc) => {
                        // gram.y: the pattern becomes like_escape(pattern, escape).
                        let et = self.expr(scope, esc)?;
                        let name = ObjectName {
                            schema: Some("pg_catalog".to_owned()),
                            name: "like_escape".to_owned(),
                        };
                        let escaped =
                            self.resolve_call(r, name, &[r, esc], &[rt, et], op_location, None)?;
                        Typed {
                            location: escaped.location,
                            ..escaped
                        }
                    }
                };
                self.infix_op(
                    handle,
                    ObjectName {
                        schema: None,
                        name: op.to_owned(),
                    },
                    op_location,
                    l,
                    &lt,
                    r,
                    &rt,
                )
            }
            FlatExpr::IsDistinctFrom(l, r) | FlatExpr::IsNotDistinctFrom(l, r) => {
                let (l, r) = (*l, *r);
                let lt = self.expr(scope, l)?;
                let rt = self.expr(scope, r)?;
                let op_location = self.next_token_start(self.span(l).end);
                // make_distinct_op: the "=" operator, result forced to boolean.
                let resolved = self.resolver.make_op(
                    &ObjectName {
                        schema: None,
                        name: "=".to_owned(),
                    },
                    Some(&lt),
                    &rt,
                    op_location,
                )?;
                if !self.types().is(resolved.result.ty, Builtin::Bool) {
                    return Err(Diagnostic::new(
                        "42804",
                        "IS DISTINCT FROM requires = operator to yield boolean",
                    )
                    .at(op_location)
                    .into());
                }
                self.analysis.operators.insert(handle, resolved.operator);
                self.record_coercions(l, resolved.left_coercions);
                self.record_coercions(r, resolved.right_coercions);
                Ok(self.bool_result(location))
            }
            FlatExpr::CustomInfix(l, op, r) => {
                let (l, r, op) = (*l, *r, *op);
                let lt = self.expr(scope, l)?;
                let rt = self.expr(scope, r)?;
                let op_span = self.text_span(op);
                let name = ObjectName {
                    schema: None,
                    name: self.text(op).to_owned(),
                };
                self.infix_op(handle, name, op_span.start, l, &lt, r, &rt)
            }
            FlatExpr::CustomPrefix(op, inner) => {
                let (op, inner) = (*op, *inner);
                let arg = self.expr(scope, inner)?;
                let op_span = self.text_span(op);
                let name = self.text(op).to_owned();
                self.prefix_op(handle, &name, op_span.start, inner, &arg)
            }
            FlatExpr::DecoratedInfix(l, op, r) => {
                let (l, r, op) = (*l, *r, *op);
                let lt = self.expr(scope, l)?;
                let rt = self.expr(scope, r)?;
                let (name, op_location) = self.decorated_operator(op)?;
                self.infix_op(handle, name, op_location, l, &lt, r, &rt)
            }
            FlatExpr::DecoratedPrefix(op, inner) => {
                let (op, inner) = (*op, *inner);
                let arg = self.expr(scope, inner)?;
                let (name, op_location) = self.decorated_operator(op)?;
                self.prefix_op_named(handle, name, op_location, inner, &arg)
            }
            FlatExpr::AtTimeZone(l, r) => {
                // gram.y: timezone(rhs, lhs), located at AT.
                let (l, r) = (*l, *r);
                let lt = self.expr(scope, l)?;
                let rt = self.expr(scope, r)?;
                let at = self.next_token_start(self.span(l).end);
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "timezone".to_owned(),
                };
                let typed =
                    self.resolve_call(handle, name, &[r, l], &[rt, lt.clone()], at, None)?;
                Ok(Typed {
                    location: lt.location,
                    ..typed
                })
            }
            #[cfg(feature = "since-pg17")]
            FlatExpr::AtLocal(l) => {
                let l = *l;
                let lt = self.expr(scope, l)?;
                let at = self.next_token_start(self.span(l).end);
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "timezone".to_owned(),
                };
                let typed =
                    self.resolve_call(handle, name, &[l], std::slice::from_ref(&lt), at, None)?;
                Ok(Typed {
                    location: lt.location,
                    ..typed
                })
            }
            FlatExpr::XmlConcat(x) => {
                // transformXmlExpr for IS_XMLCONCAT: every argument becomes xml.
                let args = self.list1(self.get(*x).args).to_vec();
                let xml = self.types().builtin(Builtin::Xml);
                for a in args {
                    let t = self.expr(scope, a)?;
                    if t.ty != xml {
                        let coerced = self.coercer().coerce_to_target_type(
                            &t,
                            xml,
                            -1,
                            CoercionContext::Assignment,
                        )?;
                        match coerced {
                            Some((_, steps)) => self.record_coercions(a, steps),
                            None => {
                                return Err(Diagnostic::new(
                                    "42804",
                                    format!(
                                        "argument of XMLCONCAT must be type xml, not type {}",
                                        self.types().format(t.ty)
                                    ),
                                )
                                .at(t.location)
                                .into());
                            }
                        }
                    }
                }
                Ok(Typed {
                    ty: xml,
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            FlatExpr::Normalize(n) => {
                // gram.y: pg_catalog.normalize(arg [, 'FORM']) as a FuncCall.
                let n = self.get(*n);
                let arg_handle = n.arg;
                let arg = self.expr(scope, arg_handle)?;
                let mut args = vec![arg];
                let mut handles = vec![arg_handle];
                if let Some(form) = n.form {
                    let form = match self.get(form) {
                        pgtk_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfc => "NFC",
                        pgtk_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfd => "NFD",
                        pgtk_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfkc => "NFKC",
                        pgtk_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfkd => "NFKD",
                    };
                    args.push(make_const(
                        self.types(),
                        LiteralKind::String,
                        form,
                        location,
                    ));
                    // The form is a constant with no node of its own; its
                    // coercion is recorded on the expression itself.
                    handles.push(handle);
                }
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "normalize".to_owned(),
                };
                self.resolve_call(handle, name, &handles, &args, location, None)
            }
            FlatExpr::CollationFor(c) => {
                // gram.y: pg_catalog.pg_collation_for(arg) as a FuncCall.
                let arg_handle = self.get(*c).arg;
                let arg = self.expr(scope, arg_handle)?;
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "pg_collation_for".to_owned(),
                };
                self.resolve_call(handle, name, &[arg_handle], &[arg], location, None)
            }
            FlatExpr::IsNormalized(e, tail) => {
                // gram.y: pg_catalog.is_normalized(arg [, 'FORM']) as a
                // FuncCall located at IS, under NOT for IS NOT NORMALIZED.
                let arg_handle = *e;
                let arg = self.expr(scope, arg_handle)?;
                let location = self.next_token_start(self.span(arg_handle).end);
                let mut args = vec![arg];
                let mut handles = vec![arg_handle];
                let form = match self.get(*tail) {
                    FlatIsNormalizedTail::Plain | FlatIsNormalizedTail::Not => None,
                    FlatIsNormalizedTail::Form(f) => Some(self.get(*f).form),
                    FlatIsNormalizedTail::NotForm(f) => Some(self.get(*f).form),
                };
                if let Some(form) = form {
                    let form = match self.get(form) {
                        pgtk_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfc => "NFC",
                        pgtk_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfd => "NFD",
                        pgtk_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfkc => "NFKC",
                        pgtk_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfkd => "NFKD",
                    };
                    args.push(make_const(
                        self.types(),
                        LiteralKind::String,
                        form,
                        location,
                    ));
                    handles.push(handle);
                }
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "is_normalized".to_owned(),
                };
                let t = self.resolve_call(handle, name, &handles, &args, location, None)?;
                Ok(Typed {
                    ty: self.types().builtin(Builtin::Bool),
                    typmod: -1,
                    form: Form::Other,
                    location: t.location,
                    returns_set: t.returns_set,
                })
            }
            FlatExpr::Collate(arg, name) => {
                // transformCollateClause: a collatable argument and a
                // collation the Search Path finds for the encoding.
                let t = self.expr(scope, *arg)?;
                let at = self.next_token_start(self.span(*arg).end);
                if !self.types().is_unknown(t.ty) && self.type_collation(t.ty).is_none() {
                    return Err(Diagnostic::new(
                        "42804",
                        format!(
                            "collations are not supported by type {}",
                            self.types().format(t.ty)
                        ),
                    )
                    .at(at)
                    .into());
                }
                let c = self.lookup_collation(*name, at)?;
                self.collate_exprs.insert(handle, (c, at));
                // coerce_type looks through a CollateExpr to its argument,
                // so an unknown literal under COLLATE still reads as one.
                Ok(Typed {
                    ty: t.ty,
                    typmod: t.typmod,
                    form: t.form,
                    location: t.location,
                    returns_set: t.returns_set,
                })
            }
            FlatExpr::Coalesce(c) => {
                let args = self.list1(self.get(*c).args).to_vec();
                self.common_type_call(scope, handle, &args, "COALESCE", location)
            }
            FlatExpr::Greatest(c) => {
                let args = self.list1(self.get(*c).args).to_vec();
                self.common_type_call(scope, handle, &args, "GREATEST", location)
            }
            FlatExpr::Least(c) => {
                let args = self.list1(self.get(*c).args).to_vec();
                self.common_type_call(scope, handle, &args, "LEAST", location)
            }
            FlatExpr::NullIf(n) => {
                let n = self.get(*n);
                let (l, r) = (n.left, n.right);
                let lt = self.expr(scope, l)?;
                let rt = self.expr(scope, r)?;
                let resolved = self.resolver.make_op(
                    &ObjectName {
                        schema: None,
                        name: "=".to_owned(),
                    },
                    Some(&lt),
                    &rt,
                    location,
                )?;
                if !self.types().is(resolved.result.ty, Builtin::Bool) {
                    return Err(Diagnostic::new(
                        "42804",
                        "NULLIF requires = operator to yield boolean",
                    )
                    .at(location)
                    .into());
                }
                if resolved.result.returns_set {
                    return Err(Diagnostic::new(
                        "42804",
                        "NULLIF requires = operator not to return a set",
                    )
                    .at(location)
                    .into());
                }
                self.analysis.operators.insert(handle, resolved.operator);
                // The result has the type of the (coerced) first argument.
                let left_type = resolved
                    .left_coercions
                    .last()
                    .map_or((lt.ty, lt.typmod), |c| (c.target, c.typmod));
                self.record_coercions(l, resolved.left_coercions);
                self.record_coercions(r, resolved.right_coercions);
                Ok(Typed {
                    ty: left_type.0,
                    typmod: left_type.1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            // Every remaining variant with two operand handles and no other
            // payload is a fixed-spelling operator; its text is read from
            // the source between the operands.
            _ => self.fixed_operator(scope, handle, location),
        }
    }

    /// Binary and prefix operators with fixed spellings, read from the
    /// source text between or before their operands.
    fn fixed_operator(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        location: usize,
    ) -> Result<Typed> {
        let node = self.get(handle);
        let operands = fixed_operands(node);
        // An `if let` chain, not an `if let` guard on the match below: the guards need a Rust
        // newer than the rust-version of the crate.
        if let Some((Some(l), r)) = operands
            && self.row_elements(l).is_some()
            && let FlatExpr::SubSelect(q) = self.get(r)
        {
            // transformAExprOp: "row op subselect" is a ROWCOMPARE sublink.
            let sub = self.get(*q).inner;
            let (name, op_location) = self.operator_text(self.span(l).end, self.span(r).start);
            return self.sublink_comparison(
                scope,
                handle,
                l,
                ObjectName { schema: None, name },
                op_location,
                sub,
            );
        }
        match operands {
            Some((Some(l), r)) => {
                let lt = self.expr(scope, l)?;
                let rt = self.expr(scope, r)?;
                let (name, op_location) = self.operator_text(self.span(l).end, self.span(r).start);
                self.infix_op(
                    handle,
                    ObjectName { schema: None, name },
                    op_location,
                    l,
                    &lt,
                    r,
                    &rt,
                )
            }
            Some((None, r)) => {
                let arg = self.expr(scope, r)?;
                let (name, op_location) = self.operator_text(location, self.span(r).start);
                self.prefix_op(handle, &name, op_location, r, &arg)
            }
            None => Err(unsupported(
                format!("expression {}", variant_name(node)),
                Some(location..location),
            )),
        }
    }

    /// The operator token in `source[from..to]`: the first non-blank,
    /// non-comment run of characters. `!=` is `<>` to PostgreSQL's scanner.
    fn operator_text(&self, from: usize, to: usize) -> (String, usize) {
        let mut i = from;
        let bytes = self.source.as_bytes();
        while i < to {
            if bytes[i].is_ascii_whitespace() {
                i += 1;
            } else if self.source[i..].starts_with("--") {
                while i < to && bytes[i] != b'\n' {
                    i += 1;
                }
            } else if self.source[i..].starts_with("/*") {
                let mut depth = 0;
                while i < to {
                    if self.source[i..].starts_with("/*") {
                        depth += 1;
                        i += 2;
                    } else if self.source[i..].starts_with("*/") {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
            } else {
                break;
            }
        }
        let start = i;
        while i < to
            && !bytes[i].is_ascii_whitespace()
            && !self.source[i..].starts_with("--")
            && !self.source[i..].starts_with("/*")
        {
            i += 1;
        }
        let text = &self.source[start..i];
        let text = if text == "!=" { "<>" } else { text };
        (text.to_owned(), start)
    }

    /// The start of the next token at or after `from`, skipping blanks and
    /// comments.
    fn next_token_start(&self, from: usize) -> usize {
        self.operator_text(from, self.source.len()).1
    }

    fn decorated_operator(
        &self,
        op: Handle<pgtk_sql::ast::shared::expr::FlatDecoratedOperator>,
    ) -> Result<(ObjectName, usize)> {
        let location = self.span(op).start;
        let d = self.get(op);
        let name = match self.get(d.name) {
            FlatQualifiedOperatorName::Plain(n) => {
                let span = self.span(*n);
                ObjectName {
                    schema: None,
                    name: self.source[span].trim().to_owned(),
                }
            }
            FlatQualifiedOperatorName::Qualified(q) => {
                let q = self.get(*q);
                let mut parts = Vec::new();
                for prefix in std::iter::once(q.first).chain(self.list(q.rest).iter().copied()) {
                    let prefix = self.get(prefix);
                    let pgtk_sql::tokens::literal::FlatIdent::Text(t) = self.get(prefix.name);
                    let span = self.text_span(*t);
                    parts.push(self.fold(self.text(*t), span.start)?);
                }
                if parts.len() > 1 {
                    return Err(unsupported(
                        "catalog-qualified operator",
                        Some(location..location),
                    ));
                }
                let span = self.span(q.name);
                ObjectName {
                    schema: parts.pop(),
                    name: self.source[span].trim().to_owned(),
                }
            }
        };
        let name = ObjectName {
            name: if name.name == "!=" {
                "<>".to_owned()
            } else {
                name.name
            },
            ..name
        };
        Ok((name, location))
    }

    fn bool_result(&self, location: usize) -> Typed {
        Typed {
            ty: self.types().builtin(Builtin::Bool),
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: false,
        }
    }

    /// A literal that `doNegate` folds a minus sign into, looking through
    /// parentheses as `gram.y` has no node for them.
    fn negatable_literal(&self, handle: Handle<FlatExpr>) -> Option<(LiteralKind, String)> {
        match self.get(handle) {
            FlatExpr::IntegerLit(t) => Some((LiteralKind::Integer, self.text(*t).to_owned())),
            FlatExpr::NumericLit(t) => Some((LiteralKind::Float, self.text(*t).to_owned())),
            FlatExpr::Parenthesized(p) => {
                let p = self.get(*p);
                if !p.indirection.is_none() {
                    return None;
                }
                self.negatable_literal(p.expr)
            }
            _ => None,
        }
    }

    fn infix_op(
        &mut self,
        handle: Handle<FlatExpr>,
        name: ObjectName,
        op_location: usize,
        l: Handle<FlatExpr>,
        lt: &Typed,
        r: Handle<FlatExpr>,
        rt: &Typed,
    ) -> Result<Typed> {
        if self.settings.transform_null_equals
            && name.schema.is_none()
            && name.name == "="
            && (lt.is_null_const() || rt.is_null_const())
        {
            return Ok(self.bool_result(lt.location));
        }
        // Two row constructors compare element by element.
        if let (Some(left), Some(right)) = (self.row_elements(l), self.row_elements(r)) {
            let scope_less = Scope::new();
            let result = self.row_comparison(&scope_less, &name, &left, &right, op_location)?;
            let _ = &scope_less;
            return Ok(Typed {
                location: lt.location,
                ..result
            });
        }
        let srf_before = self.last_srf;
        let resolved = self.resolver.make_op(&name, Some(lt), rt, op_location)?;
        if resolved.result.returns_set {
            self.check_srf_placement(op_location, srf_before)?;
        }
        self.analysis.operators.insert(handle, resolved.operator);
        self.record_coercions(l, resolved.left_coercions);
        self.record_coercions(r, resolved.right_coercions);
        Ok(Typed {
            location: lt.location,
            ..resolved.result
        })
    }

    fn prefix_op(
        &mut self,
        handle: Handle<FlatExpr>,
        name: &str,
        op_location: usize,
        r: Handle<FlatExpr>,
        rt: &Typed,
    ) -> Result<Typed> {
        self.prefix_op_named(
            handle,
            ObjectName {
                schema: None,
                name: name.to_owned(),
            },
            op_location,
            r,
            rt,
        )
    }

    fn prefix_op_named(
        &mut self,
        handle: Handle<FlatExpr>,
        name: ObjectName,
        op_location: usize,
        r: Handle<FlatExpr>,
        rt: &Typed,
    ) -> Result<Typed> {
        let resolved = self.resolver.make_op(&name, None, rt, op_location)?;
        self.analysis.operators.insert(handle, resolved.operator);
        self.record_coercions(r, resolved.right_coercions);
        Ok(Typed {
            location: op_location,
            ..resolved.result
        })
    }

    /// `transformColumnRef`.
    fn column_ref(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        names: &[String],
        location: usize,
    ) -> Result<Typed> {
        let var = match names {
            [colname] => {
                match self
                    .relations
                    .col_name_to_var(scope, &self.outer, colname, location)?
                {
                    Some(var) => var,
                    None => {
                        // A whole-row reference to a table of that name.
                        let item = self.relations.refname_namespace_item(
                            scope,
                            &self.outer,
                            None,
                            colname,
                            location,
                        )?;
                        let Some((levelsup, item)) = item else {
                            return Err(self
                                .relations
                                .error_missing_column(None, colname, location)
                                .into());
                        };
                        return self.whole_row(scope, levelsup, &item, Some(handle), location);
                    }
                }
            }
            [relname, colname] => {
                return self.item_column(scope, handle, None, relname, colname, location);
            }
            [nspname, relname, colname] => {
                return self.item_column(scope, handle, Some(nspname), relname, colname, location);
            }
            [catname, nspname, relname, colname] => {
                if catname != self.catalog().metadata().database.as_str() {
                    return Err(Diagnostic::new(
                        "0A000",
                        format!(
                            "cross-database references are not implemented: {}",
                            names.join(".")
                        ),
                    )
                    .at(location)
                    .into());
                }
                return self.item_column(scope, handle, Some(nspname), relname, colname, location);
            }
            _ => {
                return Err(Diagnostic::new(
                    "42601",
                    format!(
                        "improper qualified name (too many dotted names): {}",
                        names.join(".")
                    ),
                )
                .at(location)
                .into());
            }
        };
        if self.expr_kind == ExprKind::MergeWhen && var.varattno < 0 && var.varattno != -6 {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "cannot use system column \"{}\" in MERGE WHEN condition",
                    names.last().expect("a name")
                ),
            )
            .at(location)
            .into());
        }
        self.analysis.vars.insert(handle, var);
        if var.levelsup > 0 {
            let level = self.level() - var.levelsup;
            self.outer_vars.push(OuterRef {
                level,
                kind: self.outer_kinds[level],
                var: Var { levelsup: 0, ..var },
                location,
            });
        }
        Ok(Typed {
            ty: var.ty,
            typmod: var.typmod,
            form: Form::Var,
            location,
            returns_set: false,
        })
    }

    /// The non-junk target entries of a `SubLink`'s subquery.
    fn sublink_columns(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        sub: Handle<FlatSubquery>,
    ) -> Result<Vec<TargetEntry>> {
        self.subquery_depth += 1;
        let target_list = self.subquery(scope, sub, false);
        self.subquery_depth -= 1;
        let columns: Vec<TargetEntry> = target_list?.into_iter().filter(|t| !t.junk).collect();
        self.sublink_info.insert(
            handle,
            SublinkInfo {
                span: self.span(sub),
                first: columns.first().and_then(|c| c.collation),
            },
        );
        Ok(columns)
    }

    /// `expr op ANY/SOME/ALL (...)` with the operator read from the
    /// source: a `SubLink`, or `make_scalar_array_op` over an array.
    fn quantified(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        left: Handle<FlatExpr>,
        operator: std::ops::Range<usize>,
        tail: Handle<FlatQuantifiedComparisonTail>,
        location: usize,
    ) -> Result<Typed> {
        let _ = location;
        let text = self.source[operator.clone()].trim();
        let name = if let Some(inner) = text
            .strip_prefix("OPERATOR(")
            .or_else(|| text.strip_prefix("operator("))
            .and_then(|t| t.strip_suffix(')'))
        {
            let inner = inner.trim();
            match inner.rsplit_once('.') {
                Some((schema, op)) => ObjectName {
                    schema: Some(self.fold(schema.trim(), operator.start)?),
                    name: op.trim().to_owned(),
                },
                None => ObjectName {
                    schema: None,
                    name: inner.to_owned(),
                },
            }
        } else {
            ObjectName {
                schema: None,
                name: match text {
                    "!=" => "<>".to_owned(),
                    other => other.to_owned(),
                },
            }
        };
        self.quantified_op(scope, handle, left, name, operator.start, tail)
    }

    fn quantified_op(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        left: Handle<FlatExpr>,
        name: ObjectName,
        op_location: usize,
        tail: Handle<FlatQuantifiedComparisonTail>,
    ) -> Result<Typed> {
        let tail = self.get(tail);
        match self.get(tail.operand) {
            FlatQuantifiedComparisonOperand::Subquery(sub) => {
                self.sublink_comparison(scope, handle, left, name, op_location, *sub)
            }
            FlatQuantifiedComparisonOperand::Expr(right) => {
                self.scalar_array_op(scope, handle, left, *right, name, op_location)
            }
        }
    }

    /// `transformSubLink` for ANY and ALL: the left side is compared with
    /// each column of the subquery by the operator, as
    /// `make_row_comparison_op` does.
    fn sublink_comparison(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        left: Handle<FlatExpr>,
        name: ObjectName,
        location: usize,
        sub: Handle<FlatSubquery>,
    ) -> Result<Typed> {
        self.sublinks.insert(handle);
        let columns = self.sublink_columns(scope, handle, sub)?;
        let lt = self.expr(scope, left)?;
        // A row constructor on the left compares element by element.
        let left_list: Vec<(Option<Handle<FlatExpr>>, Typed)> = match self.row_elements(left) {
            Some(elements) => elements
                .iter()
                .map(|e| {
                    let t = self
                        .element_typed
                        .get(e)
                        .cloned()
                        .unwrap_or_else(|| lt.clone());
                    (Some(*e), t)
                })
                .collect(),
            None => vec![(Some(left), lt.clone())],
        };
        if left_list.len() < columns.len() {
            return Err(Diagnostic::new("42601", "subquery has too many columns")
                .at(location)
                .into());
        }
        if left_list.len() > columns.len() {
            return Err(Diagnostic::new("42601", "subquery has too few columns")
                .at(location)
                .into());
        }
        // make_row_comparison_op over PARAM_SUBLINK outputs.
        let bool_type = self.types().builtin(Builtin::Bool);
        for ((l_handle, l), column) in left_list.iter().zip(&columns) {
            let right = Typed {
                ty: column.ty,
                typmod: column.typmod,
                form: Form::Other,
                location: NO_LOCATION,
                returns_set: false,
            };
            let resolved = self.resolver.make_op(&name, Some(l), &right, location)?;
            if resolved.result.ty != bool_type {
                return Err(Diagnostic::new(
                    "42804",
                    format!(
                        "row comparison operator must yield type boolean, not type {}",
                        self.types().format(resolved.result.ty)
                    ),
                )
                .at(location)
                .into());
            }
            if resolved.result.returns_set {
                return Err(Diagnostic::new(
                    "42804",
                    "row comparison operator must not return a set",
                )
                .at(location)
                .into());
            }
            if left_list.len() == 1 {
                self.analysis.operators.insert(handle, resolved.operator);
            }
            if let Some(h) = l_handle {
                self.record_coercions(*h, resolved.left_coercions);
            }
        }
        Ok(Typed {
            ty: bool_type,
            typmod: -1,
            form: Form::Other,
            location: lt.location,
            returns_set: false,
        })
    }

    /// `make_scalar_array_op`: `expr op ANY/ALL (array)`.
    fn scalar_array_op(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        left: Handle<FlatExpr>,
        right: Handle<FlatExpr>,
        name: ObjectName,
        location: usize,
    ) -> Result<Typed> {
        let lt = self.expr(scope, left)?;
        let rt = self.expr(scope, right)?;
        let types = self.types();
        let rtype = if types.is_unknown(rt.ty) {
            rt.ty
        } else {
            let Some(elem) = types.element_type(types.base_type(rt.ty)) else {
                return Err(Diagnostic::new(
                    "42809",
                    "op ANY/ALL (array) requires array on right side",
                )
                .at(location)
                .into());
            };
            elem
        };
        let operator = self.resolver.oper(&name, lt.ty, rtype, location)?;
        let op = &self.catalog()[operator];
        let bool_type = types.builtin(Builtin::Bool);
        if op.result != bool_type {
            return Err(Diagnostic::new(
                "42809",
                "op ANY/ALL (array) requires operator to yield boolean",
            )
            .at(location)
            .into());
        }
        if self.catalog()[op.function].returns_set {
            return Err(Diagnostic::new(
                "42809",
                "op ANY/ALL (array) requires operator not to return a set",
            )
            .at(location)
            .into());
        }
        let actual = [lt.ty, rtype];
        let declared = [op.left.expect("binary operator"), op.right];
        let (_, declared) = self
            .coercer()
            .enforce_generic_type_consistency(&actual, &declared, op.result, false)?;
        let res_atype = if types.is_polymorphic(declared[1]) {
            rt.ty
        } else {
            let Some(a) = types.array_type(declared[1]) else {
                return Err(Diagnostic::new(
                    "42704",
                    format!(
                        "could not find array type for data type {}",
                        types.format(declared[1])
                    ),
                )
                .at(location)
                .into());
            };
            a
        };
        if actual[0] != declared[0] {
            let (_, steps) =
                self.coercer()
                    .coerce_type(&lt, declared[0], -1, CoercionContext::Implicit)?;
            self.record_coercions(left, steps);
        }
        if actual[1] != res_atype {
            let (_, steps) =
                self.coercer()
                    .coerce_type(&rt, res_atype, -1, CoercionContext::Implicit)?;
            self.record_coercions(right, steps);
        }
        self.analysis.operators.insert(handle, operator);
        Ok(Typed {
            ty: bool_type,
            typmod: -1,
            form: Form::Other,
            location: lt.location,
            returns_set: false,
        })
    }

    /// `transformFuncCall` plus `ParseFuncOrColumn` for a plain call.
    fn func_call(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        name: ObjectName,
        tail: Handle<FlatFunctionCallTail>,
        location: usize,
    ) -> Result<Typed> {
        let suffix = match self.get(tail) {
            FlatFunctionCallTail::Call(s) => self.get(*s),
            FlatFunctionCallTail::TypedLiteral(tl) => {
                // gram.y: `func_name '(' args ')' Sconst` is a typed literal
                // whose type name takes the arguments as modifiers.
                let tl = self.get(*tl);
                let mut typmods = Vec::new();
                for arg in self.list1(self.get(tl.args).0).to_vec() {
                    let e = match self.get(arg) {
                        FlatFuncArg::Named(_) => {
                            return Err(Diagnostic::new(
                                "42601",
                                "type modifier cannot have parameter name",
                            )
                            .at(self.span(arg).start)
                            .into());
                        }
                        FlatFuncArg::Plain(e) => *e,
                    };
                    typmods.push(self.typmod_of_expr(e)?);
                }
                let tn = TypeName {
                    schema: name.schema,
                    name: name.name,
                    typmods,
                    array_dims: 0,
                    location,
                };
                return self.string_const_cast(handle, tn, tl.value, location);
            }
        };
        self.call(
            scope,
            CallSite::Expr(handle),
            name,
            suffix.body,
            suffix.within_group,
            suffix.filter,
            suffix.window,
            location,
        )
    }

    /// The resolution recorded for a call site.
    fn resolved_function(&self, site: CallSite) -> Option<&FunctionResolution> {
        match site {
            CallSite::Expr(h) => self.analysis.functions.get(h),
            CallSite::From(h) => self.analysis.from_functions.get(h),
            CallSite::Statement => self.statement_function.as_ref(),
        }
    }

    fn record_function(&mut self, site: CallSite, resolution: FunctionResolution) {
        match site {
            CallSite::Expr(h) => {
                self.analysis.functions.insert(h, resolution);
            }
            CallSite::From(h) => {
                self.analysis.from_functions.insert(h, resolution);
            }
            CallSite::Statement => {
                self.statement_function = Some(resolution);
            }
        }
    }

    fn site_span(&self, site: CallSite) -> std::ops::Range<usize> {
        match site {
            CallSite::Expr(h) => self.span(h),
            CallSite::From(h) => self.span(h),
            CallSite::Statement => 0..self.source.len(),
        }
    }

    /// `transformFuncCall` plus `ParseFuncOrColumn` for the parts of a call.
    #[allow(clippy::too_many_arguments)]
    fn call(
        &mut self,
        scope: &Scope,
        site: CallSite,
        name: ObjectName,
        body: Option<Handle<FlatFunctionCallBody>>,
        within_group: Option<Handle<FlatWithinGroupClause>>,
        filter: Option<Handle<pgtk_sql::ast::shared::expr::FlatFilterClause>>,
        window: Option<Handle<FlatWindowSpec>>,
        location: usize,
    ) -> Result<Typed> {
        let srf_before = self.last_srf;
        let mut deco = Decoration {
            within_group: within_group.is_some(),
            filter: filter.is_some(),
            over: window.is_some(),
            proc_call: matches!(site, CallSite::Statement),
            ..Decoration::default()
        };
        let mut arg_handles: Vec<Handle<FlatExpr>> = Vec::new();
        let mut arg_names: Vec<Option<(String, usize)>> = Vec::new();
        let mut order_by: Option<Handle<FlatOrderByClause>> = None;
        if let Some(body) = body {
            let (items, body_order_by, (star, distinct, variadic)) = self.call_body_items(body);
            deco.star = star;
            deco.distinct = distinct;
            deco.variadic = variadic;
            order_by = body_order_by;
            for item in items {
                match self.get(item) {
                    FlatFuncArg::Plain(e) => {
                        arg_handles.push(*e);
                        arg_names.push(None);
                    }
                    FlatFuncArg::Named(n) => {
                        let n = self.get(*n);
                        let pgtk_sql::tokens::Flattype_function_name::Text(t) = self.get(n.name);
                        let at = self.text_span(*t).start;
                        arg_handles.push(n.value);
                        arg_names.push(Some((self.fold(self.text(*t), at)?, at)));
                    }
                }
            }
        }
        // Named arguments: each name once, and none before a positional one.
        let mut argnames: Vec<String> = Vec::new();
        for (i, entry) in arg_names.iter().enumerate() {
            match entry {
                Some((name, at)) => {
                    if argnames.contains(name) {
                        return Err(Diagnostic::new(
                            "42601",
                            format!("argument name \"{name}\" used more than once"),
                        )
                        .at(*at)
                        .into());
                    }
                    argnames.push(name.clone());
                }
                None => {
                    if !argnames.is_empty() {
                        return Err(Diagnostic::new(
                            "42601",
                            "positional argument cannot follow named argument",
                        )
                        .at(self.span(arg_handles[i]).start)
                        .into());
                    }
                }
            }
        }
        deco.order_by = order_by.is_some();
        // MERGE_ACTION() is a keyword form that only a MERGE may use.
        if name.schema.is_none() && name.name == "merge_action" && arg_handles.is_empty() {
            return self.merge_action(location);
        }
        // CURRENT_TIME(p) and kin are SQLValueFunction nodes with a typmod.
        if name.schema.is_none()
            && arg_handles.len() == 1
            && matches!(
                name.name.as_str(),
                "current_time" | "current_timestamp" | "localtime" | "localtimestamp"
            )
            && let Some(ty) = self.sql_value_function(&name.name)
            && let FlatExpr::IntegerLit(t) = self.get(arg_handles[0])
        {
            let text = self.text(*t);
            let precision = crate::consts::integer_value(text).unwrap_or(0);
            if precision < 0 {
                let label = match name.name.as_str() {
                    "current_time" => format!("TIME({precision}) WITH TIME ZONE"),
                    "current_timestamp" => format!("TIMESTAMP({precision}) WITH TIME ZONE"),
                    "localtime" => format!("TIME({precision})"),
                    _ => format!("TIMESTAMP({precision})"),
                };
                return Err(Diagnostic::new(
                    "22023",
                    format!("{label} precision must not be negative"),
                )
                .at(location)
                .into());
            }
            return Ok(Typed {
                ty,
                typmod: precision.min(6) as i32,
                form: Form::Other,
                location,
                returns_set: false,
            });
        }
        let outer_before = self.outer_vars.len();
        let agg_before = self.agg_refs.len();
        let cte_before = self.cte_refs.len();
        let mut args = Vec::with_capacity(arg_handles.len());
        for &a in &arg_handles {
            args.push(self.expr(scope, a)?);
        }
        // WITHIN GROUP (ORDER BY ...) expressions are appended to the
        // arguments of an ordered-set aggregate.
        let mut within_exprs: Vec<(Handle<FlatExpr>, Option<Handle<FlatUsingClause>>)> = Vec::new();
        if let Some(wg) = within_group {
            let clause = self.get(self.get(wg).order_by);
            for item in self.list1(clause.items) {
                let item = self.get(*item);
                let using = match item.order.map(|o| self.get(o)) {
                    Some(FlatSortOrder::Using(u)) => Some(*u),
                    _ => None,
                };
                within_exprs.push((item.expr, using));
            }
            for &(e, using) in &within_exprs {
                arg_handles.push(e);
                let t = self.expr(scope, e)?;
                // addTargetToSortList: an ordering operator written with
                // USING must be a btree ordering operator.
                if let Some(using) = using {
                    self.using_operator(using, t.ty, self.span(e).start)?;
                }
                args.push(t);
            }
            deco.aggregated = within_exprs.len();
        }
        // ParseComplexProjection: f(composite) may be the field f.
        let projection = if name.schema.is_none()
            && args.len() == 1
            && body.is_some()
            && !deco.star
            && !deco.distinct
            && !deco.variadic
            && argnames.is_empty()
        {
            self.projection_field(self.types().base_type(args[0].ty), &name.name)
        } else {
            None
        };
        let is_projection = projection.is_some();
        let projected_name = name.name.clone();
        let typed = self.resolve_call_decorated(
            site,
            name,
            &arg_handles,
            &args,
            &argnames,
            location,
            projection,
            deco,
            srf_before,
        )?;
        // ParseComplexProjection on a whole-row reference gives the column
        // itself, with its origin.
        if is_projection
            && self.resolved_function(site).is_none()
            && let CallSite::Expr(h) = site
            && let Some((levelsup, item)) = self.whole_rows.get(&arg_handles[0]).cloned()
        {
            let level = self.scope_at(scope, levelsup);
            if let Some(var) = self.relations.scan_ns_item_for_column(
                level,
                &item,
                levelsup,
                &projected_name,
                location,
            )? {
                self.analysis.vars.insert(h, var);
                return Ok(Typed {
                    form: Form::Var,
                    ..typed
                });
            }
        }
        // A window function call: its window, and for an aggregate its
        // FILTER.
        if let Some(over) = window
            && self.resolved_function(site).is_some_and(|f| f.window)
        {
            let is_aggregate = self
                .resolved_function(site)
                .is_some_and(|f| self.catalog()[f.function].aggregate.is_some());
            self.window_call(
                scope,
                over,
                &arg_handles,
                order_by,
                filter,
                deco,
                is_aggregate,
                location,
            )?;
            return Ok(typed);
        }
        // An aggregate's ORDER BY, DISTINCT and FILTER.
        if self
            .resolved_function(site)
            .is_some_and(|f| self.catalog()[f.function].aggregate.is_some())
        {
            let level = self.level();
            let (order_exprs, filter_expr) =
                self.aggregate_clauses(scope, &arg_handles, &args, order_by, filter, deco)?;
            // check_agg_arguments: the direct arguments of an ordered-set
            // aggregate do not set its level.
            let direct: Vec<Handle<FlatExpr>> = if within_group.is_some() {
                arg_handles[..arg_handles.len() - deco.aggregated].to_vec()
            } else {
                Vec::new()
            };
            let aggregated: Vec<Handle<FlatExpr>> = arg_handles[direct.len()..]
                .iter()
                .copied()
                .chain(order_exprs)
                .chain(filter_expr)
                .collect();
            let agg_level = self.check_agg_arguments(
                &direct,
                &aggregated,
                outer_before,
                agg_before,
                cte_before,
                location,
            )?;
            // Columns inside an aggregate's arguments are not subject to
            // the ungrouped-column check of the aggregate's level, outer
            // references from subqueries in them included.
            let kept: Vec<OuterRef> = self.outer_vars[outer_before..]
                .iter()
                .filter(|r| r.level != agg_level)
                .copied()
                .collect();
            self.outer_vars.truncate(outer_before);
            self.outer_vars.extend(kept);
            self.agg_level = (agg_level < level).then_some(agg_level);
            let registered = self.register_aggregate(location, false);
            self.agg_level = None;
            registered?;
            self.agg_refs.push(AggRef {
                level: agg_level,
                location,
            });
        }
        Ok(typed)
    }

    /// `transformWindowFuncCall`: the FILTER of an aggregate over a window,
    /// the placement rule, and the window the call names or defines.
    #[allow(clippy::too_many_arguments)]
    fn window_call(
        &mut self,
        scope: &Scope,
        over: Handle<FlatWindowSpec>,
        arg_handles: &[Handle<FlatExpr>],
        order_by: Option<Handle<FlatOrderByClause>>,
        filter: Option<Handle<pgtk_sql::ast::shared::expr::FlatFilterClause>>,
        deco: Decoration,
        is_aggregate: bool,
        location: usize,
    ) -> Result<()> {
        let _ = (arg_handles, order_by, deco);
        if is_aggregate && let Some(f) = filter {
            let expr = self.get(self.get(f).body).condition;
            let saved = self.expr_kind;
            self.expr_kind = ExprKind::Filter;
            let t = self.expr(scope, expr)?;
            let (_, steps) = self.coercer().coerce_to_boolean(&t, "FILTER")?;
            self.record_coercions(expr, steps);
            self.expr_kind = saved;
        }
        if let Some(message) = self.expr_kind.window_refusal() {
            return Err(Diagnostic::new("42P20", message).at(location).into());
        }
        match self.get(self.get(over).body) {
            FlatWindowSpecBody::Named(n) => {
                let (name, at) = self.col_id(*n)?;
                if !self
                    .windows
                    .iter()
                    .any(|w| w.name.as_deref() == Some(name.as_str()))
                {
                    return Err(Diagnostic::new(
                        "42704",
                        format!("window \"{name}\" does not exist"),
                    )
                    .at(at)
                    .into());
                }
            }
            FlatWindowSpecBody::Inline(spec) => {
                let from = self.span(over).start;
                let wd = self.window_spec(*spec, from)?;
                self.windows.push(wd);
            }
        }
        self.has_window_funcs = true;
        self.window_locations.push(location);
        Ok(())
    }

    /// A `window_specification` as written. `from` is where the clause
    /// around it starts, for an empty specification that has no span.
    fn window_spec(&self, spec: Handle<FlatInlineWindowSpec>, from: usize) -> Result<WindowDef> {
        // gram.y locates a window_specification at its opening parenthesis.
        let inner = self.span(spec).start;
        let location = if inner > self.source.len() {
            self.source[from..].find('(').map_or(from, |i| from + i)
        } else {
            let before = self.source[..inner].trim_end();
            if before.ends_with('(') {
                before.len() - 1
            } else {
                inner
            }
        };
        let s = self.get(spec);
        let refname = match s.ref_name {
            Some(r) => {
                let FlatWindowRefNameIdent::Text(t) = self.get(r);
                let span = self.text_span(*t);
                Some(self.fold(self.text(*t), span.start)?)
            }
            None => None,
        };
        let partition = match s.partition_by {
            Some(p) => self.list1(self.get(p).exprs).to_vec(),
            None => Vec::new(),
        };
        Ok(WindowDef {
            name: None,
            refname,
            partition,
            order_by: s.order_by,
            frame: s.frame,
            location,
        })
    }

    /// The start and end bounds of a frame as written; `None` for the end of
    /// a frame without `BETWEEN`. The grammar admits only the combinations
    /// that gram.y's `frame_extent` keeps.
    fn frame_bounds(
        &self,
        body: Handle<FlatWindowFrameBody>,
    ) -> (FrameBound, usize, Option<FrameBound>, usize) {
        use pgtk_sql::ast::shared::expr::{
            FlatWindowFrameBetweenExtent as Extent, FlatWindowFrameEnd as End,
            FlatWindowFrameEndAfterCurrentRow as AfterCurrent,
            FlatWindowFrameEndAfterFollowing as AfterFollowing,
            FlatWindowFrameEndAfterUnboundedFollowing as AfterUnboundedFollowing,
            FlatWindowFrameSingleBound as Single,
        };
        let preceding = |h: Handle<pgtk_sql::ast::shared::expr::FlatWindowFramePreceding>| {
            FrameBound::OffsetPreceding(self.get(h).offset)
        };
        let following = |h: Handle<pgtk_sql::ast::shared::expr::FlatWindowFrameFollowing>| {
            FrameBound::OffsetFollowing(self.get(h).offset)
        };
        let end = |h: Handle<End>| match self.get(h) {
            End::CurrentRow => FrameBound::CurrentRow,
            End::UnboundedPreceding => FrameBound::UnboundedPreceding,
            End::UnboundedFollowing => FrameBound::UnboundedFollowing,
            End::Preceding(p) => preceding(*p),
            End::Following(f) => following(*f),
        };
        match self.get(body) {
            FlatWindowFrameBody::Single(single) => {
                let start = match self.get(self.get(*single).bound) {
                    Single::CurrentRow => FrameBound::CurrentRow,
                    Single::UnboundedPreceding => FrameBound::UnboundedPreceding,
                    // A declared rule refuses it when it is reduced.
                    Single::UnboundedFollowing => FrameBound::UnboundedFollowing,
                    Single::Preceding(p) => preceding(*p),
                };
                (start, self.span(body).start, None, NO_LOCATION)
            }
            FlatWindowFrameBody::Between(between) => {
                let extent = self.get(*between).extent;
                let start_at = self.span(extent).start;
                match self.get(extent) {
                    // A declared rule refuses it when it is reduced.
                    Extent::FromUnboundedFollowing(e) => (
                        FrameBound::UnboundedFollowing,
                        start_at,
                        Some(match self.get(*e) {
                            AfterUnboundedFollowing::CurrentRow => FrameBound::CurrentRow,
                            AfterUnboundedFollowing::UnboundedPreceding => {
                                FrameBound::UnboundedPreceding
                            }
                            AfterUnboundedFollowing::UnboundedFollowing => {
                                FrameBound::UnboundedFollowing
                            }
                            AfterUnboundedFollowing::Preceding(p) => preceding(*p),
                            AfterUnboundedFollowing::Following(f) => following(*f),
                        }),
                        self.span(*e).start,
                    ),
                    Extent::FromUnboundedPreceding(e) => (
                        FrameBound::UnboundedPreceding,
                        start_at,
                        Some(end(*e)),
                        self.span(*e).start,
                    ),
                    Extent::FromPreceding(f) => {
                        let f = self.get(*f);
                        (
                            preceding(f.start),
                            start_at,
                            Some(end(f.end)),
                            self.span(f.end).start,
                        )
                    }
                    Extent::FromCurrentRow(e) => {
                        let bound = match self.get(*e) {
                            AfterCurrent::CurrentRow => FrameBound::CurrentRow,
                            // A declared rule refuses it when it is reduced.
                            AfterCurrent::UnboundedPreceding => FrameBound::UnboundedPreceding,
                            AfterCurrent::UnboundedFollowing => FrameBound::UnboundedFollowing,
                            AfterCurrent::Following(f) => following(*f),
                        };
                        (FrameBound::CurrentRow, start_at, Some(bound), NO_LOCATION)
                    }
                    Extent::FromFollowing(f) => {
                        let f = self.get(*f);
                        let bound = match self.get(f.end) {
                            // A declared rule refuses it when it is reduced.
                            AfterFollowing::UnboundedPreceding => FrameBound::UnboundedPreceding,
                            AfterFollowing::UnboundedFollowing => FrameBound::UnboundedFollowing,
                            AfterFollowing::Following(h) => following(*h),
                        };
                        (following(f.start), start_at, Some(bound), NO_LOCATION)
                    }
                }
            }
        }
    }

    /// `transformWindowDefinitions` over the query level's windows.
    fn window_definitions(
        &mut self,
        scope: &Scope,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
    ) -> Result<()> {
        let defs = std::mem::take(&mut self.windows);
        // Each definition once transformed: its name, whether it has an
        // ORDER BY, and whether it has a frame clause.
        // Each definition once transformed: its name, its ORDER BY count,
        // whether it has a frame, and the type of its first ORDER BY key.
        let mut done: Vec<(Option<String>, usize, bool, Option<TypeRef>)> = Vec::new();
        for def in &defs {
            if let Some(name) = &def.name
                && done
                    .iter()
                    .any(|(n, _, _, _)| n.as_deref() == Some(name.as_str()))
            {
                return Err(Diagnostic::new(
                    "42P20",
                    format!("window \"{name}\" is already defined"),
                )
                .at(def.location)
                .into());
            }
            let refwc = match &def.refname {
                Some(r) => {
                    let Some(found) = done
                        .iter()
                        .find(|(n, _, _, _)| n.as_deref() == Some(r.as_str()))
                    else {
                        return Err(Diagnostic::new(
                            "42704",
                            format!("window \"{r}\" does not exist"),
                        )
                        .at(def.location)
                        .into());
                    };
                    Some(found.clone())
                }
                None => None,
            };
            // ORDER BY with the SQL99 rules, then PARTITION BY.
            let mut order_count = 0;
            let mut first_order_type: Option<TypeRef> = None;
            if let Some(ob) = def.order_by {
                let clause = self.get(ob);
                self.expr_kind = ExprKind::WindowOrder;
                for item in self.list1(clause.items) {
                    let item = self.get(*item);
                    let index = self.find_targetlist_entry_sql99(
                        scope,
                        item.expr,
                        target_list,
                        target_exprs,
                    )?;
                    self.extra_sortgroup_refs.push(index);
                    let entry = &mut target_list[index];
                    if self.types().is_unknown(entry.ty) {
                        entry.ty = self.types().builtin(Builtin::Text);
                        entry.typmod = -1;
                    }
                    let ty = entry.ty;
                    if let Some(FlatSortOrder::Using(using)) = item.order.map(|o| self.get(o)) {
                        self.using_operator(*using, ty, self.span(item.expr).start)?;
                        if first_order_type.is_none() {
                            first_order_type = Some(ty);
                        }
                        order_count += 1;
                        continue;
                    }
                    if self.types().btree_ordering(ty).is_none() {
                        return Err(Diagnostic::new(
                            "42883",
                            format!(
                                "could not identify an ordering operator for type {}",
                                self.types().format(ty)
                            ),
                        )
                        .hint("Use an explicit ordering operator or modify the query.")
                        .at(self.span(item.expr).start)
                        .into());
                    }
                    if first_order_type.is_none() {
                        first_order_type = Some(ty);
                    }
                    order_count += 1;
                }
            }
            self.expr_kind = ExprKind::WindowPartition;
            for &expr in &def.partition {
                let index =
                    self.find_targetlist_entry_sql99(scope, expr, target_list, target_exprs)?;
                self.extra_sortgroup_refs.push(index);
                let entry = &mut target_list[index];
                if self.types().is_unknown(entry.ty) {
                    entry.ty = self.types().builtin(Builtin::Text);
                    entry.typmod = -1;
                }
                let ty = entry.ty;
                if self.types().equality_operator(ty).is_none() {
                    return Err(Diagnostic::new(
                        "42883",
                        format!(
                            "could not identify an equality operator for type {}",
                            self.types().format(ty)
                        ),
                    )
                    .at(self.span(expr).start)
                    .into());
                }
            }
            let mut order_type = first_order_type;
            if let Some((refname, ref_order_count, ref_has_frame, ref_order_type)) = &refwc {
                let refname = refname.clone().unwrap_or_default();
                let ref_has_order = &(*ref_order_count > 0);
                if !def.partition.is_empty() {
                    return Err(Diagnostic::new(
                        "42P20",
                        format!("cannot override PARTITION BY clause of window \"{refname}\""),
                    )
                    .at(def.location)
                    .into());
                }
                if order_count > 0 && *ref_has_order {
                    return Err(Diagnostic::new(
                        "42P20",
                        format!("cannot override ORDER BY clause of window \"{refname}\""),
                    )
                    .at(def.location)
                    .into());
                }
                if order_count == 0 {
                    order_count = *ref_order_count;
                    order_type = *ref_order_type;
                }
                if *ref_has_frame {
                    return Err(Diagnostic::new(
                        "42P20",
                        format!("cannot copy window \"{refname}\" because it has a frame clause"),
                    )
                    .at(def.location)
                    .into());
                }
            }
            let has_frame = def.frame.is_some();
            if let Some(frame) = def.frame {
                self.window_frame(scope, frame, order_count, order_type, def.location)?;
            }
            done.push((def.name.clone(), order_count, has_frame, order_type));
        }
        Ok(())
    }

    /// A frame clause: gram.y's bound rules, then the offsets typed as
    /// `transformFrameOffset` does.
    fn window_frame(
        &mut self,
        scope: &Scope,
        frame: Handle<FlatWindowFrameClause>,
        order_count: usize,
        order_type: Option<TypeRef>,
        location: usize,
    ) -> Result<()> {
        let has_order = order_count > 0;
        let f = self.get(frame);
        let unit = *self.get(f.unit);
        let (start, start_at, end, end_at) = self.frame_bounds(f.body);
        // gram.y's frame_extent rules that the grammar does not state (see
        // `WindowFrameBetween`), at the offending bound.
        let err = |m: &str, at: usize| -> crate::diagnostic::AnalyzeError {
            Diagnostic::new("42P20", m.to_owned()).at(at).into()
        };
        if start == FrameBound::UnboundedFollowing {
            return Err(err("frame start cannot be UNBOUNDED FOLLOWING", start_at));
        }
        if end == Some(FrameBound::UnboundedPreceding) {
            return Err(err("frame end cannot be UNBOUNDED PRECEDING", end_at));
        }
        let offsets: Vec<Handle<FlatExpr>> = [Some(start), end]
            .into_iter()
            .flatten()
            .filter_map(|b| match b {
                FrameBound::OffsetPreceding(e) | FrameBound::OffsetFollowing(e) => Some(e),
                _ => None,
            })
            .collect();
        match unit {
            FlatWindowFrameUnit::Range => {
                if !offsets.is_empty() && order_count != 1 {
                    return Err(Diagnostic::new(
                        "42P20",
                        "RANGE with offset PRECEDING/FOLLOWING requires exactly one ORDER BY column",
                    )
                    .at(location)
                    .into());
                }
            }
            FlatWindowFrameUnit::Groups => {
                if !has_order {
                    return Err(Diagnostic::new(
                        "42P20",
                        "GROUPS mode requires an ORDER BY clause",
                    )
                    .at(location)
                    .into());
                }
            }
            FlatWindowFrameUnit::Rows => {}
        }
        let (kind, construct) = match unit {
            FlatWindowFrameUnit::Rows => (ExprKind::WindowFrameRows, "ROWS"),
            FlatWindowFrameUnit::Groups => (ExprKind::WindowFrameGroups, "GROUPS"),
            FlatWindowFrameUnit::Range => (ExprKind::WindowFrameRange, "RANGE"),
        };
        for e in offsets {
            self.expr_kind = kind;
            let typed = self.expr(scope, e)?;
            if unit == FlatWindowFrameUnit::Range {
                let order_type = order_type.expect("one ORDER BY column");
                self.range_offset(e, &typed, order_type)?;
            } else {
                let int8 = self.types().builtin(Builtin::Int8);
                self.coerce_to_specific_type(e, &typed, int8, construct)?;
            }
            if self.contains_var(e) {
                return Err(Diagnostic::new(
                    "42P10",
                    format!("argument of {construct} must not contain variables"),
                )
                .at(typed.location)
                .into());
            }
        }
        Ok(())
    }

    /// `ORDER BY ... USING op`: the operator over the sort type must be
    /// a btree `<` or `>` member.
    fn using_operator(
        &mut self,
        using: Handle<FlatUsingClause>,
        ty: TypeRef,
        location: usize,
    ) -> Result<()> {
        // The operator as written, `!=` read as `<>` by the scanner.
        let name = match self.get(self.get(using).op) {
            FlatUsingOp::Plain(n) => {
                let span = self.span(*n);
                let written = self.source[span].trim();
                ObjectName {
                    schema: None,
                    name: if written == "!=" { "<>" } else { written }.to_owned(),
                }
            }
            FlatUsingOp::Decorated(d) => self.decorated_operator(*d)?.0,
        };
        let op = name.name.clone();
        let operand = Typed {
            ty,
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: false,
        };
        let resolved = self
            .resolver
            .make_op(&name, Some(&operand), &operand, location)?;
        let valid = match self.types().btree_ordering(ty) {
            Some(ops) => resolved.operator == ops.lt || resolved.operator == ops.gt,
            None => false,
        } || (matches!(op.as_str(), "~<~" | "~>~")
            && self.types().category(ty) == TypeCategory::String);
        if !valid {
            return Err(Diagnostic::new(
                "42809",
                format!("operator {op} is not a valid ordering operator"),
            )
            .hint("Ordering operators must be \"<\" or \">\" members of btree operator families.")
            .into());
        }
        Ok(())
    }

    /// `transformFrameOffset` for RANGE: the offset takes the type of the
    /// btree `in_range` support function for the ORDER BY key's type.
    /// The support functions are PostgreSQL's built-in ones, the same in
    /// versions 14 to 17.
    fn range_offset(
        &mut self,
        e: Handle<FlatExpr>,
        typed: &Typed,
        order_type: TypeRef,
    ) -> Result<()> {
        let types = self.types();
        let opcintype = types.ordering_input_type(order_type);
        let rights: Vec<TypeRef> = [
            (
                Builtin::Int2,
                &[Builtin::Int8, Builtin::Int4, Builtin::Int2][..],
            ),
            (
                Builtin::Int4,
                &[Builtin::Int8, Builtin::Int4, Builtin::Int2][..],
            ),
            (Builtin::Int8, &[Builtin::Int8][..]),
            (Builtin::Float4, &[Builtin::Float8][..]),
            (Builtin::Float8, &[Builtin::Float8][..]),
            (Builtin::Numeric, &[Builtin::Numeric][..]),
            (Builtin::Date, &[Builtin::Interval][..]),
            (Builtin::Timestamp, &[Builtin::Interval][..]),
            (Builtin::Timestamptz, &[Builtin::Interval][..]),
            (Builtin::Time, &[Builtin::Interval][..]),
            (Builtin::Timetz, &[Builtin::Interval][..]),
            (Builtin::Interval, &[Builtin::Interval][..]),
        ]
        .into_iter()
        .find(|(left, _)| types.is(opcintype, *left))
        .map_or_else(Vec::new, |(_, rights)| {
            rights.iter().map(|b| types.builtin(*b)).collect()
        });
        if rights.is_empty() {
            return Err(Diagnostic::new(
                "0A000",
                format!(
                    "RANGE with offset PRECEDING/FOLLOWING is not supported for column type {}",
                    types.format(opcintype)
                ),
            )
            .at(typed.location)
            .into());
        }
        let node_type = typed.ty;
        let preferred = if types.is_unknown(node_type) {
            opcintype
        } else {
            node_type
        };
        let mut nmatches = 0;
        let mut selected: Option<TypeRef> = None;
        for right in rights {
            if !self
                .coercer()
                .can_coerce_type(&[node_type], &[right], CoercionContext::Implicit)
            {
                continue;
            }
            nmatches += 1;
            if selected != Some(preferred) {
                selected = Some(right);
            }
        }
        if nmatches == 0 {
            return Err(Diagnostic::new(
                "0A000",
                format!(
                    "RANGE with offset PRECEDING/FOLLOWING is not supported for column type {} and offset type {}",
                    types.format(opcintype),
                    types.format(node_type)
                ),
            )
            .hint("Cast the offset value to an appropriate type.")
            .at(typed.location)
            .into());
        }
        if nmatches != 1 && selected != Some(preferred) {
            return Err(Diagnostic::new(
                "0A000",
                format!(
                    "RANGE with offset PRECEDING/FOLLOWING has multiple interpretations for column type {} and offset type {}",
                    types.format(opcintype),
                    types.format(node_type)
                ),
            )
            .hint("Cast the offset value to the exact intended type.")
            .at(typed.location)
            .into());
        }
        let target = selected.expect("a match");
        self.coerce_to_specific_type(e, typed, target, "RANGE")
    }

    /// `findTargetlistEntrySQL99`: an expression, reusing an equal target
    /// entry or adding a resjunk one.
    fn find_targetlist_entry_sql99(
        &mut self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
        target_list: &mut Vec<TargetEntry>,
        target_exprs: &mut Vec<Option<Handle<FlatExpr>>>,
    ) -> Result<usize> {
        let typed = self.expr(scope, expr)?;
        for (i, e) in target_exprs.iter().enumerate() {
            if let Some(e) = e
                && self.expr_equal(*e, expr)
            {
                return Ok(i);
            }
        }
        target_list.push(TargetEntry {
            collation_state: CollContext::new(),
            collation: None,
            name: String::new(),
            ty: typed.ty,
            typmod: typed.typmod,
            origin: None,
            junk: true,
            location: self.span(expr).start,
            fields: None,
        });
        target_exprs.push(Some(expr));
        Ok(target_list.len() - 1)
    }

    /// `transformAggregateCall` and `check_agglevels_and_constraints` for a
    /// resolved aggregate.
    fn aggregate_clauses(
        &mut self,
        scope: &Scope,
        arg_handles: &[Handle<FlatExpr>],
        args: &[Typed],
        order_by: Option<Handle<FlatOrderByClause>>,
        filter: Option<Handle<pgtk_sql::ast::shared::expr::FlatFilterClause>>,
        deco: Decoration,
    ) -> Result<AggregateClauses> {
        // Argument ORDER BY: SQL99 rules; a new expression is a junk entry.
        let mut order_exprs: Vec<Handle<FlatExpr>> = Vec::new();
        if let Some(ob) = order_by {
            let clause = self.get(ob);
            let saved = self.expr_kind;
            self.expr_kind = ExprKind::OrderBy;
            for item in self.list1(clause.items) {
                let item = self.get(*item);
                let mut t = self.expr(scope, item.expr)?;
                if self.types().is_unknown(t.ty) {
                    let text = self.types().builtin(Builtin::Text);
                    let steps = self.coercer().coerce_to_common_type(&t, text, "ORDER BY")?;
                    self.record_coercions(item.expr, steps);
                    t.ty = text;
                }
                if let Some(FlatSortOrder::Using(using)) = item.order.map(|o| self.get(o)) {
                    self.using_operator(*using, t.ty, self.span(item.expr).start)?;
                } else if self.types().btree_ordering(t.ty).is_none() {
                    return Err(Diagnostic::new(
                        "42883",
                        format!(
                            "could not identify an ordering operator for type {}",
                            self.types().format(t.ty)
                        ),
                    )
                    .hint("Use an explicit ordering operator or modify the query.")
                    .at(self.span(item.expr).start)
                    .into());
                }
                order_exprs.push(item.expr);
            }
            self.expr_kind = saved;
        }
        if deco.distinct {
            // transformDistinctClause(is_agg): ORDER BY items must be
            // arguments, and each argument needs an ordering operator.
            for &oe in &order_exprs {
                if !arg_handles.iter().any(|&a| self.expr_equal(a, oe)) {
                    return Err(Diagnostic::new(
                        "42P10",
                        "in an aggregate with DISTINCT, ORDER BY expressions must appear in argument list",
                    )
                    .at(self.span(oe).start)
                    .into());
                }
            }
            for (a, t) in arg_handles.iter().zip(args) {
                let ty = if self.types().is_unknown(t.ty) {
                    self.types().builtin(Builtin::Text)
                } else {
                    t.ty
                };
                if self.types().btree_ordering(ty).is_none() {
                    return Err(Diagnostic::new(
                        "42883",
                        format!(
                            "could not identify an ordering operator for type {}",
                            self.types().format(ty)
                        ),
                    )
                    .detail("Aggregates with DISTINCT must be able to sort their inputs.")
                    .at(self.span(*a).start)
                    .into());
                }
            }
            if args.is_empty() {
                return Err(Diagnostic::new(
                    "42601",
                    "an aggregate with DISTINCT must have at least one argument",
                )
                .into());
            }
        }
        let mut filter_expr = None;
        if let Some(f) = filter {
            let expr = self.get(self.get(f).body).condition;
            let saved = self.expr_kind;
            self.expr_kind = ExprKind::Filter;
            let t = self.expr(scope, expr)?;
            let (_, steps) = self.coercer().coerce_to_boolean(&t, "FILTER")?;
            self.record_coercions(expr, steps);
            self.expr_kind = saved;
            filter_expr = Some(expr);
        }
        Ok((order_exprs, filter_expr))
    }

    /// `check_agg_arguments`: the level an aggregate belongs to, the
    /// nearest level of the columns and aggregates in its aggregated
    /// arguments and FILTER, and the checks on what its arguments hold.
    /// The references found since the given marks are the ones inside the
    /// call. Returns the level, 0 being the statement.
    fn check_agg_arguments(
        &self,
        direct: &[Handle<FlatExpr>],
        aggregated: &[Handle<FlatExpr>],
        outer_before: usize,
        agg_before: usize,
        cte_before: usize,
        location: usize,
    ) -> Result<usize> {
        let level = self.level();
        let direct_spans: Vec<std::ops::Range<usize>> =
            direct.iter().map(|h| self.span(*h)).collect();
        let in_direct = |at: usize| direct_spans.iter().any(|s| at >= s.start && at < s.end);
        // Levels relative to this query: 0 is this level, 1 its parent.
        let min_var = |w: &Self, want_direct: bool, exprs: &[Handle<FlatExpr>]| -> Option<usize> {
            let local = exprs.iter().any(|&h| w.contains_var(h));
            w.outer_vars[outer_before..]
                .iter()
                .filter(|r| r.level <= level && in_direct(r.location) == want_direct)
                .map(|r| level - r.level)
                .chain(local.then_some(0))
                .min()
        };
        let min_agg = |w: &Self, want_direct: bool| -> Option<(usize, usize)> {
            w.agg_refs[agg_before..]
                .iter()
                .filter(|r| r.level <= level && in_direct(r.location) == want_direct)
                .map(|r| (level - r.level, r.location))
                .min_by_key(|(l, _)| *l)
        };
        let min_cte = |w: &Self, want_direct: bool| -> Option<(usize, String)> {
            w.cte_refs[cte_before..]
                .iter()
                .filter(|r| r.level <= level && in_direct(r.location) == want_direct)
                .map(|r| (level - r.level, r.name.clone()))
                .min_by_key(|(l, _)| *l)
        };
        let nested = |at: usize| -> Diagnostic {
            Diagnostic::new("42803", "aggregate function calls cannot be nested").at(at)
        };
        let nested_cte = |name: &str| -> Diagnostic {
            Diagnostic::new("0A000", "outer-level aggregate cannot use a nested CTE")
                .detail(format!(
                    "CTE \"{name}\" is below the aggregate's semantic level."
                ))
                .at(location)
        };
        let var_level = min_var(self, false, aggregated);
        let agg_ref = min_agg(self, false);
        let agglevel = match (var_level, agg_ref) {
            (None, None) => 0,
            (None, Some((a, _))) => a,
            (Some(v), None) => v,
            (Some(v), Some((a, _))) => v.min(a),
        };
        if let Some((a, at)) = agg_ref
            && a == agglevel
        {
            return Err(nested(at).into());
        }
        if let Some((c, cte)) = min_cte(self, false)
            && c < agglevel
        {
            return Err(nested_cte(&cte).into());
        }
        if !direct.is_empty() {
            if let Some(v) = min_var(self, true, direct)
                && v < agglevel
            {
                let at = self
                    .locate_var_of_level(direct, level - v, outer_before)
                    .unwrap_or(location);
                return Err(Diagnostic::new(
                    "42803",
                    "outer-level aggregate cannot contain a lower-level variable in its direct arguments",
                )
                .at(at)
                .into());
            }
            if let Some((a, at)) = min_agg(self, true)
                && a <= agglevel
            {
                return Err(nested(at).into());
            }
            if let Some((c, cte)) = min_cte(self, true)
                && c < agglevel
            {
                return Err(nested_cte(&cte).into());
            }
        }
        Ok(level - agglevel)
    }

    /// `locate_var_of_level`: the first column reference of the given
    /// level (0 being the statement) inside the expressions.
    fn locate_var_of_level(
        &self,
        exprs: &[Handle<FlatExpr>],
        wanted: usize,
        outer_before: usize,
    ) -> Option<usize> {
        let spans: Vec<std::ops::Range<usize>> = exprs.iter().map(|h| self.span(*h)).collect();
        let inside = |at: usize| spans.iter().any(|s| at >= s.start && at < s.end);
        let mut best: Option<usize> = self.outer_vars[outer_before..]
            .iter()
            .filter(|r| r.level == wanted && inside(r.location))
            .map(|r| r.location)
            .min();
        if wanted == self.level() {
            let sublinks: Vec<std::ops::Range<usize>> = self
                .sublinks
                .iter()
                .map(|h| self.span(*h))
                .filter(|s| inside(s.start))
                .collect();
            let local = self
                .analysis
                .vars
                .iter()
                .filter(|(h, v)| {
                    let s = self.span(*h);
                    v.levelsup == 0
                        && inside(s.start)
                        && !sublinks
                            .iter()
                            .any(|sl| s.start >= sl.start && s.end <= sl.end)
                })
                .map(|(h, _)| self.span(h).start)
                .min();
            best = match (best, local) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
        }
        best
    }

    /// `check_agglevels_and_constraints`: the aggregate is placed at its
    /// own level, which must allow one.
    fn register_aggregate(&mut self, location: usize, grouping: bool) -> Result<()> {
        let refusal = |kind: ExprKind| {
            let message = kind.aggregate_refusal();
            if grouping {
                message.replace("aggregate functions", "grouping operations")
            } else {
                message
            }
        };
        if let Some(outer) = self.agg_level {
            let kind = self.outer_kinds[outer];
            if kind.refuses_aggregate() {
                return Err(Diagnostic::new("42803", refusal(kind)).at(location).into());
            }
            self.outer_agg_levels.push(outer);
            return Ok(());
        }
        if self.expr_kind.refuses_aggregate() {
            return Err(Diagnostic::new("42803", refusal(self.expr_kind))
                .at(location)
                .into());
        }
        self.has_aggs = true;
        self.agg_locations.push(location);
        Ok(())
    }

    /// The field `name` of a composite type, for function-call syntax.
    fn projection_field(&self, ty: TypeRef, name: &str) -> Option<Projection> {
        match &self.catalog()[ty].kind {
            pgtk_catalog::TypeKind::Composite(detail) => detail
                .fields
                .iter()
                .find(|f| f.name.as_str() == name)
                .map(|f| Projection {
                    ty: f.ty,
                    typmod: f.typmod.0,
                }),
            _ => None,
        }
    }

    /// `ParseFuncOrColumn` for arguments already transformed.
    fn resolve_call(
        &mut self,
        handle: Handle<FlatExpr>,
        name: ObjectName,
        arg_handles: &[Handle<FlatExpr>],
        args: &[Typed],
        location: usize,
        projection: Option<Projection>,
    ) -> Result<Typed> {
        let srf_before = self.last_srf;
        self.resolve_call_decorated(
            CallSite::Expr(handle),
            name,
            arg_handles,
            args,
            &[],
            location,
            projection,
            Decoration::default(),
            srf_before,
        )
    }

    /// `check_srf_call_placement` and the aggregate-argument rule for a
    /// call or operator that returns a set.
    fn check_srf_placement(&mut self, location: usize, srf_before: Option<usize>) -> Result<()> {
        if self.expr_kind == ExprKind::FromFunction && self.last_srf != srf_before {
            // A set-returning function inside the arguments of one in FROM.
            return Err(Diagnostic::new(
                "0A000",
                "set-returning functions must appear at top level of FROM",
            )
            .at(self.last_srf.unwrap_or(location))
            .into());
        }
        self.last_srf = Some(location);
        if matches!(
            self.expr_kind,
            ExprKind::SelectTarget
                | ExprKind::GroupBy
                | ExprKind::OrderBy
                | ExprKind::DistinctOn
                | ExprKind::ValuesSingle
                | ExprKind::WindowPartition
                | ExprKind::WindowOrder
        ) {
            self.has_target_srfs = true;
        }
        if self.in_agg_args > 0 {
            return Err(Diagnostic::new(
                "0A000",
                "aggregate function calls cannot contain set-returning function calls",
            )
            .hint("You might be able to move the set-returning function into a LATERAL FROM item.")
            .at(location)
            .into());
        }
        if matches!(self.expr_kind, ExprKind::JoinOn | ExprKind::JoinUsing) {
            return Err(Diagnostic::new(
                "0A000",
                "set-returning functions are not allowed in JOIN conditions",
            )
            .at(location)
            .into());
        }
        if matches!(
            self.expr_kind,
            ExprKind::WindowFrameRows | ExprKind::WindowFrameRange | ExprKind::WindowFrameGroups
        ) {
            return Err(Diagnostic::new(
                "0A000",
                "set-returning functions are not allowed in window definitions",
            )
            .at(location)
            .into());
        }
        if self.expr_kind == ExprKind::MergeWhen {
            return Err(Diagnostic::new(
                "0A000",
                "set-returning functions are not allowed in MERGE WHEN conditions",
            )
            .at(location)
            .into());
        }
        if self.expr_kind == ExprKind::CallArgument {
            return Err(Diagnostic::new(
                "0A000",
                "set-returning functions are not allowed in CALL arguments",
            )
            .at(location)
            .into());
        }
        if matches!(
            self.expr_kind,
            ExprKind::IndexExpression | ExprKind::IndexPredicate
        ) {
            let what = if self.expr_kind == ExprKind::IndexExpression {
                "index expressions"
            } else {
                "index predicates"
            };
            return Err(Diagnostic::new(
                "0A000",
                format!("set-returning functions are not allowed in {what}"),
            )
            .at(location)
            .into());
        }
        if self.expr_kind.refuses_srf() {
            return Err(Diagnostic::new(
                "0A000",
                format!(
                    "set-returning functions are not allowed in {}",
                    self.expr_kind.name()
                ),
            )
            .at(location)
            .into());
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_call_decorated(
        &mut self,
        site: CallSite,
        name: ObjectName,
        arg_handles: &[Handle<FlatExpr>],
        args: &[Typed],
        argnames: &[String],
        location: usize,
        projection: Option<Projection>,
        deco: Decoration,
        srf_before: Option<usize>,
    ) -> Result<Typed> {
        match self
            .resolver
            .parse_func(&name, args, argnames, location, projection, deco)?
        {
            ResolvedFunc::Window {
                func,
                rettype,
                retset,
                declared,
                is_aggregate,
            } => {
                // ParseFuncOrColumn's checks on a WindowFunc.
                let span = self.site_span(site);
                if self.has_window_funcs
                    && let Some(&inner) = self
                        .window_locations
                        .iter()
                        .find(|&&l| l > span.start && l < span.end)
                {
                    return Err(
                        Diagnostic::new("42P20", "window function calls cannot be nested")
                            .at(inner)
                            .into(),
                    );
                }
                if deco.distinct {
                    return Err(Diagnostic::new(
                        "0A000",
                        "DISTINCT is not implemented for window functions",
                    )
                    .at(location)
                    .into());
                }
                if is_aggregate && args.is_empty() && !deco.star {
                    return Err(Diagnostic::new(
                        "42809",
                        format!(
                            "{}(*) must be used to call a parameterless aggregate function",
                            name.display()
                        ),
                    )
                    .at(location)
                    .into());
                }
                if deco.order_by {
                    return Err(Diagnostic::new(
                        "0A000",
                        "aggregate ORDER BY is not implemented for window functions",
                    )
                    .at(location)
                    .into());
                }
                if !is_aggregate && deco.filter {
                    return Err(Diagnostic::new(
                        "0A000",
                        "FILTER is not implemented for non-aggregate window functions",
                    )
                    .at(location)
                    .into());
                }
                if self.last_srf != srf_before {
                    return Err(Diagnostic::new(
                        "0A000",
                        "window function calls cannot contain set-returning function calls",
                    )
                    .hint("You might be able to move the set-returning function into a LATERAL FROM item.")
                    .at(self.last_srf.unwrap_or(location))
                    .into());
                }
                if retset {
                    return Err(
                        Diagnostic::new("42P13", "window functions cannot return sets")
                            .at(location)
                            .into(),
                    );
                }
                for (i, (&a, arg)) in arg_handles.iter().zip(args).enumerate() {
                    if i < declared.len() && arg.ty != declared[i] {
                        let (_, steps) = self.coercer().coerce_type(
                            arg,
                            declared[i],
                            -1,
                            CoercionContext::Implicit,
                        )?;
                        self.record_coercions(a, steps);
                    }
                }
                self.record_function(
                    site,
                    FunctionResolution {
                        function: func,
                        declared,
                        returns_set: false,
                        window: true,
                    },
                );
                Ok(Typed {
                    ty: rettype,
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            ResolvedFunc::Aggregate {
                func,
                rettype,
                declared,
                kind,
                direct_args,
            } => {
                // unify_hypothetical_args: each hypothetical direct
                // argument and its ordering column take a common type.
                let mut args: Vec<Typed> = args.to_vec();
                if kind == pgtk_catalog::AggregateKind::HypotheticalSet {
                    let aggregated = arg_handles.len() - direct_args;
                    let non_hypothetical = direct_args - aggregated;
                    let any = self.types().builtin(Builtin::Any);
                    for hargpos in non_hypothetical..direct_args {
                        let aargpos = direct_args + (hargpos - non_hypothetical);
                        if declared[hargpos] != any {
                            continue;
                        }
                        let (aarg, harg) = (args[aargpos].clone(), args[hargpos].clone());
                        let common = match self.coercer().common_type(&[aarg.ty, harg.ty]) {
                            Ok(t) => t,
                            Err((p, n)) => {
                                let types = self.types();
                                return Err(Diagnostic::new(
                                    "42804",
                                    format!(
                                        "WITHIN GROUP types {} and {} cannot be matched",
                                        types.format(p),
                                        types.format(n)
                                    ),
                                )
                                .at(harg.location)
                                .into());
                            }
                        };
                        for pos in [hargpos, aargpos] {
                            let steps = self.coercer().coerce_to_common_type(
                                &args[pos],
                                common,
                                "WITHIN GROUP",
                            )?;
                            self.record_coercions(arg_handles[pos], steps);
                            args[pos].ty = common;
                        }
                    }
                }
                let args = &args;
                for (i, (&a, arg)) in arg_handles.iter().zip(args).enumerate() {
                    if arg.returns_set {
                        return Err(Diagnostic::new(
                            "0A000",
                            "aggregate function calls cannot contain set-returning function calls",
                        )
                        .hint("You might be able to move the set-returning function into a LATERAL FROM item.")
                        .at(arg.location)
                        .into());
                    }
                    if i < declared.len() && arg.ty != declared[i] {
                        let (_, steps) = self.coercer().coerce_type(
                            arg,
                            declared[i],
                            -1,
                            CoercionContext::Implicit,
                        )?;
                        self.record_coercions(a, steps);
                    }
                }
                self.record_function(
                    site,
                    FunctionResolution {
                        function: func,
                        declared,
                        returns_set: false,
                        window: false,
                    },
                );
                Ok(Typed {
                    ty: rettype,
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: false,
                })
            }
            ResolvedFunc::Projection(p) => Ok(Typed {
                ty: p.ty,
                typmod: p.typmod,
                form: Form::Other,
                location,
                returns_set: false,
            }),
            ResolvedFunc::Coercion { target } => {
                let (result, steps) =
                    self.coercer()
                        .coerce_type(&args[0], target, -1, CoercionContext::Explicit)?;
                self.record_coercions(arg_handles[0], steps);
                Ok(Typed { location, ..result })
            }
            ResolvedFunc::Function {
                func,
                rettype,
                retset,
                declared,
            } => {
                for (i, (&a, arg)) in arg_handles.iter().zip(args).enumerate() {
                    if arg.ty != declared[i] {
                        let (_, steps) = self.coercer().coerce_type(
                            arg,
                            declared[i],
                            -1,
                            CoercionContext::Implicit,
                        )?;
                        self.record_coercions(a, steps);
                    }
                }
                if retset {
                    self.check_srf_placement(location, srf_before)?;
                }
                self.record_function(
                    site,
                    FunctionResolution {
                        function: func,
                        declared,
                        returns_set: retset,
                        window: false,
                    },
                );
                Ok(Typed {
                    ty: rettype,
                    typmod: -1,
                    form: Form::Other,
                    location,
                    returns_set: retset,
                })
            }
        }
    }

    /// The operand of a cast: `ARRAY[...]` learns its element type from the
    /// cast's target, as `transformTypeCast` arranges.
    fn cast_operand(
        &mut self,
        scope: &Scope,
        inner: Handle<FlatExpr>,
        cast_type: Handle<pgtk_sql::ast::shared::expr::FlatCastType>,
    ) -> Result<Typed> {
        if let FlatExpr::Array(a) = self.get(inner)
            && let FlatArrayExpr::Bracket(b) = self.get(*a)
        {
            let tn = self.type_name(cast_type)?;
            let (target, target_typmod) = self.typenames.resolve(&tn)?;
            let (base, base_typmod) = self.types().base_type_and_typmod(target, target_typmod);
            if let Some(element) = self.types().element_type(base) {
                let elements = self.array_items(self.get(*b).elements);
                let location = self.span(inner).start;
                let typed = self.array_expr(
                    scope,
                    &elements,
                    location,
                    Some((base, element, base_typmod)),
                )?;
                self.record_type(inner, &typed);
                return Ok(typed);
            }
        }
        self.expr(scope, inner)
    }

    /// `transformTypeCast`.
    fn type_cast(
        &mut self,
        handle: Handle<FlatExpr>,
        inner: Handle<FlatExpr>,
        arg: &Typed,
        cast_type: Handle<pgtk_sql::ast::shared::expr::FlatCastType>,
        cast_location: usize,
    ) -> Result<Typed> {
        let tn = self.type_name(cast_type)?;
        let (target, target_typmod) = self.typenames.resolve(&tn)?;
        let _ = handle;
        match self.coercer().coerce_to_target_type(
            arg,
            target,
            target_typmod,
            CoercionContext::Explicit,
        )? {
            Some((result, steps)) => {
                self.record_coercions(inner, steps);
                Ok(Typed {
                    location: arg.location,
                    ..result
                })
            }
            None => {
                // typeInheritsFrom / typeIsOfTypedTable: inheritance is not
                // in the Catalog, so a cast between two relation row types
                // may be one PostgreSQL allows.
                let types = self.types();
                if types.is_complex(types.base_type(arg.ty))
                    && types.is_complex(types.base_type(target))
                {
                    return Err(unsupported(
                        "row type conversion",
                        Some(cast_location..cast_location),
                    ));
                }
                Err(Diagnostic::new(
                    "42846",
                    format!(
                        "cannot cast type {} to {}",
                        types.format(arg.ty),
                        types.format(target)
                    ),
                )
                .at(cast_location)
                .into())
            }
        }
    }

    /// A typed literal, `int4 '1'`: a cast of an unknown constant whose
    /// location is the string's.
    fn typed_literal(
        &mut self,
        handle: Handle<FlatExpr>,
        f: Handle<FlatTypeCastFunc>,
        location: usize,
    ) -> Result<Typed> {
        let (tn, value) = match self.get(f) {
            FlatTypeCastFunc::Fixed(fixed) => {
                let fixed = self.get(*fixed);
                let (name, typmods) = self.fixed_type(fixed.type_name)?;
                (
                    TypeName {
                        schema: Some("pg_catalog".to_owned()),
                        name: name.to_owned(),
                        typmods,
                        array_dims: 0,
                        location,
                    },
                    fixed.value,
                )
            }
            #[cfg(feature = "since-pg17")]
            FlatTypeCastFunc::Json(j) => (
                TypeName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "json".to_owned(),
                    typmods: Vec::new(),
                    array_dims: 0,
                    location,
                },
                self.get(*j).value,
            ),
            FlatTypeCastFunc::Named(named) => {
                let named = self.get(*named);
                let pgtk_sql::tokens::Flattype_function_name::Text(t) = self.get(named.type_name);
                let name = self.fold(self.text(*t), location)?;
                (
                    TypeName {
                        schema: None,
                        name,
                        typmods: Vec::new(),
                        array_dims: 0,
                        location,
                    },
                    named.value,
                )
            }
        };
        let string_span = self.text_span(value);
        let text = string_literal_body(self.text(value));
        let arg = make_const(self.types(), LiteralKind::String, &text, string_span.start);
        let (target, target_typmod) = self.typenames.resolve(&tn)?;
        match self.coercer().coerce_to_target_type(
            &arg,
            target,
            target_typmod,
            CoercionContext::Explicit,
        )? {
            Some((result, steps)) => {
                self.record_coercions(handle, steps);
                Ok(result)
            }
            None => Err(Diagnostic::new(
                "42846",
                format!(
                    "cannot cast type {} to {}",
                    self.types().format(arg.ty),
                    self.types().format(target)
                ),
            )
            .at(location)
            .into()),
        }
    }

    /// `makeStringConstCast`: an unknown string constant cast to a type,
    /// located at the string.
    fn string_const_cast(
        &mut self,
        handle: Handle<FlatExpr>,
        tn: TypeName,
        value: Text<impl Sized>,
        cast_location: usize,
    ) -> Result<Typed> {
        let string_span = self.text_span(value);
        let text = string_literal_body(self.text(value));
        let arg = make_const(self.types(), LiteralKind::String, &text, string_span.start);
        let (target, target_typmod) = self.typenames.resolve(&tn)?;
        match self.coercer().coerce_to_target_type(
            &arg,
            target,
            target_typmod,
            CoercionContext::Explicit,
        )? {
            Some((result, steps)) => {
                self.record_coercions(handle, steps);
                Ok(result)
            }
            None => Err(Diagnostic::new(
                "42846",
                format!(
                    "cannot cast type {} to {}",
                    self.types().format(arg.ty),
                    self.types().format(target)
                ),
            )
            .at(cast_location)
            .into()),
        }
    }

    /// The elements of an `ARRAY[...]` or of one of its sub-lists.
    fn array_items(&self, elements: Option<Handle<FlatArrayElements>>) -> Vec<ArrayItem> {
        match elements.map(|e| self.get(e)) {
            None => Vec::new(),
            Some(FlatArrayElements::Nested(n)) => self
                .list1(*n)
                .iter()
                .copied()
                .map(ArrayItem::Nested)
                .collect(),
            Some(FlatArrayElements::Exprs(x)) => self
                .list1(*x)
                .iter()
                .copied()
                .map(ArrayItem::Expr)
                .collect(),
        }
    }

    /// `transformArrayExpr` without a target type: the elements share a
    /// common type and the result is its array type.
    fn array_expr(
        &mut self,
        scope: &Scope,
        elements: &[ArrayItem],
        location: usize,
        target: Option<(TypeRef, TypeRef, crate::types::Typmod)>,
    ) -> Result<Typed> {
        let mut multidims = false;
        let mut typed: Vec<(Option<Handle<FlatExpr>>, Typed)> = Vec::new();
        for &e in elements {
            match e {
                ArrayItem::Nested(n) => {
                    let inner = self.array_items(self.get(n).elements);
                    let t = self.array_expr(scope, &inner, location, target)?;
                    multidims = true;
                    typed.push((None, t));
                }
                ArrayItem::Expr(x) => {
                    let t = self.expr(scope, x)?;
                    if !multidims
                        && !self.types().is(t.ty, Builtin::Int2vector)
                        && !self.types().is(t.ty, Builtin::Oidvector)
                        && self.types().is_array(t.ty)
                    {
                        multidims = true;
                    }
                    typed.push((Some(x), t));
                }
            }
        }
        let (array_type, coerce_type, coerce_hard, typmod) =
            if let Some((array, element, typmod)) = target {
                (array, if multidims { array } else { element }, true, typmod)
            } else {
                if typed.is_empty() {
                    return Err(Diagnostic::new("42P18", "cannot determine type of empty array")
                    .hint("Explicitly cast to the desired type, for example ARRAY[]::integer[].")
                    .at(location)
                    .into());
                }
                let type_list: Vec<TypeRef> = typed.iter().map(|(_, t)| t.ty).collect();
                let coerce_type = match self.coercer().common_type(&type_list) {
                    Ok(t) => t,
                    Err((p, n)) => {
                        let offender = typed
                            .iter()
                            .find(|(_, t)| self.types().base_type(t.ty) == n)
                            .map_or(location, |(_, t)| t.location);
                        return Err(Diagnostic::new(
                            "42804",
                            format!(
                                "ARRAY types {} and {} cannot be matched",
                                self.types().format(p),
                                self.types().format(n)
                            ),
                        )
                        .at(offender)
                        .into());
                    }
                };
                let array_type = if multidims {
                    if self.types().element_type(coerce_type).is_none() {
                        return Err(Diagnostic::new(
                            "42704",
                            format!(
                                "could not find element type for data type {}",
                                self.types().format(coerce_type)
                            ),
                        )
                        .at(location)
                        .into());
                    }
                    coerce_type
                } else {
                    let Some(array) = self.types().array_type(coerce_type) else {
                        return Err(Diagnostic::new(
                            "42704",
                            format!(
                                "could not find array type for data type {}",
                                self.types().format(coerce_type)
                            ),
                        )
                        .at(location)
                        .into());
                    };
                    array
                };
                (array_type, coerce_type, false, -1)
            };
        for (h, t) in &typed {
            if coerce_hard {
                let coerced = self.coercer().coerce_to_target_type(
                    t,
                    coerce_type,
                    typmod,
                    CoercionContext::Explicit,
                )?;
                match coerced {
                    Some((_, steps)) => {
                        if let Some(h) = h {
                            self.record_coercions(*h, steps);
                        }
                    }
                    None => {
                        return Err(Diagnostic::new(
                            "42846",
                            format!(
                                "cannot cast type {} to {}",
                                self.types().format(t.ty),
                                self.types().format(coerce_type)
                            ),
                        )
                        .at(t.location)
                        .into());
                    }
                }
            } else if t.ty != coerce_type {
                let steps = self
                    .coercer()
                    .coerce_to_common_type(t, coerce_type, "ARRAY")?;
                if let Some(h) = h {
                    self.record_coercions(*h, steps);
                }
            }
        }
        Ok(Typed {
            ty: array_type,
            typmod: -1,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `transformCaseExpr`.
    fn case_expr(
        &mut self,
        scope: &Scope,
        body: Handle<FlatCaseBody>,
        location: usize,
    ) -> Result<Typed> {
        let (operand, first, rest, else_clause) = match self.get(body) {
            FlatCaseBody::Searched(s) => {
                let s = self.get(*s);
                (None, s.first_arm, s.rest_arms, s.else_clause)
            }
            FlatCaseBody::Simple(s) => {
                let s = self.get(*s);
                (Some(s.operand), s.first_arm, s.rest_arms, s.else_clause)
            }
        };
        // The operand: an unknown one becomes text.
        let placeholder = match operand {
            None => None,
            Some(o) => {
                let mut t = self.expr(scope, o)?;
                if self.types().is_unknown(t.ty) {
                    let text = self.types().builtin(Builtin::Text);
                    let steps = self.coercer().coerce_to_common_type(&t, text, "CASE")?;
                    self.record_coercions(o, steps);
                    t.ty = text;
                }
                Some(t)
            }
        };
        let mut arms = vec![first];
        arms.extend(self.list(rest));
        let mut results: Vec<(Option<Handle<FlatExpr>>, Typed)> = Vec::new();
        for arm in arms {
            let arm = self.get(arm);
            let cond = self.expr(scope, arm.condition)?;
            match &placeholder {
                None => {
                    let (_, steps) = self.coercer().coerce_to_boolean(&cond, "CASE/WHEN")?;
                    self.record_coercions(arm.condition, steps);
                }
                Some(p) => {
                    // placeholder = value, then coerced to boolean.
                    let when_location = self.span(arm.condition).start;
                    let resolved = self.resolver.make_op(
                        &ObjectName {
                            schema: None,
                            name: "=".to_owned(),
                        },
                        Some(p),
                        &cond,
                        when_location,
                    )?;
                    self.record_coercions(arm.condition, resolved.right_coercions);
                    let (_, steps) = self
                        .coercer()
                        .coerce_to_boolean(&resolved.result, "CASE/WHEN")?;
                    let _ = steps;
                }
            }
            let r = self.expr(scope, arm.result)?;
            results.push((Some(arm.result), r));
        }
        let default = match else_clause {
            Some(e) => {
                let e = self.get(e).result;
                (Some(e), self.expr(scope, e)?)
            }
            None => (
                None,
                make_const(self.types(), LiteralKind::Null, "", location),
            ),
        };
        // select_common_type over [default, results...].
        let mut all = vec![default];
        all.extend(results);
        let type_list: Vec<TypeRef> = all.iter().map(|(_, t)| t.ty).collect();
        let ptype = match self.coercer().common_type(&type_list) {
            Ok(t) => t,
            Err((p, n)) => {
                let offender = all
                    .iter()
                    .find(|(_, t)| self.types().base_type(t.ty) == n)
                    .map_or(location, |(_, t)| t.location);
                return Err(Diagnostic::new(
                    "42804",
                    format!(
                        "CASE types {} and {} cannot be matched",
                        self.types().format(p),
                        self.types().format(n)
                    ),
                )
                .at(offender)
                .into());
            }
        };
        for (i, (h, t)) in all.iter().enumerate() {
            if t.ty != ptype {
                let construct = if i == 0 { "CASE/ELSE" } else { "CASE/WHEN" };
                let steps = self.coercer().coerce_to_common_type(t, ptype, construct)?;
                if let Some(h) = h {
                    self.record_coercions(*h, steps);
                }
            }
        }
        let typmod = {
            let first = all[0].1.typmod;
            if all.iter().all(|(_, t)| t.ty == ptype && t.typmod == first) {
                first
            } else {
                -1
            }
        };
        Ok(Typed {
            ty: ptype,
            typmod,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }

    /// `transformAExprIn` for a value list: the non-variable items share a
    /// common type and the test becomes `= ANY (array)`; anything else
    /// becomes a chain of comparisons.
    fn in_list(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        left: Handle<FlatExpr>,
        items: &[Handle<FlatExpr>],
        negated: bool,
        op_location: usize,
    ) -> Result<Typed> {
        let lt = self.expr(scope, left)?;
        let mut typed = Vec::with_capacity(items.len());
        for &i in items {
            typed.push(self.expr(scope, i)?);
        }
        let op = ObjectName {
            schema: None,
            name: if negated { "<>" } else { "=" }.to_owned(),
        };
        // Items containing a column reference are compared one by one.
        let mut nonvars = Vec::new();
        let mut vars = Vec::new();
        for (h, t) in items.iter().zip(&typed) {
            if self.contains_var(*h) {
                vars.push((*h, t.clone()));
            } else {
                nonvars.push((*h, t.clone()));
            }
        }
        let mut remaining: Vec<(Handle<FlatExpr>, Typed)> = Vec::new();
        remaining.extend(vars.iter().cloned());
        let mut array_done = false;
        if nonvars.len() > 1 {
            let mut all_types = vec![lt.ty];
            all_types.extend(nonvars.iter().map(|(_, t)| t.ty));
            let scalar = self.coercer().common_type(&all_types).ok().filter(|&s| {
                all_types.iter().all(|&t| {
                    self.coercer()
                        .can_coerce_type(&[t], &[s], CoercionContext::Implicit)
                })
            });
            let array = scalar
                .filter(|&s| !self.types().is_record(s))
                .and_then(|s| self.types().array_type(s).map(|a| (s, a)));
            if let Some((scalar, array)) = array {
                for (h, t) in &nonvars {
                    if t.ty != scalar {
                        let steps = self.coercer().coerce_to_common_type(t, scalar, "IN")?;
                        self.record_coercions(*h, steps);
                    }
                }
                // make_scalar_array_op: the operator over (left, scalar).
                let _resolved =
                    self.resolver
                        .make_op(&op, Some(&lt), &nonvars[0].1, op_location)?;
                let elem_typed = Typed {
                    ty: scalar,
                    typmod: -1,
                    form: Form::Other,
                    location: op_location,
                    returns_set: false,
                };
                let resolved = self
                    .resolver
                    .make_op(&op, Some(&lt), &elem_typed, op_location)?;
                if !self.types().is(resolved.result.ty, Builtin::Bool) {
                    return Err(Diagnostic::new(
                        "42809",
                        "op ANY/ALL (array) requires operator to yield boolean",
                    )
                    .at(op_location)
                    .into());
                }
                let _ = array;
                self.analysis.operators.insert(handle, resolved.operator);
                self.record_coercions(left, resolved.left_coercions);
                array_done = true;
            } else {
                remaining = vars
                    .iter()
                    .cloned()
                    .chain(nonvars.iter().cloned())
                    .collect();
            }
        } else {
            remaining = vars
                .iter()
                .cloned()
                .chain(nonvars.iter().cloned())
                .collect();
        }
        for (h, t) in &remaining {
            let resolved = self.resolver.make_op(&op, Some(&lt), t, op_location)?;
            let (_, _) = self.coercer().coerce_to_boolean(&resolved.result, "IN")?;
            if !array_done {
                self.analysis.operators.insert(handle, resolved.operator);
            }
            self.record_coercions(*h, resolved.right_coercions);
        }
        Ok(self.bool_result(lt.location))
    }

    /// COALESCE, GREATEST and LEAST: `select_common_type` over the
    /// arguments, then each coerced to it.
    fn common_type_call(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        args: &[Handle<FlatExpr>],
        construct: &str,
        location: usize,
    ) -> Result<Typed> {
        let _ = handle;
        let mut typed = Vec::with_capacity(args.len());
        for &a in args {
            typed.push(self.expr(scope, a)?);
        }
        let types: Vec<TypeRef> = typed.iter().map(|t| t.ty).collect();
        let common = match self.coercer().common_type(&types) {
            Ok(t) => t,
            Err((p, n)) => {
                // The offending expression is the first whose base type is n.
                let offender = typed
                    .iter()
                    .find(|t| self.types().base_type(t.ty) == n)
                    .map_or(location, |t| t.location);
                return Err(Diagnostic::new(
                    "42804",
                    format!(
                        "{construct} types {} and {} cannot be matched",
                        self.types().format(p),
                        self.types().format(n)
                    ),
                )
                .at(offender)
                .into());
            }
        };
        for (&a, t) in args.iter().zip(&typed) {
            if t.ty != common {
                let steps = self.coercer().coerce_to_common_type(t, common, construct)?;
                self.record_coercions(a, steps);
            }
        }
        // select_common_typmod: shared only when every argument has it.
        let typmod = {
            let first = typed.first().map_or(-1, |t| t.typmod);
            if typed.iter().all(|t| t.ty == common && t.typmod == first) {
                first
            } else {
                -1
            }
        };
        Ok(Typed {
            ty: common,
            typmod,
            form: Form::Other,
            location,
            returns_set: false,
        })
    }
}

/// One element of an indirection chain, from any of the AST forms that
/// hold one.
#[derive(Clone, Copy)]
enum IndirPart {
    Field(Handle<pgtk_sql::ast::shared::expr::FlatIndirectionField>),
    Subscript(Handle<FlatBracketSubscript>),
    Star,
}

/// The two operand handles of a fixed-spelling operator variant: `(left,
/// right)` for infix, `(None, operand)` for prefix; `None` for anything
/// else.
fn fixed_operands(node: &FlatExpr) -> Option<(Option<Handle<FlatExpr>>, Handle<FlatExpr>)> {
    use FlatExpr::*;
    match *node {
        GeomCenter(a) | BitNot(a) | PathLength(a) | AtHashAtPrefix(a) | PointCount(a) | Abs(a)
        | BangEqMinusPrefix(a) | Sqrt(a) | Cbrt(a) | IsHorizontal(a) | IsVertical(a) => {
            Some((None, a))
        }
        TildeLeqTilde(a, b)
        | TildeGeqTilde(a, b)
        | TildeLtTilde(a, b)
        | TildeGtTilde(a, b)
        | RegexNotIMatch(a, b)
        | RegexIMatch(a, b)
        | RegexNotMatch(a, b)
        | GeomSame(a, b)
        | RegexMatch(a, b)
        | LikeOpINeg(a, b)
        | LikeOpI(a, b)
        | LikeOpNeg(a, b)
        | LikeOp(a, b)
        | RecordLte(a, b)
        | RecordGte(a, b)
        | RecordNeq(a, b)
        | RecordLt(a, b)
        | RecordGt(a, b)
        | RecordEq(a, b)
        | JsonPathText(a, b)
        | JsonPath(a, b)
        | JsonFieldText(a, b)
        | JsonField(a, b)
        | Parallel(a, b)
        | JsonAnyKey(a, b)
        | JsonAllKeys(a, b)
        | Intersect(a, b)
        | Perpendicular(a, b)
        | Horizontal(a, b)
        | Below(a, b)
        | Above(a, b)
        | JsonKey(a, b)
        | JsonContains(a, b)
        | JsonContainedBy(a, b)
        | TsMatch3(a, b)
        | TripleLt(a, b)
        | StrictlyBelow(a, b)
        | SubsetEq(a, b)
        | Distance(a, b)
        | TripleGt(a, b)
        | SupersetEq(a, b)
        | Adjacent(a, b)
        | StrictlyAbove(a, b)
        | NoExtendBelow(a, b)
        | NoExtendAbove(a, b)
        | TsMatch(a, b)
        | JsonPathExists(a, b)
        | Overlap(a, b)
        | NoExtendRight(a, b)
        | NoExtendLeft(a, b)
        | StrictlyLeft(a, b)
        | StrictlyRight(a, b)
        | TripleEq(a, b)
        | BangEqEq(a, b)
        | GeomClosest(a, b)
        | BangEq(a, b)
        | Neq(a, b)
        | Lte(a, b)
        | Gte(a, b)
        | Eq(a, b)
        | StartsWith(a, b)
        | JsonDeletePath(a, b)
        | Lt(a, b)
        | Gt(a, b)
        | Concat(a, b)
        | BitOr(a, b)
        | BitAnd(a, b)
        | BitXor(a, b)
        | Add(a, b)
        | Sub(a, b)
        | Mul(a, b)
        | Div(a, b)
        | Mod(a, b)
        | Pow(a, b)
        | AtTimeZone(a, b) => Some((Some(a), b)),
        _ => None,
    }
}

fn variant_name(node: &FlatExpr) -> String {
    let full = format!("{node:?}");
    full.split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_owned()
}

fn span_range(span: Span) -> std::ops::Range<usize> {
    span.range()
}

/// The body of a plain string literal: the text between the quotes with
/// doubled quotes collapsed. Other literal forms are returned as written.
fn string_literal_body(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(inner) = trimmed
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
    {
        inner.replace("''", "'")
    } else {
        trimmed.to_owned()
    }
}

/// A `Typed` for a `Var` that no node of the statement carries.
fn typed_var(var: &Var) -> Typed {
    Typed {
        ty: var.ty,
        typmod: var.typmod,
        form: Form::Var,
        location: NO_LOCATION,
        returns_set: false,
    }
}

/// The 1-based position of a `USING` column on one side of a join.
fn find_using_column(colnames: &[String], name: &str, side: &str) -> Result<usize> {
    let mut found: Option<usize> = None;
    for (i, c) in colnames.iter().enumerate() {
        if c == name {
            if found.is_some() {
                return Err(Diagnostic::new(
                    "42702",
                    format!("common column name \"{name}\" appears more than once in {side} table"),
                )
                .into());
            }
            found = Some(i + 1);
        }
    }
    found.ok_or_else(|| {
        Diagnostic::new(
            "42703",
            format!("column \"{name}\" specified in USING clause does not exist in {side} table"),
        )
        .into()
    })
}

/// The result columns a target list describes: the entries that are not
/// resjunk, in the terms a Describe message uses.
fn result_columns(target_list: &[TargetEntry]) -> Vec<ResultColumn> {
    target_list
        .iter()
        .filter(|t| !t.junk)
        .map(|t| ResultColumn {
            name: t.name.clone(),
            ty: t.ty,
            typmod: t.typmod,
            origin: t.origin,
        })
        .collect()
}
