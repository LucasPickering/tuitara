use std::{
    fmt::{self, Display, Formatter},
    ops::Deref,
    sync::atomic::{AtomicU64, Ordering},
};

/// A unique ID based on an auto-incrementing integer
///
/// A global atomic is used to store the next available ID. Whenever a new ID is
/// created, the atomic is incremented. It's possible for IDs to repeat if this
/// wraps around, but since it's a u64, that's extremely unlikely in practice.
///
/// Integer IDs are more human readable than UUIDs, deterministic (which is
/// useful for repeated test runs), and don't require any dependencies.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct AutoIncrementId(u64);

impl AutoIncrementId {
    /// Get a new unique component ID
    pub fn new() -> Self {
        // We use an incrementing integer because:
        // 1. They're more human-readable than UUIDs
        // 2. IDs are consistent across test runs (helpful for debugging)
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
}

/// Generate a new unique ID
impl Default for AutoIncrementId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for AutoIncrementId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A flag that starts as false and can only be enabled
#[derive(Copy, Clone, Debug, Default)]
pub struct Flag(bool);

impl Flag {
    /// Enable the flag
    pub fn set(&mut self) {
        self.0 = true;
    }

    /// Disable the flag
    pub fn unset(&mut self) {
        self.0 = false;
    }
}

impl Deref for Flag {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
