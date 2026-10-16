//! `parse_func.c` and the candidate lists of `namespace.c`: which function a
//! call names, and with what argument types.

use pgtk_catalog::{
    ArgMode, Function, FunctionKind, Handle, PseudoType, QualifiedName, SearchPath, TypeCategory,
};

use crate::coerce::Coercer;
use crate::diagnostic::{Diagnostic, Result, unsupported};
use crate::types::TypeRef;
use crate::value::{CoercionContext, CoercionPath, Typed};

/// A possibly schema-qualified name as written, already folded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObjectName {
    pub schema: Option<String>,
    pub name: String,
}

impl ObjectName {
    /// `NameListToString`.
    pub fn display(&self) -> String {
        match &self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.clone(),
        }
    }
}

/// One entry of a `FuncCandidateList`, for a function or an operator.
#[derive(Clone, Debug)]
pub(crate) struct Candidate<I: Copy> {
    /// The function or operator, or `None` once two candidates in one
    /// schema tied and PostgreSQL cleared the OID to mark the ambiguity.
    pub id: Option<I>,
    pub pathpos: usize,
    /// Declared argument types after variadic and default expansion.
    pub args: Vec<TypeRef>,
    pub nvargs: usize,
    pub ndargs: usize,
    /// For a call with named arguments: the parameter each call position
    /// supplies, defaults last.
    pub argnumbers: Option<Vec<usize>>,
}

/// `FuncDetailCode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FuncDetail {
    NotFound,
    Multiple,
    Normal,
    Procedure,
    Aggregate,
    WindowFunc,
    Coercion,
}

/// What `func_get_detail` found.
pub(crate) struct FuncFound {
    pub detail: FuncDetail,
    pub func: Option<Handle<Function>>,
    pub rettype: Option<TypeRef>,
    pub retset: bool,
    pub nvargs: usize,
    pub vatype: Option<TypeRef>,
    /// Declared argument types, with defaults appended.
    pub declared: Vec<TypeRef>,
    /// The types the default expressions of the unspecified parameters
    /// have, in the order `declared` appends them: what `exprType` gives
    /// `ParseFuncOrColumn` for `argdefaults`.
    pub defaults: Vec<TypeRef>,
}

/// Function resolution over one Catalog and Search Path.
pub(crate) struct FuncResolver<'c> {
    pub coercer: Coercer<'c>,
    pub search_path: &'c SearchPath,
}

