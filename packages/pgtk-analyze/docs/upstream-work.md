# Work in sibling crates that pgtk-analyze needs

Identified during the design session on 2026-09-20 and the Oracle runs after it.
Each item belongs to the named crate, not here.

## pgtk-sql

Done (pgtk-sql main 80e7625): `COALESCE`, `NULLIF`, `GREATEST` and `LEAST` as
their own nodes; infix `OPERATOR(schema.op)`; the identifier folding helper;
`SELECT DISTINCT` requires a target list; `ROLLUP` and `CUBE` are grouping
clauses; `SELECT ALL` parses; a set operation chain is a left-recursive
`SelectClause` tree with gram.y's precedence; `UNBOUNDED PRECEDING` and
`UNBOUNDED FOLLOWING` are their own variants of the frame bound types; `QueryBody.limit_locking`
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
qualified names, named arguments and `VARIADIC` parse; `ORDER BY x USING op`
takes any `qual_all_Op` as one `SortOrder` with ASC/DESC, in a query, an
aggregate, WITHIN GROUP and a window; `COLLATE any_name` and the collation
and operator class names of index elements are `QualifiedName`; a CAST
takes no COLLATE; function table aliases follow gram.y's `TableFuncElement`
list, with a collation per column definition; the windowless positions that
took a type of their own now hold an `Expr`, so `FROM coalesce(a, b)`
and `SELECT coalesce(a, b)` build one node and one expression walker reads
both (`SpecialFuncTableRef.func`, `IndexTarget::Special` and
`RowsFromFunc::Special`, with `CommonSubexprCall` and `JsonFuncExpr` gone);
`TREAT(expr AS type)` is `Expr::Treat`; and MERGE's WHEN clauses take
gram.y's own match kinds and actions, so `WHEN NOT MATCHED BY SOURCE` is a
matched-side clause with a `MatchedKind`, an insert is `INSERT DEFAULT
VALUES` or one `VALUES` row with no `INTO`, and the forms gram.y rejects no
longer parse.

Open:

- `FROM current_user AS x (...)` and its family stay rejected on purpose.
  pgtk-sql lexes `CURRENT_DATE`, `CURRENT_USER`, `SESSION_USER`, `LOCALTIME`,
  `CURRENT_CATALOG` and `CURRENT_SCHEMA` as identifiers, so a windowless
  position reads one as a relation name, not as the special expression
  gram.y builds, and a column definition list after it does not parse.
  `USER` is the one hard keyword of the family, so `FROM USER` does parse.
  `JSON_OBJECTAGG` and `JSON_ARRAYAGG` stay rejected in these positions
  too: their nodes carry a `FILTER` and an `OVER` that a windowless
  position does not have, and either word could start an alias or an
  operator class. An aggregate there fails PostgreSQL's own parse analysis
  anyway.

- Promoting shared-pool types that carry Annotations (`SelectItem`,
  `QualifiedRef`, `PositionalParam`, `QueryBody`, `InsertStmt`, `UpdateStmt`)
  to typed pools is deferred until the Oracle harness gives a measurement,
  per pgtk-sql's own rule.

## pgtk-catalog

Done (pgtk-catalog main 3fa026f): sequences; `Relation.primary_key` with
deferrability; `Column.identity` as `Option<Identity>` (ALWAYS or BY DEFAULT);
`Relation.parents` from `pg_inherits`, for `typeInheritsFrom`;
`Function.default_types`, the type of each default expression, for the
polymorphic resolution of defaulted arguments; `Relation.of_type` for
`typeIsOfTypedTable`; `Relation.view_definition`, `Relation.rules` and
`Relation.triggers` for the Rewrite of a data-modifying Statement on a view;
`Catalog::parameter`, the configuration parameter names from `pg_settings`,
for the Result Descriptor of SHOW; collations: the `Collation` Entity with
`collation_named` and `collation_visible`, `Type.collation`,
`Column.collation` and the database encoding, for `parse_collate`;
`Relation.constraints` for ON CONFLICT ON CONSTRAINT; operator classes
(`operator_class_visible`) for the operator class of an ON CONFLICT target;
`Rule.definition` for RETURNING through a DO INSTEAD rule.

Open:

- `Column.generated` is a bool. PostgreSQL 18 adds virtual generated columns,
  so it becomes an enum then.

## recursa

- Nothing required. The arena AST cannot yield a span from a node reference,
  and that is why pgtk-analyze uses the flat AST (ADR 0001). Exposing views from
  the arena parse would be needed only if a consumer wanted to share
  Annotations with the arena form.
