//! The conditional structure of a psql document.
//!
//! `\if`, `\elif`, `\else` and `\endif` make blocks, and blocks can nest.
//! [`PsqlDocument::conditionals`] finds the blocks and checks the nesting.
//! It does not evaluate an expression: that is a step of
//! [`crate::render`], which applies the variables. The items stay in their
//! source order; a [`ConditionalBlock`] refers to them by index.

use std::fmt;
use std::ops::Range;

use crate::ast::{MetaCommand, PsqlDocument, PsqlItem};

/// One `\if` ... `\endif` block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionalBlock {
    /// The item index of the `\if`, then of each `\elif`, then of the
    /// `\else`, if there is one. Branch `n` holds the items from
    /// `branches[n] + 1` up to the next branch command, or up to `end`.
    pub branches: Vec<usize>,
    /// The item index of the `\endif`.
    pub end: usize,
    /// The number of blocks that hold this block. A top-level block has
    /// depth 0.
    pub depth: usize,
}

impl ConditionalBlock {
    /// Whether the block has an `\else` branch.
    pub fn has_else(&self, document: &PsqlDocument<'_>) -> bool {
        self.branches.last().is_some_and(|index| {
            matches!(
                document.items[*index],
                PsqlItem::Command(MetaCommand::Else(_))
            )
        })
    }

    /// The item indices of branch `branch`: the items after its command, up
    /// to the next branch command or to the `\endif`.
    pub fn branch_items(&self, branch: usize) -> Range<usize> {
        let start = self.branches[branch] + 1;
        let end = self.branches.get(branch + 1).copied().unwrap_or(self.end);
        start..end
    }
}

/// Why the conditional commands of a document do not nest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionalErrorKind {
    /// `\elif` with no open `\if` (psql: `\elif: no matching \if`).
    ElifWithoutIf,
    /// `\elif` after the `\else` of its block (psql: `\elif: cannot occur
    /// after \else`).
    ElifAfterElse,
    /// `\else` with no open `\if` (psql: `\else: no matching \if`).
    ElseWithoutIf,
    /// A second `\else` in one block (psql: `\else: cannot occur after
    /// \else`).
    ElseAfterElse,
    /// `\endif` with no open `\if` (psql: `\endif: no matching \if`).
    EndifWithoutIf,
    /// An `\if` with no `\endif` before the end of the document (psql:
    /// `reached EOF without finding closing \endif(s)`).
    MissingEndif,
}

/// A conditional command that does not nest, with the item index and the
/// source extent of the command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionalError {
    /// What is wrong.
    pub kind: ConditionalErrorKind,
    /// The item index of the command. For [`ConditionalErrorKind::MissingEndif`],
    /// the `\if` that is not closed.
    pub item: usize,
    /// The source extent of that command.
    pub source: Range<usize>,
}

impl fmt::Display for ConditionalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            ConditionalErrorKind::ElifWithoutIf => "\\elif: no matching \\if",
            ConditionalErrorKind::ElifAfterElse => "\\elif: cannot occur after \\else",
            ConditionalErrorKind::ElseWithoutIf => "\\else: no matching \\if",
            ConditionalErrorKind::ElseAfterElse => "\\else: cannot occur after \\else",
            ConditionalErrorKind::EndifWithoutIf => "\\endif: no matching \\if",
            ConditionalErrorKind::MissingEndif => "\\if: no matching \\endif",
        };
        write!(
            formatter,
            "{message} at bytes {}..{}",
            self.source.start, self.source.end
        )
    }
}

impl std::error::Error for ConditionalError {}

/// An open block while the items are read.
struct Open {
    branches: Vec<usize>,
    has_else: bool,
}

impl<'input> PsqlDocument<'input> {
    /// The `\if` blocks of the document, ordered by the position of their
    /// `\endif`, so that an inner block comes before the block that holds it.
    ///
    /// `source` is the text that [`crate::parse`] read.
    ///
    /// Each error is one command that psql would refuse, in source order.
    /// psql continues after such a command, and so does this check: a
    /// misplaced command is ignored, and the blocks around it are still
    /// found.
    pub fn conditionals(&self, source: &str) -> (Vec<ConditionalBlock>, Vec<ConditionalError>) {
        let mut blocks = Vec::new();
        let mut errors = Vec::new();
        let mut open: Vec<Open> = Vec::new();
        for (index, item) in self.items.iter().enumerate() {
            let PsqlItem::Command(command) = item else {
                continue;
            };
            let error = |kind| ConditionalError {
                kind,
                item: index,
                source: source_range(source, command.text()),
            };
            match command {
                MetaCommand::If(_) => open.push(Open {
                    branches: vec![index],
                    has_else: false,
                }),
                MetaCommand::Elif(_) => match open.last_mut() {
                    None => errors.push(error(ConditionalErrorKind::ElifWithoutIf)),
                    Some(block) if block.has_else => {
                        errors.push(error(ConditionalErrorKind::ElifAfterElse))
                    }
                    Some(block) => block.branches.push(index),
                },
                MetaCommand::Else(_) => match open.last_mut() {
                    None => errors.push(error(ConditionalErrorKind::ElseWithoutIf)),
                    Some(block) if block.has_else => {
                        errors.push(error(ConditionalErrorKind::ElseAfterElse))
                    }
                    Some(block) => {
                        block.branches.push(index);
                        block.has_else = true;
                    }
                },
                MetaCommand::Endif(_) => match open.pop() {
                    None => errors.push(error(ConditionalErrorKind::EndifWithoutIf)),
                    Some(block) => blocks.push(ConditionalBlock {
                        branches: block.branches,
                        end: index,
                        depth: open.len(),
                    }),
                },
                _ => {}
            }
        }
        for block in open {
            let index = block.branches[0];
            let PsqlItem::Command(command) = &self.items[index] else {
                unreachable!("a branch index names a command");
            };
            errors.push(ConditionalError {
                kind: ConditionalErrorKind::MissingEndif,
                item: index,
                source: source_range(source, command.text()),
            });
        }
        errors.sort_by_key(|error| error.source.start);
        (blocks, errors)
    }
}

/// The extent of `part` inside `source`. Every token is a subslice of the
/// parsed source.
fn source_range(source: &str, part: &str) -> Range<usize> {
    let base = source.as_ptr() as usize;
    let start = part.as_ptr() as usize;
    assert!(
        start >= base && start + part.len() <= base + source.len(),
        "token text is not a subslice of the parsed source",
    );
    (start - base)..(start - base + part.len())
}
