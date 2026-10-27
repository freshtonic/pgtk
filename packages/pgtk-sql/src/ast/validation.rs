//! The checks of `gram.y` actions that read a whole Node (issue #55).
//!
//! A `gram.y` action can refuse a statement that its rule matched: it counts
//! the values of a row, it compares the types of two arguments, or it merges
//! the clauses of a parenthesized query into the query around it. A declared
//! rejection rule cannot say that, so each such Node names one function here
//! with `#[validate(...)]`. Recursa calls the function when the parse has
//! built the Node, in the value, arena and flat parses alike, and a refusal
//! ends the parse with PostgreSQL's SQLSTATE and message.
//!
//! Each check has two functions, one for the nested AST and one for the Flat
//! AST, because the two storages hold the same Node in other types. The two
//! functions read the same fields in the same order, so the storages refuse
//! the same statements.

use recursa::{Pretty as _, PrettyContext, Refusal, RejectionNote};

use crate::FlatAst;
use crate::ast::ddl::aggregate::{
    AggregateArg, AggregateArgMode, CreateAggregateArgLists, FlatAggregateArg,
    FlatAggregateArgMode, FlatCreateAggregateArgLists,
};
use crate::ast::ddl::function::{
    ArgMode, CreateFunctionStmt, FlatArgMode, FlatCreateFunctionStmt, FlatFuncReturnType,
    FlatFunctionArg, FuncReturnType, FunctionArg,
};
use crate::ast::ddl::rule::{FlatRuleQuery, RuleQuery};
use crate::ast::dml::select::{
    FetchFirstBody, FetchMode, FlatFetchFirstBody, FlatFetchMode, FlatForLockingClause,
    FlatForUpdateWait, FlatLimitLockingClause, FlatLimitOffsetClause, FlatLimitingClause,
    ForLockingClause, ForUpdateWait, LimitLockingClause, LimitOffsetClause, LimitingClause,
};
use crate::ast::dml::select::{FlatUescapedAliasName, UescapedAliasName};
use crate::ast::dml::values::{
    FlatQueryBody, FlatSelectClause, FlatSubquery, QueryBody, SelectClause, Subquery,
};
use crate::ast::shared::expr::{
    Expr, FlatExpr, FlatFuncArg, FlatFunctionCallBody, FlatFunctionCallSuffix,
    FlatFunctionTypedLiteralTail, FlatRow, FlatSconst, FlatUescapeString,
    FlatUnicodeStringLitWithEscape, FuncArg, FunctionCallBody, FunctionCallSuffix,
    FunctionTypedLiteralTail, Row, Sconst, UnicodeStringLitWithEscape,
};
use crate::ast::shared::with_clause::{FlatWithBody, FlatWithStatement, WithBody, WithStatement};
use crate::ident::{
    FoldError, escape_string_error, unicode_identifier_error, unicode_string_error,
};
use crate::tokens::literal::{
    AliasName, CreateDbOptWord, FlatAliasName, FlatCreateDbOptWord, FlatIdent, FlatIdentOnly,
    FlatRelationAliasName, FlatSelectBareAliasName, FlatUtilityOptionName, FlatWindowRefNameIdent,
    Ident, IdentOnly, RelationAliasName, SelectBareAliasName, UtilityOptionName,
    WindowRefNameIdent,
};
use crate::tokens::{
    BareColLabel, ColId, ColLabel, FlatBareColLabel, FlatColId, FlatColLabel, FlatNonReservedWord,
    FlatRoleSpecWord, FlatUidentName, Flattable_function_name, Flattype_function_name,
    Flattype_name_ident, NonReservedWord, RoleSpecWord, table_function_name, type_function_name,
    type_name_ident,
};
#[cfg(feature = "since-pg16")]
use crate::tokens::{
    FlatRoleGrantOptionWord, FlatRoleRevokeOptionWord, RoleGrantOptionWord, RoleRevokeOptionWord,
};

/// `wrong number of parameters on left side of OVERLAPS expression`.
pub static OVERLAPS_LEFT: RejectionNote = RejectionNote::new(
    "Expr",
    Some("42601"),
    "wrong number of parameters on left side of OVERLAPS expression",
    None,
    "gram.y REL_17_11 15002, a_expr: row OVERLAPS row",
);

/// `wrong number of parameters on right side of OVERLAPS expression`.
pub static OVERLAPS_RIGHT: RejectionNote = RejectionNote::new(
    "Expr",
    Some("42601"),
    "wrong number of parameters on right side of OVERLAPS expression",
    None,
    "gram.y REL_17_11 15007, a_expr: row OVERLAPS row",
);

/// `type modifier cannot have parameter name`.
pub static TYPE_MODIFIER_NAME: RejectionNote = RejectionNote::new(
    "FunctionTypedLiteralTail",
    Some("42601"),
    "type modifier cannot have parameter name",
    None,
    "gram.y REL_17_11 17250, AexprConst: func_name '(' func_arg_list opt_sort_clause ')' Sconst",
);

/// `OUT and INOUT arguments aren't allowed in TABLE functions`.
pub static TABLE_FUNCTION_MODES: RejectionNote = RejectionNote::new(
    "CreateFunctionStmt",
    Some("42601"),
    "OUT and INOUT arguments aren't allowed in TABLE functions",
    None,
    "gram.y REL_17_11 19129, mergeTableFuncParameters",
);

/// The message of an ordered-set aggregate whose last direct argument is
/// `VARIADIC`.
pub static ORDERED_SET_VARIADIC: RejectionNote = RejectionNote::new(
    "CreateAggregateArgLists",
    Some("0A000"),
    "an ordered-set aggregate with a VARIADIC direct argument must have one VARIADIC aggregated \
     argument of the same data type",
    None,
    "gram.y REL_17_11 18843, makeOrderedSetArgs",
);

/// `multiple ORDER BY clauses not allowed`.
pub static MULTIPLE_ORDER_BY: RejectionNote = RejectionNote::new(
    "QueryBody",
    Some("42601"),
    "multiple ORDER BY clauses not allowed",
    None,
    "gram.y REL_17_11 18880, insertSelectOptions",
);

