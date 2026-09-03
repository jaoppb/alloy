//! [`DisplayList`] — the boundary aggregate of the `RenderBackend` port.
//!
//! A first-class collection (`ADR-0010` rule 3), immutable once built, and with
//! no public constructor: the only way to obtain one is
//! [`crate::application::DisplayListBuilder::build`], which is what makes the
//! sanitization of `PRD-005:80` unbypassable rather than merely recommended.

use crate::domain::command::DisplayCommand;
use crate::domain::command_index::CommandIndex;

/// A sanitized, immutable sequence of drawing instructions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisplayList {
    commands: Vec<DisplayCommand>,
}

impl DisplayList {
    /// An empty list — a frame that paints nothing.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Wraps commands the builder has already sanitized and balanced.
    ///
    /// Deliberately `pub(crate)`: an outside caller with a `Vec` of commands has
    /// not been through the boundary, and letting one in would make every
    /// guarantee this crate offers conditional.
    pub(crate) const fn from_sanitized(commands: Vec<DisplayCommand>) -> Self {
        Self { commands }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.commands.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The command at `index`, or `None` past the end.
    #[must_use]
    pub fn command(&self, index: CommandIndex) -> Option<&DisplayCommand> {
        let position = usize::try_from(index.get()).ok()?;
        self.commands.get(position)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DisplayCommand> + '_ {
        self.commands.iter()
    }
}

impl<'list> IntoIterator for &'list DisplayList {
    type Item = &'list DisplayCommand;
    type IntoIter = core::slice::Iter<'list, DisplayCommand>;

    fn into_iter(self) -> Self::IntoIter {
        self.commands.iter()
    }
}
