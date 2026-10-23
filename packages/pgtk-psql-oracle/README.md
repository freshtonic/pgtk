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
- one event for each backslash command, with its name as `psqlscanslash.l` reads it,
- a final event that tells if the script ended with a complete query buffer.

The version feature (`pg14` ... `pg19-beta`, default `pg17`) selects the release from
`pgtk-oracle/pins.tsv`.

## Use

```sh
cargo test -p pgtk-psql --features psql-oracle
```

psql reads one line at a time, and it clears the query buffer after each send. The oracle scans
the whole script as one input and never clears the buffer, so that its text and the text of
pgtk-psql are comparable. `csrc/psql_oracle.c` and `docs/psql-oracle.md` in `pgtk-sql` record
these two changes.

## Licence

MIT. The PostgreSQL source that it builds has the PostgreSQL licence.
