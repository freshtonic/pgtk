//! PostgreSQL parse analysis over `pg-sql` statements and a `pg-catalog` Catalog.
//!
//! See `CONTEXT.md` for the language of this crate and `docs/adr/` for the
//! decisions behind it. The entry point is [`Analyzer`].

#![allow(clippy::too_many_arguments)]

mod analysis;
mod analyzer;
mod coerce;
mod consts;
mod diagnostic;
mod func;
mod oper;
mod relation;
mod settings;
mod typename;
mod types;
mod value;

pub use analysis::{
    Analysis, AnnotationTable, ExprType, FunctionResolution, ResultColumn, ResultDescriptor,
    TargetEntry,
};
pub use analyzer::{Analyzer, FlatAnalysis, Statement};
pub use diagnostic::{AnalyzeError, Diagnostic, Label, Severity, Unsupported};
pub use recursa::Handle as NodeHandle;
pub use relation::{RangeTableEntry, Var};
pub use settings::{Settings, TargetVersion};
pub use types::{TypeRef, Typmod};
pub use value::{Coercion, CoercionContext, CoercionPath};
