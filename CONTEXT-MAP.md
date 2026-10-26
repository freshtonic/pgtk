# Context Map

## Contexts

- [pgtk-sql](./packages/pgtk-sql/CONTEXT.md) — parses PostgreSQL SQL, and the psql client
  language, exactly as one PostgreSQL version does. `pgtk-psql` uses this glossary; it has no
  `CONTEXT.md` of its own.
- [pgtk-catalog](./packages/pgtk-catalog/CONTEXT.md) — models one PostgreSQL database's system
  catalog as an immutable snapshot.
- [pgtk-analyze](./packages/pgtk-analyze/CONTEXT.md) — performs PostgreSQL parse analysis on a
  parsed Statement against a Catalog.

The tooling crates (`pgtk-oracle`, `pgtk-psql-oracle`, `pgtk-test-server`, `pgtk-test-features`,
`pgtk-catalog-xtask`) have no domain language and no context.

## Relationships

- **pgtk-sql → pgtk-analyze**: Analysis reads a Statement that pgtk-sql parsed, and never changes
  it. pgtk-analyze uses pgtk-sql's terms for the parsed form. When pgtk-sql rejects a text, there
  is no Analysis: pgtk-analyze gives the Rejection to the caller with no change.
- **Explanation** has the same meaning in pgtk-sql (for a Rejection) and in pgtk-analyze (for a
  Diagnostic).
- **pgtk-catalog → pgtk-analyze**: Analysis reads a Catalog and never changes it. pgtk-analyze uses
  pgtk-catalog's definitions of Catalog and Search Path.

## Terms with a different meaning in each context

- **Target Version**: in pgtk-sql, the one PostgreSQL version that a build parses, fixed by a
  version feature. In pgtk-analyze, the version of the server a Statement will run on, chosen for
  each Analysis. pgtk-analyze calls pgtk-sql's Target Version the **Grammar Version**.
- **Diagnostic**: in pgtk-catalog, a note from the Loader about an Entity it could not fully
  represent. In pgtk-analyze, the error PostgreSQL would raise for a Statement.
