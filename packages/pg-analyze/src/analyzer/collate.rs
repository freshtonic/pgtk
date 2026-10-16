//! `parse_collate.c`: the collation of each expression, assigned after a
//! query level is transformed, and the collation mismatches that are
//! Diagnostics at that point.

use super::*;
use crate::analysis::{CollContext, Strength};
use crate::value::CoercionPath;
use pg_catalog::Collation;

/// What a sublink contributes to its parent's collation state.
#[derive(Clone, Debug)]
pub(super) struct SublinkInfo {
    /// The subquery's span, whose expressions belong to the inner level.
    pub span: std::ops::Range<usize>,
    /// The collation of the subquery's first output column.
    pub first: Option<pg_catalog::Handle<Collation>>,
}

impl<'a, 'input, 'c> Walker<'a, 'input, 'c> {
    /// `get_typcollation`.
    pub(super) fn type_collation(&self, ty: TypeRef) -> Option<pg_catalog::Handle<Collation>> {
        self.catalog()[ty].collation
    }

    /// `DEFAULT_COLLATION_OID`.
    fn is_default_collation(&self, c: pg_catalog::Handle<Collation>) -> bool {
        self.catalog()[c].oid == pg_catalog::Oid(100)
    }

    /// `get_collation_name`.
    pub(super) fn collation_name(&self, c: pg_catalog::Handle<Collation>) -> String {
        self.catalog()[c].name.to_string()
    }

    pub(super) fn implicit_mismatch(&self, ctx: &CollContext) -> Diagnostic {
        let a = ctx
            .collation
            .map_or_else(String::new, |c| self.collation_name(c));
        let b = ctx
            .collation2
            .map_or_else(String::new, |c| self.collation_name(c));
        Diagnostic::new(
            "42P21",
            format!("collation mismatch between implicit collations \"{a}\" and \"{b}\""),
        )
        .hint("You can choose the collation by applying the COLLATE clause to one or both expressions.")
        .at(ctx.location2)
    }

    /// `assign_expr_collations`: the collation of a standalone expression;
    /// an implicit conflict is not an error here.
    pub(super) fn assign_expr_collations(
        &mut self,
        scope: &Scope,
        e: Handle<FlatExpr>,
    ) -> Result<Option<pg_catalog::Handle<Collation>>> {
        let mut ctx = CollContext::new();
        self.assign_collations_walker(scope, e, &mut ctx)?;
        Ok(ctx.resolved())
    }

    /// `select_common_collation` over expressions.
    pub(super) fn select_common_collation(
        &mut self,
        scope: &Scope,
        exprs: &[Handle<FlatExpr>],
        none_ok: bool,
    ) -> Result<Option<pg_catalog::Handle<Collation>>> {
        let mut ctx = CollContext::new();
        for &e in exprs {
            self.assign_collations_walker(scope, e, &mut ctx)?;
        }
        if ctx.strength == Strength::Conflict {
            if none_ok {
                return Ok(None);
            }
            return Err(self.implicit_mismatch(&ctx).into());
        }
        Ok(ctx.collation)
    }

    /// `select_common_collation` over collations already derived, each
    /// implicit: the columns of a set operation or of a merged join
    /// column.
    pub(super) fn merge_implicit_collations(
        &self,
        items: &[(Option<pg_catalog::Handle<Collation>>, usize)],
        none_ok: bool,
    ) -> Result<Option<pg_catalog::Handle<Collation>>> {
        let mut ctx = CollContext::new();
        for &(c, at) in items {
            let strength = if c.is_some() {
                Strength::Implicit
            } else {
                Strength::None
            };
            self.merge_collation_state(c, strength, at, None, NO_LOCATION, &mut ctx)?;
        }
        if ctx.strength == Strength::Conflict {
            if none_ok {
                return Ok(None);
            }
            return Err(self.implicit_mismatch(&ctx).into());
        }
        Ok(ctx.collation)
    }

    /// `merge_collation_state`.
    pub(super) fn merge_collation_state(
        &self,
        collation: Option<pg_catalog::Handle<Collation>>,
        strength: Strength,
        location: usize,
        collation2: Option<pg_catalog::Handle<Collation>>,
        location2: usize,
        ctx: &mut CollContext,
    ) -> Result<()> {
        if strength > ctx.strength {
            ctx.collation = collation;
            ctx.strength = strength;
            ctx.location = location;
            if strength == Strength::Conflict {
                ctx.collation2 = collation2;
                ctx.location2 = location2;
            }
        } else if strength == ctx.strength {
            match strength {
                Strength::None | Strength::Conflict => {}
                Strength::Implicit => {
                    if collation != ctx.collation {
                        if ctx.collation.is_some_and(|c| self.is_default_collation(c)) {
                            ctx.collation = collation;
                            ctx.strength = strength;
                            ctx.location = location;
                        } else if !collation.is_some_and(|c| self.is_default_collation(c)) {
                            ctx.strength = Strength::Conflict;
                            ctx.collation2 = collation;
                            ctx.location2 = location;
                        }
                    }
                }
                Strength::Explicit => {
                    if collation != ctx.collation {
                        let a = ctx
                            .collation
                            .map_or_else(String::new, |c| self.collation_name(c));
                        let b = collation.map_or_else(String::new, |c| self.collation_name(c));
                        return Err(Diagnostic::new(
                            "42P21",
                            format!(
                                "collation mismatch between explicit collations \"{a}\" and \"{b}\""
                            ),
                        )
                        .at(location)
                        .into());
                    }
                }
            }
        }
        Ok(())
    }

