#![doc = include_str!("../README.md")]
//!
//! # Details
//!
//! An oracle for psql's own client lexer.
//!
//! `pgtk-oracle` links PostgreSQL's raw parser so that pgtk-sql can be compared
//! with the server (ADR 0002). This crate does the same one layer up: it
//! links `src/fe_utils/psqlscan.l` and `src/bin/psql/psqlscanslash.l` of the
//! same pinned PostgreSQL release, so that pgtk-psql can be compared with psql.
//!
//! The version feature (`pg14` ... `pg19-beta`, default `pg17`) selects the
//! release from `pgtk-oracle/pins.tsv`, which is the one pin table of the
//! repository (ADR 0009).
//!
//! # What it reports
//!
//! [`scan`] gives, for one psql source document:
//!
//! - [`Scan::sql`], the text psql forwards to the server;
//! - one [`Event`] for each variable interpolation the lexer recognised, with
//!   its extent in the document, the quoting psql asked for, and where the
//!   value landed in the forwarded text;
//! - one event for each `;` that submits the query buffer;
//! - one event for each backslash command, with the command name exactly as
//!   `psqlscanslash.l` reads it, and the extent and value of each argument
//!   that `psql_scan_slash_option` reads for it;
//! - one event each time psql discards the query text of an inactive `\if`
//!   branch; and
//! - a final event saying whether the document ended with a complete query
//!   buffer, and if not, which lexical state was still open.
//!
//! Those are the facts pgtk-psql models (`docs/adr/0007-give-psql-its-own-grammar-and-crate.md`).
//!
//! The shim reads the arguments of each command as `command.c` reads them. It
//! applies `\set`, `\unset`, `\if`, `\elif`, `\else` and `\endif`, and it
//! runs no other command and no backquoted shell command.
//!
//! # Two adaptations of psql's main loop
//!
//! psql reads one line at a time and resets the query buffer after every
//! submission. pgtk-psql renders one buffer for a whole document. The shim
//! therefore scans the SQL text of the whole document as one input and never
//! resets the buffer, so that the two texts are comparable. `csrc/psql_oracle.c`
//! documents both, and `docs/psql-oracle.md` records what each one costs.

#[cfg(feature = "scanner")]
use std::ffi::CStr;
use std::ops::Range;
#[cfg(feature = "scanner")]
use std::os::raw::{c_char, c_int};
#[cfg(feature = "scanner")]
use std::sync::Mutex;

/// The Git ref of the PostgreSQL release that this oracle is built from.
pub const POSTGRES_REF: &str = env!("PG_PSQL_ORACLE_POSTGRES_REF");

/// The Git commit of the PostgreSQL release that this oracle is built from.
pub const POSTGRES_COMMIT: &str = env!("PG_PSQL_ORACLE_POSTGRES_COMMIT");

#[cfg(feature = "scanner")]
#[repr(C)]
struct RawEvent {
    kind: c_int,
    source_start: c_int,
    source_end: c_int,
    output_start: c_int,
    output_end: c_int,
    detail: c_int,
    bound: c_int,
    text: *mut c_char,
}

#[cfg(feature = "scanner")]
#[repr(C)]
struct RawScan {
    events: *mut RawEvent,
    nevents: c_int,
    capacity: c_int,
    sql: *mut c_char,
    sql_len: c_int,
    copy_from_stdin: c_int,
}

#[cfg(feature = "scanner")]
unsafe extern "C" {
    fn pgpso_scan(
        source: *const c_char,
        source_len: c_int,
        names: *const *const c_char,
        values: *const *const c_char,
        nvars: c_int,
    ) -> *mut RawScan;
    fn pgpso_free(scan: *mut RawScan);
}

#[cfg(feature = "scanner")]
/// psql's lexer keeps state in file-scope flex tables; serialise every call.
static LOCK: Mutex<()> = Mutex::new(());

/// How psql asked for a variable's value. These are `PsqlScanQuoteType` in
/// `src/include/fe_utils/psqlscan.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quote {
    /// `:name` and `:{?name}`.
    Plain,
    /// `:'name'`.
    Literal,
    /// `:"name"`.
    Identifier,
    /// A variable inside a backquoted argument. This oracle never runs a
    /// backquoted command.
    ShellArgument,
}

/// Why psql stopped scanning at the end of the document. These are
/// `promptStatus_t` in `src/include/fe_utils/psqlscan.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prompt {
    Ready,
    Continue,
    Comment,
    SingleQuote,
    DoubleQuote,
    DollarQuote,
    Paren,
    Copy,
}