/// `multiple OFFSET clauses not allowed`.
pub static MULTIPLE_OFFSET: RejectionNote = RejectionNote::new(
    "QueryBody",
    Some("42601"),
    "multiple OFFSET clauses not allowed",
    None,
    "gram.y REL_17_11 18891, insertSelectOptions",
);

/// `multiple LIMIT clauses not allowed`.
pub static MULTIPLE_LIMIT: RejectionNote = RejectionNote::new(
    "QueryBody",
    Some("42601"),
    "multiple LIMIT clauses not allowed",
    None,
    "gram.y REL_17_11 18900, insertSelectOptions",
);

/// `WITH TIES cannot be specified without ORDER BY clause`.
pub static WITH_TIES_WITHOUT_ORDER_BY: RejectionNote = RejectionNote::new(
    "QueryBody",
    Some("42601"),
    "WITH TIES cannot be specified without ORDER BY clause",
    None,
    "gram.y REL_17_11 18913, insertSelectOptions",
);

/// `SKIP LOCKED and WITH TIES options cannot be used together`.
pub static WITH_TIES_SKIP_LOCKED: RejectionNote = RejectionNote::new(
    "QueryBody",
    Some("42601"),
    "SKIP LOCKED and WITH TIES options cannot be used together",
    None,
    "gram.y REL_17_11 18924, insertSelectOptions",
);

/// `multiple WITH clauses not allowed`.
pub static MULTIPLE_WITH: RejectionNote = RejectionNote::new(
    "Subquery",
    Some("42601"),
    "multiple WITH clauses not allowed",
    None,
    "gram.y REL_17_11 18936, insertSelectOptions",
);

// --- row OVERLAPS row ---

/// The number of values of one `OVERLAPS` row.
fn row_len(row: &Row<'_>) -> usize {
    match row {
        Row::Explicit(row) => row.values.as_ref().map_or(0, |values| values.len()),
        Row::Implicit(row) => 1 + row.rest.len(),
    }
}

/// The number of values of one flat `OVERLAPS` row.
fn flat_row_len<R: recursa::__private::FlatNode>(row: &FlatRow, ast: &FlatAst<'_, R>) -> usize {
    match row {
        FlatRow::Explicit(row) => ast[*row].values.map_or(0, |values| ast[values].len()),
        FlatRow::Implicit(row) => 1 + ast[ast[*row].rest].len(),
    }
}

/// gram.y `a_expr: row OVERLAPS row`: each row holds exactly two values
/// (REL_17_11 gram.y 14997-15012). The error is at the row.
pub fn overlaps(expr: &Expr<'_>) -> Result<(), Refusal> {
    let Expr::Overlaps(left, right) = expr else {
        return Ok(());
    };
    if row_len(left) != 2 {
        return Err(Refusal::new(&OVERLAPS_LEFT).at_field(0));
    }
    if row_len(right) != 2 {
        return Err(Refusal::new(&OVERLAPS_RIGHT).at_field(1));
    }
    Ok(())
}

/// The flat form of [`overlaps`].
pub fn overlaps_flat(expr: &FlatExpr, ast: &FlatAst<'_, FlatExpr>) -> Result<(), Refusal> {
    let FlatExpr::Overlaps(left, right) = expr else {
        return Ok(());
    };
    if flat_row_len(&ast[*left], ast) != 2 {
        return Err(Refusal::new(&OVERLAPS_LEFT).at_field(0));
    }
    if flat_row_len(&ast[*right], ast) != 2 {
        return Err(Refusal::new(&OVERLAPS_RIGHT).at_field(1));
    }
    Ok(())
}

// --- func_name '(' func_arg_list ')' Sconst ---

/// gram.y `AexprConst: func_name '(' func_arg_list opt_sort_clause ')'
/// Sconst` refuses a named argument in the type modifier list (REL_17_11
/// gram.y 17243-17251).
pub fn typed_literal_arguments(tail: &FunctionTypedLiteralTail<'_>) -> Result<(), Refusal> {
    if tail.args.iter().any(|arg| matches!(arg, FuncArg::Named(_))) {
        return Err(Refusal::new(&TYPE_MODIFIER_NAME).at_field(1));
    }
    Ok(())
}

/// The flat form of [`typed_literal_arguments`].
pub fn typed_literal_arguments_flat(
    tail: &FlatFunctionTypedLiteralTail,
    ast: &FlatAst<'_, FlatFunctionTypedLiteralTail>,
) -> Result<(), Refusal> {
    let args = ast[ast[tail.args].0].iter();
    if args
        .copied()
        .any(|arg| matches!(ast[arg], FlatFuncArg::Named(_)))
    {
        return Err(Refusal::new(&TYPE_MODIFIER_NAME).at_field(1));
    }
    Ok(())
}

// --- CREATE FUNCTION ... RETURNS TABLE ---

/// Reports whether a function argument is `OUT` or `INOUT`.
fn is_output(arg: &FunctionArg<'_>) -> bool {
    let mode = match arg {
        FunctionArg::NameMode(arg) => Some(&arg.mode),
        FunctionArg::Named(arg) => arg.mode.as_ref(),
        FunctionArg::Unnamed(arg) => arg.mode.as_ref(),
    };
    matches!(mode, Some(ArgMode::Out | ArgMode::Inout))
}

/// Reports whether a flat function argument is `OUT` or `INOUT`.
fn flat_is_output<R: recursa::__private::FlatNode>(
    arg: &FlatFunctionArg,
    ast: &FlatAst<'_, R>,
) -> bool {
    let mode = match arg {
        FlatFunctionArg::NameMode(arg) => Some(ast[*arg].mode),
        FlatFunctionArg::Named(arg) => ast[*arg].mode,
        FlatFunctionArg::Unnamed(arg) => ast[*arg].mode,
    };
    mode.is_some_and(|mode| matches!(ast[mode], FlatArgMode::Out | FlatArgMode::Inout))
}

