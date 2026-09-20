//! `pg-catalog` is a semantic Rust model of one PostgreSQL database's system
//! catalog: schemas, types, relations, columns, functions, operators, casts,
//! and extensions, loaded from a live server into an immutable [`Catalog`].
//!
//! The vocabulary is defined in `CONTEXT.md`; the design in `docs/design.md`
//! and `docs/adr/`; the implementation contract in `docs/spec.md`.
//!
//! # Shape of the model
//!
//! A [`Catalog`] holds one arena per kind of entity. A [`Handle`] is a typed
//! index into one arena. A handle is only valid for the catalog that made
//! it. Index a catalog with a handle to read the entity:
//!
//! ```
//! use pg_catalog::{Catalog, CatalogBuilder, Metadata, Name, Oid, Schema};
//!
//! let mut builder = CatalogBuilder::new(Metadata::placeholder());
//! builder.permit_missing_builtins();
//! let handle = builder.push_schema(Schema {
//!     oid: Oid(11),
//!     name: Name::new("pg_catalog"),
//!     is_temporary: false,
//!     extension: None,
//! });
//! let catalog: Catalog = builder.finish().unwrap();
//! assert_eq!(catalog[handle].name.as_str(), "pg_catalog");
//! ```
//!
//! An [`Oid`] is the identifier of the server. The model keeps it for
//! display and for correlation, and never uses it as its own identity.
//!
//! # Getting a catalog
//!
//! [`load`] reads a catalog from a live server through an [`Executor`]. The
//! `postgres` and `tokio-postgres` features give an executor for each of
//! those drivers. The `fixture` feature gives `fixture::pg17`, which is a
//! stock PostgreSQL 17 catalog embedded in the crate. A test that needs no
//! server starts there.

#[cfg(any(feature = "postgres", feature = "tokio-postgres"))]
mod adapters;
mod builder;
mod builtin;
mod cast;
mod catalog;
mod diagnostic;
mod error;
mod executor;
mod extension;
#[cfg(feature = "fixture")]
pub mod fixture;
mod function;
mod handle;
pub mod loader;
mod name;
mod oid;
mod operator;
mod relation;
mod schema;
mod search_path;
mod types;
mod typmod;

pub use builder::{BuildError, CatalogBuilder};
pub use builtin::Builtin;
pub use cast::{Cast, CastContext, CastMethod};
pub use catalog::{Catalog, Metadata, ServerVersion};
pub use diagnostic::{Diagnostic, EntityKind, SkipReason};
pub use error::LoadError;
#[cfg(feature = "async")]
pub use executor::{AsyncExecutor, load_async};
pub use executor::{Executor, load};
pub use extension::Extension;
pub use function::{
    AggregateDetail, AggregateKind, ArgMode, Argument, Function, FunctionKind, Volatility,
};
pub use handle::Handle;
pub use loader::{Loaded, Loader, QueryId, Rows};
pub use name::{Name, QualifiedName};
pub use oid::Oid;
pub use operator::{Operator, OperatorKind};
pub use relation::{Column, Relation, RelationKind};
pub use schema::Schema;
pub use search_path::SearchPath;
pub use types::{
    CompositeDetail, DomainDetail, Field, PseudoType, RangeDetail, Type, TypeCategory, TypeKind,
};
pub use typmod::{IntervalFields, Typmod, TypmodDetail};