impl Prompt {
    #[cfg(feature = "scanner")]
    fn from_raw(value: c_int) -> Self {
        match value {
            0 => Self::Ready,
            1 => Self::Continue,
            2 => Self::Comment,
            3 => Self::SingleQuote,
            4 => Self::DoubleQuote,
            5 => Self::DollarQuote,
            6 => Self::Paren,
            7 => Self::Copy,
            other => panic!("unknown promptStatus_t {other}"),
        }
    }

    /// Whether the document ended inside a string, a comment, a dollar-quoted
    /// body or a quoted identifier.
    ///
    /// psql tolerates this and asks for another line. pgtk-psql has no notion
    /// of an open query buffer at all (ADR 0007, freshtonic/recursa#131): a
    /// comment or a dollar-quoted body now runs its closed matcher to end of
    /// input instead, and a string or a quoted identifier still refuses the
    /// document. Either way this is the oracle's side of that deliberate
    /// divergence.
    pub fn is_open_lexical_region(self) -> bool {
        matches!(
            self,
            Self::Comment | Self::SingleQuote | Self::DoubleQuote | Self::DollarQuote
        )
    }
}

/// What psql does with the query buffer when a backslash command ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// The command returns `PSQL_CMD_SEND` and its send mode puts the buffer
    /// text on the wire. `MainLoop()` then resets the buffer.
    Send,
    /// The buffer ends, but its text does not go on the wire: `\r`, and the
    /// commands of PostgreSQL 18 that return `PSQL_CMD_SEND` with a send mode
    /// that does not send the text.
    Discard,
    /// Every other command. It does not end the query buffer, so the text
    /// before and after it belongs to one statement.
    Other,
}

/// What psql does with the query buffer after the backslash command `name`.
///
/// The table is the set of commands whose `exec_command_*` returns
/// `PSQL_CMD_SEND` in `src/bin/psql/command.c` at the pinned release, split by
/// whether the send mode transmits the buffer text
/// (`docs/research/psql-14-19-syntax-changes.md`, class (A) item A1).
///
/// Two more commands end the buffer, although they do not return
/// `PSQL_CMD_SEND`: `\watch` runs the buffer and resets it (`command.c`
/// 2993), so it is [`Command::Send`], and `\r` (`\reset`) clears it
/// (`command.c` 2352), so it is [`Command::Discard`].
pub fn classify(name: &str) -> Command {
    // `exec_command_g` serves both `\g` and `\gx`; `\gdesc` sends the buffer
    // with a Describe, so it transmits the text like the others.
    // `REL_17_11:command.c` 761, 1474, 1570, 1624, 1651.
    const SEND: &[&str] = &["crosstabview", "g", "gx", "gdesc", "gexec", "gset", "watch"];
    const DISCARD: &[&str] = &["r", "reset"];
    // Added in 18: `REL_18_6:command.c` 2529 (`\parse`) and 2855
    // (`\sendpipeline`) send the buffer text.
    #[cfg(feature = "since-pg18")]
    const SEND_18: &[&str] = &["parse", "sendpipeline"];
    // Added in 18: these end the buffer but send none of its text
    // (`REL_18_6:command.c` 775, 1702, 1721, 1939, 3072, 3091, 3110).
    #[cfg(feature = "since-pg18")]
    const DISCARD_18: &[&str] = &[
        "close_prepared",
        "endpipeline",
        "flush",
        "flushrequest",
        "getresults",
        "startpipeline",
        "syncpipeline",
    ];

    if SEND.contains(&name) {
        return Command::Send;
    }
    if DISCARD.contains(&name) {
        return Command::Discard;
    }
    #[cfg(feature = "since-pg18")]
    {
        if SEND_18.contains(&name) {
            return Command::Send;
        }
        if DISCARD_18.contains(&name) {
            return Command::Discard;
        }
    }
    Command::Other
}

