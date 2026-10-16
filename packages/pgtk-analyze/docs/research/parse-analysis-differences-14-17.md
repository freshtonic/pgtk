# Parse-analysis differences, PostgreSQL 14 -> 17

Audit of `src/backend/parser/analyze.c` and `parse_*.c` (excluding `parse_utilcmd.c`
and `parse_enr.c`) between the latest patch release of each major version.

Tags compared: `REL_14_24`, `REL_15_19`, `REL_16_15`, `REL_17_11`.
Source: `/Users/jamessadler/projects/pg-sql/vendor/postgres` (read only, no checkout).

**Method note.** Almost every bug fix in these files is back-patched. `git log A..B`
therefore lists ~80 commits per pair that are present in *both* tags. All findings below
come from `git diff`, not from the log; commit hashes are given for the master commit.

---

## Summary

| Pair | Behavioural diffs (non-feature) | New features reaching analysis | Diagnostic text changes | Catalog rows changed |
|---|---|---|---|---|
| 14 -> 15 | 2 | MERGE | 0 | cast -1, operator -6, proc +63/-21 |
| 15 -> 16 | 4 | SQL/JSON constructors, `IS JSON`, `XMLSERIALIZE INDENT`, optional subquery alias | 2 sites (rewritten hints) | cast 0, operator 0, proc +58/-5 |
| 16 -> 17 | 5 | JSON_TABLE, SQL/JSON query functions, `JSON()`/`JSON_SCALAR()`/`JSON_SERIALIZE()`, MERGE RETURNING / `MERGE_ACTION()` / NOT MATCHED BY SOURCE / MERGE into views | 3 (2 new, 1 removed) | cast 0, operator 0, proc +49/-16 |

**Headline:** the type-resolution core (`parse_coerce.c`, `parse_oper.c`, `parse_func.c`)
is effectively unchanged across all four versions. `parse_coerce.c` has a *zero*-behaviour
diff over 14 -> 17 apart from one added helper (`IsBinaryCoercibleWithCast`, 16). All of
`select_common_type`, `coerce_type`, `can_coerce_type`, `func_match_argtypes`,
`func_select_candidate`, `oper_select_candidate` are byte-identical in behaviour.

---

## 14 -> 15

### Behavioural differences

1. **`make_const()` gains a `T_Boolean` case** — `parse_node.c`,
   `941460fcf73 Add Boolean node` / `c4cc2850f4d Rename value node fields`.
   Example: `SELECT true`. In 14 the grammar emits `TypeCast(A_Const 't', bool)`; in 15 it
   emits a bare `A_Const` of node type `Boolean` and `make_const` returns a `Const` of
   `BOOLOID` directly. Resolved type is `boolean` in both. **Effect: node shape only,
   no SQL-visible change.** Also `A_Const.val.type == T_Null` becomes `A_Const.isnull`.

2. **`expandNSItemAttrs()` gains `require_col_privs`** — `parse_relation.c`,
   introduced with MERGE (`7103ebb7aae`). Only affects permission bitmaps, not resolution.

Everything else in the 14 -> 15 diff of these files is MERGE plumbing, `Value` ->
`String`/`Integer`/`Float`/`Boolean` node renaming, or comment/whitespace.

### Diagnostics

None changed. (The 15-only message `MERGE not supported in WITH query`, SQLSTATE `0A000`,
`parse_cte.c`, `92af9143f13` / `455d254d226`, is unreachable in 14 because MERGE does not
parse; see 16 -> 17 where it is removed again.)

### Features present from 15

- **MERGE** (`parse_merge.c`, new file, `7103ebb7aae`). Brings `EXPR_KIND_MERGE_WHEN` and
  five new rejections, all `parse_agg.c`/`parse_func.c`/`parse_relation.c`:
  - `aggregate functions are not allowed in MERGE WHEN conditions` (42803)
  - `grouping operations are not allowed in MERGE WHEN conditions` (42803)
  - `window functions are not allowed in MERGE WHEN conditions` (42803)
  - `set-returning functions are not allowed in MERGE WHEN conditions` (0A000)
  - `cannot use system column "%s" in MERGE WHEN condition` (42P10) — any system column
    except `tableoid`
  - `cannot execute MERGE on relation "%s"` + `errdetail_relkind_not_supported` (0A000)
  - `MERGE is not supported for relations with rules.` (0A000)
  - `unreachable WHEN clause specified after unconditional WHEN clause` (42601)
  - `name "%s" specified more than once` / `The name is used both as MERGE target table and data source.` (42712)
  - `WITH RECURSIVE is not supported for MERGE statement` (0A000)
