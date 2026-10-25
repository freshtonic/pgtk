// The README examples need the `fixture` and `postgres` features, so other builds do not run them
// as doctests. The rendered documentation always includes the README.
#![cfg_attr(
    any(not(doctest), all(feature = "fixture", feature = "postgres")),
    doc = include_str!("../README.md")
)]
//!
//! # Shape of the model
//!
//! A [`Catalog`] holds one arena per kind of entity. A [`Handle`] is a typed
//! index into one arena. A handle is only valid for the catalog that made
//! it. Index a catalog with a handle to read the entity:
//!
//! ```
//! use pgtk_catalog::{Catalog, CatalogBuilder, Metadata, Name, Oid, Schema};
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
//! those drivers. The `fixture` feature gives a stock catalog of each
//! supported version, embedded in the crate: `fixture::pg14` to
//! `fixture::pg18`, and `fixture::pg19_beta` with the `pg19-beta` feature. A
//! test that needs no server starts there.
//!
//! # Server versions
//!
//! One build loads PostgreSQL 14, 15, 16, 17, and 18. The `pg19-beta`
//! feature adds 19 beta, because the catalog of 19 can still change before
//! 19.0. The feature is additive: it changes nothing for the other versions.
//! The loader reads the version of the server first, and
//! [`Metadata::server_version`] records it.
//!
//! The model is the union of the versions. A fact that only newer servers
//! have is an `Option`, a flag with the value that older servers imply, or
//! an enum variant, and its documentation names the first version that has
//! it. Examples are [`ConstraintKind::NotNull`] and [`Generated::Virtual`]
//! (PostgreSQL 18), and [`Builtin::Oid8`] (PostgreSQL 19). The public enums
//! are `#[non_exhaustive]`, so a new version can add a variant without a
//! breaking change. [`Builtin::since`] gives the first version of a built-in
//! type, and a catalog holds only the built-ins of its version.
//!
//! ## Support for a new major version
//!
//! Do these steps when PostgreSQL releases a new major version, or a beta of
//! it. ADR 0005 (`docs/adr/0005-one-build-for-every-server-version.md`) gives
//! the reasons.
//!
//! 1. Add the pin of the version to `pgtk-oracle/pins.tsv`, so that
//!    `pgtk-test-server` can start a server of it.
//! 2. Compare the catalog headers (`src/include/catalog/pg_*.h`) of the new
//!    version with the previous version, for each catalog that
//!    `loader/queries.rs` reads. Read the release notes too.
//! 3. For a new column that the model needs, give the query a text for the
//!    new versions (`//@sql <Name> <N>-`), and keep the columns of each text
//!    the same (see [`loader::queries`]). For a new letter, add an enum
//!    variant. Document the first version of each new fact.
//! 4. Raise the highest version in `loader/mod.rs`. For a beta, put it behind
//!    a `pg<N>-beta` feature, and add that feature to `pgtk-test-features`.
//! 5. Run `cargo xtask gen-builtins` with the `pg_type.dat` of each supported
//!    version (`<major>=<path>` for each). It records the first version of
//!    each built-in type.
//! 6. Add the version to `VERSIONS` in `tests/loader_live.rs` and run the
//!    live tests. A diagnostic or a failed test shows each difference that
//!    is still missing.
//! 7. Add a test for each new fact to `tests/loader_live.rs`.
//! 8. Write the fixture with `cargo xtask fixture <url>` against a new
//!    cluster (`initdb --locale=C -E UTF8`), and add its function to
//!    `fixture`. When the model changed, increase
//!    [`Catalog::FORMAT_VERSION`] and write each fixture again.
//! 9. Add the version to `scripts/check-queries.sh`, to the README, and to
//!    this section.
//!
//! At the release of 19.0, the `pg19-beta` feature goes away: 19 becomes an
//! ordinary supported version, and `fixture::pg19_beta` becomes
//! `fixture::pg19`.

#[cfg(any(feature = "postgres", feature = "tokio-postgres"))]
mod adapters;
mod builder;
mod builtin;
mod cast;
mod catalog;
mod collation;
mod constraint;
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
mod operator_class;
mod parameter;
mod relation;
mod rule;
mod schema;
mod search_path;
mod trigger;
mod types;
mod typmod;

pub use builder::{BuildError, CatalogBuilder};
pub use builtin::Builtin;
pub use cast::{Cast, CastContext, CastMethod};
pub use catalog::{Catalog, Metadata, ServerVersion};
pub use collation::{Collation, CollationProvider, Encoding};
pub use constraint::{Constraint, ConstraintKind};
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
pub use operator_class::OperatorClass;
pub use parameter::{Parameter, ParameterKind};
pub use relation::{Column, Generated, Identity, PrimaryKey, Relation, RelationKind};
pub use rule::{FiresWhen, Rule, RuleEvent};
pub use schema::Schema;
pub use search_path::SearchPath;
pub use trigger::{Trigger, TriggerTiming};
pub use types::{
    BtreeOperators, CompositeDetail, DomainDetail, Field, PseudoType, RangeDetail, Type,
    TypeCategory, TypeKind,
};
pub use typmod::{IntervalFields, Typmod, TypmodDetail};
