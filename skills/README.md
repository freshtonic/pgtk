# Agent skills for the pgtk crates

These are [Agent Skills](https://agentskills.io/specification) for users of the published pgtk
crates. Each skill is a folder with a `SKILL.md` file. A coding agent reads the skill when the
task agrees with its description. Then the agent uses the current API of the crate correctly.

| Skill | Crate | Task |
|---|---|---|
| [`pgtk-sql-check-migration`](pgtk-sql-check-migration/SKILL.md) | pgtk-sql | Lint a migration with a `Visitor`. |
| [`pgtk-sql-minimum-version`](pgtk-sql-minimum-version/SKILL.md) | pgtk-sql | Find the oldest PostgreSQL version that accepts a statement. |
| [`pgtk-analyze-describe-query`](pgtk-analyze-describe-query/SKILL.md) | pgtk-analyze | Get the result types and the parameter types of a query. |
| [`pgtk-analyze-check-queries`](pgtk-analyze-check-queries/SKILL.md) | pgtk-analyze | Check the queries of an application against a catalog snapshot in CI. |
| [`pgtk-catalog-snapshot`](pgtk-catalog-snapshot/SKILL.md) | pgtk-catalog | Load a catalog from a server, and save it for use with no server. |

Each skill name starts with the name of its crate, so that it does not collide with the skills
of other projects.

These skills are for users of the crates. To contribute to pgtk, read `AGENTS.md` at the root of
the repository.

## Install

Copy the skill folders into the skills folder of your agent. Copy only the skills of the crates
that you use.

For Claude Code, in your project:

```sh
git clone --depth 1 https://github.com/freshtonic/pgtk.git /tmp/pgtk
mkdir -p .claude/skills
cp -R /tmp/pgtk/skills/pgtk-* .claude/skills/
```

Use `~/.claude/skills/` in place of `.claude/skills/` to install the skills for all of your
projects. For a different agent, copy the folders into the folder where that agent reads skills.
Most agents that support the Agent Skills format read one folder per skill, with a `SKILL.md` in
it.

## Use the version that agrees with your crates

A skill describes the API of the crates at one commit. When you update a pgtk crate, copy the
skills of the same release again. A skill for an old API can make an agent write code that does
not compile.

The skills are not in the published crates yet, because no Rust tool installs skills from the
dependencies of a project. When such a tool exists, the crates can include their skills.

## How CI keeps the skills correct

The Cargo-only package `packages/pgtk-skills` includes each `SKILL.md` as documentation. Rustdoc
compiles each Rust code block of a skill as a doctest. It also runs each block that is not
`no_run`, so the assertions in the examples are checked. `cargo test --workspace` in CI runs
these doctests. Thus a change to an API that a skill uses stops CI until the skill is updated.

To check the skills on your computer:

```sh
cargo test -p pgtk-skills --doc
```

If you add a skill, add a module for it in `packages/pgtk-skills/src/lib.rs`, and add it to the
table above. Give each Rust code block in a skill a complete example that compiles. Mark a block
`no_run` only when it needs a server.
