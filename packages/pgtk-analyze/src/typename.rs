//! `parse_type.c`: what a written type name means, and its typmod.

use pgtk_catalog::{Builtin, IntervalFields, QualifiedName, SearchPath};

use crate::diagnostic::{Diagnostic, Result};
use crate::types::{TypeRef, Types, Typmod};

/// A type name as written, folded: `schema.name`, modifiers, array bounds.
#[derive(Clone, Debug)]
pub(crate) struct TypeName {
    pub schema: Option<String>,
    pub name: String,
    /// Modifier values as written: integers, or identifiers for interval
    /// fields.
    pub typmods: Vec<TypmodArg>,
    /// Number of `[]` suffixes.
    pub array_dims: usize,
    /// Byte offset of the type name.
    pub location: usize,
}

/// One type modifier argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TypmodArg {
    Integer(i64),
    Ident(String),
}

impl TypeName {
    /// `TypeNameToString`.
    pub fn display(&self) -> String {
        let mut s = match &self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.clone(),
        };
        if self.array_dims > 0 {
            s.push_str("[]");
        }
        s
    }
}

/// Type-name resolution over one Catalog and Search Path.
pub(crate) struct TypeNames<'c> {
    pub types: Types<'c>,
    pub search_path: &'c SearchPath,
}

impl<'c> TypeNames<'c> {
    /// `typenameTypeIdAndMod`.
    pub fn resolve(&self, tn: &TypeName) -> Result<(TypeRef, Typmod)> {
        let catalog = self.types.catalog;
        let base = match &tn.schema {
            Some(schema) => {
                let real = crate::relation::resolve_schema_name(catalog, schema)
                    .filter(|real| catalog.schema_named(real).is_some());
                let Some(real) = real else {
                    return Err(Diagnostic::new(
                        "3F000",
                        format!("schema \"{schema}\" does not exist"),
                    )
                    .at(tn.location)
                    .into());
                };
                catalog.type_named(&QualifiedName::new(real, tn.name.as_str()))
            }
            None => catalog.type_visible(self.search_path, &tn.name),
        };
        let mut ty = base;
        if tn.array_dims > 0
            && let Some(t) = ty
        {
            ty = self.types.array_type(t);
        }
        let Some(ty) = ty else {
            return Err(Diagnostic::new(
                "42704",
                format!("type \"{}\" does not exist", tn.display()),
            )
            .at(tn.location)
            .into());
        };
        let typmod = self.typmod(tn, ty)?;
        Ok((ty, typmod))
    }