/// gram.y `mergeTableFuncParameters`: a function that `RETURNS TABLE`
/// refuses an `OUT` or `INOUT` argument (REL_17_11 gram.y 19115-19132). The
/// error has no position.
pub fn table_function_arguments(stmt: &CreateFunctionStmt<'_>) -> Result<(), Refusal> {
    let table = stmt
        .returns
        .as_ref()
        .is_some_and(|returns| matches!(returns.return_type, FuncReturnType::Table(_)));
    if table && stmt.args.iter().any(|param| is_output(&param.arg)) {
        return Err(Refusal::new(&TABLE_FUNCTION_MODES));
    }
    Ok(())
}

/// The flat form of [`table_function_arguments`].
pub fn table_function_arguments_flat(
    stmt: &FlatCreateFunctionStmt,
    ast: &FlatAst<'_, FlatCreateFunctionStmt>,
) -> Result<(), Refusal> {
    let table = stmt.returns.is_some_and(|returns| {
        matches!(ast[ast[returns].return_type], FlatFuncReturnType::Table(_))
    });
    if !table {
        return Ok(());
    }
    let params = ast[ast[stmt.args].0].iter().copied();
    for param in params {
        if flat_is_output(&ast[ast[param].arg], ast) {
            return Err(Refusal::new(&TABLE_FUNCTION_MODES));
        }
    }
    Ok(())
}

// --- aggr_args_list ORDER BY aggr_args_list ---

/// The mode of one aggregate argument, and its type as Pretty renders it.
fn aggregate_arg(arg: &AggregateArg<'_>) -> (bool, String) {
    let (mode, type_name) = match arg {
        AggregateArg::NameMode(arg) => (Some(&arg.mode), &arg.type_name),
        AggregateArg::Named(arg) => (arg.mode.as_ref(), &arg.type_name),
        AggregateArg::Unnamed(arg) => (arg.mode.as_ref(), &arg.type_name),
    };
    (
        matches!(mode, Some(AggregateArgMode::Variadic)),
        type_name.render(),
    )
}

/// The mode of one flat aggregate argument, and its type as Pretty renders it.
fn flat_aggregate_arg<R: recursa::__private::FlatNode>(
    arg: &FlatAggregateArg,
    ast: &FlatAst<'_, R>,
) -> (bool, String) {
    let (mode, type_name) = match arg {
        FlatAggregateArg::NameMode(arg) => (Some(ast[*arg].mode), ast[*arg].type_name),
        FlatAggregateArg::Named(arg) => (ast[*arg].mode, ast[*arg].type_name),
        FlatAggregateArg::Unnamed(arg) => (ast[*arg].mode, ast[*arg].type_name),
    };
    let mut context = PrettyContext::new();
    ast.pretty_node(type_name, &mut context)
        .expect("rendering into a structured sink cannot fail");
    (
        mode.is_some_and(|mode| matches!(ast[mode], FlatAggregateArgMode::Variadic)),
        context.finish(),
    )
}

/// gram.y `makeOrderedSetArgs`: when the last direct argument is `VARIADIC`,
/// the aggregated arguments must be one `VARIADIC` argument of the same type
/// (REL_17_11 gram.y 18822-18852). The error is at the aggregated arguments.
///
/// Two types are the same when Pretty renders them the same way. PostgreSQL
/// compares the parsed type names, so a pair of synonyms such as `int` and
/// `integer` is the same type there and another type here.
pub fn ordered_set_arguments(args: &CreateAggregateArgLists<'_>) -> Result<(), Refusal> {
    let Some(ordered) = &args.ordered else {
        return Ok(());
    };
    let (last_variadic, last_type) = aggregate_arg(&args.direct.last().arg);
    if !last_variadic {
        return Ok(());
    }
    let (first_variadic, first_type) = aggregate_arg(ordered.ordered.first());
    if ordered.ordered.len() != 1 || !first_variadic || first_type != last_type {
        return Err(Refusal::new(&ORDERED_SET_VARIADIC).at_field(1));
    }
    Ok(())
}

/// The flat form of [`ordered_set_arguments`].
pub fn ordered_set_arguments_flat(
    args: &FlatCreateAggregateArgLists,
    ast: &FlatAst<'_, FlatCreateAggregateArgLists>,
) -> Result<(), Refusal> {
    let Some(ordered) = args.ordered else {
        return Ok(());
    };
    let direct = &ast[args.direct];
    let last = direct[direct.len() - 1];
    let (last_variadic, last_type) = flat_aggregate_arg(&ast[ast[last].arg], ast);
    if !last_variadic {
        return Ok(());
    }
    let ordered = &ast[ast[ordered].ordered];
    let (first_variadic, first_type) = flat_aggregate_arg(&ast[ordered[0]], ast);
    if ordered.len() != 1 || !first_variadic || first_type != last_type {
        return Err(Refusal::new(&ORDERED_SET_VARIADIC).at_field(1));
    }
    Ok(())
}

// --- insertSelectOptions ---

/// The options of one select statement, merged as `insertSelectOptions`
/// merges the options of a parenthesized query into the query around it.
#[derive(Clone, Copy, Default)]
struct SelectOptions {
    /// Whether the statement has an `ORDER BY`.
    order_by: bool,
    /// Whether the statement has an `OFFSET`.
    offset: bool,
    /// Whether the statement has a `LIMIT` or a `FETCH`.
    limit: bool,
    /// Whether the statement has a `WITH`.
    with: bool,
    /// Whether a locking clause of the statement is `SKIP LOCKED`.
    skip_locked: bool,
}

/// The `OFFSET`, the `LIMIT` and whether the `FETCH` is `WITH TIES`, of one
/// limit clause.
fn limit_offset<'a>(clause: &'a LimitOffsetClause<'a>) -> (bool, Option<&'a LimitingClause<'a>>) {
    match clause {
        LimitOffsetClause::LimitOffset(clause) => (clause.offset.is_some(), Some(&clause.limit)),
        LimitOffsetClause::OffsetLimit(clause) => (true, clause.limit.as_deref()),
    }
}

