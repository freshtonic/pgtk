# pg-analyze

A Rust crate that performs PostgreSQL parse analysis: it takes one parsed Statement from `pg-sql` and a Catalog from `pg-catalog`, resolves every name, operator, function and type exactly as PostgreSQL 17 would, and reports the Diagnostic PostgreSQL would report. It exists so that a consumer can know what a Statement means without a live server.

## Language

### The pass

**Analysis**:
The single pass that turns a Statement into its resolved meaning, equivalent to PostgreSQL's `parse_analyze`. Type checking is part of Analysis, not a separate step.
_Avoid_: Type check (as a separate phase), semantic analysis, validation, resolution (for the whole pass)

**Statement**:
One parsed PostgreSQL statement as `pg-sql` produces it. Analysis reads a Statement and never changes it.
_Avoid_: Query (that is PostgreSQL's name for the analysed form), AST, parse tree

**Utility Statement**:
A Statement that PostgreSQL's `parse_analyze` does not transform, such as DDL. Analysis leaves it unanalysed, a query embedded in it included, as PostgreSQL leaves it at Parse time: the query inside CREATE VIEW, CREATE RULE, PREPARE, COPY or CREATE TABLE AS EXECUTE is analysed only when the Statement runs, so Analysis reports no Diagnostic for it and an empty Result Descriptor. EXPLAIN, DECLARE and CREATE TABLE AS with a query are not Utility Statements in this sense: `parse_analyze` transforms their query.

**Rewrite**:
The part of PostgreSQL's rewriter that runs before a Describe answers, which Analysis reproduces: a data-modifying Statement on a view is checked as the rewriter checks it and its target becomes the base relation, a rule refuses RETURNING or ON CONFLICT as the rewriter does, and generated and identity columns refuse the values the rewriter refuses. Analysis does not reproduce a rule's actions.
_Avoid_: Query rewriting (the whole of PostgreSQL's rewriter), view expansion

**Catalog**:
The immutable snapshot of one database's system catalog, as defined by `pg-catalog`. Analysis reads it and never changes it.

**Search Path**:
The ordered list of schemas used to resolve an unqualified name, as defined by `pg-catalog`.

**Settings**:
The configuration that changes what a Statement means, supplied by the caller for every Analysis. The Search Path is the one mandatory member, given as already-resolved schema names; substituting `$user` and temporary schemas is the caller's job.
_Avoid_: GUCs, config, options, session state

**Session**:
The objects a PostgreSQL session holds that a Parse or a Describe reads: its prepared statements, its open cursors and the custom configuration placeholders a SET created. A Session is a snapshot the caller supplies next to the Settings; an empty Session is the default. Analysis reads it and never changes it: EXECUTE answers with the prepared statement's Result Descriptor, FETCH with the cursor's, SHOW of a placeholder with its name.
_Avoid_: connection, backend, portal, plan cache

**Session Effect**:
What a Statement would add to or remove from the Session when it runs, reported by Analysis so that a caller replaying statements in order can fold it into the next Session: PREPARE adds a prepared statement, DECLARE a cursor, SET a placeholder, DEALLOCATE and CLOSE remove theirs, and the end of a transaction closes the cursors that are not held. Analysis reports the Effect; applying it is the caller's job.
_Avoid_: side effect, mutation, execution

**Target Version**:
The PostgreSQL major version, 14 through 17, whose analysis rules and diagnostic wording an Analysis applies. The caller chooses it by entry point; the Catalog's own version does not choose it.
_Avoid_: Compatibility mode, dialect

**Analyzer**:
The value a caller builds once from a Catalog, Settings and an optional Literal Validator, and then asks to analyse Statements.
_Avoid_: Checker, engine, context

### The result

**Annotation**:
One piece of resolved information that refers to a node of the Statement. Everything Analysis learns, including things PostgreSQL would represent as new tree nodes, is an Annotation.
_Avoid_: Metadata, resolved node, decoration

**Annotation Table**:
The collection of all Annotations of one kind for one Statement, keyed by Node Handle. Analysis produces one Annotation Table per kind of Annotation.
_Avoid_: Side table, map, index

**Node Handle**:
The per-parse identifier of one node of a Statement, as `pg-sql`'s flat form issues it. It is meaningful only for the parse that issued it.
_Avoid_: Node key, node id, address, handle (bare)

**Catalog Handle**:
The identifier of one Catalog entity, as `pg-catalog` issues it. Annotations name types, relations, functions, operators and casts by Catalog Handle.
_Avoid_: OID, handle (bare)

**Mirrored Concept**:
A concept Analysis copies from PostgreSQL's analysed form and calls by PostgreSQL's own name: range table, range table entry, target entry, resjunk, `Var`, coercion. Each is one kind of Annotation.

**Result Descriptor**:
What a client would learn from a Describe message: the result columns with their names and types, and the type of every parameter in order.
_Avoid_: Row description, schema, shape

**Declared Parameter Type**:
A type the caller states for a positional parameter before Analysis. Analysis infers the type of every parameter without one.

**Diagnostic**:
The error PostgreSQL would raise for a Statement, with the same SQLSTATE, message and cursor position, plus the spans that PostgreSQL does not have. Analysis stops at the first Diagnostic, as PostgreSQL does. This is a different concept from `pg-catalog`'s Diagnostic, which is a loader note.
_Avoid_: Error (for the PostgreSQL-defined ones), warning, finding, report

**Unsupported**:
The outcome when Analysis meets something PostgreSQL would accept but this crate cannot yet analyse. Never reported as a Diagnostic and never as a wrong answer.
_Avoid_: Unimplemented, todo, unknown

### Fidelity

**Oracle**:
A real PostgreSQL 17 server whose behaviour on a Statement is the authority for what Analysis must produce.
_Avoid_: Reference implementation, golden

**Literal Validator**:
A caller-supplied extension that checks the text of a literal against its type, doing what PostgreSQL's type input function does during analysis. Without one, Analysis never rejects a literal for its text.
