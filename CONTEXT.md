# pg-catalog

`pg-catalog` is a Rust model of one PostgreSQL database's system catalog. It
exists so that a consumer, first `pg-type-check`, can resolve the names in a
SQL statement and reason about PostgreSQL types without a live connection at
the point of use.

## Language

### Snapshot

**Catalog**:
An immutable, self-contained picture of one PostgreSQL database's system
catalog as it was at one moment. A Catalog is built by a Loader and is never
mutated; a refresh produces a new Catalog.
_Avoid_: Database, schema (both mean other things in PostgreSQL), mirror, cache

**Loader**:
The part of the crate that turns the rows of the catalog queries into a
Catalog. The Loader does no I/O itself; an Executor runs the queries.
_Avoid_: Hydrator, fetcher, sync

**Executor**:
The caller-supplied adapter that runs one SQL text against a live PostgreSQL
server and returns its rows as text. Adapters for common drivers are optional.
_Avoid_: Driver, client, connection

**Diagnostic**:
A note produced by the Loader about an Entity it could not fully represent
and either kept as opaque or skipped. Diagnostics never abort a load.
_Avoid_: Warning, error (a load error is fatal; a Diagnostic is not)

**Fixture**:
A serialized Catalog saved to a file so that a Catalog can be reconstructed
with no server. The crate ships one Fixture of a stock PostgreSQL 17 database.
_Avoid_: Snapshot file, dump

**Catalog Builder**:
The only way to construct a Catalog. The Loader drives it from query rows;
tests drive it by hand, usually starting from the Fixture and adding Entities.
_Avoid_: Factory, mock catalog

**Handle**:
A typed identifier for one Entity within one Catalog. A Handle is only
meaningful for the Catalog that issued it. A Handle for one kind of Entity
cannot address another kind.
_Avoid_: ID, reference, pointer, OID (an OID is the server's identifier, not
ours)

**OID**:
PostgreSQL's own object identifier for an Entity, kept on every Entity for
display and for correlating with server output. Never used as the Rust-side
identity.

### Entities

**Entity**:
Any catalog object the Catalog represents: a Schema, Type, Relation, Column,
Function, Operator, Cast, or Extension. Every Entity has an OID
and, when it belongs to one, an owning Extension.