- `UNIQUE NULLS [NOT] DISTINCT` — grammar + `parse_utilcmd.c` only, **out of scope** for
  these files.
- Sub-`WITH` in DML inside a CTE, CYCLE column self-reference, etc. are all back-patched
  to 14 and are **not** version differences.

### Catalog (summary)

- `pg_cast`: **1 row removed** — `path -> point` (explicit). `SELECT point(p)` /
  `CAST(p AS point)` for `path` fails from 15.
- `pg_operator`: **6 rows removed** — `@@ path`, `line <-> box`, `box <-> line`,
  `lseg ## line`, `line ## box`, plus the `^@` text operator gained an `oid_symbol`
  (no behaviour change). Example: `SELECT '((0,0),(1,1))'::line <-> '((0,0),(1,1))'::box`
  resolves in 14, `operator does not exist` in 15.
- `pg_proc`: +63 / -21. Notable new overloads affecting resolution: `regexp_count`,
  `regexp_instr`, `regexp_like`, `regexp_substr`, extra `regexp_replace(start, N, flags)`,
  `range_agg`/`multirange_agg`, `has_parameter_privilege`. Removed: `pg_start_backup`,
  `pg_stop_backup`, `pg_is_in_backup`, `pg_backup_start_time`, `point(path)`,
  `path_center`, `dist_lb`, `dist_bl`, `close_sl`, `close_lb`.

---

## 15 -> 16

### Behavioural differences

1. **Subquery alias in `FROM` becomes optional** — `parse_clause.c::transformRangeSubselect`,
   `bcedd8f5fce Make subquery aliases optional in the FROM clause.`
   `SELECT * FROM (SELECT 1)` — syntax error in 15, accepted in 16.
   The `elog(ERROR, "subquery in FROM must have an alias")` backstop is deleted.

2. **`FOR UPDATE OF <name>` no longer matches unaliased subquery/VALUES RTEs** —
   `analyze.c::transformLockingClause`, same commit. Any RTE with `alias == NULL` of kind
   `RTE_SUBQUERY` or `RTE_VALUES` is skipped, so it can no longer shadow a base relation of
   the same `eref` name. Reachable only in 16+ (15 required the alias).

3. **`make_const()` accepts non-decimal and underscore-separated integer literals** —
   `parse_node.c`, `6fcda9aba83 Non-decimal integer literals` +
   `faff8f8e47f Allow underscores in integer and numeric constants.`
   `SELECT 0x7fffffffffffffff` — syntax error in 15; in 16 resolves to `bigint`.
   `SELECT 1_000_000` likewise. Internally `strtoi64(..., 10)` became
   `pg_strtoint64_safe()`, which understands `0x`/`0o`/`0b` and `_`.

4. **Result column name for SQL/JSON constructors** — `parse_target.c::FigureColnameInternal`,
   `7081ac46ace`. Only matters from 16 because the constructs do not exist in 15.
   `SELECT JSON_OBJECT('a' VALUE 1)` names the column `json_object`.

