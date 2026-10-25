//! POLICY DDL statements (CREATE/ALTER/DROP).
#![allow(unused_imports)]

use crate::ast::shared::expr::*;
use crate::ast::shared::flags::*;
use crate::ast::shared::names::*;
use crate::ast::shared::numbers::*;
use crate::tokens::{literal, punct};

recursa::ast_node! {
    /// `row_security_cmd`: the command kind in a `FOR` clause on CREATE/ALTER
    /// POLICY — Postgres' `row_security_cmd` rule. All five forms are bare
    /// keywords. `ALL` and `SELECT` are reserved; `INSERT`/`UPDATE`/`DELETE`
    /// are soft.
    ///
    /// Variant ordering: all distinct first tokens, so order is for clarity.
    #[derive(Debug)]
    pub enum RowSecurityCmd {
        #[tok(ALL)]
        All,
        #[tok(SELECT)]
        Select,
        #[tok(INSERT)]
        Insert,
        #[tok(UPDATE)]
        Update,
        #[tok(DELETE)]
        Delete,
    }
}

recursa::ast_node! {
    /// `AS ident` permissive/restrictive selector on CREATE POLICY —
    /// Postgres' `RowSecurityDefaultPermissive`.
    ///
    /// gram.y reads the word as `IDENT` and its action compares the name
    /// with `"permissive"` and `"restrictive"` (`strcmp`). The action rejects
    /// each other name, so the corpus form `AS UGLY` is a raw-parser error
    /// (REL_17_11 gram.y 5849; REL_14_24 gram.y 5298).
    #[classify(kind, fold = crate::ident::fold_name)]
    #[reject(
        not_in(kind, "permissive", "restrictive"),
        code = "42601",
        message = "unrecognized row security option",
        hint = "Only PERMISSIVE or RESTRICTIVE policies are supported currently.",
        cite = "REL_17_11 gram.y 5849, RowSecurityDefaultPermissive",
    )]
    #[derive(Debug)]
    pub struct PolicyPermissiveClause {
        #[tok(AS, this)]
        pub kind: crate::tokens::literal::IdentOnly,
    }
}

recursa::ast_node! {
    /// `FOR row_security_cmd` clause on CREATE/ALTER POLICY.
    #[derive(Debug)]
    pub struct PolicyForClause {
        #[tok(FOR, this)]
        pub cmd: RowSecurityCmd,
    }
}

recursa::ast_node! {
    /// `TO role_list` clause on CREATE/ALTER POLICY — Postgres'
    /// `RowSecurityDefaultToRole`. `PUBLIC` is no keyword and arrives as a
    /// `RoleSpec` name; `CURRENT_USER` and its two siblings are reserved
    /// words with their own `RoleSpec` variants.
    #[derive(Debug)]
    pub struct PolicyToClause {
        #[tok(TO, this)]
        pub roles: RoleList,
    }
}

recursa::ast_node! {
    /// `USING (a_expr)` clause on CREATE/ALTER POLICY.
    #[derive(Debug)]
    pub struct PolicyUsingClause {
        #[tok(USING, LPAREN, this, RPAREN)]
        pub expr: boxed!(Expr),
    }
}

recursa::ast_node! {
    /// `WITH CHECK (a_expr)` clause on CREATE/ALTER POLICY.
    #[derive(Debug)]
    pub struct PolicyWithCheckClause {
        #[tok(WITH, CHECK, LPAREN, this, RPAREN)]
        pub expr: boxed!(Expr),
    }
}

recursa::ast_node! {
    /// `CREATE POLICY name ON table [AS PERMISSIVE|RESTRICTIVE]
    /// [FOR cmd] [TO role_list] [USING (expr)] [WITH CHECK (expr)]` —
    /// Postgres' `CreatePolicyStmt`.
    #[derive(Debug)]
    pub struct CreatePolicyStmt {
        #[tok(CREATE, POLICY, this)]
        pub name: crate::tokens::ColId,
        #[tok(ON, this)]
        pub table: QualifiedName,
        pub permissive: Option<PolicyPermissiveClause>,
        pub for_cmd: Option<PolicyForClause>,
        pub to_roles: Option<PolicyToClause>,
        pub using: Option<PolicyUsingClause>,
        pub with_check: Option<PolicyWithCheckClause>,
    }
}

recursa::ast_node! {
    /// `DROP POLICY [IF EXISTS] name ON table [CASCADE | RESTRICT]`.
    #[derive(Debug)]
    #[tok(DROP, POLICY, this)]
    pub struct DropPolicyStmt {
        pub if_exists: Option<IfExists>,
        pub name: crate::tokens::ColId,
        #[tok(ON, this)]
        pub table: crate::ast::shared::names::AnyName,
        pub behavior: Option<DropBehavior>,
    }
}

recursa::ast_node! {
    /// The modification action on `ALTER POLICY` — either `RENAME TO new`
    /// (Postgres' `RenameStmt` branch) or the standard
    /// `[TO role_list] [USING (expr)] [WITH CHECK (expr)]` action
    /// (Postgres' `AlterPolicyStmt`). Both share the `ALTER POLICY name ON
    /// qualified_name` prefix; the action discriminates between them.
    ///
    /// Variant ordering: `Rename` (single-keyword `RENAME`) is listed before
    /// `Modify` (which can start with `TO`, `USING`, `WITH`, or be empty);
    /// the two have disjoint first-token sets.
    #[derive(Debug)]
    pub enum AlterPolicyAction {
        Rename(RenameTo),
        Modify(AlterPolicyModify),
    }
}

recursa::ast_node! {
    /// `[TO role_list] [USING (expr)] [WITH CHECK (expr)]` — the non-rename
    /// action on `ALTER POLICY`. All three clauses are optional but at least
    /// one must be present at the semantic level; pgtk-sql accepts the
    /// all-empty form too because gram.y's `AlterPolicyStmt` does.
    #[derive(Debug)]
    pub struct AlterPolicyModify {
        pub to_roles: Option<PolicyToClause>,
        pub using: Option<PolicyUsingClause>,
        pub with_check: Option<PolicyWithCheckClause>,
    }
}

recursa::ast_node! {
    /// `ALTER POLICY name ON qualified_name action` — Postgres'
    /// `AlterPolicyStmt` plus the `ALTER POLICY ... RENAME TO ...` branch
    /// from `RenameStmt`. Both share the same prefix; the action enum
    /// dispatches.
    #[derive(Debug)]
    pub struct AlterPolicyStmt {
        #[tok(ALTER, POLICY, this)]
        pub name: crate::tokens::ColId,
        #[tok(ON, this)]
        pub table: QualifiedName,
        pub action: AlterPolicyAction,
    }
}
