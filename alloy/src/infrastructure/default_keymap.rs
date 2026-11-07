//! [`KeymapSource`] with no bindings of its own: the Rust default keymap.

use ui::KeymapBinding;

use crate::application::ports::KeymapSource;

/// Supplies nothing, which `Keymap::from_bindings` resolves to `Keymap::default()`
/// (`ADR-0027`). The seam #50 replaces with a script-backed source.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultKeymapSource;

impl KeymapSource for DefaultKeymapSource {
    fn bindings(&self) -> Vec<KeymapBinding> {
        Vec::new()
    }
}