Structural (not SQL-visible, but the reimplementation's output shape must match):

- **`Var.varnullingrels`** — `2489d76c490 Make Vars be outer-join-aware.` Every `Var` built
  by `scanNSItemForColumn`, `expandNSItemVars`, `transformWholeRowRef` and
  `buildVarFromNSColumn` now passes through the new `markNullableIfNeeded()`
  (`parse_relation.c`), and `transformFromClauseItem` maintains `pstate->p_nullingrels`
  via `markRelsAsNulledBy()`. The join RT index is now predicted *before*
  `addRangeTableEntryForJoin`.
- **`Query.rteperminfos` / `RTEPermissionInfo`** — `a61b1f74823 Rework query relation
  permission checking`. All `rte->requiredPerms`/`selectedCols`/`insertedCols`/`updatedCols`
  move out of `RangeTblEntry` into a parallel list. Touches `analyze.c`, `parse_clause.c`,
  `parse_relation.c`, `parse_target.c`, `parse_merge.c`.
- `EXPLAIN (GENERIC_PLAN)` sets up variable parameters in `analyze.c::transformExplainStmt`
  (`3c05284d83b`), so `$1` is accepted there from 16.

### Diagnostics

1. **`errorMissingColumn()` rewritten** — `parse_relation.c`,
   `56d0ed3b756 Give better hints for ambiguous or unreferenceable columns.`
   The "exists but not visible" case moves from `errhint` to `errdetail`, and gains two new
   hints. SQLSTATE stays `42703`, primary `errmsg` unchanged.
   - 15: `HINT: There is a column named "x" in table "t", but it cannot be referenced from this part of the query.`
   - 16: `DETAIL: There is a column named "x" in table "t", but it cannot be referenced from this part of the query.`
     plus `HINT: To reference that column, you must mark this subquery with LATERAL.`
     or `HINT: To reference that column, you must use a table-qualified name.`
   - New multi-match branch: `DETAIL: There are columns named "x", but they are in tables
     that cannot be referenced from this part of the query.` + `HINT: Try using a
     table-qualified name.`
   - Example: `SELECT (SELECT y) FROM (SELECT 1 AS y) s, LATERAL (SELECT 1) z` style
     queries; simplest: `SELECT * FROM t1, (SELECT t1.a) s;`
2. **`errorMissingRTE()`** — same commit. `HINT: There is an entry for table "t", ...`
   becomes `DETAIL: ...` plus an optional `HINT: To reference that table, you must mark
   this subquery with LATERAL.` SQLSTATE `42P01` unchanged.
3. `parseTypeString()`/`typeStringToTypeName()` lose `parser_errposition()` on
   `type "%s" does not exist` / `type "%s" is only a shell` (`858e776c84f`). Position was
   always `NULL` pstate, so no visible change; the `missing_ok` flag becomes an
   `ErrorSaveContext`.

### Features present from 16

- **SQL/JSON constructors**: `JSON_OBJECT()`, `JSON_ARRAY()`, `JSON_ARRAY(<query>)`,
  `JSON_OBJECTAGG()`, `JSON_ARRAYAGG()` — `parse_expr.c`, `7081ac46ace`, ~830 new lines
  (`transformJsonValueExpr`, `transformJsonOutput`, `coerceJsonFuncExpr`, …).
- **`IS JSON` predicate** (`IS [NOT] JSON [VALUE|SCALAR|ARRAY|OBJECT] [WITH|WITHOUT UNIQUE KEYS]`)
  — `6ee30209a6f`, `transformJsonIsPredicate`.
- **`XMLSERIALIZE(... INDENT)`** — `483bdb2afec`; `XmlExpr.indent` set in
  `transformXmlSerialize`.
- **Optional subquery alias** (above).
- Catalog-level: **`ANY_VALUE()`** aggregate, `system_user`, `random_normal()`,
  `date_add`/`date_subtract`, `array_sample`/`array_shuffle`, `erf`/`erfc`,
  `generate_series(timestamptz, timestamptz, interval, text)`,
  `pg_input_is_valid`/`pg_input_error_info`.

### Catalog (summary)

`pg_cast` and `pg_operator`: no data rows changed. `pg_proc`: +58 / -5 (the 5 removals are
signature replacements: `cume_dist`, `ntile`, `percent_rank`, `date_trunc`,
`pg_get_publication_tables`). No new implicit casts.

---

## 16 -> 17

### Behavioural differences

1. **MERGE is allowed in `WITH`** — `parse_cte.c`, `c649fa24a42 Add RETURNING support to MERGE.`
   The 15/16 rejection `MERGE not supported in WITH query` (0A000) is deleted and
   `MergeStmt` joins the DML list. `WITH m AS (MERGE INTO t USING s ON ... WHEN MATCHED
   THEN DELETE RETURNING t.id) SELECT * FROM m;` — error in 16, accepted in 17.

2. **MERGE target may be a view; the rules restriction is gone** — `parse_merge.c`,
   `5f2e179bd31 Support MERGE into updatable views.` `RELKIND_VIEW` is accepted, and the
   `cannot execute MERGE on relation "%s"` / `MERGE is not supported for relations with
   rules.` error is removed entirely.

3. **SQL/JSON constructor `RETURNING` coercion changes from explicit to assignment** —
   `parse_expr.c::coerceJsonFuncExpr`, `c2d93c3802b` / `2177306a641` (final form in 17).
   16 used `COERCION_EXPLICIT` + `COERCE_EXPLICIT_CAST`; 17 uses `COERCION_ASSIGNMENT` +
   `COERCE_IMPLICIT_CAST`. Effect: target types reachable only by an explicit cast are now
   rejected, and length/typmod coercion no longer raises on over-length values.
   Example: `SELECT JSON_OBJECT('a': 1 RETURNING char(2));` — 16 errors on truncation,
   17 silently pads/truncates per assignment semantics.

4. **`transformJsonOutput` rejects pseudo-types** — `parse_expr.c`, `6185c9737cf`.
   New error `returning pseudo-types is not supported in SQL/JSON functions` (0A000).
   Reachable for constructs that exist in both versions, e.g.
   `SELECT JSON_OBJECT('a': 1 RETURNING anyelement);`

5. **`RangeTblEntry.inh` is set uniformly** — `parse_relation.c`, `6d470211e54`. Assignment
   order changes only; the value is unchanged for every RTE kind.

Structural: `Query.mergeJoinCondition` is a new top-level field — in 17 the MERGE ON
condition no longer goes into `jointree->quals` (`makeFromExpr(p_joinlist, NULL)`).
`MergeWhenClause.matched` (bool) becomes `matchKind` (3-valued enum), and
`Query.mergeUseOuterJoin` disappears.

### Diagnostics

1. **New hint on UPDATE SET** — `analyze.c::transformUpdateTargetList`,
   `58447e31890 Add hint about not qualifying UPDATE...SET target with relation name.`
   `UPDATE t SET t.a = 1;` -> 42703 `column "t" of relation "t" does not exist` plus, from
   17, `HINT: SET target columns cannot be qualified with the relation name.`
2. **New error** `MERGE_ACTION() can only be used in the RETURNING list of a MERGE command`
   (42601), `parse_expr.c::transformMergeSupportFunc`.
3. **Removed** `MERGE is not supported for relations with rules.` and
   `MERGE not supported in WITH query` (see above).
4. Message text preserved despite refactoring: `NULLIF requires = operator to yield
   boolean` and `IS DISTINCT FROM requires = operator to yield boolean` became
   `"%s requires = operator to yield boolean"` with the construct name as an argument —
   **identical rendered text**. Same for `op_signature_string()` losing its `oprkind`
   argument (`parse_oper.c`): `operator does not exist: %s` renders the same.
5. `XMLTABLE`-specific message is generalised: `%s function has %d columns available but %d
   columns specified` now substitutes `JSON_TABLE` as well (`parse_relation.c`).

### Features present from 17

- **`JSON_TABLE(...)`** — new file `parse_jsontable.c` (543 lines, `de3600452b6`,
  `bb766cde63b` for `NESTED`), wired into `parse_clause.c::transformFromClauseItem` and
  `addRangeTableEntryForTableFunc` via the new `TableFuncType` (`TFT_XMLTABLE` /
  `TFT_JSON_TABLE`).
- **SQL/JSON query functions**: `JSON_EXISTS()`, `JSON_QUERY()`, `JSON_VALUE()` with
  `PASSING`, `ON ERROR`, `ON EMPTY` — `parse_expr.c::transformJsonFuncExpr` (`6185c9737cf`).
- **SQL/JSON constructors**: `JSON()`, `JSON_SCALAR()`, `JSON_SERIALIZE()` (`03734a7fed7`).
- **MERGE**: `RETURNING` (`c649fa24a42`, `EXPR_KIND_MERGE_RETURNING`), `MERGE_ACTION()`
  (`MergeSupportFunc`), `WHEN NOT MATCHED BY SOURCE` (`0294df2f1f8`), MERGE into updatable
  views (`5f2e179bd31`).
- Catalog-level: `random(min, max)` for int4/int8/numeric, `to_bin`/`to_oct`,
  `uuid_extract_timestamp`/`uuid_extract_version`, `xmltext`, `to_regtypemod`,
  `pg_basetype`, `unicode_assigned`.

### Catalog (summary)

`pg_cast` and `pg_operator`: no data rows changed. `pg_proc`: +49 / -16 (removals are
mostly `pg_stat_get_bgwriter_*`/`pg_stat_get_checkpoint_*` renames plus `interval_accum`
family replaced by `interval_avg_accum`). No new implicit casts.

---

## GUCs read during analysis

| GUC | Where | Effect |
|---|---|---|
| `transform_null_equals` | `parse_expr.c:937` (`transformAExprOp`), global `Transform_null_equals` | rewrites `expr = NULL` to `expr IS NULL`. Identical code in 14-17. |
| `search_path` | `parse_func.c` (`FuncnameGetCandidates`), `parse_oper.c` (`OpernameGetCandidates`, and cached in `OprCacheKey.search_path` via `fetch_search_path_array`), `parse_type.c` (`LookupTypeName`), `parse_relation.c` (`RangeVarGetRelid`) | which function/operator/type/relation an unqualified name resolves to. |
| `compute_query_id` | `analyze.c` (`IsQueryIdEnabled`) | query-ID jumbling only; no resolution effect. |

Nothing else in these files reads a GUC. `standard_conforming_strings`, `backslash_quote`,
`DateStyle` and `IntervalStyle` affect the scanner or type input functions, both outside
this scope (and outside the crate per ADR 0002).

---

## Assessment

**The version-dependent surface is small and almost entirely additive.** Counting only
differences that change how the *same* SQL text analyses against the *same* catalog:

- 14 -> 15: **2** (both node-shape, zero SQL-visible), plus the MERGE feature.
- 15 -> 16: **4** SQL-visible (optional alias, `FOR UPDATE OF` matching, non-decimal
  literals, JSON colname), **2** diagnostic-text sites, plus 4 features.
- 16 -> 17: **5** SQL-visible (MERGE in CTE, MERGE into views, JSON RETURNING coercion
  strength, pseudo-type rejection, `inh`), **3** diagnostic changes, plus 4 feature groups.

That is roughly **11 genuine behavioural differences across three major-version steps**,
against ~6,000 lines of analysis code per version. The dominant axis of variation is
*feature availability*, not *rule variation*: the resolution algorithms themselves
(`parse_coerce.c` above all) are frozen.

**Recommendation: a feature-availability table plus a very small set of rule switches.**
No difference requires a substantially different code path. Concretely:

```rust
struct VersionRules {
    // syntax/analysis gates
    subquery_alias_required: bool,            // true  <= 15
    non_decimal_int_literals: bool,           // true  >= 16
    underscore_in_numeric_literals: bool,     // true  >= 16  (same commit family)
    merge_allowed_in_cte: bool,               // true  >= 17
    merge_target_may_be_view: bool,           // true  >= 17
    merge_rejects_relations_with_rules: bool, // true  15..16
    locking_clause_skips_unaliased_subquery: bool, // true >= 16

    // coercion strength
    json_returning_coercion: CoercionContext, // Explicit in 16, Assignment in 17
    json_returning_rejects_pseudotypes: bool, // true >= 17

    // diagnostics
    missing_column_hint_style: HintStyle,     // Pre16 | Post16 (detail+LATERAL hints)
    missing_rte_hint_style: HintStyle,        // Pre16 | Post16
    update_set_qualified_hint: bool,          // true >= 17
}
```

Plus a feature table keyed on `(feature, min_version)`:
`MERGE` = 15, `IS JSON` = 16, JSON constructors (`JSON_OBJECT`/`JSON_ARRAY`/
`JSON_OBJECTAGG`/`JSON_ARRAYAGG`) = 16, `XMLSERIALIZE ... INDENT` = 16,
`JSON()`/`JSON_SCALAR()`/`JSON_SERIALIZE()` = 17, `JSON_EXISTS`/`JSON_QUERY`/`JSON_VALUE` = 17,
`JSON_TABLE` = 17, MERGE `RETURNING` / `MERGE_ACTION()` / `NOT MATCHED BY SOURCE` = 17.
`ANY_VALUE`, `UNIQUE NULLS NOT DISTINCT`, `system_user` and the new `random()` overloads
need **no** switch: `ANY_VALUE`/`system_user`/`random` are ordinary catalog rows that a
per-version catalog snapshot carries, and `UNIQUE NULLS [NOT] DISTINCT` lives in
`parse_utilcmd.c`, outside this crate.

**Caveats to keep in mind.**

1. **Output node shape varies more than behaviour does.** `Var.varnullingrels` (16+),
   `Query.rteperminfos` (16+), `Query.mergeJoinCondition` and `MergeAction.matchKind` (17)
   are real structural forks. If the annotations the crate emits are version-agnostic and
   the version only selects rules and a catalog, none of this matters. If the crate ever
   emits something isomorphic to PostgreSQL's `Query`, `varnullingrels` alone is a
   substantial per-version code path — it requires tracking `p_nullingrels` through
   `transformFromClauseItem` and predicting the join RT index. Keep the emitted form
   version-neutral and this cost disappears.
2. **Message text must be tabled, not computed.** The 16 `errorMissingColumn` rewrite is
   the only place where a per-version *branching structure* (not just a string) is needed;
   model it as two hint-selection strategies rather than sprinkling `if version >= 16`.
3. **Most of the per-version difference is catalog data, not code.** Six removed geometric
   operators and one removed cast in 15, and ~170 net new `pg_proc` rows across 15-17, do
   more to change analysis outcomes than all the parser code changes combined. A
   per-version catalog snapshot handles all of it for free. No new *implicit* casts were
   added in any of the three steps.

Single entry point per version is well justified: one `analyze(stmt, catalog, version)`
with a `VersionRules` lookup and a feature gate is enough. Do not fork the analyser.
