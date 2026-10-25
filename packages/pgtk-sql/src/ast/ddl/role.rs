//! ROLE DDL statements (CREATE/ALTER/DROP).
#![allow(unused_imports)]

use crate::ast::ddl::foreign::AlterGenericOptions;
use crate::ast::ddl::function::{FunctionBuiltinType, FunctionTypeName};
use crate::ast::ddl::table::CreateGenericOptions;
use crate::ast::shared::expr::*;
use crate::ast::shared::flags::*;
use crate::ast::shared::names::*;
use crate::ast::shared::numbers::*;
use crate::ast::utility::copy::CopySconst;
use crate::tokens::{literal, punct};

recursa::ast_node! {
    /// Optional `TEMP` or `TEMPORARY` modifier that can appear between `CREATE`
    /// and the object keyword for temporary objects (sequences, tables, views,
    /// etc.).
    ///
    /// Variant ordering: `Temporary` (longer) before `Temp` so the longer keyword
    /// wins longest-match disambiguation.
    #[derive(Debug)]
    pub enum TempModifier {
        #[tok(TEMPORARY)]
        Temporary,
        #[tok(TEMP)]
        Temp,
    }
}

recursa::ast_node! {
    /// A single `def_arg` value — Postgres' grammar:
    ///
    /// ```text
    /// def_arg: func_type
    ///        | reserved_keyword  (e.g. TRUE, FALSE, ANALYZE, ...)
    ///        | qual_all_Op
    ///        | NumericOnly
    ///        | Sconst
    ///        | NONE
    /// ```
    ///
    /// pgtk-sql models the cases the corpus actually uses:
    /// - numeric (with optional sign) — `internallength = 24`, `default = -1`
    /// - string literal — `category = 'x'`, `LOCALE = "C"` (the double-quoted
    ///   form is parsed via the `Type` arm because `func_type`'s ident path
    ///   accepts quoted identifiers)
    /// - `NONE` — `OWNED BY NONE` (used at `def_arg` level by some grammars)
    /// - `DEFAULT` keyword as a value — `default_test_row` style
    /// - reserved keyword `TRUE` / `FALSE` — `DETERMINISTIC = TRUE`,
    ///   `preferred = true`
    /// - a type-name with optional precision and array suffix — `subtype = int4[]`,
    ///   `internallength = variable`, `alignment = double`, `PROVIDER = builtin`
    /// - an operator name — `commutator = ===`, `negator = !==` on
    ///   CREATE/ALTER OPERATOR. PG's `def_arg` accepts `qual_all_Op` directly
    ///   so the right-hand side of a `def_elem` can itself be a bare operator.
    ///   The corpus only exercises the bare-name form; the `OPERATOR(any_op)`
    ///   spelling is not modelled until a corpus statement needs it.
    ///
    /// Variant ordering: definite-keyword variants (`Default`, `True`, `False`,
    /// `None`, `Any`) come first; `Numeric` next (it can start with `+`/`-`/digit
    /// and matches no other variant); `QualOp` after `Numeric` because they
    /// share `+`/`-` as a leading token — `Numeric` consumes `+1`/`-2`, and
    /// `QualOp` only wins on a lone operator token; `Type` last (it's the
    /// broadest, accepting any identifier or built-in type keyword).
    #[derive(Debug)]
    pub enum DefArg {
        #[tok(DEFAULT)]
        Default,
        #[tok(TRUE)]
        True,
        #[tok(FALSE)]
        False,
        #[tok(NONE)]
        None,
        #[tok(ANY)]
        /// Reserved keyword used as a `def_arg` value. The corpus uses this
        /// for `ALTER SUBSCRIPTION ... SET (origin = any)`. Per gram.y
        /// `def_arg` accepts `reserved_keyword` directly; pgtk-sql only adds
        /// keywords actually exercised by the corpus.
        Any,
        String(CopySconst),
        Numeric(NumericOnly),
        /// A bare operator name on the RHS of a `def_elem`. Postgres' `def_arg`
        /// accepts `qual_all_Op`; the corpus exercises this on CREATE/ALTER
        /// OPERATOR's `commutator`/`negator` attributes.
        QualOp(crate::ast::shared::names::OperatorName),
        /// `SETOF type` — used in CREATE OPERATOR `leftarg`/`rightarg`
        /// (PG gram.y `def_arg: func_type`, and `func_type: Typename | type_function_name … | SETOF SimpleTypename …`).
        /// Listed before `Type` so the SETOF keyword reliably anchors this variant.
        Setof(DefArgSetof),
        /// A built-in type name with ordinary cast-type suffixes.
        BuiltinType(FunctionBuiltinType),
        /// An identifier-spelled type or `name(typename_list)` function-style
        /// value. The qualified name is parsed once before deciding whether a
        /// parenthesized argument is a typmod list or a type-name list.
        NamedType(DefArgNamedType),
    }
}

