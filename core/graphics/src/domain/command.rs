//! [`DisplayCommand`] — the declarative drawing vocabulary of `PRD-005:65-70`.
//!
//! Layout, CSS, UI and `DevTools` never issue a draw call; they produce these
//! commands and a backend decides how to realise them (`PRD-005:62-63`). All six
//! are declared here even though the v0.3 software backend refuses two of them:
//! the contract freezes at `F4` (`ADR-0011:121`), so a command added later would
//! cost a schema bump. Declaring the whole vocabulary now and reporting
//! [`crate::GraphicsError::Unsupported`] for the unimplemented half is cheaper
//! and more honest.
//!
//! Every coordinate here is an [`crate::Au`]: by the time a command exists it has
//! already crossed the sanitizing boundary of
//! [`crate::application::DisplayListBuilder`], so a backend never has to check
//! for `NaN` (`PRD-005:80`).

use crate::domain::color::{Color, Opacity};
use crate::domain::command_kind::CommandKind;
use crate::domain::font::{FontId, GlyphRun};
use crate::domain::geometry::Rect;
use crate::domain::image::ImageId;
use crate::domain::path::{Path, Stroke};
use crate::domain::unit::Au;

/// One sanitized drawing instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DisplayCommand {
    /// A filled rectangle, optionally with rounded corners.
    DrawRect {
        rect: Rect,
        color: Color,
        corner_radius: Au,
    },
    /// A run of already-positioned glyphs from one face.
    DrawText {
        glyphs: GlyphRun,
        color: Color,
        font: FontId,
    },
    /// A decoded image, scaled from `source` into `destination`. Refused by the
    /// v0.3 backend.
    DrawImage {
        image: ImageId,
        source: Rect,
        destination: Rect,
    },
    /// A vector outline. Refused by the v0.3 backend.
    DrawPath {
        path: Path,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    /// Narrows the drawing region until the matching [`Self::PopClip`].
    PushClip { region: Rect },
    /// Restores the drawing region in force before the matching push.
    PopClip,
    /// Opens a layer composited at `opacity` until the matching
    /// [`Self::PopOpacity`].
    PushOpacity { opacity: Opacity },
    /// Closes the layer opened by the matching push.
    PopOpacity,
}

impl DisplayCommand {
    /// The name of this command, without its payload — what an `Unsupported`
    /// error reports.
    #[must_use]
    pub const fn kind(&self) -> CommandKind {
        match self {
            Self::DrawRect { .. } => CommandKind::DrawRect,
            Self::DrawText { .. } => CommandKind::DrawText,
            Self::DrawImage { .. } => CommandKind::DrawImage,
            Self::DrawPath { .. } => CommandKind::DrawPath,
            Self::PushClip { .. } => CommandKind::PushClip,
            Self::PopClip => CommandKind::PopClip,
            Self::PushOpacity { .. } => CommandKind::PushOpacity,
            Self::PopOpacity => CommandKind::PopOpacity,
        }
    }

    /// Whether this command opens a state scope that must be closed before the
    /// list is complete.
    #[must_use]
    pub const fn opens_scope(&self) -> bool {
        matches!(self, Self::PushClip { .. } | Self::PushOpacity { .. })
    }
}
