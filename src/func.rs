//! `parse_func.c` and the candidate lists of `namespace.c`: which function a
//! call names, and with what argument types.

use pg_catalog::{
    Function, FunctionKind, Handle, PseudoType, QualifiedName, SearchPath, TypeCategory,
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
}

/// Function resolution over one Catalog and Search Path.
pub(crate) struct FuncResolver<'c> {
    pub coercer: Coercer<'c>,
    pub search_path: &'c SearchPath,
}

impl<'c> FuncResolver<'c> {
    pub fn catalog(&self) -> &'c pg_catalog::Catalog {
        self.coercer.types.catalog
    }

    /// The position of a schema in the Search Path, for the hiding rule.
    fn pathpos(&self, schema: Handle<pg_catalog::Schema>) -> Option<usize> {
        self.search_path.schemas().iter().position(|s| *s == schema)
    }

    /// `LookupExplicitNamespace` with `missing_ok = false`.
    pub fn lookup_schema(&self, name: &str, location: usize) -> Result<Handle<pg_catalog::Schema>> {
        self.catalog().schema_named(name).ok_or_else(|| {
            Diagnostic::new("3F000", format!("schema \"{name}\" does not exist"))
                .at(location)
                .into()
        })
    }

    /// The functions a name can mean, in the order `FuncnameGetCandidates`
    /// visits them: for an unqualified name, every schema on the path.
    fn functions_named(
        &self,
        name: &ObjectName,
        location: usize,
    ) -> Result<Vec<(Handle<Function>, usize)>> {
        let catalog = self.catalog();
        match &name.schema {
            Some(schema) => {
                self.lookup_schema(schema, location)?;
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

    /// `FuncnameGetCandidates` without named arguments.
    pub fn funcname_get_candidates(
        &self,
        name: &ObjectName,
        nargs: usize,
        expand_variadic: bool,
        expand_defaults: bool,
        location: usize,
    ) -> Result<Vec<Candidate<Handle<Function>>>> {
        let catalog = self.catalog();
        let explicit_schema = name.schema.is_some();
        let mut result: Vec<Candidate<Handle<Function>>> = Vec::new();
        let mut any_special = false;
        for (func, pathpos) in self.functions_named(name, location)? {
            let proc = &catalog[func];
            let proargtypes: Vec<TypeRef> = proc.input_arguments().map(|a| a.ty).collect();
            let pronargs = proargtypes.len();
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
            let effective_nargs = pronargs.max(nargs);
            let mut args = proargtypes.clone();
            args.resize(
                effective_nargs,
                proargtypes.last().copied().unwrap_or(
                    args.first()
                        .copied()
                        .unwrap_or_else(|| catalog.builtin(pg_catalog::Builtin::Any)),
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
        let t = match &name.schema {
            Some(schema) => {
                catalog.type_named(&QualifiedName::new(schema.as_str(), name.name.as_str()))?
            }
            None => catalog.type_visible(self.search_path, &name.name)?,
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
        expand_variadic: bool,
        expand_defaults: bool,
        location: usize,
    ) -> Result<FuncFound> {
        let catalog = self.catalog();
        let types = &self.coercer.types;
        let argtypes: Vec<TypeRef> = fargs.iter().map(|a| a.ty).collect();
        let nargs = argtypes.len();
        let raw =
            self.funcname_get_candidates(name, nargs, expand_variadic, expand_defaults, location)?;
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
            });
        };
        let proc = &catalog[func];
        let detail = match proc.kind {
            FunctionKind::Aggregate => FuncDetail::Aggregate,
            FunctionKind::Function => FuncDetail::Normal,
            FunctionKind::Procedure => FuncDetail::Procedure,
            FunctionKind::Window => FuncDetail::WindowFunc,
        };
        Ok(FuncFound {
            detail,
            func: Some(func),
            rettype: Some(proc.returns),
            retset: proc.returns_set,
            nvargs: best.nvargs,
            vatype: if best.nvargs > 0 { proc.variadic } else { None },
            declared: best.args,
        })
    }

    /// `func_signature_string`.
    pub fn func_signature_string(&self, name: &ObjectName, argtypes: &[TypeRef]) -> String {
        let args = argtypes
            .iter()
            .map(|&t| self.coercer.types.format(t))
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
        location: usize,
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
        let found = self.func_get_detail(name, fargs, true, true, location)?;
        match found.detail {
            FuncDetail::Procedure => {
                return Err(Diagnostic::new(
                    "42809",
                    format!(
                        "{} is a procedure",
                        self.func_signature_string(name, &argtypes)
                    ),
                )
                .hint("To call a procedure, use CALL.")
                .at(location)
                .into());
            }
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
                        self.func_signature_string(name, &argtypes)
                    ),
                )
                .hint("Could not choose a best candidate function. You might need to add explicit type casts.")
                .at(location)
                .into());
            }
            FuncDetail::NotFound => {
                return Err(Diagnostic::new(
                    "42883",
                    format!(
                        "function {} does not exist",
                        self.func_signature_string(name, &argtypes)
                    ),
                )
                .hint("No function matches the given name and argument types. You might need to add explicit type casts.")
                .at(location)
                .into());
            }
            FuncDetail::Aggregate => {
                return Err(unsupported(
                    "aggregate function call",
                    Some(location..location),
                ));
            }
            FuncDetail::WindowFunc => {
                return Err(Diagnostic::new(
                    "42809",
                    format!("window function {} requires an OVER clause", name.display()),
                )
                .at(location)
                .into());
            }
            FuncDetail::Normal => {}
        }
        let func = found.func.expect("normal function");
        // Defaults: their types are the declared types.
        let mut actual = argtypes.clone();
        let declared = found.declared.clone();
        for &d in &declared[actual.len()..] {
            actual.push(d);
        }
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
            let elem = declared[fargs.len() - found.nvargs];
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

/// What a function call resolved to.
pub(crate) enum ResolvedFunc {
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
