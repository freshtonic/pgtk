//! The psql document AST.
//!
//! A psql document is a sequence of items: runs of SQL text the client
//! forwards untouched, variable interpolations it rewrites, the terminators
//! that submit a query buffer, and the meta-commands that psql runs itself.
//! That is what `vendor/postgres/src/fe_utils/psqlscan.l` and
//! `vendor/postgres/src/bin/psql/psqlscanslash.l` distinguish, and this AST
//! models no more than that: the SQL inside a text run is not parsed here,
//! because psql does not parse it either.
//!
//! # Meta-commands
//!
//! Each backslash command is one token: the backslash, the command name, and
//! the argument region that psql reads for that command. The token patterns
//! follow the states of `psqlscanslash.l`:
//!
//! - `xslashcmd`: a command name ends at `space` or at a backslash;
//! - `xslashargstart` and `xslasharg`: an argument starts after `space` and
//!   ends at an unquoted `space` or backslash;
//! - `xslashquote`, `xslashdquote` and `xslashbackquote`: `'...'` (with
//!   backslash escapes), `"..."` and `` `...` `` are parts of one argument;
//! - `xslashwholeline`: an `OT_WHOLE_LINE` argument runs to the end of the
//!   line, and an `OT_FILEPIPE` argument does so when it starts with `|`;
//! - `xslashend`: a `\\` after the arguments ends the command, and SQL text
//!   continues after it.
//!
//! psql reads one line at a time, so no argument region crosses a line
//! break. The argument type of each command comes from `exec_command` in
//! `src/bin/psql/command.c`. `scripts/meta-command-patterns.py` makes the
//! patterns; edit the script, not the patterns.

recursa::ast_node! {
    /// One complete psql source document.
    ///
    /// The item list covers the whole source: the repetition ends only at end of
    /// input, so an empty document is an empty list.
    #[derive(Debug, Clone, derive_more :: Deref)]
    pub struct PsqlDocument {
        #[deref]
        pub items: zero_or_many!(PsqlItem),
    }
}

recursa::ast_node! {
    /// One item of a psql document.
    #[derive(Debug, Clone)]
    pub enum PsqlItem {
        /// A variable interpolation, which substitution rewrites.
        Interpolation(Interpolation),
        /// A terminator, which submits the query buffer.
        Terminator(Terminator),
        /// A meta-command that does not end the query buffer.
        Command(MetaCommand),
        /// A run of text psql forwards to the server unchanged.
        Sql(SqlText),
    }
}

