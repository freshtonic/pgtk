# pg-catalog design decisions

Settled in the design session of 2026-09-15. Vocabulary is in `CONTEXT.md`;
the decisions that need a rationale are in `docs/adr/`. This file records the
rest so that they are not re-decided.

## Scope

- Entities: Schema, Type, Relation, Column, Function (with optional Aggregate
  detail), Operator, Cast, Extension.
- Relations: tables, views, materialized views, foreign tables, partitioned
  tables, sequences. A sequence is a Relation with its three columns
  (`last_value`, `log_cnt`, `is_called`), no row type, and no `pg_sequence`
  detail. Not indexes, TOAST, or composite-backing classes.
- Schemas: all except `pg_toast*` and the `pg_temp*` of other sessions;
  Entities inside those are skipped. The temporary schema of the loader's own
  session (`pg_my_temp_schema()`) is loaded and flagged `is_temporary`, so a
  consumer on the same connection can resolve a `CREATE TEMP TABLE`.
- Server versions: 14 through 17, one binary, version detected at load.
- Data source: `pg_catalog` only. `information_schema` is never read.
- Out of scope: see `CONTEXT.md`.

## Type model

- Kinds: base, composite, domain, enum, range, multirange, array, pseudo.
- Array detection: `typsubscript = 'array_subscript_handler'`. `typelem` alone
  is not sufficient (`name`, `point`, `lseg`, `box`, `line` have one).
- Every Type: name, Schema, category, is-preferred, collatable flag,
  `array: Option<Handle<Type>>`, OID, owning Extension.
- Array: element Handle. Domain: base, Type Modifier, NOT NULL, count of CHECK
  constraints. Enum: labels in `enumsortorder`. Range: subtype, multirange
  Handle. Multirange: range Handle. Composite: ordered Fields plus
  `Option<Handle<Relation>>` for row types.
- Pseudo-types: closed enum of the 26 in PostgreSQL 16/17 plus `Other(Name)`
  for future ones.
- Type category: closed enum over the documented `typcategory` letters plus
  `Other(char)`.
- Built-in Types: closed `Builtin` enum generated from `pg_type.dat`
  (fixed-OID types a checker can name, their arrays, all pseudo-types).
  Resolved to Handles at load; a missing one is a fatal load error.
- Type Modifier: `Typmod(i32)` with `decode(Builtin) -> Option<TypmodDetail>`
  for character, numeric, bit, time/timestamp precision, interval fields.

## Relations and Columns

- Relation: kind, name, Schema, `row_type: Option<Handle<Type>>` (`None` only
  for a sequence, whose `reltype` is 0), ordered Columns,
  `of_type: Option<Handle<Type>>` (`reloftype` of a typed table, for
  `typeIsOfTypedTable`; not the same type as `row_type`),
  `parents: Vec<Handle<Relation>>` (the direct `pg_inherits` parents in
  `inhseqno` order, for `typeInheritsFrom`; the consumer walks the chain), and
  `primary_key: Option<PrimaryKey { columns: Vec<i16>, deferrable }>`. The key
  columns are attribute positions (`conkey`) in key order. This is what
  `check_functional_grouping` reads. `PrimaryKey::grouping_columns` is `None`
  for a deferrable key, which PostgreSQL does not accept as proof. No other
  constraint is loaded; the check does not use a unique constraint.
- Column: name, position, Type, Type Modifier, NOT NULL, has-default,
  generated, `identity: Option<Identity { Always, ByDefault }>`. The kind is
  necessary because the rewriter rejects a non-DEFAULT value only for a
  `GENERATED ALWAYS` identity column (428C9). `generated` stays a flag:
  PostgreSQL 14 to 17 have only stored generated columns. Dropped attributes
  excluded. No default text.

## Functions, Operators, Casts

- Function: kind (function, procedure, aggregate, window), arguments
  `{name: Option<Name>, ty, mode: In|Out|InOut|Variadic|Table}` in one list,
  `variadic: Option<Handle<Type>>`, `default_count`, `default_types` (the
  `exprType()` of each default, read from the `proargdefaults` node tree; the
  expression itself is not kept), `returns`,
  `returns_set`, `strict`, volatility, optional Aggregate
  `{kind: Normal|OrderedSet|HypotheticalSet, direct_args}`. Procedures return
  `void`. No language, source, cost, security, parallel safety.