/// Reports whether one limiting clause is `FETCH ... WITH TIES`.
fn with_ties(limit: &LimitingClause<'_>) -> bool {
    let LimitingClause::FetchFirst(fetch) = limit else {
        return false;
    };
    let mode = match &fetch.body {
        FetchFirstBody::NoCount(body) => &body.mode,
        FetchFirstBody::WithCount(body) => &body.mode,
    };
    matches!(mode, FetchMode::WithTies)
}

/// The limit clause of one query body.
fn body_limit<'a>(body: &'a QueryBody<'_>) -> Option<&'a LimitOffsetClause<'a>> {
    locking_limit(body.limit_locking.as_deref()?)
}

/// The limit clause of one limit and locking tail.
fn locking_limit<'a>(clause: &'a LimitLockingClause<'_>) -> Option<&'a LimitOffsetClause<'a>> {
    match clause {
        LimitLockingClause::LockingFirst(clause) => clause.limit_offset.as_deref(),
        LimitLockingClause::LimitFirst(clause) => Some(&clause.limit_offset),
    }
}

/// Reports whether a locking clause has a `SKIP LOCKED` item.
fn skip_locked(locking: Option<&ForLockingClause<'_>>) -> bool {
    locking.is_some_and(|locking| {
        locking
            .items()
            .iter()
            .any(|item| matches!(item.wait, Some(ForUpdateWait::SkipLocked)))
    })
}

/// The merged options of the statement one select clause names: none for a
/// plain clause, the options of the parenthesized query for a `(...)`.
fn clause_options(clause: &SelectClause<'_>) -> SelectOptions {
    let SelectClause::Parens(parens) = clause else {
        return SelectOptions::default();
    };
    subquery_options(&parens.inner)
}

/// The merged options of one subquery.
fn subquery_options(query: &Subquery<'_>) -> SelectOptions {
    let mut options = body_options(&query.body);
    options.with |= query.with.is_some();
    options
}

/// The merged options of one query body.
fn body_options(body: &QueryBody<'_>) -> SelectOptions {
    let mut options = clause_options(&body.clause);
    options.order_by |= body.order_by.is_some();
    options.skip_locked |= skip_locked(
        body.limit_locking
            .as_deref()
            .and_then(LimitLockingClause::locking),
    );
    if let Some(clause) = body_limit(body) {
        let (offset, limit) = limit_offset(clause);
        options.offset |= offset;
        options.limit |= limit.is_some();
    }
    options
}

/// gram.y `insertSelectOptions`: the `ORDER BY`, `OFFSET`, `LIMIT` and `WITH
/// TIES` of a query refuse a parenthesized query that already has one, and
/// `WITH TIES` needs an `ORDER BY` (REL_17_11 gram.y 18862-18929). The error
/// is at the clause.
pub fn query_options(body: &QueryBody<'_>) -> Result<(), Refusal> {
    let inner = clause_options(&body.clause);
    if body.order_by.is_some() && inner.order_by {
        return Err(Refusal::new(&MULTIPLE_ORDER_BY).at_field(1));
    }
    let Some(clause) = body_limit(body) else {
        return Ok(());
    };
    let (offset, limit) = limit_offset(clause);
    if offset && inner.offset {
        return Err(Refusal::new(&MULTIPLE_OFFSET).at_field(2));
    }
    if limit.is_some() && inner.limit {
        return Err(Refusal::new(&MULTIPLE_LIMIT).at_field(2));
    }
    if limit.is_some_and(with_ties) {
        if !(body.order_by.is_some() || inner.order_by) {
            return Err(Refusal::new(&WITH_TIES_WITHOUT_ORDER_BY));
        }
        let locking = body
            .limit_locking
            .as_deref()
            .and_then(LimitLockingClause::locking);
        if skip_locked(locking) || inner.skip_locked {
            return Err(Refusal::new(&WITH_TIES_SKIP_LOCKED));
        }
    }
    Ok(())
}

/// gram.y `insertSelectOptions`: the `WITH` of a query refuses a
/// parenthesized query that already has one (REL_17_11 gram.y 18930-18938).
pub fn query_with(query: &Subquery<'_>) -> Result<(), Refusal> {
    if query.with.is_some() && body_options(&query.body).with {
        return Err(Refusal::new(&MULTIPLE_WITH).at_field(0));
    }
    Ok(())
}

/// The flat form of [`limit_offset`].
fn flat_limit_offset<R: recursa::__private::FlatNode>(
    clause: &FlatLimitOffsetClause,
    ast: &FlatAst<'_, R>,
) -> (bool, Option<FlatLimitingClause>) {
    match clause {
        FlatLimitOffsetClause::LimitOffset(clause) => {
            let clause = &ast[*clause];
            (clause.offset.is_some(), Some(ast[clause.limit]))
        }
        FlatLimitOffsetClause::OffsetLimit(clause) => {
            (true, ast[*clause].limit.map(|limit| ast[limit]))
        }
    }
}

/// The flat form of [`with_ties`].
fn flat_with_ties<R: recursa::__private::FlatNode>(
    limit: FlatLimitingClause,
    ast: &FlatAst<'_, R>,
) -> bool {
    let FlatLimitingClause::FetchFirst(fetch) = limit else {
        return false;
    };
    let mode = match ast[ast[fetch].body] {
        FlatFetchFirstBody::NoCount(body) => ast[body].mode,
        FlatFetchFirstBody::WithCount(body) => ast[body].mode,
    };
    matches!(ast[mode], FlatFetchMode::WithTies)
}

/// The flat form of [`body_limit`].
fn flat_body_limit<R: recursa::__private::FlatNode>(
    body: &FlatQueryBody,
    ast: &FlatAst<'_, R>,
) -> Option<FlatLimitOffsetClause> {
    flat_locking_limit(body.limit_locking?, ast)
}

