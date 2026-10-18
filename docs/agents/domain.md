# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **`CONTEXT-MAP.md`** at the repo root — it points at one `CONTEXT.md` per package. Read each one relevant to the topic.
- **`docs/adr/`** — read ADRs that touch the area you're about to work in. Also check `packages/<package>/docs/adr/` for package-scoped decisions.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

This repo is a multi-context Cargo workspace. A package that has its own domain language is one context, and `CONTEXT-MAP.md` lists them. A package that uses the glossary of another package, and the tooling crates, have no `CONTEXT.md`:

```
/
├── Cargo.toml                         ← workspace manifest
├── CONTEXT-MAP.md
├── docs/adr/                          ← workspace-wide decisions
└── packages/
    ├── <package-a>/
    │   ├── Cargo.toml
    │   ├── CONTEXT.md
    │   ├── docs/adr/                  ← package-specific decisions
    │   └── src/
    └── <package-b>/
        ├── Cargo.toml
        ├── CONTEXT.md
        ├── docs/adr/
        └── src/
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in the relevant `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Cite ADRs with their package

The root and each package number their ADRs separately, so more than one ADR has the number 0001. In a document inside a package, "ADR 0009" means that package's ADR. Everywhere else — the root, another package, an issue, a PR, a commit message — name the owner: "pgtk-sql ADR 0009", or "workspace ADR 0001" for one in `docs/adr/`. Never renumber an ADR.

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders) — but worth reopening because…_
