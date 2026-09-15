//! Name resolution — a thin wrapper over [`ToSocketAddrs`], and deliberately
//! nothing more.
//!
//! There is **no DNS client here**. Writing one would put a hand-rolled parser
//! on the attacker-controlled-byte surface for no gain over the resolver the
//! operating system already runs, and `PRD-009` scopes it out.
//!
//! [`ToSocketAddrs`] is blocking and offers no way to bound itself. `ADR-0019`
//! answers that: the whole `execute` runs on a `std::thread` pool worker the
//! consumer can abandon, and the result comes back over `mpsc`.

use std::net::{SocketAddr, ToSocketAddrs};

use crate::domain::authority::Authority;
use crate::domain::error::NetworkError;

/// Resolve an authority to the addresses to try, in the order the resolver
/// gave them.
///
/// # Errors
///
/// [`NetworkError::Unresolved`] when the resolver fails or returns nothing.
pub fn resolve(authority: &Authority) -> Result<Vec<SocketAddr>, NetworkError> {
    let host = unbracket(authority.host().as_str());
    let addresses: Vec<SocketAddr> = (host, authority.port().number())
        .to_socket_addrs()
        .map_err(|error| NetworkError::unresolved(authority.to_text(), error.to_string()))?
        .collect();
    if addresses.is_empty() {
        return Err(NetworkError::unresolved(
            authority.to_text(),
            "the resolver returned no addresses",
        ));
    }
    Ok(addresses)
}

/// `[::1]` is how a URL spells an IPv6 literal; the resolver wants `::1`.
fn unbracket(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(host)
}
