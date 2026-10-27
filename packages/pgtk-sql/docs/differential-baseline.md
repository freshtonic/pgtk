# Differential baseline: PostgreSQL 17.9 corpus, one oracle per target version

`baselines/postgresql-17.9.json` is the named, machine-readable floor for
PostgreSQL statement parity. It records the immutable legacy Git identities,
the PostgreSQL 17.9 gitlink, all 226 regression SQL files, the legacy suite's
222 included files and four explained exclusions, and per-file statement and
pass/skip/fail counts.

The differential test no longer checks those legacy statements. Its
statement set is what psql sends to the server for each file: the frozen
psql submissions (#10, see "The statement set" below).

## Frozen corpus and pinned oracle

The corpus and the oracle are two different inputs:

- The **corpus** is frozen at PostgreSQL 17.9 (`6d396980fc5`). It is the set
  of regression SQL files whose psql submissions are frozen in
  `baselines/postgresql-17.9-submissions.json`. The tests read each file by
  its Git blob ID from the
  `vendor/postgres` object database, not from the checked-out tree. The
  baseline file names and their `postgres` fields name this corpus.
- The **oracle** is the raw parser of the pinned PostgreSQL release of the
  target version. `pgtk-oracle/pins.tsv` is the one pin table. For `pg17`, the
  pin is PostgreSQL 17.11 (`REL_17_11`, `083ac033419`), and the
  `vendor/postgres` submodule gitlink must be the same commit (a pgtk-oracle
  test checks this). `pgtk-oracle` extracts the pinned commit with
  `git archive` and builds it again when the pin moves.

Thus a move of the pin changes only the oracle. The submissions, the legacy
statement spans and the per-file counts stay valid, so each outcome change
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

## The statement set: psql's submissions (#10)

The regression files are psql scripts, not SQL. The differential now gives
the raw parser only what psql sends to the server: each query buffer of each
script. Until #10 the statement set was the legacy statement ranges
(`baselines/postgresql-17.9-statements.json`), which a lenient splitter made.
Those ranges stay for the accepted legacy gap ledger, the corpus tests of
parity, recovery and minimum version, and the benchmarks.

### The frozen submissions

`baselines/postgresql-17.9-submissions.json` records the submissions. For
each corpus file it records the Git blob ID and size (the same as the legacy
spans), the source extent of each submission, its kind, and the rewrites
inside it. A rewrite is a place where psql sends other text than the source:
`\;` becomes `;`, a send command such as `\g` becomes `;`, and a variable
becomes its value. The test reads each file by its blob ID and applies the
rewrites, so the corpus stays reproducible without pgtk-psql.

`pgtk-psql/tests/submissions.rs` derives the record. It renders each file
with `pgtk_psql::render` and takes `Rendered::submissions`, with the two
variables that pg_regress gives psql (`pg_regress_main.c`:
`-v HIDE_TABLEAM=on -v HIDE_TOAST_COMPRESSION=on`). The corpus is the 17.9
corpus and pg_regress runs it with the psql of the same release, so the
derivation is that of the `pg17` build, and all six version baselines use
the same submissions. The test requires the same bytes as the record, so a
change of pgtk-psql that moves a submission fails there.

`Rendered::submissions` follows psql's main loop:

- The text between two query-buffer boundaries is one submission: at a `;`
  outside parentheses and outside a `BEGIN ... END` routine body, and at a
  send command. A `\;` does not end the buffer, so a submission can hold more
  than one statement.
- psql sends nothing at `\r` (or a discard command of 18) and nothing in an
  inactive `\if` branch.
- While the buffer is empty, psql keeps whitespace and `--` comments out of
  it, so a submission starts at its first other byte. A block comment stays.
- At the end of the input psql sends the text that remains, also when the
  buffer is not complete.
- COPY data and the `\.` line are never in a submission. A meta-command
  sends nothing.

Each submission has one of four kinds. The differential checks only server
SQL, and it counts each other kind with a stable reason:

| Kind | Reason | Count | What it is |
|---|---|---|---|
| `S` | (checked) | 44,132 | Server SQL. |
| `E` | `empty-query` | 23 | No SQL token but `;`. At a `;` psql sends an empty query, and at a send command with an empty buffer it sends the previous query again. |
| `U` | `unbound-variable` | 270 | An interpolation of a variable that is not set, also through the value of another variable (`\set f :abs_srcdir '/x'`). The value comes from the execution: `\gset`, `\getenv` or `LASTOID`. |
| `C` | `unknown-meta-command` | 3 | An unknown meta-command. psql reports it and sends none of it. pgtk-psql keeps it in the text (ADR 0013). |

The `pgtk-sql` check of a submission (`check_submission` in
`tests/support/diff_check.rs`) is the check of a statement, with one
addition: each statement of the query string parses in turn. A statement
is the shortest run of tokens up to a `;` (or to the end) that parses as one
whole `Statement`, so a `;` in a `BEGIN ATOMIC` body is not a split. A
submission with one statement gives the same result and the same diagnostic
as before.

To regenerate the record after a reviewed change of pgtk-psql:

```sh
cargo test -p pgtk-psql --test submissions -- --ignored regenerate_frozen_submissions
```

Then regenerate the version baselines (below), because their identities are
submission indexes.

### The change in membership

The legacy split gave 43,474 statements: 43,349 statement items and 125
parse-error items. psql gives 44,428 submissions. Of these, 43,243 are
exactly a legacy item. The diagnostic
`cargo test -p pgtk-psql --test submissions -- --ignored --nocapture compare_with_the_legacy_ranges`
(with `PGTK_SUBMISSIONS_VERBOSE=1` for each submission) gives the comparison
below. The other submissions are in five classes:

- **New: 1,027.** The legacy split dropped each statement with a colon that
  looked like a psql variable, also a colon in a string, a comment or a
  dollar-quoted body (`'{1:"abc"}'::jsonb`, `'HH24:MI:SS'`). psql sends
  these, and 768 of them are now server SQL that the differential checks.
  The other 259 are new `U` and `E` submissions: the legacy split dropped
  these too, as non-SQL.
  A new submission can hold more than one statement: in `update.sql`,
  `:init_range_parted` is a variable whose value holds a `;`.
- **Merged: 39.** One submission holds more than one legacy item. 26 are
  `\;` lists in `transactions.sql`, and one is a `\g` in `psql.sql`. 12 are
  `BEGIN ATOMIC` bodies that the legacy split cut at each `;`
  (`create_function_sql.sql` 6, `create_procedure.sql`, `rules.sql` and
  `window.sql` 2 each). Each such legacy piece was a statement that the
  oracle rejects, so it was a pass with no meaning.
- **Earlier start: 75.** The submission starts at a block comment before
  the legacy statement (`create_table_like.sql` 20, `index_including.sql` 12,
  `partition_prune.sql` 9). psql sends the comment.
- **Other end: 44.** The submission ends at a send command and its
  arguments (`\gset`, `\crosstabview` in `psql.sql`, `psql_crosstab.sql`,
  `stats.sql`), or the legacy range kept a comment after the statement.
- **Legacy only: 19, all in `copyselect.sql`.** psql 17.11 (the oracle pin)
  reads COPY data after each `COPY ... FROM STDIN` of a buffer, also when
  the server rejects the command (`46fa1f6f373`, CVE-2026-6464). The 17.9
  script has no `\.` after the rejected `copy (select * from test1) from
  stdin;`, so psql 17.11 reads the next 18 statements and one parse-error
  item as COPY data, up to the `\.` of a later COPY. The 17.11 script has
  the `\.` line. The last submission of the file then starts in that data,
  and holds `\.`, so it is a `C` submission.

Two more `C` submissions are in `strings.sql`. psql reads a string with the
`standard_conforming_strings` setting of the server, and the script sets it
to `off`. pgtk-psql reads the string as the setting `on` gives it, so it
finds a meta-command in the string. The legacy split misread the same
strings: it made one item (410) of six statements there.

The table gives each file where the membership changed. "Pass" is the
`pg17` pass count before and after.

| File | Legacy statements | Submissions | Same | Merged | Earlier start | Other end | New | Legacy only | Excluded | Pass before → after |
|---|---|---|---|---|---|---|---|---|---|---|
| `advisory_lock.sql` | 23 | 38 | 23 | 0 | 0 | 0 | 15 | 0/0 | unbound-variable 15 | 23 → 23 |
| `aggregates.sql` | 513 | 514 | 513 | 0 | 0 | 0 | 1 | 0/0 | unbound-variable 1 | 513 → 513 |
| `alter_generic.sql` | 331 | 332 | 331 | 0 | 0 | 0 | 1 | 0/0 | unbound-variable 1 | 331 → 331 |
| `alter_table.sql` | 1653 | 1653 | 1649 | 0 | 4 | 0 | 0 | 0/0 | 0 | 1653 → 1653 |
| `arrays.sql` | 483 | 486 | 483 | 0 | 0 | 0 | 3 | 0/0 | unbound-variable 30 | 483 → 456 |
| `brin.sql` | 66 | 68 | 66 | 0 | 0 | 0 | 2 | 0/0 | 0 | 66 → 68 |
| `brin_bloom.sql` | 38 | 40 | 38 | 0 | 0 | 0 | 2 | 0/0 | 0 | 38 → 40 |
| `brin_multi.sql` | 170 | 172 | 170 | 0 | 0 | 0 | 2 | 0/0 | 0 | 170 → 172 |
| `btree_index.sql` | 92 | 96 | 92 | 0 | 0 | 0 | 4 | 0/0 | unbound-variable 4 | 92 → 92 |
| `collate.sql` | 144 | 144 | 143 | 0 | 1 | 0 | 0 | 0/0 | 0 | 144 → 144 |
| `comments.sql` | 6 | 7 | 4 | 0 | 2 | 0 | 1 | 0/0 | empty-query 1 | 6 → 6 |
| `constraints.sql` | 317 | 319 | 317 | 0 | 0 | 0 | 2 | 0/0 | unbound-variable 2 | 317 → 317 |
| `conversion.sql` | 63 | 65 | 63 | 0 | 0 | 0 | 2 | 0/0 | unbound-variable 2 | 63 → 63 |
| `copy.sql` | 77 | 87 | 77 | 0 | 0 | 0 | 10 | 0/0 | unbound-variable 10 | 77 → 77 |
| `copy2.sql` | 199 | 200 | 199 | 0 | 0 | 0 | 1 | 0/0 | 0 | 199 → 200 |
| `copyselect.sql` | 40 | 21 | 20 | 0 | 1 | 0 | 0 | 18/1 | unknown-meta-command 1 | 40 → 20 |
| `create_function_c.sql` | 2 | 5 | 2 | 0 | 0 | 0 | 3 | 0/0 | unbound-variable 3 | 2 → 2 |
| `create_function_sql.sql` | 154 | 147 | 141 | 6 | 0 | 0 | 0 | 0/0 | 0 | 154 → 147 |
| `create_index.sql` | 608 | 611 | 608 | 0 | 0 | 0 | 3 | 0/0 | unbound-variable 3 | 608 → 608 |
| `create_procedure.sql` | 109 | 107 | 105 | 2 | 0 | 0 | 0 | 0/0 | 0 | 109 → 107 |
| `create_table_like.sql` | 144 | 144 | 124 | 0 | 20 | 0 | 0 | 0/0 | 0 | 144 → 144 |
| `create_type.sql` | 80 | 85 | 80 | 0 | 0 | 0 | 5 | 0/0 | unbound-variable 5 | 80 → 80 |
| `create_view.sql` | 305 | 307 | 305 | 0 | 0 | 0 | 2 | 0/0 | unbound-variable 2 | 305 → 305 |
| `domain.sql` | 462 | 462 | 462 | 0 | 0 | 0 | 0 | 0/0 | unbound-variable 1 | 462 → 461 |
| `encoding.sql` | 60 | 67 | 59 | 0 | 1 | 0 | 7 | 0/0 | unbound-variable 6 | 60 → 61 |
| `errors.sql` | 85 | 87 | 84 | 0 | 1 | 1 | 1 | 0/0 | 0 | 85 → 87 |
| `event_trigger_login.sql` | 10 | 11 | 10 | 0 | 0 | 0 | 1 | 0/0 | unbound-variable 1 | 10 → 10 |
| `explain.sql` | 50 | 51 | 50 | 0 | 0 | 0 | 1 | 0/0 | 0 | 50 → 51 |
| `foreign_data.sql` | 529 | 530 | 529 | 0 | 0 | 0 | 1 | 0/0 | unbound-variable 1 | 529 → 529 |
| `groupingsets.sql` | 171 | 172 | 171 | 0 | 0 | 0 | 1 | 0/0 | 0 | 171 → 172 |
| `hash_func.sql` | 41 | 43 | 41 | 0 | 0 | 0 | 2 | 0/0 | 0 | 41 → 43 |
| `hash_index.sql` | 96 | 100 | 96 | 0 | 0 | 0 | 4 | 0/0 | unbound-variable 4 | 96 → 96 |
| `horology.sql` | 330 | 388 | 330 | 0 | 0 | 0 | 58 | 0/0 | 0 | 330 → 388 |
| `index_including.sql` | 135 | 135 | 123 | 0 | 12 | 0 | 0 | 0/0 | 0 | 135 → 135 |
| `index_including_gist.sql` | 50 | 50 | 43 | 0 | 7 | 0 | 0 | 0/0 | 0 | 50 → 50 |
| `indirect_toast.sql` | 27 | 28 | 27 | 0 | 0 | 0 | 1 | 0/0 | unbound-variable 1 | 27 → 27 |
| `inet.sql` | 109 | 111 | 109 | 0 | 0 | 0 | 2 | 0/0 | 0 | 109 → 111 |
| `inherit.sql` | 677 | 677 | 674 | 0 | 3 | 0 | 0 | 0/0 | 0 | 677 → 677 |
| `insert_conflict.sql` | 257 | 261 | 257 | 0 | 0 | 0 | 4 | 0/0 | empty-query 4 | 257 → 257 |
| `interval.sql` | 442 | 443 | 442 | 0 | 0 | 0 | 1 | 0/0 | 0 | 442 → 443 |
| `json.sql` | 406 | 460 | 406 | 0 | 0 | 0 | 54 | 0/0 | 0 | 406 → 460 |
| `jsonb.sql` | 855 | 1059 | 855 | 0 | 0 | 0 | 204 | 0/0 | unbound-variable 4 | 855 → 1055 |
| `jsonb_jsonpath.sql` | 766 | 821 | 766 | 0 | 0 | 0 | 55 | 0/0 | 0 | 766 → 821 |
| `largeobject.sql` | 111 | 127 | 111 | 0 | 0 | 0 | 16 | 0/0 | unbound-variable 16 | 111 → 111 |
| `lock.sql` | 130 | 131 | 130 | 0 | 0 | 0 | 1 | 0/0 | unbound-variable 1 | 130 → 130 |
| `macaddr.sql` | 30 | 35 | 30 | 0 | 0 | 0 | 5 | 0/0 | 0 | 30 → 35 |
| `macaddr8.sql` | 63 | 71 | 63 | 0 | 0 | 0 | 8 | 0/0 | 0 | 63 → 71 |
| `misc.sql` | 55 | 61 | 55 | 0 | 0 | 0 | 6 | 0/0 | unbound-variable 6 | 55 → 55 |
| `misc_functions.sql` | 115 | 122 | 115 | 0 | 0 | 0 | 7 | 0/0 | unbound-variable 7 | 115 → 115 |
| `mvcc.sql` | 10 | 11 | 10 | 0 | 0 | 0 | 1 | 0/0 | unbound-variable 1 | 10 → 10 |
| `numerology.sql` | 88 | 88 | 86 | 0 | 0 | 2 | 0 | 0/0 | 0 | 88 → 88 |
| `partition_prune.sql` | 697 | 701 | 688 | 0 | 9 | 0 | 4 | 0/0 | empty-query 3 | 697 → 698 |
| `password.sql` | 45 | 51 | 45 | 0 | 0 | 0 | 6 | 0/0 | 0 | 45 → 51 |
| `privileges.sql` | 1216 | 1216 | 1214 | 0 | 2 | 0 | 0 | 0/0 | 0 | 1216 → 1216 |
| `psql.sql` | 177 | 418 | 170 | 1 | 1 | 15 | 231 | 0/0 | empty-query 14, unbound-variable 8 | 177 → 396 |
| `psql_crosstab.sql` | 29 | 30 | 14 | 0 | 0 | 15 | 1 | 0/0 | empty-query 1 | 29 → 29 |
| `publication.sql` | 655 | 655 | 652 | 0 | 3 | 0 | 0 | 0/0 | 0 | 655 → 655 |
| `regproc.sql` | 105 | 105 | 102 | 0 | 3 | 0 | 0 | 0/0 | 0 | 105 → 105 |
| `reloptions.sql` | 63 | 66 | 63 | 0 | 0 | 0 | 3 | 0/0 | unbound-variable 3 | 63 → 63 |
| `rules.sql` | 620 | 618 | 616 | 2 | 0 | 0 | 0 | 0/0 | 0 | 620 → 618 |
| `sqljson.sql` | 219 | 219 | 218 | 0 | 1 | 0 | 0 | 0/0 | 0 | 219 → 219 |
| `sqljson_queryfuncs.sql` | 299 | 312 | 298 | 0 | 1 | 0 | 13 | 0/0 | 0 | 299 → 312 |
| `stats.sql` | 364 | 446 | 358 | 0 | 0 | 6 | 82 | 0/0 | unbound-variable 82 | 364 → 364 |
| `strings.sql` | 460 | 471 | 459 | 0 | 1 | 5 | 6 | 0/0 | unknown-meta-command 2 | 460 → 469 |
| `subscription.sql` | 158 | 159 | 158 | 0 | 0 | 0 | 1 | 0/0 | unbound-variable 1 | 158 → 158 |
| `tablespace.sql` | 202 | 204 | 202 | 0 | 0 | 0 | 2 | 0/0 | unbound-variable 2 | 202 → 202 |
| `temp.sql` | 161 | 162 | 161 | 0 | 0 | 0 | 1 | 0/0 | 0 | 161 → 162 |
| `test_setup.sql` | 60 | 69 | 60 | 0 | 0 | 0 | 9 | 0/0 | unbound-variable 9 | 60 → 60 |
| `text.sql` | 73 | 73 | 71 | 0 | 2 | 0 | 0 | 0/0 | 0 | 73 → 73 |
| `timestamp.sql` | 169 | 171 | 169 | 0 | 0 | 0 | 2 | 0/0 | 0 | 169 → 171 |
| `timestamptz.sql` | 361 | 382 | 361 | 0 | 0 | 0 | 21 | 0/0 | 0 | 361 → 382 |
| `transactions.sql` | 490 | 433 | 407 | 26 | 0 | 0 | 0 | 0/0 | 0 | 490 → 433 |
| `triggers.sql` | 1046 | 1057 | 1046 | 0 | 0 | 0 | 11 | 0/0 | unbound-variable 5 | 1046 → 1052 |
| `tsdicts.sql` | 126 | 131 | 126 | 0 | 0 | 0 | 5 | 0/0 | 0 | 126 → 131 |
| `tsearch.sql` | 408 | 442 | 408 | 0 | 0 | 0 | 34 | 0/0 | unbound-variable 1 | 408 → 441 |
| `tstypes.sql` | 223 | 238 | 223 | 0 | 0 | 0 | 15 | 0/0 | 0 | 223 → 238 |
| `tuplesort.sql` | 104 | 108 | 104 | 0 | 0 | 0 | 4 | 0/0 | unbound-variable 6 | 104 → 102 |
| `txid.sql` | 38 | 47 | 38 | 0 | 0 | 0 | 9 | 0/0 | unbound-variable 5 | 38 → 42 |
| `type_sanity.sql` | 60 | 61 | 60 | 0 | 0 | 0 | 1 | 0/0 | 0 | 60 → 61 |
| `update.sql` | 257 | 286 | 257 | 0 | 0 | 0 | 29 | 0/0 | unbound-variable 14 | 257 → 272 |
| `vacuum.sql` | 294 | 296 | 294 | 0 | 0 | 0 | 2 | 0/0 | unbound-variable 2 | 294 → 294 |
| `window.sql` | 382 | 384 | 378 | 2 | 0 | 0 | 4 | 0/0 | 0 | 382 → 384 |
| `xid.sql` | 72 | 84 | 72 | 0 | 0 | 0 | 12 | 0/0 | unbound-variable 5 | 72 → 79 |
| `xml.sql` | 251 | 270 | 251 | 0 | 0 | 0 | 19 | 0/0 | 0 | 251 → 270 |

Before #10 the `pg17` build checked 43,474 statements: 43,474 pass, 0 skip
and 0 fail (the 18 frozen legacy skips all pass). After #10 it checks 44,132
server submissions: 44,132 pass, 0 skip and 0 fail, and it excludes 296
with a reason. The new statement set found one grammar gap at first:
`rules.sql` submission 571, a `BEGIN ATOMIC` body with a `MERGE` whose
`INSERT` action names a column with a subscript, `INSERT (filling[1], id)`
(gram.y's `merge_insert` takes an `insert_column_list`). The legacy split
cut the body at its first `;`, so no check saw that `MERGE` before. Commit
`c928d997` fixed the grammar and removed the gap. The oracle rejects 173 of the
checked submissions; it rejected 353 of the legacy statements, because many
legacy pieces were not whole statements.

## One baseline for each target version

Each target version (ADR 0009) has its own version baseline in
`baselines/target-versions/<feature>.json`, for example `pg16.json`. All six
use the same frozen 17.9 corpus. The oracle is the raw parser of the pinned
release of that version, from `pgtk-oracle/pins.tsv`:

| Feature | Oracle | Expected version gaps | Gaps proved by a build |
|---|---|---|---|
| `pg14` | `REL_14_24` | 0 | not yet (#10) |
| `pg15` | `REL_15_19` | 0 | yes (after `c928d997`) |
| `pg16` | `REL_16_15` | 0 | not yet (#10) |
| `pg17` | `REL_17_11` | 0 | yes |
| `pg18` | `REL_18_6` | 0 | yes (after `c928d997`) |
| `pg19-beta` | `REL_19_STABLE` at `b73d13c` | 0 | not yet (#10) |

A version baseline records:

- the oracle ref and commit, which must equal the pin of the version;
- the corpus name and gitlink (`postgresql-17.9`), and the name of the
  frozen submissions;
- for each corpus file, the oracle outcome of each submission, in submission
  order: `A` (accept), `R` (reject), or `-` for a submission that the
  differential does not check (kind `E`, `U` or `C`);
- the totals: the submissions, the checked and the excluded ones, the oracle
  accepts, the expected pass, skip and fail counts, and the expected version
  gaps. Every checked submission passes but the expected version gaps.

An **expected version gap** is a submission where a build of the version does
not pass the differential check with that version's oracle. Each gap records
the submission identity (file, zero-based submission index and source byte
range), the pgtk-sql outcome and the oracle outcome:

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
empty.** The `pg17` list is empty.

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
over-acceptances that are not fixed. Each entry has the submission identity
(file, submission index and source byte range, schema 2) and a named cause. The differential test expects the "over-acceptance"
failure for an entry where the oracle of the target version rejects the
statement. So a new over-acceptance fails, and an entry that pgtk-sql now
rejects fails until its entry is removed. The list is empty. A statement that
the oracle rejects for a reason outside the raw parser goes in the version
baseline as a `"fail"` gap with the oracle outcome `"reject"`, with its reason
in this document.

The rule needs the other target versions too. Their oracles reject statements
that the `pg17` oracle accepts. Over the legacy statements these were 1009
for `pg14`, 616 for `pg15` and 369 for `pg16`. Over the server submissions
(#10) they are 1024, 631 and 374 (none for `pg18` and `pg19-beta`). Before #95 a build of an older
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

A checked submission that is not an expected gap must pass: a skip outside
the list fails too.

The fast test (`differential_baseline`) checks all six files without an
oracle: the pins, the corpus identity, that each server submission and no
other has an oracle outcome, the totals, and the order and ranges of the
gaps.

The accepted legacy gap test still names its statements by the legacy
ranges. It asks the live oracle about each one, and it finds an expected
version gap by the submission that overlaps the statement.

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

The oracle outcomes need only the oracle of the version, not a pgtk-sql
build of it. `pgtk-oracle/tests/corpus_outcomes.rs` mounts the baseline
module of this crate and writes the oracle outcomes of its version. It keeps
each recorded gap whose submission and oracle outcome did not change.
`PGTK_SEED_GAPS_FROM=pg17` also adds each skip gap of `pg17` whose submission
this oracle accepts:

```sh
PGTK_SEED_GAPS_FROM=pg17 cargo test -p pgtk-oracle --no-default-features \
  --features pg16,parser --test corpus_outcomes -- --ignored --nocapture
```

For #10 the `pg17` baseline comes from a full regeneration. The other five
come from this test, with the gaps seeded from `pg17`. Their oracle outcomes
are exact: the minimum-version corpus test of each build reads them. Their
gap lists are proved only where the table above says so: the `pg15` and
`pg18` builds ran the differential over the new statement set after
`c928d997`. The release gate (`scripts/release-gate pgNN`) runs the
differential test of each version and fails on a stale or missing gap. Then
run `regenerate_version_baseline` with that version and review the diff.

## The psql oracle

`docs/psql-oracle.md` describes the second differential: `pgtk-psql-oracle`
links `psqlscan.l` and `psqlscanslash.l` of the same pinned release, and
`pgtk-psql/tests/psql_oracle.rs` compares pgtk-psql's lexing with psql's own. It
reads the same frozen corpus, by Git blob ID, and pins its outcomes per
target version in `baselines/psql-oracle/<feature>.json`.

## Frozen corpus rules

These are the rules of the legacy statement spans, which the accepted legacy
gap ledger, the benchmarks and some corpus tests still read. The
differential reads the psql submissions instead (#10).

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
