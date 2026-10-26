//! TRANSFORM DDL statements (CREATE/ALTER/DROP).
#![allow(unused_imports)]

use crate::ast::shared::expr::*;
use crate::ast::shared::flags::*;
use crate::ast::shared::names::*;
use crate::ast::shared::numbers::*;
use crate::tokens::{literal, punct};

recursa::ast_node! {
    /// gram.y `transform_element_list` (REL_17_11 9149-9164): one `FROM SQL
    /// WITH FUNCTION fn` and one `TO SQL WITH FUNCTION fn`, in either order,
    /// or one of them. So a second `TO SQL` element is a syntax error (issue
    /// #56). The rule is the same in each target version.
    #[derive(Debug)]
    pub enum TransformElements {
        FromTo(TransformFromElement, #[tok(COMMA, this)] TransformToElement),
        ToFrom(TransformToElement, #[tok(COMMA, this)] TransformFromElement),
        From(TransformFromElement),
        To(TransformToElement),
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub struct TransformFromElement {
        #[tok(FROM, SQL, WITH, FUNCTION, this)]
        pub func: crate::ast::ddl::function::DropFunctionTarget,
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub struct TransformToElement {
        #[tok(TO, SQL, WITH, FUNCTION, this)]
        pub func: crate::ast::ddl::function::DropFunctionTarget,
    }
}

recursa::ast_node! {
    /// Parenthesized `CREATE TRANSFORM` element list.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct TransformElementList(#[deref] pub TransformElements);
}

recursa::ast_node! {
    /// `CREATE [OR REPLACE] TRANSFORM FOR Typename LANGUAGE name (elements)`
    /// (PG `CreateTransformStmt` in gram.y). The element list is one or two
    /// `{FROM|TO} SQL WITH FUNCTION ...` entries; pgtk-sql models the list as
    /// `Seq1` of `TransformElement` separated by `Comma`, and relies on PG to reject duplicates and
    /// empty lists at semantic-analysis time.
    #[derive(Debug)]
    pub struct CreateTransformStmt {
        #[tok(CREATE, this, TRANSFORM, FOR)]
        #[presence(OR, REPLACE)]
        pub or_replace: bool,
        pub type_name: crate::ast::shared::names::TypeName,
        /// gram.y `LANGUAGE name` (REL_17_11 gram.y 9136): a `ColId`, never
        /// a string (issue #52).
        #[tok(LANGUAGE, this)]
        pub lang_name: crate::tokens::ColId,
        pub elements: TransformElementList,
    }
}

recursa::ast_node! {
    /// `DROP TRANSFORM [IF EXISTS] FOR Typename LANGUAGE name [CASCADE|RESTRICT]`.
    #[derive(Debug)]
    #[tok(DROP, TRANSFORM, this)]
    pub struct DropTransformStmt {
        pub if_exists: Option<IfExists>,
        #[tok(FOR, this)]
        pub type_name: crate::ast::shared::names::TypeName,
        /// gram.y `LANGUAGE name` (REL_17_11 gram.y 9168): a `ColId`, never
        /// a string (issue #52).
        #[tok(LANGUAGE, this)]
        pub lang_name: crate::tokens::ColId,
        pub behavior: Option<DropBehavior>,
    }
}
