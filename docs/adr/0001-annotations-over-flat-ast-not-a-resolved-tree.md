---
status: accepted
---

# Analysis produces Annotation Tables keyed by flat-AST handles, not a resolved tree

PostgreSQL's `parse_analyze` builds a new `Query` tree. We do not. Analysis reads the `pg-sql` flat AST and produces one Annotation Table per kind of Annotation, each keyed by the flat AST's per-parse `Handle<T>`. Everything PostgreSQL would express as an invented node (expanded `*` columns, implicit casts, the range table, resjunk target entries) is an Annotation that refers to the syntax node it belongs to.

## Considered options

- **A resolved tree mirroring `Query`.** Rejected: it duplicates the parse tree, and a consumer that needs to rewrite SQL then has two trees to keep in step. The `Query` shape can be derived from the Annotation Tables when an Oracle comparison needs it.
- **Annotations keyed by node address on the arena AST**, as `sqltk` does. Rejected: the arena parse exposes no path from a node reference to its span, and zero-sized marker nodes make address identity unsafe. Both are fixable in `recursa`, but the flat AST already has stable handles, spans per node, and a side-table type.

## Consequences

- A consumer must parse with `pg-sql`'s flat entry points to use this crate.
- Annotations hold `pg-catalog` handles, so a result is meaningful only for the Catalog and the parse it came from.
