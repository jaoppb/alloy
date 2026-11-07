//! [`UiEffect`] — what handling input asks the executor to do.

use network::Url;
use region::RegionId;

/// The effect of one input on the chrome. `core/ui` mutates nothing outside
/// itself: `alloy`'s executor carries these out (`ADR-0026`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEffect {
    /// Nothing changed.
    None,
    /// The chrome changed and must be painted again.
    ChromeDamaged,
    /// The user submitted an address or search to navigate to.
    Navigate(Url),
    /// The user activated a link; the browser owns what it points at.
    FollowLink(RegionId),
}