/// The flat form of [`locking_limit`].
fn flat_locking_limit<R: recursa::__private::FlatNode>(
    clause: recursa::__private::Handle<FlatLimitLockingClause>,
    ast: &FlatAst<'_, R>,
) -> Option<FlatLimitOffsetClause> {
    match ast[clause] {
        FlatLimitLockingClause::LockingFirst(clause) => {
            ast[clause].limit_offset.map(|limit| ast[limit])
        }
        FlatLimitLockingClause::LimitFirst(clause) => Some(ast[ast[clause].limit_offset]),
    }
}

/// The flat form of [`skip_locked`]: the locking clause of one flat limit
/// and locking tail.
fn flat_skip_locked<R: recursa::__private::FlatNode>(
    clause: Option<recursa::__private::Handle<FlatLimitLockingClause>>,
    ast: &FlatAst<'_, R>,
) -> bool {
    let Some(clause) = clause else {
        return false;
    };
    let locking = match ast[clause] {
        FlatLimitLockingClause::LockingFirst(clause) => Some(ast[clause].locking),
        FlatLimitLockingClause::LimitFirst(clause) => ast[clause].locking,
    };
    let Some(locking) = locking else {
        return false;
    };
    match ast[locking] {
        FlatForLockingClause::ReadOnly => false,
        FlatForLockingClause::Items(items) => ast[items].iter().any(|item| {
            ast[*item]
                .wait
                .is_some_and(|wait| matches!(ast[wait], FlatForUpdateWait::SkipLocked))
        }),
    }
}

/// The flat form of [`clause_options`].
fn flat_clause_options<R: recursa::__private::FlatNode>(
    clause: &FlatSelectClause,
    ast: &FlatAst<'_, R>,
) -> SelectOptions {
    let FlatSelectClause::Parens(parens) = clause else {
        return SelectOptions::default();
    };
    flat_subquery_options(&ast[ast[*parens].inner], ast)
}

/// The flat form of [`subquery_options`].
fn flat_subquery_options<R: recursa::__private::FlatNode>(
    query: &FlatSubquery,
    ast: &FlatAst<'_, R>,
) -> SelectOptions {
    let mut options = flat_body_options(&ast[query.body], ast);
    options.with |= query.with.is_some();
    options
}

/// The flat form of [`body_options`].
fn flat_body_options<R: recursa::__private::FlatNode>(
    body: &FlatQueryBody,
    ast: &FlatAst<'_, R>,
) -> SelectOptions {
    let mut options = flat_clause_options(&ast[body.clause], ast);
    options.order_by |= body.order_by.is_some();
    options.skip_locked |= flat_skip_locked(body.limit_locking, ast);
    if let Some(clause) = flat_body_limit(body, ast) {
        let (offset, limit) = flat_limit_offset(&clause, ast);
        options.offset |= offset;
        options.limit |= limit.is_some();
    }
    options
}

/// The flat form of [`query_options`].
pub fn query_options_flat(
    body: &FlatQueryBody,
    ast: &FlatAst<'_, FlatQueryBody>,
) -> Result<(), Refusal> {
    let inner = flat_clause_options(&ast[body.clause], ast);
    if body.order_by.is_some() && inner.order_by {
        return Err(Refusal::new(&MULTIPLE_ORDER_BY).at_field(1));
    }
    let Some(clause) = flat_body_limit(body, ast) else {
        return Ok(());
    };
    let (offset, limit) = flat_limit_offset(&clause, ast);
    if offset && inner.offset {
        return Err(Refusal::new(&MULTIPLE_OFFSET).at_field(2));
    }
    if limit.is_some() && inner.limit {
        return Err(Refusal::new(&MULTIPLE_LIMIT).at_field(2));
    }
    if limit.is_some_and(|limit| flat_with_ties(limit, ast)) {
        if !(body.order_by.is_some() || inner.order_by) {
            return Err(Refusal::new(&WITH_TIES_WITHOUT_ORDER_BY));
        }
        if flat_skip_locked(body.limit_locking, ast) || inner.skip_locked {
            return Err(Refusal::new(&WITH_TIES_SKIP_LOCKED));
        }
    }
    Ok(())
}

/// The flat form of [`query_with`].
pub fn query_with_flat(
    query: &FlatSubquery,
    ast: &FlatAst<'_, FlatSubquery>,
) -> Result<(), Refusal> {
    if query.with.is_some() && flat_body_options(&ast[query.body], ast).with {
        return Err(Refusal::new(&MULTIPLE_WITH).at_field(0));
    }
    Ok(())
}

/// gram.y `insertSelectOptions` for a `WITH` statement: its `WITH` refuses a
/// parenthesized query that already has one (REL_17_11 gram.y 18930-18938).
pub fn with_statement(stmt: &WithStatement<'_>) -> Result<(), Refusal> {
    let WithBody::Query(body) = &stmt.body else {
        return Ok(());
    };
    if body_options(body).with {
        return Err(Refusal::new(&MULTIPLE_WITH).at_field(0));
    }
    Ok(())
}

/// The flat form of [`with_statement`].
pub fn with_statement_flat(
    stmt: &FlatWithStatement,
    ast: &FlatAst<'_, FlatWithStatement>,
) -> Result<(), Refusal> {
    let FlatWithBody::Query(body) = ast[stmt.body] else {
        return Ok(());
    };
    if flat_body_options(&ast[body], ast).with {
        return Err(Refusal::new(&MULTIPLE_WITH).at_field(0));
    }
    Ok(())
}

/// gram.y `insertSelectOptions` for the query of a rule action, which has no
/// parenthesized form: `WITH TIES` needs an `ORDER BY` (REL_17_11 gram.y
/// 18909-18913).
pub fn rule_query_options(query: &RuleQuery<'_>) -> Result<(), Refusal> {
    let limit = query
        .limit_locking
        .as_deref()
        .and_then(locking_limit)
        .and_then(|clause| limit_offset(clause).1);
    if limit.is_some_and(with_ties) && query.order_by.is_none() {
        return Err(Refusal::new(&WITH_TIES_WITHOUT_ORDER_BY));
    }
    Ok(())
}

