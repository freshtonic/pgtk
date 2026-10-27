//! The psql differential: pgtk-psql against psql's own lexer.
//!
//! `pgtk-sql` compares its parses with PostgreSQL's raw parser through
//! `pgtk-oracle`. This suite does the same one layer up: `pgtk-psql-oracle` links
//! `psqlscan.l` and `psqlscanslash.l` of the pinned release of the build's
//! target version, and every document below is read by both.
//!
//! The corpus is `baselines/psql-oracle/corpus.tsv`, the PostgreSQL 17.9
//! regression SQL files, frozen by Git blob ID exactly as the SQL
//! differential corpus is (`docs/differential-baseline.md`), plus a set of
//! shaped documents that hold the constructs pgtk-psql's own tests cover.
//!
//! The outcome of every document is pinned in
//! `baselines/psql-oracle/<feature>.json`. A document that does not agree
//! with psql is an **expected psql gap** with a recorded reason, and the
//! reasons are the deliberate limits of what pgtk-psql models (ADR 0007, ADR
//! 0013): it models `psqlscan.l` lexing and its query-buffer boundaries, the
//! meta-commands as `psqlscanslash.l` reads them, `\set`, `\unset` and the `\if`
//! blocks, and the COPY data that psql reads itself, but not the error path of
//! an unknown command.
//!
//! Regenerate after a change of pin, of oracle or of the psql grammar:
//!
//! ```sh
//! cargo test -p pgtk-psql --no-default-features --features pg17,psql-oracle \
//!   --test psql_oracle -- --ignored regenerate_psql_oracle_baseline --nocapture
//! ```

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::PathBuf;

use pgtk_psql::{Command as PsqlCommand, MetaCommand, PsqlItem, Terminator, Variables};
use pgtk_psql_oracle::{Command, Event};

// --- The corpus -------------------------------------------------------------

const CORPUS: &str = include_str!("../../pgtk-sql/baselines/psql-oracle/corpus.tsv");
const CORPUS_COMMIT: &str = "6d396980fc5aed4f1a525e0bd75cb16b25ed40ca";

/// The pin table of the oracle: `feature<TAB>ref<TAB>commit`.
const ORACLE_PINS: &str = include_str!("../../pgtk-oracle/pins.tsv");

/// The version feature of this build.
fn target_version() -> &'static str {
    // The features are mutually exclusive (ADR 0009), so exactly one arm
    // applies.
    #[cfg(not(feature = "since-pg15"))]
    return "pg14";
    #[cfg(all(feature = "since-pg15", not(feature = "since-pg16")))]
    return "pg15";
    #[cfg(all(feature = "since-pg16", not(feature = "since-pg17")))]
    return "pg16";
    #[cfg(all(feature = "since-pg17", not(feature = "since-pg18")))]
    return "pg17";
    #[cfg(all(feature = "since-pg18", not(feature = "since-pg19")))]
    return "pg18";
    #[cfg(feature = "since-pg19")]
    return "pg19-beta";
}

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/postgres")
}

fn baseline_path(feature: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pgtk-sql/baselines/psql-oracle")
        .join(format!("{feature}.json"))
}

/// One corpus document: the regression file name and its frozen blob.
struct CorpusFile {
    name: String,
    blob: String,
}

fn corpus() -> Vec<CorpusFile> {
    CORPUS
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let (name, blob) = line
                .split_once('\t')
                .unwrap_or_else(|| panic!("corpus row has two fields: {line:?}"));
            assert!(
                blob.len() == 40 && blob.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "corpus row {name} has no Git blob ID",
            );
            CorpusFile {
                name: name.to_owned(),
                blob: blob.to_owned(),
            }
        })
        .collect()
}

impl CorpusFile {
    /// The frozen source, read by its Git blob ID from the `vendor/postgres`
    /// object database rather than from the checked-out tree.
    fn source(&self) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repository())
            .args(["cat-file", "blob", &self.blob])
            .output()
            .unwrap_or_else(|error| panic!("cannot run git cat-file: {error}"));
        assert!(
            output.status.success(),
            "cannot read the frozen corpus blob {} ({}). \
             Initialise the submodule: git submodule update --init vendor/postgres",
            self.blob,
            self.name,
        );
        String::from_utf8(output.stdout)
            .unwrap_or_else(|_| panic!("corpus blob {} is not UTF-8", self.blob))
    }
}

