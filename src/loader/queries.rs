// PLACEHOLDER. Work item W1-queries replaces this file with the real SQL
// text for every query, and with a comment on each query that lists its
// column indexes. This file holds the public surface only, so that the rest
// of the crate compiles. Every `sql` call gives back `SELECT 1`.
//
// This file names nothing else in the crate, by the rule in section 10 of
// `docs/spec.md`.

//! The SQL text of each catalog query.

/// One catalog query.
///
/// The loader asks for the SQL of each query, runs it through an executor,
/// and feeds the rows back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryId {
    /// The server version number.
    Version,
    /// The name of the current database.
    Database,
    /// The schemas of the current search path.
    SearchPath,
    /// The `pg_namespace` rows.
    Schemas,
    /// The `pg_extension` rows.
    Extensions,
    /// The `pg_depend` rows that give extension ownership.
    ExtensionMembers,
    /// The `pg_type` rows.
    Types,
    /// The `pg_enum` rows.
    EnumLabels,
    /// The `pg_class` rows that are relations.
    Relations,
    /// The `pg_attribute` rows that are live columns.
    Attributes,
    /// The `pg_proc` rows.
    Functions,
    /// One row per function argument.
    FunctionArguments,
    /// The `pg_aggregate` rows.
    Aggregates,
    /// The `pg_operator` rows.
    Operators,
    /// The `pg_cast` rows.
    Casts,
}

impl QueryId {
    /// Every query, in a fixed order.
    pub const ALL: &'static [QueryId] = &[
        QueryId::Version,
        QueryId::Database,
        QueryId::SearchPath,
        QueryId::Schemas,
        QueryId::Extensions,
        QueryId::ExtensionMembers,
        QueryId::Types,
        QueryId::EnumLabels,
        QueryId::Relations,
        QueryId::Attributes,
        QueryId::Functions,
        QueryId::FunctionArguments,
        QueryId::Aggregates,
        QueryId::Operators,
        QueryId::Casts,
    ];
}

/// The SQL text of one query.
///
/// This placeholder gives back `SELECT 1` for every query.
pub fn sql(id: QueryId) -> &'static str {
    match id {
        QueryId::Version
        | QueryId::Database
        | QueryId::SearchPath
        | QueryId::Schemas
        | QueryId::Extensions
        | QueryId::ExtensionMembers
        | QueryId::Types
        | QueryId::EnumLabels
        | QueryId::Relations
        | QueryId::Attributes
        | QueryId::Functions
        | QueryId::FunctionArguments
        | QueryId::Aggregates
        | QueryId::Operators
        | QueryId::Casts => "SELECT 1",
    }
}

#[cfg(test)]
mod tests {
    use super::{QueryId, sql};

    #[test]
    fn every_query_has_sql() {
        assert_eq!(QueryId::ALL.len(), 15);
        for id in QueryId::ALL {
            assert!(!sql(*id).is_empty(), "query {id:?}");
        }
    }
}
