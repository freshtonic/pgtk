# pgtk-oracle

**PostgreSQL's real parser, as the judge of pgtk-sql.**

pgtk-oracle builds the raw parser (`gram.y` and `scan.l`) of a pinned PostgreSQL release from the
vendored source, and links it into Rust. The tests of `pgtk-sql` give each statement to both
parsers and compare the answers. If PostgreSQL and pgtk-sql do not agree, the test fails.

pgtk-oracle is a development crate of [pgtk](https://github.com/freshtonic/pgtk). It is not
published.

## What it gives

- `parse_ok(sql)`: does PostgreSQL's raw parser accept this SQL?
- `parse_error(sql)`: the error message of the raw parser, or `None` if it accepts the SQL.
- `parse_equal(a, b)`: do two SQL texts give the same parse tree?
- `node_to_string(sql)`: the parse tree, as PostgreSQL prints it.
- `POSTGRES_REF` and `POSTGRES_COMMIT`: the release that this build linked.

## The pins

`pins.tsv` is the one pin table of the repository. It gives the exact PostgreSQL release of each
version feature. The build script, `scripts/build-pg.sh`, CI and the tests all read it.

| Feature | Release |
|---|---|
| `pg14` | `REL_14_24` |
| `pg15` | `REL_15_19` |
| `pg16` | `REL_16_15` |
| `pg17` (default) | `REL_17_11` |
| `pg18` | `REL_18_6` |
| `pg19-beta` | `REL_19_STABLE` at `b73d13c` |

The parser is behind the `parser` feature, which is on by default. Without it, the crate has only
the pin constants, and the build script builds no PostgreSQL.

## Use

```sh
packages/pgtk-sql/scripts/fetch-postgres-pins
cargo test -p pgtk-sql --features postgres-oracle --test differential
```

The first build extracts the pinned commit from `vendor/postgres` and builds it. Later builds use
the cache. See `docs/differential-baseline.md` in `pgtk-sql`.

## Licence

MIT. The PostgreSQL source that it builds has the PostgreSQL licence.
