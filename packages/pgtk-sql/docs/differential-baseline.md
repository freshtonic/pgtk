# Differential baseline: PostgreSQL 17.9 corpus, one oracle per target version

`baselines/postgresql-17.9.json` is the named, machine-readable floor for
PostgreSQL statement parity. It records the immutable legacy Git identities,
the PostgreSQL 17.9 gitlink, all 226 regression SQL files, the legacy suite's
222 included files and four explained exclusions, and per-file statement and
pass/skip/fail counts.

## Frozen corpus and pinned oracle

The corpus and the oracle are two different inputs:

- The **corpus** is frozen at PostgreSQL 17.9 (`6d396980fc5`). It is the set
  of regression SQL files that the legacy parser split into the frozen
  statement spans. The tests read each file by its Git blob ID from the
  `vendor/postgres` object database, not from the checked-out tree. The
  baseline file names and their `postgres` fields name this corpus.
- The **oracle** is the raw parser of the pinned PostgreSQL release of the
  target version. `pgtk-oracle/pins.tsv` is the one pin table. For `pg17`, the
  pin is PostgreSQL 17.11 (`REL_17_11`, `083ac033419`), and the
  `vendor/postgres` submodule gitlink must be the same commit (a pgtk-oracle
  test checks this). `pgtk-oracle` extracts the pinned commit with
  `git archive` and builds it again when the pin moves.

Thus a move of the pin changes only the oracle. The statement spans, the
legacy item kinds and the per-file counts stay valid, so each outcome change
has one cause: a change in the raw parser. The submodule must contain the
history back to 17.9, because the tests read the frozen blobs from it. Its
object store must also contain the pin of each target version
(`scripts/fetch-postgres-pins`).

### Pin move from 17.9 to 17.11 (#71)

Between `REL_17_9` and `REL_17_11`, `src/backend/parser/gram.y`, `scan.l`,
`parser.c` and `src/include/parser/kwlist.h` did not change. The only change in
the oracle sources is in node support: `CreateStatsStmt` has a new `owner`
field (commit `75a03c569c7`) and `makeJsonIsPredicate` has a new assertion.
Neither changes which statements the raw parser accepts. The differential
suite has the same result at 17.11 as at 17.9: 222 of 222 corpus files pass,
with no outcome change for any statement.

The 17.11 regression files change 40 corpus files. A simple split of these
files gives 345 statements that are not in the frozen corpus. They are not in
the baseline. A one-time probe of these statements with the differential check
and the 17.11 oracle found no failure.

### Soft errors in the oracle (#80)

`pgtk-oracle` made `errsave_start` an `errstart(ERROR, ...)` and ignored its
context, so every soft error became a hard error. From PostgreSQL 16 the raw
parser uses the soft-error API: `process_integer_literal` in `scan.l` calls
`pg_strtoint32_safe` with an `ErrorSaveContext`, and a literal that does not
fit in `int32` becomes an `FCONST`. The oracle thus rejected statements that
PostgreSQL accepts, such as `SELECT 2147483648`. The stub now follows
`elog.c`: with an `ErrorSaveContext` it records the error and returns false,
and with no context the error stays hard.

The `pg16`, `pg17`, `pg18` and `pg19-beta` baselines each record an accept
for the same 131 statements that they recorded as a reject. All 131 hold an
integer literal above `INT32_MAX`, in decimal (`int8.sql`, `numeric.sql`,
`random.sql`), in a non-decimal form (`numerology.sql`) or with `_`
separators (`partition_prune.sql`). Each is a legacy statement item, so the
`pass` and `skip` totals do not move, and pgtk-sql agrees with the oracle on
every one, so no version gap is added.

`pg14` and `pg15` do not change. Their `process_integer_literal` reaches the
same `FCONST` fallback through `strtoint` and `errno`, with no soft-error
context, so the defect never touched them.

Only the token changes, not the grammar rules that take it. The token is an
`FCONST`, so a rule that takes only an `Iconst` still rejects it: at 14
`createdb_opt_item` takes a `SignedIconst` and rejects
`CREATE DATABASE d OID = 3000000000`, while 15 and later take a `NumericOnly`
and accept it. `pgtk-oracle/tests/soft_errors.rs` holds these cases.

