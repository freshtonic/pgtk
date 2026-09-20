//! The SQL that the Loader runs, and the identifier of each query.
//!
//! This module has no dependency on the rest of the crate. It holds only the
//! [`QueryId`] enum, one `&'static str` per query, and [`sql`].
//!
//! # Column order is the contract
//!
//! Each SQL constant has a comment that lists the index and the meaning of
//! every selected column. `loader/parse.rs` reads the columns by index. Do
//! not add, remove, or reorder a column without a change to the parser.
//!
//! # Rules that every query obeys
//!
//! - Every selected column is cast to `text`. A SQL NULL arrives as `None`.
//! - A boolean arrives as `t` or `f`. A cast of a boolean straight to `text`
//!   gives `true` or `false`, so every boolean goes through a `CASE` that
//!   gives `t` or `f`. The `CASE` keeps a NULL as a NULL.
//! - Every catalog relation, and every `::regclass` and `::regproc` literal,
//!   is qualified with `pg_catalog.`. The SQL does not depend on the session
//!   `search_path`.
//! - A query that reads entities in a schema joins `pg_catalog.pg_namespace`
//!   and applies the schema filter:
//!   `nspname NOT LIKE 'pg\_toast%' AND (nspname NOT LIKE 'pg\_temp%' OR
//!   oid = pg_catalog.pg_my_temp_schema())`. The filter keeps the temporary
//!   schema of this session and drops that of each other session.
//!   `pg_my_temp_schema()` gives 0 when the session has no temporary schema.
//!   The name `pg_toast_temp_N` matches `pg\_toast%`, so it stays out.
//! - A column of type `regproc`, such as `pg_operator.oprcode` or
//!   `pg_aggregate.aggfnoid`, is cast to `oid` before `text`. A direct cast
//!   to `text` gives the function name, not the OID.
//! - The SQL is the same for PostgreSQL 14, 15, 16, and 17.
//!
//! # Extraction markers
//!
//! `scripts/check-queries.sh` reads the SQL out of this file. It holds no
//! second copy. The convention is:
//!
//! - A line `//@sql <VariantName>` comes directly before the constant.
//! - The constant opens a raw string on the next line. That line ends with
//!   `r#"`.
//! - A line that holds only `"#;` closes the SQL.
//!
//! Keep that shape when you add or edit a query.

/// The identifier of one catalog query.
///
/// The Loader runs every query in [`QueryId::ALL`] and feeds the rows back.
/// [`sql`] gives the SQL text of each one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryId {
    /// The server version number, as `server_version_num`.
    Version,
    /// The name of the current database.
    Database,
    /// The effective search path, as `current_schemas(true)`.
    SearchPath,
    /// Every schema, except the TOAST and the temporary schemas.
    Schemas,
    /// Every installed extension.
    Extensions,
    /// The objects that belong to an extension.
    ExtensionMembers,
    /// Every type in a loaded schema.
    Types,
    /// The labels of every enum type.
    EnumLabels,
    /// Every table, view, materialized view, foreign table, partitioned
    /// table, and sequence in a loaded schema.
    Relations,
    /// The live attributes of every relation and of every composite type.
    Attributes,
    /// Every function, procedure, aggregate, and window function in a loaded
    /// schema.
    Functions,
    /// One row per function argument, in argument order.
    FunctionArguments,
    /// The aggregate detail of every aggregate function.
    Aggregates,
    /// Every operator in a loaded schema.
    Operators,
    /// Every cast.
    Casts,
    /// The same-type operators of every default btree and hash operator
    /// class.
    DefaultOperatorClasses,
    /// One row per column of every primary key, in key order.
    PrimaryKeys,
}

impl QueryId {
    /// Every query, in declaration order.
    pub const ALL: &'static [QueryId] = &[
        QueryId::Version,
        QueryId::Database,
        QueryId::SearchPath,
        QueryId::Schemas,
        QueryId::Extensions,
        QueryId::ExtensionMembers,
        QueryId::Types,
        QueryId::EnumLabels,
        QueryId::Relations,
        QueryId::Attributes,
        QueryId::Functions,
        QueryId::FunctionArguments,
        QueryId::Aggregates,
        QueryId::Operators,
        QueryId::Casts,
        QueryId::DefaultOperatorClasses,
        QueryId::PrimaryKeys,
    ];
}

