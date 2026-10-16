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

## Target Versions after 17 and below 17

ADR 0005 lets the Target Version be 14 through 19. Only 17 is measured.

- Target Versions 18 and 19 apply the 17 rules. Audit the analysis changes
  after 17, as `docs/research/parse-analysis-differences-14-17.md` does for
  14 to 17, and add the version rules it finds.
- The Oracle harness and its baseline exist for 17 only. Add a baseline and
  a run for each other version. The server of each version is the Oracle of
  that Target Version.
- pg-catalog loads servers 14 through 17 only, so a Catalog from an 18 or 19
  server cannot be loaded yet.

## Reject syntax newer than the Target Version

When the Grammar Version is newer than the Target Version, Analysis rejects
each construct the Target Version cannot lex or parse, from pg-sql's Minimum
Version (ADR 0005). Two gaps remain, both waiting on pg-sql:

- pg-sql answers the Minimum Version on its arena parse only. `analyze_sql`
  parses the statement a second time for the check; `analyze`, which takes a
  flat parse from the caller, does not check. When pg-sql answers it on the
  flat form, check in `analyze`, drop the second parse, and drop the
  subquery-alias check in `subquery_ref`, which the Minimum Version covers.
- pg-sql reports six widenings too low (ADR 0005 lists them). Analysis can
  accept them for a Target Version that rejects them.