/// The flat form of [`rule_query_options`].
pub fn rule_query_options_flat(
    query: &FlatRuleQuery,
    ast: &FlatAst<'_, FlatRuleQuery>,
) -> Result<(), Refusal> {
    let limit = query
        .limit_locking
        .and_then(|clause| flat_locking_limit(clause, ast))
        .and_then(|clause| flat_limit_offset(&clause, ast).1);
    if limit.is_some_and(|limit| flat_with_ties(limit, ast)) && query.order_by.is_none() {
        return Err(Refusal::new(&WITH_TIES_WITHOUT_ORDER_BY));
    }
    Ok(())
}

// --- func_expr: func_application within_group_clause ---

/// `cannot use multiple ORDER BY clauses with WITHIN GROUP`.
pub static WITHIN_GROUP_ORDER_BY: RejectionNote = RejectionNote::new(
    "FunctionCallSuffix",
    Some("42601"),
    "cannot use multiple ORDER BY clauses with WITHIN GROUP",
    None,
    "gram.y REL_17_11 15598, func_expr",
);

/// `cannot use DISTINCT with WITHIN GROUP`.
pub static WITHIN_GROUP_DISTINCT: RejectionNote = RejectionNote::new(
    "FunctionCallSuffix",
    Some("42601"),
    "cannot use DISTINCT with WITHIN GROUP",
    None,
    "gram.y REL_17_11 15603, func_expr",
);

/// `cannot use VARIADIC with WITHIN GROUP`.
pub static WITHIN_GROUP_VARIADIC: RejectionNote = RejectionNote::new(
    "FunctionCallSuffix",
    Some("42601"),
    "cannot use VARIADIC with WITHIN GROUP",
    None,
    "gram.y REL_17_11 15608, func_expr",
);

/// The `WITHIN GROUP` checks of one call, in the order of gram.y: an
/// argument `ORDER BY`, `DISTINCT`, then `VARIADIC`.
fn within_group_refusal(order_by: bool, distinct: bool, variadic: bool) -> Result<(), Refusal> {
    // The error is at the `WITHIN GROUP` clause, field 3.
    let note = if order_by {
        &WITHIN_GROUP_ORDER_BY
    } else if distinct {
        &WITHIN_GROUP_DISTINCT
    } else if variadic {
        &WITHIN_GROUP_VARIADIC
    } else {
        return Ok(());
    };
    Err(Refusal::new(note).at_field(3))
}

/// gram.y `func_expr: func_application within_group_clause filter_clause
/// over_clause`: the order of `WITHIN GROUP` and the aggregate `ORDER BY`
/// share one field, so the action refuses a call with both. It also refuses
/// `DISTINCT` and `VARIADIC` with `WITHIN GROUP` (REL_17_11 gram.y
/// 15581-15612; REL_14_24 has the same checks).
pub fn within_group(call: &FunctionCallSuffix<'_>) -> Result<(), Refusal> {
    if call.within_group.is_none() {
        return Ok(());
    }
    let (order_by, distinct, variadic) = match &call.body {
        None | Some(FunctionCallBody::Star(_)) => (false, false, false),
        Some(FunctionCallBody::All(args)) => (args.order_by.is_some(), false, false),
        Some(FunctionCallBody::Distinct(args)) => (args.order_by.is_some(), true, false),
        Some(FunctionCallBody::LeadingVariadic(args)) => (args.order_by.is_some(), false, true),
        Some(FunctionCallBody::Args(args)) => (
            args.order_by.is_some(),
            false,
            args.trailing_variadic.is_some(),
        ),
    };
    within_group_refusal(order_by, distinct, variadic)
}

/// The flat form of [`within_group`].
pub fn within_group_flat(
    call: &FlatFunctionCallSuffix,
    ast: &FlatAst<'_, FlatFunctionCallSuffix>,
) -> Result<(), Refusal> {
    if call.within_group.is_none() {
        return Ok(());
    }
    let (order_by, distinct, variadic) = match call.body.map(|body| ast[body]) {
        None | Some(FlatFunctionCallBody::Star(_)) => (false, false, false),
        Some(FlatFunctionCallBody::All(args)) => (ast[args].order_by.is_some(), false, false),
        Some(FlatFunctionCallBody::Distinct(args)) => (ast[args].order_by.is_some(), true, false),
        Some(FlatFunctionCallBody::LeadingVariadic(args)) => {
            (ast[args].order_by.is_some(), false, true)
        }
        Some(FlatFunctionCallBody::Args(args)) => (
            ast[args].order_by.is_some(),
            false,
            ast[args].trailing_variadic.is_some(),
        ),
    };
    within_group_refusal(order_by, distinct, variadic)
}

// --- Unicode escapes in string constants ---

/// `invalid Unicode escape` in a `U&'...'` string.
pub static UNICODE_STRING_ESCAPE: RejectionNote = RejectionNote::new(
    "UnicodeStringLitWithEscape",
    Some("42601"),
    "invalid Unicode escape",
    Some("Unicode escapes must be \\XXXX or \\+XXXXXX."),
    "parser.c REL_17_11 494, str_udeescape",
);

/// `invalid Unicode escape value` in a `U&'...'` string.
pub static UNICODE_STRING_VALUE: RejectionNote = RejectionNote::new(
    "UnicodeStringLitWithEscape",
    Some("42601"),
    "invalid Unicode escape value",
    None,
    "parser.c REL_17_11 347, check_unicode_value",
);

/// `invalid Unicode surrogate pair` in a `U&'...'` string.
pub static UNICODE_STRING_PAIR: RejectionNote = RejectionNote::new(
    "UnicodeStringLitWithEscape",
    Some("42601"),
    "invalid Unicode surrogate pair",
    None,
    "parser.c REL_17_11 523, str_udeescape",
);

