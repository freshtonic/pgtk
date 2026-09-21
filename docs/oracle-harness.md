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

## Order and state

Files run in `parallel_schedule` order, because later files use tables that
`test_setup.sql` and others create. Each statement is run through the simple
protocol after it is compared, so the server's state moves as it would under
`pg_regress`. Three things follow:

- **Catalog reloads.** A statement that may change the catalog marks it
  dirty. The Catalog is reloaded lazily, before the next statement that is
  compared, on the same connection, so uncommitted DDL is seen exactly as the
  Oracle sees it. The list of statement kinds that keep the catalog is in
  `replay.rs`; anything not listed counts as DDL.
- **Search path.** `current_schemas(false)` is read before every comparison,
  so `SET search_path` and `SET ROLE` need no tracking.
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
docker run -d --name pg-analyze-oracle-17 -e POSTGRES_USER=oracle \
  -e POSTGRES_PASSWORD=oracle -e POSTGRES_DB=regression -p 54317:5432 postgres:17
PG_ANALYZE_ORACLE_URL=postgres://oracle:oracle@localhost:54317/regression \
  cargo test --test oracle -- --nocapture
```

The server must be disposable. The corpus creates and drops roles, schemas
and databases, and some statements terminate the session. Without the
variable the test prints a message and passes.

## What is deferred

- The tree-level Oracle, a `nodeToString` extension, is a later workspace
  member (design session Q15 and Q34).
- Oracles for 14, 15 and 16 need only a container of that version and a
  baseline file each; the harness reads the major from the Catalog.