/// Gives the SQL text of one query.
///
/// The text is the same for PostgreSQL 14 through 17.
pub fn sql(id: QueryId) -> &'static str {
    match id {
        QueryId::Version => VERSION,
        QueryId::Database => DATABASE,
        QueryId::SearchPath => SEARCH_PATH,
        QueryId::Schemas => SCHEMAS,
        QueryId::Extensions => EXTENSIONS,
        QueryId::ExtensionMembers => EXTENSION_MEMBERS,
        QueryId::Types => TYPES,
        QueryId::EnumLabels => ENUM_LABELS,
        QueryId::Relations => RELATIONS,
        QueryId::Attributes => ATTRIBUTES,
        QueryId::Functions => FUNCTIONS,
        QueryId::FunctionArguments => FUNCTION_ARGUMENTS,
        QueryId::Aggregates => AGGREGATES,
        QueryId::Operators => OPERATORS,
        QueryId::Casts => CASTS,
        QueryId::DefaultOperatorClasses => DEFAULT_OPERATOR_CLASSES,
        QueryId::PrimaryKeys => PRIMARY_KEYS,
    }
}

// One row.
//
// Columns:
//   0  server_version_num, for example `170004`
//@sql Version
const VERSION: &str = r#"
SELECT current_setting('server_version_num')::text
"#;

// One row.
//
// Columns:
//   0  the name of the current database
//@sql Database
const DATABASE: &str = r#"
SELECT current_database()::text
"#;

// One row per schema on the effective search path, in path order.
//
// Columns:
//   0  schema name
//@sql SearchPath
const SEARCH_PATH: &str = r#"
SELECT s::text
FROM unnest(current_schemas(true)) WITH ORDINALITY AS t(s, n)
ORDER BY n
"#;

// One row per schema, except the TOAST schemas and the temporary schemas of
// other sessions. The temporary schema of this session is here, when the
// session has one.
//
// Columns:
//   0  oid           schema OID
//   1  nspname       schema name
//   2  is_temporary  `t` for the temporary schema of this session
//@sql Schemas
const SCHEMAS: &str = r#"
SELECT
    n.oid::text,
    n.nspname::text,
    (CASE WHEN n.oid = pg_catalog.pg_my_temp_schema() THEN 't' ELSE 'f' END)::text
        AS is_temporary
FROM pg_catalog.pg_namespace n
WHERE n.nspname NOT LIKE 'pg\_toast%'
  AND (n.nspname NOT LIKE 'pg\_temp%'
       OR n.oid = pg_catalog.pg_my_temp_schema())
ORDER BY n.oid
"#;

// One row per installed extension.
//
// Columns:
//   0  oid           extension OID
//   1  extname       extension name
//   2  extversion    installed version
//   3  extnamespace  OID of the schema that holds the extension
//@sql Extensions
const EXTENSIONS: &str = r#"
SELECT
    e.oid::text,
    e.extname::text,
    e.extversion::text,
    e.extnamespace::text
FROM pg_catalog.pg_extension e
ORDER BY e.oid
"#;

// One row per object that belongs to an extension.
//
// `class_label` says which catalog holds the object. The Loader uses it to
// pick the arena that `objid` addresses. A catalog that the model does not
// hold gives `other`.
//
// Columns:
//   0  class_label  one of `type`, `class`, `proc`, `operator`, `cast`,
//                   `namespace`, or `other`
//   1  objid        OID of the member object
//   2  refobjid     OID of the extension that owns it
//@sql ExtensionMembers
const EXTENSION_MEMBERS: &str = r#"
SELECT
    (CASE d.classid
        WHEN 'pg_catalog.pg_type'::pg_catalog.regclass THEN 'type'
        WHEN 'pg_catalog.pg_class'::pg_catalog.regclass THEN 'class'
        WHEN 'pg_catalog.pg_proc'::pg_catalog.regclass THEN 'proc'
        WHEN 'pg_catalog.pg_operator'::pg_catalog.regclass THEN 'operator'
        WHEN 'pg_catalog.pg_cast'::pg_catalog.regclass THEN 'cast'
        WHEN 'pg_catalog.pg_namespace'::pg_catalog.regclass THEN 'namespace'
        ELSE 'other'
    END)::text AS class_label,
    d.objid::text,
    d.refobjid::text
