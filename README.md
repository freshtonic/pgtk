# pgtk

**PostgreSQL's own understanding of SQL, as Rust libraries.**

pgtk parses SQL the way PostgreSQL parses it. It knows the database the way the system catalog
knows it. It finds the type of each expression, the columns of each result and the error of each
bad statement, the way the server finds them. And it does all of this in your process, with no
server and no network round trip.

```text
            ┌────────────┐
  SQL ────▶ │  pgtk-sql  │──── Statement ────┐
            └────────────┘                   ▼
                                     ┌───────────────┐
                                     │ pgtk-analyze  │──▶ types, columns, errors
                                     └───────────────┘
            ┌──────────────┐                 ▲
  server ─▶ │ pgtk-catalog │──── Catalog ────┘
            └──────────────┘
```

## Why pgtk?

Linters, migration checkers, query builders, language servers, proxies and code generators must
all know what a SQL statement means. Today, a tool gets this answer in one of four ways:

- **Regular expressions and string checks.** These are fast, but they fail on a comment or a
  dollar-quoted string at the wrong position.
- **A generic SQL parser.** It parses "most" PostgreSQL. It does not parse all of it, and its
  AST, shared by all SQL dialects, can hold SQL that PostgreSQL rejects.
- **Bindings to PostgreSQL's C parser** (pg_query). The parse is exact, but you build and link
  C, and the tree is one large enum where any node can hold any other node. There is no type
  information and no analysis.
- **A live server.** It gives the correct answer, but you must start, migrate and connect to a
  database for each check.

pgtk gives a fifth way: **the answer of the server, without the server.**

| | pgtk | pg_query.rs | sqlparser-rs |
|---|---|---|---|
| Accepts exactly what PostgreSQL accepts | ✅ for each version, 14 to 19 beta | ✅ for one version | ❌ |
| Pure Rust | ✅ | ❌ links C | ✅ |
| AST types that match the PostgreSQL grammar | ✅ | ❌ one node enum | ❌ shared by all dialects |
| Visitor | ✅ | ❌ a flat list of nodes | ✅ |
| `Arbitrary` values that are always valid SQL | ✅ | ❌ | ❌ |
| Minimum PostgreSQL version of a statement | ✅ | ❌ | ❌ |
| Types, result columns and errors (parse analysis) | ✅ | ❌ | ❌ |
| Fingerprinting and normalisation | ❌ not yet | ✅ | ❌ |

## The proof is in the numbers

pgtk does not make guesses about PostgreSQL. It tests against PostgreSQL. Each crate compares its
results with PostgreSQL itself, on PostgreSQL's own regression suite:

| | Result |
|---|---|
| **Parser parity** | **43,474** regression-suite statements, **0 differences** from PostgreSQL's raw parser, on each of PostgreSQL **14, 15, 16, 17, 18 and 19 beta** |
| **Analysis parity** | **41,732** statements get the same types, columns or error as a live PostgreSQL 17 server, with **0 disagreements** and **0 unsupported** statements. Most of the 818 known divergences are errors that PostgreSQL raises after parse analysis, for example on bad input to a type. These are out of scope. |
| **Speed** | **35 MiB/s**: more than **2×** the speed of `sqlparser` on the same statements (40 MiB/s with the flat AST) |

The parser and analysis numbers come from pinned baselines that `cargo test` checks. The test
fails if a single answer changes. The speed numbers come from the `parse` benchmark of pgtk-sql
on the regression corpus.

## The crates

### 🧩 [`pgtk-sql`](packages/pgtk-sql): the parser

A PostgreSQL SQL parser that agrees with PostgreSQL on each statement. It accepts what
`gram.y` accepts, and it rejects what `gram.y` rejects. A Cargo feature selects the PostgreSQL
version, from 14 to 19 beta.

- **The AST is the grammar.** A struct is a sequence of grammar parts, an enum is a choice
  between them, and each list says if it can be empty. A value that PostgreSQL cannot parse
  cannot exist, so `Arbitrary` gives only valid SQL, and `match` covers each form.
- **Source spans** on each node, and a visitor for each AST type.
- **A flat AST**: one contiguous store for each parse, for tools that must be fast.
- **A minimum version for each statement.** pgtk-sql tells you that a statement needs
  PostgreSQL 16, and it shows the clause that causes this.
- **A formatter and railroad diagrams** of the grammar.

### 🗂️ [`pgtk-catalog`](packages/pgtk-catalog): the catalog

A semantic model of the system catalog of one PostgreSQL database: schemas, types, tables,
columns, functions, operators and casts. Load it from a live server once, then use it with no
server.

- **Not a copy of the `pg_*` tables.** It is a model with typed handles and name resolution.
- **Bring your own driver.** It has adapters for `postgres` and `tokio-postgres`, and a small
  trait for each other driver.
