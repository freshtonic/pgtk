# The Oracle harness

The harness is the first thing built and the measure of every slice after it.
It replays PostgreSQL's regression corpus against a live server, the Oracle,
and against this crate, the Subject, and compares what each says about every
statement. `tests/oracle.rs` is the entry point; `tests/support/` holds the
parts.

## What is compared

For each statement the Oracle is asked through the extended protocol: a Parse
and a Describe, never an Execute. That is the closest a stock server comes to
exposing `parse_analyze`. It answers with either the parameter types and the
result columns (name, type, typmod, source table and column), or the
Diagnostic it raised (SQLSTATE, message, cursor position). The Subject answers
the same question in the same terms. The verdict for a statement is one of:

| Verdict | Meaning |
|---|---|
| Skipped | Not compared: a psql meta-command, text `pg-sql` cannot parse, or the Oracle's session was in a failed transaction. |
| Unsupported | The Subject cannot analyse it yet. Never a wrong answer. |
| Agree | Both accepted with the same descriptor, or both rejected with the same SQLSTATE, message and position. |
| Disagree | Anything else. Written to `target/oracle/<file>.txt` with both answers. |

A whole-row reference has a source table and no source column: the wire
protocol sends column 0, which the client reads as none, and the Subject's
answer says the same.

The per-file counts are pinned in `baselines/oracle-postgresql-<major>.json`.
The test fails when a run differs from the pinned counts in either direction,
so an improvement is pinned on purpose with `PG_ANALYZE_UPDATE_BASELINE=1`.
This is the pattern `pg-sql` uses for its own differential baseline.

## How the corpus is split

The corpus is the `vendor/postgres` submodule, pinned to the same
PostgreSQL 17.9 commit `pg-sql` vendors, so both crates read the same bytes.
The regression files are psql documents. Ninety-seven of them use
meta-commands such as `\set`, `\d` and `\if` that neither `pg-sql` nor
`pg-psql` model, so the strict document parser rejects those files whole. The
harness therefore reads the statement byte ranges `pg-sql` froze when it
captured its own baseline, copied to `baselines/postgresql-17.9-statements.json`,
and treats every item that split marked as a non-statement as Skipped. The
reader checks that the submodule is at the commit the ranges were frozen from.
The design session chose the document parser as the primary splitter with the
frozen ranges as a cross-check; that is reversed until the meta-command gap in
`pg-psql` closes.

Each item is rendered through `pg-psql` before anything else, which strips
psql's own terminators (`\;`, `\g`, `\gset`, `\crosstabview` and kin) the
way psql does before sending. That leaves eight statements `pg-sql` rejects,
all of which PostgreSQL rejects at parse too.

## Order, state and parallelism

Files depend on state left by earlier files, so the schedule matters, and
the run is parallel anyway. The harness does it in two passes. Both run in
one Docker container that the harness makes from the image the run names,
and keeps between runs (`tests/support/cluster.rs`).

**Snapshots.** The first pass makes an empty cluster with the `oracle`
superuser and the `regression` database, as pg_regress has it. It executes
every file in `parallel_schedule` order on that one server, with a fresh
session per file as pg_regress uses. After each schedule group it stops the
server and copies the whole cluster into a snapshot. No comparison happens;
it is pure execution. The snapshots are in the container under a directory
named after the vendored PostgreSQL commit and a format number. A run whose
snapshots are complete skips the pass; `PG_ANALYZE_ORACLE_REBUILD=1` forces
it.

**Replay.** The second pass gives each file a server of its own: it copies
the snapshot before the file's group, starts a server on the copy on the
worker's own port, executes the file's earlier siblings from the same group
on it, then replays the file in a fresh session, asking the Oracle and the
Subject about every statement. When the file is done, the server is stopped
and its cluster removed. That is exactly the state the serial schedule gives
the file, so files can run in any order on a worker pool, and one file can
run alone in seconds with `PG_ANALYZE_ORACLE_FILES`.

**Isolation.** A snapshot is a whole cluster, so it holds the objects the
whole server shares as well: roles, tablespaces, databases, subscriptions,
settings and the cumulative statistics. Because every file has its own
server, no file can see or change what another file does, whatever runs at
the same time. This is true by construction: the workers share no server, no
port and no data directory. Inside one group pg_regress's own rule applies:
files use disjoint names.

