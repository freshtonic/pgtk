# One build supports PostgreSQL 14 to 19 beta, and the server selects the version

Status: accepted

`pgtk-catalog` supports PostgreSQL 14, 15, 16, 17, 18, and 19 beta in one build.
The Loader reads `server_version_num` before it runs any other query, so it
knows the version of each server that it loads, and each Catalog records that
version in `metadata().server_version`. We do not select the version with
Cargo features, as the sibling `pgtk-sql` does (its ADR 0009). The model is the
union of the supported versions: a fact that only newer servers have is an
`Option` or an enum variant, and its documentation names the first version
that has it. The only version feature is `pg19-beta`, which is additive: it
lets the Loader accept a 19 server and adds the 19 fixture. It exists because
the 19 catalog can still change before 19.0, not because the model differs
by build.

## Considered Options

- **Mutually exclusive version features, as in `pgtk-sql`.** `pgtk-sql` chose them
  because its grammars are not additive (PostgreSQL removes syntax and changes
  keywords and lexing) and because each generated grammar is about 25 MB.
  Neither reason applies here. The catalog model is nearly additive: the
  known changes between versions add columns, rows, and letters, and remove
  none that the model holds. The version-specific part is a few lines of SQL.
  Exclusive features would also stop one process from loading servers of
  different versions, which the Oracle harness and `scripts/check-queries.sh`
  do, and they would add a second feature that each consumer must keep equal
  to its `pgtk-sql` feature, or its build fails.
- **Additive `since-pgN` features that add fields to the model.** A consumer
  that loads a 14 server with a build for 18 would get fields that the server
  cannot fill, and a consumer built for 14 could not load an 18 server. An
  `Option` says the same thing at run time, with no build difference.
- **A separate catalog type for each version.** This multiplies the public API
  by six to express differences that are a few fields and variants.

## Consequences

- **Model.** A change that a newer version brings is additive in the model.
  Examples that we expect from PostgreSQL 18 are a `NotNull` constraint kind,
  a virtual kind of generated column (so `Column.generated` becomes an enum),
  and a flag for a constraint that is `NOT ENFORCED`. The first live run
  against each version decides the exact list. (The first runs, against
  18.6 and the pinned 19 commit, found these three, plus the `conperiod` flag
  of a temporal key, and the built-in types `oid8` and `regdatabase` in 19.
  The model now holds each of them.) Public enums become
  `#[non_exhaustive]` in one breaking release, so that later versions can add
  variants without more breaks. A server that does not have a fact gives
  `None`, or never gives the variant.
- **Queries.** `queries::sql` takes the server version. A query keeps one text
  for every version when it can. It gets a variant for a range of versions
  only when a column does not exist in older servers: a `CASE` on the version
  cannot hide such a column, because the server resolves column names before
  it runs the query. The extraction marker names the range, and
  `scripts/check-queries.sh` runs each variant only on its versions.
- **Built-in types.** `xtask gen-builtins` generates the union of the
  versions, and each `Builtin` records the first version that has it. The
  check for missing built-ins requires only the built-ins of the version of
  the loaded server.
- **Node trees.** The reader of `pg_node_tree` text accepts the labels of each
  supported version, as it already accepts both `ARRAY` and `ARRAYEXPR`. The
  live test that requires a type for each default of each stock function
  finds a new label.
- **Fixtures.** There is one fixture for each version: `fixture::pg14()` to
  `fixture::pg18()`, and `fixture::pg19_beta()` behind `pg19-beta`.
  `Catalog::FORMAT_VERSION` is independent of the PostgreSQL version.
- **Tests.** The CI matrix and the default versions of
  `scripts/check-queries.sh` are 14 to 18, and 19 beta when `pg19-beta` is on.
  19 beta is pinned to one image tag until 19.0, as `pgtk-sql` pins it to one
  commit. At 19.0 the `pg19-beta` feature goes away and 19 becomes an ordinary
  supported version.
- **Consumers.** A consumer that also uses `pgtk-sql` builds that crate for one
  `pgtk_sql::TARGET_VERSION`. The consumer, not `pgtk-catalog`, rejects a Catalog
  whose major version is not that target, and selects the matching fixture.
  `pgtk-catalog` has no compile-time version setting for it to keep in step.
- **Order of work.** Version 18 comes first, because a live 18 server shows
  the real differences. Version 19 beta follows, behind its feature.
