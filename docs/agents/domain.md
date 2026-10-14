# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **`CONTEXT-MAP.md`** at the repo root — it points at one `CONTEXT.md` per crate. Read each one relevant to the topic.
- **`docs/adr/`** — read ADRs that touch the area you're about to work in. Also check `crates/<crate>/docs/adr/` for crate-scoped decisions.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

This repo is a multi-context Cargo workspace. Each crate is one context:

```
/
├── Cargo.toml                         ← workspace manifest
├── CONTEXT-MAP.md
├── docs/adr/                          ← workspace-wide decisions
└── crates/
    ├── <crate-a>/
    │   ├── Cargo.toml
    │   ├── CONTEXT.md
    │   ├── docs/adr/                  ← crate-specific decisions
    │   └── src/
    └── <crate-b>/
        ├── Cargo.toml
        ├── CONTEXT.md
        ├── docs/adr/
        └── src/
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in the relevant `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders) — but worth reopening because…_