- **A built-in PostgreSQL 17 catalog**, for tests and for tools that work offline.

### 🔬 [`pgtk-analyze`](packages/pgtk-analyze): the type checker

PostgreSQL parse analysis in Rust. It takes a statement from pgtk-sql and a catalog from
pgtk-catalog. It then does what PostgreSQL's `parse_analyze` does: it resolves each name,
operator, function and type with the same rules.

- **The Describe result, before a server sees the query**: the type, name and source column of
  each result column, and the type of each parameter.
- **The error of the server**, with the same SQLSTATE, message and cursor position.
- **The full language**: joins, CTEs (recursive CTEs too), window functions, grouping sets,
  `MERGE`, `ON CONFLICT`, `RETURNING`, SQL/JSON, `JSON_TABLE`, collations and more.
- **Version checks.** It analyses for the server version that the SQL will run on.

### 💬 [`pgtk-psql`](packages/pgtk-psql): the psql client language

psql scripts contain more than SQL. They contain `:variable` interpolation, `:'quoted'` values and
`\g` commands. pgtk-psql reads a psql script, does the substitutions and gives SQL for the server.
A source map links each byte of that SQL to the byte of the script that it came from. Thus an
error in the SQL can point to the line that the user wrote.

pgtk-psql is not on crates.io yet.

## What you can build

- **Migration linters** that find a slow `ALTER TABLE` or a column that does not exist, before
  the migration runs.
- **Typed query macros** that know the result types at compile time, with no database in CI.
- **Language servers** that show the type of each expression and the error of each bad query in
  the editor.
- **Database proxies** that read, route or rewrite the queries that pass through them.
- **Version checks** that stop SQL that needs PostgreSQL 17 before it goes to a PostgreSQL 15
  server.
- **Code generators** that make Rust types from the result columns of a query.

## Quick start

```toml
[dependencies]
pgtk-analyze = "0.1"
pgtk-catalog = { version = "0.1", features = ["fixture"] }
```

```rust,ignore
use pgtk_analyze::{AnalyzeError, Analyzer, Session, Settings, TargetVersion};

let catalog = pgtk_catalog::fixture::pg17();
let settings = Settings::new(&catalog, TargetVersion::Pg17, ["public"]).unwrap();
let analyzer = Analyzer::new(&catalog, settings);
let session = Session::new();

// What will the server send back?
let analysis = analyzer.analyze_sql("SELECT relname, relpages * 8192 FROM pg_class", &[], &session)?;
for column in &analysis.result.columns {
    println!("{}: {}", column.name, catalog[column.ty].name);
}
// relname: name
// ?column?: int4

// What will the server say about a typo?
if let Err(AnalyzeError::Diagnostic(error)) =
    analyzer.analyze_sql("SELECT relname FROM pg_class WHERE relnmae = 'x'", &[], &session)
{
    println!("{} {} at {:?}", error.sqlstate, error.message, error.position);
    // 42703 column "relnmae" does not exist at Some(36)
}
```

The parser is slow to build the first time: its build script generates the grammar. Add the
[build-override profile](packages/pgtk-sql/README.md#build-time) to your workspace to make it
approximately 8 times faster.

## How pgtk stays correct

The parity numbers are not a one-time result. Each test run checks them again.

- **Oracles.** `pgtk-oracle` links the real raw parser of PostgreSQL, and `pgtk-psql-oracle`
  links the real psql lexer. Both come from the PostgreSQL source in `vendor/postgres`, at a
  pinned release for each version.
- **Real servers.** `pgtk-test-server` builds each pinned PostgreSQL release from source and
  starts a temporary server for each test. No Docker, no installed PostgreSQL.
- **Pinned baselines.** Each result is pinned in a baseline file. A change in either direction
  stops the test, so a regression cannot pass unseen, and an improvement must be pinned
  intentionally.
- **Gates.** The merge gate tests each change. The release gate tests each PostgreSQL version
  before a release.

## Built on recursa

pgtk-sql and pgtk-psql are generated by [recursa](https://crates.io/crates/recursa), an
AST-first parser generator for Rust. You write the AST as Rust types. recursa makes the lexer,
the LR parser, the span tracking and the flat AST from these types. This is why the full
PostgreSQL grammar is Rust code that you can read.

## Status

pgtk is before version 1.0. The parser and the analyzer are complete for the regression suite of
each supported version. The public API can change between minor versions.

## Contributing

pgtk builds with Cargo and with Buck2. [`DEVELOPMENT.md`](DEVELOPMENT.md) tells you how to get
the PostgreSQL sources, build, test and run the gates. [`CONTEXT-MAP.md`](CONTEXT-MAP.md) gives the
domain language of each crate.

## Licence

MIT. See [`LICENSE`](LICENSE).
