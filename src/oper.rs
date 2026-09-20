//! `parse_oper.c`: which operator an expression names.

use pg_catalog::{Handle, Operator, OperatorKind, QualifiedName};

use crate::diagnostic::{Diagnostic, Result};
use crate::func::{Candidate, FuncResolver, ObjectName};
use crate::types::TypeRef;
use crate::value::{Coercion, Typed};

/// What `make_op` decided.
pub(crate) struct ResolvedOp {
    pub operator: Handle<Operator>,
    pub result: Typed,
    /// Coercions applied to the left operand, when there is one.
    pub left_coercions: Vec<Coercion>,
    /// Coercions applied to the right operand.
    pub right_coercions: Vec<Coercion>,
}

impl<'c> FuncResolver<'c> {
    /// `OpernameGetOprid`: the exact operator, honouring the Search Path.
    fn oper_exact(
        &self,
        name: &ObjectName,
        left: Option<TypeRef>,
        right: TypeRef,
        location: usize,
    ) -> Result<Option<Handle<Operator>>> {
        let catalog = self.catalog();
        let matches = |o: Handle<Operator>| {
            let op = &catalog[o];
            op.left == left && op.right == right
        };
        match &name.schema {
            Some(schema) => {
                if catalog.schema_named(schema).is_none() {
                    // LookupExplicitNamespace(missing_ok = true)
                    let _ = location;
                    return Ok(None);
                }
                Ok(catalog
                    .operators_named(&QualifiedName::new(schema.as_str(), name.name.as_str()))
                    .iter()
                    .copied()
                    .find(|&o| matches(o)))
            }
            None => Ok(catalog
                .operators_visible(self.search_path, &name.name)
                .into_iter()
                .find(|&o| matches(o))),
        }
    }

    /// `OpernameGetCandidates` for one operator kind, with the hiding rule.
    fn oper_candidates(
        &self,
        name: &ObjectName,
        kind: OperatorKind,
        location: usize,
    ) -> Result<Vec<Candidate<Handle<Operator>>>> {
        let catalog = self.catalog();
        let ops: Vec<(Handle<Operator>, usize)> = match &name.schema {
            Some(schema) => {
                self.lookup_schema(schema, location)?;
                catalog
                    .operators_named(&QualifiedName::new(schema.as_str(), name.name.as_str()))
                    .iter()
                    .map(|&o| (o, 0))
                    .collect()
            }
            None => catalog
                .operators_visible(self.search_path, &name.name)
                .into_iter()
                .filter_map(|o| {
                    self.search_path
                        .schemas()
                        .iter()
                        .position(|s| *s == catalog[o].schema)
                        .map(|p| (o, p))
                })
                .collect(),
        };
        let mut result: Vec<Candidate<Handle<Operator>>> = Vec::new();
        for (o, pathpos) in ops {
            let op = &catalog[o];
            if op.kind != kind {
                continue;
            }
            let args = vec![op.left.unwrap_or(op.right), op.right];
            if name.schema.is_none()
                && let Some(prev) = result.iter_mut().find(|c| {
                    let p = &catalog[c.id.expect("operator candidates keep their id")];
                    p.left == op.left && p.right == op.right
                })
            {
                if pathpos > prev.pathpos {
                    continue;
                }
                prev.pathpos = pathpos;
                prev.id = Some(o);
                continue;
            }
            result.push(Candidate {
                id: Some(o),
                pathpos,
                args,
                nvargs: 0,
                ndargs: 0,
            });
        }
        Ok(result)
    }

    /// `binary_oper_exact`.
    fn binary_oper_exact(
        &self,
        name: &ObjectName,
        mut arg1: TypeRef,
        mut arg2: TypeRef,
        location: usize,
    ) -> Result<Option<Handle<Operator>>> {
        let types = &self.coercer.types;
        let mut was_unknown = false;
        if types.is_unknown(arg1) {
            arg1 = arg2;
            was_unknown = true;
        } else if types.is_unknown(arg2) {
            arg2 = arg1;
            was_unknown = true;
        }
        if let Some(o) = self.oper_exact(name, Some(arg1), arg2, location)? {
            return Ok(Some(o));
        }
        if was_unknown {
            let base = types.base_type(arg1);
            if base != arg1
                && let Some(o) = self.oper_exact(name, Some(base), base, location)?
            {
                return Ok(Some(o));
            }
        }
        Ok(None)
    }

    /// `oper_select_candidate` over operator candidates.
    fn oper_select_candidate(
        &self,
        input: &[TypeRef],
        candidates: Vec<Candidate<Handle<Operator>>>,
    ) -> (Option<Handle<Operator>>, bool) {
        let matched = self.func_match_argtypes(input, candidates);
        match matched.len() {
            0 => (None, false),
            1 => (matched[0].id, false),
            _ => match self.func_select_candidate(input, matched) {
                Some(c) => (c.id, false),
                None => (None, true),
            },
        }
    }