/// One fact psql's lexer reported about a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A variable interpolation the lexer recognised.
    Interpolation {
        /// The variable name, without the interpolation's punctuation.
        name: String,
        /// The interpolation's extent in the document. `None` when psql was
        /// rescanning a substituted value, which has no place in the source.
        source: Option<Range<usize>>,
        /// The quoting psql asked for.
        quote: Quote,
        /// Whether the variable was bound. An unbound one is forwarded as it
        /// stands, except `:{?name}`, which answers `FALSE`.
        bound: bool,
        /// Where the substituted value starts in the forwarded text.
        output_start: usize,
        /// Where it ends, when psql emits it in place. `None` for `:name`,
        /// whose value psql pushes back into the lexer and rescans.
        output_end: Option<usize>,
    },
    /// A `;` that submits the query buffer.
    Terminator {
        /// The semicolon's extent in the document. `None` when the
        /// semicolon came from a substituted `:name` value, which psql
        /// rescans.
        source: Option<Range<usize>>,
        /// Its extent in the forwarded text. psql forwards the `;`.
        output: Range<usize>,
    },
    /// A backslash command.
    Backslash {
        /// The command name, without the backslash, exactly as
        /// `psqlscanslash.l` reads it.
        name: String,
        /// The whole command: the backslash, the name, the arguments that
        /// psql reads for it, and a trailing `\\`.
        source: Range<usize>,
        /// The backslash and the name alone.
        command: Range<usize>,
        /// Where it sits in the forwarded text. psql forwards none of it, so
        /// the range is empty.
        output: usize,
        /// Each argument that `psql_scan_slash_option` read, in order.
        options: Vec<SlashOption>,
    },
    /// psql discarded the query text of an inactive `\if` branch
    /// (`discard_query_text` in `command.c`). The oracle keeps that text in
    /// [`Scan::sql`], so that the offsets of the other events stay valid;
    /// `output` is the range of it that psql removes.
    Discard {
        /// The range of the forwarded text that psql removes.
        output: Range<usize>,
    },
    /// The end of the document, with a complete query buffer.
    End { prompt: Prompt },
    /// The end of the document, with an incomplete query buffer.
    Incomplete { prompt: Prompt },
}

/// One argument of a backslash command, as `psql_scan_slash_option` read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashOption {
    /// The argument's extent in the document, quotes included.
    pub source: Range<usize>,
    /// The value psql read, or `None` when psql refused the argument (an
    /// unterminated quote).
    pub value: Option<String>,
}

/// What psql's lexer made of one document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scan {
    sql: String,
    events: Vec<Event>,
    copy_from_stdin: usize,
}

impl Scan {
    /// The text psql forwards to the server, for the whole document.
    ///
    /// A backslash command contributes nothing: psql sends the query buffer
    /// and the command never reaches the server. The text of an inactive
    /// `\if` branch is here; [`Event::Discard`] says where it is. Use [`Scan::events`] to see
    /// where the boundaries are.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Every fact the lexer reported, in document order.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// How many `COPY ... FROM STDIN` commands psqlscan.l counted. psql reads
    /// the data lines that follow one of these itself; this oracle and
    /// pgtk-psql both lex them as ordinary text.
    pub fn copy_from_stdin(&self) -> usize {
        self.copy_from_stdin
    }

    /// Whether the document ended with a complete query buffer.
    pub fn is_complete(&self) -> bool {
        matches!(self.events.last(), Some(Event::End { .. }))
    }

    /// The prompt psql would show after the document.
    pub fn final_prompt(&self) -> Prompt {
        match self.events.last() {
            Some(Event::End { prompt } | Event::Incomplete { prompt }) => *prompt,
            _ => panic!("a scan always ends with an End or Incomplete event"),
        }
    }

    /// The backslash command names, in order.
    pub fn backslash_commands(&self) -> impl Iterator<Item = &str> {
        self.events.iter().filter_map(|event| match event {
            Event::Backslash { name, .. } => Some(name.as_str()),
            _ => None,
        })
    }
}

#[cfg(feature = "scanner")]
/// Scans `source` as psql would, with `variables` bound.
///
/// `variables` is a list of `(name, value)` pairs. A name that is not in the
/// list is unbound, and psql forwards its interpolation unchanged.
///
/// # Panics
///
/// If `source` holds a NUL byte, or is longer than `i32::MAX`.
pub fn scan(source: &str, variables: &[(&str, &str)]) -> Scan {
    assert!(
        !source.as_bytes().contains(&0),
        "a psql document must not hold a NUL byte"
    );
    let length = c_int::try_from(source.len()).expect("the document fits in an int");

    let names = variables
        .iter()
        .map(|(name, _)| std::ffi::CString::new(*name).expect("NUL in a variable name"))
        .collect::<Vec<_>>();
    let values = variables
        .iter()
        .map(|(_, value)| std::ffi::CString::new(*value).expect("NUL in a variable value"))
        .collect::<Vec<_>>();
    let name_pointers = names.iter().map(|name| name.as_ptr()).collect::<Vec<_>>();
    let value_pointers = values
        .iter()
        .map(|value| value.as_ptr())
        .collect::<Vec<_>>();

    let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    // SAFETY: the pointers live for the call, and the lengths agree with them.
    unsafe {
        let raw = pgpso_scan(
            source.as_ptr() as *const c_char,
            length,
            name_pointers.as_ptr(),
            value_pointers.as_ptr(),
            c_int::try_from(variables.len()).expect("the variable count fits in an int"),
        );
        assert!(!raw.is_null(), "pgpso_scan returned no result");
        let scan = read_scan(&*raw, source.len());
        pgpso_free(raw);
        scan
    }
}

