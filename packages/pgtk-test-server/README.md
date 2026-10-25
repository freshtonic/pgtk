# pgtk-test-server

**A real PostgreSQL server for each test, with no Docker.**

pgtk-test-server builds the pinned PostgreSQL release of a version from the vendored source, once
for each machine. Then it starts a temporary server for a test, and it stops the server when the
test drops it.

pgtk-test-server is a development crate of [pgtk](https://github.com/freshtonic/pgtk). It is not
published.

## Why pgtk-test-server?

- **No setup.** A developer does not install PostgreSQL or Docker. `cargo test` builds what it
  needs.
- **Each version.** Start a server of PostgreSQL 14, 15, 16, 17, 18 or 19 beta from the same
  test.
- **No collisions.** Each server listens only on a Unix socket in a directory of its own, and it
  keeps its cluster in that directory. Servers share no port and no data directory, so any number
  of them can run at the same time.
- **Built once.** Each pinned release is built one time, into a shared cache. The tests of all
  checkouts and worktrees use it.

## Use

```rust,no_run
use pgtk_test_server::Server;

// A server of the version of the current version feature.
let server = Server::start();
let url = server.connection_string("postgres");

// Or a server of a given version.
let pg14 = Server::start_of("pg14");
```

`installation_of(feature)` gives the installation: its `bin` directory, and a `Command` for each
of its programs.

## Licence

MIT.
