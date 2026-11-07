//! Turning a [`KeymapSource`] into the [`Keymap`] the router resolves with.

use ui::Keymap;

use crate::application::ports::KeymapSource;

/// The keymap `source` supplies.
///
/// Each rejected binding is reported and skipped; when none is valid the result is
/// `Keymap::default()`, so a failing script never leaves the browser without shortcuts
/// (`ADR-0004`).
#[must_use]
pub fn resolve_keymap(source: &impl KeymapSource) -> Keymap {
    let (keymap, rejected) = Keymap::from_bindings(source.bindings());
    for error in rejected {
        tracing::warn!(%error, "keymap binding skipped");
    }
    keymap
}

#[cfg(test)]
mod tests {
    use ui::{Keymap, KeymapBinding};

    use super::resolve_keymap;
    use crate::application::ports::KeymapSource;

    struct Bindings(Vec<KeymapBinding>);

    impl KeymapSource for Bindings {
        fn bindings(&self) -> Vec<KeymapBinding> {
            self.0.clone()
        }
    }

    #[test]
    fn a_source_with_no_bindings_resolves_to_the_default_keymap() {
        assert_eq!(resolve_keymap(&Bindings(Vec::new())), Keymap::default());
    }

    #[test]
    fn a_source_with_only_invalid_bindings_resolves_to_the_default_keymap() {
        let bad = Bindings(vec![KeymapBinding::new("ctrl+k", "no_such_action")]);

        assert_eq!(resolve_keymap(&bad), Keymap::default());
    }

    #[test]
    fn one_valid_binding_replaces_the_default_and_an_invalid_one_is_skipped() {
        let source = Bindings(vec![
            KeymapBinding::new("ctrl+k", "focus_omnibox"),
            KeymapBinding::new("ctrl+j", "no_such_action"),
        ]);

        let keymap = resolve_keymap(&source);

        assert_ne!(keymap, Keymap::default());
    }
}