FROM pg_catalog.pg_depend d
WHERE d.deptype = 'e'
  AND d.refclassid = 'pg_catalog.pg_extension'::pg_catalog.regclass
"#;

// One row per type in a loaded schema.
//
// `is_array` tells a true array from a type that only has an element type,
// such as `name` or `point`. `typelem` on a non-array is ignored.
//
// `rngsubtype` and `rngmultitypid` are not NULL when the type is a range.
// `multirange_of` is not NULL when the type is a multirange.
//
// Columns:
//    0  oid            type OID
//    1  typname        type name
//    2  typnamespace   OID of the schema that holds the type
//    3  typtype        kind letter: b c d e p r m
//    4  typcategory    category letter
//    5  typispreferred `t` when the type is preferred in its category
//    6  collatable     `t` when the type has a collation
//    7  typarray       OID of the array type, 0 when there is none
//    8  typelem        OID of the element type, 0 when there is none
//    9  is_array       `t` when the type is a true array
//   10  typrelid       OID of the class that backs a composite, else 0
//   11  relkind        kind letter of that class, NULL when there is none
//   12  typbasetype    OID of the base type of a domain, else 0
//   13  typtypmod      type modifier of a domain, else -1
//   14  typnotnull     `t` when a domain has NOT NULL
//   15  rngsubtype     OID of the subtype of a range
//   16  rngmultitypid  OID of the multirange of a range
//   17  multirange_of  OID of the range that this multirange holds
//   18  check_count    number of CHECK constraints on a domain
//@sql Types
const TYPES: &str = r#"
SELECT
    t.oid::text,
    t.typname::text,
    t.typnamespace::text,
    t.typtype::text,
    t.typcategory::text,
    (CASE WHEN t.typispreferred THEN 't' ELSE 'f' END)::text AS typispreferred,
    (CASE WHEN t.typcollation <> 0 THEN 't' ELSE 'f' END)::text AS collatable,
    t.typarray::text,
    t.typelem::text,
    (CASE
        WHEN t.typsubscript
            = 'pg_catalog.array_subscript_handler'::pg_catalog.regproc
        THEN 't' ELSE 'f'
    END)::text AS is_array,
    t.typrelid::text,
    c.relkind::text,
    t.typbasetype::text,
    t.typtypmod::text,
    (CASE WHEN t.typnotnull THEN 't' ELSE 'f' END)::text AS typnotnull,
    r.rngsubtype::text,
    r.rngmultitypid::text,
    mr.rngtypid::text AS multirange_of,
    (
        SELECT count(*)
        FROM pg_catalog.pg_constraint k
        WHERE k.contypid = t.oid
          AND k.contype = 'c'
    )::text AS check_count
FROM pg_catalog.pg_type t
JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
LEFT JOIN pg_catalog.pg_class c ON c.oid = t.typrelid
LEFT JOIN pg_catalog.pg_range r ON r.rngtypid = t.oid
LEFT JOIN pg_catalog.pg_range mr ON mr.rngmultitypid = t.oid
WHERE n.nspname NOT LIKE 'pg\_toast%'
  AND (n.nspname NOT LIKE 'pg\_temp%'
       OR n.oid = pg_catalog.pg_my_temp_schema())
ORDER BY t.oid
"#;