**Name**:
An identifier exactly as the server stores it, already case-folded and
unquoted. Names match byte-for-byte; folding and quoting belong to the parser.
_Avoid_: Identifier (the parser's term for the unfolded form)

**Qualified Name**:
A Schema Name and an Entity Name together, naming one Entity without a Search
Path.

**Type**:
A PostgreSQL data type: a `pg_type` row. Never a Rust type; say "Rust struct"
for those. A Type is one of: base, composite, domain, enum, range, multirange,
array, or pseudo-type.
_Avoid_: Datatype, PgType (in prose)

**Domain**:
A Type that wraps another Type and adds constraints. A Domain is a kind of
Type, not a separate Entity.

**Custom Type**:
Any Type that is not part of a stock PostgreSQL installation: created by a
user or installed by an Extension. Not a distinct kind; a Custom Type is still
a base, composite, domain, enum, or range Type.
_Avoid_: User-defined type (excludes Extension types)

**Built-in Type**:
A Type that every PostgreSQL server ships with a fixed OID, such as `int4`,
`text`, or `anyelement`. The model names each one in a closed set so a
consumer can reach it without a name lookup. A server missing one is not
PostgreSQL.
_Avoid_: Core type, system type, well-known type

**Field**:
One member of a composite Type, with its Name and Type. A Field belongs to a
Type; a Column belongs to a Relation. A table's row Type has one Field per
Column.
_Avoid_: Attribute, composite column

**Pseudo-type**:
A Type that stands for a family of Types or a special value rather than a
storable value: `anyelement`, `anyarray`, `anycompatible`, `record`, `void`,
`unknown`, and their kin. Pseudo-types are a closed set in the model.
_Avoid_: Polymorphic type (only some Pseudo-types are polymorphic)

**Type Modifier**:
The per-column integer that refines a Type, such as the length of a
`varchar` or the precision and scale of a `numeric`. Held raw; its meaning
is specific to each Type.
_Avoid_: Typmod (in prose), length, precision

**Relation**:
A `pg_class` row that has Columns and can be named in a `FROM` clause: a
table, view, materialized view, foreign table, partitioned table, or
sequence. A sequence has exactly three Columns: `last_value`, `log_cnt`, and
`is_called`. The server gives a sequence no row Type, so a sequence is the
one Relation without one. The model holds no parameter of a sequence, such as
the start or the increment. Indexes and TOAST tables are not Relations in
this model.

**Parent**:
A Relation that another Relation inherits from, or the partitioned table of a
partition: a `pg_inherits` row. A Relation holds its direct Parents, in
order. PostgreSQL accepts the row Type of a Relation where it expects the row
Type of one of its ancestors.

**Column**:
One live attribute of a Relation, with its Type, Type Modifier, and
nullability. An identity Column also has its kind, `ALWAYS` or `BY DEFAULT`,
because only an `ALWAYS` Column rejects a value other than `DEFAULT`. Dropped
attributes are not Columns.
_Avoid_: Attribute, field (a Field belongs to a composite Type)

**Primary Key**:
The Columns that a Relation's `PRIMARY KEY` constraint names, in key order,
with a flag that says if the constraint is deferrable. PostgreSQL reads it in
its check of an ungrouped Column: when `GROUP BY` names each Column of a
Primary Key that is not deferrable, each other Column of that Relation is
grouped too. No other constraint is in the model.

**Function**:
A `pg_proc` row: a plain function, procedure, aggregate, or window function.
A Function that has defaults holds the Type of each default expression, and
not the expression. PostgreSQL reads that Type when a call leaves out a
polymorphic argument.
_Avoid_: Routine, proc

**Aggregate**:
A Function that combines a set of rows, with an aggregate kind (normal,
ordered-set, hypothetical-set) and a count of direct arguments. An Aggregate
is a kind of Function, not a separate Entity.

**Operator**:
A `pg_operator` row: a named symbol with zero-or-one left operand Type, one
right operand Type, a result Type, and an implementing Function. Distinct from
an operator class or operator family, which describe index behaviour. The
model holds no operator class. It holds only the Sort Operators that a default
operator class gives a Type.

**Sort Operators**:
The Operators that PostgreSQL uses to sort and to group values of one Type:
the `<`, `<=`, `=`, `>=`, and `>` members of the Type's default btree operator
class, and the `=` member of its default hash operator class. Each takes the
Type on both sides. The lookup is for the exact Type; the rules for a Domain,
a binary-coercible Type, and a container Type belong to the consumer. A Type
without them, such as `json` or `xml`, cannot be sorted or grouped.
_Avoid_: Op

**Cast**:
A `pg_cast` row: a conversion from one Type to another, with a Cast Context
and a method.

**Cast Context**:
The strength of a Cast: implicit (applied silently), assignment (applied on
assignment to a Column), or explicit (applied only when written).
_Avoid_: Cast kind, coercion level

**Schema**:
A `pg_namespace` row. The container that qualifies an Entity's name. At most
one Schema in a Catalog is temporary: the `pg_temp_N` Schema of the session
that the Loader used.
_Avoid_: Namespace (in prose)

**Extension**:
A `pg_extension` row. An installed package that owns Entities; ownership comes
from `pg_depend`.

**Search Path**:
An ordered list of Schemas used to resolve an unqualified name, with
PostgreSQL's implicit rules already applied: the temporary Schema first, then
`pg_catalog`, unless the list names them. The name `pg_temp` is an alias for
the temporary Schema. A lookup of a Function or of an Operator skips the
temporary Schema, as PostgreSQL does. A Catalog does not hold a Search Path;
lookups take one as an argument.

## Out of scope

These are deliberate exclusions, not omissions:

- Operator classes and operator families as Entities, all access methods other
  than btree and hash, non-default operator classes, and cross-type members of
  an operator family (such as `int4 < int8`). Only the Sort Operators of each
  Type are in scope.
- Collations. A Type keeps a collatable flag so collation checking can be
  added later without reshaping the model.
- Indexes, constraints (other than Primary Keys and counts on Domains),
  triggers, rules, policies, roles, and privileges. Unique, foreign key,
  check, and exclusion constraints on a Relation are out of scope.
- Default and constraint expression text. The Type of the default expression
  of a Function argument is in scope; the expression is not.
- The temporary Schemas of other sessions, and the Entities in them. The
  temporary Schema of the Loader's own session is in scope.
- Function bodies, languages, cost, and execution detail of Aggregates.
