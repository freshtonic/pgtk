// PLACEHOLDER. Work item W2-loader adds `Loader`, `Rows`, `Loaded`, and the
// `parse` and `build` modules here. This file holds only the query surface,
// so that the rest of the crate compiles.

//! The loader: it turns query rows into a [`crate::Catalog`].

pub mod queries;

pub use queries::QueryId;