// One row per label of every enum type, in label order.
//
// Columns:
//   0  enumtypid      OID of the enum type
//   1  enumsortorder  sort position of the label
//   2  enumlabel      the label
//@sql EnumLabels
const ENUM_LABELS: &str = r#"
SELECT
    e.enumtypid::text,
    e.enumsortorder::text,
    e.enumlabel::text
FROM pg_catalog.pg_enum e
ORDER BY e.enumtypid, e.enumsortorder
"#;

// One row per relation in a loaded schema.
//
// A relation is a table, a view, a materialized view, a foreign table, a
// partitioned table, or a sequence. An index and a TOAST table are not
// relations in this model.
//
// Columns:
//   0  oid           relation OID
//   1  relname       relation name
//   2  relnamespace  OID of the schema that holds the relation
//   3  relkind       kind letter: r v m f p S
//   4  reltype       OID of the row type of the relation
//@sql Relations
const RELATIONS: &str = r#"
SELECT
    c.oid::text,
    c.relname::text,
    c.relnamespace::text,
    c.relkind::text,
    c.reltype::text
FROM pg_catalog.pg_class c
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE c.relkind IN ('r', 'v', 'm', 'f', 'p', 'S')
  AND n.nspname NOT LIKE 'pg\_toast%'
  AND (n.nspname NOT LIKE 'pg\_temp%'
       OR n.oid = pg_catalog.pg_my_temp_schema())
ORDER BY c.oid
"#;

// One row per live attribute, in attribute order.
//
// The rows cover both relations and the classes that back composite types,
// so `relkind` also allows `c`. A sequence gives its three attributes:
// `last_value`, `log_cnt`, and `is_called`. A dropped attribute and a system
// attribute are not here.
//
// Columns:
//   0  attrelid   OID of the relation or of the composite class
//   1  attnum     attribute position, from 1
//   2  attname    attribute name
//   3  atttypid   OID of the type of the attribute
//   4  atttypmod  type modifier, -1 when there is none
//   5  attnotnull `t` when the attribute has NOT NULL
//   6  atthasdef  `t` when the attribute has a default
//   7  generated  `t` when the attribute is a generated column
//   8  identity   `t` when the attribute is an identity column
//@sql Attributes
const ATTRIBUTES: &str = r#"
SELECT
    a.attrelid::text,
    a.attnum::text,
    a.attname::text,
    a.atttypid::text,
    a.atttypmod::text,
    (CASE WHEN a.attnotnull THEN 't' ELSE 'f' END)::text AS attnotnull,
    (CASE WHEN a.atthasdef THEN 't' ELSE 'f' END)::text AS atthasdef,
    (CASE WHEN a.attgenerated <> '' THEN 't' ELSE 'f' END)::text AS generated,
    (CASE WHEN a.attidentity <> '' THEN 't' ELSE 'f' END)::text AS identity
FROM pg_catalog.pg_attribute a
JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE a.attnum > 0
  AND NOT a.attisdropped
  AND c.relkind IN ('r', 'v', 'm', 'f', 'p', 'S', 'c')
  AND n.nspname NOT LIKE 'pg\_toast%'
  AND (n.nspname NOT LIKE 'pg\_temp%'
       OR n.oid = pg_catalog.pg_my_temp_schema())
ORDER BY a.attrelid, a.attnum
"#;

// One row per function in a loaded schema.
//
// Columns:
//   0  oid             function OID
//   1  proname         function name
//   2  pronamespace    OID of the schema that holds the function
//   3  prokind         kind letter: f p a w
//   4  provariadic     OID of the variadic element type, 0 when there is none
//   5  pronargdefaults number of arguments that have a default
//   6  prorettype      OID of the return type
//   7  proretset       `t` when the function returns a set
//   8  proisstrict     `t` when the function is strict
//   9  provolatile     volatility letter: i s v
//@sql Functions
const FUNCTIONS: &str = r#"
SELECT
    p.oid::text,
    p.proname::text,
    p.pronamespace::text,
    p.prokind::text,
    p.provariadic::text,
    p.pronargdefaults::text,
    p.prorettype::text,
    (CASE WHEN p.proretset THEN 't' ELSE 'f' END)::text AS proretset,
    (CASE WHEN p.proisstrict THEN 't' ELSE 'f' END)::text AS proisstrict,
    p.provolatile::text