/// Documents shaped after pgtk-psql's own tests: every construct those tests
/// cover, read here by psql instead of by a written expectation.
///
/// These are held once, in source order by subject, and the count is pinned
/// in the baseline. A construct that only some target versions have is gated
/// like the grammar it exercises.
fn shaped_documents() -> Vec<(&'static str, String)> {
    let mut documents: Vec<(&'static str, String)> =
        vec![
        // Interpolation in every position.
        ("interp/target", "SELECT :v".into()),
        ("interp/literal", "SELECT :'v'".into()),
        ("interp/identifier", "SELECT :\"v\"".into()),
        ("interp/test", "SELECT :{?v}".into()),
        ("interp/argument", "SELECT f(:v)".into()),
        ("interp/typed-literal", "SELECT int :'v'".into()),
        ("interp/copy-target", "COPY t FROM :'f'".into()),
        ("interp/subquery", "SELECT (SELECT :v)".into()),
        ("interp/start-of-input", ":v".into()),
        ("interp/after-punctuation", "(:v)".into()),
        // Not an interpolation: every lexical shell psqlscan.l enters.
        ("shell/string", "SELECT ':v'".into()),
        ("shell/doubled-quote", "SELECT ':''v'".into()),
        ("shell/escape-string", "SELECT E':v'".into()),
        ("shell/escape-backslash", "SELECT E'\\\\:v'".into()),
        ("shell/unicode-string", "SELECT U&':v'".into()),
        ("shell/bit-string", "SELECT B'0:1'".into()),
        ("shell/hex-string", "SELECT X'0:1'".into()),
        ("shell/quoted-identifier", "SELECT \":v\"".into()),
        ("shell/unicode-identifier", "SELECT U&\":v\"".into()),
        ("shell/dollar-quote", "SELECT $$:v$$".into()),
        ("shell/tagged-dollar-quote", "SELECT $tag$:v$tag$".into()),
        ("shell/non-ascii-tag", "SELECT $tagé$:v$tagé$".into()),
        ("shell/line-comment", "SELECT 1 -- :v\nSELECT 2".into()),
        ("shell/block-comment", "SELECT 1 /* :v */".into()),
        (
            "shell/nested-block-comment",
            "SELECT 1 /* /* :v */ */".into(),
        ),
        ("shell/escaped-newline", "SELECT E'a\\\nb' :v".into()),
        // Operators that begin with a colon.
        ("colon/cast", "SELECT a::v".into()),
        ("colon/assignment", "SELECT a :=v".into()),
        ("colon/slice", "SELECT a[:2]".into()),
        ("colon/escaped", "SELECT \\:v".into()),
        ("colon/escaped-literal", "SELECT \\:'v'".into()),
        ("colon/escaped-then-interpolation", "SELECT \\::v".into()),
        // Incomplete interpolation forms, which fall back to a bare colon.
        ("incomplete/literal", "SELECT :'v b'".into()),
        ("incomplete/identifier", "SELECT :\"v b\"".into()),
        ("incomplete/test", "SELECT :{?v b}".into()),
        ("incomplete/brace", "SELECT :{v}".into()),
        ("incomplete/bare-colon", "SELECT : v".into()),
        // Terminators.
        ("terminator/one", "SELECT 1;".into()),
        ("terminator/two", "SELECT 1; SELECT 2;".into()),
        (
            "terminator/batch-separator",
            "SELECT 1 \\; SELECT 2;".into(),
        ),
        ("terminator/in-parentheses", "SELECT (1;".into()),
        ("terminator/in-string", "SELECT 'a;b';".into()),
        (
            "terminator/begin-atomic",
            "CREATE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1; END;".into(),
        ),
        // Where psql ends the query buffer: psqlscan.l's parenthesis and
        // `BEGIN` depths, which psql_scan_reset sets to zero after a send.
        (
            "terminator/begin-atomic-two",
            "CREATE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1; SELECT 2; END;\nSELECT 3;"
                .into(),
        ),
        (
            "terminator/case-in-begin",
            "CREATE PROCEDURE p() BEGIN ATOMIC SELECT CASE WHEN true THEN 1 END; END;\nSELECT 2;"
                .into(),
        ),
        (
            "terminator/or-replace",
            "CREATE OR REPLACE PROCEDURE p() BEGIN ATOMIC SELECT 1; END;\nSELECT 2;".into(),
        ),
        (
            "terminator/begin-outside-routine",
            "BEGIN; SELECT 1; END;".into(),
        ),
        (
            "terminator/national-string-prefix",
            "CREATE n'' FUNCTION f() BEGIN ATOMIC SELECT 1; END;\nSELECT 2;".into(),
        ),
        (
            "terminator/nested-parentheses",
            "SELECT ((1); 2);\nSELECT 3;".into(),
        ),
        (
            "terminator/send-in-parentheses",
            "SELECT (1 \\g\nSELECT 2;".into(),
        ),
        (
            "terminator/reset-in-parentheses",
            "SELECT (1 \\r\nSELECT 2;".into(),
        ),
        (
            "terminator/batch-in-parentheses",
            "SELECT (1 \\; 2);\nSELECT 3;".into(),
        ),
        (
            "terminator/inactive-parenthesis",
            "\\if false\n(\n\\endif\nSELECT 1;".into(),
        ),
        // Send commands.
        ("send/g", "SELECT 1 \\g".into()),
        ("send/gx", "SELECT 1 \\gx".into()),
        ("send/gset", "SELECT 1 \\gset".into()),
        ("send/gset-prefix", "SELECT 1 \\gset pre_".into()),
        ("send/gexec", "SELECT 1 \\gexec".into()),
        ("send/gdesc", "SELECT 1 \\gdesc".into()),
        ("send/crosstabview", "SELECT 1 \\crosstabview".into()),
        ("send/then-more-sql", "SELECT 1 \\gset\nSELECT 2;".into()),
        // A command name is read whole.
        ("name/gsetfoo", "SELECT 1 \\gsetfoo".into()),
        ("name/gexecx", "SELECT 1 \\gexecx".into()),
        ("name/getenv", "\\getenv abs_srcdir".into()),
        // Meta-commands and their arguments.
        ("meta/set", "\\set foo 1".into()),
        ("meta/d", "\\d users".into()),
        ("meta/timing", "\\timing on".into()),
        ("meta/bare-backslash", "SELECT 1 \\".into()),
        (
            "meta/quoted-arguments",
            "\\echo 'a b' \"c d\" 'it''s' 'x\\'y' e".into(),
        ),
        ("meta/separator", "\\pset null x \\\\ SELECT 1;".into()),
        ("meta/chained", "\\x \\a \\t\nSELECT 1;".into()),
        ("meta/after-sql", "SELECT 1 \\echo x\nFROM t;".into()),
        (
            "meta/whole-line",
            "\\copy t to stdout \\\\ SELECT 1;\nSELECT 2;".into(),
        ),
        ("meta/shell", "\\! echo a \\\\ b\nSELECT 1;".into()),
        (
            "meta/file-pipe",
            "\\o |cat \\\\ x\n\\o out \\\\ SELECT 1;".into(),
        ),
        (
            "meta/g-options",
            "SELECT 1 \\g (format=csv) |cat\nSELECT 2;".into(),
        ),
        ("meta/g-file", "SELECT 1 \\g out.txt \\\\ SELECT 2;".into()),
        (
            "meta/gset-prefix",
            "SELECT 1 AS a \\gset p_\nSELECT :p_a;".into(),
        ),
        (
            "meta/argument-interpolation",
            "\\echo :v :'v' :\"v\" :{?v}".into(),
        ),
        ("meta/backquote", "\\echo `echo never` x".into()),
        ("meta/reset", "SELECT 1;\nSELECT 2 \\r\nSELECT 3;".into()),
        ("meta/watch", "SELECT 1 \\watch 1\nSELECT 2;".into()),
        ("meta/unknown", "\\nosuch a \\\\ b\nSELECT 1;".into()),
        ("meta/copy-upper", "\\COPY t TO stdout".into()),
        // `\set` and `\unset` change the variables of the script.
        (
            "set/use",
            "\\set t users\nSELECT * FROM :\"t\" WHERE a = :'t';".into(),
        ),
        ("set/concatenate", "\\set v 4 2\nSELECT :v;".into()),
        ("set/empty", "\\set v\nSELECT :'v';".into()),
        (
            "set/from-variable",
            "\\set a 1\n\\set b :a:a\nSELECT :b;".into(),
        ),
        ("set/unset", "\\set v 1\n\\unset v\nSELECT :v;".into()),
        ("set/quoted", "\\set v 'a b'\nSELECT :'v';".into()),
        // `\if` blocks.
        (
            "if/true",
            "\\if true\nSELECT 1;\n\\else\nSELECT 2;\n\\endif\n".into(),
        ),
        (
            "if/false",
            "\\if off\nSELECT 1;\n\\elif yes\nSELECT 2;\n\\else\nSELECT 3;\n\\endif\n".into(),
        ),
        (
            "if/variable",
            "\\set flag on\n\\if :flag\nSELECT 1;\n\\endif\n".into(),
        ),
        ("if/unrecognised", "\\if maybe\nSELECT 1;\n\\endif\n".into()),
        (
            "if/nested",
            "\\if false\n\\if true\nSELECT 1;\n\\endif\n\\else\nSELECT 2;\n\\endif\n".into(),
        ),
        (
            "if/in-statement",
            "select\n\\if true\n42\n\\else\n(bogus\n\\endif\nforty_two;\n".into(),
        ),
        (
            "if/inactive-set",
            "\\if false\n\\set v 1\n\\else\n\\set v 2\n\\endif\nSELECT :v;".into(),
        ),
        (
            "if/inactive-interpolation",
            "\\set v 1\n\\if false\nSELECT :v;\n\\endif\nSELECT :v;".into(),
        ),
        (
            "if/test-variable",
            "\\set i 1\n\\if :{?i}\nSELECT 1;\n\\endif\n".into(),
        ),
        // Unterminated lexical regions.
        ("open/string", "SELECT 'a".into()),
        ("open/quoted-identifier", "SELECT \"a".into()),
        ("open/dollar-quote", "SELECT $$a".into()),
        ("open/block-comment", "SELECT /* a".into()),
        ("open/parenthesis", "SELECT (1".into()),
        (
            "open/begin",
            "CREATE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1;".into(),
        ),
        // Numbers, whose extent moved in 15.
        ("number/exponent-string", "SELECT 1e'\\' :v".into()),
        ("number/junk", "SELECT 123abc :v".into()),
        ("number/param-junk", "SELECT $1e'\\' :v".into()),
        ("number/comment-after", "SELECT 1e--c\n:v".into()),
        // Leading trivia.
        ("trivia/leading-comment", "-- a\nSELECT :v;".into()),
        ("trivia/leading-space", "   SELECT :v;".into()),
        (
            "trivia/between-statements",
            "SELECT 1;\n-- b\nSELECT 2;".into(),
        ),
        // COPY ... FROM STDIN, whose data psql reads itself.
        (
            "copy/from-stdin",
            "COPY t FROM STDIN;\n1\t:v\n\\.\nSELECT 2;".into(),
        ),
        (
            "copy/rest-of-line",
            "COPY t FROM STDIN; SELECT 1;\n1\n\\.\nSELECT 2;".into(),
        ),
        (
            "copy/two-on-a-line",
            "COPY a FROM STDIN; COPY b FROM STDIN;\n1\n\\.\n2\n\\.\nSELECT 3;".into(),
        ),
        (
            "copy/batch",
            "COPY a FROM stdin \\; COPY b FROM stdin;\n1\n\\.\n2\n\\.\nSELECT 3;".into(),
        ),
        (
            "copy/send-command",
            "COPY t FROM STDIN \\g\n1\n\\.\nSELECT 2;".into(),
        ),
        (
            "copy/send-command-batch",
            "COPY a FROM STDIN \\; SELECT 1 \\g\n1\n\\.\nSELECT 2;".into(),
        ),
        ("copy/gdesc", "COPY t FROM STDIN \\gdesc\nSELECT 2;".into()),
        (
            "copy/stale-words",
            "COPY t FROM STDIN;\n1\n\\.\n;\n2\n\\.\nSELECT 3;".into(),
        ),
        ("copy/binary", "COPY BINARY t FROM STDIN;\n1\n\\.\nSELECT 2;".into()),
        (
            "copy/schema-and-columns",
            "COPY s.t (a, b) FROM STDIN (FORMAT csv);\n1,2\n\\.\nSELECT 2;".into(),
        ),
        ("copy/to-stdout", "COPY t TO STDOUT;\nSELECT 2;".into()),
        ("copy/from-file", "COPY t FROM '/tmp/x';\nSELECT 2;".into()),
        ("copy/query", "COPY (SELECT 1) TO STDOUT;\nSELECT 2;".into()),
        (
            "copy/marker-with-space",
            "COPY t FROM STDIN;\n1\n\\. \n \\.\n\\.\nSELECT 2;".into(),
        ),
        (
            "copy/crlf",
            "COPY t FROM STDIN;\r\n1\r\n\\.\r\nSELECT 2;\r\n".into(),
        ),
        ("copy/missing-marker", "COPY t FROM STDIN;\n1\n2".into()),
        ("copy/marker-at-end", "COPY t FROM STDIN;\n1\n\\.".into()),
        ("copy/empty", "COPY t FROM STDIN;\n\\.\nSELECT 2;".into()),
        ("copy/at-end", "COPY t FROM STDIN;".into()),
        (
            "copy/opaque-data",
            "COPY t FROM STDIN;\nSELECT 'a;\n\\set x 1\n\\if\n\\.\nSELECT :'x';".into(),
        ),
        (
            "copy/inactive-branch",
            "\\if false\nCOPY t FROM STDIN;\n\\endif\nSELECT 1;".into(),
        ),
        (
            "copy/client",
            "\\copy t from stdin\n1\t:v\n\\.\nSELECT 2;".into(),
        ),
        (
            "copy/client-columns",
            "\\COPY t (a, b) FROM STDIN WITH (FORMAT csv)\n1,2\n\\.\nSELECT 2;".into(),
        ),
        ("copy/client-to", "\\copy (select 1) to stdout\nSELECT 2;".into()),
        ("copy/client-pstdin", "\\copy t from pstdin\nSELECT 2;".into()),
        ("copy/client-quoted-file", "\\copy t from 'stdin'\nSELECT 2;".into()),
    ];
    // Added in 17: a vertical tab ends a command name
    // (docs/research/psql-14-19-syntax-changes.md, item A4).
    documents.push(("name/vertical-tab", "SELECT 1 \\g\x0bx".into()));
    // Added in 18: the pipeline and prepared-statement commands, two of
    // which send the buffer text and seven of which do not
    // (docs/research/psql-14-19-syntax-changes.md, class (A) item A1).
    #[cfg(feature = "since-pg18")]
    for (name, command) in [
        ("pipeline/parse", "parse"),
        ("pipeline/sendpipeline", "sendpipeline"),
        ("pipeline/startpipeline", "startpipeline"),
        ("pipeline/syncpipeline", "syncpipeline"),
        ("pipeline/endpipeline", "endpipeline"),
        ("pipeline/flush", "flush"),
        ("pipeline/flushrequest", "flushrequest"),
        ("pipeline/getresults", "getresults"),
        ("pipeline/close-prepared", "close_prepared"),
    ] {
        documents.push((name, format!("SELECT 1; SELECT 2 \\{command}\nSELECT 3;")));
    }
    documents
}

// --- What both sides report -------------------------------------------------

/// The facts a document yields, from either side.
#[derive(Debug, PartialEq, Eq)]
struct Facts {
    /// The text forwarded to the server.
    sql: String,
    /// Each variable interpolation in SQL text: its extent in the document
    /// and its name. An interpolation in a command argument is not here.
    interpolations: Vec<(Range<usize>, String)>,
    /// The extent of each construct that ends the query buffer.
    submissions: Vec<Range<usize>>,
    /// Each backslash command: its whole extent, and the extent of each of
    /// its arguments.
    commands: Vec<(Range<usize>, Vec<Range<usize>>)>,
    /// Each block of COPY data: the extent of the rows and of the `\.` line.
    copy_data: Vec<(Range<usize>, Option<Range<usize>>)>,
    /// Whether the document ends with a complete query buffer.
    complete: bool,
}

/// The extent of `part` inside `source`. Every token pgtk-psql reads is a
/// subslice of the parsed source.
fn span_of(source: &str, part: &str) -> Range<usize> {
    let base = source.as_ptr() as usize;
    let start = part.as_ptr() as usize;
    assert!(
        start >= base && start + part.len() <= base + source.len(),
        "token text is not a subslice of the parsed source",
    );
    (start - base)..(start - base + part.len())
}

/// The extent of a command and of each of its arguments.
fn command_facts(source: &str, command: &PsqlCommand<'_>) -> (Range<usize>, Vec<Range<usize>>) {
    (
        span_of(source, command.text()),
        command
            .arguments()
            .iter()
            .map(|argument| span_of(source, argument.text()))
            .collect(),
    )
}

/// What pgtk-psql makes of `source`. `None` when it refuses the document.
fn pg_psql_facts(source: &str, variables: &Variables) -> Option<Facts> {
    let document = pgtk_psql::parse(source).ok()?;
    let rendered = pgtk_psql::render(source, variables).ok()?;

    let mut interpolations = Vec::new();
    let mut commands = Vec::new();
    let mut copy_data = Vec::new();
    for item in &document.items {
        let command = match item {
            PsqlItem::Interpolation(interpolation) => {
                interpolations.push((
                    span_of(source, interpolation.text()),
                    interpolation.name().to_owned(),
                ));
                continue;
            }
            PsqlItem::Terminator(Terminator::Semi) | PsqlItem::Sql(_) => continue,
            PsqlItem::CopyData(data) => {
                copy_data.push((
                    span_of(source, data.rows.text()),
                    data.end
                        .as_ref()
                        .map(|end| span_of(source, end.marker.text())),
                ));
                continue;
            }
            PsqlItem::Terminator(Terminator::Send(send)) => send.command(),
            PsqlItem::Terminator(Terminator::Reset(reset)) => reset.command(),
            #[cfg(feature = "since-pg18")]
            PsqlItem::Terminator(Terminator::Discard(discard)) => discard.command(),
            PsqlItem::Command(command) => command.command(),
        };
        commands.push(command_facts(source, &command));
    }

    Some(Facts {
        sql: rendered.sql().to_owned(),
        interpolations,
        // A `;` boundary is the `;`, and a command boundary is the backslash
        // and the name, as the oracle reports them.
        submissions: rendered
            .boundaries()
            .iter()
            .map(|boundary| boundary.source.clone())
            .collect(),
        commands,
        copy_data,
        complete: rendered.is_complete(),
    })
}

/// What psql makes of `source`.
///
/// psql resets the query buffer at every submission and sends each one on its
/// own, while pgtk-psql renders one SQL string for the whole document (ADR
/// 0007). The two are made comparable here, on psql's own facts: a send
/// command becomes the `;` that separates the two statements it stands
/// between, a discard command (`\r`, and the commands of 18) removes the
/// buffer it ends, keeping the whitespace before it, and the query text of an
/// inactive `\if` branch goes when psql discards it. That is exactly what
/// `pgtk_psql::substitution::render_document` does.
fn oracle_facts(source: &str, variables: &[(&str, &str)]) -> Facts {
    let scanned = pgtk_psql_oracle::scan(source, variables);
    let raw = scanned.sql();
    let mut sql = String::with_capacity(raw.len());
    // How much of `raw` is in `sql`.
    let mut copied = 0;
    // Pairs of a `raw` offset and the `sql` length at that offset, one for
    // each event, so that a discarded range of `raw` can find its place in
    // `sql`.
    let mut marks: Vec<(usize, usize)> = vec![(0, 0)];
    let mut buffer_start = 0;
    let mut interpolations = Vec::new();
    let mut submissions = Vec::new();
    let mut commands = Vec::new();
    let mut copy_data = Vec::new();
    for event in scanned.events() {
        match event {
            Event::Interpolation { name, source, .. } => {
                if let Some(range) = source {
                    interpolations.push((range.clone(), name.clone()));
                }
            }
            Event::Terminator { source, output } => {
                // A `;` from a rescanned `:name` value has no place in the
                // document, and pgtk-psql does not rescan a value.
                if let Some(source) = source {
                    submissions.push(source.clone());
                }
                sql.push_str(&raw[copied..output.end]);
                copied = output.end;
                buffer_start = sql.len();
            }
            Event::Backslash {
                name,
                command,
                source: extent,
                output,
                options,
            } => {
                commands.push((
                    extent.clone(),
                    options
                        .iter()
                        .map(|option| option.source.clone())
                        .collect::<Vec<_>>(),
                ));
                sql.push_str(&raw[copied..*output]);
                copied = *output;
                match pgtk_psql_oracle::classify(name) {
                    Command::Send => {
                        submissions.push(command.clone());
                        sql.push(';');
                        buffer_start = sql.len();
                    }
                    Command::Discard => {
                        submissions.push(command.clone());
                        let buffer = &sql[buffer_start..];
                        let leading = buffer.len() - buffer.trim_start().len();
                        sql.truncate(buffer_start + leading);
                        buffer_start = sql.len();
                    }
                    Command::Other => {}
                }
            }
            Event::Discard { output } => {
                // psql removes `output` from the buffer. The branch command
                // before this event has copied the raw text to its end, and
                // the last mark at or before its start says where that start
                // is in `sql`: between two events the raw text is copied
                // unchanged. The discarded ranges nest, so an earlier mark
                // is never inside text that a later discard removed.
                sql.push_str(&raw[copied..output.end]);
                copied = output.end;
                let (at, length) = *marks
                    .iter()
                    .rev()
                    .find(|(at, _)| *at <= output.start)
                    .expect("the first mark is the start");
                sql.truncate((length + (output.start - at)).min(sql.len()));
                buffer_start = buffer_start.min(sql.len());
            }
            Event::CopyData { rows, end } => copy_data.push((rows.clone(), end.clone())),
            Event::End { .. } | Event::Incomplete { .. } => {}
        }
        marks.push((copied, sql.len()));
    }
    sql.push_str(&raw[copied..]);

    // An interpolation in a command argument is part of the command.
    interpolations.retain(|(range, _)| {
        !commands
            .iter()
            .any(|(extent, _)| extent.start <= range.start && range.end <= extent.end)
    });

    Facts {
        sql,
        interpolations,
        submissions,
        commands,
        copy_data,
        complete: scanned.is_complete(),
    }
}

// --- One document's outcome -------------------------------------------------

/// Which of the four comparisons agreed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Outcome {
    text: bool,
    interpolations: bool,
    submissions: bool,
    commands: bool,
    copy_data: bool,
    complete: bool,
}

