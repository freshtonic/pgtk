//! Casts: the conversions that the server stores.

use crate::extension::Extension;
use crate::function::Function;
use crate::handle::Handle;
use crate::oid::Oid;
use crate::types::Type;

/// A `pg_cast` row.
///
/// The model keeps the stored casts only. A synthetic cast, such as the
/// input and output cast to `text` or the cast between a domain and its
/// base, belongs to the algorithm of the consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cast {
    /// The server's identifier for this cast.
    pub oid: Oid,
    /// The type that the cast reads.
    pub source: Handle<Type>,
    /// The type that the cast produces.
    pub target: Handle<Type>,
    /// The context in which the server applies the cast.
    pub context: CastContext,
    /// How the server performs the cast.
    pub method: CastMethod,
    /// The extension that owns this cast, when one does.
    pub extension: Option<Handle<Extension>>,
}

/// The strength of a cast, from `castcontext`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum CastContext {
    /// The server applies the cast without a request. The letter is `i`.
    Implicit,
    /// The server applies the cast on assignment. The letter is `a`.
    Assignment,
    /// The server applies the cast only when the query writes it. The letter
    /// is `e`.
    Explicit,
}

/// How the server performs a cast, from `castmethod`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum CastMethod {
    /// The server calls this function. The letter is `f`.
    Function(Handle<Function>),
    /// The two types share a binary form. The letter is `b`.
    Binary,
    /// The server uses the output and input functions. The letter is `i`.
    InOut,
}
