use bitflags::bitflags;

bitflags! {
    /// Fine-grained permission flags granted to an `ExecutionContext` (PRD-003:35-48).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Capability: u32 {
        /// Allows querying and traversing DOM nodes.
        const DOM_READ         = 1 << 0;
        /// Allows modifying, creating, or deleting DOM nodes.
        const DOM_MUTATE       = 1 << 1;
        /// Allows outbound network requests (HTTP fetch).
        const NETWORK_FETCH    = 1 << 2;
        /// Allows binding network listeners / sockets.
        const NETWORK_LISTEN   = 1 << 3;
        /// Allows reading script files from local storage.
        const FS_READ_SCRIPTS  = 1 << 4;
        /// Allows writing cached assets to local storage.
        const FS_WRITE_CACHE   = 1 << 5;
        /// Allows emitting render commands to the display list.
        const GRAPHICS_DRAW    = 1 << 6;
        /// Allows resizing, moving, or altering window states.
        const WINDOW_MANAGE    = 1 << 7;
        /// Allows sending inspection events to the DevTools protocol.
        const DEVTOOLS_INSPECT = 1 << 8;
    }
}

/// A validated, immutable or explicitly mutated set of capabilities governing an execution isolate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    flags: Capability,
}

impl CapabilitySet {
    /// Constructs a capability set with specific initial flags.
    #[must_use]
    pub const fn new(flags: Capability) -> Self {
        Self { flags }
    }

    /// Constructs an empty capability set (denies all privileged operations).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            flags: Capability::empty(),
        }
    }

    /// Constructs a capability set granting all permissions (trusted superuser / system muscle).
    #[must_use]
    pub const fn all() -> Self {
        Self {
            flags: Capability::all(),
        }
    }

    /// Checks if a required capability is granted.
    #[must_use]
    pub fn contains(&self, capability: Capability) -> bool {
        self.flags.contains(capability)
    }

    /// Grants a capability to this set.
    pub fn grant(&mut self, capability: Capability) {
        self.flags.insert(capability);
    }

    /// Revokes a capability from this set.
    pub fn revoke(&mut self, capability: Capability) {
        self.flags.remove(capability);
    }

    /// Returns the raw bitflags.
    #[must_use]
    pub const fn flags(&self) -> Capability {
        self.flags
    }
}

/// Standard capability profiles for browser subsystems according to PRD-003 §3.2.
pub struct SubsystemProfile;

impl SubsystemProfile {
    /// Capabilities for DOM parser & HTML tree builder: `DOM_READ | DOM_MUTATE`.
    #[must_use]
    pub const fn dom_parser() -> CapabilitySet {
        CapabilitySet::new(Capability::DOM_READ.union(Capability::DOM_MUTATE))
    }

    /// Capabilities for CSS style & cascade: `DOM_READ | GRAPHICS_DRAW`.
    #[must_use]
    pub const fn css_cascade() -> CapabilitySet {
        CapabilitySet::new(Capability::DOM_READ.union(Capability::GRAPHICS_DRAW))
    }

    /// Capabilities for Network fetcher: `NETWORK_FETCH | FS_WRITE_CACHE`.
    #[must_use]
    pub const fn network_interceptor() -> CapabilitySet {
        CapabilitySet::new(Capability::NETWORK_FETCH.union(Capability::FS_WRITE_CACHE))
    }

    /// Capabilities for Window manager & UI: `WINDOW_MANAGE | GRAPHICS_DRAW | DOM_READ`.
    #[must_use]
    pub const fn ui_window() -> CapabilitySet {
        CapabilitySet::new(
            Capability::WINDOW_MANAGE
                .union(Capability::GRAPHICS_DRAW)
                .union(Capability::DOM_READ),
        )
    }
}
