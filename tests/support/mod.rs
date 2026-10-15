//! Shared support for the Oracle harness.
//!
//! The harness replays PostgreSQL's regression corpus against a live server
//! (the Oracle) and against the crate (the Subject), and compares what each
//! says about every statement. `docs/oracle-harness.md` explains the design.

#![allow(dead_code)]

pub mod baseline;
pub mod cluster;
pub mod corpus;
pub mod oracle;
pub mod plan;
pub mod replay;
pub mod scheduler;
pub mod snapshots;
pub mod subject;