- Operator: kind (prefix, infix), `left: Option<Handle<Type>>`, right, result,
  Function, commutator, negator. No planner columns.
- Sort Operators: a Type holds `btree: Option<BtreeOperators { lt, le, eq, ge,
  gt }>` and `hash_equality: Option<Handle<Operator>>`, from the default btree
  and hash operator classes whose input type is that Type. These are what
  `get_sort_group_operators` reads, so a consumer can tell that `json` has no
  ordering operator and `xml` has no equality operator. The operator class
  itself is not an Entity (ADR 0001: a semantic model, not a row mirror). The
  lookup is exact-type: the domain, binary-coercible, and
  `anyarray`/`anyenum`/`anyrange`/`anymultirange`/`record` fallbacks of
  `GetDefaultOpClass`, and the element-type checks of the type cache, are the
  consumer's algorithm. Cross-type family members are not loaded.
- Cast: source, target, Cast Context, method
  `Function(Handle<Function>) | Binary | InOut`. Stored casts only; synthetic
  casts (I/O to text, domain/base, array element, composite) are the
  consumer's algorithm.

## Identity and lookups

- `Handle<T>`: `u32` + `PhantomData<fn() -> T>`; `Copy`, `Send`, `Sync`.
  Valid only for the Catalog that issued it; not checked at runtime.
- Arenas sorted by OID at load; iteration is OID order.
- Lookups: `Index<Handle<T>>`; `*_by_oid(Oid) -> Option<Handle<T>>`;
  `*_named(&QualifiedName)`; `*_visible(&SearchPath, &Name)` returning the
  first match for Types and Relations and all candidates in Search Path then
  OID order for Functions and Operators; `types()` etc. for iteration.
  Hash indexes built once at load.
- `SearchPath`: built from names against a Catalog; drops unknown Schemas;
  applies the implicit `pg_catalog` rule. Loader captures
  `current_schemas(true)` and returns it beside the Catalog.
- `Name`: newtype over `String`, byte equality, already folded.
- Catalog is immutable, `Send + Sync`, no interior mutability; callers `Arc`.

## Loader

- Sans-IO: `Loader::bootstrap_query()`, `Loader::new(version_row)?`,
  `queries() -> [(QueryId, &'static str)]`, `feed(QueryId, rows)`,
  `finish() -> (Catalog, Vec<Diagnostic>)`. Rows are
  `Vec<Vec<Option<String>>>`; every query casts columns to `text`.
- `Executor` trait, sync; async twin with native `async fn` behind a feature.
  Adapters: `postgres` and `tokio-postgres` behind features of the same name.
- Failure policy: fatal only for connection errors, unsupported version, a
  failed query, or a missing Built-in Type. Anything else is a Diagnostic
  (structured enum with `Display`), and the Entity is kept opaque or skipped.
- Extension ownership: `pg_depend` with `deptype = 'e'` only.
- Metadata: server version, database name, `current_schemas(true)`, load time
  as Unix seconds, Fixture format version.

## Fixture and tests

- `serde` feature gates all derives. `fixture` feature embeds a stock
  PostgreSQL 17 Catalog as zstd-compressed JSON via `include_bytes!`.
- Regenerated by an unpublished `xtask` against a PostgreSQL 17 server and
  committed. A test asserts the embedded format version matches the crate's.
- `CatalogBuilder` is the only constructor; Loader and tests both use it.
  Test Catalogs usually start from the Fixture and add Entities. No DSL yet.
- Loader integration tests read `PG_CATALOG_TEST_URL` and skip when unset.
  CI runs 14, 15, 16, 17 containers and asserts no unexpected Diagnostics.

## Crate

- Name `pg-catalog`, MIT, published to crates.io when stable. No dependency
  on `pg-sql` in either direction.
- Edition 2024, MSRV 1.88, `missing_docs = deny`, `unsafe_code = forbid`.
- Required dependencies: `thiserror` only. Optional: `serde`, `serde_json`,
  `zstd`, `postgres`, `tokio-postgres`. No `chrono`, no `tracing`.