impl<'c> FuncResolver<'c> {
    pub fn catalog(&self) -> &'c pgtk_catalog::Catalog {
        self.coercer.types.catalog
    }

    /// The position of a schema in the Search Path, for the hiding rule.
    fn pathpos(&self, schema: Handle<pgtk_catalog::Schema>) -> Option<usize> {
        self.search_path.schemas().iter().position(|s| *s == schema)
    }
    /// `LookupExplicitNamespace` with `missing_ok = false`. Returns the
    /// schema and its real name, which for `pg_temp` is `pg_temp_N`.
    pub fn lookup_schema(
        &self,
        name: &str,
        location: usize,
    ) -> Result<(Handle<pgtk_catalog::Schema>, String)> {
        let catalog = self.catalog();
        let real = crate::relation::resolve_schema_name(catalog, name);
        let found = real.and_then(|real| catalog.schema_named(real).map(|h| (h, real.to_owned())));
        found.ok_or_else(|| {
            Diagnostic::new("3F000", format!("schema \"{name}\" does not exist"))
                .at(location)
                .into()
        })
    }

    /// The functions a name can mean, in the order `FuncnameGetCandidates`
    /// visits them: for an unqualified name, every schema on the path.
    pub(crate) fn functions_named(
        &self,
        name: &ObjectName,
        location: usize,
    ) -> Result<Vec<(Handle<Function>, usize)>> {
        let catalog = self.catalog();
        match &name.schema {
            Some(schema) => {
                let (_, schema) = self.lookup_schema(schema, location)?;
                Ok(catalog
                    .functions_named(&QualifiedName::new(schema.as_str(), name.name.as_str()))
                    .iter()
                    .map(|&f| (f, 0))
                    .collect())
            }
            None => Ok(catalog
                .functions_visible(self.search_path, &name.name)
                .into_iter()
                .filter_map(|f| self.pathpos(catalog[f].schema).map(|p| (f, p)))
                .collect()),
        }
    }

    /// `MatchNamedCall`: the parameter each call position supplies, when
    /// the named arguments match the function's parameter names.
    pub(crate) fn match_named_call(
        proc: &Function,
        nargs: usize,
        argnames: &[String],
        pronargs: usize,
        include_out: bool,
    ) -> Option<Vec<usize>> {
        let numposargs = nargs - argnames.len();
        let params: Vec<&pgtk_catalog::Argument> = if include_out {
            proc.arguments
                .iter()
                .filter(|a| a.mode != ArgMode::Table)
                .collect()
        } else {
            proc.input_arguments().collect()
        };
        let mut arggiven = vec![false; pronargs];
        let mut argnumbers = Vec::with_capacity(pronargs);
        for (ap, given) in arggiven.iter_mut().enumerate().take(numposargs) {
            argnumbers.push(ap);
            *given = true;
        }
        for argname in argnames {
            let pp = params
                .iter()
                .position(|p| p.name.as_ref().is_some_and(|n| n.to_string() == *argname))?;
            if arggiven[pp] {
                return None;
            }
            arggiven[pp] = true;
            argnumbers.push(pp);
        }
        if nargs < pronargs {
            let first_arg_with_default = pronargs - usize::from(proc.default_count);
            for (pp, given) in arggiven.iter().enumerate().skip(numposargs) {
                if *given {
                    continue;
                }
                if pp < first_arg_with_default {
                    return None;
                }
                argnumbers.push(pp);
            }
        }
        Some(argnumbers)
    }

    /// `FuncnameGetCandidates`. `argnames` names the last arguments of the
    /// call, in order.
    pub fn funcname_get_candidates(
        &self,
        name: &ObjectName,
        nargs: usize,
        argnames: &[String],
        expand_variadic: bool,
        expand_defaults: bool,
        include_out: bool,
        location: usize,
    ) -> Result<Vec<Candidate<Handle<Function>>>> {
        let catalog = self.catalog();
        let explicit_schema = name.schema.is_some();
        let mut result: Vec<Candidate<Handle<Function>>> = Vec::new();
        let mut any_special = false;
        for (func, pathpos) in self.functions_named(name, location)? {
            let proc = &catalog[func];
            // A procedure's OUT arguments are part of its signature for CALL.
            let proargtypes: Vec<TypeRef> = if include_out && proc.kind == FunctionKind::Procedure {
                proc.arguments
                    .iter()
                    .filter(|a| a.mode != ArgMode::Table)
                    .map(|a| a.ty)
                    .collect()
            } else {
                proc.input_arguments().map(|a| a.ty).collect()
            };
            let pronargs = proargtypes.len();
            let mut argnumbers: Option<Vec<usize>> = None;
            let (va_elem_type, variadic, use_defaults) = if !argnames.is_empty() {
                // Named arguments: no variadic expansion, defaults allowed,
                // and the names must match.
                if proc.variadic.is_some() && expand_variadic {
                    continue;
                }
                let use_defaults = if pronargs > nargs && expand_defaults {
                    if nargs + usize::from(proc.default_count) < pronargs {
                        continue;
                    }
                    true
                } else {
                    false
                };
                if pronargs != nargs && !use_defaults {
                    continue;
                }
                let Some(numbers) = Self::match_named_call(
                    proc,
                    nargs,
                    argnames,
                    pronargs,
                    include_out && proc.kind == FunctionKind::Procedure,
                ) else {
                    continue;
                };
                argnumbers = Some(numbers);
                any_special = true;
                (None, false, use_defaults)
            } else {
                let (va_elem_type, variadic) = if pronargs <= nargs && expand_variadic {
                    let v = proc.variadic;
                    any_special |= v.is_some();
                    (v, v.is_some())
                } else {
                    (None, false)
                };
                let use_defaults = if pronargs > nargs && expand_defaults {
                    if nargs + usize::from(proc.default_count) < pronargs {
                        continue;
                    }
                    any_special = true;
                    true
                } else {
                    false
                };
                if pronargs != nargs && !variadic && !use_defaults {
                    continue;
                }
                (va_elem_type, variadic, use_defaults)
            };
            let effective_nargs = pronargs.max(nargs);
            let mut args: Vec<TypeRef> = match &argnumbers {
                Some(numbers) => numbers.iter().map(|&j| proargtypes[j]).collect(),
                None => proargtypes.clone(),
            };
            args.resize(
                effective_nargs,
                proargtypes.last().copied().unwrap_or(
                    args.first()
                        .copied()
                        .unwrap_or_else(|| catalog.builtin(pgtk_catalog::Builtin::Any)),
                ),
            );
            let nvargs = if variadic {
                let va = va_elem_type.expect("variadic implies an element type");
                for a in args.iter_mut().skip(pronargs - 1) {
                    *a = va;
                }
                effective_nargs - pronargs + 1
            } else {
                0
            };
            let ndargs = if use_defaults { pronargs - nargs } else { 0 };
            let candidate = Candidate {
                id: Some(func),
                pathpos,
                args,
                nvargs,
                ndargs,
                argnumbers,
            };
            if !result.is_empty() && (any_special || !explicit_schema) {
                let cmp_nargs = candidate.args.len() - candidate.ndargs;
                let prev = result.iter().position(|prev| {
                    cmp_nargs == prev.args.len() - prev.ndargs
                        && candidate.args[..cmp_nargs] == prev.args[..cmp_nargs]
                });
                if let Some(index) = prev {
                    let prev = &result[index];
                    let preference: i64 = if candidate.pathpos != prev.pathpos {
                        candidate.pathpos as i64 - prev.pathpos as i64
                    } else if variadic && prev.nvargs == 0 {
                        1
                    } else if !variadic && prev.nvargs > 0 {
                        -1
                    } else {
                        0
                    };
                    if preference > 0 {
                        continue;
                    } else if preference < 0 {
                        result.remove(index);
                    } else {
                        result[index].id = None;
                        continue;
                    }
                }
            }
            result.push(candidate);
        }
        Ok(result)
    }

    /// `func_match_argtypes`.
    pub fn func_match_argtypes<I: Copy>(
        &self,
        input: &[TypeRef],
        candidates: Vec<Candidate<I>>,
    ) -> Vec<Candidate<I>> {
        candidates
            .into_iter()
            .filter(|c| {
                self.coercer.can_coerce_type(
                    input,
                    &c.args[..input.len()],
                    CoercionContext::Implicit,
                )
            })
            .collect()
    }

    /// `func_select_candidate`. Returns the single best candidate, or `None`.
    pub fn func_select_candidate<I: Copy>(
        &self,
        input: &[TypeRef],
        candidates: Vec<Candidate<I>>,
    ) -> Option<Candidate<I>> {
        let types = &self.coercer.types;
        let nargs = input.len();
        let mut nunknowns = 0;
        let input_base: Vec<Option<TypeRef>> = input
            .iter()
            .map(|&t| {
                if types.is_unknown(t) {
                    nunknowns += 1;
                    None
                } else {
                    Some(types.base_type(t))
                }
            })
            .collect();

        // Exact matches on known args.
        let mut best: Vec<Candidate<I>> = Vec::new();
        let mut nbest = 0;
        for c in candidates {
            let nmatch = (0..nargs)
                .filter(|&i| input_base[i].is_some_and(|t| c.args[i] == t))
                .count();
            if nmatch > nbest || best.is_empty() {
                nbest = nmatch;
                best = vec![c];
            } else if nmatch == nbest {
                best.push(c);
            }
        }
        if best.len() == 1 {
            return best.pop();
        }

        // Exact or preferred-type matches on known args.
        let slot_category: Vec<Option<TypeCategory>> = input_base
            .iter()
            .map(|t| t.map(|t| types.category(t)))
            .collect();
        let candidates = best;
        let mut best: Vec<Candidate<I>> = Vec::new();
        let mut nbest = 0;
        for c in candidates {
            let nmatch = (0..nargs)
                .filter(|&i| match (input_base[i], slot_category[i]) {
                    (Some(t), Some(cat)) => c.args[i] == t || types.is_preferred(cat, c.args[i]),
                    _ => false,
                })
                .count();
            if nmatch > nbest || best.is_empty() {
                nbest = nmatch;
                best = vec![c];
            } else if nmatch == nbest {
                best.push(c);
            }
        }
        if best.len() == 1 {
            return best.pop();
        }
        if nunknowns == 0 {
            return None;
        }

        // Resolve unknowns by the category the candidates agree on.
        let mut candidates = best;
        let mut slot_cat: Vec<Option<TypeCategory>> = vec![None; nargs];
        let mut slot_has_preferred = vec![false; nargs];
        let mut resolved_unknowns = false;
        for i in 0..nargs {
            if input_base[i].is_some() {
                continue;
            }
            resolved_unknowns = true;
            let mut have_conflict = false;
            for c in &candidates {
                let t = c.args[i];
                let cat = types.category(t);
                let pref = types.catalog[t].is_preferred;
                match slot_cat[i] {
                    None => {
                        slot_cat[i] = Some(cat);
                        slot_has_preferred[i] = pref;
                    }
                    Some(sc) if sc == cat => {
                        slot_has_preferred[i] |= pref;
                    }
                    Some(_) => {
                        if cat == TypeCategory::String {
                            slot_cat[i] = Some(cat);
                            slot_has_preferred[i] = pref;
                        } else {
                            have_conflict = true;
                        }
                    }
                }
            }
            if have_conflict && slot_cat[i] != Some(TypeCategory::String) {
                resolved_unknowns = false;
                break;
            }
        }
        if resolved_unknowns {
            let kept: Vec<Candidate<I>> = candidates
                .iter()
                .filter(|c| {
                    (0..nargs).all(|i| {
                        if input_base[i].is_some() {
                            return true;
                        }
                        let t = c.args[i];
                        types.category(t) == slot_cat[i].expect("set above")
                            && !(slot_has_preferred[i] && !types.catalog[t].is_preferred)
                    })
                })
                .cloned()
                .collect();
            if !kept.is_empty() {
                candidates = kept;
            }
            if candidates.len() == 1 {
                return candidates.pop();
            }
        }

        // All known args share one type: try it for the unknowns too.
        if nunknowns < nargs {
            let mut known: Option<TypeRef> = None;
            let mut consistent = true;
            for t in input_base.iter().flatten() {
                match known {
                    None => known = Some(*t),
                    Some(k) if k != *t => {
                        consistent = false;
                        break;
                    }
                    _ => {}
                }
            }
            if consistent && let Some(k) = known {
                let all_known = vec![k; nargs];
                let mut found: Option<Candidate<I>> = None;
                let mut n = 0;
                for c in &candidates {
                    if self.coercer.can_coerce_type(
                        &all_known,
                        &c.args[..nargs],
                        CoercionContext::Implicit,
                    ) {
                        n += 1;
                        if n > 1 {
                            break;
                        }
                        found = Some(c.clone());
                    }
                }
                if n == 1 {
                    return found;
                }
            }
        }
        None
    }

    /// `FuncNameAsType`: the type a function name could be a cast to.
    fn func_name_as_type(&self, name: &ObjectName) -> Option<TypeRef> {
        let catalog = self.catalog();
        // LookupTypeNameExtended with temp_ok = false: a type in the
        // temporary schema is found only by its qualified name.
        let t = match &name.schema {
            Some(schema) => {
                let schema = crate::relation::resolve_schema_name(catalog, schema)?;
                catalog.type_named(&QualifiedName::new(schema, name.name.as_str()))?
            }
            None => {
                let t = catalog.type_visible(self.search_path, &name.name)?;
                if Some(catalog[t].schema) == catalog.temporary_schema() {
                    return None;
                }
                t
            }
        };
        if self.coercer.types.is_complex(t) {
            return None;
        }
        Some(t)
    }

    /// `func_get_detail` without named arguments.
    pub fn func_get_detail(
        &self,
        name: &ObjectName,
        fargs: &[Typed],
        argnames: &[String],
        expand_variadic: bool,
        expand_defaults: bool,
        include_out: bool,
        location: usize,
    ) -> Result<FuncFound> {
        let catalog = self.catalog();
        let types = &self.coercer.types;
        let argtypes: Vec<TypeRef> = fargs.iter().map(|a| a.ty).collect();
        let nargs = argtypes.len();
        let raw = self.funcname_get_candidates(
            name,
            nargs,
            argnames,
            expand_variadic,
            expand_defaults,
            include_out,
            location,
        )?;
        let mut best = raw
            .iter()
            .find(|c| nargs == 0 || c.args[..nargs] == argtypes[..])
            .cloned();
        if best.is_none() {
            if nargs == 1
                && let Some(target) = self.func_name_as_type(name)
            {
                let source = argtypes[0];
                let iscoercion = if types.is_unknown(source) && fargs[0].is_const() {
                    true
                } else {
                    match self
                        .coercer
                        .find_coercion_pathway(target, source, CoercionContext::Explicit)
                        .map(|p| p.path)
                    {
                        Some(CoercionPath::RelabelType) => true,
                        Some(CoercionPath::CoerceViaIO) => {
                            !((types.is_record(source) || types.is_complex(source))
                                && types.category(target) == TypeCategory::String)
                        }
                        _ => false,
                    }
                };
                if iscoercion {
                    return Ok(FuncFound {
                        detail: FuncDetail::Coercion,
                        func: None,
                        rettype: Some(target),
                        retset: false,
                        nvargs: 0,
                        vatype: None,
                        declared: vec![target],
                        defaults: Vec::new(),
                    });
                }
            }
            if !raw.is_empty() {
                let matched = self.func_match_argtypes(&argtypes, raw);
                match matched.len() {
                    0 => {}
                    1 => best = matched.into_iter().next(),
                    _ => match self.func_select_candidate(&argtypes, matched) {
                        Some(c) => best = Some(c),
                        None => {
                            return Ok(FuncFound {
                                detail: FuncDetail::Multiple,
                                func: None,
                                rettype: None,
                                retset: false,
                                nvargs: 0,
                                vatype: None,
                                declared: Vec::new(),
                                defaults: Vec::new(),
                            });
                        }
                    },
                }
            }
        }
        let Some(best) = best else {
            return Ok(FuncFound {
                detail: FuncDetail::NotFound,
                func: None,
                rettype: None,
                retset: false,
                nvargs: 0,
                vatype: None,
                declared: Vec::new(),
                defaults: Vec::new(),
            });
        };
        let Some(func) = best.id else {
            return Ok(FuncFound {
                detail: FuncDetail::Multiple,
                func: None,
                rettype: None,
                retset: false,
                nvargs: 0,
                vatype: None,
                declared: Vec::new(),
                defaults: Vec::new(),
            });
        };
        // A VARIADIC argument written with names must stay the last one.
        if !argnames.is_empty()
            && !expand_variadic
            && nargs > 0
            && best
                .argnumbers
                .as_ref()
                .is_some_and(|n| n[nargs - 1] != nargs - 1)
        {
            return Ok(FuncFound {
                detail: FuncDetail::NotFound,
                func: None,
                rettype: None,
                retset: false,
                nvargs: 0,
                vatype: None,
                declared: Vec::new(),
                defaults: Vec::new(),
            });
        }
        let proc = &catalog[func];
        let detail = match proc.kind {
            FunctionKind::Aggregate => FuncDetail::Aggregate,
            FunctionKind::Function => FuncDetail::Normal,
            FunctionKind::Procedure => FuncDetail::Procedure,
            FunctionKind::Window => FuncDetail::WindowFunc,
        };
        // The defaults' own types, for the parameters the call left out.
        let defaults: Vec<TypeRef> = (nargs..best.args.len())
            .map(|i| {
                let param = best.argnumbers.as_ref().map_or(i, |numbers| numbers[i]);
                proc.default_type_of(param)
                    .unwrap_or_else(|| self.default_actual_type(best.args[i]))
            })
            .collect();
        Ok(FuncFound {
            detail,
            func: Some(func),
            rettype: Some(proc.returns),
            retset: proc.returns_set,
            nvargs: best.nvargs,
            vatype: if best.nvargs > 0 { proc.variadic } else { None },
            declared: best.args,
            defaults,
        })
    }

    /// The type a defaulted argument contributes to polymorphism
    /// resolution when the Catalog does not know its default expression:
    /// a polymorphic parameter's default was stored unchanged, so it is
    /// taken as an unknown-typed expression there.
    fn default_actual_type(&self, declared: TypeRef) -> TypeRef {
        let types = &self.coercer.types;
        if types.is_polymorphic(declared) {
            types.builtin(pgtk_catalog::Builtin::Unknown)
        } else {
            declared
        }
    }

    /// `func_signature_string`: `argnames` names the last arguments.
    pub fn func_signature_string(
        &self,
        name: &ObjectName,
        argtypes: &[TypeRef],
        argnames: &[String],
    ) -> String {
        let numposargs = argtypes.len().saturating_sub(argnames.len());
        let args = argtypes
            .iter()
            .enumerate()
            .map(|(i, &t)| {
                let ty = self.coercer.types.format(t);
                match argnames.get(i.wrapping_sub(numposargs)) {
                    Some(n) if i >= numposargs => format!("{n} => {ty}"),
                    _ => ty,
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({args})", name.display())
    }

    /// `ParseFuncOrColumn` for a plain function call: no aggregate
    /// decoration, no OVER, no named arguments. Returns the function, its
    /// result type facts, and the declared types each argument is coerced
    /// to (already resolved for polymorphism).
    pub fn parse_func(
        &self,
        name: &ObjectName,
        fargs: &[Typed],
        argnames: &[String],
        location: usize,
        projection: Option<Projection>,
        deco: Decoration,
    ) -> Result<ResolvedFunc> {
        let types = &self.coercer.types;
        if fargs.len() > 100 {
            return Err(Diagnostic::new(
                "54023",
                "cannot pass more than 100 arguments to a function".to_owned(),
            )
            .at(location)
            .into());
        }
        let argtypes: Vec<TypeRef> = fargs.iter().map(|a| a.ty).collect();
        let found = self.func_get_detail(
            name,
            fargs,
            argnames,
            !deco.variadic,
            true,
            deco.proc_call,
            location,
        )?;
        if deco.proc_call {
            match found.detail {
                FuncDetail::Procedure | FuncDetail::Multiple => {}
                FuncDetail::NotFound => {
                    return Err(Diagnostic::new(
                        "42883",
                        format!(
                            "procedure {} does not exist",
                            self.func_signature_string(name, &argtypes, argnames)
                        ),
                    )
                    .hint("No procedure matches the given name and argument types. You might need to add explicit type casts.")
                    .at(location)
                    .into());
                }
                _ => {
                    return Err(Diagnostic::new(
                        "42809",
                        format!(
                            "{} is not a procedure",
                            self.func_signature_string(name, &argtypes, argnames)
                        ),
                    )
                    .hint("To call a function, use SELECT.")
                    .at(location)
                    .into());
                }
            }
        }
        let wrong = |what: &str| -> crate::diagnostic::AnalyzeError {
            Diagnostic::new("42809", what.to_owned())
                .at(location)
                .into()
        };
        if matches!(
            found.detail,
            FuncDetail::Normal | FuncDetail::Procedure | FuncDetail::Coercion
        ) {
            let n = name.display();
            if deco.star {
                return Err(wrong(&format!(
                    "{n}(*) specified, but {n} is not an aggregate function"
                )));
            }
            if deco.distinct {
                return Err(wrong(&format!(
                    "DISTINCT specified, but {n} is not an aggregate function"
                )));
            }
            if deco.within_group {
                return Err(wrong(&format!(
                    "WITHIN GROUP specified, but {n} is not an aggregate function"
                )));
            }
            if deco.order_by {
                return Err(wrong(&format!(
                    "ORDER BY specified, but {n} is not an aggregate function"
                )));
            }
            if deco.filter {
                return Err(wrong(&format!(
                    "FILTER specified, but {n} is not an aggregate function"
                )));
            }
            if deco.over {
                return Err(wrong(&format!(
                    "OVER specified, but {n} is not a window function nor an aggregate function"
                )));
            }
        }
        if found.detail == FuncDetail::Aggregate {
            let func = found.func.expect("aggregate");
            let proc = &self.catalog()[func];
            let detail = proc.aggregate.as_ref().expect("aggregate detail");
            let n = name.display();
            let ordered_set = !matches!(detail.kind, pgtk_catalog::AggregateKind::Normal);
            if ordered_set {
                if !deco.within_group {
                    return Err(wrong(&format!(
                        "WITHIN GROUP is required for ordered-set aggregate {n}"
                    )));
                }
                if deco.over {
                    return Err(Diagnostic::new(
                        "0A000",
                        format!("OVER is not supported for ordered-set aggregate {n}"),
                    )
                    .at(location)
                    .into());
                }
                let num_direct = fargs.len() - deco.aggregated;
                let cat_direct = usize::from(detail.direct_args);
                if found.vatype.is_none() && num_direct != cat_direct {
                    let hint = if cat_direct == 1 {
                        format!(
                            "There is an ordered-set aggregate {n}, but it requires {cat_direct} direct argument, not {num_direct}."
                        )
                    } else {
                        format!(
                            "There is an ordered-set aggregate {n}, but it requires {cat_direct} direct arguments, not {num_direct}."
                        )
                    };
                    return Err(Diagnostic::new(
                        "42883",
                        format!(
                            "function {} does not exist",
                            self.func_signature_string(name, &argtypes, argnames)
                        ),
                    )
                    .hint(hint)
                    .at(location)
                    .into());
                }
                if detail.kind == pgtk_catalog::AggregateKind::HypotheticalSet {
                    // The hypothetical direct arguments pair with the
                    // ordering columns, one to one.
                    let aggregated = deco.aggregated;
                    if found.nvargs != 2 * aggregated {
                        return Err(Diagnostic::new(
                            "42883",
                            format!(
                                "function {} does not exist",
                                self.func_signature_string(name, &argtypes, argnames)
                            ),
                        )
                        .hint(format!(
                            "To use the hypothetical-set aggregate {n}, the number of hypothetical direct arguments (here {}) must match the number of ordering columns (here {aggregated}).",
                            found.nvargs.saturating_sub(aggregated)
                        ))
                        .at(location)
                        .into());
                    }
                } else if found.vatype.is_some() {
                    return Err(unsupported(
                        "variadic ordered-set aggregate",
                        Some(location..location),
                    ));
                }
            } else if deco.within_group {
                return Err(wrong(&format!(
                    "{n} is not an ordered-set aggregate, so it cannot have WITHIN GROUP"
                )));
            }
            if deco.over {
                // An aggregate over a window: its arguments coerced like
                // any aggregate's, checked by the caller as a window call.
                let mut actual: Vec<TypeRef> = argtypes.clone();
                let declared = found.declared.clone();
                actual.extend(found.defaults.iter().copied());
                let (rettype, declared) = self.coercer.enforce_generic_type_consistency(
                    &actual,
                    &declared,
                    found.rettype.expect("aggregate"),
                    false,
                )?;
                return Ok(ResolvedFunc::Window {
                    func,
                    rettype,
                    retset: found.retset,
                    declared,
                    is_aggregate: true,
                });
            }
            if fargs.is_empty() && !deco.star && !deco.within_group {
                return Err(wrong(&format!(
                    "{n}(*) must be used to call a parameterless aggregate function"
                )));
            }
            if found.retset {
                return Err(Diagnostic::new("42P13", "aggregates cannot return sets")
                    .at(location)
                    .into());
            }
            let mut actual: Vec<TypeRef> = argtypes.clone();
            let declared = found.declared.clone();
            actual.extend(found.defaults.iter().copied());
            let (rettype, declared) = self.coercer.enforce_generic_type_consistency(
                &actual,
                &declared,
                found.rettype.expect("aggregate"),
                false,
            )?;
            return Ok(ResolvedFunc::Aggregate {
                func,
                rettype,
                declared,
                kind: detail.kind,
                direct_args: fargs.len() - deco.aggregated,
            });
        }
        match found.detail {
            FuncDetail::Procedure if !deco.proc_call => {
                return Err(Diagnostic::new(
                    "42809",
                    format!(
                        "{} is a procedure",
                        self.func_signature_string(name, &argtypes, argnames)
                    ),
                )
                .hint("To call a procedure, use CALL.")
                .at(location)
                .into());
            }
            FuncDetail::Procedure => {}
            FuncDetail::Coercion => {
                return Ok(ResolvedFunc::Coercion {
                    target: found.rettype.expect("coercion has a target"),
                });
            }
            FuncDetail::Multiple => {
                return Err(Diagnostic::new(
                    "42725",
                    format!(
                        "function {} is not unique",
                        self.func_signature_string(name, &argtypes, argnames)
                    ),
                )
                .hint("Could not choose a best candidate function. You might need to add explicit type casts.")
                .at(location)
                .into());
            }
            FuncDetail::NotFound => {
                if let Some(projection) = projection {
                    return Ok(ResolvedFunc::Projection(projection));
                }
                return Err(Diagnostic::new(
                    "42883",
                    format!(
                        "function {} does not exist",
                        self.func_signature_string(name, &argtypes, argnames)
                    ),
                )
                .hint("No function matches the given name and argument types. You might need to add explicit type casts.")
                .at(location)
                .into());
            }
            FuncDetail::Aggregate => unreachable!("handled above"),
            FuncDetail::WindowFunc => {
                if !deco.over {
                    return Err(Diagnostic::new(
                        "42809",
                        format!("window function {} requires an OVER clause", name.display()),
                    )
                    .at(location)
                    .into());
                }
                if deco.within_group {
                    return Err(Diagnostic::new(
                        "42809",
                        format!(
                            "window function {} cannot have WITHIN GROUP",
                            name.display()
                        ),
                    )
                    .at(location)
                    .into());
                }
                let func = found.func.expect("window function");
                let mut actual: Vec<TypeRef> = argtypes.clone();
                let declared = found.declared.clone();
                actual.extend(found.defaults.iter().copied());
                let (rettype, declared) = self.coercer.enforce_generic_type_consistency(
                    &actual,
                    &declared,
                    found.rettype.expect("window function"),
                    false,
                )?;
                return Ok(ResolvedFunc::Window {
                    func,
                    rettype,
                    retset: found.retset,
                    declared,
                    is_aggregate: false,
                });
            }
            FuncDetail::Normal => {}
        }
        let func = found.func.expect("normal function");
        if deco.variadic
            && !fargs.is_empty()
            && self.catalog()[func]
                .variadic
                .is_some_and(|v| matches!(types.pseudo(v), Some(PseudoType::Any)))
        {
            let last = &fargs[fargs.len() - 1];
            if types.element_type(types.base_type(last.ty)).is_none() {
                return Err(
                    Diagnostic::new("42804", "VARIADIC argument must be an array")
                        .at(last.location)
                        .into(),
                );
            }
        }
        // Defaults: their types are the declared types.
        let mut actual = argtypes.clone();
        let declared = found.declared.clone();
        actual.extend(found.defaults.iter().copied());
        let (rettype, declared) = self.coercer.enforce_generic_type_consistency(
            &actual,
            &declared,
            found.rettype.expect("normal function"),
            false,
        )?;
        if found.nvargs > 0
            && let Some(va) = found.vatype
            && !matches!(types.pseudo(va), Some(PseudoType::Any))
        {
            // The array's element type is the first variadic argument's
            // actual type once polymorphism is resolved.
            let k = fargs.len() - found.nvargs;
            let elem = if types.is_polymorphic(declared[k]) {
                fargs[k].ty
            } else {
                declared[k]
            };
            if types.array_type(elem).is_none() {
                return Err(Diagnostic::new(
                    "42704",
                    format!(
                        "could not find array type for data type {}",
                        types.format(elem)
                    ),
                )
                .at(fargs[fargs.len() - found.nvargs].location)
                .into());
            }
        }
        Ok(ResolvedFunc::Function {
            func,
            rettype,
            retset: found.retset,
            declared,
        })
    }
}

/// The decoration of a call as written: `f(*)`, `DISTINCT`, an argument
/// `ORDER BY`, `WITHIN GROUP`, `FILTER` and `OVER`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Decoration {
    pub star: bool,
    pub distinct: bool,
    pub order_by: bool,
    pub within_group: bool,
    pub filter: bool,
    pub over: bool,
    /// Number of `WITHIN GROUP (ORDER BY ...)` expressions appended to the
    /// arguments, for ordered-set aggregates.
    pub aggregated: usize,
    /// The `VARIADIC` keyword was written on the last argument.
    pub variadic: bool,
    /// The call is a CALL statement: a procedure is wanted, and its OUT
    /// arguments count in the signature.
    pub proc_call: bool,
}

/// A field of a composite value selected with function-call syntax,
/// `ParseComplexProjection`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Projection {
    pub ty: TypeRef,
    pub typmod: crate::types::Typmod,
}

/// What a function call resolved to.
pub(crate) enum ResolvedFunc {
    /// A field of the composite argument, not a function at all.
    Projection(Projection),
    /// A window function call: a window function, or an aggregate with
    /// `OVER`. Arguments are coerced to their declared types.
    Window {
        func: Handle<Function>,
        rettype: TypeRef,
        retset: bool,
        declared: Vec<TypeRef>,
        /// The function is an aggregate used as a window function.
        is_aggregate: bool,
    },
    /// An aggregate: coerce each argument to its declared type; the direct
    /// arguments of an ordered-set aggregate come first.
    Aggregate {
        func: Handle<Function>,
        rettype: TypeRef,
        declared: Vec<TypeRef>,
        kind: pgtk_catalog::AggregateKind,
        direct_args: usize,
    },
    /// A real function: coerce each argument to its declared type.
    Function {
        func: Handle<Function>,
        rettype: TypeRef,
        retset: bool,
        declared: Vec<TypeRef>,
    },
    /// The type-coercion syntax `typename(arg)`.
    Coercion { target: TypeRef },
}
