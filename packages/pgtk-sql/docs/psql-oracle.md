# The psql oracle: pgtk-psql against psql's own lexer

`pgtk-oracle` links PostgreSQL's raw parser so that pgtk-sql can be compared with
the server (ADR 0002). `pgtk-psql-oracle` does the same one layer up: it links
`src/fe_utils/psqlscan.l` and `src/bin/psql/psqlscanslash.l` of the same
pinned PostgreSQL release, so that pgtk-psql can be compared with psql.

The two oracles share one pin table, `pgtk-oracle/pins.tsv`, and one build
script, `pgtk-oracle/scripts/build-pg.sh` (ADR 0009). A build of
`pgtk-psql-oracle` selects its release with the version feature, as every other
crate does.

## What the oracle links

| Source | What it gives |
|---|---|
| `src/fe_utils/psqlscan.c` | The psql SQL lexer: text runs, interpolation, `;` and the backslash boundary. |
| `src/bin/psql/psqlscanslash.c` | `psql_scan_slash_command`, the one authority on where a command name ends. |
| `src/fe_utils/conditional.c` | The `\if` stack that `psqlscanslash.c` refers to. |
| `src/bin/psql/stringutils.c` | `strtokx`, which reads the arguments of `\copy` as `parse_slash_copy` does. |
| `src/interfaces/libpq/libpq.a` | `PQExpBuffer` and the multibyte helpers. |
| `src/common/libpgcommon.a`, `src/port/libpgport.a` | The frontend runtime. |

`csrc/psql_oracle.c` is the shim. `src/lib.rs` turns its result into
[`Scan`], which reports the text psql forwards to the server, one event for
each interpolation, one for each `;` that submits the query buffer, one for
each backslash command with the name `psqlscanslash.l` read and the extent and
value of each of its arguments, one for each inactive `\if` branch whose query
text psql discards, one for each block of COPY data that psql reads from the
script, and whether the document ended with a complete buffer.

The shim reads the arguments of each command as `HandleSlashCmds` and
`exec_command` in `command.c` read them: with the same `psql_scan_slash_option`
calls and option types (`OT_NORMAL`, `OT_WHOLE_LINE`, `OT_FILEPIPE`, and the
option list of `\g`), then the extra options that `HandleSlashCmds` drops, then
`psql_scan_slash_command_end`. A name that psql does not know takes the error
path, which drops the rest of the line. The shim applies `\set` and `\unset`
to its own variables, and `\if`, `\elif`, `\else` and `\endif` to its own
conditional stack, as `exec_command_if` and the others do: it evaluates the
expression with `ParseVariableBool`, substitutes no variable in an inactive
branch, and saves and restores the query length and the lexer state as
`save_query_text_state` and `discard_query_text` do. It runs no other command.
The passthrough of the lexer is a conditional stack whose only entry is
inactive, so `psqlscanslash.l` never runs a backquoted shell command.

The shim resets the lexer state (`psql_scan_reset`) where `MainLoop` and
`command.c` reset it: after a `;` that sends the buffer, after a command that
ends the buffer, and at `\r` and `\watch`, each in an active branch only. The
parenthesis depth and the `BEGIN` depth of `psqlscan.l` then start again for
the next buffer, as in psql.

### COPY data

When psql sends a buffer, it counts the `COPY ... FROM STDIN` commands of the
buffer: `MainLoop` asks `psql_scan_count_copy_from_stdin` after a `;`, and
`ExecQueryAndProcessResults` scans the buffer text again after a send command
such as `\g`. `do_copy` reads one block for `\copy ... from stdin`. For each
COPY, `handleCopyIn` (`src/bin/psql/copy.c`) reads lines from the script itself,
not from the lexer: from the line after the line that sent the buffer, up to a
line that is exactly `\.` and LF or CRLF, or to end of input. psql reads the
data even when the server refuses the COPY, so that the script stays in step.
The rest of the line that sent the buffer is SQL text again after the data.
The rules are the same in each target version.

The shim does the same, with psql's own counts: it reports each block, it lexes
the rest of the line, and then it continues after the data. These limits
apply to both sides:

- psql reads a binary COPY (`PQbinaryTuples`) to end of input. The format
  comes from the server, so both sides read every COPY as text, as psql does
  when the server refuses the COPY.
- `\watch` runs the query again and again, and reads the data each time. Both
  sides read it once.
