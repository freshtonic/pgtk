//! DO anonymous code block.

use crate::ast::shared::expr::Sconst;

// --- DO ---

recursa::ast_node! {
    /// `DO { body | LANGUAGE lang } ...` anonymous code block: gram.y
    /// `DoStmt: DO dostmt_opt_list`, with `dostmt_opt_list` one or more
    /// `dostmt_opt_item`s (REL_17_11 gram.y 9037-9061; the rules are the same
    /// from 14 to 19). The raw parser takes the items in any order and allows
    /// repeats; `ExecuteDoStmt` refuses a missing body and a repeated item
    /// after parsing, so `DO LANGUAGE plpgsql` and `DO 'a' 'b'` parse here.
    #[derive(Debug)]
    #[tok(DO, this)]
    pub struct DoStmt {
        pub items: one_or_many!(DoItem),
    }
}

recursa::ast_node! {
    /// gram.y `dostmt_opt_item: Sconst | LANGUAGE NonReservedWord_or_Sconst`.
    #[derive(Debug)]
    pub enum DoItem {
        /// The code block: any `Sconst` spelling, not only a dollar-quoted
        /// string (issue #102).
        Body(Sconst),
        Language(DoLanguage),
    }
}

recursa::ast_node! {
    /// `LANGUAGE lang` item of a `DO` block.
    #[derive(Debug)]
    pub struct DoLanguage {
        #[tok(LANGUAGE, this)]
        pub name: DoLanguageName,
    }
}

recursa::ast_node! {
    /// The language of a `DO` block: gram.y `NonReservedWord_or_Sconst`
    /// (REL_17_11 gram.y 1885), a string constant or a word that is not a
    /// reserved keyword.
    #[derive(Debug)]
    pub enum DoLanguageName {
        String(Sconst),
        Word(crate::tokens::NonReservedWord),
    }
}
