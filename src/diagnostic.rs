//! What Analysis reports when it cannot analyse a Statement.

use std::fmt;
use std::ops::Range;

/// The location of a node that has none, as PostgreSQL's `-1`: a
/// diagnostic placed there reports no position.
pub(crate) const NO_LOCATION: usize = usize::MAX;

/// The severity PostgreSQL would use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// An error: Analysis stopped here, as PostgreSQL stops.
    Error,
}

/// A secondary span with a note, for rendering. PostgreSQL has no equivalent,
/// so labels are never compared with an Oracle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// Byte range in the statement text.
    pub span: Range<usize>,
    /// What to say about it.
    pub text: String,
}

/// The error PostgreSQL would raise for a Statement, with the same SQLSTATE,
/// message and cursor position, plus spans that PostgreSQL does not have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// The five-character SQLSTATE, such as `42703`.
    pub sqlstate: &'static str,
    /// PostgreSQL's primary message, byte for byte.
    pub message: String,
    /// PostgreSQL's cursor position: the 1-based character index into the
    /// statement text, when PostgreSQL reports one.
    pub position: Option<u32>,
    /// PostgreSQL's DETAIL line, when it has one.
    pub detail: Option<String>,
    /// PostgreSQL's HINT line, when it has one.
    pub hint: Option<String>,
    /// The severity.
    pub severity: Severity,
    /// The primary span, as a byte range in the statement text, when known.
    pub span: Option<Range<usize>>,
    /// Secondary spans with notes.
    pub labels: Vec<Label>,
    /// The byte offset PostgreSQL would report as the cursor, before it is
    /// turned into `position` against the statement text.
    pub(crate) location: Option<usize>,
}

impl Diagnostic {
    /// A new error with a message and no position yet.
    pub(crate) fn new(sqlstate: &'static str, message: impl Into<String>) -> Self {
        Self {
            sqlstate,
            message: message.into(),
            position: None,
            detail: None,
            hint: None,
            severity: Severity::Error,
            span: None,
            labels: Vec::new(),
            location: None,
        }
    }

    /// Set the cursor to a byte offset in the statement text.
    pub(crate) fn at(mut self, location: usize) -> Self {
        if location != NO_LOCATION {
            self.location = Some(location);
        }
        self
    }

    /// Turn the byte location into PostgreSQL's 1-based character position
    /// and a span, against the statement text.
    pub(crate) fn resolve(&mut self, text: &str) {
        if let Some(location) = self.location
            && self.position.is_none()
        {
            let chars = text
                .get(..location)
                .map_or(0, |prefix| prefix.chars().count());
            self.position = Some(chars as u32 + 1);
            if self.span.is_none() {
                self.span = Some(location..location);
            }
        }
    }

    pub(crate) fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub(crate) fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.sqlstate, self.message)?;
        if let Some(position) = self.position {
            write!(f, " (at character {position})")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}

/// The outcome when Analysis meets something PostgreSQL would accept but this
/// crate cannot yet analyse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported {
    /// What was met, in a few words.
    pub what: String,
    /// Where, as a byte range in the statement text, when known.
    pub span: Option<Range<usize>>,
}

impl fmt::Display for Unsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported: {}", self.what)
    }
}

impl std::error::Error for Unsupported {}

/// Why an Analysis did not produce a result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalyzeError {
    /// PostgreSQL would reject the Statement with this Diagnostic.
    Diagnostic(Box<Diagnostic>),
    /// This crate cannot analyse the Statement yet.
    Unsupported(Box<Unsupported>),
}

impl fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Diagnostic(d) => d.fmt(f),
            Self::Unsupported(u) => u.fmt(f),
        }
    }
}

impl std::error::Error for AnalyzeError {}

impl From<Diagnostic> for AnalyzeError {
    fn from(d: Diagnostic) -> Self {
        Self::Diagnostic(Box::new(d))
    }
}

impl From<Unsupported> for AnalyzeError {
    fn from(u: Unsupported) -> Self {
        Self::Unsupported(Box::new(u))
    }
}

/// Shorthand for the crate's fallible functions.
pub(crate) type Result<T> = std::result::Result<T, AnalyzeError>;

/// Build an Unsupported outcome.
pub(crate) fn unsupported(what: impl Into<String>, span: Option<Range<usize>>) -> AnalyzeError {
    AnalyzeError::Unsupported(Box::new(Unsupported {
        what: what.into(),
        span,
    }))
}
