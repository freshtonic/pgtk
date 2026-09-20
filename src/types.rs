//! Type helpers over the Catalog: the questions `parse_coerce.c` asks of
//! `pg_type`, answered from `pg-catalog` handles.

use pg_catalog::{Builtin, Catalog, Handle, PseudoType, Type, TypeCategory, TypeKind};

/// A Catalog Handle for a type, or the absence of one where PostgreSQL uses
/// `InvalidOid` (a prefix operator's missing left operand).
pub type TypeRef = Handle<Type>;

/// A type modifier as PostgreSQL stores it; `-1` is none.
pub type Typmod = i32;

/// Questions about types, answered from the Catalog.
#[allow(dead_code)]
pub(crate) struct Types<'c> {
    pub catalog: &'c Catalog,
}

#[allow(dead_code)]
impl<'c> Types<'c> {
    pub fn new(catalog: &'c Catalog) -> Self {
        Self { catalog }
    }

    pub fn builtin(&self, b: Builtin) -> TypeRef {
        self.catalog.builtin(b)
    }

    pub fn is(&self, t: TypeRef, b: Builtin) -> bool {
        self.catalog.as_builtin(t) == Some(b)
    }

    pub fn oid(&self, t: TypeRef) -> u32 {
        self.catalog[t].oid.0
    }

    pub fn pseudo(&self, t: TypeRef) -> Option<&PseudoType> {
        match &self.catalog[t].kind {
            TypeKind::Pseudo(p) => Some(p),
            _ => None,
        }
    }

    pub fn is_unknown(&self, t: TypeRef) -> bool {
        matches!(self.pseudo(t), Some(PseudoType::Unknown))
    }

    pub fn is_polymorphic(&self, t: TypeRef) -> bool {
        self.pseudo(t).is_some_and(PseudoType::is_polymorphic)
    }

    /// `IsPolymorphicTypeFamily1`: the `any*` family without `anycompatible*`.
    pub fn is_polymorphic_family1(&self, t: TypeRef) -> bool {
        matches!(
            self.pseudo(t),
            Some(
                PseudoType::AnyElement
                    | PseudoType::AnyArray
                    | PseudoType::AnyNonArray
                    | PseudoType::AnyEnum
                    | PseudoType::AnyRange
                    | PseudoType::AnyMultirange
            )
        )
    }

    /// `getBaseType`: a domain's ultimate base type, otherwise the type.
    pub fn base_type(&self, mut t: TypeRef) -> TypeRef {
        loop {
            match &self.catalog[t].kind {
                TypeKind::Domain(d) => t = d.base,
                _ => return t,
            }
        }
    }

    /// `getBaseTypeAndTypmod`: base type and the typmod to use for it, which
    /// is the domain's own typmod when it has one.
    pub fn base_type_and_typmod(&self, mut t: TypeRef, mut typmod: Typmod) -> (TypeRef, Typmod) {
        loop {
            match &self.catalog[t].kind {
                TypeKind::Domain(d) => {
                    if !d.typmod.is_none() {
                        typmod = d.typmod.0;
                    }
                    t = d.base;
                }
                _ => return (t, typmod),
            }
        }
    }

    /// `get_element_type`: the element type of a true array type.
    pub fn element_type(&self, t: TypeRef) -> Option<TypeRef> {
        match &self.catalog[t].kind {
            TypeKind::Array { element } => Some(*element),
            _ => None,
        }
    }

    /// `get_base_element_type`: the element type, looking through domains.
    pub fn base_element_type(&self, t: TypeRef) -> Option<TypeRef> {
        self.element_type(self.base_type(t))
    }

    /// `type_is_array`.
    pub fn is_array(&self, t: TypeRef) -> bool {
        self.element_type(t).is_some()
    }

    /// `type_is_array_domain`.
    pub fn is_array_domain(&self, t: TypeRef) -> bool {
        self.base_element_type(t).is_some()
    }

    /// `get_array_type`: the array type over `t`.
    pub fn array_type(&self, t: TypeRef) -> Option<TypeRef> {
        self.catalog[t].array
    }