recursa::ast_node! {
    /// The type modifiers of an identifier-spelled def-arg type: gram.y
    /// `opt_type_modifiers: '(' expr_list ')'` (REL_17_11 14400). The
    /// support functions of the old `CREATE AGGREGATE` form are spelled this
    /// way, `sfunc = f(int8, int4)`: to gram.y, `int8` is a column reference
    /// in the `expr_list`. So `f(bit(1))` and `f(int ARRAY)` are syntax
    /// errors (issue #59).
    #[derive(Debug)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct DefArgNamedParameters {
        #[sep(COMMA)]
        pub values: one_or_many!(Expr),
    }
}

recursa::ast_node! {
    /// Identifier-spelled def-arg type or function-style value: gram.y
    /// `GenericType` with the array forms of `Typename`. The keyword-spelled
    /// types, with their `PRECISION`, `VARYING`, time zone and interval
    /// suffixes, are [`DefArg::BuiltinType`].
    #[derive(Debug)]
    pub struct DefArgNamedType {
        pub name: FunctionTypeName,
        pub parameters: Option<DefArgNamedParameters>,
        pub array: Option<TypeArray>,
    }
}

recursa::ast_node! {
    /// `SETOF type` value in a `def_list` (PG: `func_type` form on `def_arg`).
    #[derive(Debug)]
    pub struct DefArgSetof {
        #[tok(SETOF, this)]
        pub type_name: CastType,
    }
}

recursa::ast_node! {
    /// `value` separator on `def_elem` — Postgres uses `'='`.
    ///
    /// One-variant enum so the AST has a typed node where the literal sits.
    #[derive(Debug)]
    pub struct DefElemValue {
        #[tok(EQ, this)]
        pub arg: DefArg,
    }
}

recursa::ast_node! {
    /// One entry in a `def_list` — `name [= value]`.
    ///
    /// The name is `AliasName` so any keyword or identifier is accepted (Postgres
    /// `ColLabel` permits every keyword class). The value is optional: some
    /// CREATE TYPE base-type forms use bare names like `passedbyvalue`.
    #[derive(Debug)]
    pub struct DefElem {
        pub name: literal::AliasName,
        pub value: Option<DefElemValue>,
    }
}

recursa::ast_node! {
    /// A parenthesised `def_list`: `(name [= value], ...)`.
    #[derive(Debug)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct DefList {
        #[sep(COMMA)]
        pub items: one_or_many!(DefElem),
    }
}

recursa::ast_node! {
    /// Password value in `PASSWORD { sconst | NULL }` — Postgres'
    /// `AlterOptRoleElem` PASSWORD branch.
    #[derive(Debug)]
    pub enum PasswordValue {
        #[tok(NULL)]
        Null,
        /// An `Sconst`: plain, `E'…'` or `U&'…'`.
        String(CopySconst),
    }
}

