//! Rewrite rules, and the setting that says when a rule or a trigger fires.

use crate::name::Name;

/// A rewrite rule of a relation: one `pg_rewrite` row.
///
/// The model holds what PostgreSQL's rewriter needs to decide if a statement
/// on a view is permitted. It holds no action and no condition of the rule.
///
/// The `_RETURN` rule of a view is not here. It is the only kind of
/// `ON SELECT` rule that the server permits, and
/// [`Relation::view_definition`](crate::Relation::view_definition) holds what
/// it says.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rule {
    /// The name of the rule. The rewriter applies the rules of a relation in
    /// name order.
    pub name: Name,
    /// The statement kind that the rule applies to.
    pub event: RuleEvent,
    /// True for a `DO INSTEAD` rule. False for a `DO ALSO` rule.
    pub instead: bool,
    /// True when the rule has no `WHERE` condition.
    ///
    /// The server stores the condition in `ev_qual`, which is never NULL. An
    /// empty condition is the node tree `<>`.
    pub unconditional: bool,
    /// When the rule fires, from `ev_enabled`.
    pub fires: FiresWhen,
}

impl Rule {
    /// True for a rule that replaces each statement of its kind: an
    /// unconditional `DO INSTEAD` rule that fires in an ordinary session.
    ///
    /// PostgreSQL permits a statement on a view that is not updatable when
    /// the view has such a rule for the statement kind.
    pub fn replaces(&self, event: RuleEvent) -> bool {
        self.event == event && self.instead && self.unconditional && self.fires.in_origin()
    }
}

/// The statement kind of a rule, from `ev_type`.
///
/// There is no `Select` kind. The `_RETURN` rule of a view is the only
/// `ON SELECT` rule that the server permits, and the model does not hold it
/// as a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RuleEvent {
    /// `ON UPDATE`. `ev_type` is `2`.
    Update,
    /// `ON INSERT`. `ev_type` is `3`.
    Insert,
    /// `ON DELETE`. `ev_type` is `4`.
    Delete,
}

/// When a rule or a trigger fires, from `ev_enabled` or `tgenabled`.
///
/// The setting `session_replication_role` selects the mode of a session. An
/// ordinary session is in the `origin` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FiresWhen {
    /// In the `origin` mode and the `local` mode. This is the default. The
    /// letter is `O`.
    Origin,
    /// In the `replica` mode only. The letter is `R`.
    Replica,
    /// In each mode. The letter is `A`.
    Always,
    /// Never. The letter is `D`.
    Disabled,
}

impl FiresWhen {
    /// True when the rule or the trigger fires in an ordinary session.
    pub fn in_origin(self) -> bool {
        matches!(self, FiresWhen::Origin | FiresWhen::Always)
    }
}