    pub fn is_enum(&self, t: TypeRef) -> bool {
        matches!(self.catalog[t].kind, TypeKind::Enum { .. })
    }

    pub fn is_range(&self, t: TypeRef) -> bool {
        matches!(self.catalog[t].kind, TypeKind::Range(_))
    }

    pub fn is_multirange(&self, t: TypeRef) -> bool {
        matches!(self.catalog[t].kind, TypeKind::Multirange { .. })
    }

    /// `get_range_subtype`.
    pub fn range_subtype(&self, t: TypeRef) -> Option<TypeRef> {
        match &self.catalog[t].kind {
            TypeKind::Range(r) => Some(r.subtype),
            _ => None,
        }
    }

    /// `get_multirange_range`.
    pub fn multirange_range(&self, t: TypeRef) -> Option<TypeRef> {
        match &self.catalog[t].kind {
            TypeKind::Multirange { range } => Some(*range),
            _ => None,
        }
    }

    /// `get_range_multirange`.
    pub fn range_multirange(&self, t: TypeRef) -> Option<TypeRef> {
        match &self.catalog[t].kind {
            TypeKind::Range(r) => Some(r.multirange),
            _ => None,
        }
    }

    /// `ISCOMPLEX`: a composite type.
    pub fn is_complex(&self, t: TypeRef) -> bool {
        matches!(self.catalog[t].kind, TypeKind::Composite(_))
    }

    pub fn is_record(&self, t: TypeRef) -> bool {
        matches!(self.pseudo(t), Some(PseudoType::Record))
    }

    pub fn is_record_array(&self, t: TypeRef) -> bool {
        matches!(self.pseudo(t), Some(PseudoType::RecordArray))
    }

    /// `is_complex_array`.
    pub fn is_complex_array(&self, t: TypeRef) -> bool {
        self.element_type(t).is_some_and(|e| self.is_complex(e))
    }

    pub fn category(&self, t: TypeRef) -> TypeCategory {
        self.catalog[t].category
    }

    /// `IsPreferredType(category, type)`.
    pub fn is_preferred(&self, category: TypeCategory, t: TypeRef) -> bool {
        let ty = &self.catalog[t];
        if ty.category == category {
            ty.is_preferred
        } else {
            false
        }
    }

    pub fn is_collatable(&self, t: TypeRef) -> bool {
        self.catalog[t].is_collatable
    }

    /// `format_type_be`: PostgreSQL's name for a type in a message.
    pub fn format(&self, t: TypeRef) -> String {
        self.format_with_typmod(t, None)
    }

    /// `format_type_extended` with an optional typmod.
    pub fn format_with_typmod(&self, t: TypeRef, typmod: Option<Typmod>) -> String {
        let (t, is_array) = match self.element_type(t) {
            Some(element)
                if !self.is(t, Builtin::Int2vector) && !self.is(t, Builtin::Oidvector) =>
            {
                (element, true)
            }
            _ => (t, false),
        };
        let with_typmod = typmod.is_some_and(|m| m >= 0);
        let m = typmod.unwrap_or(-1);
        let special = match self.catalog.as_builtin(t) {
            Some(Builtin::Bit) => Some(if with_typmod {
                format!("bit({m})")
            } else {
                "bit".to_owned()
            }),
            Some(Builtin::Bool) => Some("boolean".to_owned()),
            Some(Builtin::Bpchar) => Some(if with_typmod {
                format!("character({})", m - 4)
            } else {
                "character".to_owned()
            }),
            Some(Builtin::Float4) => Some("real".to_owned()),
            Some(Builtin::Float8) => Some("double precision".to_owned()),
            Some(Builtin::Int2) => Some("smallint".to_owned()),
            Some(Builtin::Int4) => Some("integer".to_owned()),
            Some(Builtin::Int8) => Some("bigint".to_owned()),
            Some(Builtin::Numeric) => Some(if with_typmod {
                numeric_typmod_out(m)
            } else {
                "numeric".to_owned()
            }),
            Some(Builtin::Interval) => Some(if with_typmod {
                interval_typmod_out(m)
            } else {
                "interval".to_owned()
            }),
            Some(Builtin::Time) => Some(if with_typmod {
                format!("time({m}) without time zone")
            } else {
                "time without time zone".to_owned()
            }),
            Some(Builtin::Timetz) => Some(if with_typmod {
                format!("time({m}) with time zone")
            } else {
                "time with time zone".to_owned()
            }),
            Some(Builtin::Timestamp) => Some(if with_typmod {
                format!("timestamp({m}) without time zone")
            } else {
                "timestamp without time zone".to_owned()
            }),
            Some(Builtin::Timestamptz) => Some(if with_typmod {
                format!("timestamp({m}) with time zone")
            } else {
                "timestamp with time zone".to_owned()
            }),
            Some(Builtin::Varbit) => Some(if with_typmod {
                format!("bit varying({m})")
            } else {
                "bit varying".to_owned()
            }),
            Some(Builtin::Varchar) => Some(if with_typmod {
                format!("character varying({})", m - 4)
            } else {
                "character varying".to_owned()
            }),
            Some(Builtin::Json) => Some("json".to_owned()),
            _ => None,
        };
        let mut buf = match special {
            Some(s) => s,
            None => {
                let ty = &self.catalog[t];
                let mut name = quote_identifier(&ty.name);
                if !self.type_is_visible(t) {
                    let schema = &self.catalog[ty.schema].name;
                    name = format!("{}.{}", quote_identifier(schema), name);
                }
                if with_typmod {
                    name = format!("{name}({m})");
                }
                name
            }
        };
        if is_array {
            buf.push_str("[]");
        }
        buf
    }

