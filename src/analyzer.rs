//! The Analyzer: the walk over `pg-sql`'s flat AST that ties the resolution
//! algorithms together and records what they decide as Annotations.

use pg_catalog::{Builtin, Catalog};
use pg_sql::FlatAst;
use pg_sql::ast::FlatStatement;
use pg_sql::ast::dml::select::{
    FlatAlias, FlatColNameTableTail, FlatFromClause, FlatNamedTableRefTail, FlatPlainTableAlias,
    FlatSelectBody, FlatSelectHead, FlatSelectItem, FlatSelectStmt, FlatSelectTargets,
    FlatSimpleTableRef, FlatTableFunctionName, FlatWhereCondition,
};
use pg_sql::ast::dml::values::{FlatSelectClause, FlatSimpleSelect};
use pg_sql::ast::shared::expr::{
    FlatBoolTestKind, FlatCastTypeHead, FlatDateTimeCastTypeName, FlatExpr,
    FlatFixedTypeCastFuncName, FlatFuncArg, FlatFunctionCallBody, FlatFunctionCallTail,
    FlatGeneralCastTypeName, FlatIntervalCastTypeModifier, FlatIntervalQualifier, FlatParenContent,
    FlatParenthesizedIndirection, FlatQualifiedRefFirstIndirection, FlatStringLitSeq0,
    FlatTimeZoneQualifier, FlatTypeCastFunc, FlatTypeModifierSign,
};
use pg_sql::ast::shared::names::FlatQualifiedOperatorName;
use pg_sql::tokens::FlatColId;
use pg_sql::tokens::literal::FlatAliasName;
use recursa::{Handle, List, List1, Span, Text};

