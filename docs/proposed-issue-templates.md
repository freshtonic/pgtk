# Proposed issue templates

Status: proposal. When it is accepted, put each file below in `.github/ISSUE_TEMPLATE/`.

The templates are GitHub issue forms (YAML). They ask for the facts that make a pgtk bug easy to
reproduce: the SQL text, the crate, its version, the PostgreSQL version feature, what PostgreSQL
does and what pgtk does. Each template adds the `needs-triage` label (see
`docs/agents/triage-labels.md`).

The `docs-feedback` template is adapted from the CipherStash Proxy repository.

## `config.yml`

No blank issues. The security link sends a vulnerability report to private reporting, not to a
public issue.

```yaml
blank_issues_enabled: false
contact_links:
  - name: Report a security vulnerability
    url: https://github.com/freshtonic/pgtk/security/advisories/new
    about: Report a vulnerability privately. Do not open a public issue. See SECURITY.md.
```

## `parse-bug.yml`

For pgtk-sql and pgtk-psql: a statement that pgtk parses differently from PostgreSQL or psql.

```yaml
name: Parser bug
description: pgtk-sql or pgtk-psql accepts or rejects SQL differently from PostgreSQL.
title: "[parse]: "
labels: ["needs-triage", "parser"]
body:
  - type: markdown
    attributes:
      value: |
        pgtk-sql must accept exactly what PostgreSQL accepts. A difference is a bug.
  - type: textarea
    id: sql
    attributes:
      label: SQL
      description: The smallest SQL text that shows the problem.
      render: sql
    validations:
      required: true
  - type: dropdown
    id: crate
    attributes:
      label: Crate
      options: [pgtk-sql, pgtk-psql]
    validations:
      required: true
  - type: input
    id: version
    attributes:
      label: Crate version
      placeholder: "0.1.0"
    validations:
      required: true
  - type: dropdown
    id: feature
    attributes:
      label: PostgreSQL version feature
      options: [pg14, pg15, pg16, pg17, pg18, pg19-beta]
    validations:
      required: true
  - type: textarea
    id: postgres
    attributes:
      label: What PostgreSQL does
      description: The result of PostgreSQL (or psql) of the same version, with its error if it has one.
    validations:
      required: true
  - type: textarea
    id: pgtk
    attributes:
      label: What pgtk does
      description: The result of pgtk, with its error if it has one.
    validations:
      required: true
```

## `analysis-bug.yml`

For pgtk-analyze and pgtk-catalog: a type, a column or an error that is different from the answer
of PostgreSQL.

```yaml
name: Analysis bug
description: pgtk-analyze or pgtk-catalog gives a different answer from PostgreSQL.
title: "[analysis]: "
labels: ["needs-triage", "analysis"]
body:
  - type: textarea
    id: sql
    attributes:
      label: SQL
      description: The statement, and the DDL that makes the tables and the types that it uses.
      render: sql
    validations:
      required: true
  - type: dropdown
    id: crate
    attributes:
      label: Crate
      options: [pgtk-analyze, pgtk-catalog]
    validations:
      required: true
  - type: input
    id: version
    attributes:
      label: Crate version
      placeholder: "0.1.0"
    validations:
      required: true
  - type: input
    id: server
    attributes:
      label: PostgreSQL server version
      placeholder: "17.11"
    validations:
      required: true
  - type: dropdown
    id: catalog
    attributes:
      label: Catalog
      options:
        - Built-in catalog (fixture)
        - Loaded from a server
    validations:
      required: true
  - type: textarea
    id: postgres
    attributes:
      label: What PostgreSQL does
      description: The Describe result (for example from `\gdesc` in psql), or the error, with its SQLSTATE and position.
    validations:
      required: true
  - type: textarea
    id: pgtk
    attributes:
      label: What pgtk does
    validations:
      required: true
```

## `feature-request.yml`

```yaml
name: Feature request
description: Ask for a new capability in a pgtk crate.
title: "[feature]: "
labels: ["needs-triage", "enhancement"]
body:
  - type: dropdown
    id: crate
    attributes:
      label: Crate
      multiple: true
      options: [pgtk-sql, pgtk-catalog, pgtk-analyze, pgtk-psql, other]
    validations:
      required: true
  - type: textarea
    id: problem
    attributes:
      label: What problem do you want to solve?
      description: Tell us what you are building, and where pgtk stops you.
    validations:
      required: true
  - type: textarea
    id: proposal
    attributes:
      label: What do you propose?
      description: An API, an example of its use, or the PostgreSQL behaviour to reproduce.
    validations:
      required: false
```

## `docs-feedback.yml`

```yaml
name: Docs feedback
description: Help us make the documentation more useful.
title: "[docs]: "
labels: ["needs-triage", "docs"]
body:
  - type: markdown
    attributes:
      value: |
        Thank you for your feedback!
  - type: input
    id: page
    attributes:
      label: Page
      description: The README, the docs.rs page or the file.
    validations:
      required: false
  - type: textarea
    id: what-problem
    attributes:
      label: What problem did you try to solve?
      placeholder: Tell us what you looked for in the documentation.
    validations:
      required: true
  - type: textarea
    id: additional-info
    attributes:
      label: Anything else that you want us to know?
    validations:
      required: false
```

## Open questions

- The labels `parser`, `analysis`, `enhancement` and `docs` do not exist yet. Make them, or remove
  them from the templates.
- The proxy templates require agreement to a code of conduct. pgtk has no `CODE_OF_CONDUCT.md`
  yet, because it needs a contact for reports. When it has one, add the checkbox to each template.