impl Outcome {
    fn agrees(self) -> bool {
        self.text
            && self.interpolations
            && self.submissions
            && self.commands
            && self.copy_data
            && self.complete
    }

    /// The comparisons that disagreed, in a stable order.
    fn reasons(self) -> Vec<&'static str> {
        [
            (!self.text, "text"),
            (!self.interpolations, "interpolations"),
            (!self.submissions, "submissions"),
            (!self.commands, "commands"),
            (!self.copy_data, "copy_data"),
            (!self.complete, "complete"),
        ]
        .into_iter()
        .filter_map(|(failed, name)| failed.then_some(name))
        .collect()
    }
}

/// The result of reading one document with both sides.
#[derive(Debug)]
struct Comparison {
    outcome: Outcome,
    /// Why the document does not agree; `""` when it does.
    class: &'static str,
    /// The first difference, for the failure message.
    detail: String,
}

/// Reads `source` with both sides, unbound and then with every variable psql
/// recognised bound to an inert value.
fn compare(source: &str) -> Comparison {
    let scanned = pgtk_psql_oracle::scan(source, &[]);
    let unbound_oracle = oracle_facts(source, &[]);

    // Bind each variable psql recognised. The values hold no colon, quote,
    // backslash or semicolon, so psql's rescan of a `:name` value echoes it
    // unchanged and the two sides stay comparable even though pgtk-psql
    // substitutes in one pass (ADR 0007).
    let names = unbound_oracle
        .interpolations
        .iter()
        .map(|(_, name)| name.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let bindings = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), format!("Vv{index}")))
        .collect::<Vec<_>>();
    let oracle_bindings = bindings
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    let variables = bindings.iter().cloned().collect::<Variables>();

    let bound_oracle = oracle_facts(source, &oracle_bindings);
    let unbound = pg_psql_facts(source, &Variables::new());
    let bound = pg_psql_facts(source, &variables);

    let Some(unbound_psql) = unbound else {
        // pgtk-psql refused the document.
        let outcome = Outcome {
            text: false,
            interpolations: false,
            submissions: false,
            commands: false,
            copy_data: false,
            complete: !unbound_oracle.complete,
        };
        return Comparison {
            outcome,
            class: classify(source, None, outcome),
            detail: format!(
                "pgtk-psql refuses the document; psql ends with {:?}",
                scanned.final_prompt(),
            ),
        };
    };
    let bound_psql = bound.expect("a document that parses unbound also parses bound");

    let outcome = Outcome {
        text: unbound_psql.sql == unbound_oracle.sql && bound_psql.sql == bound_oracle.sql,
        interpolations: unbound_psql.interpolations == unbound_oracle.interpolations,
        submissions: unbound_psql.submissions == unbound_oracle.submissions,
        commands: unbound_psql.commands == unbound_oracle.commands,
        copy_data: unbound_psql.copy_data == unbound_oracle.copy_data
            && bound_psql.copy_data == bound_oracle.copy_data,
        complete: unbound_psql.complete == unbound_oracle.complete,
    };
    let detail = if outcome.agrees() {
        String::new()
    } else if unbound_psql.sql != unbound_oracle.sql {
        first_difference(&unbound_psql.sql, &unbound_oracle.sql)
    } else if bound_psql.sql != bound_oracle.sql {
        first_difference(&bound_psql.sql, &bound_oracle.sql)
    } else {
        format!(
            "pgtk-psql {:?} / {:?}; psql {:?} / {:?}",
            unbound_psql.interpolations,
            unbound_psql.submissions,
            unbound_oracle.interpolations,
            unbound_oracle.submissions,
        )
    };
    Comparison {
        outcome,
        class: classify(source, Some(&unbound_psql), outcome),
        detail,
    }
}

