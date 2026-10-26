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
| `src/interfaces/libpq/libpq.a` | `PQExpBuffer` and the multibyte helpers. |
| `src/common/libpgcommon.a`, `src/port/libpgport.a` | The frontend runtime. |

`csrc/psql_oracle.c` is the shim. `src/lib.rs` turns its result into
[`Scan`], which reports the text psql forwards to the server, one event for
each interpolation, one for each `;` that submits the query buffer, one for
each backslash command with the name `psqlscanslash.l` read and the extent and
value of each of its arguments, one for each inactive `\if` branch whose query
text psql discards, and whether the document ended with a complete buffer.

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

Five facts, for each document, unbound and then with every variable psql
recognised bound to an inert value:

- the text forwarded to the server;
- the extent and name of each interpolation in SQL text;
- the extent of each query-buffer boundary;
- the extent of each backslash command and of each of its arguments; and
- whether the document ends with a complete buffer.

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
| `copy-data` | The document holds `COPY ... FROM STDIN` or `\copy ... from stdin`, and an unknown command. psql reads the data lines after the command itself, up to `\.`. Neither side reads them as data: both read them as psql text, where `\N` and `\.` are unknown commands, and the two read an unknown command differently (the next class). |
| `unknown-command` | The document holds a name that is not a command of the target version. psql reports an error and drops the rest of the line. pgtk-psql keeps the name only, as `MetaCommand::Unknown`, and forwards it, so that the server SQL shows the error. Before 15 `\getenv` is unknown, so each corpus file that uses it is here. |
| `boundary-suppressed-by-nesting` | psql suppresses the `;` boundary inside parentheses and inside a tracked `BEGIN ... END` block of a `CREATE FUNCTION` (`psqlscan.l` 650, `psqlscan_is_create_routine`). pgtk-psql tracks neither, because it renders one string for the whole document rather than splitting it into submissions, so it only ever has more boundaries than psql. The forwarded text is the same. |
| `open-lexical-region` | The document reaches end of input inside a string, a quoted identifier, a dollar-quoted body or a comment. psql asks for another line. pgtk-psql runs each such region to end of input as one opaque token, so the forwarded text and the interpolations agree, but it has no notion of an open query buffer (freshtonic/pg-sql#11), so every one of these documents disagrees on `complete` only. See "What the oracle found" (ADR 0007, freshtonic/recursa#131, freshtonic/pg-sql#89). |
| `refused` | pgtk-psql refuses a document for any other reason. The list is empty at every target version. |

The classes `unmodelled-meta-command` and `send-command-arguments` are gone.
pgtk-psql reads every meta-command with its arguments now (freshtonic/pgtk#42,
ADR 0013).

### Outcomes by target version

| Feature | Documents | Agree | `copy-data` | `unknown-command` | `boundary-suppressed-by-nesting` | `open-lexical-region` |
|---|---|---|---|---|---|---|
| `pg14` | 327 | 266 | 22 | 25 | 8 | 6 |
| `pg15` | 327 | 287 | 22 | 6 | 8 | 4 |
| `pg16` | 327 | 287 | 22 | 6 | 8 | 4 |
| `pg17` | 327 | 288 | 22 | 5 | 8 | 4 |
| `pg18` | 336 | 297 | 22 | 5 | 8 | 4 |
| `pg19-beta` | 336 | 297 | 22 | 5 | 8 | 4 |

Before freshtonic/pgtk#42, 104 or 105 documents of each version were
`unmodelled-meta-command` gaps and 182 documents agreed at `pg17`.

These differences across the six have a cause:

- `pg18` and `pg19-beta` have nine more documents, the pipeline and
  prepared-statement commands of 18. All nine agree.
- Before 17 a vertical tab does not end a command name in `psqlscanslash.l`,
  so `\g<VT>x` is one unknown command rather than `\g` with an argument
  (`docs/research/psql-14-19-syntax-changes.md`, item A4). The shaped document
  `name/vertical-tab` is an `unknown-command` gap before 17 and agrees from 17.
- `pg14` has no `\getenv` (added in 15), so 18 corpus files that use it (and
  hold no COPY data) and `shaped/name/getenv` are `unknown-command` gaps.
- `pg14` has two more `open-lexical-region` documents: `number/exponent-string`
  and `number/param-junk`. See below.

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

So every `open-lexical-region` document now disagrees on `complete` only:
`shaped/open/string`, `shaped/open/quoted-identifier`,
`shaped/open/dollar-quote` and `shaped/open/block-comment` at every version,
and `shaped/number/exponent-string` and `shaped/number/param-junk` at `pg14`.
From 15 the two number documents agree, because `1e` is one trailing-junk
token in both and no escape string is opened.

The rest of the gaps are the documented limits: at `pg17`, 22 documents hold
COPY data, 5 hold an unknown command, 8 hold a `;` psql suppresses inside
parentheses or a `BEGIN ... END` block, and 4 end inside an open region.

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
