//! Adapters: the executors for the drivers that this crate knows.
//!
//! Each adapter holds one trait implementation and nothing else. A feature
//! turns each one on.

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "tokio-postgres")]
mod tokio_postgres;