/// A short, readable account of where two forwarded texts part.
fn first_difference(psql_side: &str, oracle_side: &str) -> String {
    let at = psql_side
        .bytes()
        .zip(oracle_side.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(psql_side.len().min(oracle_side.len()));
    let window = |text: &str| {
        let start = at.saturating_sub(20).min(text.len());
        let end = (at + 30).min(text.len());
        text.get(start..end)
            .unwrap_or("<not a boundary>")
            .to_owned()
    };
    format!(
        "forwarded text differs at byte {at}: pgtk-psql {:?} vs psql {:?}",
        window(psql_side),
        window(oracle_side),
    )
}

/// Why a document does not agree.
///
/// Every class is a deliberate limit of what pgtk-psql models, and a
/// disagreement that fits none of them is `unclassified`, so a new one cannot
/// be recorded without a reason. The order of the rules is the order of
/// causes: a later rule only speaks for a document no earlier one explains.
fn classify(source: &str, psql: Option<&Facts>, outcome: Outcome) -> &'static str {
    if outcome.agrees() {
        return "";
    }
    if psql.is_none() {
        return "refused";
    }
    // psql reports an unknown command as an error and drops the rest of its
    // line. pgtk-psql keeps the name only, as `MetaCommand::Unknown`, and
    // forwards it, so that the server SQL shows the error.
    if has_unknown_command(source) {
        return "unknown-command";
    }
    "unclassified"
}