use crate::analysis::{
    Analysis, ExprType, FunctionResolution, ResultColumn, ResultDescriptor, TargetEntry,
};
use crate::coerce::Coercer;
use crate::consts::{LiteralKind, make_const};
use crate::diagnostic::{AnalyzeError, Diagnostic, Result, unsupported};
use crate::func::{FuncResolver, ObjectName, ResolvedFunc};
use crate::relation::{NamespaceItem, Relations, Scope};
use crate::settings::Settings;
use crate::typename::{TypeName, TypeNames, TypmodArg};
use crate::types::{TypeRef, Types};
use crate::value::{Coercion, CoercionContext, Form, Typed};

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
                if body.order_by.is_some() {
                    return Err(unsupported("ORDER BY", None));
                }
                if body.limit_offset.is_some() {
                    return Err(unsupported("LIMIT/OFFSET", None));
                }
                if body.for_update.is_some() {
                    return Err(unsupported("FOR UPDATE", None));
                }
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
                let target_list = self.select(select)?;
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
            _ => Err(unsupported("utility statement", None)),
        }
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
    fn select(&mut self, handle: Handle<FlatSelectStmt>) -> Result<Vec<TargetEntry>> {
        let stmt = self.get(handle);
        if stmt.group_by.is_some() {
            return Err(unsupported("GROUP BY", None));
        }
        if stmt.having.is_some() {
            return Err(unsupported("HAVING", None));
        }
        if stmt.window.is_some() {
            return Err(unsupported("WINDOW", None));
        }
        let mut scope = Scope::new();
        let (items, from) = match stmt.head {
            None => (None, None),
            Some(head) => match self.get(head) {
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
                FlatSelectHead::Distinct(_) | FlatSelectHead::DistinctOn(_) => {
                    return Err(unsupported("DISTINCT", None));
                }
            },
        };
        if let Some(from) = from {
            self.from_clause(&mut scope, from)?;
        }
        let mut target_list = Vec::new();
        if let Some(items) = items {
            for item in self.list1(items) {
                self.target_item(&scope, *item, &mut target_list)?;
            }
        }
        // markTargetListOrigins happened as entries were made.
        if let Some(where_clause) = stmt.where_clause {
            let clause = self.get(where_clause);
            let FlatWhereCondition::Expr(expr) = self.get(clause.condition) else {
                return Err(unsupported("WHERE CURRENT OF", None));
            };
            let typed = self.expr(&scope, *expr)?;
            let (_, steps) = self.coercer().coerce_to_boolean(&typed, "WHERE")?;
            self.record_coercions(*expr, steps);
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

    // ----- FROM -----

    fn from_clause(&mut self, scope: &mut Scope, from: Handle<FlatFromClause>) -> Result<()> {
        let from = self.get(from);
        for table in self.list1(from.tables) {
            let table_ref = self.get(*table);
            if table_ref.tablesample.is_some() {
                return Err(unsupported("TABLESAMPLE", None));
            }
            if !self.list(table_ref.joins).is_empty() {
                return Err(unsupported("JOIN", None));
            }
            let location = self.span(*table).start;
            let (schema, name, inh, alias) = match self.get(table_ref.base) {
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
                _ => return Err(unsupported("FROM item", Some(location..location))),
            };
            let (alias_name, column_aliases) = match alias {
                None => (None, Vec::new()),
                Some(a) => {
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
                    (Some(name), cols)
                }
            };
            let rt_index = self.relations.add_range_table_entry(
                scope,
                schema.as_deref(),
                &name,
                alias_name.as_deref(),
                &column_aliases,
                inh,
                location,
            )?;
            self.relations.check_namespace_conflicts(scope, rt_index)?;
            scope.namespace.push(NamespaceItem {
                rt_index,
                rel_visible: true,
                cols_visible: true,
            });
        }
        Ok(())
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
                let origin = self.analysis.vars.get(e.expr).and_then(|var| {
                    let rte = &scope.rtable[var.varno - 1];
                    (var.levelsup == 0 && matches!(typed.form, Form::Var))
                        .then_some((rte.relation, var.varattno))
                });
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
        for (i, name) in rte.colnames.iter().enumerate() {
            if name.is_empty() {
                continue;
            }
            let var = self.relations.make_var(item.rt_index, rte, i as i16 + 1, 0);
            out.push(TargetEntry {
                name: name.clone(),
                ty: var.ty,
                typmod: var.typmod,
                origin: Some((rte.relation, var.varattno)),
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
        // The CASE body shapes are not walked in this slice; the name falls
        // back to "case".
        let _ = body;
        None
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
        match self.get(q) {
            FlatIntervalQualifier::Year => (YEAR, None),
            FlatIntervalQualifier::Month => (MONTH, None),
            FlatIntervalQualifier::Day => (DAY, None),
            FlatIntervalQualifier::Hour => (HOUR, None),
            FlatIntervalQualifier::Minute => (MINUTE, None),
            FlatIntervalQualifier::Second(_) => (SECOND, None),
            FlatIntervalQualifier::YearToMonth => (YEAR | MONTH, None),
            FlatIntervalQualifier::DayToHour => (DAY | HOUR, None),
            FlatIntervalQualifier::DayToMinute => (DAY | HOUR | MINUTE, None),
            FlatIntervalQualifier::DayToSecond(_) => (DAY | HOUR | MINUTE | SECOND, None),
            FlatIntervalQualifier::HourToMinute => (HOUR | MINUTE, None),
            FlatIntervalQualifier::HourToSecond(_) => (HOUR | MINUTE | SECOND, None),
            FlatIntervalQualifier::MinuteToSecond(_) => (MINUTE | SECOND, None),
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
                let arg = self.expr(scope, inner)?;
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
                let arg = self.expr(scope, value)?;
                self.type_cast(handle, value, &arg, target, location)
            }
            FlatExpr::CastFunc(f) => self.typed_literal(handle, *f, location),
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
                if esc.is_some() {
                    return Err(unsupported("LIKE ... ESCAPE", Some(location..location)));
                }
                let (l, r) = (*l, *r);
                let op = match node {
                    FlatExpr::Like(..) => "~~",
                    FlatExpr::Ilike(..) => "~~*",
                    FlatExpr::NotLike(..) => "!~~",
                    _ => "!~~*",
                };
                let lt = self.expr(scope, l)?;
                let rt = self.expr(scope, r)?;
                let op_location = self.next_token_start(self.span(l).end);
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
        if suffix.within_group.is_some() || suffix.filter.is_some() || suffix.window.is_some() {
            return Err(unsupported(
                "aggregate or window decoration",
                Some(location..location),
            ));
        }
        let mut arg_handles: Vec<Handle<FlatExpr>> = Vec::new();
        if let Some(body) = suffix.body {
            match self.get(body) {
                FlatFunctionCallBody::Args(args) => {
                    let args = self.get(*args);
                    if args.trailing_variadic.is_some() || args.order_by.is_some() {
                        return Err(unsupported(
                            "VARIADIC or ORDER BY in call",
                            Some(location..location),
                        ));
                    }
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
                _ => return Err(unsupported("aggregate call form", Some(location..location))),
            }
        }
        let mut args = Vec::with_capacity(arg_handles.len());
        for &a in &arg_handles {
            args.push(self.expr(scope, a)?);
        }
        match self.resolver.parse_func(&name, &args, location)? {
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
                for (i, (&a, arg)) in arg_handles.iter().zip(&args).enumerate() {
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