These details follow:

- **Catalog reloads.** A statement that may change the catalog marks it
  dirty. The Catalog is reloaded lazily, before the next statement the
  Subject says it wants, on the same connection, so uncommitted DDL is seen
  exactly as the Oracle sees it. A statement the Subject does not want, such
  as a Utility Statement without a query, is still compared, against
  whatever Catalog the session holds, so it costs no reload. The list of
  statement kinds that keep the catalog is in `replay.rs`; anything not
  listed counts as DDL.
- **Known divergences.** A rejection the Oracle raises outside parse
  analysis while the Subject analysed counts as a known divergence, not a
  disagreement: data errors from input functions (classes 22, 54 and 55,
  and 42602), domain checks folded at parse time (class 23), scanner-level
  syntax (42601, 0A000), the rewriter's policy and view checks (42P17,
  55000), and catalog lookups done by `reg*` input functions on unknown
  literals. One divergence runs the other way: an EXECUTE or a FETCH whose
  name the harness's Session misses, although the server holds it. The rule
  is `compare` in `replay.rs`.
- **Search path.** `current_schemas(false)` is read before every comparison,
  so `SET search_path` and `SET ROLE` need no tracking.
- **The Session.** EXECUTE, FETCH and SHOW of a custom parameter are
  answered from session state, so the harness keeps a `Session` for each
  file it replays. After a statement the server accepted, it folds in the
  Session Effect the Subject reported, so a PREPARE, a DECLARE or a SET
  earlier in the file is there when the EXECUTE, the FETCH or the SHOW
  arrives. A statement the server rejected changes nothing. The end of a
  transaction closes the cursors that are not held; a transaction that ends
  while it is failed closes every cursor, since it rolls everything back.
  A Session holds Catalog handles, so it is rewritten against the new
  Catalog whenever the harness reloads one, and a reconnection empties it.
  A name the Session does not hold answers with an empty Result Descriptor,
  as the server's Describe answers it, so the two agree. The Session the
  harness keeps can still miss a name the server holds, since it sees only
  the statements it replays: a cursor a function or a plpgsql body opened
  is invisible to it, and so is one whose columns come from a table that
  was dropped, because a Catalog Handle names a live relation. The Subject
  then answers with no columns while the server answers with the cursor's,
  which is the known divergence above.
- **Failed transactions.** PostgreSQL rejects everything after an error inside
  a transaction with `25P02`, which is not an analysis outcome. The harness
  probes the session before each statement and skips while it is failed.

`COPY` statements are compared but never run: `FROM STDIN` would put the
connection into copy mode, and every other form names a file the server
cannot reach. Statements that end the connection on purpose cause a
reconnect and a catalog reload.

## Session settings

The connection applies what `pg_regress` applies: `lc_messages` and the other
locale settings to `C`, `bytea_output` to `hex`, the `America/Los_Angeles`
time zone, `Postgres, MDY` date style and `postgres_verbose` interval style,
so message text and literal handling match the expected output files.

## Running it

```sh
PG_ANALYZE_ORACLE_IMAGE=postgres:17 cargo test --release --test oracle -- --nocapture
```

The harness makes a container named after the image, for example
`pg-analyze-oracle-postgres-17`, if it does not exist. The container
publishes ports 54400 to 54431 on 127.0.0.1, one for each worker, so a run
has at most 32 workers. Each cluster has durability off, because it is
disposable and the copies are I/O bound. `PG_ANALYZE_ORACLE_JOBS` sets the
worker count, default the CPU count. `PG_ANALYZE_ORACLE_FILES=int4,join`
replays only those files and checks only their counts; with
`PG_ANALYZE_UPDATE_BASELINE=1` it re-pins only those entries.

Only one run may use the container at a time: a run stops every server it
finds in the container when it starts. Without the variable the test prints
a message and passes.

## What is deferred

- The tree-level Oracle, a `nodeToString` extension, is a later workspace
  member (design session Q15 and Q34).
- Oracles for 14, 15 and 16 need only an image of that version and a
  baseline file each; the harness reads the major from the image's
  binaries.
