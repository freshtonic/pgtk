// The README examples need a grammar of PostgreSQL 17 or later, so other builds do not run them
// as doctests. The rendered documentation always includes the README.
#![cfg_attr(any(not(doctest), feature = "since-pg17"), doc = include_str!("../README.md"))]
//!
//! # Where to start
//!
//! The entry point is [`Analyzer`]. See `CONTEXT.md` for the language of this crate and
//! `docs/adr/` for the decisions behind it.

#![allow(clippy::too_many_arguments)]

mod analysis;
mod analyzer;
mod coerce;
mod consts;
mod diagnostic;
mod func;
mod oper;
mod relation;
mod session;
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
/// The Grammar Version of this build: the version of the `pgtk-sql` grammar
/// that parses a Statement. A Target Version must not be newer.
pub use pgtk_sql::TARGET_VERSION as GRAMMAR_VERSION;
pub use pgtk_sql::TargetVersion;
pub use recursa::Handle as NodeHandle;
pub use relation::{RangeTableEntry, Var};
pub use session::{Cursor, Placeholder, PreparedStatement, Session, SessionEffect};
pub use settings::{Settings, TargetVersionError};
pub use types::{TypeRef, Typmod};
pub use value::{Coercion, CoercionContext, CoercionPath};