    /// The direct child expressions of `e` at this level: the ones of a
    /// sublink's subquery belong to the inner level.
    fn collation_children(&self, e: Handle<FlatExpr>) -> Vec<Handle<FlatExpr>> {
        let children = self.child_exprs(e);
        let Some(info) = self.sublink_info.get(&e) else {
            return children;
        };
        let inner = info.span.clone();
        children
            .into_iter()
            .filter(|h| {
                let s = self.span(*h);
                !(s.start >= inner.start && s.end <= inner.end)
            })
            .collect()
    }

    /// `assign_collations_walker` at a `TargetEntry`: the entry's own
    /// state, which must not be a conflict when the entry is a sort or
    /// group key.
    fn target_entry_collations(
        &mut self,
        scope: &Scope,
        e: Handle<FlatExpr>,
        sortgroup: bool,
        ctx: &mut CollContext,
    ) -> Result<CollContext> {
        let mut local = CollContext::new();
        self.assign_collations_walker(scope, e, &mut local)?;
        if local.strength == Strength::Conflict && sortgroup {
            return Err(self.implicit_mismatch(&local).into());
        }
        self.merge_collation_state(
            local.collation,
            local.strength,
            local.location,
            local.collation2,
            local.location2,
            ctx,
        )?;
        Ok(local)
    }

    /// `assign_collations_walker`: the collation state of `e`, merged
    /// into `ctx`, and `exprCollation(e)` recorded.
    pub(super) fn assign_collations_walker(
        &mut self,
        scope: &Scope,
        e: Handle<FlatExpr>,
        ctx: &mut CollContext,
    ) -> Result<()> {
        let mut local = CollContext::new();
        let span = self.span(e);
        let location = span.start;
        let mut result: Option<(Option<pg_catalog::Handle<Collation>>, Strength, usize)> = None;
        if let Some(&(c, at)) = self.collate_exprs.get(&e) {
            // CollateExpr.
            for child in self.collation_children(e) {
                self.assign_collations_walker(scope, child, &mut local)?;
            }
            result = Some((Some(c), Strength::Explicit, at));
        } else if let Some(var) = self.analysis.vars.get(e).copied() {
            // Var: subscripts, if any, are independent expressions.
            for child in self.collation_children(e) {
                self.assign_expr_collations(scope, child)?;
            }
            let c = self.var_collation(scope, &var)?;
            let strength = if c.is_some() {
                Strength::Implicit
            } else {
                Strength::None
            };
            result = Some((c, strength, location));
        } else if self.row_elements(e).is_some() {
            // RowExpr: each field on its own; the row has no collation.
            for child in self.collation_children(e) {
                self.assign_expr_collations(scope, child)?;
            }
            result = Some((None, Strength::None, NO_LOCATION));
        } else if let Some(info) = self.sublink_info.get(&e).cloned() {
            // SubLink: the test expression, then the subquery as its first
            // output column.
            for child in self.collation_children(e) {
                self.assign_collations_walker(scope, child, &mut local)?;
            }
            let strength = if info.first.is_some() {
                Strength::Implicit
            } else {
                Strength::None
            };
            self.merge_collation_state(
                info.first,
                strength,
                info.span.start,
                None,
                NO_LOCATION,
                &mut local,
            )?;
        } else if let Some(agg) = self.aggregate_call_parts(e) {
            self.aggregate_collations(scope, e, &agg, &mut local)?;
        } else {
            for child in self.collation_children(e) {
                self.assign_collations_walker(scope, child, &mut local)?;
            }
        }
        let (mut collation, mut strength, mut at) = match result {
            Some(r) => r,
            None => {
                // The default: a collatable result takes the merged state
                // of the children, else the type's own collation.
                let ty = self.analysis.expr_types.get(e).map(|t| t.ty);
                self.result_collation(ty, &local, location)
            }
        };
        // The coercions applied to the node are nodes of their own.
        if let Some(steps) = self.analysis.coercions.get(e).cloned() {
            for step in steps {
                let inner = CollContext {
                    collation,
                    strength,
                    location: at,
                    collation2: local.collation2,
                    location2: local.location2,
                };
                let (c, s, l) = if matches!(step.path, CoercionPath::Domain) {
                    // CoerceToDomain: the domain's own collation wins,
                    // unless it is the default one.
                    match self.type_collation(step.target) {
                        Some(tc) if !self.is_default_collation(tc) => {
                            (Some(tc), Strength::Implicit, location)
                        }
                        Some(_) => (inner.collation, inner.strength, inner.location),
                        None => (None, Strength::None, NO_LOCATION),
                    }
                } else {
                    self.result_collation(Some(step.target), &inner, location)
                };
                collation = c;
                strength = s;
                at = l;
            }
        }
        self.expr_collations.insert(
            e,
            if strength == Strength::Conflict {
                None
            } else {
                collation
            },
        );
        self.merge_collation_state(
            collation,
            strength,
            at,
            local.collation2,
            local.location2,
            ctx,
        )
    }

