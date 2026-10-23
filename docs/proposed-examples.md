# Proposed examples

Status: proposal. No example in this list is written yet.

Each published crate needs an `examples/` folder. People learn a crate mostly from its examples,
and so do LLMs. Each example must:

- solve one real task, which a user can copy and change;
- run with `cargo run -p <crate> --example <name>`, with no server where possible (use
  `pgtk_catalog::fixture::pg17()`);
- have a module comment that tells what it shows, and how to run it;
- compile in CI (`cargo build --examples`), so that it stays correct.

The examples marked ⭐ are the first ones to write.

## pgtk-sql

| Example | What it shows |
|---|---|
| ⭐ `parse` | Read a SQL file, parse each statement, and print its kind and its span. It also shows the rejection of SQL that PostgreSQL rejects, with the location. The smallest useful program. |
| ⭐ `version_check` | A CLI: `version_check --server 15 migrations/*.sql`. For each statement, print the oldest PostgreSQL version that accepts it, and the clause that needs that version. Exit with an error if a statement needs a newer server. Uses `minimum_version_above` and the lexical scan. |
| ⭐ `migration_lint` | A linter for migrations, written as a `Visit` implementation. Rules: `CREATE INDEX` without `CONCURRENTLY`, `ADD COLUMN ... NOT NULL` with no default, `DROP COLUMN`, `ALTER COLUMN ... TYPE`, `UPDATE` or `DELETE` with no `WHERE`. Shows that a lint is a `match` on typed nodes. |
| `rewrite` | A `VisitMut` that changes the SQL: rename a table, or add a schema to each table name. Then render the result with the formatter. |
| `format` | Pretty-print SQL with `PrettyConfig`: width, indentation, and the formatter fixtures as input. |
| `arbitrary_roundtrip` | Make random statements with `arbitrary`, render them, and parse them again. Shows that each value of the AST is valid SQL. The best demonstration of the AST design. |
| `flat_ast` | Parse into the flat AST, walk it with `FlatVisitor`, and compare the time with the nested AST. |
| `railroad` | Write the railroad diagram of one grammar rule to an SVG file. |

## pgtk-catalog

| Example | What it shows |
|---|---|
| `inspect` | Exists. Find each entity with a name: a type, a relation, the function overloads and the operator overloads. |
| ⭐ `describe_schema` | Load the catalog from a live server (`postgres` feature), and print the tables of a schema, with their columns, types, primary keys and parents. Like `\d` in psql. |
| ⭐ `snapshot` | Load a catalog from a server, and save it to a JSON file (`serde`). Later, load the file with no server. This is how a CI job checks queries without a database. |
| `async_load` | The same load with `tokio-postgres` and `load_async`. |
| `custom_executor` | Implement `Executor` for a driver that pgtk does not support. The example replays rows that it recorded, so it needs no server. |
| `resolve` | Resolve a function call or an operator on a search path, as PostgreSQL does, and print each candidate. |

## pgtk-analyze

| Example | What it shows |
|---|---|
| ⭐ `describe` | "Describe this query": print the name, type and source column of each result column, and the type of each parameter. Like `\gdesc` in psql, but with no server. |
| ⭐ `check_queries` | A CI check: analyse each `.sql` file of an application against a catalog snapshot (see `snapshot` above). Print each error as PostgreSQL prints it, with the SQLSTATE, the message, and a caret under the location. Exit with an error if a query fails. |
| `query_types` | Generate a Rust struct for the result of each query, from the Result Descriptor. A small version of what sqlc and the typed query macros do. |
| `target_version` | Analyse the same query for PostgreSQL 14 and for PostgreSQL 17. Show a query that only the newer server accepts. |
| `session` | `PREPARE`, then `EXECUTE`, and `DECLARE`, then `FETCH`: apply each Session Effect, and show how the Session changes the answer. |
| `annotations` | Walk the Annotation Tables: the type of each expression, the coercions, and the expansion of each `*`. For tools such as a language server. |
