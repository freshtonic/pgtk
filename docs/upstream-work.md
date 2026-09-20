# Work in sibling crates that pg-analyze needs

Identified during the design session on 2026-09-20. Each item belongs to the named crate, not here.

## pg-sql

- `COALESCE`, `NULLIF`, `GREATEST` and `LEAST` parse as ordinary function calls. PostgreSQL's raw tree has distinct nodes with their own typing rules, and `pg_catalog.coalesce(...)` written as a call is an error in PostgreSQL. Until fixed, pg-analyze reports Unsupported for these.
- Plain infix `a OPERATOR(schema.op) b` appears unmodelled in `Expr`. Only the quantified form (`op ANY (...)`) has it.
- An identifier folding helper: downcase unquoted identifiers, strip quotes, collapse `""`, handle `U&"..."`. pg-catalog's Names are already folded and its contract assigns folding to the parser. pg-analyze builds its symbol table from the folded form.
- Promoting shared-pool types that carry Annotations (`SelectItem`, `QualifiedRef`, `PositionalParam`, `QueryBody`, `InsertStmt`, `UpdateStmt`) to typed pools is deferred until the Oracle harness gives a measurement, per pg-sql's own rule.

## pg-catalog

- Sequences (`relkind = 'S'`) are not loaded, so `SELECT * FROM seq` cannot be analysed. Needed before milestone 1.
- Collations are absent entirely: no `pg_collation`, no column, type or range collation. Needed for `parse_collate` and every collation Diagnostic. Planned as a milestone of its own after DML lands, in both crates together.
- Function argument default expressions (`proargdefaults`) are not loaded. Needed only for the tree-level Oracle comparison, where PostgreSQL inserts them into the call.

## recursa

- Nothing required. The arena AST cannot yield a span from a node reference, and that is why pg-analyze uses the flat AST (ADR 0001). Exposing views from the arena parse would be needed only if a consumer wanted to share Annotations with the arena form.
