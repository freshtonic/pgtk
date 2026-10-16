---
status: accepted
---

# Literal text is never validated by the crate; a Literal Validator is a caller extension

PostgreSQL runs a type's input function during `parse_analyze` when it coerces an `unknown` literal, so `SELECT 'abc'::int` fails inside analysis. Matching that needs an input parser for every type and depends on session settings such as `DateStyle`. We decided that Analysis never rejects a literal for its text, and instead accepts an optional caller-supplied Literal Validator that does what an input function would. This is the one class of PostgreSQL Diagnostic the crate knowingly does not produce on its own, and the Oracle baseline records those cases as expected divergences rather than bugs.

## Considered options

- Implement input validation for the common built-in types. Rejected for the first milestones: unbounded work for little value to the known consumers, and the extension point leaves the door open.