pgtk-sql makes the same split (issue #88). Its `NumericLit` pattern for each
target version also takes an integer above `INT32_MAX`, in each literal form
of that version, and wins the tie with `IntegerLit`. So each `Iconst` and
`SignedIconst` position rejects such a literal, for example
`CREATE ROLE r SYSID 3000000000`, and each `NumericOnly`, `FCONST` or
expression position accepts it. No frozen corpus statement has the rejected
shape, so the differential check cannot see it. The `strict_statement` tests
check each position against the oracle, at the edges of the `int32` range and
in each literal form (issue #96).

## One baseline for each target version

Each target version (ADR 0009) has its own version baseline in
`baselines/target-versions/<feature>.json`, for example `pg16.json`. All six
use the same frozen 17.9 corpus. The oracle is the raw parser of the pinned
release of that version, from `pgtk-oracle/pins.tsv`:

| Feature | Oracle | Expected version gaps |
|---|---|---|
| `pg14` | `REL_14_24` | 0 |
| `pg15` | `REL_15_19` | 0 |
| `pg16` | `REL_16_15` | 0 |
| `pg17` | `REL_17_11` | 0 |
| `pg18` | `REL_18_6` | 0 |
| `pg19-beta` | `REL_19_STABLE` at `b73d13c` | 0 |

A version baseline records:

- the oracle ref and commit, which must equal the pin of the version;
- the corpus name and gitlink (`postgresql-17.9`);
- for each corpus file, the oracle outcome of each statement: `A` (accept) or
  `R` (reject), in statement order;
- the totals, and the expected version gaps.

An **expected version gap** is a statement where a build of the version does
not pass the differential check with that version's oracle. Each gap records
the statement identity (file, zero-based statement index and byte range), the
pgtk-sql outcome and the oracle outcome:

- `"pg_sql": "skip"` with `"oracle": "accept"`: the oracle accepts the
  statement, and pgtk-sql cannot parse it.
- `"pg_sql": "fail"`: pgtk-sql over-accepts the statement, or its output
  changes the parse tree, or the oracle rejects its output.

A gap exists when the grammar of a version is not complete. The `pg14`
list had 53 gaps before #76: the trailing junk after numbers that 14 accepts
(`numerology.sql`), which include the non-decimal and `_` forms. 14 lexes
`0x42F` as `0 AS x42F`, and a `pg14` build had the 15 lexing, which rejects
it. The `pg14` build now lexes numbers as the 14 scanner does. The grammars
of `pg14` (#76), `pg15` (#75) and `pg16` (#74) are complete, so their lists
are empty.

An empty list does not show that a version is complete: the check below
cannot find newer syntax that a build accepts. Explicit negative tests cover
that (`14-14`, `14-15` and `14-16` in `embedded-tests/inventory.tsv`). The keywords
that a version does not have are gated with their `lookahead` and
`precedence` entries, so a build accepts them where gram.y takes a bare
`IDENT`: `format`, `json`, `keys` and `scalar` before 16, and `path` and
`nested` before 17.

**Each version increment (#74 to #78) must drain the list of its version to
empty.** The `pg17` list is empty, and the `pg17` baseline must give the frozen
legacy outcome counts of `baselines/postgresql-17.9.json` (the fast test checks
this).

Before #95 the differential check found an over-acceptance only when the
oracle accepted the output of pgtk-sql. A statement that a version rejects is
normally rejected again after the format step, so an older version's build
that accepted newer syntax (for example `MERGE` in `pg14`) had no gap in this
list. The check is now strict (see below), so such a statement fails. The
version increments also have explicit negative tests for each "removed in N"
and "added in N" change (decision 11 of
`docs/plans/2026-09-22-target-versions.md`).

### A statement that the oracle rejects (#95)

Version parity says that a build rejects what the raw parser of its target
version rejects. So when the oracle rejects a statement, pgtk-sql must reject
it too, and an accept is a failure, whatever the rendering is:

- "over-permissive": the oracle accepts the rendering of pgtk-sql. pgtk-sql
  changed the meaning of the statement. This is the more specific failure.
- "over-acceptance": the oracle also rejects the rendering.

Before #95 the check failed only the first case. An over-accepted statement
usually keeps its defect in the rendering, so the check did not find it. PRs
#72 to #90 fixed about 20 such statements of the corpus because the fuzz
round trip found them. The strict rule found 23 more on `pg17`, in six
causes. The fixes of #95 remove all of them:

| Cause | Statements | Rule | Fix |
|---|---|---|---|
| An incorrect Unicode escape in `U&'...'` | 10 (`strings.sql`) | parser.c `str_udeescape` | validation hook on `UnicodeStringLitWithEscape` |
| An incorrect `\u` or `\U` escape in `E'...'` | 8 (`strings.sql`) | scan.l `<xe>{xeunicode}`, `{xeunicodefail}`, `addunicode` | validation hook on the `E'...'` arms of `Sconst` and `Expr` |
| A zero-length delimited identifier `""` | 1 (`create_am.sql`) | scan.l `<xd>{xdstop}` | the identifier patterns need one character |
| An argument `ORDER BY` with `WITHIN GROUP` | 1 (`aggregates.sql`) | gram.y `func_expr` | validation hook on `FunctionCallSuffix` |
| `WITH TIES` with `SKIP LOCKED` | 1 (`limit.sql`) | gram.y `insertSelectOptions` | the `QueryBody` validation hook |
| `CURRENT_SCHEMA` as a table name | 2 (`publication.sql`) | kwlist.h, gram.y `qualified_name` | declared rejection on `QualifiedName` |

Each rejection comes from the raw parser (`scan.l`, `parser.c` or a `gram.y`
action), not from a soft error of the oracle. So no version baseline records
one of them as a gap.

`baselines/pending-over-acceptance.json` is the temporary list of the
over-acceptances that are not fixed. Each entry has the statement identity
and a named cause. The differential test expects the "over-acceptance"
failure for an entry where the oracle of the target version rejects the
statement. So a new over-acceptance fails, and an entry that pgtk-sql now
rejects fails until its entry is removed. The list is empty. A statement that
the oracle rejects for a reason outside the raw parser goes in the version
baseline as a `"fail"` gap with the oracle outcome `"reject"`, with its reason
in this document.

The rule needs the other target versions too. Their oracles reject statements
that the `pg17` oracle accepts: 1009 for `pg14`, 616 for `pg15` and 369 for
`pg16` (none for `pg18` and `pg19-beta`). Before #95 a build of an older
version could accept such a statement and pass. Now it fails unless the
version gates of the grammar refuse it.

### How the tests use the version baseline

The differential test reads the baseline of the build's target version. For
each corpus file, it fails if:

- the live oracle outcome of a statement differs from the recorded outcome;
- a listed gap now agrees with the oracle, or has a different outcome;
- a statement that is not listed disagrees with the oracle, also a statement
  that the oracle rejects and pgtk-sql accepts; or
- a pending over-acceptance changed (`baselines/pending-over-acceptance.json`).

Thus the list cannot become stale in either direction. The accepted legacy
gap test keeps the exact 17 contract for `pg17`. For another version, it skips
the ledger statements that the version rejects or that are expected gaps. A
legacy parse error that only that version accepts must pass or be an expected
gap.

The fast test (`differential_baseline`) checks all six files without an
oracle: the pins, the corpus identity, the outcome counts and the order and
ranges of the gaps.

### Regenerate a version baseline

Regenerate the file after a change of pin, of oracle, or of the grammar of the
version, and review the diff. The ignored test writes the file of the build's
target version and prints the reason of each gap:

```sh
cargo test -p pgtk-sql --no-default-features --features pg16,spans,postgres-oracle \
  --test differential -- --ignored regenerate_version_baseline --nocapture
```

The first oracle build of each version extracts the pinned commit from the
`vendor/postgres` object store and builds PostgreSQL under Cargo's `OUT_DIR`.
Fetch the pins first with `scripts/fetch-postgres-pins`.

## The psql oracle

`docs/psql-oracle.md` describes the second differential: `pgtk-psql-oracle`
links `psqlscan.l` and `psqlscanslash.l` of the same pinned release, and
`pgtk-psql/tests/psql_oracle.rs` compares pgtk-psql's lexing with psql's own. It
reads the same frozen corpus, by Git blob ID, and pins its outcomes per
target version in `baselines/psql-oracle/<feature>.json`.

## Frozen corpus rules

The inclusion rule is the exact `corpus_tests!` declaration at legacy commit
`1e71421d66baac15c8c5264e8f29b5f80122f50e`. The only excluded files are the
four `collate.*` fixtures whose dotted names could not become macro-generated
Rust test identifiers. Discovery fails if a future valid identifier is absent
from the declaration, so an undeclared fixture cannot silently become a new
exclusion.

The frozen statement rules are:

- psql backslash directives and COPY-from-stdin payload regions are non-SQL;
- statements containing psql variable interpolation are not standalone SQL;
- a whole-file legacy parse failure yields no extracted statements; and
- a PostgreSQL-accepted statement that the legacy parser cannot model is a
  skip, while formatter/tree differences and over-permissive parses are fails.

The 18 captured skips are frozen by fixture: `amutils` 1, `create_index` 1,
`create_view` 2, `join` 6, `returning` 2, `rules` 1, `select` 1, and `with` 4.
Every other included fixture has zero skips.

`baselines/postgresql-17.9-accepted-legacy-gaps.json` assigns those aggregate
skips stable statement identities: fixture, zero-based statement index, and
absolute source byte range. It also freezes their current strict-statement
outcome so that a resolved legacy gap cannot silently regress. Seventeen now
pass the generated parser and formatter comparison. The remaining permanent
gap is `join.sql` statement 171 (`16727:16787`), whose strict parse diagnostic
is `RCA4100` with statement-relative region and primary anchor `29:31`.

The fast baseline test validates the ledger's provenance, ordering, unique
identities, ranges, legacy parse-error classification, and per-file counts
against both pinned baseline artifacts. The Linux differential test then
re-derives PostgreSQL acceptance and compares every current outcome, including
the permanent gap's exact diagnostic contract:

```sh
cargo test --locked -p pgtk-sql --test differential_baseline
cargo test --locked -p pgtk-sql --features postgres-oracle --test accepted_legacy_gaps frozen_accepted_legacy_gap_contracts_are_exact -- --exact
```

`baselines/postgresql-17.9-statements.json` is the companion statement-span
artifact. It freezes each included source blob, byte length, exact statement
byte ranges, and whether the legacy extractor produced a statement or a parse
error for that range. Validate its provenance, membership, counts, ranges, and
the checked-out PostgreSQL source blobs without rebuilding the legacy parser:

```sh
cargo run --locked -p pgtk-sql-migrate -- baseline verify-statements
```

Fresh review and update commands are recorded in the artifact itself. Both use
detached disposable clones and the pinned legacy statement-span capture fixture;
updating the artifact is therefore an explicit baseline review rather than a
normal test operation.

CI also performs the full statement-span review from a full-history `pgtk-sql`
checkout. The review command clones that checkout without hardlinks, detaches at
legacy commit `1e71421d66baac15c8c5264e8f29b5f80122f50e`, recaptures every range,
and byte-compares the canonical result with the committed artifact:

```sh
cargo run --locked -p pgtk-sql-migrate -- baseline review-statements --legacy-repository . --postgres-repository vendor/postgres --baseline baselines/postgresql-17.9.json --spans baselines/postgresql-17.9-statements.json
```

This gate is deliberately stronger than `verify-statements`: changing the
artifact and its internally consistent metadata cannot pass without reproducing
the same bytes from the immutable legacy parser and PostgreSQL corpus.

Capture uses detached, no-hardlink disposable clones of both local source
repositories. PostgreSQL is generated and built only inside that disposable
clone, under Cargo's `OUT_DIR`, using the pinned capture-only build plumbing in
`migration-tool/fixtures/baseline/`. The legacy parser, formatter, oracle C
sources, and differential tests are unchanged. Capture fails if the disposable
PostgreSQL source worktree changes during the build. It also checks that the
original legacy and PostgreSQL repositories have the same HEAD and worktree
state before and after the run.

The machine-readable baseline records the exact SHA-256 of both capture-only
build fixtures. For the reviewed capture they are
`b97860f909c6bb11c453281c021235fa2ab9c19560d393faa887fe742bcb54d7` for
`build-pg.sh` and
`f208ffc91be7ad270b827e9fb6fb38ed8433542bc81e20480794087629617d2f` for
`pgtk-oracle-build.rs`. Changing either fixture is therefore an explicit
baseline update even when statement outcomes happen to remain equal.

Review the committed bytes by repeating the full capture:

```sh
cargo run --locked -p pgtk-sql-migrate -- baseline review --legacy-repository ../recursa-old --postgres-repository vendor/postgres --baseline baselines/postgresql-17.9.json
```

After deliberately reviewing a changed input or rule, update the record with:

```sh
cargo run --locked -p pgtk-sql-migrate -- baseline capture --legacy-repository ../recursa-old --postgres-repository vendor/postgres --output baselines/postgresql-17.9.json
```
