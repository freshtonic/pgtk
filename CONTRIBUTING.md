# How to contribute to pgtk

## I want to report a bug, or request a feature

Use [GitHub Issues](https://github.com/freshtonic/pgtk/issues) to report a bug, to request a
feature, or to suggest a change to the documentation.

[Before you open an issue](https://github.com/freshtonic/pgtk/issues/new/choose), look at the
[open issues](https://github.com/freshtonic/pgtk/issues?q=is%3Aissue+is%3Aopen+sort%3Aupdated-desc)
and the
[issues that were closed recently](https://github.com/freshtonic/pgtk/issues?q=is%3Aissue+sort%3Aupdated-desc+is%3Aclosed),
to make sure that nobody reported the problem before. Include as much information as you can.
For a parser or an analysis bug, the most useful information is:

- the SQL text,
- the crate, its version and the PostgreSQL version feature (`pg14` ... `pg19-beta`),
- what PostgreSQL does with the same SQL, and what pgtk does.

## I want to contribute a change

[Fork the repository](https://github.com/freshtonic/pgtk/fork) and
[open a pull request](https://github.com/freshtonic/pgtk/compare).

[`DEVELOPMENT.md`](DEVELOPMENT.md) tells you how to get the PostgreSQL sources, build and test.
[`AGENTS.md`](AGENTS.md) gives the rules of the repository, and
[`CONTEXT-MAP.md`](CONTEXT-MAP.md) gives the domain language of each crate.

Before you open the pull request, make sure that these commands pass:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
cargo test --workspace
```

When you commit:

- Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) for the commit
  messages. release-plz makes the changelogs from them. CI checks the commits of each pull
  request; run `scripts/check-conventional-commits origin/main` to check them before you push.
- Sign off each commit, as part of the
  [Developer Certificate of Origin (DCO)](https://developercertificate.org).

  To do this, give the `-s` or `--signoff` flag to `git commit`:

  ```sh
  git commit -s -m "fix(pgtk-catalog): keep the collation of a domain"
  ```

# Security issue notifications

If you find a possible security problem in this project, report it with
[private vulnerability reporting](https://github.com/freshtonic/pgtk/security/advisories/new).
See [`SECURITY.md`](SECURITY.md).

Do not open a public GitHub issue for it.

## Licensing

See the [`LICENSE`](LICENSE) file for the licence of the project. You agree that your
contributions have the same licence.
