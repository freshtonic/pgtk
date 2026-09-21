# Work in sibling crates that pg-analyze needs

Identified during the design session on 2026-09-20 and the Oracle runs after it.
Each item belongs to the named crate, not here.

## pg-sql

Done (pg-sql main 358f3e3): `COALESCE`, `NULLIF`, `GREATEST` and `LEAST` as
their own nodes; infix `OPERATOR(schema.op)`; the identifier folding helper;
`SELECT DISTINCT` requires a target list; `ROLLUP` and `CUBE` are grouping
clauses; `SELECT ALL` parses; a set operation chain is a left-recursive
`SelectClause` tree with gram.y's precedence; `UNBOUNDED PRECEDING` and
`UNBOUNDED FOLLOWING` are `WindowFrameBound::Unbounded`; `QueryBody.limit_locking`
holds the LIMIT and the locking clauses in either written order, with several
locking items, `FOR READ ONLY`, and qualified names in `OF`; `CREATE RULE ...
DO ALSO SELECT 1 UNION (SELECT 2)` parses; `BETWEEN SYMMETRIC` is
`BetweenLow.modifier`; `TABLESAMPLE` belongs to the relation forms only, with
a schema-qualified method; UPDATE, DELETE, MERGE and TABLE name their target
through one `RelationExpr`, with `ONLY (t)` and `t *`; a parenthesised
table reference holds a subquery or a join only (`ParenTableRef::Query` and
`ParenTableRef::Join`, with `JoinedTable` and `JoinChain`), so `FROM (t)` and
`FROM (t) x` are rejected as gram.y rejects them; `CALL` takes gram.y's
`func_application` (`CallStmt.call` is a `FunctionApplicationExpr`), so
qualified names, named arguments and `VARIADIC` parse.

Open:

- `ORDER BY x USING +` does not parse, while gram.y's `sortby` takes any
  `qual_all_Op`, `MathOp` included; PostgreSQL rejects it at analysis.
  Reported 2026-09-21.
- Promoting shared-pool types that carry Annotations (`SelectItem`,
  `QualifiedRef`, `PositionalParam`, `QueryBody`, `InsertStmt`, `UpdateStmt`)
  to typed pools is deferred until the Oracle harness gives a measurement,
  per pg-sql's own rule.

## pg-catalog

Done (pg-catalog main 1160b7d): sequences; `Relation.primary_key` with
deferrability; `Column.identity` as `Option<Identity>` (ALWAYS or BY DEFAULT);
`Relation.parents` from `pg_inherits`, for `typeInheritsFrom`;
`Function.default_types`, the type of each default expression, for the
polymorphic resolution of defaulted arguments; `Relation.of_type` for
`typeIsOfTypedTable`; `Relation.view_definition`, `Relation.rules` and
`Relation.triggers` for the Rewrite of a data-modifying Statement on a view.

Open:

- Constraints other than the primary key are absent. `ON CONFLICT ON
  CONSTRAINT name` looks the name up (`get_relation_constraint_attnos`), so
  that form is Unsupported until unique and exclusion constraints are in the
  Catalog.
- A rule's actions are not loaded, only its event and kind. Whether an
  INSTEAD rule returns rows is therefore unknown, so a data-modifying
  Statement with RETURNING against a relation with an INSTEAD rule is
  Unsupported.
- Operator classes are absent, so an `ON CONFLICT` target with an operator
  class or a `COLLATE` is Unsupported.
- Collations are absent entirely: no `pg_collation`, no column, type or range
  collation. Needed for `parse_collate` and every collation Diagnostic.
  Planned as a milestone of its own, in both crates together.
- `Column.generated` is a bool. PostgreSQL 18 adds virtual generated columns,
  so it becomes an enum then.

## recursa

- Nothing required. The arena AST cannot yield a span from a node reference,
  and that is why pg-analyze uses the flat AST (ADR 0001). Exposing views from
  the arena parse would be needed only if a consumer wanted to share
  Annotations with the arena form.
