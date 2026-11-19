# Supported PostgreSQL versions

`pgtk-proto` supports PostgreSQL 14, 15, 16, 17, and 18.

Every version runs the same live suite against the pinned release of the pgtk
workspace, which `pgtk-test-server` builds from the vendored source. The suite covers startup/protocol negotiation, cleartext, MD5,
SCRAM-SHA-256, SCRAM-SHA-256-PLUS over TLS, asynchronous backend messages,
extended query, error draining, and COPY IN/OUT.

PostgreSQL 14–17 negotiate a requested protocol 3.2 startup down to protocol
3.0. PostgreSQL 18 reports protocol 3.2. Both behaviours are asserted explicitly.

`cargo test` runs PostgreSQL 18. Run another version with:

```bash
PGTK_PROTO_POSTGRES_VERSION=14 \
  cargo test -p pgtk-proto --lib internal_tests::postgres_container
```
