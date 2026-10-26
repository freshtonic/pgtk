# pgtk-psql

**psql scripts, understood the way psql understands them.**

[![crates.io](https://img.shields.io/crates/v/pgtk-psql.svg)](https://crates.io/crates/pgtk-psql)
[![docs.rs](https://img.shields.io/docsrs/pgtk-psql)](https://docs.rs/pgtk-psql)
[![MIT licence](https://img.shields.io/badge/licence-MIT-blue.svg)](https://github.com/freshtonic/pgtk/blob/main/LICENSE)

A psql script is not SQL. It contains `:name` interpolation, `:'quoted'` and `:"identifier"`
values, meta-commands such as `\set`, `\d` and `\copy`, send commands such as `\g` and `\gset`,
and `\if` blocks. psql changes all of this into SQL before the server sees it. pgtk-psql does the
same change, with the same rules, and it records where each byte of the result came from.

pgtk-psql is part of [pgtk](https://github.com/freshtonic/pgtk), the PostgreSQL toolkit for Rust.

## Why pgtk-psql?

- **psql's rules, not approximate rules.** pgtk-psql recognises what `psqlscan.l` and
  `psqlscanslash.l` recognise, and nothing more. A colon in a string, a comment or a
  dollar-quoted body is never substituted. A variable that is not set stays as it is, as psql
  leaves it.
- **Meta-commands as psql reads them.** Each meta-command is one token, with the arguments that
  psql reads for that command: up to a space or a backslash, the whole line for `\copy` and
  `\!`, and up to `\\` when SQL follows on the same line.
- **Query buffers as psql ends them.** A `;` inside parentheses or inside the `BEGIN ... END`
  body of a `CREATE FUNCTION` does not end the buffer, as in psql. `Rendered::boundaries` gives
  each place where psql sends the buffer, and `Rendered::is_complete` tells whether the script
  ends with an open buffer.
- **COPY data as psql reads it.** The data lines after `COPY ... FROM STDIN` or
  `\copy ... from stdin`, up to `\.`, are one `PsqlItem::CopyData` item. They are never read as
  SQL or as psql text, and they are not part of the rendered SQL.
- **Scripts that change their own variables.** `\set` and `\unset` change the variables for the
  rest of the script, and only the active branches of an `\if` block go to the server.
- **Errors that point to the script.** A source map links each byte of the rendered SQL to the
  byte of the script that it came from. A parse error in the SQL can thus point to the line that
  the user wrote.
- **A clean split.** psql is a client program, not a SQL dialect. PostgreSQL's grammar contains
  no psql construct, so pgtk-psql is a separate crate and `pgtk-sql` stays exactly the server
  grammar.
- **Checked against psql.** `pgtk-psql-oracle` links psql's own lexers from the PostgreSQL source.
  The tests compare the two on each run.

## Quick start

The pipeline has three steps: parse the psql script, render it to server SQL, then parse that
SQL with `pgtk-sql`.

```rust
let mut variables = pgtk_psql::Variables::new();
variables.set("id", "42");

let script = r#"
\set table users
\if :{?id}
SELECT * FROM :"table" WHERE id = :'id';
\else
SELECT * FROM :"table";
\endif
"#;

// 1. Parse the psql script.  2. Apply \set and \if, substitute, and render server SQL.
let rendered = pgtk_psql::render(script, &variables).expect("a valid psql script");
assert_eq!(rendered.sql().trim(), r#"SELECT * FROM "users" WHERE id = '42';"#);

// 3. Only now parse the result as PostgreSQL SQL.
let document = rendered.parse_sql().expect("valid PostgreSQL");
assert_eq!(document.statements().len(), 1);
```

A variable that is not set stays in the text, as psql leaves it.

## Meta-commands

`pgtk_psql::parse` gives the items of a script in source order. A meta-command is a
`PsqlItem::Command`, or a `PsqlItem::Terminator` when it ends the query buffer, as `\g` does.
`command()` divides it as psql does:

```rust
let document = pgtk_psql::parse(r"\pset null '(null)' \\ SELECT 1;").unwrap();
let pgtk_psql::PsqlItem::Command(command) = &document.items[0] else {
    panic!("the first item is the command");
};
let command = command.command();
assert_eq!(command.name(), "pset");
let arguments = command.arguments();
assert_eq!(arguments[1].text(), "'(null)'");
assert_eq!(
    arguments[1].value(&pgtk_psql::Variables::new()).as_deref(),
    Some("(null)")
);
assert_eq!(command.separator(), Some(r"\\"));
```

`PsqlDocument::conditionals` gives the `\if` blocks and the commands that do not nest, with no
evaluation.

## Features

| Feature | What it does |
|---|---|
| `pg14` ... `pg19-beta` | Selects the PostgreSQL version of `pgtk-sql` and of the psql lexer. Use exactly one. The default is `pg17`. |
| `psql-oracle` | Development only: builds psql's lexers for the differential tests. |

## What it does not do

pgtk-psql runs no command. It does not read an included file, run a shell command or a
backquoted argument, or connect to a server. A command name that psql does not know stays in the SQL text, so the
result is an error, never a silent change. `docs/adr/0013-keep-most-meta-command-arguments-as-text.md`
in `pgtk-sql` gives the details.

## Licence

MIT.
