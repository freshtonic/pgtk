//! SCHEMA DDL statements (CREATE/ALTER/DROP).
#![allow(unused_imports)]

use crate::ast::ddl::sequence::CreateSequenceStmt;
use crate::ast::ddl::trigger::CreateTriggerStmt;
use crate::ast::shared::expr::*;
use crate::ast::shared::flags::*;
use crate::ast::shared::names::*;
use crate::ast::shared::numbers::*;
use crate::ast::utility::grant::GrantPrivilegeStmt;
use crate::tokens::{literal, punct};

recursa::ast_node! {
    /// `AUTHORIZATION role_spec` clause on `CREATE SCHEMA`.
    #[derive(Debug)]
    pub struct SchemaAuthorization {
        #[tok(AUTHORIZATION, this)]
        pub role: RoleSpec,
    }
}

recursa::ast_node! {
    /// `name [AUTHORIZATION role]` — schema name with optional authorization.
    #[derive(Debug)]
    pub struct SchemaNameAndAuth {
        pub name: crate::tokens::ColId,
        pub authorization: Option<SchemaAuthorization>,
    }
}

recursa::ast_node! {
    /// The name/authorization clause of `CREATE SCHEMA` — Postgres allows either
    /// a schema name (with optional `AUTHORIZATION`) or just `AUTHORIZATION role`
    /// (which implicitly names the schema after the role).
    ///
    /// Variant ordering: the `Authorization` form must come first so a leading
    /// `AUTHORIZATION` keyword does not get consumed as the schema-name `Ident`
    /// (soft keywords are reclaimable as identifiers, and `AUTHORIZATION` is
    /// soft).
    #[derive(Debug)]
    pub enum SchemaNameClause {
        Authorization(SchemaAuthorization),
        Named(SchemaNameAndAuth),
    }
}

recursa::ast_node! {
    /// A single statement nested inside `CREATE SCHEMA … schema_element*`.
    ///
    /// Postgres' `OptSchemaEltList` accepts a small subset of statements:
    /// `CreateStmt | IndexStmt | CreateSeqStmt | CreateTrigStmt | GrantStmt |
    /// ViewStmt`. We type each one with the relevant statement struct rather
    /// than a general `Statement` to mirror the grammar precisely.
    ///
    /// Variant ordering: multi-keyword `GRANT` distinct from the `CREATE …`
    /// family; among the CREATE-led variants, `CreateView` (matches `CREATE [OR
    /// REPLACE] [TEMP|TEMPORARY] [RECURSIVE] VIEW …`) is the most specific.
    /// The other CREATE variants disambiguate on their `CREATE { TABLE | INDEX
    /// | SEQUENCE | TRIGGER }` second-token.
    #[derive(Debug)]
    pub enum SchemaElement {
        Grant(GrantPrivilegeStmt),
        CreateView(boxed!(crate::ast::ddl::view::CreateViewStmt)),
        CreateTable(boxed!(crate::ast::ddl::table::CreatePlainTableStmt)),
        CreateIndex(boxed!(crate::ast::ddl::index::CreateIndexStmt)),
        CreateSequence(CreateSequenceStmt),
        CreateTrigger(boxed!(CreateTriggerStmt)),
    }
}

recursa::ast_node! {
    /// gram.y `CreateSchemaStmt` (REL_17_11 gram.y 1527-1585).
    #[derive(Debug)]
    #[tok(CREATE, SCHEMA, this)]
    pub struct CreateSchemaStmt {
        pub form: CreateSchemaForm,
    }
}

recursa::ast_node! {
    /// The two forms of `CREATE SCHEMA`. The actions of the `IF NOT EXISTS`
    /// productions reject a schema element list that is not empty (REL_17_11
    /// gram.y 1559 and 1575; REL_14_24 gram.y 1418 and 1433), so only the
    /// plain form has elements.
    ///
    /// Variant ordering: `IfNotExists` leads with `IF`, which is also a
    /// `ColId`; the token after `IF` parts the two forms, as in gram.y.
    #[derive(Debug)]
    pub enum CreateSchemaForm {
        IfNotExists(CreateSchemaIfNotExists),
        Plain(CreateSchemaPlain),
    }
}

recursa::ast_node! {
    /// `IF NOT EXISTS head`, with no schema element.
    #[derive(Debug)]
    pub struct CreateSchemaIfNotExists {
        #[tok(IF, NOT, EXISTS, this)]
        pub head: SchemaNameClause,
    }
}

recursa::ast_node! {
    /// `head schema_element*`.
    #[derive(Debug)]
    pub struct CreateSchemaPlain {
        /// Schema name and/or `AUTHORIZATION` clause. Postgres requires at least
        /// one — the enum forces that structurally.
        pub head: SchemaNameClause,
        /// Nested `schema_element` statements — `OptSchemaEltList`. Each element
        /// is a top-level statement type; the surrounding semicolons live on the
        /// enclosing statement, not on the nested ones.
        pub elements: zero_or_many!(SchemaElement),
    }
}

impl<'input> CreateSchemaStmt<'input> {
    /// Whether the statement has `IF NOT EXISTS`.
    pub fn if_not_exists(&self) -> bool {
        matches!(self.form, CreateSchemaForm::IfNotExists(_))
    }

    /// The schema name and/or `AUTHORIZATION` clause.
    pub fn head(&self) -> &SchemaNameClause<'input> {
        match &self.form {
            CreateSchemaForm::IfNotExists(form) => &form.head,
            CreateSchemaForm::Plain(form) => &form.head,
        }
    }

    /// The nested schema elements. The `IF NOT EXISTS` form has none.
    pub fn elements(&self) -> &[SchemaElement<'input>] {
        match &self.form {
            CreateSchemaForm::IfNotExists(_) => &[],
            CreateSchemaForm::Plain(form) => &form.elements,
        }
    }
}

recursa::ast_node! {
    /// `DROP SCHEMA [IF EXISTS] name [, ...] [CASCADE | RESTRICT]`.
    #[derive(Debug)]
    #[tok(DROP, SCHEMA, this)]
    pub struct DropSchemaStmt {
        pub if_exists: Option<IfExists>,
        pub names: crate::ast::shared::names::ColIdList,
        pub behavior: Option<DropBehavior>,
    }
}

recursa::ast_node! {
    /// One action on `ALTER SCHEMA name action` — covers Postgres'
    /// `RenameStmt` and `AlterOwnerStmt` branches for schemas. Schemas
    /// have no SET SCHEMA action.
    ///
    /// Variant ordering: variants begin with distinct keywords (`RENAME`,
    /// `OWNER`), so order is for clarity only.
    #[derive(Debug)]
    pub enum AlterSchemaAction {
        Rename(RenameTo),
        Owner(OwnerTo),
    }
}

recursa::ast_node! {
    /// `ALTER SCHEMA name action` — Postgres' `RenameStmt` /
    /// `AlterOwnerStmt` branches for schemas.
    #[derive(Debug)]
    pub struct AlterSchemaStmt {
        #[tok(ALTER, SCHEMA, this)]
        pub name: crate::tokens::ColId,
        pub action: AlterSchemaAction,
    }
}