- psql reads no data in pipeline mode (from 18). Neither side models pipeline
  mode, but `\sendpipeline` reads no data.
- psql reads the rest of the line after the data, with the lexer state at the
  end of that line, so a string that is open there continues after the data.
  pgtk-psql lexes each segment between two data regions alone, so the string
  ends at the data. No document of the corpus has this.

The interpolation events come from psql's own `get_variable` callback. That
matters: the oracle reports what the scanner recognised, not what a second
reading of the text suggests, so a colon that psql swallowed inside a string
or a dollar-quoted body can never appear as an interpolation.

### The PostgreSQL tree

`pgtk-psql-oracle` reads its tree from the shared PostgreSQL cache of the
machine, `build-pg.sh`'s own default (ADR 0012): `<cache>/<ref>-<commit>`,
where the cache is `$PG_SQL_POSTGRES_CACHE`, else
`${XDG_CACHE_HOME:-~/.cache}/pgtk-sql/postgres`. The tree is keyed by the
pinned commit, so one debug build, one release build, every checkout and
worktree, `pgtk-oracle` and a manual `pgtk-oracle/scripts/build-pg.sh <feature>`
all share it, and each pinned release is built once per machine.
`build-pg.sh` takes a lock on the tree, so the two build scripts can run at
the same time.

## Adaptations, and what each one costs

psql reads one line at a time and resets the query buffer after every
submission. pgtk-psql renders one SQL string for a whole document (ADR 0007).
Three adaptations make the two comparable. Each is deliberate, and each is
recorded here because it bounds what the oracle can prove.

1. **The SQL text of the whole document is one input.** The flex rules of
   `psqlscan.l` do not read line boundaries, so the lexing is the same. psql's
   line-at-a-time reading does bound how far the arguments of a backslash
   command can run, so the shim reads a command and its arguments from the
   rest of its line only, which is where psql's own input buffer ends. SQL
   lexing then continues where the command ends, on the same line when a
   `\\` ends the command.
2. **The query buffer is never reset.** The `{whitespace}` rule of
   `psqlscan.l` does not echo while the buffer is empty, so psql drops the
   leading trivia of every statement. The shim seeds the buffer with one byte
   and removes it at the end, so the trivia survives and the forwarded text
   can be compared byte for byte with pgtk-psql's rendering. For the same
   reason the shim keeps the text of an inactive `\if` branch in the buffer
   and reports the range that psql discards; the test removes it.
3. **Every known command succeeds.** psql drops the rest of the line after a
   command that fails when it runs. The shim runs no command, so it reads every
   known command as one that succeeds, as pgtk-psql does. It does not model
   the hooks of psql's special variables either.

Two boundaries are not `PSQL_CMD_SEND` in `command.c`, and [`classify`] still
names them: `\watch` runs the query buffer and resets it, so it is
`Command::Send`, and `\r` clears it, so it is `Command::Discard`.

The oracle quotes a bound value itself, mirroring `PQescapeLiteral` and
`PQescapeIdentifier` (`fe-exec.c`, `PQescapeInternal`), because those need the
client encoding and the `standard_conforming_strings` of a live connection and
the oracle has none. The oracle therefore proves where psql substitutes, not
how libpq escapes; pgtk-psql's own tests cover the escaping.

## The differential

`pgtk-psql/tests/psql_oracle.rs` reads every document with both sides. It needs
the `psql-oracle` feature, exactly as pgtk-sql's differential needs
`postgres-oracle`:

```sh
cargo test -p pgtk-psql --no-default-features --features pg17,psql-oracle
```

### The corpus

`baselines/psql-oracle/corpus.tsv` is the PostgreSQL 17.9 regression SQL
files, frozen by Git blob ID exactly as the SQL differential corpus is
(`docs/differential-baseline.md`). The tests read each blob from the
`vendor/postgres` object database, not from the checked-out tree, so a move of
an oracle pin does not change the corpus.

One of the 226 files is left out. `collate.windows.win1252.sql` is WIN1252,
not UTF-8. psql lexes it with a WIN1252 client encoding, where
`psqlscan_prepare_buffer` substitutes `0xFF` for the trailing bytes of every
multibyte character (`psqlscan_int.h`); pgtk-psql reads a Rust `&str`, so the two
cannot be compared over it.

