use crate::domain::property::Color;

/// Port defining color resolution operations for named colors and syntax (C-15, C-16).
pub trait ColorResolver: Send + Sync {
    /// Resolves a raw CSS color string into a strongly typed `Color`.
    fn resolve(&self, raw: &str) -> Option<Color>;
}
