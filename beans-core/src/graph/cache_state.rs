//! Cache state for graph nodes.
//!
//! Per ADR-0009 / ADR-0010: nodes are either `Fresh` with a generation stamp,
//! `Stale` (will be recomputed on next pull), or `Computing` (recompute is
//! in flight; used for cycle detection).

/// Monotonically increasing counter, bumped each time the graph invalidates
/// any node. The freshness of a cached value is anchored to the generation
/// at which it was computed.
///
/// The inner `u64` is `pub(crate)` so snapshot save/load and intra-crate
/// tests can construct generations freely; consumers observe the bits via
/// `raw()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Generation(pub(crate) u64);

impl Generation {
    pub const ZERO: Generation = Generation(0);

    pub fn bump(self) -> Generation {
        Generation(self.0 + 1)
    }

    /// Observe the underlying `u64`. For logging and snapshot serialization.
    pub fn raw(self) -> u64 {
        self.0
    }

    #[allow(dead_code)] // symmetry with raw(); used by future snapshot loader.
    pub(crate) fn from_raw(raw: u64) -> Self {
        Generation(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    Fresh(Generation),
    Stale,
    Computing,
}

impl CacheState {
    pub fn is_fresh(self) -> bool {
        matches!(self, CacheState::Fresh(_))
    }

    pub fn is_stale(self) -> bool {
        matches!(self, CacheState::Stale)
    }
}