FROM pg_catalog.pg_proc p
JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
WHERE n.nspname NOT LIKE 'pg\_toast%'
  AND (n.nspname NOT LIKE 'pg\_temp%'
       OR n.oid = pg_catalog.pg_my_temp_schema())
ORDER BY p.oid
"#;

// One row per function argument, in argument order.
//
// `proallargtypes` holds every argument when the function has an OUT or an
// INOUT argument. It is NULL otherwise, and then `proargtypes` holds the IN
// arguments. The `coalesce` picks the one that applies. A function with no
// argument produces no row.
//
// `proargmodes` and `proargnames` are NULL when every argument is an IN
// argument, or when no argument has a name. The LEFT JOIN keeps the argument
// row and gives NULL for the missing mode or the missing name.
//
// Columns:
//   0  oid   OID of the function
//   1  n     argument position, from 1
//   2  t     OID of the type of the argument
//   3  mode  mode letter: i o b v t. NULL means IN
//   4  name  argument name. NULL or the empty string means no name
//@sql FunctionArguments
const FUNCTION_ARGUMENTS: &str = r#"
SELECT
    p.oid::text,
    a.n::text,
    a.t::text,
    m.mode::text,
    nm.name::text
FROM pg_catalog.pg_proc p
JOIN pg_catalog.pg_namespace ns ON ns.oid = p.pronamespace
CROSS JOIN LATERAL unnest(coalesce(p.proallargtypes, p.proargtypes::oid[]))
    WITH ORDINALITY AS a(t, n)
LEFT JOIN LATERAL unnest(p.proargmodes)
    WITH ORDINALITY AS m(mode, n) ON m.n = a.n
LEFT JOIN LATERAL unnest(p.proargnames)
    WITH ORDINALITY AS nm(name, n) ON nm.n = a.n
WHERE ns.nspname NOT LIKE 'pg\_toast%'
  AND (ns.nspname NOT LIKE 'pg\_temp%'
       OR ns.oid = pg_catalog.pg_my_temp_schema())
ORDER BY p.oid, a.n
"#;

// One row per aggregate function.
//
// `aggfnoid` is a `regproc`. The cast to `oid` keeps the number. A cast
// straight to `text` would give the function name.
//
// Columns:
//   0  aggfnoid         OID of the function that the row describes
//   1  aggkind          kind letter: n o h
//   2  aggnumdirectargs number of direct arguments of an ordered-set or of a
//                       hypothetical-set aggregate
//@sql Aggregates
const AGGREGATES: &str = r#"
SELECT
    a.aggfnoid::oid::text,
    a.aggkind::text,
    a.aggnumdirectargs::text
FROM pg_catalog.pg_aggregate a
ORDER BY a.aggfnoid::oid
"#;

// One row per operator in a loaded schema.
//
// `oprcode` is a `regproc`. The cast to `oid` keeps the number. `oprcode` is
// 0 for a shell operator.
//
// Columns:
//   0  oid          operator OID
//   1  oprname      operator name
//   2  oprnamespace OID of the schema that holds the operator
//   3  oprkind      kind letter: b for infix, l for prefix
//   4  oprleft      OID of the left operand type, 0 for a prefix operator
//   5  oprright     OID of the right operand type
//   6  oprresult    OID of the result type
//   7  oprcode      OID of the function that implements the operator
//   8  oprcom       OID of the commutator, 0 when there is none
//   9  oprnegate    OID of the negator, 0 when there is none
//@sql Operators
const OPERATORS: &str = r#"
SELECT
    o.oid::text,
    o.oprname::text,
    o.oprnamespace::text,
    o.oprkind::text,
    o.oprleft::text,
    o.oprright::text,
    o.oprresult::text,
    o.oprcode::oid::text,
    o.oprcom::text,
    o.oprnegate::text