Beside the corpus are the shaped documents in `shaped_documents()`: the
constructs pgtk-psql's own tests cover — every interpolation form, every lexical
shell that swallows a colon, the send and discard commands, the number forms
whose extent moved in 15, and the unterminated regions — read here by psql
rather than by a written expectation.

### What is compared

Six facts, for each document, unbound and then with every variable psql
recognised bound to an inert value:

- the text forwarded to the server;
- the extent and name of each interpolation in SQL text;
- the extent of each query-buffer boundary (pgtk-psql's
  `Rendered::boundaries`);
- the extent of each backslash command and of each of its arguments;
- the extent of the rows and of the `\.` line of each block of COPY data; and
- whether the document ends with a complete buffer (`Rendered::is_complete`).

The oracle's text is composed the way pgtk-psql composes its own: a send command
becomes the `;` that separates the two statements it stands between, a
discard command (`\r`, and the commands of 18) removes the buffer it ends,
keeping the whitespace before it, and the text of an inactive `\if` branch goes
where psql discards it. An interpolation inside a command argument is part of
the command, so it is not in the second list.

### The pinned outcomes

`baselines/psql-oracle/<feature>.json` records, for each target version, the
oracle pin, the corpus identity, the document count, and every document that
does not agree with its class. A document that agrees and is listed, or
disagrees and is not, fails the test, so the list cannot go stale in either
direction.

Regenerate after a change of pin, of oracle or of the psql grammar, and review
the diff:

```sh
cargo test -p pgtk-psql --no-default-features --features pg17,psql-oracle \
  --test psql_oracle -- --ignored regenerate_psql_oracle_baseline --nocapture
```

### The gap classes

An **expected psql gap** is a document where pgtk-psql does not agree with psql.
Every class is a deliberate limit of what pgtk-psql models, and a disagreement
that fits none of them is `unclassified`, which fails the test.

| Class | Why |
|---|---|
| `unknown-command` | The document holds a name that is not a command of the target version. psql reports an error and drops the rest of the line. pgtk-psql keeps the name only, as `MetaCommand::Unknown`, and forwards it, so that the server SQL shows the error. Before 15 `\getenv` is unknown, so each corpus file that uses it is here. |
| `refused` | pgtk-psql refuses a document for any other reason. The list is empty at every target version. |

