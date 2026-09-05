//! [`GraphicsError`] — the **one** typed error for this port (`ADR-0011` item 4).
//!
//! Every backend maps its native failures into these variants; no adapter type
//! (a `vulkano` result, a `glow` enum, an `io::Error`) ever crosses the seam.
//! `thiserror` rather than a hand-written `Display`: the manual carve-out of
//! `ADR-0015` applies only to `core/engine`, and this crate follows `core/dom`.

use core::fmt;

use crate::domain::command_index::CommandIndex;
use crate::domain::command_kind::CommandKind;
use crate::domain::font::FontId;
use crate::domain::image::ImageId;
use crate::domain::tier::BackendTier;

/// A failure raised while building, submitting or reading back a display list.
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GraphicsError {
    /// The requested tier could not be initialised on this machine. The cascade
    /// of `PRD-005:33-58` treats this as "try the next rung", never as fatal —
    /// which is the mechanism behind **C-17**.
    #[error("the {tier} backend is unavailable on this system")]
    BackendUnavailable { tier: BackendTier },

    /// The surface went away mid-frame — a window closed, a device reset.
    #[error("the render surface was lost during the frame")]
    SurfaceLost,

    /// A command was refused at the builder boundary (`PRD-005:80`). Carries the
    /// position of the offending command, the `ADR-0011:93-95` location metadata
    /// for this port.
    #[error("display command {index} was refused: {reason}")]
    InvalidCommand {
        index: CommandIndex,
        reason: CommandRejection,
    },

    /// The command is part of the frozen contract but this backend does not
    /// implement it. `DrawImage` and `DrawPath` in v0.3: the contract is born
    /// whole, the implementation arrives incrementally.
    #[error("the {tier} backend does not implement {command}")]
    Unsupported {
        tier: BackendTier,
        command: CommandKind,
    },

    /// Reading the frame back into host memory failed.
    #[error("could not read the frame back from the {tier} backend")]
    ReadbackFailed { tier: BackendTier },

    /// A frame operation was called out of order — `submit` before
    /// `begin_frame`, `read_back` before `end_frame`.
    #[error("{attempted} is not valid while the backend is {state}")]
    FrameOutOfOrder {
        attempted: FrameOperation,
        state: FrameState,
    },

    /// `DrawText` named a [`FontId`] its [`FontProvider`](crate::application::FontProvider)
    /// does not have registered (v0.5 B3). The command names a font, not a
    /// missing glyph — an unmapped glyph within a known font rasterizes to an
    /// empty [`crate::domain::font::GlyphBitmap`], which is not an error.
    #[error("font {font} is not registered with this backend")]
    FontUnavailable { font: FontId },

    /// `DrawImage` named an [`ImageId`] its [`ImageProvider`](crate::application::ImageProvider)
    /// does not have registered (v0.5 Phase X).
    #[error("image {image} is not registered with this backend")]
    ImageUnavailable { image: ImageId },
}

/// Why a command was refused at the builder boundary.
///
/// A typed reason rather than a string: the sanitization rules of `PRD-005:80`
/// are a closed set, and a test that asserts *which* rule fired is worth more
/// than one that greps a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CommandRejection {
    /// A coordinate was `NaN` or `±inf`. There is no correct reading, so the
    /// command is refused rather than clamped (v0.3 report §2.3).
    NonFiniteCoordinate,
    /// A width or height was negative.
    NegativeExtent,
    /// `PopClip` with no matching `PushClip`.
    ClipPopWithoutPush,
    /// `PopOpacity` with no matching `PushOpacity`.
    OpacityPopWithoutPush,
    /// The list ended with a clip still open. An unbalanced stack corrupts every
    /// command after it, so the whole list is refused rather than patched.
    ClipLeftOpen,
    /// The list ended with an opacity layer still open.
    OpacityLeftOpen,
}

impl CommandRejection {
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NonFiniteCoordinate => "a coordinate was NaN or infinite",
            Self::NegativeExtent => "a width or height was negative",
            Self::ClipPopWithoutPush => "PopClip has no matching PushClip",
            Self::OpacityPopWithoutPush => "PopOpacity has no matching PushOpacity",
            Self::ClipLeftOpen => "the list ends with a clip still open",
            Self::OpacityLeftOpen => "the list ends with an opacity layer still open",
        }
    }
}

impl fmt::Display for CommandRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

/// A method on the backend port, named so an out-of-order call can say what was
/// attempted without carrying a string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FrameOperation {
    BeginFrame,
    Submit,
    EndFrame,
    ReadBack,
}

impl FrameOperation {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BeginFrame => "begin_frame",
            Self::Submit => "submit",
            Self::EndFrame => "end_frame",
            Self::ReadBack => "read_back",
        }
    }
}

impl fmt::Display for FrameOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Where a backend sits in the frame lifecycle. Documented here because
/// `ADR-0011` item 5 requires the lifecycle to be written down, not implied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FrameState {
    /// No frame started; only `begin_frame` is valid.
    #[default]
    Idle,
    /// Between `begin_frame` and `end_frame`; `submit` and `end_frame` are valid.
    Recording,
    /// After `end_frame`; `read_back` and a new `begin_frame` are valid.
    Presented,
}

impl FrameState {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Recording => "recording",
            Self::Presented => "presented",
        }
    }
}

impl fmt::Display for FrameState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
