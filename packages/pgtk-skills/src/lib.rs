//! The agent skills of pgtk, as doctests.
//!
//! Each module below includes one `skills/<name>/SKILL.md` as its documentation. Rustdoc compiles
//! each Rust code block of a skill as a doctest, and runs each block that is not `no_run`. Thus
//! `cargo test -p pgtk-skills --doc` fails when a skill uses an API that does not exist any more.
//!
//! This crate is not published, and it has no other code. See `skills/README.md`.

#[doc = include_str!("../../../skills/pgtk-sql-check-migration/SKILL.md")]
pub mod pgtk_sql_check_migration {}

#[doc = include_str!("../../../skills/pgtk-sql-minimum-version/SKILL.md")]
pub mod pgtk_sql_minimum_version {}

#[doc = include_str!("../../../skills/pgtk-analyze-describe-query/SKILL.md")]
pub mod pgtk_analyze_describe_query {}

#[doc = include_str!("../../../skills/pgtk-analyze-check-queries/SKILL.md")]
pub mod pgtk_analyze_check_queries {}

#[doc = include_str!("../../../skills/pgtk-catalog-snapshot/SKILL.md")]
pub mod pgtk_catalog_snapshot {}
