---
status: accepted
---

# The Session is an input next to the Settings, and Analysis reports the Session Effect of a Statement

EXECUTE, FETCH and SHOW of a custom parameter mean nothing on their own: a Describe answers them from what the session holds. PostgreSQL keeps that state in the backend, and this crate has no backend. We decided that the caller supplies a Session snapshot next to the Settings, with an empty Session as the default, and that Analysis reads it and never changes it. A Statement that would change the session reports what it would change as a Session Effect in `Analysis::session_effect`. A caller that replays statements in order folds each Effect into its own Session with `Session::apply`, exactly as the Oracle harness does for every file it replays.

The Session holds the three things a Parse or a Describe reads: the prepared statements, the open cursors and the custom configuration placeholders a SET made. EXECUTE answers with the prepared statement's Result Descriptor, FETCH with the cursor's, and SHOW of a placeholder with one text column named with the placeholder's name. A prepared statement or a cursor the Session does not hold answers with an empty Result Descriptor and no Diagnostic, because that is what a Describe answers: `UtilityTupleDescriptor` says that raising `26000` or `34000` there is "not our business", and PostgreSQL raises it when the Statement runs. A configuration parameter that does not exist is different: `GetPGVariableResultDesc` looks the name up as the Describe answers, so SHOW of an unknown name is the `42704` PostgreSQL raises for it.

A prepared statement keeps the text of its PREPARE as well as its Result Descriptor, because PostgreSQL's plan cache keeps the query's raw parse tree and analyses it again whenever it revalidates. A Describe of an EXECUTE revalidates, so it answers from the query as the Catalog is now: the relation the query names may be gone, and the columns may come from a table that was made again. PostgreSQL refuses a prepared statement whose result columns changed, with `0A000 cached plan must not change result type`, and so does Analysis. A cursor is not revalidated, because its portal holds the descriptor the DECLARE computed.

## Considered options

- **Analysis owns a mutable session.** The Analyzer would hold the session and change it as it analyses each Statement. Rejected: the Analyzer is a value the caller builds once and asks about any Statement, in any order, from any thread. Hidden state would make the answer depend on the order of earlier calls, and it would make one Analysis unrepeatable. The caller already owns the order in which statements run, so the caller owns the state.
- **Unsupported for a name the Session does not hold.** Rejected: the crate knows what the Statement means. An Unsupported is for what the crate cannot analyse, not for what it can.
- **A Diagnostic for a name the Session does not hold.** The answer would be the `26000` or the `34000` PostgreSQL raises for it. Rejected: a Describe never raises it. `UtilityTupleDescriptor` answers with no columns and leaves the error to the moment the Statement runs, so a Diagnostic here would be an answer the Oracle never gives, and the harness had to carry it as a known divergence. An empty Result Descriptor is what the modelled message answers.
- **A Session inside the Settings.** Rejected: the Settings are what changes the meaning of any Statement, and they are usually the same for a whole run. The Session changes from statement to statement. Keeping them apart keeps the common case simple.

## Consequences

- `Analyzer::analyze` and `Analyzer::analyze_sql` take a `&Session` as their last argument. A caller with no session state passes `&Session::new()`.
- Analysis never applies an Effect, not even to itself: the Effect of a PREPARE does not change what the PREPARE answers, and EXPLAIN reports no Effect, because EXPLAIN runs nothing.
- PREPARE stays a Utility Statement, so its own Analysis leaves the query unanalysed and answers with an empty Result Descriptor. The Effect needs the query's Result Descriptor, so the Effect is built from a second Analysis of the query's text. A second Analysis that fails reports no Effect at all, because PostgreSQL would fail when the Statement ran and the prepared statement would not exist.
- A Session holds Catalog handles, so it is meaningful only for the Catalog it was built against. A caller that reloads the Catalog must rebuild the Session, as the Oracle harness does.
- A caller that wants the `26000` or the `34000` reads it from its own Session, which is the thing that knows the name is missing. Analysis answers the Describe question only.
