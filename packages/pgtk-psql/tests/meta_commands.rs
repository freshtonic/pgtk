//! psql meta-commands: where each command and each argument ends, the
//! argument values, `\set` and `\unset`, and the `\if` blocks.
//!
//! The boundaries are written against `src/bin/psql/psqlscanslash.l` and
//! `src/bin/psql/command.c`. `tests/psql_oracle.rs` compares the same facts
//! with psql's own lexers.

use pgtk_psql::{
    ArgumentKind, ConditionalErrorKind, MetaCommand, PsqlItem, Terminator, Variables, meta,
};

fn variables(pairs: &[(&str, &str)]) -> Variables {
    pairs.iter().copied().collect()
}

fn render(source: &str, bindings: &Variables) -> String {
    pgtk_psql::render(source, bindings)
        .unwrap_or_else(|error| panic!("{source:?} must parse as psql: {error}"))
        .sql()
        .to_owned()
}

/// Each meta-command of `source`: its text, its name and its arguments.
fn commands(source: &str) -> Vec<(String, String, Vec<String>)> {
    pgtk_psql::parse(source)
        .unwrap_or_else(|error| panic!("{source:?} must parse as psql: {error}"))
        .items
        .iter()
        .filter_map(|item| {
            let command = match item {
                PsqlItem::Command(command) => command.command(),
                PsqlItem::Terminator(Terminator::Send(send)) => send.command(),
                PsqlItem::Terminator(Terminator::Reset(reset)) => reset.command(),
                _ => return None,
            };
            Some((
                command.text().to_owned(),
                command.name().to_owned(),
                command
                    .arguments()
                    .iter()
                    .map(|argument| argument.text().to_owned())
                    .collect(),
            ))
        })
        .collect()
}

fn one(text: &str, name: &str, arguments: &[&str]) -> (String, String, Vec<String>) {
    (
        text.to_owned(),
        name.to_owned(),
        arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect(),
    )
}

// --- Boundaries -------------------------------------------------------------

