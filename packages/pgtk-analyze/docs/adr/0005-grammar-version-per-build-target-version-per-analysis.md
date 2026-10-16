---
status: accepted
---

# The Grammar Version is chosen for each build and the Target Version for each Analysis

pgtk-sql now selects one grammar for each build with mutually exclusive features (`pg14` … `pg19-beta`, pgtk-sql ADR 0009), and the Statement type differs between builds. A consumer can still run in front of an older server than its grammar: a pg18 build that analyses for a live 15 server must accept only what 15 accepts. We decided that pgtk-analyze has the same six features and forwards them to pgtk-sql, which fixes the Grammar Version, and that the caller still chooses the Target Version at run time. The Target Version must not be newer than the Grammar Version. Analysis rejects a Statement whose Minimum Version, as pgtk-sql reports it, is newer than the Target Version, with gram.y's own message where gram.y has one and `syntax error at or near` otherwise. This supersedes ADR 0003, and keeps its rule that the Catalog's own version never chooses the rules.

## Considered options

- One version for each build, no run-time Target Version. Rejected: a consumer with a newer build than its server could not enforce the server's rules.
- Allow a Target Version newer than the Grammar Version. Rejected: syntax removed after the Grammar Version would need its own checks, and syntax added after it cannot be parsed at all.
- pgtk-analyze keeps its own list of version-gated syntax. Rejected: pgtk-sql's `since-pgN` gates already hold that knowledge, and a copy would drift.

## Consequences

- Analysis is sound but not complete when the Grammar Version is newer: a newer grammar can reject SQL that the Target Version accepts (lexing of `123abc` before 15, keyword category of `json` from 17).
- Analysis is sound only as far as pgtk-sql's Minimum Version is. pgtk-sql reports six widenings too low (its `docs/minimum-version.md`, class 4): `REVOKE <name> OPTION FOR` and `GRANT … WITH <name> OPTION` (16), `ALTER OPERATOR … SET (name)` and `COPY … FORCE NOT NULL *` (17), `ANALYZE`/`VACUUM ONLY` and the foreign key `PERIOD` element (18). They are exceptions to the guarantee until pgtk-sql closes them; pgtk-analyze keeps no list of its own.
- Minimum Version is answered on pgtk-sql's arena parse, and Analysis reads the flat form. Until pgtk-sql answers it on the flat form, `analyze_sql` parses the arena form a second time for the check, and `analyze` does not check.
- Targets 18 and 19 apply the 17 rules until an audit of the analysis changes after 17 and an Oracle of each version exist. Their fidelity is not verified.
- With Cargo feature unification, a crate between a consumer and pgtk-analyze uses `default-features = false` and forwards the version features.