recursa::ast_node! {
    /// `[ENCRYPTED | UNENCRYPTED] PASSWORD value` — the PASSWORD branches of
    /// Postgres' `AlterOptRoleElem` (REL_17_11 gram.y 1209-1230):
    ///
    /// ```text
    /// PASSWORD Sconst | PASSWORD NULL_P
    /// | ENCRYPTED PASSWORD Sconst | UNENCRYPTED PASSWORD Sconst
    /// ```
    ///
    /// Only the plain form takes `NULL`, so `ENCRYPTED PASSWORD NULL` is a
    /// syntax error (issue #53). `ENCRYPTED` is a backward-compat noise word
    /// (passwords are always encrypted today). The action of the
    /// `UNENCRYPTED PASSWORD` form raises an error; pgtk-sql accepts the form
    /// and round-trips it byte-faithfully.
    ///
    /// Variant ordering: the three forms part on their first keyword.
    #[derive(Debug)]
    pub enum PasswordOption {
        /// `PASSWORD { Sconst | NULL }`.
        Plain(#[tok(PASSWORD, this)] PasswordValue),
        /// `ENCRYPTED PASSWORD Sconst`.
        Encrypted(#[tok(ENCRYPTED, PASSWORD, this)] CopySconst),
        /// `UNENCRYPTED PASSWORD Sconst`.
        Unencrypted(#[tok(UNENCRYPTED, PASSWORD, this)] CopySconst),
    }
}

recursa::ast_node! {
    /// `CONNECTION LIMIT signed_iconst` — role-attribute connection limit.
    #[derive(Debug)]
    pub struct ConnectionLimitOption {
        #[tok(CONNECTION, LIMIT, this)]
        pub value: SignedIconst,
    }
}

recursa::ast_node! {
    /// `VALID UNTIL sconst` — role-attribute expiry.
    #[derive(Debug)]
    pub struct ValidUntilOption {
        #[tok(VALID, UNTIL, this)]
        pub value: CopySconst,
    }
}

recursa::ast_node! {
    /// `IN { ROLE | GROUP } role_list` — role membership target list.
    ///
    /// Variant ordering: keyword kind discriminates first; both are single-token.
    #[derive(Debug)]
    pub enum InRoleOrGroup {
        #[tok(ROLE)]
        Role,
        #[tok(GROUP)]
        Group,
    }
}

recursa::ast_node! {
    /// `IN { ROLE | GROUP } role [, ...]` — Postgres' `CreateOptRoleElem`
    /// `IN_P ROLE`/`IN_P GROUP_P` branch.
    #[derive(Debug)]
    pub struct InRoleOption {
        #[tok(IN, this)]
        pub kind: InRoleOrGroup,
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    /// `SYSID iconst` — legacy noise option preserved for backward compat.
    #[derive(Debug)]
    pub struct SysIdOption {
        #[tok(SYSID, this)]
        pub value: literal::IntegerLit,
    }
}

recursa::ast_node! {
    /// `ADMIN role_list` — `CreateOptRoleElem` ADMIN branch (creates role with
    /// admin members).
    #[derive(Debug)]
    pub struct AdminOption {
        #[tok(ADMIN, this)]
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    /// `ROLE role_list` — `CreateOptRoleElem` ROLE branch (creates role with
    /// child members).
    #[derive(Debug)]
    pub struct RoleMembersOption {
        #[tok(ROLE, this)]
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    /// `USER role_list` — legacy `CREATE GROUP name [WITH] USER u1, u2`
    /// (supported but undocumented for ALTER GROUP); also matches
    /// `AlterOptRoleElem`'s undocumented `USER role_list` branch.
    #[derive(Debug)]
    pub struct UserMembersOption {
        #[tok(USER, this)]
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    /// A single CREATE ROLE / CREATE USER / CREATE GROUP option — Postgres'
    /// `CreateOptRoleElem`. Options are unordered and repeatable.
    ///
    /// Variant ordering: multi-keyword forms (`IN ROLE`, `CONNECTION LIMIT`,
    /// `VALID UNTIL`, `ENCRYPTED PASSWORD`, `UNENCRYPTED PASSWORD`) before any
    /// single-keyword form they share a first-token with, so longest-match-wins
    /// disambiguates. `ROLE role_list` and `IN ROLE …` both start with `ROLE`/
    /// `IN` so the longer `IN ROLE` form wins on its leading `IN`.
    #[derive(Debug)]
    pub enum CreateRoleOption {
        // Multi-keyword forms first.
        InRole(InRoleOption),
        ConnectionLimit(ConnectionLimitOption),
        ValidUntil(ValidUntilOption),
        Password(PasswordOption),
        SysId(SysIdOption),
        Admin(AdminOption),
        Role(RoleMembersOption),
        User(UserMembersOption),
        // Single-keyword forms — soft keywords for the role-attribute names.
        #[tok(SUPERUSER)]
        Superuser,
        #[tok(NOSUPERUSER)]
        NoSuperuser,
        #[tok(CREATEDB)]
        CreateDb,
        #[tok(NOCREATEDB)]
        NoCreateDb,
        #[tok(CREATEROLE)]
        CreateRole,
        #[tok(NOCREATEROLE)]
        NoCreateRole,
        #[tok(INHERIT)]
        Inherit,
        #[tok(NOINHERIT)]
        NoInherit,
        #[tok(LOGIN)]
        Login,
        #[tok(NOLOGIN)]
        NoLogin,
        #[tok(REPLICATION)]
        Replication,
        #[tok(NOREPLICATION)]
        NoReplication,
        #[tok(BYPASSRLS)]
        BypassRls,
        #[tok(NOBYPASSRLS)]
        NoBypassRls,
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub struct CreateGroupStmt {
        #[tok(CREATE, GROUP, this)]
        pub name: RoleId,
        // `this_each` states the element-wise attachment this site already
        // lowers to: the lead-in wraps every option, so `WITH LOGIN WITH
        // SUPERUSER` is accepted where gram.y `CREATE GROUP RoleId opt_with
        // OptRoleList` reads `WITH` once. RCA3152 asks only that the reading
        // be explicit; the container repair is #59 section B, which is open.
        #[tok(optional(WITH), this_each)]
        pub options: zero_or_many!(CreateRoleOption),
    }
}

recursa::ast_node! {
    /// `DROP GROUP [IF EXISTS] role [, ...]` — no `CASCADE`/`RESTRICT`.
    #[derive(Debug)]
    #[tok(DROP, GROUP, this)]
    pub struct DropGroupStmt {
        pub if_exists: Option<IfExists>,
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub struct CreateRoleStmt {
        #[tok(CREATE, ROLE, this)]
        pub name: RoleId,
        // `this_each` states the element-wise attachment this site already
        // lowers to: the lead-in wraps every option, so `WITH LOGIN WITH
        // SUPERUSER` is accepted where gram.y `CREATE ROLE RoleId opt_with
        // OptRoleList` reads `WITH` once. RCA3152 asks only that the reading
        // be explicit; the container repair is #59 section B, which is open.
        #[tok(optional(WITH), this_each)]
        pub options: zero_or_many!(CreateRoleOption),
    }
}

recursa::ast_node! {
    /// `DROP ROLE [IF EXISTS] role [, ...]` — no `CASCADE`/`RESTRICT`.
    #[derive(Debug)]
    #[tok(DROP, ROLE, this)]
    pub struct DropRoleStmt {
        pub if_exists: Option<IfExists>,
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub struct CreateUserStmt {
        #[tok(CREATE, USER, this)]
        pub name: RoleId,
        // `this_each` states the element-wise attachment this site already
        // lowers to: the lead-in wraps every option, so `WITH LOGIN WITH
        // SUPERUSER` is accepted where gram.y `CREATE USER RoleId opt_with
        // OptRoleList` reads `WITH` once. RCA3152 asks only that the reading
        // be explicit; the container repair is #59 section B, which is open.
        #[tok(optional(WITH), this_each)]
        pub options: zero_or_many!(CreateRoleOption),
    }
}

recursa::ast_node! {
    /// `DROP USER [IF EXISTS] role [, ...]` — no `CASCADE`/`RESTRICT`.
    #[derive(Debug)]
    #[tok(DROP, USER, this)]
    pub struct DropUserStmt {
        pub if_exists: Option<IfExists>,
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    /// A single option in an `AlterOptRoleList` — Postgres' `AlterOptRoleElem`.
    ///
    /// This is a strict subset of [`CreateRoleOption`] / `CreateOptRoleElem`:
    /// the create-only options (`SYSID iconst`, `ADMIN role_list`, `ROLE
    /// role_list`, `IN ROLE role_list`, `IN GROUP role_list`) are excluded by
    /// gram.y. `USER role_list` is allowed (officially undocumented; supported
    /// "for use by ALTER GROUP", per the gram.y comment).
    ///
    /// Variant ordering: multi-keyword forms (`CONNECTION LIMIT`, `VALID
    /// UNTIL`, `[ENCRYPTED|UNENCRYPTED] PASSWORD`) before any single-keyword
    /// form they share a first-token with — though here they have disjoint
    /// first tokens, order is for clarity. The bare attribute keywords
    /// (`SUPERUSER` etc.) are all distinct soft keywords.
    #[derive(Debug)]
    pub enum AlterRoleOption {
        // Multi-keyword forms first.
        ConnectionLimit(ConnectionLimitOption),
        ValidUntil(ValidUntilOption),
        Password(PasswordOption),
        User(UserMembersOption),
        // Single-keyword forms — soft keywords for the role-attribute names.
        #[tok(SUPERUSER)]
        Superuser,
        #[tok(NOSUPERUSER)]
        NoSuperuser,
        #[tok(CREATEDB)]
        CreateDb,
        #[tok(NOCREATEDB)]
        NoCreateDb,
        #[tok(CREATEROLE)]
        CreateRole,
        #[tok(NOCREATEROLE)]
        NoCreateRole,
        #[tok(INHERIT)]
        Inherit,
        #[tok(NOINHERIT)]
        NoInherit,
        #[tok(LOGIN)]
        Login,
        #[tok(NOLOGIN)]
        NoLogin,
        #[tok(REPLICATION)]
        Replication,
        #[tok(NOREPLICATION)]
        NoReplication,
        #[tok(BYPASSRLS)]
        BypassRls,
        #[tok(NOBYPASSRLS)]
        NoBypassRls,
    }
}

recursa::ast_node! {
    /// `ALL [IN DATABASE name] SetResetClause` — gram.y `AlterRoleSetStmt:
    /// ALTER ROLE ALL opt_in_database SetResetClause` (REL_17_11 gram.y 1386).
    /// `ALL` takes only a SET or a RESET (issue #53).
    #[derive(Debug)]
    pub struct AlterRoleAll {
        #[tok(ALL, this)]
        pub set_reset: AlterRoleSetReset,
    }
}

recursa::ast_node! {
    /// `IN DATABASE name` — Postgres' `opt_in_database` clause on
    /// `AlterRoleSetStmt`. Scopes a SET/RESET to a particular database.
    #[derive(Debug)]
    pub struct InDatabaseClause {
        #[tok(IN, DATABASE, this)]
        pub name: crate::tokens::ColId,
    }
}

recursa::ast_node! {
    /// `SET set_rest | VariableResetStmt` — Postgres' `SetResetClause`.
    ///
    /// The `SET` half is the whole of `set_rest`, so every special form of a
    /// top-level unscoped `SET` is legal here: `ALTER ROLE r SET TIME ZONE
    /// 'UTC'` and `ALTER DATABASE d SET TRANSACTION ISOLATION LEVEL
    /// SERIALIZABLE` are accepted. `LOCAL` and `SESSION` are not, because
    /// `SetResetClause` names the literal `SET` itself.
    ///
    /// Reuses [`crate::ast::session::set_reset::SetUnscoped`] (`SET set_rest`)
    /// and [`crate::ast::session::set_reset::ResetStmt`]
    /// (`VariableResetStmt`); both start with their own keyword, so the two
    /// variants are keyword-disjoint.
    #[derive(Debug)]
    pub enum SetResetClause {
        Set(crate::ast::session::set_reset::SetUnscoped),
        Reset(crate::ast::session::set_reset::ResetStmt),
    }
}

recursa::ast_node! {
    /// `[IN DATABASE name] SetResetClause` — the body of `AlterRoleSetStmt`.
    #[derive(Debug)]
    pub struct AlterRoleSetReset {
        pub in_database: Option<InDatabaseClause>,
        pub clause: SetResetClause,
    }
}

recursa::ast_node! {
    /// `WITH AlterOptRoleList` — Postgres' `opt_with AlterOptRoleList` form
    /// when the explicit `WITH` keyword is present. The option list itself
    /// may be empty (gram.y's `AlterOptRoleList` is right-recursive with an
    /// `/* EMPTY */` base case).
    #[derive(Debug)]
    #[tok(WITH, this)]
    pub struct AlterRoleWithOptions {
        pub options: zero_or_many!(AlterRoleOption),
    }
}

recursa::ast_node! {
    /// One non-empty `AlterOptRoleList` (no leading `WITH`) — at least one
    /// option. The peek of [`AlterRoleOption`] gates this variant so the
    /// empty-list case never matches (it falls through to no action at all,
    /// which gram.y also allows but the corpus never uses).
    #[derive(Debug)]
    pub struct AlterRoleOptionsOnly {
        pub options: one_or_many!(AlterRoleOption),
    }
}

recursa::ast_node! {
    /// One action on `ALTER ROLE role_spec`/`ALTER USER role_spec` — covers
    /// Postgres' `AlterRoleStmt` (`[WITH] AlterOptRoleList`) and
    /// `AlterRoleSetStmt` (`[IN DATABASE name] SetResetClause`). The `RENAME
    /// TO` branch of `RenameStmt` names a `RoleId`, so it is
    /// [`AlterRoleRename`].
    ///
    /// Variant ordering: keyword-distinct branches first (`SetReset` on
    /// `IN`/`SET`/`RESET`, `With` on `WITH`). The
    /// catch-all `Options` is last; its peek is the union of all
    /// [`AlterRoleOption`] first tokens (the soft attribute keywords and
    /// `PASSWORD`/`CONNECTION`/`VALID`/`ENCRYPTED`/`UNENCRYPTED`/`USER`),
    /// none of which collide with `IN`/`SET`/`RESET`/`WITH`.
    #[derive(Debug)]
    pub enum AlterRoleAction {
        SetReset(AlterRoleSetReset),
        With(AlterRoleWithOptions),
        Options(AlterRoleOptionsOnly),
    }
}

recursa::ast_node! {
    /// `ALTER GROUP role_spec { ADD | DROP } USER role_list` — Postgres'
    /// `AlterGroupStmt` add/drop form.
    ///
    /// Variant ordering: `ADD` and `DROP` are keyword-disjoint.
    #[derive(Debug)]
    pub enum AddDrop {
        #[tok(ADD)]
        Add,
        #[tok(DROP)]
        Drop,
    }
}

recursa::ast_node! {
    /// `{ ADD | DROP } USER role_list` — body of Postgres' `AlterGroupStmt`.
    #[derive(Debug)]
    pub struct AlterGroupUsers {
        pub add_drop: AddDrop,
        #[tok(USER, this)]
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    /// `RoleId RENAME TO RoleId` — the `RenameStmt` branches `ALTER ROLE
    /// RoleId RENAME TO RoleId`, `ALTER USER RoleId RENAME TO RoleId` and
    /// `ALTER GROUP_P RoleId RENAME TO RoleId` (REL_17_11 gram.y 9761, 9771
    /// and 9358). Both names are `RoleId`, so `ALTER ROLE CURRENT_ROLE RENAME
    /// TO r` is rejected (issue #53).
    #[derive(Debug)]
    pub struct AlterRoleRename {
        pub name: RoleId,
        #[tok(RENAME, TO, this)]
        pub new_name: RoleId,
    }
}

recursa::ast_node! {
    /// `role_spec { ADD | DROP } USER role_list` — gram.y `AlterGroupStmt`.
    #[derive(Debug)]
    pub struct AlterGroupMembers {
        pub name: RoleSpec,
        pub users: AlterGroupUsers,
    }
}

recursa::ast_node! {
    /// The body of `ALTER GROUP` — Postgres' `AlterGroupStmt` (`role_spec
    /// add_drop USER role_list`) and the `RENAME TO` branch of `RenameStmt`.
    ///
    /// Variant ordering: both start with a role word and part on the
    /// `RENAME`, `ADD` or `DROP` after it.
    #[derive(Debug)]
    pub enum AlterGroupBody {
        Rename(AlterRoleRename),
        Members(AlterGroupMembers),
    }
}

recursa::ast_node! {
    /// `ALTER GROUP …` — Postgres' `AlterGroupStmt` (`add_drop USER
    /// role_list`) plus the `RENAME TO` branch from `RenameStmt`.
    #[derive(Debug)]
    pub struct AlterGroupStmt {
        #[tok(ALTER, GROUP, this)]
        pub body: AlterGroupBody,
    }
}

recursa::ast_node! {
    /// `role_spec action` — `ALTER ROLE` or `ALTER USER` with a role and an
    /// option list or a SET or RESET.
    #[derive(Debug)]
    pub struct AlterRoleSpecAction {
        pub role: RoleSpec,
        pub action: AlterRoleAction,
    }
}

recursa::ast_node! {
    /// The body of `ALTER ROLE` and `ALTER USER`: gram.y `AlterRoleStmt`,
    /// `AlterRoleSetStmt` and the `RENAME TO` branch of `RenameStmt`.
    ///
    /// Variant ordering: `All` leads with its keyword. `Rename` and `Role`
    /// start with a role word and part on the `RENAME` after it.
    #[derive(Debug)]
    pub enum AlterRoleBody {
        All(AlterRoleAll),
        Rename(AlterRoleRename),
        Role(AlterRoleSpecAction),
    }
}

recursa::ast_node! {
    /// `ALTER ROLE …` — Postgres' `AlterRoleStmt`, `AlterRoleSetStmt`, and
    /// the `RENAME TO` branch from `RenameStmt`.
    #[derive(Debug)]
    pub struct AlterRoleStmt {
        #[tok(ALTER, ROLE, this)]
        pub body: AlterRoleBody,
    }
}

recursa::ast_node! {
    /// Postgres' `auth_ident` — the authorization identifier on
    /// `CREATE/ALTER/DROP USER MAPPING FOR ...`.
    ///
    /// gram.y: `auth_ident: RoleSpec | USER` — either a role spec (plain
    /// identifier; the reserved-word pseudo-roles `current_user`,
    /// `current_role`, `session_user`, `public` are not modelled as keywords
    /// in pgtk-sql's lexer and thus arrive here as plain `Ident`s) or the
    /// literal `USER` keyword meaning the current user.
    ///
    /// Variant ordering: `User` is the `USER` hard keyword, `Role` is a
    /// plain `Ident` — keyword-disjoint, so order is for clarity.
    #[derive(Debug)]
    pub enum UserMappingFor {
        #[tok(USER)]
        User,
        Role(RoleSpec),
    }
}

recursa::ast_node! {
    /// `CREATE USER MAPPING [IF NOT EXISTS] FOR auth_ident SERVER name
    /// [OPTIONS (...)]` — Postgres' `CreateUserMappingStmt`.
    #[derive(Debug)]
    #[tok(CREATE, USER, MAPPING, this)]
    pub struct CreateUserMappingStmt {
        pub if_not_exists: Option<IfNotExists>,
        #[tok(FOR, this)]
        pub auth_ident: UserMappingFor,
        #[tok(SERVER, this)]
        pub server_name: crate::tokens::ColId,
        pub options: Option<CreateGenericOptions>,
    }
}

recursa::ast_node! {
    /// `ALTER USER MAPPING FOR auth_ident SERVER name OPTIONS (...)` —
    /// Postgres' `AlterUserMappingStmt`. The `OPTIONS` clause is mandatory
    /// in gram.y.
    #[derive(Debug)]
    pub struct AlterUserMappingStmt {
        #[tok(ALTER, USER, MAPPING, FOR, this)]
        pub auth_ident: UserMappingFor,
        #[tok(SERVER, this)]
        pub server_name: crate::tokens::ColId,
        pub options: AlterGenericOptions,
    }
}

recursa::ast_node! {
    /// `DROP USER MAPPING [IF EXISTS] FOR auth_ident SERVER name` —
    /// Postgres' `DropUserMappingStmt`. No CASCADE/RESTRICT (gram.y
    /// comment: "XXX you'd think this should have a CASCADE/RESTRICT
    /// option, even if it's only pro forma; but the SQL standard doesn't
    /// show one.").
    #[derive(Debug)]
    #[tok(DROP, USER, MAPPING, this)]
    pub struct DropUserMappingStmt {
        pub if_exists: Option<IfExists>,
        #[tok(FOR, this)]
        pub auth_ident: UserMappingFor,
        #[tok(SERVER, this)]
        pub server_name: crate::tokens::ColId,
    }
}

recursa::ast_node! {
    /// `ALTER USER …` — Postgres' `AlterRoleStmt`
    /// / `AlterRoleSetStmt` (USER alias for ROLE) plus the `RENAME TO`
    /// branch from `RenameStmt`.
    ///
    /// The `ALTER USER MAPPING ...` form lives in its own top-level
    /// [`AlterUserMappingStmt`] / `Statement::AlterUserMapping`; the
    /// `Statement` enum dispatches on the three-keyword `ALTER USER
    /// MAPPING` lead before the bare `ALTER USER` variant.
    #[derive(Debug)]
    pub struct AlterUserStmt {
        #[tok(ALTER, USER, this)]
        pub body: AlterRoleBody,
    }
}