#[test]
fn an_argument_ends_at_an_unquoted_space_or_backslash() {
    assert_eq!(
        commands(r"\set a 'b c' d"),
        vec![one(r"\set a 'b c' d", "set", &["a", "'b c'", "d"])]
    );
    // A backslash ends the argument and starts the next command.
    assert_eq!(
        commands(r"\set a \x"),
        vec![one(r"\set a ", "set", &["a"]), one(r"\x", "x", &[])]
    );
    // `\'` inside `'...'` does not end the quote, and `"..."` and `` `...` ``
    // are parts of one argument too.
    assert_eq!(
        commands(r#"\set a 'it\'s' "x y"z `echo q`"#),
        vec![one(
            r#"\set a 'it\'s' "x y"z `echo q`"#,
            "set",
            &["a", r"'it\'s'", r#""x y"z"#, "`echo q`"]
        )]
    );
}

#[test]
fn a_double_backslash_ends_the_command_and_sql_continues() {
    let source = r"\set a b \\ SELECT 1;";
    assert_eq!(
        commands(source),
        vec![one(r"\set a b \\", "set", &["a", "b"])]
    );
    let document = pgtk_psql::parse(source).unwrap();
    let PsqlItem::Command(command) = &document.items[0] else {
        panic!("the first item is the command");
    };
    assert_eq!(command.command().separator(), Some(r"\\"));
    assert_eq!(render(source, &Variables::new()), " SELECT 1;");
}

#[test]
fn a_command_name_ends_at_space_or_backslash() {
    // `\setx` is not `\set` with an argument.
    let document = pgtk_psql::parse(r"\setx 1").unwrap();
    assert!(matches!(
        document.items[0],
        PsqlItem::Command(MetaCommand::Unknown(_))
    ));
    // A name may hold any other character: `\d+` and `\dt+` are commands.
    assert_eq!(
        commands(r"\d+ users"),
        vec![one(r"\d+ users", "d+", &["users"])]
    );
}

#[test]
fn chained_commands_on_one_line_are_separate() {
    assert_eq!(
        commands("\\x \\x\nSELECT 2;"),
        vec![one(r"\x ", "x", &[]), one(r"\x", "x", &[])]
    );
    // A command after SQL text on the same line ends the text run, and SQL
    // continues on the next line.
    let source = "SELECT 1 \\set x 1\nFROM t;";
    assert_eq!(commands(source), vec![one(r"\set x 1", "set", &["x", "1"])]);
    assert_eq!(render(source, &Variables::new()), "SELECT 1 \nFROM t;");
}

#[test]
fn a_whole_line_argument_runs_to_the_end_of_the_line() {
    let source = "\\copy t from stdin \\\\ x\nSELECT 1;";
    assert_eq!(
        commands(source),
        vec![one(
            r"\copy t from stdin \\ x",
            "copy",
            &[r"t from stdin \\ x"]
        )]
    );
    // `\copy` is matched without regard to case, as `exec_command` does.
    assert_eq!(commands(r"\COPY t to stdout")[0].1, "COPY");
    for name in [
        "ef",
        "ev",
        "h",
        "help",
        "sf",
        "sf+",
        "sv",
        "sv+",
        "unrestrict",
        "!",
    ] {
        let source = format!("\\{name} a \\\\ b");
        let found = commands(&source);
        assert_eq!(found.len(), 1, "{source:?}");
        assert_eq!(found[0].0, source, "{source:?} is one command");
    }
}

#[test]
fn a_file_or_pipe_argument_that_starts_with_a_bar_runs_to_the_end_of_the_line() {
    assert_eq!(
        commands(r"\o |cat \\ x"),
        vec![one(r"\o |cat \\ x", "o", &[r"|cat \\ x"])]
    );
    assert_eq!(
        commands(r"\o out.txt \\ SELECT 1;"),
        vec![one(r"\o out.txt \\", "o", &["out.txt"])]
    );
    // `\g (options)` and then the file or pipe.
    assert_eq!(
        commands(r"SELECT 1 \g (format=csv) |cat a"),
        vec![one(
            r"\g (format=csv) |cat a",
            "g",
            &["(format=csv)", "|cat a"]
        )]
    );
    let document = pgtk_psql::parse(r"SELECT 1 \g x").unwrap();
    let Some(PsqlItem::Terminator(Terminator::Send(send))) = document.items.last() else {
        panic!("the last item is the send command");
    };
    assert_eq!(send.command().argument_kind(), ArgumentKind::Send);
}

#[test]
fn a_send_command_renders_as_a_boundary_without_its_arguments() {
    for (source, expected) in [
        (r"SELECT 1 \g out.txt", "SELECT 1 ;"),
        (r"SELECT 1 \gset pre_", "SELECT 1 ;"),
        (r"SELECT 1 \gx (format=csv)", "SELECT 1 ;"),
        (r"SELECT 1 \crosstabview a b", "SELECT 1 ;"),
        (r"SELECT 1 \watch 1", "SELECT 1 ;"),
        (r"SELECT 1 \g \\ SELECT 2;", "SELECT 1 ; SELECT 2;"),
    ] {
        assert_eq!(render(source, &Variables::new()), expected, "{source:?}");
    }
}

#[test]
fn reset_clears_the_query_buffer() {
    let source = "SELECT 1;\nSELECT 2 \\r\nSELECT 3;";
    assert_eq!(render(source, &Variables::new()), "SELECT 1;\n\nSELECT 3;");
    assert_eq!(render("SELECT 2 \\reset", &Variables::new()), "");
}

// --- Argument values --------------------------------------------------------

/// The value of the only argument of `\echo <argument>`.
fn value(argument: &str, bindings: &Variables) -> Option<String> {
    let source = format!("\\echo {argument}");
    let document = pgtk_psql::parse(&source).unwrap();
    let PsqlItem::Command(command) = &document.items[0] else {
        panic!("{source:?} is one command");
    };
    let arguments = command.command().arguments();
    assert_eq!(arguments.len(), 1, "{source:?} has one argument");
    arguments[0].value(bindings)
}

#[test]
fn an_argument_value_is_read_as_psql_scan_slash_option_reads_it() {
    let none = Variables::new();
    assert_eq!(value("plain", &none).as_deref(), Some("plain"));
    assert_eq!(value("'a b'", &none).as_deref(), Some("a b"));
    assert_eq!(value("'it''s'", &none).as_deref(), Some("it's"));
    assert_eq!(value(r"'a\tb\nc'", &none).as_deref(), Some("a\tb\nc"));
    assert_eq!(value(r"'\101\x42'", &none).as_deref(), Some("AB"));
    assert_eq!(value(r"'\q'", &none).as_deref(), Some("q"));
    // A double-quoted part keeps its quotes.
    assert_eq!(value(r#"x"a b"y"#, &none).as_deref(), Some(r#"x"a b"y"#));
    // A backquoted shell command is never run.
    assert_eq!(value("`date`", &none), None);
}

#[test]
fn an_argument_value_substitutes_variables() {
    let bindings = variables(&[("v", "it's")]);
    assert_eq!(value(":v", &bindings).as_deref(), Some("it's"));
    assert_eq!(value(":'v'", &bindings).as_deref(), Some("'it''s'"));
    assert_eq!(value(":\"v\"", &bindings).as_deref(), Some("\"it's\""));
    assert_eq!(value(":{?v}", &bindings).as_deref(), Some("TRUE"));
    assert_eq!(value(":{?w}", &bindings).as_deref(), Some("FALSE"));
    // An unset variable stays as it is spelled.
    assert_eq!(value(":w", &bindings).as_deref(), Some(":w"));
    assert_eq!(value("a:v", &bindings).as_deref(), Some("ait's"));
}

#[test]
fn a_boolean_is_read_as_parse_variable_bool_reads_it() {
    for (text, expected) in [
        ("true", Some(true)),
        ("T", Some(true)),
        ("yes", Some(true)),
        ("on", Some(true)),
        ("1", Some(true)),
        ("false", Some(false)),
        ("f", Some(false)),
        ("no", Some(false)),
        ("off", Some(false)),
        ("of", Some(false)),
        ("0", Some(false)),
        ("o", None),
        ("", None),
        ("2", None),
        ("truex", None),
    ] {
        assert_eq!(meta::parse_bool(text), expected, "{text:?}");
    }
}

// --- \set and \unset --------------------------------------------------------

#[test]
fn set_changes_the_variables_of_the_rest_of_the_script() {
    let source = "SELECT :v;\n\\set v 4 2\nSELECT :v;\n\\unset v\nSELECT :v;";
    let rendered = pgtk_psql::render(source, &Variables::new()).unwrap();
    // `\set` concatenates its values; `\unset` removes the variable.
    assert_eq!(rendered.sql(), "SELECT :v;\n\nSELECT 42;\n\nSELECT :v;");
    assert_eq!(rendered.unbound().len(), 2);
    assert_eq!(rendered.variables().get("v"), None);

    let source = "\\set t users\n\\set q 'it''s'\nSELECT * FROM :\"t\" WHERE n = :'q';";
    assert_eq!(
        render(source, &Variables::new()),
        "\n\nSELECT * FROM \"users\" WHERE n = 'it''s';"
    );
    // `\set name` sets the empty string, and a value can use a variable.
    let rendered = pgtk_psql::render("\\set a\n\\set b x:c", &variables(&[("c", "1")])).unwrap();
    assert_eq!(rendered.variables().get("a"), Some(""));
    assert_eq!(rendered.variables().get("b"), Some("x1"));
}

#[test]
fn set_with_a_backquoted_value_is_not_evaluated() {
    let rendered = pgtk_psql::render("\\set v `date`\nSELECT :v;", &Variables::new()).unwrap();
    assert_eq!(rendered.sql(), "\nSELECT :v;");
    assert_eq!(rendered.unevaluated(), std::slice::from_ref(&(0..13)));
}

#[test]
fn set_with_an_invalid_name_changes_nothing() {
    let rendered = pgtk_psql::render("\\set 'a b' 1", &Variables::new()).unwrap();
    assert_eq!(rendered.variables(), &Variables::new());
}

// --- \if blocks -------------------------------------------------------------

#[test]
fn only_the_active_branch_is_sent() {
    let source = "\\if :x\nSELECT 1;\n\\elif :y\nSELECT 2;\n\\else\nSELECT 3;\n\\endif\n";
    for (bindings, expected) in [
        (variables(&[("x", "on")]), "\nSELECT 1;\n\n"),
        (variables(&[("x", "off"), ("y", "yes")]), "\nSELECT 2;\n\n"),
        (variables(&[("x", "0"), ("y", "0")]), "\nSELECT 3;\n\n"),
        // A value that psql does not recognise as a Boolean is false.
        (Variables::new(), "\nSELECT 3;\n\n"),
    ] {
        assert_eq!(render(source, &bindings), expected, "{bindings:?}");
    }
}

#[test]
fn a_set_in_an_inactive_branch_changes_nothing() {
    let source = "\\if false\n\\set v 1\n\\else\n\\set v 2\n\\endif\nSELECT :v;";
    assert_eq!(render(source, &Variables::new()), "\n\n\nSELECT 2;");
}

#[test]
fn a_nested_block_in_an_inactive_branch_is_not_evaluated() {
    let source = "\\if false\n\\if true\nSELECT 1;\n\\endif\n\\else\nSELECT 2;\n\\endif";
    assert_eq!(render(source, &Variables::new()), "\nSELECT 2;\n");
    let source = "\\if true\n\\if false\nSELECT 1;\n\\else\nSELECT 2;\n\\endif\n\\endif";
    assert_eq!(render(source, &Variables::new()), "\n\nSELECT 2;\n\n");
}

#[test]
fn an_if_inside_a_statement_keeps_the_active_text() {
    let source = "select\n\\if true\n42\n\\else\n(bogus\n\\endif\nforty_two;";
    assert_eq!(
        render(source, &Variables::new()),
        "select\n\n42\n\nforty_two;"
    );
}

#[test]
fn an_if_with_a_backquoted_expression_is_false_and_reported() {
    let rendered = pgtk_psql::render("\\if `true`\nSELECT 1;\n\\endif", &Variables::new()).unwrap();
    assert_eq!(rendered.sql(), "");
    assert_eq!(rendered.unevaluated(), std::slice::from_ref(&(0..10)));
}

#[test]
fn the_conditional_structure_is_found_without_evaluation() {
    let source = "\\if a\n\\if b\n\\endif\n\\elif c\n\\else\nSELECT 1;\n\\endif";
    let document = pgtk_psql::parse(source).unwrap();
    let (blocks, errors) = document.conditionals(source);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(blocks.len(), 2);
    // The inner block closes first.
    assert_eq!(blocks[0].depth, 1);
    assert_eq!(blocks[0].branches.len(), 1);
    assert_eq!(blocks[1].depth, 0);
    assert_eq!(blocks[1].branches.len(), 3);
    assert!(blocks[1].has_else(&document));
    let else_items = blocks[1].branch_items(2);
    assert!(
        document.items[else_items]
            .iter()
            .any(|item| matches!(item, PsqlItem::Terminator(Terminator::Semi)))
    );
}

#[test]
fn a_misplaced_conditional_command_is_reported() {
    for (source, kind, at) in [
        ("\\elif x", ConditionalErrorKind::ElifWithoutIf, 0..7),
        ("\\else", ConditionalErrorKind::ElseWithoutIf, 0..5),
        ("\\endif", ConditionalErrorKind::EndifWithoutIf, 0..6),
        (
            "\\if x\n\\else\n\\else\n\\endif",
            ConditionalErrorKind::ElseAfterElse,
            12..17,
        ),
        (
            "\\if x\n\\else\n\\elif y\n\\endif",
            ConditionalErrorKind::ElifAfterElse,
            12..19,
        ),
        (
            "SELECT 1;\n\\if x",
            ConditionalErrorKind::MissingEndif,
            10..15,
        ),
    ] {
        let document = pgtk_psql::parse(source).unwrap();
        let (_, errors) = document.conditionals(source);
        assert_eq!(errors.len(), 1, "{source:?}: {errors:?}");
        assert_eq!(errors[0].kind, kind, "{source:?}");
        assert_eq!(errors[0].source, at, "{source:?}");
        // The document still parses and renders: psql reports these when it
        // runs the script.
        pgtk_psql::render(source, &Variables::new()).unwrap();
    }
}
