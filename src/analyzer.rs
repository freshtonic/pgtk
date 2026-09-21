//! The Analyzer: the walk over `pg-sql`'s flat AST that ties the resolution
//! algorithms together and records what they decide as Annotations.

use pg_catalog::{Builtin, Catalog};
use pg_sql::FlatAst;
use pg_sql::ast::FlatStatement;
use pg_sql::ast::ddl::table::FlatCreateTableBody;
use pg_sql::ast::dml::select::{
    FlatAlias, FlatColNameTableTail, FlatFromClause, FlatNamedTableRefTail, FlatPlainTableAlias,
    FlatSelectBody, FlatSelectHead, FlatSelectItem, FlatSelectStmt, FlatSelectTargets,
    FlatSimpleTableRef, FlatTableFunctionName, FlatWhereCondition,
};
use pg_sql::ast::dml::select::{
    FlatGroupByItem, FlatLimitOffsetClause, FlatLimitingClause, FlatOrderByClause, FlatSortDir,
};
use pg_sql::ast::dml::values::{FlatSelectClause, FlatSimpleSelect};
use pg_sql::ast::shared::expr::{
    FlatArrayElement, FlatArrayExpr, FlatCaseBody, FlatExtractField, FlatInContent,
    FlatSubstringTail, FlatTrimDir, FlatTrimTail, FlatTrimValuesRest,
};
use pg_sql::ast::shared::expr::{
    FlatBoolTestKind, FlatCastTypeHead, FlatDateTimeCastTypeName, FlatExpr,
    FlatFixedTypeCastFuncName, FlatFuncArg, FlatFunctionCallBody, FlatFunctionCallTail,
    FlatGeneralCastTypeName, FlatIntervalCastTypeModifier, FlatIntervalQualifier, FlatParenContent,
    FlatParenthesizedIndirection, FlatQualifiedRefFirstIndirection, FlatStringLitSeq0,
    FlatTimeZoneQualifier, FlatTypeCastFunc, FlatTypeModifierSign,
};
use pg_sql::ast::shared::names::FlatQualifiedOperatorName;
use pg_sql::ast::utility::copy::FlatCopyBody;
use pg_sql::tokens::FlatColId;
use pg_sql::tokens::literal::FlatAliasName;
use recursa::{Handle, List, List1, Span, Text};

use crate::analysis::{
    Analysis, ExprType, FunctionResolution, ResultColumn, ResultDescriptor, TargetEntry,
};
use crate::coerce::Coercer;
use crate::consts::{LiteralKind, make_const};
use crate::diagnostic::{AnalyzeError, Diagnostic, NO_LOCATION, Result, unsupported};
use crate::func::{Decoration, FuncResolver, ObjectName, Projection, ResolvedFunc};
use crate::relation::{JoinType, NamespaceItem, Relations, RteKind, Scope, Var};
use crate::settings::Settings;
use crate::typename::{TypeName, TypeNames, TypmodArg};
use crate::types::{TypeRef, Types};
use crate::value::{Coercion, CoercionContext, Form, Typed};
use pg_sql::ast::dml::select::{
    FlatJoinCondition, FlatJoinSuffix, FlatJoinType, FlatParenTableBody, FlatTableRef,
    FlatUnqualifiedJoinKind,
};

/// The Analysis of one Statement, keyed by `pg-sql`'s flat handles.
pub type FlatAnalysis = Analysis<Handle<FlatExpr>, Handle<FlatSelectStmt>, Handle<FlatSelectItem>>;

/// The parsed statement Analysis reads.
pub type Statement<'input> = FlatAst<'input, FlatStatement>;

