//! The name of a display command, without its payload.
//!
//! A backend that refuses a command reports *which* command it refused, and an
//! error should not have to carry a whole `DisplayCommand` to say so. This is
//! also what lets `GraphicsError` stay `Eq` and cheap to clone.

use core::fmt;

/// One of the six declarative commands of `PRD-005:65-70`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CommandKind {
    DrawRect,
    DrawText,
    DrawImage,
    DrawPath,
    PushClip,
    PopClip,
    PushOpacity,
    PopOpacity,
}

impl CommandKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DrawRect => "DrawRect",
            Self::DrawText => "DrawText",
            Self::DrawImage => "DrawImage",
            Self::DrawPath => "DrawPath",
            Self::PushClip => "PushClip",
            Self::PopClip => "PopClip",
            Self::PushOpacity => "PushOpacity",
            Self::PopOpacity => "PopOpacity",
        }
    }
}

impl fmt::Display for CommandKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