recursa::ast_node! {
    /// A psql variable interpolation.
    ///
    /// Each form is one lexical token, exactly as `psqlscan.l` scans it. That
    /// matters: pgtk-sql previously lexed the colon and the name separately and
    /// joined them in the SQL grammar, which made `SELECT int :'x'` ambiguous
    /// against the SQL/JSON `int : value` entry and cost 8 LALR conflicts.
    ///
    /// Every form shares one name class, `psqlscan.l:376` `variable_char`,
    /// written there as `[A-Za-z\200-\377_0-9]`. The high-byte range covers
    /// every non-ASCII byte, which in UTF-8 source is every non-ASCII scalar.
    /// Unlike an identifier it has no separate start class, so `:1` is a
    /// variable reference.
    ///
    /// An incomplete form falls back to a bare colon followed by ordinary text,
    /// which is what `psqlscan.l:775-796` does with `yyless(1)`: `:'a b'` is a
    /// colon and a string literal, not an interpolation, because a space is not
    /// a `variable_char`.
    #[derive(Debug, Clone)]
    pub enum Interpolation {
        /// `:'name'` — psqlscan.l:756. Substituted as a SQL string literal.
        Literal(#[lex(pattern = r":'[A-Za-z0-9_\u{0080}-\u{10FFFF}]+'")] LiteralInterpolation),
        /// `:"name"` — psqlscan.l:761. Substituted as a quoted identifier.
        Identifier(
            #[lex(pattern = r#":"[A-Za-z0-9_\u{0080}-\u{10FFFF}]+""#)] IdentifierInterpolation,
        ),
        /// `:{?name}` — psqlscan.l:766. Substituted as `TRUE` or `FALSE`
        /// according to whether the variable is set.
        Test(#[lex(pattern = r":\{\?[A-Za-z0-9_\u{0080}-\u{10FFFF}]+\}")] TestInterpolation),
        /// `:name` — psqlscan.l:710. Substituted as raw text.
        ///
        /// Listed last so the three punctuated forms are tried first; they are
        /// separate token kinds, so the order records intent rather than
        /// resolving a conflict.
        Raw(#[lex(pattern = r":[A-Za-z0-9_\u{0080}-\u{10FFFF}]+")] RawInterpolation),
    }
}

impl<'input> Interpolation<'input> {
    /// The exact source spelling, punctuation included.
    pub fn text(&self) -> &str {
        match self {
            Self::Literal(token) => token.text(),
            Self::Identifier(token) => token.text(),
            Self::Test(token) => token.text(),
            Self::Raw(token) => token.text(),
        }
    }

    /// The variable name, with the interpolation's punctuation removed.
    ///
    /// The lexical patterns fix each form's shape, and every delimiter is
    /// ASCII, so the byte slices below always land on scalar boundaries.
    pub fn name(&self) -> &str {
        let text = self.text();
        match self {
            // `:'name'` and `:"name"` — two leading bytes, one trailing.
            Self::Literal(_) | Self::Identifier(_) => &text[2..text.len() - 1],
            // `:{?name}` — three leading bytes, one trailing.
            Self::Test(_) => &text[3..text.len() - 1],
            // `:name` — one leading byte.
            Self::Raw(_) => &text[1..],
        }
    }
}

recursa::ast_node! {
    /// A terminator: psql ends the current query buffer and starts a new one.
    #[derive(Debug, Clone)]
    pub enum Terminator {
        /// `;` — psqlscan.l:681, the only terminator `psqlscan.l` itself
        /// returns. The server sees it unchanged, so rendering copies it.
        ///
        /// psql suppresses the boundary inside parentheses and inside a tracked
        /// `BEGIN ... END` block; this crate does not track either, because it
        /// renders one SQL string for the whole document rather than splitting
        /// it into submissions. The distinction is invisible to the rendered
        /// text, which pgtk-sql then frames on its own semicolons.
        #[tok(SEMI)]
        Semi,
        /// `\g`, `\gx`, `\gset`, `\gdesc`, `\gexec`, `\crosstabview` and
        /// `\watch` — client syntax the server never sees, so rendering
        /// replaces the command and its arguments with `;`. From 18 also
        /// `\parse` and `\sendpipeline`.
        Send(SendCommand),
        /// `\r` and `\reset`: psql clears the query buffer and sends nothing
        /// (`command.c`, `exec_command_reset`). Rendering removes the buffer
        /// text together with the command.
        Reset(ResetCommand),
        /// Added in 18: a command that ends the query buffer but does not
        /// send its text (docs/research/psql-14-19-syntax-changes.md, class
        /// (A) item A1; `REL_18_6:command.c` 350-452, `mainloop.c` 513).
        #[cfg(feature = "since-pg18")]
        Discard(DiscardCommand),
    }
}

recursa::ast_node! {
    /// A psql send command, with its arguments.
    ///
    /// psql sends the query buffer after each of these: the command returns
    /// `PSQL_CMD_SEND`, or, for `\watch`, runs the buffer and then resets it
    /// (`command.c`, `exec_command_watch`).
    ///
    /// Each carries an explicit priority so it wins the tie against the
    /// [`MetaCommand::Unknown`] catch-all, which matches the same name. Where
    /// the catch-all matches a *longer* name it wins on length instead, which
    /// is what makes `\gsetfoo` one unknown command rather than `\gset`
    /// followed by `foo` — `psqlscanslash.l` reads a whole command name before
    /// looking it up.
    ///
    /// `\g` and `\gx` read an `OT_FILEPIPE` argument: when it starts with
    /// `|`, it runs to the end of the line. A first argument that starts with
    /// `(` opens the parenthesised `\pset` options of `\g`, and the file or
    /// pipe argument follows them (`command.c`, `exec_command_g`).
    #[derive(Debug, Clone)]
    pub enum SendCommand {
        /// `\crosstabview [colV [colH [colD [sortcolH]]]]`
        Crosstabview(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\crosstabview(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\crosstabview(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            SendCrosstabview,
        ),
        /// Added in 18: `\parse statement_name` sends the buffer text with
        /// `PQsendPrepare` (docs/research/psql-14-19-syntax-changes.md, class
        /// (A) item A1; `REL_18_6:command.c` 424, 2508).
        #[cfg(feature = "since-pg18")]
        Parse(
            #[lex(pattern = r#"\\parse(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            SendParse,
        ),
        /// Added in 18: `\sendpipeline` sends the buffer text in pipeline
        /// mode (docs/research/psql-14-19-syntax-changes.md, class (A) item
        /// A1; `REL_18_6:command.c` 440, 2844).
        #[cfg(feature = "since-pg18")]
        Sendpipeline(
            #[lex(pattern = r#"\\sendpipeline(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            SendSendpipeline,
        ),
        /// `\gdesc` sends the buffer text with a Describe, so it is a
        /// statement boundary like the rest
        /// (docs/research/psql-14-19-syntax-changes.md, PostgreSQL 14, "Send
        /// commands"; `REL_17_11:command.c` 1570 returns `PSQL_CMD_SEND`).
        Gdesc(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\gdesc(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\gdesc(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            SendGdesc,
        ),
        /// `\gexec`
        Gexec(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\gexec(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\gexec(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            SendGexec,
        ),
        /// `\gset [prefix]`
        Gset(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\gset(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\gset(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            SendGset,
        ),
        /// `\watch [options]`: runs the query buffer again and again, then
        /// resets it (`REL_17_11:command.c` 2993).
        Watch(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\watch(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\watch(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            SendWatch,
        ),
        /// `\gx [(options)] [filename | |command]`
        Gx(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\gx(?:[ \t\r\f\x0b]+\((?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)*(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*)?(?:[ \t\r\f\x0b]+\|[^\n]*|(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?)"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\gx(?:[ \t\r\f]+\((?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)*(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*)?(?:[ \t\r\f]+\|[^\n]*|(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?)"#, priority = 2))]
            SendGx,
        ),
        /// `\g [(options)] [filename | |command]`
        G(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\g(?:[ \t\r\f\x0b]+\((?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)*(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*)?(?:[ \t\r\f\x0b]+\|[^\n]*|(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?)"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\g(?:[ \t\r\f]+\((?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)*(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*)?(?:[ \t\r\f]+\|[^\n]*|(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?)"#, priority = 2))]
            SendG,
        ),
    }
}

impl<'input> SendCommand<'input> {
    /// The exact source spelling: the command and its arguments.
    pub fn text(&self) -> &str {
        match self {
            Self::Crosstabview(token) => token.text(),
            #[cfg(feature = "since-pg18")]
            Self::Parse(token) => token.text(),
            #[cfg(feature = "since-pg18")]
            Self::Sendpipeline(token) => token.text(),
            Self::Gdesc(token) => token.text(),
            Self::Gexec(token) => token.text(),
            Self::Gset(token) => token.text(),
            Self::Watch(token) => token.text(),
            Self::Gx(token) => token.text(),
            Self::G(token) => token.text(),
        }
    }

    /// The command as `psqlscanslash.l` reads it.
    pub fn command(&self) -> crate::meta::Command<'_> {
        let kind = match self {
            Self::G(_) | Self::Gx(_) => crate::meta::ArgumentKind::Send,
            _ => crate::meta::ArgumentKind::Normal,
        };
        crate::meta::Command::new(self.text(), kind)
    }
}

recursa::ast_node! {
    /// `\r` or `\reset`, which clears the query buffer.
    #[derive(Debug, Clone)]
    pub enum ResetCommand {
        /// `\r` or `\reset`
        Reset(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\(?:r|reset)(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\(?:r|reset)(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            ResetCommandText,
        ),
    }
}

impl<'input> ResetCommand<'input> {
    /// The exact source spelling: the command and its arguments.
    pub fn text(&self) -> &str {
        match self {
            Self::Reset(token) => token.text(),
        }
    }

    /// The command as `psqlscanslash.l` reads it.
    pub fn command(&self) -> crate::meta::Command<'_> {
        crate::meta::Command::new(self.text(), crate::meta::ArgumentKind::Normal)
    }
}

// Added in 18: seven pipeline and prepared-statement commands end the query
// buffer without sending its text (docs/research/psql-14-19-syntax-changes.md,
// class (A) item A1; `REL_18_6:command.c` 350-452, `common.c` 1602-1725).
#[cfg(feature = "since-pg18")]
recursa::ast_node! {
    /// A psql command that ends the query buffer but does not send its text.
    ///
    /// Each of these returns `PSQL_CMD_SEND`, so `MainLoop()` resets the query
    /// buffer after it, as after `\g`. But the send mode of each one does not
    /// send the buffer text to the server. Rendering therefore removes the
    /// text of the buffer together with the command and its arguments, and
    /// the server SQL does not contain it.
    ///
    /// Like [`SendCommand`], each has a priority that wins the tie against
    /// the [`MetaCommand::Unknown`] catch-all, and a longer name wins on
    /// length: `\flushrequest` is not `\flush` followed by `request`.
    #[derive(Debug, Clone)]
    pub enum DiscardCommand {
        /// `\close_prepared statement_name`
        #[cfg(feature = "since-pg18")]
        ClosePrepared(
            #[lex(pattern = r#"\\close_prepared(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            DiscardClosePrepared,
        ),
        /// `\endpipeline`
        #[cfg(feature = "since-pg18")]
        Endpipeline(
            #[lex(pattern = r#"\\endpipeline(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            DiscardEndpipeline,
        ),
        /// `\flushrequest`
        #[cfg(feature = "since-pg18")]
        Flushrequest(
            #[lex(pattern = r#"\\flushrequest(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            DiscardFlushrequest,
        ),
        /// `\flush`
        #[cfg(feature = "since-pg18")]
        Flush(
            #[lex(pattern = r#"\\flush(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            DiscardFlush,
        ),
        /// `\getresults [number_results]`
        #[cfg(feature = "since-pg18")]
        Getresults(
            #[lex(pattern = r#"\\getresults(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            DiscardGetresults,
        ),
        /// `\startpipeline`
        #[cfg(feature = "since-pg18")]
        Startpipeline(
            #[lex(pattern = r#"\\startpipeline(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            DiscardStartpipeline,
        ),
        /// `\syncpipeline`
        #[cfg(feature = "since-pg18")]
        Syncpipeline(
            #[lex(pattern = r#"\\syncpipeline(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            DiscardSyncpipeline,
        ),
    }
}

#[cfg(feature = "since-pg18")]
impl<'input> DiscardCommand<'input> {
    /// The exact source spelling: the command and its arguments.
    pub fn text(&self) -> &str {
        match self {
            Self::ClosePrepared(token) => token.text(),
            Self::Endpipeline(token) => token.text(),
            Self::Flushrequest(token) => token.text(),
            Self::Flush(token) => token.text(),
            Self::Getresults(token) => token.text(),
            Self::Startpipeline(token) => token.text(),
            Self::Syncpipeline(token) => token.text(),
        }
    }

    /// The command as `psqlscanslash.l` reads it.
    pub fn command(&self) -> crate::meta::Command<'_> {
        crate::meta::Command::new(self.text(), crate::meta::ArgumentKind::Normal)
    }
}

recursa::ast_node! {
    /// A psql meta-command that does not end the query buffer.
    ///
    /// The command and its arguments are one token. pgtk-psql never runs a
    /// command: an include is not read, a shell command is not started, and a
    /// connection is not changed. [`crate::render`] applies only `\set`,
    /// `\unset` and the conditional commands, because they change what psql
    /// sends (`packages/pgtk-sql/docs/adr/0013-keep-most-meta-command-arguments-as-text.md`).
    ///
    /// Each named variant has a priority that wins the tie against
    /// [`MetaCommand::Unknown`], which matches the same name.
    #[derive(Debug, Clone)]
    pub enum MetaCommand {
        /// `\set [name [value ...]]`
        Set(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\set(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\set(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            SetCommandText,
        ),
        /// `\unset name`
        Unset(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\unset(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\unset(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            UnsetCommandText,
        ),
        /// `\if expression`
        If(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\if(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\if(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            IfCommandText,
        ),
        /// `\elif expression`
        Elif(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\elif(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\elif(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            ElifCommandText,
        ),
        /// `\else`
        Else(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\else(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\else(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            ElseCommandText,
        ),
        /// `\endif`
        Endif(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\endif(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\endif(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            EndifCommandText,
        ),
        /// `\i`, `\include`, `\ir` and `\include_relative`: input from a file,
        /// which pgtk-psql never reads.
        Include(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\(?:i|include|ir|include_relative)(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\(?:i|include|ir|include_relative)(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            IncludeCommandText,
        ),
        /// `\q` and `\quit`. psql stops reading the script here; pgtk-psql
        /// keeps the text after it.
        Quit(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\(?:q|quit)(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\(?:q|quit)(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            QuitCommandText,
        ),
        /// A command whose argument is the rest of the line: `\copy`, `\ef`,
        /// `\ev`, `\h`, `\help`, `\sf`, `\sf+`, `\sv`, `\sv+`, `\unrestrict`
        /// and `\!`. `\copy` is matched without regard to case, as
        /// `exec_command` matches it.
        WholeLine(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\(?:(?i:copy)|ef|ev|h|help|sf|sf\+|sv|sv\+|unrestrict|!)(?:(?:[ \t\r\f\x0b]|\\)[^\n]*)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\(?:(?i:copy)|ef|ev|h|help|sf|sf\+|sv|sv\+|unrestrict|!)(?:(?:[ \t\r\f]|\\)[^\n]*)?"#, priority = 2))]
            WholeLineCommandText,
        ),
        /// `\o`, `\out`, `\w` and `\write`, whose `OT_FILEPIPE` argument runs
        /// to the end of the line when it starts with `|`.
        Output(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\(?:o|out|w|write)(?:[ \t\r\f\x0b]+\|[^\n]*|(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?)"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\(?:o|out|w|write)(?:[ \t\r\f]+\|[^\n]*|(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?)"#, priority = 2))]
            OutputCommandText,
        ),
        /// Every other command of the target version: `\a`, `\C`, `\c`,
        /// `\connect`, `\cd`, `\conninfo`, `\copyright`, each name that starts
        /// with `d` or `lo_`, `\e`, `\edit`, `\echo`, `\qecho`, `\warn`,
        /// `\encoding`, `\errverbose`, `\f`, `\H`, `\html`, `\l`, `\list`,
        /// `\l+`, `\list+`, `\p`, `\print`, `\password`, `\prompt`, `\pset`,
        /// `\restrict`, `\s`, `\setenv`, `\t`, `\T`, `\timing`, `\x`, `\z` and
        /// `\?`.
        Other(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\(?:a|C|c|connect|cd|conninfo|copyright|d[^ \t\r\f\x0b\n\\]*|e|edit|echo|qecho|warn|encoding|errverbose|f|H|html|l|list|l\+|list\+|lo_[^ \t\r\f\x0b\n\\]*|p|print|password|prompt|pset|restrict|s|setenv|t|T|timing|x|z|\?)(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\(?:a|C|c|connect|cd|conninfo|copyright|d[^ \t\r\f\n\\]*|e|edit|echo|qecho|warn|encoding|errverbose|f|H|html|l|list|l\+|list\+|lo_[^ \t\r\f\n\\]*|p|print|password|prompt|pset|restrict|s|setenv|t|T|timing|x|z|\?)(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            OtherCommandText,
        ),
        /// Added in 15: `\getenv`.
        #[cfg(feature = "since-pg15")]
        OtherSince15(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\getenv(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\getenv(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            OtherSince15CommandText,
        ),
        /// Added in 16: `\bind` and `\zS`.
        #[cfg(feature = "since-pg16")]
        OtherSince16(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\(?:bind|zS)(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\(?:bind|zS)(?:[ \t\r\f]+(?:[^ \t\r\f\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f]*(?:\\\\)?"#, priority = 2))]
            OtherSince16CommandText,
        ),
        /// Added in 18: `\bind_named`, `\lx`, `\listx`, `\lx+`, `\listx+`,
        /// `\l+x`, `\list+x`, `\zx`, `\zSx` and `\zxS`.
        #[cfg(feature = "since-pg18")]
        OtherSince18(
            #[lex(pattern = r#"\\(?:bind_named|lx|listx|lx\+|listx\+|l\+x|list\+x|zx|zSx|zxS)(?:[ \t\r\f\x0b]+(?:[^ \t\r\f\x0b\n\\'"`]|'(?:[^'\\\n]|\\[^\n])*'|"[^"\n]*"|`[^`\n]*`)+)*[ \t\r\f\x0b]*(?:\\\\)?"#, priority = 2)]
            OtherSince18CommandText,
        ),
        /// A name that is not a command of the target version, or a
        /// backslash with no name. psql reports an error and drops the rest
        /// of the line. pgtk-psql keeps the name only, and renders it as it
        /// stands, so that the server SQL shows the error.
        ///
        /// The name never starts with `;` or `:`, because `\;` and `\:` are
        /// [`SqlAtom::Escaped`].
        Unknown(
            #[cfg_attr(feature = "since-pg17", lex(pattern = r#"\\(?:[^ \t\r\f\x0b\n\\;:][^ \t\r\f\x0b\n\\]*)?"#))]
            #[cfg_attr(not(feature = "since-pg17"), lex(pattern = r#"\\(?:[^ \t\r\f\n\\;:][^ \t\r\f\n\\]*)?"#))]
            UnknownCommandText,
        ),
    }
}

impl<'input> MetaCommand<'input> {
    /// The exact source spelling: the command and its arguments.
    pub fn text(&self) -> &str {
        match self {
            Self::Set(token) => token.text(),
            Self::Unset(token) => token.text(),
            Self::If(token) => token.text(),
            Self::Elif(token) => token.text(),
            Self::Else(token) => token.text(),
            Self::Endif(token) => token.text(),
            Self::Include(token) => token.text(),
            Self::Quit(token) => token.text(),
            Self::WholeLine(token) => token.text(),
            Self::Output(token) => token.text(),
            Self::Other(token) => token.text(),
            #[cfg(feature = "since-pg15")]
            Self::OtherSince15(token) => token.text(),
            #[cfg(feature = "since-pg16")]
            Self::OtherSince16(token) => token.text(),
            #[cfg(feature = "since-pg18")]
            Self::OtherSince18(token) => token.text(),
            Self::Unknown(token) => token.text(),
        }
    }

    /// The command as `psqlscanslash.l` reads it.
    pub fn command(&self) -> crate::meta::Command<'_> {
        use crate::meta::ArgumentKind;
        let kind = match self {
            Self::WholeLine(_) => ArgumentKind::WholeLine,
            Self::Output(_) => ArgumentKind::FilePipe,
            Self::Unknown(_) => ArgumentKind::None,
            _ => ArgumentKind::Normal,
        };
        crate::meta::Command::new(self.text(), kind)
    }

    /// Whether the command is `\if`, `\elif`, `\else` or `\endif`.
    pub fn is_conditional(&self) -> bool {
        matches!(
            self,
            Self::If(_) | Self::Elif(_) | Self::Else(_) | Self::Endif(_)
        )
    }
}

recursa::ast_node! {
    /// A maximal run of source that psql forwards to the server unchanged.
    ///
    /// An atom lookahead can either extend this run or begin another `SqlText`
    /// item in the document repetition. Extending the current run is the psql
    /// scanner's maximal-match behaviour, so resolve that exact family of cells
    /// as shifts.
    ///
    /// The lookahead family has one more token in each version range: from
    /// 15 the `Number` atom, and before 15 the `LineComment` atom.
    #[derive(Debug, Clone, derive_more :: Deref)]
    #[cfg_attr(feature = "since-pg15", parse(lr_conflict(
    action = shift,
    against = ast::SqlAtom,
    lookahead = {
        StringText,
        EscapeStringText,
        UnicodeStringText,
        BitStringText,
        HexStringText,
        UnicodeIdentifierText,
        QuotedIdentifierText,
        OpenQuotedText,
        DollarString,
        DollarNumber,
        EscapedText,
        WordText,
        DigitsText,
        PunctText,
        COLONCOLON,
        COLONEQUALS,
        COLON,
        MINUS,
        SLASH,
        DOLLAR,
        NumberText,
    },
    expect = 21
)))]
    #[cfg_attr(not(feature = "since-pg15"), parse(lr_conflict(
    action = shift,
    against = ast::SqlAtom,
    lookahead = {
        StringText,
        EscapeStringText,
        UnicodeStringText,
        BitStringText,
        HexStringText,
        UnicodeIdentifierText,
        QuotedIdentifierText,
        OpenQuotedText,
        DollarString,
        DollarNumber,
        EscapedText,
        WordText,
        DigitsText,
        PunctText,
        COLONCOLON,
        COLONEQUALS,
        COLON,
        MINUS,
        SLASH,
        DOLLAR,
        LineCommentText,
    },
    expect = 21
)))]
    pub struct SqlText {
        #[deref]
        pub atoms: one_or_many!(SqlAtom),
    }
}

recursa::ast_node! {
    /// One lexical unit of a forwarded text run.
    ///
    /// The quoted and dollar-quoted forms are here for one reason: they are the
    /// states in which `psqlscan.l` does *not* recognise an interpolation. A
    /// colon inside a string, a dollar-quoted body, a quoted identifier or a
    /// comment is ordinary content, and it stays ordinary content because the
    /// whole construct is one token. Comments reach the same end by being
    /// ignored trivia, which rendering preserves because it copies source.
    #[derive(Debug, Clone)]
    pub enum SqlAtom {
        /// `'...'` — psqlscan.l:485-491 `xq`, with `''` doubling. psql's
        /// default `standard_conforming_strings` is on, so a backslash here is
        /// an ordinary character.
        String(#[lex(pattern = r"'[^']*(?:''[^']*)*'")] StringText),
        /// `E'...'` — psqlscan.l:492-495 `xe`, where a backslash escapes.
        ///
        /// The escape is `[\s\S]` rather than `.`: psqlscan.l:213 spells it
        /// `xeescape [\\][^0-7]`, a negated class that matches a newline, while
        /// `.` here does not.
        EscapeString(#[lex(pattern = r"(?i:E)'(?:[^'\\]|\\[\s\S]|'')*'")] EscapeStringText),
        /// `U&'...'` — psqlscan.l `xus`.
        UnicodeString(#[lex(pattern = r"(?i:U)&'[^']*(?:''[^']*)*'")] UnicodeStringText),
        /// `B'...'` — psqlscan.l `xb`.
        BitString(#[lex(pattern = r"(?i:B)'[^']*'")] BitStringText),
        /// `X'...'` — psqlscan.l `xh`.
        HexString(#[lex(pattern = r"(?i:X)'[^']*'")] HexStringText),
        /// `U&"..."` — psqlscan.l `xui`.
        UnicodeIdentifier(#[lex(pattern = r#"(?i:U)&"[^"]*(?:""[^"]*)*""#)] UnicodeIdentifierText),
        /// `"..."` — psqlscan.l:622-624 `xd`, with `""` doubling.
        QuotedIdentifier(#[lex(pattern = r#""[^"]*(?:""[^"]*)*""#)] QuotedIdentifierText),
        /// A string or a quoted identifier that reaches end of input before
        /// its closing quote. psqlscan.l:951-983 has one `<<EOF>>` rule for
        /// every exclusive state, so psql asks for another line inside an open
        /// `xq`, `xe`, `xus`, `xb`, `xh`, `xd` or `xui` and never scans the rest
        /// for an interpolation (freshtonic/pg-sql#89).
        ///
        /// Each alternative is a closed form above without its closing quote,
        /// so it stops before the first quote that would close the region.
        /// Where that quote follows, the closed form is one character longer
        /// and wins; only at end of input does this one win. The `xe`
        /// alternative also takes a final lone backslash, which has nothing
        /// left to escape.
        OpenQuoted(
            #[lex(
                pattern = r#"'[^']*(?:''[^']*)*|(?i:E)'(?:[^'\\]|\\[\s\S]|'')*\\?|(?i:U)&'[^']*(?:''[^']*)*|(?i:B)'[^']*|(?i:X)'[^']*|(?i:U)&"[^"]*(?:""[^"]*)*|"[^"]*(?:""[^"]*)*"#
            )]
            OpenQuotedText,
        ),
        /// `$tag$...$tag$` — psqlscan.l:564-601 `xdolq`.
        DollarString(#[lex(matcher)] DollarString),
        /// `$1` — psqlscan.l:352 `param`, which is `\${decdigit}+` and carries
        /// no exclusion: `$1a` is a param and an identifier, and psql forwards
        /// both for the server to object to.
        DollarNumber(#[lex(pattern = r"\$[0-9]+")] DollarNumber),
        /// `\;` and `\:` — psqlscan.l:697-702, whose rule is `"\\"[;:]` and
        /// whose body emits `yytext + 1`. Neither is a submission boundary: each
        /// forces its second character into the query buffer, so the server
        /// receives one byte where the user wrote two.
        ///
        /// `\:` is how a psql user writes a colon that must *not* be
        /// interpolated, so this token is what stops `\:name` becoming a
        /// variable reference. It has to be one token to do that: a bare
        /// backslash followed by a separate colon would leave `:name` to the
        /// interpolation rule.
        Escaped(#[lex(pattern = r"\\[;:]")] EscapedText),
        /// An identifier-shaped run, `psqlscan.l:340` `identifier`. `$` is a
        /// continuation character there, so `a$$b$$` is one identifier and not
        /// an identifier followed by a dollar-quoted string.
        Word(
            #[lex(pattern = r"[A-Za-z_\u{0080}-\u{10FFFF}][A-Za-z0-9_$\u{0080}-\u{10FFFF}]*")]
            WordText,
        ),
        /// A run of digits. psql's numeric rules are finer, but nothing here
        /// interprets a number, and no numeric spelling can contain or start an
        /// interpolation.
        Digits(#[lex(pattern = r"[0-9]+")] DigitsText),
        /// From 15: a number or `$n` that an identifier follows directly, or a
        /// number with an exponent sign. psqlscan.l copies the trailing-junk
        /// rules of scan.l, `integer_junk`, `decimal_junk`, `real_junk`,
        /// `param_junk` and `realfail`, and echoes each as one token
        /// (docs/research/psql-14-19-syntax-changes.md, class (A) item A2;
        /// `REL_15_19:psqlscan.l` 341-348, commits 2549f0661, f37ac613a83).
        ///
        /// The extent matters where a quote or a `-` follows: from 15,
        /// `1e'\'` is `1e` and a standard string, and `1e--c` has no comment.
        /// Before 15, `realfail1` and `realfail2` give back the `e` and the
        /// sign, so `e'\'` starts an escape string and `--c` is a comment.
        #[cfg(feature = "since-pg15")]
        Number(
            #[lex(
                pattern = r"(?:[0-9]+|[0-9]*\.[0-9]+|[0-9]+\.[0-9]*)(?:[eE][+-]?[0-9]+)?[A-Za-z_\u{0080}-\u{10FFFF}][A-Za-z0-9_$\u{0080}-\u{10FFFF}]*|(?:[0-9]+|[0-9]*\.[0-9]+|[0-9]+\.[0-9]*)[eE][+-][0-9]*|\$[0-9]+[A-Za-z_\u{0080}-\u{10FFFF}][A-Za-z0-9_$\u{0080}-\u{10FFFF}]*"
            )]
            NumberText,
        ),
        /// Before 15: a `--` comment, which psql removes from the text that it
        /// sends. psqlscan.l's `{whitespace}` rule does not echo a match that
        /// starts with `-` (docs/research/psql-14-19-syntax-changes.md, class
        /// (A) item A3; `REL_14_24:psqlscan.l` 388; commit 83884682f restores
        /// the comments in 15). Rendering removes the comment and keeps the
        /// line ending.
        ///
        /// The token also takes the line ending, so it has the extent of the
        /// ignored `LineComment` and wins the tie by its priority.
        #[cfg(not(feature = "since-pg15"))]
        LineComment(
            #[lex(pattern = r"--[^\r\n]*(?:\r\n|\r|\n)?", priority = 3)] LineCommentText,
        ),
        /// Any other run of operator and delimiter characters. The excluded
        /// characters are exactly those that can begin a longer token above.
        Punct(#[lex(pattern = r#"[^\s\-/:;'"$\\A-Za-z0-9_\u{0080}-\u{10FFFF}]+"#)] PunctText),
        #[tok(COLONCOLON)]
        Cast,
        #[tok(COLONEQUALS)]
        ColonEquals,
        #[tok(COLON)]
        Colon,
        #[tok(MINUS)]
        Minus,
        #[tok(SLASH)]
        Slash,
        #[tok(DOLLAR)]
        Dollar,
    }
}