    /// The state of a node of type `ty` whose children merged into
    /// `local`.
    fn result_collation(
        &self,
        ty: Option<TypeRef>,
        local: &CollContext,
        location: usize,
    ) -> (Option<pg_catalog::Handle<Collation>>, Strength, usize) {
        match ty.and_then(|t| self.type_collation(t)) {
            Some(tc) => {
                if local.strength > Strength::None {
                    (local.collation, local.strength, local.location)
                } else {
                    (Some(tc), Strength::Implicit, location)
                }
            }
            None => (None, Strength::None, NO_LOCATION),
        }
    }

    /// `exprCollation` of a `Var`.
    pub(super) fn var_collation(
        &self,
        scope: &Scope,
        var: &Var,
    ) -> Result<Option<pg_catalog::Handle<Collation>>> {
        if var.varattno <= 0 {
            return Ok(None);
        }
        let level = self.scope_at(scope, var.levelsup);
        self.var_collation_at(level, var)
    }

    pub(super) fn var_collation_at(
        &self,
        level: &Scope,
        var: &Var,
    ) -> Result<Option<pg_catalog::Handle<Collation>>> {
        if var.varattno <= 0 {
            return Ok(None);
        }
        let rte = &level.rtable[var.varno - 1];
        let index = usize::try_from(var.varattno - 1).expect("attnum");
        Ok(match &rte.kind {
            RteKind::Cte { columns, .. } => columns.get(index).and_then(|c| c.collation),
            RteKind::Subquery { columns, .. } => columns.get(index).and_then(|c| c.collation),
            RteKind::Values { collations, .. } | RteKind::SetOperation { collations, .. } => {
                collations.get(index).copied().flatten()
            }
            RteKind::Function { columns, .. } => columns
                .get(index)
                .and_then(|(ty, _)| self.type_collation(*ty)),
            RteKind::Join {
                columns, inputs, ..
            } => {
                let merged = inputs.get(index).filter(|i| !i.is_empty());
                match merged {
                    Some(inputs) => {
                        let mut items = Vec::with_capacity(inputs.len());
                        for v in inputs {
                            items.push((self.var_collation_at(level, v)?, NO_LOCATION));
                        }
                        self.merge_implicit_collations(&items, true)?
                    }
                    None => match columns.get(index) {
                        Some(v) => self.var_collation_at(level, v)?,
                        None => None,
                    },
                }
            }
            _ => match rte.relation() {
                Some(rel) => self.catalog()[rel]
                    .column_at(var.varattno)
                    .and_then(|c| c.collation),
                None => None,
            },
        })
    }

    /// `assign_query_collations` for one level: the target entries, with
    /// the sort and group keys checked, then the other clauses.
    pub(super) fn assign_query_collations(
        &mut self,
        scope: &Scope,
        target_list: &mut [TargetEntry],
        target_exprs: &[Option<Handle<FlatExpr>>],
        sortgroup: &[usize],
    ) -> Result<()> {
        for (i, entry) in target_list.iter_mut().enumerate() {
            let Some(e) = target_exprs.get(i).copied().flatten() else {
                // An expanded column: a Var with its column's collation.
                entry.collation_state = CollContext::implicit(entry.collation, entry.location);
                continue;
            };
            let mut ctx = CollContext::new();
            let mut local =
                self.target_entry_collations(scope, e, sortgroup.contains(&i), &mut ctx)?;
            // resolveTargetListUnknowns: an unknown literal read as text has
            // the default collation.
            if local.strength == Strength::None
                && self
                    .analysis
                    .expr_types
                    .get(e)
                    .is_some_and(|t| self.types().is_unknown(t.ty))
                && let Some(tc) = self.type_collation(entry.ty)
            {
                local = CollContext {
                    collation: Some(tc),
                    strength: Strength::Implicit,
                    location: self.span(e).start,
                    ..local
                };
            }
            entry.collation = local.resolved();
            entry.collation_state = local;
        }
        let clause_exprs = std::mem::take(&mut self.clause_exprs);
        for e in clause_exprs {
            if target_exprs.contains(&Some(e)) {
                continue;
            }
            self.assign_expr_collations(scope, e)?;
        }
        Ok(())
    }
}