The classes `unmodelled-meta-command` and `send-command-arguments` are gone.
pgtk-psql reads every meta-command with its arguments now (freshtonic/pgtk#42,
ADR 0013). The classes `boundary-suppressed-by-nesting`, `open-lexical-region`
and `copy-data` are gone too. pgtk-psql keeps psqlscan.l's parenthesis and
`BEGIN` depths and tells where the buffer is open at end of input
(freshtonic/pgtk#6), and it reads the COPY data as psql does
(freshtonic/pgtk#9).

### Outcomes by target version

| Feature | Documents | Agree | `unknown-command` |
|---|---|---|---|
| `pg14` | 364 | 332 | 32 |
| `pg15` | 364 | 356 | 8 |
| `pg16` | 364 | 356 | 8 |
| `pg17` | 364 | 357 | 7 |
| `pg18` | 373 | 366 | 7 |
| `pg19-beta` | 373 | 366 | 7 |

Before freshtonic/pgtk#42, 104 or 105 documents of each version were
`unmodelled-meta-command` gaps and 182 documents agreed at `pg17`. Before
freshtonic/pgtk#6 and freshtonic/pgtk#9, each version had 22 `copy-data`
gaps, 8 `boundary-suppressed-by-nesting` gaps and 4 `open-lexical-region` gaps
(6 at `pg14`), with 327 documents (336 from 18).

Two corpus files were `copy-data` gaps and are `unknown-command` gaps now.
`copyselect.sql` line 44 is `copy (select * from test1) from stdin;`, which
the server refuses. psqlscan.l reads words, not grammar, so psql reads the
lines after it as COPY data, up to the first `\.` line, at line 92. The second
data block of line 90 is then psql text, where `\.` is an unknown command.
`psql.sql` holds `\lo` with arguments, which is an unknown command.

These differences across the six have a cause:

- `pg18` and `pg19-beta` have nine more documents, the pipeline and
  prepared-statement commands of 18. All nine agree.
- 37 shaped documents test the query-buffer boundaries and the COPY data of
  freshtonic/pgtk#6 and freshtonic/pgtk#9. All of them agree at each version.
- Before 17 a vertical tab does not end a command name in `psqlscanslash.l`,
  so `\g<VT>x` is one unknown command rather than `\g` with an argument
  (`docs/research/psql-14-19-syntax-changes.md`, item A4). The shaped document
  `name/vertical-tab` is an `unknown-command` gap before 17 and agrees from 17.
- `pg14` has no `\getenv` (added in 15), so 18 corpus files that use it (and
  hold no COPY data) and `shaped/name/getenv` are `unknown-command` gaps.

## What the oracle found

**`\gdesc` was missing, and is fixed here.** It returns `PSQL_CMD_SEND` in
every target version (`REL_17_11:command.c` 1570), so it ends the query buffer
and its text goes to the server. pgtk-psql kept it as an unparsed
meta-command. It is now a `SendCommand`, and `pgtk-psql/tests/psql.rs` records
the corrected behaviour.

**An unterminated `E'...'` was re-read, and is fixed here.** ADR 0007 said
that pgtk-psql refuses a document that reaches end of input inside an open
lexical region, so that the region is never reinterpreted as ordinary text.
The oracle found a case where that did not hold. `SELECT 1e'\' :v` at `pg14`:

- psql gives the `e` back (`realfail1`), so `e'` opens an escape string, `\'`
  is an escaped quote, and the document ends inside the string. `:v` is
  string content and psql interpolates nothing.
- pgtk-psql found no `EscapeString` token, fell back to `e` as a word and
  `'\'` as a plain string, and then read `:v` as an interpolation and
  substituted it.

The quoted forms are plain `#[lex(pattern = ...)]` tokens, and an unmatched
pattern simply does not match, so the lexer read the same bytes as shorter
tokens. Each quoted form now also has an open form, `SqlAtom::OpenQuoted`:
the same pattern without its closing quote. Where the closing quote follows,
the closed form is one character longer and wins, so the open form wins only
at end of input, which is exactly psqlscan.l's `<<EOF>>` case
(freshtonic/pg-sql#89). `DollarString` and `BlockComment` reach the same
result as closed matchers with `unterminated = to_end_of_input`
(freshtonic/recursa#131).

So the documents that end inside an open region disagreed on `complete`
only. `Rendered::is_complete` now tells that the buffer is open there, and
they agree (freshtonic/pgtk#6).

**A `;` in parentheses or in a `BEGIN ... END` body was a boundary, and is not
now.** psqlscan.l ends the buffer at a `;` only when its parenthesis depth and
its `BEGIN` depth are zero, and it counts `BEGIN`, `CASE` and `END` only in a
`CREATE [OR REPLACE] {FUNCTION|PROCEDURE}` (`psqlscan_track_identifier`).
`pgtk-psql/src/scan_state.rs` keeps the same state over the tokens, and
`Rendered::boundaries` gives only the boundaries that psql has. Eight
documents of each version disagreed for this reason (freshtonic/pgtk#6).

**COPY data was read as psql text, and is not now.** Both sides read the
lines after a `COPY ... FROM STDIN` as SQL text, where `\N` and `\.` are
unknown commands. 22 documents of each version disagreed for this reason. Both
sides now read the data as psql does (freshtonic/pgtk#9).

The rest of the gaps are the documented limit: at `pg17`, 7 documents hold an
unknown command.

**Arguments were read to the end of the line, and are not now.** The oracle
used to resume SQL lexing at the line break after a command name. That was
wrong for `\x \x`, for `\set a b \\ SELECT 1;` and for every command whose
arguments end before the line does. The oracle now reads the arguments with
`psql_scan_slash_option`, as `command.c` does, and pgtk-psql agrees with it on
the extent of every command and argument of the corpus.

**`\watch` and `\r` end the query buffer.** Neither returns `PSQL_CMD_SEND`,
so the table of [`classify`] left them out. `\watch` runs the buffer and
resets it, and `\r` clears it. `psql.sql` uses both, so the table now names
them, and pgtk-psql renders `\watch` as a send command and `\r` as a discard.

**An inactive branch restores the lexer state.** `psql.sql` has `(bogus` in an
inactive branch. psql restores the parenthesis depth when it discards the
branch (`discard_query_text`), so the `;` after the block ends the statement.
The oracle does the same.
