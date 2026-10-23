# pgtk-psql-oracle

**psql's real lexer, as the judge of pgtk-psql.**

pgtk-psql-oracle builds psql's own lexers, `psqlscan.l` and `psqlscanslash.l`, from the pinned
PostgreSQL release, and links them into Rust. The tests of `pgtk-psql` give each script to both
and compare the answers.

pgtk-psql-oracle is a development crate of [pgtk](https://github.com/freshtonic/pgtk). It is not
published.

## What it gives

`scan(source, variables)` gives, for one psql script:

- the SQL text that psql sends to the server,
- one event for each variable interpolation, with its location, its quoting and where the value
  went in the SQL text,
- one event for each `;` that sends the query buffer,
- one event for each backslash command, with its name as `psqlscanslash.l` reads it, and the
  extent and value of each argument that `psql_scan_slash_option` reads for it,
- one event each time psql discards the query text of an inactive `\if` branch,
- a final event that tells if the script ended with a complete query buffer.

The oracle reads the arguments of each command as `command.c` reads them. It applies `\set`,
`\unset`, `\if`, `\elif`, `\else` and `\endif`. It runs no other command, and it never runs a
backquoted shell command.

The version feature (`pg14` ... `pg19-beta`, default `pg17`) selects the release from
`pgtk-oracle/pins.tsv`.

## Use

```sh
cargo test -p pgtk-psql --features psql-oracle
```

The `scanner` feature (on by default) builds the lexers. Without it the crate has only the pin
constants, so pgtk-psql can have it as a dev-dependency that builds no PostgreSQL.

psql reads one line at a time, and it clears the query buffer after each send. The oracle scans
the SQL text of the whole script as one input and never clears the buffer, so that its text and
the text of pgtk-psql are comparable. It reads the arguments of a command from the rest of its
line only, as psql does. `csrc/psql_oracle.c` and `docs/psql-oracle.md` in `pgtk-sql` record
these two changes.

## Licence

MIT. The PostgreSQL source that it builds has the PostgreSQL licence.
