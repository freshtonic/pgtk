# Notes

Things to do later, outside the current slices.

## Upstream the corpus gaps to PostgreSQL

When pg-analyze is complete, analyse the changes pg-sql needed to support
it. `docs/upstream-work.md` lists them as they were requested; the pg-sql
history holds the fixes. Each of them was a form gram.y accepts that pg-sql
did not parse, or parsed differently, and that no statement in PostgreSQL's
regression corpus exercised, because pg-sql's differential tests run that
corpus against `raw_parser`. The list is therefore a list of syntax the
corpus does not cover. Turn it into regression test additions and submit
them upstream to PostgreSQL as a patch.