#[cfg(feature = "scanner")]
/// Reads the C result. `source_len` bounds every source offset, so a bad
/// offset stops the test rather than producing a silent mismatch.
unsafe fn read_scan(raw: &RawScan, source_len: usize) -> Scan {
    let sql = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(
            raw.sql as *const u8,
            raw.sql_len as usize,
        ))
    }
    .expect("psql forwards the document's own bytes, so the text is UTF-8")
    .to_owned();

    let raw_events =
        unsafe { std::slice::from_raw_parts(raw.events as *const RawEvent, raw.nevents as usize) };
    let mut events: Vec<Event> = Vec::with_capacity(raw_events.len());
    for raw in raw_events {
        if raw.kind == 6 {
            let option = read_option(raw, source_len);
            // Interpolation and discard events of the arguments come
            // between a command and its options.
            match events
                .iter_mut()
                .rev()
                .find(|event| matches!(event, Event::Backslash { .. }))
            {
                Some(Event::Backslash { options, .. }) => options.push(option),
                _ => panic!("an option event follows its backslash command"),
            }
            continue;
        }
        events.push(read_event(raw, source_len));
    }

    Scan {
        sql,
        events,
        copy_from_stdin: raw.copy_from_stdin as usize,
    }
}

#[cfg(feature = "scanner")]
fn read_option(event: &RawEvent, source_len: usize) -> SlashOption {
    let (start, end) = (event.source_start as usize, event.source_end as usize);
    assert!(
        start <= end && end <= source_len,
        "the oracle reported the option range {start}..{end} \
         outside a document of {source_len} bytes",
    );
    let value = (!event.text.is_null()).then(|| {
        unsafe { CStr::from_ptr(event.text) }
            .to_string_lossy()
            .into_owned()
    });
    SlashOption {
        source: start..end,
        value,
    }
}

#[cfg(feature = "scanner")]
fn read_event(event: &RawEvent, source_len: usize) -> Event {
    let text = || {
        assert!(!event.text.is_null(), "this event kind carries text");
        unsafe { CStr::from_ptr(event.text) }
            .to_str()
            .expect("psql reads the document's own bytes, so the text is UTF-8")
            .to_owned()
    };
    let source = |start: c_int, end: c_int| {
        let (start, end) = (start as usize, end as usize);
        assert!(
            start <= end && end <= source_len,
            "the oracle reported the source range {start}..{end} \
             outside a document of {source_len} bytes",
        );
        start..end
    };

    match event.kind {
        1 => Event::Interpolation {
            name: text(),
            source: (event.source_start >= 0).then(|| source(event.source_start, event.source_end)),
            quote: match event.detail {
                0 => Quote::Plain,
                1 => Quote::Literal,
                2 => Quote::Identifier,
                3 => Quote::ShellArgument,
                other => panic!("unknown PsqlScanQuoteType {other}"),
            },
            bound: event.bound != 0,
            output_start: event.output_start as usize,
            output_end: (event.output_end >= 0).then_some(event.output_end as usize),
        },
        2 => Event::Terminator {
            source: (event.source_start >= 0).then(|| source(event.source_start, event.source_end)),
            output: (event.output_start as usize)..(event.output_end as usize),
        },
        3 => {
            let name = text();
            let whole = source(event.source_start, event.source_end);
            let command = whole.start..(whole.start + event.detail as usize);
            Event::Backslash {
                name,
                source: whole,
                command,
                output: event.output_start as usize,
                options: Vec::new(),
            }
        }
        7 => Event::Discard {
            output: (event.output_start as usize)..(event.output_end as usize),
        },
        4 => Event::End {
            prompt: Prompt::from_raw(event.detail),
        },
        5 => Event::Incomplete {
            prompt: Prompt::from_raw(event.detail),
        },
        other => panic!("unknown pgpso event kind {other}"),
    }
}