/// `invalid Unicode escape` in an `E'...'` string.
pub static ESCAPE_STRING_ESCAPE: RejectionNote = RejectionNote::new(
    "EscapeStringLit",
    Some("22025"),
    "invalid Unicode escape",
    Some("Unicode escapes must be \\uXXXX or \\UXXXXXXXX."),
    "scan.l REL_17_11 716, <xe,xeu>{xeunicodefail}",
);

/// `invalid Unicode escape value` in an `E'...'` string.
pub static ESCAPE_STRING_VALUE: RejectionNote = RejectionNote::new(
    "EscapeStringLit",
    Some("42601"),
    "invalid Unicode escape value",
    None,
    "scan.l REL_17_11 1393, addunicode",
);

/// `invalid Unicode surrogate pair` in an `E'...'` string.
pub static ESCAPE_STRING_PAIR: RejectionNote = RejectionNote::new(
    "EscapeStringLit",
    Some("42601"),
    "invalid Unicode surrogate pair",
    None,
    "scan.l REL_17_11 677, <xe>{xeunicode}",
);

/// The refusal of a `U&'...'` string with the escape error `error`.
fn unicode_string_refusal(error: Option<FoldError>) -> Result<(), Refusal> {
    unicode_refusal(
        error,
        [
            &UNICODE_STRING_ESCAPE,
            &UNICODE_STRING_VALUE,
            &UNICODE_STRING_PAIR,
        ],
    )
}

/// The refusal for the `str_udeescape` error `error`, with the notes of one
/// site: the incorrect escape, the incorrect value and the surrogate pair.
fn unicode_refusal(
    error: Option<FoldError>,
    [escape, value, pair]: [&'static RejectionNote; 3],
) -> Result<(), Refusal> {
    let note = match error {
        None => return Ok(()),
        Some(FoldError::InvalidEscapeValue) => value,
        Some(FoldError::InvalidSurrogatePair) => pair,
        Some(_) => escape,
    };
    // The error is at the string or the name, field 0.
    Err(Refusal::new(note).at_field(0))
}

/// `invalid byte sequence for encoding "UTF8"` in an `E'...'` string.
pub static ESCAPE_STRING_BYTES: RejectionNote = RejectionNote::new(
    "EscapeStringLit",
    Some("22021"),
    "invalid byte sequence for encoding \"UTF8\"",
    None,
    "scan.l REL_17_11 633, pg_verifymbstr",
);

/// The refusal of an `E'...'` string with the escape error `error`.
fn escape_string_refusal(error: Option<FoldError>) -> Result<(), Refusal> {
    let note = match error {
        None => return Ok(()),
        Some(FoldError::InvalidEscapeValue) => &ESCAPE_STRING_VALUE,
        Some(FoldError::InvalidSurrogatePair) => &ESCAPE_STRING_PAIR,
        Some(FoldError::InvalidByteSequence) => &ESCAPE_STRING_BYTES,
        Some(_) => &ESCAPE_STRING_ESCAPE,
    };
    Err(Refusal::new(note))
}

/// The flat form of [`uescape_text`].
fn flat_uescape_text<'a, R: recursa::__private::FlatNode>(
    string: FlatUescapeString,
    ast: &'a FlatAst<'_, R>,
) -> &'a str {
    match string {
        FlatUescapeString::Escape(string) => ast[string].as_str(),
        FlatUescapeString::Dollar(string) => ast[string].as_str(),
        FlatUescapeString::Sequence(string) => ast[string].as_str(),
        FlatUescapeString::Plain(string) => ast[string].as_str(),
    }
}

/// `parser.c` `base_yylex` decodes a `U&'...'` string with `str_udeescape`,
/// with the escape character of its `UESCAPE` clause, and refuses an
/// incorrect Unicode escape (REL_17_11 parser.c 316-319 and 380-530).
pub fn unicode_string(string: &UnicodeStringLitWithEscape<'_>) -> Result<(), Refusal> {
    let uescape = string
        .uescape
        .as_ref()
        .map(|uescape| uescape.escape_char.text());
    unicode_string_refusal(unicode_string_error(string.lit.text(), uescape))
}

/// The flat form of [`unicode_string`].
pub fn unicode_string_flat(
    string: &FlatUnicodeStringLitWithEscape,
    ast: &FlatAst<'_, FlatUnicodeStringLitWithEscape>,
) -> Result<(), Refusal> {
    let uescape = string
        .uescape
        .map(|uescape| flat_uescape_text(ast[ast[uescape].escape_char], ast));
    unicode_string_refusal(unicode_string_error(ast[string.lit].as_str(), uescape))
}

/// `scan.l` refuses an `E'...'` string with an incorrect Unicode escape, or
/// whose value is no valid UTF-8 (`crate::ident::escape_string_error`). The check is on each node that holds the
/// token: the `Escape` arm of [`Sconst`] and of [`Expr`].
pub fn escape_sconst(value: &Sconst<'_>) -> Result<(), Refusal> {
    let Sconst::Escape(string) = value else {
        return Ok(());
    };
    escape_string_refusal(escape_string_error(string.text()))
}

/// The flat form of [`escape_sconst`].
pub fn escape_sconst_flat(
    value: &FlatSconst,
    ast: &FlatAst<'_, FlatSconst>,
) -> Result<(), Refusal> {
    let FlatSconst::Escape(string) = value else {
        return Ok(());
    };
    escape_string_refusal(escape_string_error(ast[*string].as_str()))
}

/// The [`Expr`] form of [`escape_sconst`].
pub fn escape_expr(expr: &Expr<'_>) -> Result<(), Refusal> {
    let Expr::EscapeStringLit(string) = expr else {
        return Ok(());
    };
    escape_string_refusal(escape_string_error(string.text()))
}

/// The flat form of [`escape_expr`].
pub fn escape_expr_flat(expr: &FlatExpr, ast: &FlatAst<'_, FlatExpr>) -> Result<(), Refusal> {
    let FlatExpr::EscapeStringLit(string) = expr else {
        return Ok(());
    };
    escape_string_refusal(escape_string_error(ast[*string].as_str()))
}