    /// `typenameTypeMod`: the typmod for the written modifiers, for the
    /// built-in types that accept them.
    fn typmod(&self, tn: &TypeName, ty: TypeRef) -> Result<Typmod> {
        if tn.typmods.is_empty() {
            return Ok(-1);
        }
        let types = &self.types;
        let element = types.element_type(ty).unwrap_or(ty);
        let builtin = types.catalog.as_builtin(element);
        let too_many = || Diagnostic::new("22023", "invalid type modifier").at(tn.location);
        let int_arg = |i: usize| -> Result<i64> {
            match tn.typmods.get(i) {
                Some(TypmodArg::Integer(v)) => Ok(*v),
                _ => Err(Diagnostic::new("22023", "invalid type modifier")
                    .at(tn.location)
                    .into()),
            }
        };
        let m = match builtin {
            Some(Builtin::Bpchar | Builtin::Varchar) => {
                if tn.typmods.len() != 1 {
                    return Err(too_many().into());
                }
                let len = int_arg(0)?;
                let name = if builtin == Some(Builtin::Bpchar) {
                    "char"
                } else {
                    "varchar"
                };
                if len < 1 {
                    return Err(Diagnostic::new(
                        "22023",
                        format!("length for type {name} must be at least 1"),
                    )
                    .at(tn.location)
                    .into());
                }
                if len > 10485760 {
                    return Err(Diagnostic::new(
                        "22023",
                        format!("length for type {name} cannot exceed {}", 10485760),
                    )
                    .at(tn.location)
                    .into());
                }
                len as i32 + 4
            }
            Some(Builtin::Numeric) => {
                if tn.typmods.is_empty() || tn.typmods.len() > 2 {
                    return Err(Diagnostic::new("22023", "invalid NUMERIC type modifier")
                        .at(tn.location)
                        .into());
                }
                let precision = int_arg(0)?;
                let scale = if tn.typmods.len() == 2 {
                    int_arg(1)?
                } else {
                    0
                };
                if !(1..=1000).contains(&precision) {
                    return Err(Diagnostic::new(
                        "22023",
                        format!(
                            "NUMERIC precision {precision} must be between 1 and {}",
                            1000
                        ),
                    )
                    .at(tn.location)
                    .into());
                }
                if !(-1000..=1000).contains(&scale) {
                    return Err(Diagnostic::new(
                        "22023",
                        format!(
                            "NUMERIC scale {scale} must be between {} and {}",
                            -1000, 1000
                        ),
                    )
                    .at(tn.location)
                    .into());
                }
                ((precision as i32) << 16 | ((scale as i32) & 0x7ff)) + 4
            }
            Some(Builtin::Bit | Builtin::Varbit) => {
                if tn.typmods.len() != 1 {
                    return Err(too_many().into());
                }
                let len = int_arg(0)?;
                let name = if builtin == Some(Builtin::Bit) {
                    "bit"
                } else {
                    "bit varying"
                };
                if len < 1 {
                    return Err(Diagnostic::new(
                        "22023",
                        format!("length for type {name} must be at least 1"),
                    )
                    .at(tn.location)
                    .into());
                }
                if len > 83886080 {
                    return Err(Diagnostic::new(
                        "22023",
                        format!("length for type {name} cannot exceed {}", 83886080),
                    )
                    .at(tn.location)
                    .into());
                }
                len as i32
            }
            Some(Builtin::Time | Builtin::Timetz | Builtin::Timestamp | Builtin::Timestamptz) => {
                if tn.typmods.len() != 1 {
                    return Err(too_many().into());
                }
                let p = int_arg(0)?;
                let name = match builtin {
                    Some(Builtin::Time) => "TIME",
                    Some(Builtin::Timetz) => "TIME(%d) WITH TIME ZONE",
                    Some(Builtin::Timestamp) => "TIMESTAMP",
                    _ => "TIMESTAMP(%d) WITH TIME ZONE",
                };
                if p < 0 {
                    let msg = match builtin {
                        Some(Builtin::Time) => format!("TIME({p}) precision must not be negative"),
                        Some(Builtin::Timetz) => {
                            format!("TIME({p}) WITH TIME ZONE precision must not be negative")
                        }
                        Some(Builtin::Timestamp) => {
                            format!("TIMESTAMP({p}) precision must not be negative")
                        }
                        _ => {
                            format!("TIMESTAMP({p}) WITH TIME ZONE precision must not be negative")
                        }
                    };
                    return Err(Diagnostic::new("22023", msg).at(tn.location).into());
                }
                let _ = name;
                // Precision above 6 is reduced to 6 with a WARNING.
                p.min(6) as i32
            }
            Some(Builtin::Interval) => {
                // gram.y encodes fields as an integer typmod and precision as
                // a second; the walker passes them in that order.
                let fields = match tn.typmods.first() {
                    Some(TypmodArg::Integer(v)) => *v as i32,
                    _ => IntervalFields::FULL_RANGE,
                };
                let precision = match tn.typmods.get(1) {
                    Some(TypmodArg::Integer(p)) => {
                        if *p < 0 {
                            return Err(Diagnostic::new(
                                "22023",
                                format!("INTERVAL({p}) precision must not be negative"),
                            )
                            .at(tn.location)
                            .into());
                        }
                        (*p).min(6) as i32
                    }
                    _ => 0xffff,
                };
                (fields << 16) | precision
            }
            _ => {
                return Err(Diagnostic::new(
                    "42601",
                    format!("type modifier is not allowed for type \"{}\"", tn.display()),
                )
                .at(tn.location)
                .into());
            }
        };
        Ok(m)
    }
}

/// `INTERVAL_FULL_RANGE`.
trait FullRange {
    const FULL_RANGE: i32;
}

impl FullRange for IntervalFields {
    const FULL_RANGE: i32 = 0x7fff;
}
