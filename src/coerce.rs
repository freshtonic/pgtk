//! `parse_coerce.c`: can one type become another, and how.

use pg_catalog::{Builtin, CastContext, CastMethod, PseudoType, TypeCategory};

use crate::diagnostic::{Diagnostic, Result};
use crate::types::{TypeRef, Types, Typmod};
use crate::value::{Coercion, CoercionContext, CoercionPath, Form, Typed};

/// The coercion machinery over one Catalog.
pub(crate) struct Coercer<'c> {
    pub types: Types<'c>,
    /// The type of each positional parameter as known so far, shared with
    /// the walk: `variable_coerce_param_hook` fixes a parameter's type the
    /// first time it is coerced.
    pub params: std::cell::RefCell<Vec<Option<TypeRef>>>,
}

/// What `find_coercion_pathway` found.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pathway {
    pub path: CoercionPath,
    pub cast: Option<pg_catalog::Handle<pg_catalog::Cast>>,
}

impl<'c> Coercer<'c> {
    pub fn new(catalog: &'c pg_catalog::Catalog) -> Self {
        Self {
            types: Types::new(catalog),
            params: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// `coerce_to_common_type`.
    pub fn coerce_to_common_type(
        &self,
        node: &Typed,
        target: TypeRef,
        context: &str,
    ) -> Result<Vec<Coercion>> {
        if node.ty == target {
            return Ok(Vec::new());
        }
        if self.can_coerce_type(&[node.ty], &[target], CoercionContext::Implicit) {
            let (_, steps) = self.coerce_type(node, target, -1, CoercionContext::Implicit)?;
            Ok(steps)
        } else {
            Err(Diagnostic::new(
                "42846",
                format!(
                    "{context} could not convert type {} to {}",
                    self.types.format(node.ty),
                    self.types.format(target)
                ),
            )
            .at(node.location)
            .into())
        }
    }

    fn pseudo_is(&self, t: TypeRef, p: PseudoType) -> bool {
        self.types.pseudo(t) == Some(&p)
    }

    /// `find_coercion_pathway`.
    pub fn find_coercion_pathway(
        &self,
        target: TypeRef,
        source: TypeRef,
        ccontext: CoercionContext,
    ) -> Option<Pathway> {
        let types = &self.types;
        let source = types.base_type(source);
        let target = types.base_type(target);
        if source == target {
            return Some(Pathway {
                path: CoercionPath::RelabelType,
                cast: None,
            });
        }
        if let Some(cast) = types.catalog.cast(source, target) {
            let row = &types.catalog[cast];
            let castcontext = match row.context {
                CastContext::Implicit => CoercionContext::Implicit,
                CastContext::Assignment => CoercionContext::Assignment,
                CastContext::Explicit => CoercionContext::Explicit,
            };
            if ccontext >= castcontext {
                let path = match row.method {
                    CastMethod::Function(f) => CoercionPath::Func(f),
                    CastMethod::InOut => CoercionPath::CoerceViaIO,
                    CastMethod::Binary => CoercionPath::RelabelType,
                };
                return Some(Pathway {
                    path,
                    cast: Some(cast),
                });
            }
            return None;
        }
        let mut result = None;
        if !types.is(target, Builtin::Oidvector) && !types.is(target, Builtin::Int2vector) {
            if let (Some(target_elem), Some(source_elem)) =
                (types.element_type(target), types.element_type(source))
                && self
                    .find_coercion_pathway(target_elem, source_elem, ccontext)
                    .is_some()
            {
                result = Some(Pathway {
                    path: CoercionPath::ArrayCoerce,
                    cast: None,
                });
            }
        }
        if result.is_none() {
            if ccontext >= CoercionContext::Assignment
                && types.category(target) == TypeCategory::String
            {
                result = Some(Pathway {
                    path: CoercionPath::CoerceViaIO,
                    cast: None,
                });
            } else if ccontext >= CoercionContext::Explicit
                && types.category(source) == TypeCategory::String
            {
                result = Some(Pathway {
                    path: CoercionPath::CoerceViaIO,
                    cast: None,
                });
            }
        }
        result
    }

    /// `find_typmod_coercion_function`: the length-coercion function for a
    /// type, which is the cast from the type to itself. For an array it is
    /// the element's, applied element-wise.
    fn find_typmod_coercion_function(&self, t: TypeRef) -> Option<CoercionPath> {
        let types = &self.types;
        let (t, is_array) = match types.element_type(t) {
            Some(elem) => (elem, true),
            None => (t, false),
        };
        let cast = types.catalog.cast(t, t)?;
        match types.catalog[cast].method {
            CastMethod::Function(f) => Some(if is_array {
                CoercionPath::ArrayCoerce
            } else {
                CoercionPath::Func(f)
            }),
            _ => None,
        }
    }

    /// `can_coerce_type`.
    pub fn can_coerce_type(
        &self,
        inputs: &[TypeRef],
        targets: &[TypeRef],
        ccontext: CoercionContext,
    ) -> bool {
        let types = &self.types;
        let mut have_generics = false;
        for (&input, &target) in inputs.iter().zip(targets) {
            if input == target {
                continue;
            }
            if self.pseudo_is(target, PseudoType::Any) {
                continue;
            }
            if types.is_polymorphic(target) {
                have_generics = true;
                continue;
            }
            if types.is_unknown(input) {
                continue;
            }
            if self
                .find_coercion_pathway(target, input, ccontext)
                .is_some()
            {
                continue;
            }
            if types.is_record(input) && types.is_complex(target) {
                continue;
            }
            if types.is_record(target) && types.is_complex(input) {
                continue;
            }
            if types.is_record_array(target) && types.is_complex_array(input) {
                continue;
            }
            // typeInheritsFrom / typeIsOfTypedTable: inheritance is not in
            // the Catalog, so a row-type conversion is never offered.
            return false;
        }
        if have_generics && !self.check_generic_type_consistency(inputs, targets) {
            return false;
        }
        true
    }

    /// `coerce_to_target_type`: `None` when the coercion is not possible.
    pub fn coerce_to_target_type(
        &self,
        expr: &Typed,
        target: TypeRef,
        target_typmod: Typmod,
        ccontext: CoercionContext,
    ) -> Result<Option<(Typed, Vec<Coercion>)>> {
        if !self.can_coerce_type(&[expr.ty], &[target], ccontext) {
            return Ok(None);
        }
        let (result, mut steps) = self.coerce_type(expr, target, target_typmod, ccontext)?;
        let (result, more) = self.coerce_type_typmod(&result, target, target_typmod, ccontext);
        steps.extend(more);
        Ok(Some((result, steps)))
    }

    /// `coerce_type`. The caller has checked `can_coerce_type`.
    pub fn coerce_type(
        &self,
        node: &Typed,
        target: TypeRef,
        target_typmod: Typmod,
        ccontext: CoercionContext,
    ) -> Result<(Typed, Vec<Coercion>)> {
        let types = &self.types;
        let input = node.ty;
        if target == input {
            return Ok((node.clone(), Vec::new()));
        }
        if matches!(
            types.pseudo(target),
            Some(
                PseudoType::Any
                    | PseudoType::AnyElement
                    | PseudoType::AnyNonArray
                    | PseudoType::AnyCompatible
                    | PseudoType::AnyCompatibleNonArray
            )
        ) {
            return Ok((node.clone(), Vec::new()));
        }
        if matches!(
            types.pseudo(target),
            Some(
                PseudoType::AnyArray
                    | PseudoType::AnyEnum
                    | PseudoType::AnyRange
                    | PseudoType::AnyMultirange
                    | PseudoType::AnyCompatibleArray
                    | PseudoType::AnyCompatibleRange
                    | PseudoType::AnyCompatibleMultirange
            )
        ) && !types.is_unknown(input)
        {
            let base = types.base_type(input);
            if base != input {
                let step = Coercion {
                    path: CoercionPath::RelabelType,
                    cast: None,
                    target: base,
                    typmod: -1,
                    context: ccontext,
                };
                return Ok((
                    Typed {
                        ty: base,
                        typmod: -1,
                        form: Form::Other,
                        ..node.clone()
                    },
                    vec![step],
                ));
            }
            return Ok((node.clone(), Vec::new()));
        }
        if types.is_unknown(input) && node.is_const() {
            let (base, base_typmod) = types.base_type_and_typmod(target, target_typmod);
            let input_typmod = if types.is(base, Builtin::Interval) {
                base_typmod
            } else {
                -1
            };
            let text = match &node.form {
                Form::Const { text, .. } => text.clone(),
                _ => None,
            };
            let mut steps = vec![Coercion {
                path: CoercionPath::UnknownLiteral,
                cast: None,
                target: base,
                typmod: input_typmod,
                context: ccontext,
            }];
            let mut result = Typed {
                ty: base,
                typmod: input_typmod,
                form: Form::Const {
                    is_null: node.is_null_const(),
                    text,
                },
                location: node.location,
                returns_set: false,
            };
            if base != target {
                let (r, more) = self.coerce_to_domain(&result, base, base_typmod, target, ccontext);
                result = r;
                steps.extend(more);
            }
            return Ok((result, steps));
        }
        if let Form::Param { number } = node.form
            && types.is_unknown(input)
        {
            // variable_coerce_param_hook.
            let mut params = self.params.borrow_mut();
            let slot = &mut params[number - 1];
            match *slot {
                None => *slot = Some(target),
                Some(t) if t == target || types.is_unknown(t) => *slot = Some(target),
                Some(t) => {
                    return Err(Diagnostic::new(
                        "42P08",
                        format!("inconsistent types deduced for parameter ${number}"),
                    )
                    .detail(format!(
                        "{} versus {}",
                        types.format(t),
                        types.format(target)
                    ))
                    .at(node.location)
                    .into());
                }
            }
            let step = Coercion {
                path: CoercionPath::ParamType,
                cast: None,
                target,
                typmod: -1,
                context: ccontext,
            };
            return Ok((
                Typed {
                    ty: target,
                    typmod: -1,
                    form: Form::Param { number },
                    location: node.location,
                    returns_set: false,
                },
                vec![step],
            ));
        }
        if let Some(pathway) = self.find_coercion_pathway(target, input, ccontext) {
            if pathway.path != CoercionPath::RelabelType {
                let (base, base_typmod) = types.base_type_and_typmod(target, target_typmod);
                let (mut result, mut steps) =
                    self.build_coercion_expression(node, pathway, base, base_typmod, ccontext);
                if target != base {
                    let (r, more) =
                        self.coerce_to_domain(&result, base, base_typmod, target, ccontext);
                    result = r;
                    steps.extend(more);
                }
                return Ok((result, steps));
            }
            let (base, base_typmod) = types.base_type_and_typmod(target, -1);
            let (result, steps) = self.coerce_to_domain(node, base, base_typmod, target, ccontext);
            if steps.is_empty() {
                let step = Coercion {
                    path: CoercionPath::RelabelType,
                    cast: pathway.cast,
                    target,
                    typmod: -1,
                    context: ccontext,
                };
                return Ok((
                    Typed {
                        ty: target,
                        typmod: -1,
                        form: Form::Other,
                        ..node.clone()
                    },
                    vec![step],
                ));
            }
            return Ok((result, steps));
        }
        if types.is_record(input) && types.is_complex(target) {
            let step = Coercion {
                path: CoercionPath::RecordToComplex,
                cast: None,
                target,
                typmod: -1,
                context: ccontext,
            };
            return Ok((
                Typed {
                    ty: target,
                    typmod: -1,
                    form: Form::Other,
                    ..node.clone()
                },
                vec![step],
            ));
        }
        if types.is_record(target) && types.is_complex(input) {
            return Ok((node.clone(), Vec::new()));
        }
        if types.is_record_array(target) && types.is_complex_array(input) {
            return Ok((node.clone(), Vec::new()));
        }
        Err(crate::diagnostic::unsupported(
            format!(
                "conversion from {} to {}",
                types.format(input),
                types.format(target)
            ),
            None,
        ))
    }

    /// `build_coercion_expression`: the type facts after a pathway is taken.
    fn build_coercion_expression(
        &self,
        node: &Typed,
        pathway: Pathway,
        target: TypeRef,
        target_typmod: Typmod,
        ccontext: CoercionContext,
    ) -> (Typed, Vec<Coercion>) {
        // A function coercion with a typmod argument carries the typmod;
        // otherwise the result has none. CoerceViaIO and ArrayCoerce carry
        // the target typmod only when the element path does; keep it simple
        // and honest: -1 unless a three-argument cast function is used.
        let typmod = match pathway.path {
            CoercionPath::Func(f) => {
                let nargs = self.types.catalog[f].input_arguments().count();
                if nargs >= 2 { target_typmod } else { -1 }
            }
            CoercionPath::ArrayCoerce => target_typmod,
            _ => -1,
        };
        let step = Coercion {
            path: pathway.path,
            cast: pathway.cast,
            target,
            typmod,
            context: ccontext,
        };
        (
            Typed {
                ty: target,
                typmod,
                form: Form::Other,
                location: node.location,
                returns_set: node.returns_set,
            },
            vec![step],
        )
    }

    /// `coerce_to_domain`.
    fn coerce_to_domain(
        &self,
        arg: &Typed,
        base: TypeRef,
        base_typmod: Typmod,
        domain: TypeRef,
        ccontext: CoercionContext,
    ) -> (Typed, Vec<Coercion>) {
        if base == domain {
            return (arg.clone(), Vec::new());
        }
        let (arg, mut steps) = self.coerce_type_typmod(arg, base, base_typmod, ccontext);
        steps.push(Coercion {
            path: CoercionPath::Domain,
            cast: None,
            target: domain,
            typmod: -1,
            context: ccontext,
        });
        (
            Typed {
                ty: domain,
                typmod: -1,
                form: Form::Other,
                ..arg
            },
            steps,
        )
    }

    /// `coerce_type_typmod`.
    fn coerce_type_typmod(
        &self,
        node: &Typed,
        target: TypeRef,
        target_typmod: Typmod,
        ccontext: CoercionContext,
    ) -> (Typed, Vec<Coercion>) {
        if target_typmod == node.typmod {
            return (node.clone(), Vec::new());
        }
        let pathway = if target_typmod < 0 {
            None
        } else {
            self.find_typmod_coercion_function(target)
        };
        match pathway {
            Some(path) => {
                let step = Coercion {
                    path,
                    cast: None,
                    target,
                    typmod: target_typmod,
                    context: ccontext,
                };
                (
                    Typed {
                        ty: target,
                        typmod: target_typmod,
                        form: Form::Other,
                        ..node.clone()
                    },
                    vec![step],
                )
            }
            None => {
                // applyRelabelType: a RelabelType carrying the typmod.
                let step = Coercion {
                    path: CoercionPath::RelabelType,
                    cast: None,
                    target,
                    typmod: target_typmod,
                    context: ccontext,
                };
                (
                    Typed {
                        ty: target,
                        typmod: target_typmod,
                        form: Form::Other,
                        ..node.clone()
                    },
                    vec![step],
                )
            }
        }
    }

    /// `coerce_to_boolean`.
    pub fn coerce_to_boolean(
        &self,
        node: &Typed,
        construct: &str,
    ) -> Result<(Typed, Vec<Coercion>)> {
        let types = &self.types;
        let bool_type = types.builtin(Builtin::Bool);
        let (node, steps) = if node.ty != bool_type {
            match self.coerce_to_target_type(node, bool_type, -1, CoercionContext::Assignment)? {
                Some(r) => r,
                None => {
                    return Err(Diagnostic::new(
                        "42804",
                        format!(
                            "argument of {construct} must be type {}, not type {}",
                            "boolean",
                            types.format(node.ty)
                        ),
                    )
                    .at(node.location)
                    .into());
                }
            }
        } else {
            (node.clone(), Vec::new())
        };
        if node.returns_set {
            return Err(Diagnostic::new(
                "42804",
                format!("argument of {construct} must not return a set"),
            )
            .at(node.location)
            .into());
        }
        Ok((node, steps))
    }

    /// `check_generic_type_consistency`.
    pub fn check_generic_type_consistency(&self, actual: &[TypeRef], declared: &[TypeRef]) -> bool {
        let types = &self.types;
        let mut elem_typeid = None;
        let mut array_typeid = None;
        let mut range_typeid = None;
        let mut multirange_typeid = None;
        let mut anycompatible_range_typeid = None;
        let mut anycompatible_range_typelem = None;
        let mut anycompatible_multirange_typeid = None;
        let mut anycompatible_multirange_typelem: Option<TypeRef> = None;
        let mut have_anynonarray = false;
        let mut have_anyenum = false;
        let mut have_anycompatible_nonarray = false;
        let mut anycompatible_actual: Vec<TypeRef> = Vec::new();

        for (&actual_type, &decl_type) in actual.iter().zip(declared) {
            let Some(decl) = types.pseudo(decl_type).cloned() else {
                continue;
            };
            let unknown = types.is_unknown(actual_type);
            match decl {
                PseudoType::AnyElement | PseudoType::AnyNonArray | PseudoType::AnyEnum => {
                    if decl == PseudoType::AnyNonArray {
                        have_anynonarray = true;
                    } else if decl == PseudoType::AnyEnum {
                        have_anyenum = true;
                    }
                    if unknown {
                        continue;
                    }
                    if elem_typeid.is_some_and(|e| e != actual_type) {
                        return false;
                    }
                    elem_typeid = Some(actual_type);
                }
                PseudoType::AnyArray => {
                    if unknown {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    if array_typeid.is_some_and(|e| e != a) {
                        return false;
                    }
                    array_typeid = Some(a);
                }
                PseudoType::AnyRange => {
                    if unknown {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    if range_typeid.is_some_and(|e| e != a) {
                        return false;
                    }
                    range_typeid = Some(a);
                }
                PseudoType::AnyMultirange => {
                    if unknown {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    if multirange_typeid.is_some_and(|e| e != a) {
                        return false;
                    }
                    multirange_typeid = Some(a);
                }
                PseudoType::AnyCompatible | PseudoType::AnyCompatibleNonArray => {
                    if decl == PseudoType::AnyCompatibleNonArray {
                        have_anycompatible_nonarray = true;
                    }
                    if unknown {
                        continue;
                    }
                    anycompatible_actual.push(actual_type);
                }
                PseudoType::AnyCompatibleArray => {
                    if unknown {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    let Some(elem) = types.element_type(a) else {
                        return false;
                    };
                    anycompatible_actual.push(elem);
                }
                PseudoType::AnyCompatibleRange => {
                    if unknown {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    match anycompatible_range_typeid {
                        Some(r) => {
                            if r != a {
                                return false;
                            }
                        }
                        None => {
                            anycompatible_range_typeid = Some(a);
                            let Some(sub) = types.range_subtype(a) else {
                                return false;
                            };
                            anycompatible_range_typelem = Some(sub);
                            anycompatible_actual.push(sub);
                        }
                    }
                }
                PseudoType::AnyCompatibleMultirange => {
                    if unknown {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    match anycompatible_multirange_typeid {
                        Some(r) => {
                            if r != a {
                                return false;
                            }
                        }
                        None => {
                            anycompatible_multirange_typeid = Some(a);
                            let Some(range) = types.multirange_range(a) else {
                                return false;
                            };
                            anycompatible_multirange_typelem = Some(range);
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(array) = array_typeid
            && !self.pseudo_is(array, PseudoType::AnyArray)
        {
            let Some(elem) = types.element_type(array) else {
                return false;
            };
            match elem_typeid {
                None => elem_typeid = Some(elem),
                Some(e) if e != elem => return false,
                _ => {}
            }
        }
        if let Some(multirange) = multirange_typeid {
            let Some(range) = types.multirange_range(multirange) else {
                return false;
            };
            match range_typeid {
                None => {
                    range_typeid = Some(range);
                    if types.range_subtype(range).is_none() {
                        return false;
                    }
                }
                Some(r) if r != range => return false,
                _ => {}
            }
        }
        if let Some(range) = range_typeid {
            let Some(sub) = types.range_subtype(range) else {
                return false;
            };
            match elem_typeid {
                None => elem_typeid = Some(sub),
                Some(e) if e != sub => return false,
                _ => {}
            }
        }
        if have_anynonarray && elem_typeid.is_some_and(|e| types.is_array_domain(e)) {
            return false;
        }
        if have_anyenum && !elem_typeid.is_some_and(|e| types.is_enum(e)) {
            return false;
        }
        if let Some(multirange) = anycompatible_multirange_typeid {
            let telem = anycompatible_multirange_typelem.expect("set together");
            match anycompatible_range_typeid {
                Some(r) => {
                    if telem != r {
                        return false;
                    }
                }
                None => {
                    anycompatible_range_typeid = Some(telem);
                    let Some(sub) = types.range_subtype(telem) else {
                        return false;
                    };
                    anycompatible_range_typelem = Some(sub);
                    anycompatible_actual.push(sub);
                }
            }
            let _ = multirange;
        }
        if !anycompatible_actual.is_empty() {
            let Ok(common) = self.common_type(&anycompatible_actual) else {
                return false;
            };
            if !self.verify_common_type_from_oids(common, &anycompatible_actual) {
                return false;
            }
            if have_anycompatible_nonarray && types.is_array_domain(common) {
                return false;
            }
            if anycompatible_range_typelem.is_some_and(|e| e != common) {
                return false;
            }
        }
        true
    }

    /// `enforce_generic_type_consistency`: resolve the declared types and
    /// the result type. Returns the result type and the resolved declared
    /// argument types.
    pub fn enforce_generic_type_consistency(
        &self,
        actual: &[TypeRef],
        declared: &[TypeRef],
        rettype: TypeRef,
        allow_poly: bool,
    ) -> Result<(TypeRef, Vec<TypeRef>)> {
        let types = &self.types;
        let mut declared = declared.to_vec();
        let mismatch = |msg: String| -> crate::diagnostic::AnalyzeError {
            Diagnostic::new("42804", msg).into()
        };
        let ret_pseudo = types.pseudo(rettype).cloned();
        let mut have_poly_anycompatible = false;
        let mut have_poly_unknowns = false;
        let mut elem_typeid: Option<TypeRef> = None;
        let mut array_typeid: Option<TypeRef> = None;
        let mut range_typeid: Option<TypeRef> = None;
        let mut multirange_typeid: Option<TypeRef> = None;
        let mut anycompatible_typeid: Option<TypeRef> = None;
        let mut anycompatible_array_typeid: Option<TypeRef> = None;
        let mut anycompatible_range_typeid: Option<TypeRef> = None;
        let mut anycompatible_range_typelem: Option<TypeRef> = None;
        let mut anycompatible_multirange_typeid: Option<TypeRef> = None;
        let mut anycompatible_multirange_typelem: Option<TypeRef> = None;
        let mut have_anynonarray = ret_pseudo == Some(PseudoType::AnyNonArray);
        let mut have_anyenum = ret_pseudo == Some(PseudoType::AnyEnum);
        let mut have_anymultirange = ret_pseudo == Some(PseudoType::AnyMultirange);
        let mut have_anycompatible_nonarray = ret_pseudo == Some(PseudoType::AnyCompatibleNonArray);
        let mut have_anycompatible_array = ret_pseudo == Some(PseudoType::AnyCompatibleArray);
        let mut have_anycompatible_range = ret_pseudo == Some(PseudoType::AnyCompatibleRange);
        let mut have_anycompatible_multirange =
            ret_pseudo == Some(PseudoType::AnyCompatibleMultirange);
        let mut n_poly_args = 0;
        let mut anycompatible_actual: Vec<TypeRef> = Vec::new();

        for (j, &actual_type) in actual.iter().enumerate() {
            let decl_type = declared[j];
            let Some(decl) = types.pseudo(decl_type).cloned() else {
                continue;
            };
            let unknown = types.is_unknown(actual_type);
            match decl {
                PseudoType::AnyElement | PseudoType::AnyNonArray | PseudoType::AnyEnum => {
                    n_poly_args += 1;
                    if decl == PseudoType::AnyNonArray {
                        have_anynonarray = true;
                    } else if decl == PseudoType::AnyEnum {
                        have_anyenum = true;
                    }
                    if unknown {
                        have_poly_unknowns = true;
                        continue;
                    }
                    if allow_poly && decl_type == actual_type {
                        continue;
                    }
                    if let Some(e) = elem_typeid
                        && e != actual_type
                    {
                        return Err(Diagnostic::new(
                            "42804",
                            "arguments declared \"anyelement\" are not all alike".to_owned(),
                        )
                        .detail(format!(
                            "{} versus {}",
                            types.format(e),
                            types.format(actual_type)
                        ))
                        .into());
                    }
                    elem_typeid = Some(actual_type);
                }
                PseudoType::AnyArray => {
                    n_poly_args += 1;
                    if unknown {
                        have_poly_unknowns = true;
                        continue;
                    }
                    if allow_poly && decl_type == actual_type {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    if let Some(e) = array_typeid
                        && e != a
                    {
                        return Err(Diagnostic::new(
                            "42804",
                            "arguments declared \"anyarray\" are not all alike".to_owned(),
                        )
                        .detail(format!("{} versus {}", types.format(e), types.format(a)))
                        .into());
                    }
                    array_typeid = Some(a);
                }
                PseudoType::AnyRange => {
                    n_poly_args += 1;
                    if unknown {
                        have_poly_unknowns = true;
                        continue;
                    }
                    if allow_poly && decl_type == actual_type {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    if let Some(e) = range_typeid
                        && e != a
                    {
                        return Err(Diagnostic::new(
                            "42804",
                            "arguments declared \"anyrange\" are not all alike".to_owned(),
                        )
                        .detail(format!("{} versus {}", types.format(e), types.format(a)))
                        .into());
                    }
                    range_typeid = Some(a);
                }
                PseudoType::AnyMultirange => {
                    n_poly_args += 1;
                    have_anymultirange = true;
                    if unknown {
                        have_poly_unknowns = true;
                        continue;
                    }
                    if allow_poly && decl_type == actual_type {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    if let Some(e) = multirange_typeid
                        && e != a
                    {
                        return Err(Diagnostic::new(
                            "42804",
                            "arguments declared \"anymultirange\" are not all alike".to_owned(),
                        )
                        .detail(format!("{} versus {}", types.format(e), types.format(a)))
                        .into());
                    }
                    multirange_typeid = Some(a);
                }
                PseudoType::AnyCompatible | PseudoType::AnyCompatibleNonArray => {
                    have_poly_anycompatible = true;
                    if decl == PseudoType::AnyCompatibleNonArray {
                        have_anycompatible_nonarray = true;
                    }
                    if unknown {
                        continue;
                    }
                    if allow_poly && decl_type == actual_type {
                        continue;
                    }
                    anycompatible_actual.push(actual_type);
                }
                PseudoType::AnyCompatibleArray => {
                    have_poly_anycompatible = true;
                    have_anycompatible_array = true;
                    if unknown {
                        continue;
                    }
                    if allow_poly && decl_type == actual_type {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    let Some(elem) = types.element_type(a) else {
                        return Err(mismatch(format!(
                            "argument declared {} is not an array but type {}",
                            "anycompatiblearray",
                            types.format(a)
                        )));
                    };
                    anycompatible_actual.push(elem);
                }
                PseudoType::AnyCompatibleRange => {
                    have_poly_anycompatible = true;
                    have_anycompatible_range = true;
                    if unknown {
                        continue;
                    }
                    if allow_poly && decl_type == actual_type {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    match anycompatible_range_typeid {
                        Some(r) => {
                            if r != a {
                                return Err(Diagnostic::new(
                                    "42804",
                                    "arguments declared \"anycompatiblerange\" are not all alike"
                                        .to_owned(),
                                )
                                .detail(format!("{} versus {}", types.format(r), types.format(a)))
                                .into());
                            }
                        }
                        None => {
                            anycompatible_range_typeid = Some(a);
                            let Some(sub) = types.range_subtype(a) else {
                                return Err(mismatch(format!(
                                    "argument declared {} is not a range type but type {}",
                                    "anycompatiblerange",
                                    types.format(a)
                                )));
                            };
                            anycompatible_range_typelem = Some(sub);
                            anycompatible_actual.push(sub);
                        }
                    }
                }
                PseudoType::AnyCompatibleMultirange => {
                    have_poly_anycompatible = true;
                    have_anycompatible_multirange = true;
                    if unknown {
                        continue;
                    }
                    if allow_poly && decl_type == actual_type {
                        continue;
                    }
                    let a = types.base_type(actual_type);
                    match anycompatible_multirange_typeid {
                        Some(r) => {
                            if r != a {
                                return Err(Diagnostic::new(
                                    "42804",
                                    "arguments declared \"anycompatiblemultirange\" are not all alike".to_owned(),
                                )
                                .detail(format!("{} versus {}", types.format(r), types.format(a)))
                                .into());
                            }
                        }
                        None => {
                            anycompatible_multirange_typeid = Some(a);
                            let Some(range) = types.multirange_range(a) else {
                                return Err(mismatch(format!(
                                    "argument declared {} is not a multirange type but type {}",
                                    "anycompatiblemultirange",
                                    types.format(a)
                                )));
                            };
                            anycompatible_multirange_typelem = Some(range);
                        }
                    }
                }
                _ => {}
            }
        }

        if n_poly_args == 0 && !have_poly_anycompatible {
            return Ok((rettype, declared));
        }

        if n_poly_args > 0 {
            if let Some(array) = array_typeid {
                let array_typelem = if self.pseudo_is(array, PseudoType::AnyArray) {
                    if n_poly_args != 1
                        || (ret_pseudo != Some(PseudoType::AnyArray)
                            && types.is_polymorphic_family1(rettype))
                    {
                        return Err(mismatch(
                            "cannot determine element type of \"anyarray\" argument".to_owned(),
                        ));
                    }
                    types.builtin(Builtin::Anyelement)
                } else {
                    match types.element_type(array) {
                        Some(e) => e,
                        None => {
                            return Err(mismatch(format!(
                                "argument declared {} is not an array but type {}",
                                "anyarray",
                                types.format(array)
                            )));
                        }
                    }
                };
                match elem_typeid {
                    None => elem_typeid = Some(array_typelem),
                    Some(e) if e != array_typelem => {
                        return Err(Diagnostic::new(
                            "42804",
                            format!(
                                "argument declared {} is not consistent with argument declared {}",
                                "anyarray", "anyelement"
                            ),
                        )
                        .detail(format!(
                            "{} versus {}",
                            types.format(array),
                            types.format(e)
                        ))
                        .into());
                    }
                    _ => {}
                }
            }
            if let Some(multirange) = multirange_typeid {
                let Some(range) = types.multirange_range(multirange) else {
                    return Err(mismatch(format!(
                        "argument declared {} is not a multirange type but type {}",
                        "anymultirange",
                        types.format(multirange)
                    )));
                };
                match range_typeid {
                    None => range_typeid = Some(range),
                    Some(r) if r != range => {
                        return Err(Diagnostic::new(
                            "42804",
                            format!(
                                "argument declared {} is not consistent with argument declared {}",
                                "anymultirange", "anyrange"
                            ),
                        )
                        .detail(format!(
                            "{} versus {}",
                            types.format(multirange),
                            types.format(r)
                        ))
                        .into());
                    }
                    _ => {}
                }
            } else if have_anymultirange && let Some(range) = range_typeid {
                multirange_typeid = types.range_multirange(range);
            }
            if let Some(range) = range_typeid {
                let Some(sub) = types.range_subtype(range) else {
                    return Err(mismatch(format!(
                        "argument declared {} is not a range type but type {}",
                        "anyrange",
                        types.format(range)
                    )));
                };
                match elem_typeid {
                    None => elem_typeid = Some(sub),
                    Some(e) if e != sub => {
                        return Err(Diagnostic::new(
                            "42804",
                            format!(
                                "argument declared {} is not consistent with argument declared {}",
                                "anyrange", "anyelement"
                            ),
                        )
                        .detail(format!(
                            "{} versus {}",
                            types.format(range),
                            types.format(e)
                        ))
                        .into());
                    }
                    _ => {}
                }
            }
            if elem_typeid.is_none() {
                if allow_poly {
                    elem_typeid = Some(types.builtin(Builtin::Anyelement));
                    array_typeid = Some(types.builtin(Builtin::Anyarray));
                    range_typeid = Some(types.builtin(Builtin::Anyrange));
                    multirange_typeid = Some(types.builtin(Builtin::Anymultirange));
                } else {
                    return Err(mismatch(format!(
                        "could not determine polymorphic type because input has type {}",
                        "unknown"
                    )));
                }
            }
            let elem = elem_typeid.expect("set above");
            if have_anynonarray
                && !self.pseudo_is(elem, PseudoType::AnyElement)
                && types.is_array_domain(elem)
            {
                return Err(mismatch(format!(
                    "type matched to anynonarray is an array type: {}",
                    types.format(elem)
                )));
            }
            if have_anyenum && !self.pseudo_is(elem, PseudoType::AnyElement) && !types.is_enum(elem)
            {
                return Err(mismatch(format!(
                    "type matched to anyenum is not an enum type: {}",
                    types.format(elem)
                )));
            }
        }

        if have_poly_anycompatible {
            if let Some(multirange) = anycompatible_multirange_typeid {
                let telem = anycompatible_multirange_typelem.expect("set together");
                match anycompatible_range_typeid {
                    Some(r) => {
                        if telem != r {
                            return Err(Diagnostic::new(
                                "42804",
                                format!(
                                    "argument declared {} is not consistent with argument declared {}",
                                    "anycompatiblemultirange", "anycompatiblerange"
                                ),
                            )
                            .detail(format!("{} versus {}", types.format(multirange), types.format(r)))
                            .into());
                        }
                    }
                    None => {
                        anycompatible_range_typeid = Some(telem);
                        let Some(sub) = types.range_subtype(telem) else {
                            return Err(mismatch(format!(
                                "argument declared {} is not a multirange type but type {}",
                                "anycompatiblemultirange",
                                types.format(multirange)
                            )));
                        };
                        anycompatible_range_typelem = Some(sub);
                        have_anycompatible_range = true;
                        anycompatible_actual.push(sub);
                    }
                }
            } else if have_anycompatible_multirange && let Some(r) = anycompatible_range_typeid {
                anycompatible_multirange_typeid = types.range_multirange(r);
            }
            if !anycompatible_actual.is_empty() {
                let common = self.select_common_type_from_oids(&anycompatible_actual)?;
                if !self.verify_common_type_from_oids(common, &anycompatible_actual) {
                    return Err(mismatch(
                        "arguments of anycompatible family cannot be cast to a common type"
                            .to_owned(),
                    ));
                }
                anycompatible_typeid = Some(common);
                if have_anycompatible_array {
                    match types.array_type(common) {
                        Some(a) => anycompatible_array_typeid = Some(a),
                        None => {
                            return Err(Diagnostic::new(
                                "42704",
                                format!(
                                    "could not find array type for data type {}",
                                    types.format(common)
                                ),
                            )
                            .into());
                        }
                    }
                }
                if have_anycompatible_range {
                    if anycompatible_range_typeid.is_none() {
                        return Err(mismatch(format!(
                            "could not determine polymorphic type {} because input has type {}",
                            "anycompatiblerange", "unknown"
                        )));
                    }
                    if anycompatible_range_typelem != Some(common) {
                        return Err(mismatch(format!(
                            "anycompatiblerange type {} does not match anycompatible type {}",
                            types.format(anycompatible_range_typeid.expect("checked")),
                            types.format(common)
                        )));
                    }
                }
                if have_anycompatible_multirange {
                    if anycompatible_multirange_typeid.is_none() {
                        return Err(mismatch(format!(
                            "could not determine polymorphic type {} because input has type {}",
                            "anycompatiblemultirange", "unknown"
                        )));
                    }
                    if anycompatible_range_typelem != Some(common) {
                        return Err(mismatch(format!(
                            "anycompatiblemultirange type {} does not match anycompatible type {}",
                            types.format(anycompatible_multirange_typeid.expect("checked")),
                            types.format(common)
                        )));
                    }
                }
                if have_anycompatible_nonarray && types.is_array_domain(common) {
                    return Err(mismatch(format!(
                        "type matched to anycompatiblenonarray is an array type: {}",
                        types.format(common)
                    )));
                }
            } else if allow_poly {
                anycompatible_typeid = Some(types.builtin(Builtin::Anycompatible));
                anycompatible_array_typeid = Some(types.builtin(Builtin::Anycompatiblearray));
                anycompatible_range_typeid = Some(types.builtin(Builtin::Anycompatiblerange));
                anycompatible_multirange_typeid =
                    Some(types.builtin(Builtin::Anycompatiblemultirange));
            } else {
                anycompatible_typeid = Some(types.builtin(Builtin::Text));
                anycompatible_array_typeid = Some(types.builtin(Builtin::TextArray));
                if have_anycompatible_range {
                    return Err(mismatch(format!(
                        "could not determine polymorphic type {} because input has type {}",
                        "anycompatiblerange", "unknown"
                    )));
                }
                if have_anycompatible_multirange {
                    return Err(mismatch(format!(
                        "could not determine polymorphic type {} because input has type {}",
                        "anycompatiblemultirange", "unknown"
                    )));
                }
            }
            for d in declared.iter_mut() {
                match types.pseudo(*d) {
                    Some(PseudoType::AnyCompatible | PseudoType::AnyCompatibleNonArray) => {
                        *d = anycompatible_typeid.expect("set above");
                    }
                    Some(PseudoType::AnyCompatibleArray) => {
                        if let Some(a) = anycompatible_array_typeid {
                            *d = a;
                        }
                    }
                    Some(PseudoType::AnyCompatibleRange) => {
                        if let Some(a) = anycompatible_range_typeid {
                            *d = a;
                        }
                    }
                    Some(PseudoType::AnyCompatibleMultirange) => {
                        if let Some(a) = anycompatible_multirange_typeid {
                            *d = a;
                        }
                    }
                    _ => {}
                }
            }
        }

        if have_poly_unknowns {
            let elem = elem_typeid.expect("set when n_poly_args > 0");
            for (j, &actual_type) in actual.iter().enumerate() {
                if !types.is_unknown(actual_type) {
                    continue;
                }
                match types.pseudo(declared[j]) {
                    Some(
                        PseudoType::AnyElement | PseudoType::AnyNonArray | PseudoType::AnyEnum,
                    ) => {
                        declared[j] = elem;
                    }
                    Some(PseudoType::AnyArray) => {
                        if array_typeid.is_none() {
                            match types.array_type(elem) {
                                Some(a) => array_typeid = Some(a),
                                None => {
                                    return Err(Diagnostic::new(
                                        "42704",
                                        format!(
                                            "could not find array type for data type {}",
                                            types.format(elem)
                                        ),
                                    )
                                    .into());
                                }
                            }
                        }
                        declared[j] = array_typeid.expect("set above");
                    }
                    Some(PseudoType::AnyRange) => match range_typeid {
                        Some(r) => declared[j] = r,
                        None => {
                            return Err(mismatch(format!(
                                "could not determine polymorphic type {} because input has type {}",
                                "anyrange", "unknown"
                            )));
                        }
                    },
                    Some(PseudoType::AnyMultirange) => match multirange_typeid {
                        Some(r) => declared[j] = r,
                        None => {
                            return Err(mismatch(format!(
                                "could not determine polymorphic type {} because input has type {}",
                                "anymultirange", "unknown"
                            )));
                        }
                    },
                    _ => {}
                }
            }
        }

        let result = match ret_pseudo {
            Some(PseudoType::AnyElement | PseudoType::AnyNonArray | PseudoType::AnyEnum) => {
                elem_typeid.expect("set when n_poly_args > 0")
            }
            Some(PseudoType::AnyArray) => match array_typeid {
                Some(a) => a,
                None => {
                    let elem = elem_typeid.expect("set");
                    match types.array_type(elem) {
                        Some(a) => a,
                        None => {
                            return Err(Diagnostic::new(
                                "42704",
                                format!(
                                    "could not find array type for data type {}",
                                    types.format(elem)
                                ),
                            )
                            .into());
                        }
                    }
                }
            },
            Some(PseudoType::AnyRange) => match range_typeid {
                Some(r) => r,
                None => {
                    return Err(mismatch(format!(
                        "could not determine polymorphic type {} because input has type {}",
                        "anyrange", "unknown"
                    )));
                }
            },
            Some(PseudoType::AnyMultirange) => match multirange_typeid {
                Some(r) => r,
                None => {
                    return Err(mismatch(format!(
                        "could not determine polymorphic type {} because input has type {}",
                        "anymultirange", "unknown"
                    )));
                }
            },
            Some(PseudoType::AnyCompatible | PseudoType::AnyCompatibleNonArray) => {
                match anycompatible_typeid {
                    Some(t) => t,
                    None => {
                        return Err(mismatch("could not identify anycompatible type".to_owned()));
                    }
                }
            }
            Some(PseudoType::AnyCompatibleArray) => match anycompatible_array_typeid {
                Some(t) => t,
                None => {
                    return Err(mismatch(
                        "could not identify anycompatiblearray type".to_owned(),
                    ));
                }
            },
            Some(PseudoType::AnyCompatibleRange) => match anycompatible_range_typeid {
                Some(t) => t,
                None => {
                    return Err(mismatch(
                        "could not identify anycompatiblerange type".to_owned(),
                    ));
                }
            },
            Some(PseudoType::AnyCompatibleMultirange) => match anycompatible_multirange_typeid {
                Some(t) => t,
                None => {
                    return Err(mismatch(
                        "could not identify anycompatiblemultirange type".to_owned(),
                    ));
                }
            },
            _ => rettype,
        };
        Ok((result, declared))
    }

    /// `select_common_type_from_oids` in its erroring form.
    pub fn select_common_type_from_oids(&self, typeids: &[TypeRef]) -> Result<TypeRef> {
        self.common_type(typeids).map_err(|(p, n)| {
            Diagnostic::new(
                "42804",
                format!(
                    "argument types {} and {} cannot be matched",
                    self.types.format(p),
                    self.types.format(n)
                ),
            )
            .into()
        })
    }

    /// The shared body of `select_common_type` and its OID form: `Err` is
    /// the pair of types that could not be matched.
    pub fn common_type(
        &self,
        typeids: &[TypeRef],
    ) -> std::result::Result<TypeRef, (TypeRef, TypeRef)> {
        let types = &self.types;
        let mut ptype = typeids[0];
        let mut i = 1;
        if !types.is_unknown(ptype) {
            while i < typeids.len() && typeids[i] == ptype {
                i += 1;
            }
            if i == typeids.len() {
                return Ok(ptype);
            }
        }
        ptype = types.base_type(ptype);
        let mut pcategory = types.category(ptype);
        let mut pispreferred = types.catalog[ptype].is_preferred;
        while i < typeids.len() {
            let ntype = types.base_type(typeids[i]);
            i += 1;
            if !types.is_unknown(ntype) && ntype != ptype {
                let ncategory = types.category(ntype);
                let nispreferred = types.catalog[ntype].is_preferred;
                if types.is_unknown(ptype) {
                    ptype = ntype;
                    pcategory = ncategory;
                    pispreferred = nispreferred;
                } else if ncategory != pcategory {
                    return Err((ptype, ntype));
                } else if !pispreferred
                    && self.can_coerce_type(&[ptype], &[ntype], CoercionContext::Implicit)
                    && !self.can_coerce_type(&[ntype], &[ptype], CoercionContext::Implicit)
                {
                    ptype = ntype;
                    pcategory = ncategory;
                    pispreferred = nispreferred;
                }
            }
        }
        if types.is_unknown(ptype) {
            ptype = types.builtin(Builtin::Text);
        }
        Ok(ptype)
    }

    /// `verify_common_type_from_oids`.
    fn verify_common_type_from_oids(&self, common: TypeRef, typeids: &[TypeRef]) -> bool {
        typeids
            .iter()
            .all(|&t| self.can_coerce_type(&[t], &[common], CoercionContext::Implicit))
    }
}
