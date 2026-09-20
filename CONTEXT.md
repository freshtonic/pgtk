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
A Statement that PostgreSQL's `parse_analyze` does not transform, such as DDL. Analysis leaves it unanalysed except for any query embedded in it.

**Catalog**:
The immutable snapshot of one database's system catalog, as defined by `pg-catalog`. Analysis reads it and never changes it.

**Search Path**:
The ordered list of schemas used to resolve an unqualified name, as defined by `pg-catalog`.

**Settings**:
The session state that changes what a Statement means, supplied by the caller for every Analysis. The Search Path is the one mandatory member.
_Avoid_: GUCs, config, options, session

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