    /// `op_signature_string`.
    fn op_signature_string(
        &self,
        name: &ObjectName,
        arg1: Option<TypeRef>,
        arg2: TypeRef,
    ) -> String {
        let types = &self.coercer.types;
        let mut s = String::new();
        if let Some(a) = arg1 {
            s.push_str(&types.format(a));
            s.push(' ');
        }
        s.push_str(&name.display());
        s.push(' ');
        s.push_str(&types.format(arg2));
        s
    }

    fn op_error(
        &self,
        name: &ObjectName,
        arg1: Option<TypeRef>,
        arg2: TypeRef,
        multiple: bool,
        location: usize,
    ) -> Diagnostic {
        if multiple {
            Diagnostic::new(
                "42725",
                format!("operator is not unique: {}", self.op_signature_string(name, arg1, arg2)),
            )
            .hint("Could not choose a best candidate operator. You might need to add explicit type casts.")
            .at(location)
        } else {
            let d = Diagnostic::new(
                "42883",
                format!(
                    "operator does not exist: {}",
                    self.op_signature_string(name, arg1, arg2)
                ),
            );
            let d = if arg1.is_none() {
                d.hint("No operator matches the given name and argument type. You might need to add an explicit type cast.")
            } else {
                d.hint("No operator matches the given name and argument types. You might need to add explicit type casts.")
            };
            d.at(location)
        }
    }

    /// `oper`: a binary operator for the operand types.
    pub fn oper(
        &self,
        name: &ObjectName,
        ltype: TypeRef,
        rtype: TypeRef,
        location: usize,
    ) -> Result<Handle<Operator>> {
        if let Some(o) = self.binary_oper_exact(name, ltype, rtype, location)? {
            return Ok(o);
        }
        let candidates = self.oper_candidates(name, OperatorKind::Infix, location)?;
        let mut multiple = false;
        if !candidates.is_empty() {
            let (found, m) = self.oper_select_candidate(&[ltype, rtype], candidates);
            multiple = m;
            if let Some(o) = found {
                return Ok(o);
            }
        }
        Err(self
            .op_error(name, Some(ltype), rtype, multiple, location)
            .into())
    }

    /// `left_oper`: a prefix operator for the operand type.
    pub fn left_oper(
        &self,
        name: &ObjectName,
        arg: TypeRef,
        location: usize,
    ) -> Result<Handle<Operator>> {
        if let Some(o) = self.oper_exact(name, None, arg, location)? {
            return Ok(o);
        }
        let candidates = self.oper_candidates(name, OperatorKind::Prefix, location)?;
        let mut multiple = false;
        if !candidates.is_empty() {
            let candidates = candidates
                .into_iter()
                .map(|c| Candidate {
                    args: vec![c.args[1]],
                    ..c
                })
                .collect();
            let (found, m) = self.oper_select_candidate(&[arg], candidates);
            multiple = m;
            if let Some(o) = found {
                return Ok(o);
            }
        }
        Err(self.op_error(name, None, arg, multiple, location).into())
    }

    /// `make_op`.
    pub fn make_op(
        &self,
        name: &ObjectName,
        left: Option<&Typed>,
        right: &Typed,
        location: usize,
    ) -> Result<ResolvedOp> {
        let catalog = self.catalog();
        let operator = match left {
            None => self.left_oper(name, right.ty, location)?,
            Some(l) => self.oper(name, l.ty, right.ty, location)?,
        };
        let op = &catalog[operator];
        let (actual, declared): (Vec<TypeRef>, Vec<TypeRef>) = match left {
            None => (vec![right.ty], vec![op.right]),
            Some(l) => (
                vec![l.ty, right.ty],
                vec![op.left.expect("infix operator has a left type"), op.right],
            ),
        };
        let (rettype, declared) = self
            .coercer
            .enforce_generic_type_consistency(&actual, &declared, op.result, false)?;
        let mut left_coercions = Vec::new();
        let mut right_coercions = Vec::new();
        let mut i = 0;
        if let Some(l) = left {
            if actual[0] != declared[0] {
                let (_, steps) = self.coercer.coerce_type(
                    l,
                    declared[0],
                    -1,
                    crate::value::CoercionContext::Implicit,
                )?;
                left_coercions = steps;
            }
            i = 1;
        }
        if actual[i] != declared[i] {
            let (_, steps) = self.coercer.coerce_type(
                right,
                declared[i],
                -1,
                crate::value::CoercionContext::Implicit,
            )?;
            right_coercions = steps;
        }
        let retset = catalog[op.function].returns_set;
        Ok(ResolvedOp {
            operator,
            result: Typed {
                ty: rettype,
                typmod: -1,
                form: crate::value::Form::Other,
                location,
                returns_set: retset,
            },
            left_coercions,
            right_coercions,
        })
    }
}