    /// `TypeIsVisible` for message formatting. A type is visible when its
    /// schema is in the metadata search path and no earlier schema has a
    /// type of the same name. `pg_catalog` is always searched first.
    fn type_is_visible(&self, t: TypeRef) -> bool {
        let ty = &self.catalog[t];
        let path = pg_catalog::SearchPath::from_metadata(self.catalog);
        for schema in path.schemas() {
            if *schema == ty.schema {
                return true;
            }
            let name = pg_catalog::QualifiedName::new(
                self.catalog[*schema].name.as_str(),
                ty.name.as_str(),
            );
            if self.catalog.type_named(&name).is_some() {
                return false;
            }
        }
        false
    }
}

/// `quote_identifier`: quote when the name is not a plain lower-case
/// identifier or is a reserved word. Reserved-word quoting is not modelled;
/// a built-in type name never needs it.
pub(crate) fn quote_identifier(name: &str) -> String {
    let plain = !name.is_empty()
        && name
            .bytes()
            .enumerate()
            .all(|(i, b)| b == b'_' || b.is_ascii_lowercase() || (i > 0 && b.is_ascii_digit()));
    if plain {
        name.to_owned()
    } else {
        format!("\"{}\"", name.replace('"', "\"\""))
    }
}

/// `numeric_typmod_out`: `numeric(precision,scale)`.
fn numeric_typmod_out(typmod: Typmod) -> String {
    let m = typmod - 4;
    let precision = (m >> 16) & 0xffff;
    let scale = ((m & 0x7ff) ^ 1024) - 1024;
    format!("numeric({precision},{scale})")
}

/// `intervaltypmodout`.
fn interval_typmod_out(typmod: Typmod) -> String {
    let fields = (typmod >> 16) & 0x7fff;
    let precision = typmod & 0xffff;
    // INTERVAL_MASK: MONTH=2, YEAR=4, DAY=8, HOUR=1024, MINUTE=2048, SECOND=4096.
    let fieldstr = match fields {
        0x0004 => " year",
        0x0002 => " month",
        0x0008 => " day",
        0x0400 => " hour",
        0x0800 => " minute",
        0x1000 => " second",
        0x0006 => " year to month",
        0x0408 => " day to hour",
        0x0c08 => " day to minute",
        0x1c08 => " day to second",
        0x0c00 => " hour to minute",
        0x1c00 => " hour to second",
        0x1800 => " minute to second",
        _ => "",
    };
    if precision != 0xffff {
        format!("interval{fieldstr}({precision})")
    } else {
        format!("interval{fieldstr}")
    }
}
