//! [`ImageProvider`] — the replaceable decoded-image lookup port (v0.5 Phase X).
//!
//! `DrawImage` carries an [`ImageId`] (`PRD-005:65-70`). What the backend needs,
//! and does not itself know how to produce, is *pixels* for each registered image:
//! an `ImageProvider` resolves an [`ImageId`] to a [`Framebuffer`] containing
//! straight-alpha RGBA8 pixels.
//!
//! Object-safe and speaks only this crate's own types.

use std::sync::Arc;

use crate::domain::error::GraphicsError;
use crate::domain::framebuffer::Framebuffer;
use crate::domain::image::ImageId;

/// Resolves a registered [`ImageId`] to a decoded pixel buffer (v0.5 Phase X).
pub trait ImageProvider: Send + Sync {
    /// Returns the decoded [`Framebuffer`] for `image`.
    ///
    /// An unregistered `image` returns [`GraphicsError::ImageUnavailable`].
    fn get(&self, image: ImageId) -> Result<Arc<Framebuffer>, GraphicsError>;
}
