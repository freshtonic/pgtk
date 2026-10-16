# Model the catalog semantically, not as a row mirror

`pgtk-catalog` could expose each `pg_catalog` row as a Rust struct with every
column, or expose only what a consumer reasons about. We chose the semantic
model: a Type is an enum over base, composite, domain, enum, range,
multirange, array, and pseudo-type, and storage columns such as `typlen`,
`typbyval`, and `typalign` are dropped. Every entity keeps its OID. Raw row
structs stay private to the Loader. A row mirror would make every PostgreSQL
version difference a public API change and would push the interpretation of
each column onto every consumer.
