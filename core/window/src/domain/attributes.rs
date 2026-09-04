//! [`WindowAttributes`] — what a caller asks
//! [`WindowSystem::create_window`](crate::application::ports::WindowSystem::create_window)
//! for — and [`WindowId`], the handle it hands back.

use core::fmt;

use crate::domain::surface::SurfaceSize;

/// A window's human-readable title.
///
/// Wrapped rather than a bare `String` (Object Calisthenics: no naked
/// primitives in the domain model) — the same shape as `dom::TagName`
/// wrapping a validated `String`, minus the validation this field has no need
/// of (any string is a legal title).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowTitle(String);

impl WindowTitle {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for WindowTitle {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for WindowTitle {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for WindowTitle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// What a caller asks a [`WindowSystem`](crate::application::ports::WindowSystem)
/// to create: a title and an initial surface size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowAttributes {
    title: WindowTitle,
    initial_size: SurfaceSize,
}

impl WindowAttributes {
    #[must_use]
    pub fn new(title: impl Into<WindowTitle>, initial_size: SurfaceSize) -> Self {
        Self {
            title: title.into(),
            initial_size,
        }
    }

    #[must_use]
    pub const fn title(&self) -> &WindowTitle {
        &self.title
    }

    #[must_use]
    pub const fn initial_size(&self) -> SurfaceSize {
        self.initial_size
    }
}

/// A window's identity, stable for its lifetime.
///
/// Opaque outside this crate — wraps a raw handle whose only promise is
/// per-run uniqueness (`ADR-0011` item 3). Never constructed by a caller;
/// only [`WindowSystem::create_window`](crate::application::ports::WindowSystem::create_window)
/// hands one out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(u64);

impl WindowId {
    /// Wraps a raw identifier an adapter already has (`winit`'s own
    /// `WindowId`, a sequence counter, …). Adapter-only: application code
    /// never builds a `WindowId` itself, it only ever receives one back.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "window#{}", self.0)
    }
}
