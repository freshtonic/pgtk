/// ANALYZE statement AST: `ANALYZE [table [(col, ...)]]`.
use crate::tokens::literal;

recursa::ast_node! {
    /// ANALYZE statement with optional qualified table name and column list.
    ///
    /// ```sql
    /// ANALYZE [VERBOSE] [table_name [(column, ...)]]
    /// ```
    ///
    /// gram.y `AnalyzeStmt` has either the `VERBOSE` keyword or the
    /// parenthesized options, never both (REL_17_11 gram.y 11832-11855), so
    /// they are the two variants of [`AnalyzeForm`] (issue #58).
    #[derive(Debug)]
    #[tok(ANALYZE, this)]
    pub struct AnalyzeStmt {
        pub form: AnalyzeForm,
        #[sep(COMMA)]
        pub targets: Option<one_or_many!(AnalyzeTarget)>,
    }
}

recursa::ast_node! {
    /// The options of `ANALYZE`: the parenthesized list, or the legacy
    /// optional `VERBOSE` keyword.
    ///
    /// Variant ordering: `Options` starts with `(`; `Legacy` may be empty.
    #[derive(Debug)]
    pub enum AnalyzeForm {
        /// Parenthesized options list, e.g.
        /// `(VERBOSE, SKIP_LOCKED, BUFFER_USAGE_LIMIT '512 kB')`.
        Options(AnalyzeOptions),
        Legacy(AnalyzeLegacy),
    }
}

recursa::ast_node! {
    /// gram.y `opt_verbose` of the legacy `ANALYZE` form.
    #[derive(Debug)]
    pub struct AnalyzeLegacy {
        /// Optional `VERBOSE` keyword (legacy bareword form).
        #[presence(VERBOSE)]
        pub verbose: bool,
    }
}

impl<'input> AnalyzeStmt<'input> {
    /// Whether the legacy `VERBOSE` keyword is present.
    pub fn verbose(&self) -> bool {
        matches!(&self.form, AnalyzeForm::Legacy(legacy) if legacy.verbose)
    }

    /// The parenthesized options list.
    pub fn options(&self) -> Option<&AnalyzeOptions<'input>> {
        match &self.form {
            AnalyzeForm::Options(options) => Some(options),
            AnalyzeForm::Legacy(_) => None,
        }
    }
}

recursa::ast_node! {
    /// Parenthesized options owned as one comma-separated list.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct AnalyzeOptions(
        #[sep(COMMA)]
        #[deref]
        pub one_or_many!(AnalyzeOption),
    );
}

recursa::ast_node! {
    /// One option inside the parenthesized `ANALYZE (...)` options list.
    ///
    /// Each option is a keyword-ish name (so we use `AliasName` to tolerate
    /// identifiers that happen to collide with keywords) followed by an optional
    /// value (string literal, integer, or ON/OFF-style AliasName).
    #[derive(Debug)]
    pub struct AnalyzeOption {
        pub name: literal::AliasName,
        pub value: Option<AnalyzeOptionValue>,
    }
}

recursa::ast_node! {
    #[derive(Debug)]
    pub enum AnalyzeOptionValue {
        // Use the canonical content token from `tokens::literal`. Keeping an
        // inline lexer declaration here would create a second `StringLit`
        // definition and prevent lookahead filters from naming it uniquely.
        String(literal::StringLit),
        /// gram.y `utility_option_arg: ... | NumericOnly` (REL_17_11 gram.y
        /// 11884): a signed integer or float, which includes an integer above
        /// `int32` (issue #88).
        Number(crate::ast::shared::numbers::NumericOnly),
        Name(literal::AliasName),
    }
}

recursa::ast_node! {
    /// Optional parenthesized column list on an ANALYZE target.
    #[derive(Debug, derive_more :: Deref)]
    #[tok(LPAREN, this, RPAREN)]
    pub struct AnalyzeColumnList(
        #[sep(COMMA)]
        #[deref]
        pub one_or_many!(crate::tokens::ColId),
    );
}

recursa::ast_node! {
    /// `table_name [(column, ...)]` target of an ANALYZE statement.
    #[derive(Debug)]
    pub struct AnalyzeTarget {
        /// `ANALYZE` and `VACUUM` share gram.y `vacuum_relation`, so they share
        /// the node that holds its relation, and with it the two shapes that
        /// 18 adds (`crate::ast::utility::vacuum::VacuumRelationName`).
        pub table_name: crate::ast::utility::vacuum::VacuumRelationName,
        pub columns: Option<AnalyzeColumnList>,
    }
}

impl<'input> AnalyzeTarget<'input> {
    /// The name of the target relation, whichever form wraps it.
    pub fn relation_name(&self) -> &crate::ast::shared::names::QualifiedName<'input> {
        self.table_name.name()
    }
}
