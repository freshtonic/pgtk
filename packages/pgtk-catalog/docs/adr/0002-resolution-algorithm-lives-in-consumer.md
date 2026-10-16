# Function, operator, and cast resolution lives in the consumer

PostgreSQL's overload resolution algorithm (`parse_func.c`, `parse_oper.c`,
`parse_coerce.c`) needs `typcategory`, `typispreferred`, `castcontext`, and
polymorphic pseudo-types. `pgtk-catalog` carries that data and offers
algorithm-free lookups: functions by name visible on a search path, the cast
between two types if one exists, the array type of a type. It does not
implement the resolution algorithm; `pg-type-check` does. This keeps
`pgtk-catalog` a stable data crate and puts the opinionated, heavily tested
logic in one place.
