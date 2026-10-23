# pgtk-catalog-xtask

**The generators behind pgtk-catalog.**

pgtk-catalog-xtask makes the generated files of `pgtk-catalog`. It is a development crate of
[pgtk](https://github.com/freshtonic/pgtk). It is not published.

## Tasks

Run the tasks in `packages/pgtk-catalog`, where `.cargo/config.toml` defines the `xtask` alias.

```sh
# Make the Builtin enum (src/builtin/generated.rs) from PostgreSQL's pg_type.dat.
cargo xtask gen-builtins <pg_type.dat>

# Make the built-in catalog (fixtures/pg17.json.zst) from a live PostgreSQL 17 server.
cargo xtask fixture <postgres-url> [--out <file>]
```

Do not edit the generated files by hand. Run the task again. See `docs/spec.md` in
`pgtk-catalog`.

## Licence

MIT.
