# pgtk-psql

**psql scripts, understood the way psql understands them.**

A psql script is not SQL. It contains `:name` interpolation, `:'quoted'` and `:"identifier"`
values, and send commands such as `\g` and `\gset`. psql changes all of this into SQL before
the server sees it. pgtk-psql does the same change, with the same rules, and it records where
each byte of the result came from.

pgtk-psql is part of [pgtk](https://github.com/freshtonic/pgtk), the PostgreSQL toolkit for Rust.
It is not on crates.io yet.

## Why pgtk-psql?

- **psql's rules, not approximate rules.** pgtk-psql recognises what `psqlscan.l` recognises,
  and nothing more. A colon in a string, a comment or a dollar-quoted body is never
  substituted. A variable that is not set stays as it is, as psql leaves it.
- **Errors that point to the script.** A source map links each byte of the rendered SQL to the
  byte of the script that it came from. A parse error in the SQL can thus point to the line that
  the user wrote.
- **A clean split.** psql is a client program, not a SQL dialect. PostgreSQL's grammar contains
  no psql construct, so pgtk-psql is a separate crate and `pgtk-sql` stays exactly the server
  grammar.
- **Checked against psql.** `pgtk-psql-oracle` links psql's own lexer from the PostgreSQL source.
  The tests compare the two on each run.

## Quick start

The pipeline has three steps: parse the psql script, render it to server SQL, then parse that
SQL with `pgtk-sql`.

```rust,ignore
let mut variables = pgtk_psql::Variables::new();
variables.set("table", "users");

let rendered = pgtk_psql::render(r#"SELECT * FROM :"table" WHERE id = :'id';"#, &variables)?;
assert_eq!(rendered.sql(), r#"SELECT * FROM "users" WHERE id = :'id';"#);

// Only now parse the result as PostgreSQL SQL.
let document = rendered.parse_sql().map_err(|error| error.to_string())?;
```

`id` is not set, so `:'id'` stays in the text, as psql leaves it.

## Features

| Feature | What it does |
|---|---|
| `pg14` ... `pg19-beta` | Selects the PostgreSQL version of `pgtk-sql` and of the psql lexer. Use exactly one. The default is `pg17`. |
| `psql-oracle` | Development only: builds psql's lexer for the differential tests. |

## What it does not do yet

psql meta-commands, such as `\set`, `\d`, `\if` and `\copy`, are not parsed. A meta-command that
is not a send command stays in the SQL text, as it is. The server cannot parse such a document,
so the result is an error, never a silent change.

## Licence

MIT.
