# pgtk-catalog-xtask

**The generators behind pgtk-catalog.**

pgtk-catalog-xtask makes the generated files of `pgtk-catalog`. It is a development crate of
[pgtk](https://github.com/freshtonic/pgtk). It is not published.

## Tasks

Run the tasks in `packages/pgtk-catalog`, where `.cargo/config.toml` defines the `xtask` alias.

```sh
# Make the Builtin enum (src/builtin/generated.rs) from the pg_type.dat of each supported
# version. The enum is the union, and each variant records its first version.
cargo xtask gen-builtins 14=<pg_type.dat> 15=<pg_type.dat> ... 19=<pg_type.dat>

# Make the built-in catalog of the version of a live server (fixtures/pg<N>.json.zst).
# For a PostgreSQL 19 server, turn on the pg19-beta feature:
#   cargo run -p pgtk-catalog-xtask --features pg19-beta -- fixture <postgres-url>
cargo xtask fixture <postgres-url> [--out <file>]
```

Do not edit the generated files by hand. Run the task again. See `docs/spec.md` in
`pgtk-catalog`.

## Licence

MIT.