// --- Unicode escapes in identifiers ---

/// `invalid Unicode escape` in a `U&"..."` identifier.
pub static UNICODE_IDENT_ESCAPE: RejectionNote = RejectionNote::new(
    "UIDENT",
    Some("42601"),
    "invalid Unicode escape",
    Some("Unicode escapes must be \\XXXX or \\+XXXXXX."),
    "parser.c REL_17_11 494, str_udeescape",
);

/// `invalid Unicode escape value` in a `U&"..."` identifier.
pub static UNICODE_IDENT_VALUE: RejectionNote = RejectionNote::new(
    "UIDENT",
    Some("42601"),
    "invalid Unicode escape value",
    None,
    "parser.c REL_17_11 347, check_unicode_value",
);

/// `invalid Unicode surrogate pair` in a `U&"..."` identifier.
pub static UNICODE_IDENT_PAIR: RejectionNote = RejectionNote::new(
    "UIDENT",
    Some("42601"),
    "invalid Unicode surrogate pair",
    None,
    "parser.c REL_17_11 523, str_udeescape",
);

/// The notes of a `U&"..."` identifier.
const UNICODE_IDENT_NOTES: [&RejectionNote; 3] = [
    &UNICODE_IDENT_ESCAPE,
    &UNICODE_IDENT_VALUE,
    &UNICODE_IDENT_PAIR,
];

/// The refusal of the identifier with the source text `raw` and no `UESCAPE`
/// clause: `base_yylex` decodes a `U&"..."` name with `str_udeescape` and the
/// escape character `\` (REL_17_11 parser.c 315-343; the same in REL_14_24).
fn identifier_refusal(raw: &str) -> Result<(), Refusal> {
    unicode_refusal(unicode_identifier_error(raw, None), UNICODE_IDENT_NOTES)
}

/// `parser.c` decodes the name of `AS U&"..." UESCAPE 'c'` with the escape
/// character of the clause and refuses an incorrect escape.
pub fn uescaped_alias_name(name: &UescapedAliasName<'_>) -> Result<(), Refusal> {
    unicode_refusal(
        unicode_identifier_error(name.name.text(), Some(name.uescape.escape_char.text())),
        UNICODE_IDENT_NOTES,
    )
}

/// The flat form of [`uescaped_alias_name`].
pub fn uescaped_alias_name_flat(
    name: &FlatUescapedAliasName,
    ast: &FlatAst<'_, FlatUescapedAliasName>,
) -> Result<(), Refusal> {
    let FlatUidentName::Text(text) = ast[name.name];
    let uescape = flat_uescape_text(ast[ast[name.uescape].escape_char], ast);
    unicode_refusal(
        unicode_identifier_error(ast[text].as_str(), Some(uescape)),
        UNICODE_IDENT_NOTES,
    )
}

/// The hooks of the identifier types: a `U&"..."` name is checked with the
/// escape character `\` ([`identifier_refusal`]). Each type holds one token;
/// a `UESCAPE` clause is part of no identifier type (only of
/// `UescapedAliasName`).
macro_rules! identifier_hooks {
    ($($(#[$meta:meta])* $hook:ident, $hook_flat:ident, $ty:ident, $flat:ident;)*) => {$(
        $(#[$meta])*
        #[doc = concat!("The `str_udeescape` check of a `", stringify!($ty), "`.")]
        pub fn $hook(name: &$ty<'_>) -> Result<(), Refusal> {
            identifier_refusal(name.text())
        }

        $(#[$meta])*
        #[doc = concat!("The flat form of [`", stringify!($hook), "`].")]
        pub fn $hook_flat(name: &$flat, ast: &FlatAst<'_, $flat>) -> Result<(), Refusal> {
            match name {
                $flat::Text(text) => identifier_refusal(ast[*text].as_str()),
                #[allow(unreachable_patterns)]
                _ => Ok(()),
            }
        }
    )*};
}

identifier_hooks! {
    ident_col_id, ident_col_id_flat, ColId, FlatColId;
    ident_col_label, ident_col_label_flat, ColLabel, FlatColLabel;
    ident_non_reserved_word, ident_non_reserved_word_flat, NonReservedWord, FlatNonReservedWord;
    ident_role_spec_word, ident_role_spec_word_flat, RoleSpecWord, FlatRoleSpecWord;
    ident_type_function_name, ident_type_function_name_flat, type_function_name,
        Flattype_function_name;
    ident_table_function_name, ident_table_function_name_flat, table_function_name,
        Flattable_function_name;
    ident_type_name_ident, ident_type_name_ident_flat, type_name_ident, Flattype_name_ident;
    ident_bare_col_label, ident_bare_col_label_flat, BareColLabel, FlatBareColLabel;
    #[cfg(feature = "since-pg16")]
    ident_role_revoke_option_word, ident_role_revoke_option_word_flat, RoleRevokeOptionWord,
        FlatRoleRevokeOptionWord;
    #[cfg(feature = "since-pg16")]
    ident_role_grant_option_word, ident_role_grant_option_word_flat, RoleGrantOptionWord,
        FlatRoleGrantOptionWord;
    ident_ident, ident_ident_flat, Ident, FlatIdent;
    ident_create_db_opt_word, ident_create_db_opt_word_flat, CreateDbOptWord, FlatCreateDbOptWord;
    ident_utility_option_name, ident_utility_option_name_flat, UtilityOptionName,
        FlatUtilityOptionName;
    ident_ident_only, ident_ident_only_flat, IdentOnly, FlatIdentOnly;
    ident_relation_alias_name, ident_relation_alias_name_flat, RelationAliasName,
        FlatRelationAliasName;
    ident_select_bare_alias_name, ident_select_bare_alias_name_flat, SelectBareAliasName,
        FlatSelectBareAliasName;
    ident_window_ref_name_ident, ident_window_ref_name_ident_flat, WindowRefNameIdent,
        FlatWindowRefNameIdent;
    ident_alias_name, ident_alias_name_flat, AliasName, FlatAliasName;
}