FROM pg_catalog.pg_operator o
JOIN pg_catalog.pg_namespace n ON n.oid = o.oprnamespace
WHERE n.nspname NOT LIKE 'pg\_toast%'
  AND (n.nspname NOT LIKE 'pg\_temp%'
       OR n.oid = pg_catalog.pg_my_temp_schema())
ORDER BY o.oid
"#;

// One row per cast.
//
// Columns:
//   0  oid         cast OID
//   1  castsource  OID of the source type
//   2  casttarget  OID of the target type
//   3  castfunc    OID of the cast function, 0 when there is none
//   4  castcontext context letter: e a i
//   5  castmethod  method letter: f b i
//@sql Casts
const CASTS: &str = r#"
SELECT
    c.oid::text,
    c.castsource::text,
    c.casttarget::text,
    c.castfunc::text,
    c.castcontext::text,
    c.castmethod::text
FROM pg_catalog.pg_cast c
ORDER BY c.oid
"#;

// One row per operator that a default btree or hash operator class gives its
// input type.
//
// A row is a member of the operator family of the class, for a search
// (`amoppurpose` is `s`), with the input type of the class on both sides. A
// cross-type member, such as `int4 < int8`, is not here. A class is here only
// when it is the default for its input type, so one type has at most one
// class for each access method.
//
// The schema filter applies to the schema of the class.
//
// Columns:
//   0  oid           operator class OID
//   1  opcname       operator class name
//   2  amname        access method name: `btree` or `hash`
//   3  opcintype     OID of the input type of the class
//   4  amopstrategy  strategy number: 1 to 5 for a btree, 1 for a hash
//   5  amopopr       OID of the operator
//@sql DefaultOperatorClasses
const DEFAULT_OPERATOR_CLASSES: &str = r#"
SELECT
    c.oid::text,
    c.opcname::text,
    am.amname::text,
    c.opcintype::text,
    o.amopstrategy::text,
    o.amopopr::text
FROM pg_catalog.pg_opclass c
JOIN pg_catalog.pg_am am ON am.oid = c.opcmethod
JOIN pg_catalog.pg_namespace n ON n.oid = c.opcnamespace
JOIN pg_catalog.pg_amop o
    ON o.amopfamily = c.opcfamily
   AND o.amopmethod = c.opcmethod
   AND o.amoplefttype = c.opcintype
   AND o.amoprighttype = c.opcintype
   AND o.amoppurpose = 's'
WHERE c.opcdefault
  AND am.amname IN ('btree', 'hash')
  AND n.nspname NOT LIKE 'pg\_toast%'
  AND (n.nspname NOT LIKE 'pg\_temp%'
       OR n.oid = pg_catalog.pg_my_temp_schema())
ORDER BY c.oid, o.amopstrategy
"#;

// One row per column of every primary key, in key order.
//
// A primary key is a `pg_constraint` row with `contype` `p`. A relation has
// at most one. `conkey` holds the attribute numbers of the key columns. No
// other constraint is here.
//
// The schema filter applies to the schema of the relation, as in the
// Relations query.
//
// Columns:
//   0  conrelid       OID of the relation
//   1  condeferrable  `t` when the constraint is deferrable
//   2  n              position of the column in the key, from 1
//   3  attnum         attribute number of the column
//@sql PrimaryKeys
const PRIMARY_KEYS: &str = r#"
SELECT
    k.conrelid::text,
    (CASE WHEN k.condeferrable THEN 't' ELSE 'f' END)::text AS condeferrable,
    a.n::text,
    a.attnum::text
FROM pg_catalog.pg_constraint k
JOIN pg_catalog.pg_class c ON c.oid = k.conrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
CROSS JOIN LATERAL unnest(k.conkey) WITH ORDINALITY AS a(attnum, n)
WHERE k.contype = 'p'
  AND n.nspname NOT LIKE 'pg\_toast%'
  AND (n.nspname NOT LIKE 'pg\_temp%'
       OR n.oid = pg_catalog.pg_my_temp_schema())
ORDER BY k.conrelid, a.n
"#;