/// The parts of an aggregate call the collation rules tell apart.
pub(super) struct AggregateParts {
    pub kind: pg_catalog::AggregateKind,
    /// The arguments before WITHIN GROUP.
    pub args: Vec<Handle<FlatExpr>>,
    /// The ORDER BY expressions, or the WITHIN GROUP ones.
    pub order: Vec<Handle<FlatExpr>>,
    pub within_group: bool,
    pub filter: Option<Handle<FlatExpr>>,
    pub variadic: bool,
}

impl<'a, 'input, 'c> Walker<'a, 'input, 'c> {
    /// The parts of `e` when it is an aggregate call.
    fn aggregate_call_parts(&self, e: Handle<FlatExpr>) -> Option<AggregateParts> {
        let resolution = self.analysis.functions.get(e)?;
        if resolution.window {
            return None;
        }
        let proc = &self.catalog()[resolution.function];
        let detail = proc.aggregate.as_ref()?;
        let FlatExpr::Func(fc) = self.get(e) else {
            return None;
        };
        let FlatFunctionCallTail::Call(suffix) = self.get(self.get(*fc).tail) else {
            return None;
        };
        let suffix = self.get(*suffix);
        let mut args = Vec::new();
        let mut order = Vec::new();
        if let Some(body) = suffix.body {
            let (items, order_by, _) = self.call_body_items(body);
            for item in items {
                args.push(match self.get(item) {
                    FlatFuncArg::Plain(x) => *x,
                    FlatFuncArg::Named(n) => self.get(*n).value,
                });
            }
            if let Some(ob) = order_by {
                for item in self.list1(self.get(ob).items) {
                    order.push(self.get(*item).expr);
                }
            }
        }
        if let Some(wg) = suffix.within_group {
            let clause = self.get(self.get(wg).order_by);
            for item in self.list1(clause.items) {
                order.push(self.get(*item).expr);
            }
        }
        let filter = suffix.filter.and_then(|f| {
            let clause = self.get(self.get(f).body);
            match self.get(clause.condition) {
                FlatWhereCondition::Expr(x) => Some(*x),
                _ => None,
            }
        });
        Some(AggregateParts {
            kind: detail.kind,
            args,
            order,
            within_group: suffix.within_group.is_some(),
            filter,
            variadic: proc.variadic.is_some(),
        })
    }

    /// `assign_aggregate_collations`, `assign_ordered_set_collations` and
    /// `assign_hypothetical_collations`, then the FILTER on its own.
    fn aggregate_collations(
        &mut self,
        scope: &Scope,
        e: Handle<FlatExpr>,
        agg: &AggregateParts,
        local: &mut CollContext,
    ) -> Result<()> {
        let _ = e;
        if !agg.within_group {
            // Each argument is a target entry; an ORDER BY expression that
            // is not an argument is a junk entry with a sort reference.
            for &a in &agg.args {
                let sortgroup = agg.order.iter().any(|&o| self.expr_equal(o, a));
                self.target_entry_collations(scope, a, sortgroup, local)?;
            }
            for &o in &agg.order {
                if agg.args.iter().any(|&a| self.expr_equal(o, a)) {
                    continue;
                }
                let mut ignored = CollContext::new();
                self.target_entry_collations(scope, o, true, &mut ignored)?;
            }
        } else if agg.kind == pg_catalog::AggregateKind::HypotheticalSet {
            let extra = agg.args.len().saturating_sub(agg.order.len());
            for &a in &agg.args[..extra] {
                self.assign_collations_walker(scope, a, local)?;
            }
            for (&h, &s) in agg.args[extra..].iter().zip(&agg.order) {
                let mut pair = CollContext::new();
                self.assign_collations_walker(scope, h, &mut pair)?;
                self.assign_collations_walker(scope, s, &mut pair)?;
                if pair.strength == Strength::Conflict {
                    return Err(self.implicit_mismatch(&pair).into());
                }
            }
        } else {
            for &a in &agg.args {
                self.assign_collations_walker(scope, a, local)?;
            }
            let merge_sort = agg.order.len() == 1 && !agg.variadic;
            for &s in &agg.order {
                if merge_sort {
                    self.target_entry_collations(scope, s, true, local)?;
                } else {
                    let mut ignored = CollContext::new();
                    self.target_entry_collations(scope, s, true, &mut ignored)?;
                }
            }
        }
        if let Some(f) = agg.filter {
            self.assign_expr_collations(scope, f)?;
        }
        Ok(())
    }
}