/// Whether pgtk-psql reads a command name that is not a command of the
/// target version.
fn has_unknown_command(source: &str) -> bool {
    pgtk_psql::parse(source).is_ok_and(|document| {
        document
            .items
            .iter()
            .any(|item| matches!(item, PsqlItem::Command(MetaCommand::Unknown(_))))
    })
}

// --- The pinned baseline ----------------------------------------------------

/// The oracle pin of `feature`, from `pgtk-oracle/pins.tsv`.
fn oracle_pin(feature: &str) -> (String, String) {
    ORACLE_PINS
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .find_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            (fields[0] == feature).then(|| (fields[1].to_owned(), fields[2].to_owned()))
        })
        .unwrap_or_else(|| panic!("pins.tsv has no pin for {feature}"))
}

/// Reads every document with both sides, in a stable order.
fn run_all() -> Vec<(String, Comparison)> {
    let mut results = Vec::new();
    for document in corpus() {
        let comparison = compare(&document.source());
        results.push((document.name.clone(), comparison));
    }
    for (name, source) in shaped_documents() {
        results.push((format!("shaped/{name}"), compare(&source)));
    }
    results
}

fn render_baseline(feature: &str, results: &[(String, Comparison)]) -> String {
    let (reference, commit) = oracle_pin(feature);
    let agree = results
        .iter()
        .filter(|(_, comparison)| comparison.outcome.agrees())
        .count();
    let mut by_class: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, comparison) in results {
        if !comparison.outcome.agrees() {
            *by_class.entry(comparison.class).or_default() += 1;
        }
    }

    let mut out = String::new();
    out.push_str("{\n  \"schema_version\": 1,\n");
    out.push_str(&format!(
        "  \"corpus\": {{ \"name\": \"postgresql-17.9\", \"commit\": \"{CORPUS_COMMIT}\" }},\n"
    ));
    out.push_str(&format!(
        "  \"oracle\": {{ \"feature\": \"{feature}\", \"ref\": \"{reference}\", \
         \"commit\": \"{commit}\" }},\n"
    ));
    out.push_str(&format!(
        "  \"totals\": {{ \"documents\": {}, \"agree\": {}, \"gaps\": {} }},\n",
        results.len(),
        agree,
        results.len() - agree,
    ));
    out.push_str("  \"gap_classes\": {");
    let classes = by_class
        .iter()
        .map(|(class, count)| format!(" \"{class}\": {count}"))
        .collect::<Vec<_>>()
        .join(",");
    out.push_str(&classes);
    out.push_str(if classes.is_empty() { "},\n" } else { " },\n" });
    out.push_str("  \"gaps\": [\n");
    let gaps = results
        .iter()
        .filter(|(_, comparison)| !comparison.outcome.agrees())
        .map(|(name, comparison)| {
            format!(
                "    {{ \"document\": \"{name}\", \"class\": \"{}\", \"reasons\": [{}] }}",
                comparison.class,
                comparison
                    .outcome
                    .reasons()
                    .iter()
                    .map(|reason| format!("\"{reason}\""))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    out.push_str(&gaps);
    out.push_str(if gaps.is_empty() {
        "  ]\n}\n"
    } else {
        "\n  ]\n}\n"
    });
    out
}

// --- The tests --------------------------------------------------------------

#[test]
fn the_oracle_is_the_pin_of_the_target_version() {
    let feature = target_version();
    let (reference, commit) = oracle_pin(feature);
    assert_eq!(pgtk_psql_oracle::POSTGRES_REF, reference);
    assert_eq!(pgtk_psql_oracle::POSTGRES_COMMIT, commit);
}

#[test]
fn the_corpus_is_the_frozen_regression_corpus() {
    let files = corpus();
    assert_eq!(
        files.len(),
        225,
        "the frozen corpus has 226 SQL files, one of which is not UTF-8",
    );
    let names = files.iter().map(|file| &file.name).collect::<Vec<_>>();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "corpus names are unique");
}

#[test]
fn every_document_matches_its_pinned_outcome() {
    let feature = target_version();
    let path = baseline_path(feature);
    let recorded: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display())),
    )
    .expect("the baseline is JSON");

    let (reference, commit) = oracle_pin(feature);
    assert_eq!(recorded["oracle"]["ref"].as_str(), Some(reference.as_str()));
    assert_eq!(recorded["oracle"]["commit"].as_str(), Some(commit.as_str()));
    assert_eq!(recorded["corpus"]["commit"].as_str(), Some(CORPUS_COMMIT));

    let expected = recorded["gaps"]
        .as_array()
        .expect("the baseline lists its gaps")
        .iter()
        .map(|gap| {
            (
                gap["document"].as_str().expect("gap document").to_owned(),
                (
                    gap["class"].as_str().expect("gap class").to_owned(),
                    gap["reasons"]
                        .as_array()
                        .expect("gap reasons")
                        .iter()
                        .map(|reason| reason.as_str().expect("a reason").to_owned())
                        .collect::<Vec<_>>(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let results = run_all();
    assert_eq!(
        results.len() as u64,
        recorded["totals"]["documents"].as_u64().unwrap(),
        "the document count moved; regenerate the baseline",
    );

    let mut unexpected = Vec::new();
    let mut resolved = Vec::new();
    for (name, comparison) in &results {
        let reasons = comparison
            .outcome
            .reasons()
            .iter()
            .map(|reason| (*reason).to_owned())
            .collect::<Vec<_>>();
        match (expected.get(name), comparison.outcome.agrees()) {
            (None, true) => {}
            (None, false) => unexpected.push(format!(
                "{name}: {} ({}) — {}",
                reasons.join("+"),
                comparison.class,
                comparison.detail,
            )),
            (Some(_), true) => resolved.push(name.clone()),
            (Some((class, expected_reasons)), false) => {
                assert_eq!(
                    (&comparison.class.to_owned(), &reasons),
                    (class, expected_reasons),
                    "{name} disagrees for a different reason than the baseline records",
                );
            }
        }
    }
    assert!(
        unexpected.is_empty(),
        "{} documents disagree with psql and are not in the baseline:\n{}",
        unexpected.len(),
        unexpected.join("\n"),
    );
    assert!(
        resolved.is_empty(),
        "{} recorded psql gaps now agree; remove them from the baseline:\n{}",
        resolved.len(),
        resolved.join("\n"),
    );

    // No gap may be recorded without a reason the crate documents.
    for (class, _) in expected.values() {
        assert!(
            ["unknown-command", "refused"].contains(&class.as_str()),
            "unknown psql gap class {class:?}",
        );
    }
}

/// Writes the baseline of the build's target version. Run it after a change
/// of pin, of oracle or of the psql grammar, and review the diff.
#[test]
#[ignore = "writes baselines/psql-oracle/<feature>.json"]
fn regenerate_psql_oracle_baseline() {
    let feature = target_version();
    let results = run_all();
    // `PSQL_ORACLE_DUMP=<directory>` writes both forwarded texts of each gap,
    // unbound, for a diff.
    if let Some(directory) = std::env::var_os("PSQL_ORACLE_DUMP") {
        let directory = PathBuf::from(directory);
        let sources = corpus()
            .into_iter()
            .map(|file| (file.name.clone(), file.source()))
            .chain(
                shaped_documents()
                    .into_iter()
                    .map(|(name, source)| (format!("shaped/{name}"), source)),
            )
            .collect::<BTreeMap<_, _>>();
        for (name, comparison) in &results {
            if comparison.outcome.agrees() {
                continue;
            }
            let source = &sources[name];
            let file = name.replace('/', "_");
            if let Some(facts) = pg_psql_facts(source, &Variables::new()) {
                std::fs::write(directory.join(format!("{file}.pgtk")), facts.sql)
                    .expect("write a dump");
            }
            std::fs::write(
                directory.join(format!("{file}.psql")),
                oracle_facts(source, &[]).sql,
            )
            .expect("write a dump");
        }
    }
    for (name, comparison) in &results {
        if !comparison.outcome.agrees() {
            println!(
                "gap {name}: {} ({}) — {}",
                comparison.outcome.reasons().join("+"),
                comparison.class,
                comparison.detail,
            );
        }
    }
    let path = baseline_path(feature);
    std::fs::create_dir_all(path.parent().expect("the baseline has a directory"))
        .expect("create the baseline directory");
    std::fs::write(&path, render_baseline(feature, &results)).expect("write the baseline");
    println!("wrote {}", path.display());
}
