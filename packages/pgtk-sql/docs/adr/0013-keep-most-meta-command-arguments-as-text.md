# Keep most meta-command arguments as text

Status: accepted

pgtk-psql reads each psql meta-command as `psqlscanslash.l` reads it
(freshtonic/pgtk#42). Each command is one token: the backslash, the name, and
the argument region that `exec_command` in `src/bin/psql/command.c` reads for
that command. `pgtk_psql::Command` divides the token into the name, the
arguments and a trailing `\\`, and `pgtk_psql::Argument::value` gives the value
that `psql_scan_slash_option` returns for an `OT_NORMAL` argument.

This record tells which commands have a meaning in pgtk-psql, and which keep
their arguments as text only.

## The decision

pgtk-psql gives a meaning to the commands that change the text that psql
sends to the server. All other commands keep their arguments as text.

| Commands | What pgtk-psql does |
|---|---|
| `\set`, `\unset` | `pgtk_psql::render` changes the variables of the script, in an active branch only. The variables then apply to the interpolations that follow. |
| `\if`, `\elif`, `\else`, `\endif` | `PsqlDocument::conditionals` finds the blocks and checks the nesting, with no evaluation. `render` evaluates each expression as `is_true_boolean_expression` does, and sends only the text of the active branches, as `discard_query_text` does. |
| `\g`, `\gx`, `\gset`, `\gdesc`, `\gexec`, `\crosstabview`, `\watch`; from 18 `\parse`, `\sendpipeline` | Each ends the query buffer and renders as one `;`. The arguments do not go to the server. |
| `\r`, `\reset`; from 18 the seven commands that end the buffer and send nothing | Each ends the query buffer, and `render` removes the text of the buffer. |
| Every other known command | The command and its arguments render as nothing. The arguments are text. |
| A name that is not a command | `MetaCommand::Unknown`: the name only, rendered as it stands. |

## Why

A consumer of pgtk-psql wants the server SQL of a script, and the source of
each byte of that SQL. Only the commands in the first four rows change that
SQL. psql runs every other command on the client: it writes output, opens a
file, changes a setting or a connection. pgtk-psql runs no command, so a
meaning for those arguments has no use in pgtk-psql yet. The text of each
argument, with its exact extent, is enough for a tool that wants to do more,
and `Argument::value` gives its psql value.

`\set` and `\if` must have a meaning. The regression corpus sets variables
with `\set` and uses them in SQL, for example
`\set filename :abs_srcdir '/data/agg.data'` and then
`COPY ... FROM :'filename'`. Without `\set`, the rendered SQL holds an
interpolation that psql replaces. `psql.sql` also uses `\if` blocks whose
inactive branches hold text that psql never sends.

## What pgtk-psql does not do

- It runs no shell command. An argument that holds a backquoted command has no
  value (`Argument::value` gives `None`). A `\set` with such a value changes
  nothing, and an `\if` with such an expression is false. `Rendered::unevaluated`
  gives the extent of each one.
- It does not model the hooks of psql's special variables, such as
  `AUTOCOMMIT` or `ON_ERROR_STOP`. psql changes or refuses some values of
  these variables.
- It does not read an included file (`\i`), and it does not stop at `\q`.
- It does not read the data lines after `COPY ... FROM STDIN` or
  `\copy ... from stdin`. psql reads them itself. pgtk-psql reads them as
  psql text, where `\N` and `\.` are unknown commands.
- An unknown command, and a command that fails when psql runs it, makes psql
  drop the rest of the line. pgtk-psql cannot know which command fails, so it
  reads every known command as a command that succeeds. It keeps the name of
  an unknown command only.
- `\g (options) file`: psql ends the option list at the first argument whose
  value ends with `)`. The token pattern ends it at a `|` argument, so an
  unusual `\g (a) x |y \\` reads the `|y` argument to the end of the line.
- Each name that starts with `d` is `\d` and each name that starts with `lo_`
  is a large-object command, as `exec_command` sends them on. psql rejects
  some of these names when it runs them.

## How it is checked

`pgtk-psql-oracle` reads the arguments of each command with the same
`psql_scan_slash_option` calls as `command.c`. It applies `\set`, `\unset` and
the conditional commands, and reports the extent of each command and each
argument. `tests/psql_oracle.rs` of pgtk-psql compares these facts with
pgtk-psql for the regression corpus and the shaped documents, for each target
version (`docs/psql-oracle.md`).

## How the patterns are made

The token patterns of `packages/pgtk-psql/src/ast.rs` follow the states of
`psqlscanslash.l`. `packages/pgtk-psql/scripts/meta-command-patterns.py`
makes them for each definition of `space` (with the vertical tab from 17).
Change the script, and copy its output into `ast.rs`.
