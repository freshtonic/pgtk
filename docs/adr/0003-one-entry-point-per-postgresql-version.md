---
status: superseded by ADR-0005
---

# One analysis entry point per PostgreSQL major version, 14 through 17, over one rule set

pg-catalog loads Catalogs from PostgreSQL 14 through 17, and consumers run in front of all of them. An audit of the analysis source between those versions (`docs/research/parse-analysis-differences-14-17.md`) found the type-resolution core frozen and about eleven behavioural differences in total, almost all feature availability or diagnostic wording. We decided to support all four versions from one code base: the caller picks a Target Version through a per-version entry point, and the Analyzer applies a small set of version rules plus a feature-availability table. The Catalog's own version never chooses the rules, because a consumer may deliberately analyse for a different version than the server it snapshotted.

## Considered options

- Support 17 only and refuse other Catalogs. Rejected: blocks real consumers for a difference measured in a dozen switches.
- Choose rules from the Catalog's recorded version. Rejected: hides the choice from the caller and makes the same Statement analyse differently depending on where the Catalog came from.

## Consequences

- Diagnostic message text is versioned where the audit shows it changed, so the Oracle comparison for each version uses that version's wording.
- Annotations stay version-neutral. Fields PostgreSQL added to its `Query` in 16 and 17 are not mirrored, which keeps the future tree-level Oracle rendering from forking per version.
- Fidelity is tested against an Oracle of each version, not only 17.
