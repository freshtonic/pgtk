# Security Policy

We take the security of pgtk, and of the tools that people build with it, seriously.
This document describes the supported versions, how to report a vulnerability, and the security
rules for contributors.

## Supported Software

This repository contains these published crates:

- `pgtk-sql`, the PostgreSQL parser
- `pgtk-catalog`, the model of the PostgreSQL system catalog
- `pgtk-analyze`, PostgreSQL parse analysis

**Security fixes are released for the latest release of each crate.** Reports are welcome for any
version, but the fixes go into the latest release. If you use an older version, upgrade to get
them.

All crates follow semantic versioning.

---

## Reporting a Vulnerability

If you think that you found a security vulnerability in pgtk:

🔒 **Report it privately on GitHub:
[Report a vulnerability](https://github.com/freshtonic/pgtk/security/advisories/new)**

**Do not open a public issue**, and do not disclose the problem publicly before we can examine it
and release a fix.

Include, where you can:

- A description of the vulnerability
- The steps to reproduce it, for example the SQL text that causes it
- The crate, the version and the PostgreSQL version feature
- The impact, or how someone could misuse it
- A suggested fix, if you have one

We will acknowledge your report within **48 hours**, and we will give you regular updates until
the problem is resolved.

---

## Disclosure and Response Policy

We follow a **coordinated disclosure** process:

1. **Report** the problem privately, through GitHub.
2. **Acknowledgement** within 48 hours.
3. **Assessment** of the severity, with CVSS.
4. **Fix** in a private fork, and a patch release.
5. **Coordinated disclosure**, with:
   - The patch release or releases
   - A GitHub security advisory and a RustSec advisory
   - Credit to the reporter (if you want it)

We will never take legal action against good-faith security researchers who follow this policy.

---

## Scope

pgtk often reads SQL that comes from users that it cannot trust, for example in a proxy, a linter
or a language server. These are **in scope**:

- Input that makes a published crate panic, overflow its stack, or use memory or time without
  limit
- A parse that accepts SQL which PostgreSQL rejects, or rejects SQL which PostgreSQL accepts,
  if a tool could make an unsafe decision from it
- An analysis result (a type, a column, a table or an error) that is different from the result of
  PostgreSQL, if a tool could make an unsafe decision from it
- Unsound code, or undefined behaviour, in a published crate
- Documentation or examples that could cause unsafe use

These are **out of scope**:

- The development crates, which are not published: `pgtk-oracle`, `pgtk-psql-oracle`,
  `pgtk-test-server`, `pgtk-test-features` and `pgtk-catalog-xtask`. We are still grateful for
  reports about them.
- Vulnerabilities in PostgreSQL itself, including the source in `vendor/postgres`. Report them to
  the [PostgreSQL Security Team](https://www.postgresql.org/support/security/).
- Social engineering, physical attacks, and attacks on GitHub or crates.io
- Attacks that need privileged access to developer machines or to CI

---

## Security Rules for Contributors

To keep the security of pgtk high, contributors MUST obey these rules.

### ⚙️ Safe code

- Do **not** add `unsafe` code to a published crate without discussion first. `pgtk-catalog`
  forbids it.
- Keep foreign function interfaces (FFI) in the oracle crates, which are not published.
- Never commit keys, secrets, tokens or credentials, also not for tests.

### 🛡 Dependencies

- Add a dependency only when it is necessary.
- Keep the dependencies up to date, and examine each new one.
- Each dependency of a published crate must have a licence that `deny.toml` permits.
- Use Rust for all new code.

### 🔍 Tests and review

- Add tests for each fix, and for the edge cases and the misuse of each change.
- Add input that caused a panic, or used time without limit, to the tests.
- Do not merge a change that makes a security control weaker or adds an unsafe default.

---

## Questions

For questions about security that are not a vulnerability report, open a
[discussion or an issue](https://github.com/freshtonic/pgtk/issues).

For vulnerability reports, use
[private vulnerability reporting](https://github.com/freshtonic/pgtk/security/advisories/new).

---

Thank you for helping to keep pgtk, and the tools that use it, secure.