/// What a caller builds once and asks to analyse Statements.
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
    /// from, for cursor positions. `declared_params` gives a Declared
    /// Parameter Type per positional parameter, in `$n` order; a missing or
    /// `None` entry is inferred.
    pub fn analyze(
        &self,
        source: &str,
        statement: &Statement<'_>,
        declared_params: &[Option<TypeRef>],
    ) -> Result<FlatAnalysis> {
        let coercer = Coercer::new(self.catalog);
        let resolver = FuncResolver {
            coercer,
            search_path: &self.settings.search_path,
        };
        let mut walker = Walker {
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
            analysis: FlatAnalysis::default(),
            expr_kind: ExprKind::SelectTarget,
            in_agg_args: 0,
            has_aggs: false,
            agg_locations: Vec::new(),
        };
        *walker.resolver.coercer.params.borrow_mut() = declared_params.to_vec();
        match walker.statement() {
            Ok(()) => Ok(walker.analysis),
            Err(AnalyzeError::Diagnostic(mut d)) => {
                d.resolve(source);
                Err(AnalyzeError::Diagnostic(d))
            }
            Err(e) => Err(e),
        }
    }

    /// Lex, parse and analyse one statement's text. A statement `pg-sql`
    /// rejects is Unsupported here, because this crate does not reproduce
    /// PostgreSQL's syntax errors.
    pub fn analyze_sql(
        &self,
        sql: &str,
        declared_params: &[Option<TypeRef>],
    ) -> Result<FlatAnalysis> {
        let lexed = pg_sql::lex(sql);
        if lexed.errors().next().is_some() {
            return Err(unsupported("statement does not lex", None));
        }
        // A trailing semicolon is the document's terminator, not part of
        // the statement.
        let terminator = lexed
            .tokens()
            .last()
            .filter(|token| token.kind() == pg_sql::TokenKind::SEMI)
            .map(|token| token.span());
        let lexed = match terminator {
            None => lexed,
            Some(terminator) => {
                let mut builder = pg_sql::LexBuilder::new(sql);
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
        let mut input = lexed.input();
        let Ok(statement) = pg_sql::ast::Statement::parse_flat(&mut input) else {
            return Err(unsupported("statement does not parse", None));
        };
        if !input.is_eof() {
            return Err(unsupported("statement does not parse", None));
        }
        self.analyze(sql, &statement, declared_params)
    }
}

/// The walk over one Statement.
struct Walker<'a, 'input, 'c> {
    ast: &'a Statement<'input>,
    source: &'a str,
    resolver: FuncResolver<'c>,
    relations: Relations<'c>,
    typenames: TypeNames<'c>,
    settings: &'a Settings,
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
        )
    }
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
        match pg_sql::ident::fold_identifier(raw) {
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
                let body = self.get(*body);
                if body.for_update.is_some() {
                    return Err(unsupported("FOR UPDATE", None));
                }
                let order_by = body.order_by;
                let limit_offset = body.limit_offset;
                let FlatSelectClause::Simple(simple) = self.get(body.clause) else {
                    return Err(unsupported("parenthesized query", None));
                };
                let FlatSimpleSelect::Body(compound) = self.get(*simple) else {
                    return Err(unsupported("TABLE or parenthesized set operation", None));
                };
                let compound = self.get(*compound);
                if compound.set_op.is_some() {
                    return Err(unsupported("set operation", None));
                }
                let FlatSelectBody::Select(select) = self.get(compound.body) else {
                    return Err(unsupported("VALUES", None));
                };
                let select = *select;
                let target_list = self.select(select, order_by, limit_offset)?;
                self.analysis.result = ResultDescriptor {
                    columns: target_list
                        .iter()
                        .filter(|t| !t.junk)
                        .map(|t| ResultColumn {
                            name: t.name.clone(),
                            ty: t.ty,
                            typmod: t.typmod,
                            origin: t.origin,
                        })
                        .collect(),
                    params: self.finish_params()?,
                };
                self.analysis.target_lists.insert(select, target_list);
                Ok(())
            }
            FlatStatement::With(_) => Err(unsupported("WITH", None)),
            FlatStatement::Insert(_) => Err(unsupported("INSERT", None)),
            FlatStatement::Update(_) => Err(unsupported("UPDATE", None)),
            FlatStatement::Delete(_) => Err(unsupported("DELETE", None)),
            FlatStatement::Merge(_) => Err(unsupported("MERGE", None)),
            // transformStmt rewrites these at parse analysis.
            FlatStatement::Explain(_) => Err(unsupported("EXPLAIN", None)),
            FlatStatement::Declare(_) => Err(unsupported("DECLARE CURSOR", None)),
            FlatStatement::Call(_) => Err(unsupported("CALL", None)),
            FlatStatement::CreateMaterializedView(_) => {
                Err(unsupported("CREATE MATERIALIZED VIEW", None))
            }
            FlatStatement::CreateTable(t) => match self.get(self.get(*t).body) {
                FlatCreateTableBody::AsQuery(_) | FlatCreateTableBody::ColumnsAsQuery(_) => {
                    Err(unsupported("CREATE TABLE AS", None))
                }
                _ => self.utility(),
            },
            // Describe answers these from session state.
            FlatStatement::Execute(_) => Err(unsupported("EXECUTE", None)),
            FlatStatement::Fetch(_) => Err(unsupported("FETCH", None)),
            FlatStatement::Show(_) => Err(unsupported("SHOW", None)),
            // Utility Statements with a query inside them, which Analysis
            // resolves even though parse analysis does not.
            FlatStatement::CreateView(_) => Err(unsupported("CREATE VIEW", None)),
            FlatStatement::CreateRule(_) => Err(unsupported("CREATE RULE", None)),
            FlatStatement::Prepare(_) => Err(unsupported("PREPARE", None)),
            FlatStatement::Copy(c) => match self.get(self.get(*c).body) {
                FlatCopyBody::Query(_) => Err(unsupported("COPY (query)", None)),
                _ => self.utility(),
            },
            _ => self.utility(),
        }
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
    ) -> Result<Vec<TargetEntry>> {
        let stmt = self.get(handle);
        if stmt.window.is_some() {
            return Err(unsupported("WINDOW", None));
        }
        let mut scope = Scope::new();
        let mut distinct = false;
        let mut distinct_on: Option<Vec<Handle<FlatExpr>>> = None;
        let (items, from) = match stmt.head {
            None => (None, None),
            Some(head) => match self.get(head) {
                FlatSelectHead::DistinctOn(d) => {
                    let d = self.get(*d);
                    distinct_on = Some(self.list(self.get(d.qualifier).exprs).to_vec());
                    let list = self.get(d.targets);
                    if list.into.is_some() {
                        return Err(unsupported("SELECT INTO", None));
                    }
                    (Some(list.items), list.from_clause)
                }
                FlatSelectHead::Distinct(d) => {
                    distinct = true;
                    let list = self.get(self.get(*d).targets);
                    if list.into.is_some() {
                        return Err(unsupported("SELECT INTO", None));
                    }
                    (Some(list.items), list.from_clause)
                }
                FlatSelectHead::All(a) => match self.get(*a).targets {
                    None => (None, None),
                    Some(targets) => match self.get(targets) {
                        FlatSelectTargets::Items(list) => {
                            let list = self.get(*list);
                            if list.into.is_some() {
                                return Err(unsupported("SELECT INTO", None));
                            }
                            (Some(list.items), list.from_clause)
                        }
                        FlatSelectTargets::Empty(from) => (None, Some(*from)),
                        FlatSelectTargets::Into(_) => return Err(unsupported("SELECT INTO", None)),
                    },
                },
                FlatSelectHead::Plain(targets) => match self.get(*targets) {
                    FlatSelectTargets::Items(list) => {
                        let list = self.get(*list);
                        if list.into.is_some() {
                            return Err(unsupported("SELECT INTO", None));
                        }
                        (Some(list.items), list.from_clause)
                    }
                    FlatSelectTargets::Empty(from) => (None, Some(*from)),
                    FlatSelectTargets::Into(_) => return Err(unsupported("SELECT INTO", None)),
                },
            },
        };
        if let Some(from) = from {
            self.from_clause(&mut scope, from)?;
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
        if let Some(where_clause) = stmt.where_clause {
            let clause = self.get(where_clause);
            let FlatWhereCondition::Expr(expr) = self.get(clause.condition) else {
                return Err(unsupported("WHERE CURRENT OF", None));
            };
            self.expr_kind = ExprKind::Where;
            let typed = self.expr(&scope, *expr)?;
            let (_, steps) = self.coercer().coerce_to_boolean(&typed, "WHERE")?;
            self.record_coercions(*expr, steps);
        }
        // HAVING, then the sort and group clauses, as transformSelectStmt orders them.
        let mut having_expr = None;
        if let Some(having) = stmt.having {
            let expr = self.get(having).condition;
            self.expr_kind = ExprKind::Having;
            let typed = self.expr(&scope, expr)?;
            let (_, steps) = self.coercer().coerce_to_boolean(&typed, "HAVING")?;
            self.record_coercions(expr, steps);
            having_expr = Some(expr);
        }
        // transformSortClause, with the SQL92 rules.
        let mut sort_refs: Vec<usize> = Vec::new();
        self.expr_kind = ExprKind::OrderBy;
        if let Some(order_by) = order_by {
            let clause = self.get(order_by);
            for item in self.list1(clause.items) {
                let item = self.get(*item);
                if item.using.is_some() {
                    return Err(unsupported("ORDER BY ... USING", None));
                }
                let index = self.find_targetlist_entry_sql92(
                    &scope,
                    item.expr,
                    &mut target_list,
                    &mut target_exprs,
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
                let desc = matches!(item.dir.map(|d| self.get(d)), Some(FlatSortDir::Desc));
                let _ = desc;
                // get_sort_group_operators(needLT or needGT, needEQ).
                match self.types().btree_ordering(entry.ty) {
                    Some(_) => {}
                    None => {
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
        }
        // transformGroupClause without grouping sets.
        let mut group_entries: Vec<GroupEntry> = Vec::new();
        let has_group_by = stmt.group_by.is_some();
        if let Some(group_by) = stmt.group_by {
            let clause = self.get(group_by);
            if clause.modifier.is_some() {
                return Err(unsupported("GROUP BY DISTINCT/ALL", None));
            }
            self.expr_kind = ExprKind::GroupBy;
            for item in self.list1(clause.items) {
                let expr = match self.get(*item) {
                    FlatGroupByItem::Expr(e) => *e,
                    FlatGroupByItem::GroupingSets(_)
                    | FlatGroupByItem::Rollup(_)
                    | FlatGroupByItem::Cube(_)
                    | FlatGroupByItem::Empty(_) => {
                        return Err(unsupported("grouping sets", None));
                    }
                };
                let index = self.find_targetlist_entry_sql92(
                    &scope,
                    expr,
                    &mut target_list,
                    &mut target_exprs,
                    ExprKind::GroupBy,
                )?;
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
                group_entries.push(match var {
                    Some(v) => GroupEntry::Var(v),
                    None => GroupEntry::Expr(entry_expr.unwrap_or(expr)),
                });
            }
        }
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
        // transformLimitClause for LIMIT and OFFSET.
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
            if let Some(limit) = limit {
                match self.get(limit) {
                    FlatLimitingClause::Limit(l) => {
                        let count = self.get(*l).count;
                        self.expr_kind = ExprKind::Limit;
                        self.limit_expr(&scope, count, "LIMIT")?;
                    }
                    FlatLimitingClause::FetchFirst(_) => {
                        return Err(unsupported("FETCH FIRST", None));
                    }
                }
            }
            if let Some(offset) = offset {
                let count = self.get(offset).count;
                self.expr_kind = ExprKind::Offset;
                self.limit_expr(&scope, count, "OFFSET")?;
            }
        }
        // parseCheckAggregates
        if self.has_aggs || has_group_by || having_expr.is_some() {
            let mut checked: Vec<Handle<FlatExpr>> =
                target_exprs.iter().flatten().copied().collect();
            checked.extend(having_expr);
            for expr in checked {
                self.check_ungrouped(&scope, expr, &group_entries)?;
            }
        }
        // resolveTargetListUnknowns
        let text = self.types().builtin(Builtin::Text);
        for entry in target_list.iter_mut() {
            if self.types().is_unknown(entry.ty) {
                entry.ty = text;
                entry.typmod = -1;
            }
        }
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
        if matches!(typed.form, Form::Var)
            || self.analysis.vars.get(expr).is_some()
            || self.contains_var(expr)
        {
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
    fn contains_var(&self, expr: Handle<FlatExpr>) -> bool {
        let span = self.span(expr);
        self.analysis.vars.iter().any(|(h, _)| {
            let s = self.span(h);
            s.start >= span.start && s.end <= span.end
        })
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
        let construct = kind.name();
        let location = self.span(expr).start;
        if let FlatExpr::ColumnRef(c) = self.get(expr) {
            let c = self.get(*c);
            if self.list(c.subscripts).is_empty() {
                let (name, _) = self.col_id(c.name)?;
                // In GROUP BY a name that is a column of the FROM clause is
                // not matched against output names.
                let is_column = kind == ExprKind::GroupBy
                    && self
                        .relations
                        .col_name_to_var(scope, &name, location)?
                        .is_some();
                let mut found: Option<usize> = None;
                let _ = is_column;
                for (i, entry) in target_list.iter().enumerate() {
                    if !is_column && !entry.junk && entry.name == name {
                        if found.is_some() {
                            // Two output columns with the name: ambiguous
                            // unless the expressions are equal, which
                            // needs structural equality this slice lacks.
                            return Err(unsupported(
                                "ambiguous ORDER BY name",
                                Some(location..location),
                            ));
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
            name: String::new(),
            ty: typed.ty,
            typmod: typed.typmod,
            origin: None,
            junk: true,
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
            if self.list(p.indirection).is_empty()
                && let FlatParenContent::Exprs(list) = self.get(p.content)
            {
                let exprs = self.list1(*list);
                if exprs.len() == 1 {
                    return self.plain_var(exprs[0]);
                }
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
            ) => match (self.analysis.vars.get(a), self.analysis.vars.get(b)) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            },
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
            if !self.list(p.indirection).is_empty() {
                return e;
            }
            match self.get(p.content) {
                FlatParenContent::Exprs(list) if self.list1(*list).len() == 1 => {
                    e = self.list1(*list)[0]
                }
                _ => return e,
            }
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
        location: usize,
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
                self.check_ungrouped_var(scope, *input, groups, NO_LOCATION)?;
            }
            return Ok(());
        }
        // check_functional_grouping: every column of a non-deferrable
        // primary key of the same range table entry is grouped.
        let key = rte
            .relation()
            .and_then(|r| self.catalog()[r].primary_key.as_ref())
            .and_then(pg_catalog::PrimaryKey::grouping_columns);
        if let Some(key) = key
            && !key.is_empty()
            && key.iter().all(|&k| {
                groups.iter().any(|g| {
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
        Err(Diagnostic::new(
            "42803",
            format!(
                "column \"{}.{}\" must appear in the GROUP BY clause or be used in an aggregate function",
                rte.eref_name, attname
            ),
        )
        .at(location)
        .into())
    }

    /// `check_ungrouped_columns` over one expression: every column
    /// reference must be grouped, be inside an aggregate, or be part of an
    /// expression equal to a GROUP BY expression.
    fn check_ungrouped(
        &self,
        scope: &Scope,
        expr: Handle<FlatExpr>,
        groups: &[GroupEntry],
    ) -> Result<()> {
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
            return self.check_ungrouped_var(scope, *var, groups, self.span(expr).start);
        }
        // An aggregate's arguments are not checked (only direct arguments
        // of ordered-set aggregates would be).
        if self
            .analysis
            .functions
            .get(expr)
            .is_some_and(|f| self.catalog()[f.function].aggregate.is_some())
        {
            return Ok(());
        }
        for child in self.child_exprs(expr) {
            self.check_ungrouped(scope, child, groups)?;
        }
        Ok(())
    }

    // ----- FROM -----

    /// `transformFromClause`.
    fn from_clause(&mut self, scope: &mut Scope, from: Handle<FlatFromClause>) -> Result<()> {
        let from = self.get(from);
        for table in self.list1(from.tables) {
            let (_, items) = self.table_ref(scope, *table, None)?;
            self.relations
                .check_namespace_conflicts(scope, &scope.namespace, &items)?;
            scope.namespace.extend(items);
        }
        Ok(())
    }

    /// `transformFromClauseItem` for a table reference with its chain of
    /// joins. `alias` is the alias of the parentheses around the chain,
    /// which the grammar hangs on the last join.
    fn table_ref(
        &mut self,
        scope: &mut Scope,
        handle: Handle<FlatTableRef>,
        alias: Option<Handle<FlatPlainTableAlias>>,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        let table_ref = self.get(handle);
        if table_ref.tablesample.is_some() {
            return Err(unsupported("TABLESAMPLE", None));
        }
        let location = self.span(handle).start;
        let mut current = self.simple_table_ref(scope, table_ref.base, location)?;
        let joins = self.list(table_ref.joins);
        if joins.is_empty() && alias.is_some() {
            return Err(unsupported(
                "alias on a parenthesized table reference",
                Some(location..location),
            ));
        }
        for (i, join) in joins.iter().enumerate() {
            let join_alias = if i + 1 == joins.len() { alias } else { None };
            let join_location = self.span(*join).start;
            match self.get(*join) {
                FlatJoinSuffix::Unqualified(u) => {
                    let u = self.get(*u);
                    if u.tablesample.is_some() {
                        return Err(unsupported("TABLESAMPLE", None));
                    }
                    let (join_type, natural) = match self.get(u.kind) {
                        FlatUnqualifiedJoinKind::Cross => (JoinType::Inner, false),
                        FlatUnqualifiedJoinKind::Natural(n) => {
                            (self.join_type(self.get(*n).join_type), true)
                        }
                    };
                    let right_location = self.span(u.table).start;
                    let right = self.simple_table_ref(scope, u.table, right_location)?;
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
                    let right = self.table_ref(scope, q.table, None)?;
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
            for &(li, ri) in &pairs {
                let lt = typed_var(&self.side_var(scope, &l_top, li));
                let rt = typed_var(&self.side_var(scope, &r_top, ri));
                let resolved = self.resolver.make_op(&eq, Some(&lt), &rt, NO_LOCATION)?;
                self.coercer()
                    .coerce_to_boolean(&resolved.result, construct)?;
            }
        } else if let Some(cond) = condition
            && let FlatJoinCondition::On(o) = self.get(cond)
        {
            // transformJoinOnClause: only the two sides are visible.
            let expr = self.get(*o).condition;
            let saved_ns = std::mem::replace(&mut scope.namespace, my_ns.clone());
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
        });
        if let Some(name) = using_alias {
            let item = NamespaceItem {
                rt_index: rtindex,
                rel_visible: true,
                cols_visible: true,
                using_alias: Some((name, using.len())),
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

    /// `transformFromClauseItem` for one table reference without joins.
    fn simple_table_ref(
        &mut self,
        scope: &mut Scope,
        base: Handle<FlatSimpleTableRef>,
        location: usize,
    ) -> Result<(NamespaceItem, Vec<NamespaceItem>)> {
        {
            let (schema, name, inh, alias) = match self.get(base) {
                FlatSimpleTableRef::Named(named) => {
                    let named = self.get(*named);
                    let (schema, name) = match self.get(named.name) {
                        FlatTableFunctionName::Name(n) => {
                            let pg_sql::tokens::Flattable_function_name::Text(t) = self.get(*n);
                            let span = self.text_span(*t);
                            (None, self.fold(self.text(*t), span.start)?)
                        }
                        FlatTableFunctionName::Qualified(q) => {
                            let q = self.get(*q);
                            let (first, _) = self.col_id(q.first)?;
                            let mut parts = vec![first];
                            for part in self.list1(q.tail) {
                                parts.push(self.name_part(*part)?);
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
                    let (inh, alias) = match named.tail.map(|t| self.get(t)) {
                        None => (true, None),
                        Some(FlatNamedTableRefTail::Alias(a)) => (true, Some(*a)),
                        Some(FlatNamedTableRefTail::Inherited(i)) => (true, self.get(*i).alias),
                        Some(FlatNamedTableRefTail::Function(_)) => {
                            return Err(unsupported("function in FROM", Some(location..location)));
                        }
                    };
                    (schema, name, inh, alias)
                }
                FlatSimpleTableRef::Only(only) => {
                    let only = self.get(*only);
                    let qn = self.get(only.name);
                    let (first, _) = self.col_id(qn.first)?;
                    let mut parts = vec![first];
                    for part in self.list(qn.rest) {
                        let part = self.get(*part);
                        parts.push(self.col_label(part.name)?);
                    }
                    if parts.len() > 2 {
                        return Err(unsupported(
                            "catalog-qualified table name",
                            Some(location..location),
                        ));
                    }
                    let name = parts.pop().expect("at least one part");
                    (parts.pop(), name, false, only.alias)
                }
                FlatSimpleTableRef::ColName(c) => {
                    let c = self.get(*c);
                    // A keyword usable as a table name; its spelling is the
                    // source text, folded.
                    let span = self.span(c.name);
                    let raw = &self.source[span.clone()];
                    let name = self.fold(raw, span.start)?;
                    let (inh, alias) = match c.tail.map(|t| self.get(t)) {
                        None => (true, None),
                        Some(FlatColNameTableTail::Alias(a)) => (true, Some(*a)),
                        Some(FlatColNameTableTail::Inherited(i)) => (true, self.get(*i).alias),
                    };
                    (None, name, inh, alias)
                }
                FlatSimpleTableRef::Paren(p) => {
                    let p = self.get(*p);
                    return match self.get(p.body) {
                        FlatParenTableBody::Table(t) => self.table_ref(scope, *t, p.alias),
                        FlatParenTableBody::Query(_) => {
                            Err(unsupported("subquery in FROM", Some(location..location)))
                        }
                    };
                }
                _ => return Err(unsupported("FROM item", Some(location..location))),
            };
            let (alias_name, column_aliases) = self.plain_alias(alias)?;
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
            Ok((item.clone(), vec![item]))
        }
    }

    fn name_part(
        &self,
        handle: Handle<pg_sql::ast::shared::expr::FlatFuncCallNamePart>,
    ) -> Result<String> {
        let span = self.span(handle);
        let raw = self.source[span.clone()].trim_start_matches('.').trim();
        self.fold(raw, span.start)
    }

    fn col_label(&self, handle: Handle<pg_sql::tokens::FlatColLabel>) -> Result<String> {
        let pg_sql::tokens::FlatColLabel::Text(t) = self.get(handle);
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
                let entries = self.expand_all_tables(scope, location)?;
                self.analysis.star_expansions.insert(item, entries.clone());
                out.extend(entries);
                Ok(())
            }
            FlatSelectItem::Expr(e) => {
                let e = self.get(*e);
                // `t.*` inside an expression item.
                if let FlatExpr::QualRef(q_handle) = self.get(e.expr) {
                    let q_handle = *q_handle;
                    let q = self.get(q_handle);
                    let star =
                        matches!(self.get(q.first), FlatQualifiedRefFirstIndirection::Star(_))
                            || self.list(q.rest).last().is_some_and(|last| {
                                matches!(self.get(*last), FlatParenthesizedIndirection::Star(_))
                            });
                    if star && q.call.is_none() && e.alias.is_none() {
                        let entries = self.expand_qualified_star(scope, q_handle)?;
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
                            match w.uescape {
                                None => self.alias_name(w.name)?.0,
                                Some(u) => {
                                    let FlatAliasName::Text(t) = self.get(w.name);
                                    let raw = self.text(*t);
                                    let escape =
                                        string_literal_body(self.text(self.get(u).escape_char));
                                    match pg_sql::ident::fold_identifier_with_uescape(raw, &escape)
                                    {
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
                            let pg_sql::tokens::literal::FlatSelectBareAliasName::Text(t) =
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
                out.push(TargetEntry {
                    name,
                    ty: typed.ty,
                    typmod: typed.typmod,
                    origin,
                    junk: false,
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
    ) -> Option<(pg_catalog::Handle<pg_catalog::Relation>, i16)> {
        if let Some(var) = self.analysis.vars.get(expr) {
            let rte = &scope.rtable[var.varno - 1];
            return (var.levelsup == 0)
                .then(|| rte.relation().map(|r| (r, var.varattno)))
                .flatten();
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
                if !self.list(p.indirection).is_empty() {
                    return None;
                }
                match self.get(p.content) {
                    FlatParenContent::Exprs(list) => {
                        let exprs = self.list1(*list);
                        (exprs.len() == 1)
                            .then(|| self.var_origin(scope, exprs[0]))
                            .flatten()
                    }
                    FlatParenContent::Subquery(_) => None,
                }
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
            out.extend(self.expand_ns_item(scope, item));
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

    /// `expandNSItemAttrs` for a relation entry.
    fn expand_ns_item(&self, scope: &Scope, item: &NamespaceItem) -> Vec<TargetEntry> {
        let rte = &scope.rtable[item.rt_index - 1];
        let mut out = Vec::new();
        let ncols = item
            .using_alias
            .as_ref()
            .map_or(rte.colnames.len(), |(_, n)| *n);
        for (i, name) in rte.colnames[..ncols].iter().enumerate() {
            if name.is_empty() {
                continue;
            }
            let var = self.relations.make_var(item.rt_index, rte, i as i16 + 1, 0);
            out.push(TargetEntry {
                name: name.clone(),
                ty: var.ty,
                typmod: var.typmod,
                origin: scope.rtable[var.varno - 1]
                    .relation()
                    .map(|r| (r, var.varattno)),
                junk: false,
            });
        }
        out
    }

    /// `ExpandColumnRefStar` for `t.*` and `s.t.*`.
    fn expand_qualified_star(
        &self,
        scope: &Scope,
        q: Handle<pg_sql::ast::shared::expr::FlatQualifiedRef>,
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
        let item = self
            .relations
            .refname_namespace_item(scope, schema, relname, location)?;
        let Some(item) = item else {
            return Err(self
                .relations
                .error_missing_rte(scope, schema, relname, location)
                .into());
        };
        Ok(self.expand_ns_item(scope, item))
    }

    /// The dotted names of a qualified reference, and whether it ends in `*`.
    fn qualified_names(
        &self,
        q: Handle<pg_sql::ast::shared::expr::FlatQualifiedRef>,
    ) -> Result<(Vec<String>, bool)> {
        let q = self.get(q);
        let (table, _) = self.col_id(q.table)?;
        let mut names = vec![table];
        let mut star = false;
        match self.get(q.first) {
            FlatQualifiedRefFirstIndirection::Field(f) => {
                names.push(self.alias_name(self.get(*f).name)?.0)
            }
            FlatQualifiedRefFirstIndirection::Star(_) => star = true,
        }
        for rest in self.list(q.rest) {
            if star {
                return Err(unsupported("indirection after .*", None));
            }
            match self.get(*rest) {
                FlatParenthesizedIndirection::Field(f) => {
                    names.push(self.alias_name(self.get(*f).name)?.0)
                }
                FlatParenthesizedIndirection::Star(_) => star = true,
                FlatParenthesizedIndirection::Subscript(_) => {
                    return Err(unsupported("subscript", None));
                }
            }
        }
        Ok((names, star))
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
                let name = self.col_id(c.name).ok().map(|(n, _)| n);
                Colname { name, strength: 2 }
            }
            FlatExpr::QualRef(q) => {
                let q_handle = *q;
                if self.get(q_handle).call.is_some() {
                    return Colname {
                        name: self
                            .qualified_names(q_handle)
                            .ok()
                            .and_then(|(n, _)| n.last().cloned()),
                        strength: 2,
                    };
                }
                match self.qualified_names(q_handle) {
                    Ok((names, _)) => Colname {
                        name: names.last().cloned(),
                        strength: 2,
                    },
                    Err(_) => none,
                }
            }
            FlatExpr::Func(f) => {
                let f = self.get(*f);
                let pg_sql::tokens::Flattype_function_name::Text(t) = self.get(f.name);
                let name = pg_sql::ident::fold_identifier(self.text(*t))
                    .ok()
                    .map(|f| f.name.into_owned());
                Colname { name, strength: 2 }
            }
            FlatExpr::NullIf(_) => Colname {
                name: Some("nullif".to_owned()),
                strength: 2,
            },
            FlatExpr::AtTimeZone(..) | FlatExpr::AtLocal(_) => Colname {
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
            FlatExpr::CastFunc(f) => Colname {
                name: self.typed_literal_type_name(*f),
                strength: 1,
            },
            FlatExpr::Collate(inner, _) => self.figure_colname(*inner),
            FlatExpr::Parenthesized(p) => {
                let p = self.get(*p);
                if !self.list(p.indirection).is_empty() {
                    // A_Indirection: the last field name, if any.
                    let mut name = None;
                    for ind in self.list(p.indirection) {
                        if let FlatParenthesizedIndirection::Field(f) = self.get(*ind) {
                            name = self.alias_name(self.get(*f).name).ok().map(|(n, _)| n);
                        }
                    }
                    if name.is_some() {
                        return Colname { name, strength: 2 };
                    }
                }
                match self.get(p.content) {
                    FlatParenContent::Exprs(list) => {
                        let exprs = self.list1(*list);
                        if exprs.len() == 1 {
                            self.figure_colname(exprs[0])
                        } else {
                            Colname {
                                name: Some("row".to_owned()),
                                strength: 2,
                            }
                        }
                    }
                    FlatParenContent::Subquery(_) => none,
                }
            }
            FlatExpr::PositionalParam(_) => none,
            _ => none,
        }
    }

    fn case_else(
        &self,
        body: Handle<pg_sql::ast::shared::expr::FlatCaseBody>,
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
        cast_type: Handle<pg_sql::ast::shared::expr::FlatCastType>,
    ) -> Option<String> {
        let tn = self.type_name(cast_type).ok()?;
        Some(tn.name)
    }

    fn typed_literal_type_name(&self, f: Handle<FlatTypeCastFunc>) -> Option<String> {
        match self.get(f) {
            FlatTypeCastFunc::Fixed(fixed) => {
                let fixed = self.get(*fixed);
                Some(
                    match self.get(fixed.type_name) {
                        FlatFixedTypeCastFuncName::DoublePrecision => "float8",
                        FlatFixedTypeCastFuncName::Boolean => "bool",
                        FlatFixedTypeCastFuncName::Integer | FlatFixedTypeCastFuncName::Int => {
                            "int4"
                        }
                        FlatFixedTypeCastFuncName::Numeric => "numeric",
                        FlatFixedTypeCastFuncName::Varchar => "varchar",
                        FlatFixedTypeCastFuncName::Bit => {
                            if fixed.varying {
                                "varbit"
                            } else {
                                "bit"
                            }
                        }
                        FlatFixedTypeCastFuncName::Character => {
                            if fixed.varying {
                                "varchar"
                            } else {
                                "bpchar"
                            }
                        }
                    }
                    .to_owned(),
                )
            }
            FlatTypeCastFunc::Json(_) => Some("json".to_owned()),
            FlatTypeCastFunc::Named(named) => {
                let named = self.get(*named);
                let pg_sql::tokens::Flattype_function_name::Text(t) = self.get(named.type_name);
                pg_sql::ident::fold_identifier(self.text(*t))
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
        cast_type: Handle<pg_sql::ast::shared::expr::FlatCastType>,
    ) -> Result<TypeName> {
        let ct = self.get(cast_type);
        let location = self.span(cast_type).start;
        let mut array_dims = self.list(ct.array_suffixes).len();
        if let Some(kw) = ct.array_kw_suffix {
            let _ = self.get(kw);
            array_dims += 1;
        }
        let (schema, name, typmods) = match self.get(ct.head) {
            FlatCastTypeHead::General(g) => {
                let g = self.get(*g);
                let typmods = match g.precision {
                    Some(p) => self.typmod_args(p)?,
                    None => Vec::new(),
                };
                let (schema, name) = match self.get(g.base) {
                    FlatGeneralCastTypeName::Bool | FlatGeneralCastTypeName::Boolean => {
                        (Some("pg_catalog"), "bool".to_owned())
                    }
                    FlatGeneralCastTypeName::Text => (Some("pg_catalog"), "text".to_owned()),
                    FlatGeneralCastTypeName::Integer | FlatGeneralCastTypeName::Int => {
                        (Some("pg_catalog"), "int4".to_owned())
                    }
                    FlatGeneralCastTypeName::Serial => (None, "serial".to_owned()),
                    FlatGeneralCastTypeName::Numeric => (Some("pg_catalog"), "numeric".to_owned()),
                    FlatGeneralCastTypeName::Varchar => (Some("pg_catalog"), "varchar".to_owned()),
                    FlatGeneralCastTypeName::DoublePrecision => {
                        (Some("pg_catalog"), "float8".to_owned())
                    }
                    FlatGeneralCastTypeName::Bit => (
                        Some("pg_catalog"),
                        if g.varying { "varbit" } else { "bit" }.to_owned(),
                    ),
                    FlatGeneralCastTypeName::Character => (
                        Some("pg_catalog"),
                        if g.varying { "varchar" } else { "bpchar" }.to_owned(),
                    ),
                    FlatGeneralCastTypeName::Unknown => (Some("pg_catalog"), "unknown".to_owned()),
                    FlatGeneralCastTypeName::Ident(ident) => {
                        let ident = self.get(*ident);
                        let mut parts = Vec::new();
                        let mut raw_first = "";
                        for part in self.list1(ident.parts) {
                            let pg_sql::tokens::Flattype_name_ident::Text(t) = self.get(*part);
                            let span = self.text_span(*t);
                            if parts.is_empty() {
                                raw_first = self.text(*t);
                            }
                            parts.push(self.fold(self.text(*t), span.start)?);
                        }
                        if parts.len() > 2 {
                            return Err(unsupported(
                                "catalog-qualified type name",
                                Some(location..location),
                            ));
                        }
                        // gram.y's SystemTypeName: an unquoted SQL type
                        // keyword names a pg_catalog type.
                        if parts.len() == 1
                            && !raw_first.starts_with('"')
                            && let Some((name, typmods)) = self.system_type_name(
                                &parts[0],
                                g.varying,
                                typmods.clone(),
                                location,
                            )?
                        {
                            return Ok(TypeName {
                                schema: Some("pg_catalog".to_owned()),
                                name,
                                typmods,
                                array_dims,
                                location,
                            });
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
                let typmods = match d.precision {
                    Some(p) => self.typmod_args(p)?,
                    None => Vec::new(),
                };
                (Some("pg_catalog"), name.to_owned(), typmods)
            }
            FlatCastTypeHead::Interval(i) => {
                let i = self.get(*i);
                let typmods = match i.modifier.map(|m| self.get(m)) {
                    None => Vec::new(),
                    Some(FlatIntervalCastTypeModifier::Precision(p)) => {
                        let mut v = vec![TypmodArg::Integer(0x7fff)];
                        v.extend(self.typmod_args(*p)?);
                        v
                    }
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

    /// `SystemTypeName` for the SQL keywords `gram.y` maps itself, with
    /// `FLOAT(p)`'s precision rule. `None` when the word is an ordinary name.
    fn system_type_name(
        &self,
        word: &str,
        varying: bool,
        typmods: Vec<TypmodArg>,
        location: usize,
    ) -> Result<Option<(String, Vec<TypmodArg>)>> {
        let name = match word {
            "bigint" => "int8",
            "smallint" => "int2",
            "int" | "integer" => "int4",
            "real" => "float4",
            "boolean" => "bool",
            "decimal" | "dec" | "numeric" => "numeric",
            "varchar" => "varchar",
            "char" | "character" | "nchar" => {
                if varying {
                    "varchar"
                } else {
                    "bpchar"
                }
            }
            "float" => {
                let precision = match typmods.first() {
                    Some(TypmodArg::Integer(p)) => *p,
                    None => return Ok(Some(("float8".to_owned(), Vec::new()))),
                    Some(_) => return Ok(None),
                };
                if precision < 1 {
                    return Err(Diagnostic::new(
                        "22023",
                        "precision for type float must be at least 1 bit",
                    )
                    .at(location)
                    .into());
                }
                if precision <= 24 {
                    return Ok(Some(("float4".to_owned(), Vec::new())));
                }
                if precision <= 53 {
                    return Ok(Some(("float8".to_owned(), Vec::new())));
                }
                return Err(Diagnostic::new(
                    "22023",
                    "precision for type float must be less than 54 bits",
                )
                .at(location)
                .into());
            }
            _ => return Ok(None),
        };
        Ok(Some((name.to_owned(), typmods)))
    }

    fn interval_qualifier(&self, q: Handle<FlatIntervalQualifier>) -> (i32, Option<i64>) {
        const YEAR: i32 = 4;
        const MONTH: i32 = 2;
        const DAY: i32 = 8;
        const HOUR: i32 = 1024;
        const MINUTE: i32 = 2048;
        const SECOND: i32 = 4096;
        let precision =
            |h: &Handle<pg_sql::ast::shared::expr::FlatSecondWithPrecision>| -> Option<i64> {
                let p = self.get(*h).precision?;
                match self.typmod_args(p).ok()?.first() {
                    Some(TypmodArg::Integer(v)) => Some(*v),
                    _ => None,
                }
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

    fn typmod_args(
        &self,
        p: Handle<pg_sql::ast::shared::expr::FlatTypePrecision>,
    ) -> Result<Vec<TypmodArg>> {
        let p = self.get(p);
        let mut out = Vec::new();
        for arg in self.list(p.0) {
            let arg = self.get(*arg);
            let raw = self.text(arg.value);
            let negative = matches!(
                arg.sign.map(|s| self.get(s)),
                Some(FlatTypeModifierSign::Neg)
            );
            match crate::consts::integer_value(raw) {
                Some(v) => out.push(TypmodArg::Integer(if negative { -v } else { v })),
                None => out.push(TypmodArg::Ident(raw.to_owned())),
            }
        }
        Ok(out)
    }

    // ----- expressions -----

    /// `transformExprRecurse`. Records the expression's type and returns
    /// its type facts.
    fn expr(&mut self, scope: &Scope, handle: Handle<FlatExpr>) -> Result<Typed> {
        let typed = self.expr_inner(scope, handle)?;
        self.record_type(handle, &typed);
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
            FlatExpr::Default => Err(unsupported("DEFAULT", Some(location..location))),
            FlatExpr::PositionalParam(p) => {
                let p = self.get(*p);
                if !self.list(p.subscripts).is_empty() {
                    return Err(unsupported(
                        "subscripted parameter",
                        Some(location..location),
                    ));
                }
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
                if !self.list(c.subscripts).is_empty() {
                    return Err(unsupported("subscript", Some(location..location)));
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
                if let Some(call) = self.get(q_handle).call {
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
                let (names, star) = self.qualified_names(q_handle)?;
                if star {
                    return Err(unsupported("whole-row reference", Some(location..location)));
                }
                self.column_ref(scope, handle, &names, location)
            }
            FlatExpr::Func(f) => {
                let f = self.get(*f);
                let pg_sql::tokens::Flattype_function_name::Text(t) = self.get(f.name);
                let name = ObjectName {
                    schema: None,
                    name: self.fold(self.text(*t), location)?,
                };
                self.func_call(scope, handle, name, f.tail, location)
            }
            FlatExpr::Parenthesized(p) => {
                let p = self.get(*p);
                if !self.list(p.indirection).is_empty() {
                    return Err(unsupported(
                        "indirection on a parenthesized expression",
                        Some(location..location),
                    ));
                }
                match self.get(p.content) {
                    FlatParenContent::Exprs(list) => {
                        let exprs = self.list1(*list);
                        if exprs.len() != 1 {
                            return Err(unsupported("row constructor", Some(location..location)));
                        }
                        self.expr(scope, exprs[0])
                    }
                    FlatParenContent::Subquery(_) => {
                        Err(unsupported("subquery", Some(location..location)))
                    }
                }
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
                if inner.collate.is_some() {
                    return Err(unsupported("COLLATE", Some(location..location)));
                }
                let (value, target) = (inner.value, inner.target);
                let arg = self.cast_operand(scope, value, target)?;
                self.type_cast(handle, value, &arg, target, location)
            }
            FlatExpr::CastFunc(f) => self.typed_literal(handle, *f, location),
            FlatExpr::TimestampLit(t) => {
                let t = self.get(*t);
                let tz = matches!(t.tz.map(|z| self.get(z)), Some(FlatTimeZoneQualifier::With));
                let typmods = match t.precision {
                    Some(p) => self.typmod_args(p)?,
                    None => Vec::new(),
                };
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
                let typmods = match t.precision {
                    Some(p) => self.typmod_args(p)?,
                    None => Vec::new(),
                };
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
                if let Some(p) = i.precision {
                    typmods.push(TypmodArg::Integer(0x7fff));
                    typmods.extend(self.typmod_args(p)?);
                } else if let Some(q) = i.qualifier {
                    let (fields, precision) = self.interval_qualifier(q);
                    typmods.push(TypmodArg::Integer(i64::from(fields)));
                    if let Some(p) = precision {
                        typmods.push(TypmodArg::Integer(p));
                    }
                }
                let tn = TypeName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "interval".to_owned(),
                    typmods,
                    array_dims: 0,
                    location,
                };
                self.string_const_cast(handle, tn, i.value, location)
            }
            FlatExpr::Array(a) => match self.get(*a) {
                FlatArrayExpr::Subquery(_) => {
                    Err(unsupported("ARRAY(subquery)", Some(location..location)))
                }
                FlatArrayExpr::Bracket(b) => {
                    let elements = self.list(self.get(*b).elements).to_vec();
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
                let items = match self.get(content) {
                    FlatInContent::Subquery(_) => {
                        return Err(unsupported("IN (subquery)", Some(location..location)));
                    }
                    FlatInContent::Exprs(list) => self.list1(*list).to_vec(),
                };
                let op_location = self.next_token_start(self.span(l).end);
                self.in_list(scope, handle, l, &items, false, op_location)
            }
            FlatExpr::NotInExpr(l, suffix) => {
                let (l, suffix) = (*l, *suffix);
                let content = self.get(self.get(suffix).list).0;
                let items = match self.get(content) {
                    FlatInContent::Subquery(_) => {
                        return Err(unsupported("NOT IN (subquery)", Some(location..location)));
                    }
                    FlatInContent::Exprs(list) => self.list1(*list).to_vec(),
                };
                let op_location = self.next_token_start(self.span(l).end);
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
            FlatExpr::AtLocal(l) => {
                let l = *l;
                let lt = self.expr(scope, l)?;
                let at = self.next_token_start(self.span(l).end);
                let name = ObjectName {
                    schema: Some("pg_catalog".to_owned()),
                    name: "timezone".to_owned(),
                };
                let typed = self.resolve_call(handle, name, &[l], &[lt.clone()], at, None)?;
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
                        pg_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfc => "NFC",
                        pg_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfd => "NFD",
                        pg_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfkc => "NFKC",
                        pg_sql::ast::shared::expr::FlatUnicodeNormalForm::Nfkd => "NFKD",
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
        op: Handle<pg_sql::ast::shared::expr::FlatDecoratedOperator>,
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
                    let pg_sql::tokens::literal::FlatIdent::Text(t) = self.get(prefix.name);
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
                if !self.list(p.indirection).is_empty() {
                    return None;
                }
                match self.get(p.content) {
                    FlatParenContent::Exprs(list) => {
                        let exprs = self.list1(*list);
                        if exprs.len() == 1 {
                            self.negatable_literal(exprs[0])
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
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
        let resolved = self.resolver.make_op(&name, Some(lt), rt, op_location)?;
        if resolved.result.returns_set {
            self.check_srf_placement(op_location)?;
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
                match self.relations.col_name_to_var(scope, colname, location)? {
                    Some(var) => var,
                    None => {
                        // A whole-row reference to a table of that name.
                        if self
                            .relations
                            .refname_namespace_item(scope, None, colname, location)?
                            .is_some()
                        {
                            return Err(unsupported(
                                "whole-row reference",
                                Some(location..location),
                            ));
                        }
                        return Err(self
                            .relations
                            .error_missing_column(None, colname, location)
                            .into());
                    }
                }
            }
            [relname, colname] => {
                let item = self
                    .relations
                    .refname_namespace_item(scope, None, relname, location)?;
                let Some(item) = item else {
                    return Err(self
                        .relations
                        .error_missing_rte(scope, None, relname, location)
                        .into());
                };
                match self
                    .relations
                    .scan_ns_item_for_column(scope, item, 0, colname, location)?
                {
                    Some(var) => var,
                    None => {
                        return Err(unsupported(
                            "column notation on a whole-row reference",
                            Some(location..location),
                        ));
                    }
                }
            }
            [nspname, relname, colname] => {
                let item = self.relations.refname_namespace_item(
                    scope,
                    Some(nspname),
                    relname,
                    location,
                )?;
                let Some(item) = item else {
                    return Err(self
                        .relations
                        .error_missing_rte(scope, Some(nspname), relname, location)
                        .into());
                };
                match self
                    .relations
                    .scan_ns_item_for_column(scope, item, 0, colname, location)?
                {
                    Some(var) => var,
                    None => {
                        return Err(unsupported(
                            "column notation on a whole-row reference",
                            Some(location..location),
                        ));
                    }
                }
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
                let item = self.relations.refname_namespace_item(
                    scope,
                    Some(nspname),
                    relname,
                    location,
                )?;
                let Some(item) = item else {
                    return Err(self
                        .relations
                        .error_missing_rte(scope, Some(nspname), relname, location)
                        .into());
                };
                match self
                    .relations
                    .scan_ns_item_for_column(scope, item, 0, colname, location)?
                {
                    Some(var) => var,
                    None => {
                        return Err(unsupported(
                            "column notation on a whole-row reference",
                            Some(location..location),
                        ));
                    }
                }
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
        self.analysis.vars.insert(handle, var);
        Ok(Typed {
            ty: var.ty,
            typmod: var.typmod,
            form: Form::Var,
            location,
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
            FlatFunctionCallTail::TypedLiteral(_) => {
                return Err(unsupported(
                    "typed literal with modifiers",
                    Some(location..location),
                ));
            }
        };
        let mut deco = Decoration {
            within_group: suffix.within_group.is_some(),
            filter: suffix.filter.is_some(),
            over: suffix.window.is_some(),
            ..Decoration::default()
        };
        let mut arg_handles: Vec<Handle<FlatExpr>> = Vec::new();
        let mut order_by: Option<Handle<FlatOrderByClause>> = None;
        if let Some(body) = suffix.body {
            match self.get(body) {
                FlatFunctionCallBody::Star(_) => deco.star = true,
                FlatFunctionCallBody::Args(args) => {
                    let args = self.get(*args);
                    if args.trailing_variadic.is_some() {
                        return Err(unsupported("VARIADIC", Some(location..location)));
                    }
                    order_by = args.order_by;
                    for arg in self.list1(args.args) {
                        match self.get(*arg) {
                            FlatFuncArg::Plain(e) => arg_handles.push(*e),
                            FlatFuncArg::Named(_) => {
                                return Err(unsupported(
                                    "named argument",
                                    Some(location..location),
                                ));
                            }
                        }
                    }
                }
                FlatFunctionCallBody::All(all) => {
                    let all = self.get(*all);
                    order_by = all.order_by;
                    for arg in self.list1(self.get(all.args).0) {
                        match self.get(*arg) {
                            FlatFuncArg::Plain(e) => arg_handles.push(*e),
                            FlatFuncArg::Named(_) => {
                                return Err(unsupported(
                                    "named argument",
                                    Some(location..location),
                                ));
                            }
                        }
                    }
                }
                FlatFunctionCallBody::Distinct(d) => {
                    let d = self.get(*d);
                    deco.distinct = true;
                    order_by = d.order_by;
                    for arg in self.list1(self.get(d.args).0) {
                        match self.get(*arg) {
                            FlatFuncArg::Plain(e) => arg_handles.push(*e),
                            FlatFuncArg::Named(_) => {
                                return Err(unsupported(
                                    "named argument",
                                    Some(location..location),
                                ));
                            }
                        }
                    }
                }
                FlatFunctionCallBody::LeadingVariadic(_) => {
                    return Err(unsupported("VARIADIC", Some(location..location)));
                }
            }
        }
        deco.order_by = order_by.is_some();
        if deco.over {
            return Err(unsupported(
                "window function call",
                Some(location..location),
            ));
        }
        // MERGE_ACTION() is a keyword form that only a MERGE may use.
        if name.schema.is_none() && name.name == "merge_action" && arg_handles.is_empty() {
            return Err(Diagnostic::new(
                "42601",
                "MERGE_ACTION() can only be used in the RETURNING list of a MERGE command",
            )
            .at(location)
            .into());
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
        let mut args = Vec::with_capacity(arg_handles.len());
        for &a in &arg_handles {
            args.push(self.expr(scope, a)?);
        }
        // WITHIN GROUP (ORDER BY ...) expressions are appended to the
        // arguments of an ordered-set aggregate.
        let mut within_exprs: Vec<Handle<FlatExpr>> = Vec::new();
        if let Some(wg) = suffix.within_group {
            let clause = self.get(self.get(wg).order_by);
            for item in self.list1(clause.items) {
                let item = self.get(*item);
                if item.using.is_some() {
                    return Err(unsupported(
                        "WITHIN GROUP ... USING",
                        Some(location..location),
                    ));
                }
                within_exprs.push(item.expr);
            }
            for &e in &within_exprs {
                arg_handles.push(e);
                args.push(self.expr(scope, e)?);
            }
            deco.aggregated = within_exprs.len();
        }
        // ParseComplexProjection: f(composite) may be the field f.
        let projection = if name.schema.is_none()
            && args.len() == 1
            && suffix.body.is_some()
            && !deco.star
            && !deco.distinct
        {
            self.projection_field(args[0].ty, &name.name)
        } else {
            None
        };
        let typed = self.resolve_call_decorated(
            handle,
            name,
            &arg_handles,
            &args,
            location,
            projection,
            deco,
        )?;
        // An aggregate's ORDER BY, DISTINCT and FILTER.
        if self
            .analysis
            .functions
            .get(handle)
            .is_some_and(|f| self.catalog()[f.function].aggregate.is_some())
        {
            self.aggregate_clauses(
                scope,
                handle,
                &arg_handles,
                &args,
                order_by,
                suffix.filter,
                deco,
                location,
            )?;
        }
        Ok(typed)
    }

    /// `transformAggregateCall` and `check_agglevels_and_constraints` for a
    /// resolved aggregate.
    fn aggregate_clauses(
        &mut self,
        scope: &Scope,
        handle: Handle<FlatExpr>,
        arg_handles: &[Handle<FlatExpr>],
        args: &[Typed],
        order_by: Option<Handle<FlatOrderByClause>>,
        filter: Option<Handle<pg_sql::ast::shared::expr::FlatFilterClause>>,
        deco: Decoration,
        location: usize,
    ) -> Result<()> {
        let _ = handle;
        // Argument ORDER BY: SQL99 rules; a new expression is a junk entry.
        let mut order_exprs: Vec<Handle<FlatExpr>> = Vec::new();
        if let Some(ob) = order_by {
            let clause = self.get(ob);
            let saved = self.expr_kind;
            self.expr_kind = ExprKind::OrderBy;
            for item in self.list1(clause.items) {
                let item = self.get(*item);
                if item.using.is_some() {
                    return Err(unsupported(
                        "aggregate ORDER BY ... USING",
                        Some(location..location),
                    ));
                }
                let mut t = self.expr(scope, item.expr)?;
                if self.types().is_unknown(t.ty) {
                    let text = self.types().builtin(Builtin::Text);
                    let steps = self.coercer().coerce_to_common_type(&t, text, "ORDER BY")?;
                    self.record_coercions(item.expr, steps);
                    t.ty = text;
                }
                if self.types().btree_ordering(t.ty).is_none() {
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
        if let Some(f) = filter {
            let clause = self.get(self.get(f).body);
            let FlatWhereCondition::Expr(expr) = self.get(clause.condition) else {
                return Err(unsupported(
                    "FILTER (WHERE CURRENT OF)",
                    Some(location..location),
                ));
            };
            let saved = self.expr_kind;
            self.expr_kind = ExprKind::Filter;
            let t = self.expr(scope, *expr)?;
            let (_, steps) = self.coercer().coerce_to_boolean(&t, "FILTER")?;
            self.record_coercions(*expr, steps);
            self.expr_kind = saved;
        }
        // check_agglevels_and_constraints for one query level.
        if self.expr_kind.refuses_aggregate() {
            return Err(Diagnostic::new(
                "42803",
                format!(
                    "aggregate functions are not allowed in {}",
                    self.expr_kind.name()
                ),
            )
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
            pg_catalog::TypeKind::Composite(detail) => detail
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
        self.resolve_call_decorated(
            handle,
            name,
            arg_handles,
            args,
            location,
            projection,
            Decoration::default(),
        )
    }

    /// `check_srf_call_placement` and the aggregate-argument rule for a
    /// call or operator that returns a set.
    fn check_srf_placement(&self, location: usize) -> Result<()> {
        if self.in_agg_args > 0 {
            return Err(Diagnostic::new(
                "0A000",
                "aggregate function calls cannot contain set-returning function calls",
            )
            .hint("You might be able to move the set-returning function into a LATERAL FROM item.")
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

    fn resolve_call_decorated(
        &mut self,
        handle: Handle<FlatExpr>,
        name: ObjectName,
        arg_handles: &[Handle<FlatExpr>],
        args: &[Typed],
        location: usize,
        projection: Option<Projection>,
        deco: Decoration,
    ) -> Result<Typed> {
        match self
            .resolver
            .parse_func(&name, args, location, projection, deco)?
        {
            ResolvedFunc::Aggregate {
                func,
                rettype,
                declared,
                kind,
                direct_args,
            } => {
                let _ = (kind, direct_args);
                // check_agg_arguments: an aggregate inside the arguments.
                let span = self.span(handle);
                if let Some(&inner) = self
                    .agg_locations
                    .iter()
                    .find(|&&l| l > span.start && l < span.end)
                {
                    return Err(Diagnostic::new(
                        "42803",
                        "aggregate function calls cannot be nested",
                    )
                    .at(inner)
                    .into());
                }
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
                self.analysis.functions.insert(
                    handle,
                    FunctionResolution {
                        function: func,
                        declared,
                        returns_set: false,
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
                    self.check_srf_placement(location)?;
                }
                self.analysis.functions.insert(
                    handle,
                    FunctionResolution {
                        function: func,
                        declared,
                        returns_set: retset,
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
        cast_type: Handle<pg_sql::ast::shared::expr::FlatCastType>,
    ) -> Result<Typed> {
        if let FlatExpr::Array(a) = self.get(inner)
            && let FlatArrayExpr::Bracket(b) = self.get(*a)
        {
            let tn = self.type_name(cast_type)?;
            let (target, target_typmod) = self.typenames.resolve(&tn)?;
            let (base, base_typmod) = self.types().base_type_and_typmod(target, target_typmod);
            if let Some(element) = self.types().element_type(base) {
                let elements = self.list(self.get(*b).elements).to_vec();
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
        cast_type: Handle<pg_sql::ast::shared::expr::FlatCastType>,
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
                let name = match self.get(fixed.type_name) {
                    FlatFixedTypeCastFuncName::DoublePrecision => "float8",
                    FlatFixedTypeCastFuncName::Boolean => "bool",
                    FlatFixedTypeCastFuncName::Integer | FlatFixedTypeCastFuncName::Int => "int4",
                    FlatFixedTypeCastFuncName::Numeric => "numeric",
                    FlatFixedTypeCastFuncName::Varchar => "varchar",
                    FlatFixedTypeCastFuncName::Bit => {
                        if fixed.varying {
                            "varbit"
                        } else {
                            "bit"
                        }
                    }
                    FlatFixedTypeCastFuncName::Character => {
                        if fixed.varying {
                            "varchar"
                        } else {
                            "bpchar"
                        }
                    }
                };
                let typmods = match fixed.typmods {
                    Some(p) => self.typmod_args(p)?,
                    None => Vec::new(),
                };
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
                let pg_sql::tokens::Flattype_function_name::Text(t) = self.get(named.type_name);
                (
                    TypeName {
                        schema: None,
                        name: self.fold(self.text(*t), location)?,
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

    /// `transformArrayExpr` without a target type: the elements share a
    /// common type and the result is its array type.
    fn array_expr(
        &mut self,
        scope: &Scope,
        elements: &[Handle<FlatArrayElement>],
        location: usize,
        target: Option<(TypeRef, TypeRef, crate::types::Typmod)>,
    ) -> Result<Typed> {
        let mut multidims = false;
        let mut typed: Vec<(Option<Handle<FlatExpr>>, Typed)> = Vec::new();
        for &e in elements {
            match self.get(e) {
                FlatArrayElement::Nested(n) => {
                    let inner = self.list(self.get(*n).elements).to_vec();
                    let t = self.array_expr(scope, &inner, location, target)?;
                    multidims = true;
                    typed.push((None, t));
                }
                FlatArrayElement::Expr(x) => {
                    let x = *x;
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
                let resolved = self
                    .resolver
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
        | Overlaps(a, b)
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
