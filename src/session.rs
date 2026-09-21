//! The Session: what a PostgreSQL session holds that a Parse or a Describe
//! reads, and what a Statement would add to it or remove from it.

use crate::analysis::ResultDescriptor;
use crate::types::TypeRef;

/// One prepared statement of a Session, as PREPARE made it.
#[derive(Clone, Debug)]
pub struct PreparedStatement {
    /// The name, folded as the parser folds an identifier.
    pub name: String,
    /// The type of every parameter in `$n` order: the declared types
    /// first, then the inferred ones.
    pub param_types: Vec<TypeRef>,
    /// What a Describe of an EXECUTE of this statement answered when the
    /// PREPARE made it.
    pub result: ResultDescriptor,
    /// The text of the PREPARE statement that made it.
    ///
    /// PostgreSQL's plan cache keeps the query's raw parse tree and
    /// analyses it again whenever it revalidates, which a Describe of an
    /// EXECUTE does. Analysis therefore answers an EXECUTE from this text
    /// and the Catalog as it is now, and reports `0A000` when the result
    /// columns are no longer the ones in `result`. A prepared statement
    /// the caller built with an empty text answers from `result`.
    pub source: String,
}

/// One open cursor of a Session, as DECLARE made it.
#[derive(Clone, Debug)]
pub struct Cursor {
    /// The name, folded as the parser folds an identifier.
    pub name: String,
    /// What a Describe of a FETCH from this cursor answers.
    pub result: ResultDescriptor,
    /// The cursor was declared SCROLL.
    pub scroll: bool,
    /// The cursor was declared WITH HOLD, so the end of a transaction
    /// does not close it.
    pub hold: bool,
}

/// One custom configuration placeholder of a Session, such as
/// `custom.my_guc`, as SET made it. A placeholder is a parameter whose
/// name has a dot and which no extension defined.
#[derive(Clone, Debug)]
pub struct Placeholder {
    /// The name, folded as the parser folds it, dot included.
    pub name: String,
    /// The value, as the SET statement wrote it.
    pub value: String,
}

/// The objects a PostgreSQL session holds that a Parse or a Describe reads.
///
/// A Session is a snapshot the caller supplies next to the [`Settings`], and
/// an empty Session is the default. Analysis reads it and never changes it.
/// A caller that replays statements in order folds each
/// [`SessionEffect`](SessionEffect) into its own Session with
/// [`Session::apply`].
///
/// [`Settings`]: crate::Settings
#[derive(Clone, Debug, Default)]
pub struct Session {
    prepared: Vec<PreparedStatement>,
    cursors: Vec<Cursor>,
    placeholders: Vec<Placeholder>,
}

impl Session {
    /// An empty Session: no prepared statement, no cursor and no
    /// placeholder.
    pub fn new() -> Self {
        Self::default()
    }

    /// The prepared statement of that name. The name must be folded, as
    /// the name in a Statement is.
    pub fn prepared(&self, name: &str) -> Option<&PreparedStatement> {
        self.prepared.iter().find(|p| p.name == name)
    }

    /// The cursor of that name. The name must be folded, as the name in a
    /// Statement is.
    pub fn cursor(&self, name: &str) -> Option<&Cursor> {
        self.cursors.iter().find(|c| c.name == name)
    }

    /// The placeholder of that name. A parameter name is matched without
    /// case, as PostgreSQL matches one.
    pub fn placeholder(&self, name: &str) -> Option<&Placeholder> {
        self.placeholders
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// Every prepared statement, in the order they were added.
    pub fn prepared_statements(&self) -> impl ExactSizeIterator<Item = &PreparedStatement> {
        self.prepared.iter()
    }

    /// Every open cursor, in the order they were added.
    pub fn cursors(&self) -> impl ExactSizeIterator<Item = &Cursor> {
        self.cursors.iter()
    }

    /// Every placeholder, in the order they were added.
    pub fn placeholders(&self) -> impl ExactSizeIterator<Item = &Placeholder> {
        self.placeholders.iter()
    }

    /// Add a prepared statement. An entry of the same name is replaced.
    pub fn add_prepared(&mut self, prepared: PreparedStatement) {
        match self.prepared.iter_mut().find(|p| p.name == prepared.name) {
            Some(slot) => *slot = prepared,
            None => self.prepared.push(prepared),
        }
    }

    /// Add a cursor. An entry of the same name is replaced.
    pub fn add_cursor(&mut self, cursor: Cursor) {
        match self.cursors.iter_mut().find(|c| c.name == cursor.name) {
            Some(slot) => *slot = cursor,
            None => self.cursors.push(cursor),
        }
    }

    /// Add a placeholder. An entry of the same name is replaced.
    pub fn add_placeholder(&mut self, placeholder: Placeholder) {
        match self
            .placeholders
            .iter_mut()
            .find(|p| p.name.eq_ignore_ascii_case(&placeholder.name))
        {
            Some(slot) => *slot = placeholder,
            None => self.placeholders.push(placeholder),
        }
    }

    /// Fold one Session Effect in, as running the Statement would.
    pub fn apply(&mut self, effect: &SessionEffect) {
        match effect {
            SessionEffect::Prepare(prepared) => self.add_prepared(prepared.clone()),
            SessionEffect::Deallocate(name) => self.prepared.retain(|p| &p.name != name),
            SessionEffect::DeallocateAll => self.prepared.clear(),
            SessionEffect::Declare(cursor) => self.add_cursor(cursor.clone()),
            SessionEffect::Close(name) => self.cursors.retain(|c| &c.name != name),
            SessionEffect::CloseAll => self.cursors.clear(),
            SessionEffect::Set(placeholder) => self.add_placeholder(placeholder.clone()),
            SessionEffect::TransactionEnd => self.end_transaction(),
        }
    }

    /// End the transaction: every cursor that is not held closes, as
    /// PostgreSQL closes it at COMMIT or ROLLBACK.
    pub fn end_transaction(&mut self) {
        self.cursors.retain(|c| c.hold);
    }
}

/// What a Statement would add to the Session or remove from it when it
/// runs.
///
/// Analysis reports the Effect in
/// [`Analysis::session_effect`](crate::Analysis::session_effect) and never
/// applies it. A caller that replays statements in order applies it with
/// [`Session::apply`].
#[derive(Clone, Debug)]
pub enum SessionEffect {
    /// PREPARE adds a prepared statement.
    Prepare(PreparedStatement),
    /// DEALLOCATE removes one prepared statement.
    Deallocate(String),
    /// DEALLOCATE ALL removes every prepared statement.
    DeallocateAll,
    /// DECLARE opens a cursor.
    Declare(Cursor),
    /// CLOSE closes one cursor.
    Close(String),
    /// CLOSE ALL closes every cursor.
    CloseAll,
    /// SET of a custom configuration parameter adds a placeholder.
    Set(Placeholder),
    /// The end of a transaction closes every cursor that is not held.
    /// COMMIT, ROLLBACK, END and ABORT end one, and so do COMMIT PREPARED
    /// and ROLLBACK PREPARED.
    TransactionEnd,
}

/// The empty Session, for an Analysis that has none of its own: the query
/// of a view definition, for example.
pub(crate) static EMPTY_SESSION: Session = Session {
    prepared: Vec::new(),
    cursors: Vec::new(),
    placeholders: Vec::new(),
};
