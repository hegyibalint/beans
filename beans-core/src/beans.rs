//! `Beans` — top-level engine instance.
//!
//! Per workspace there is exactly one `Beans`. It owns the graph and the
//! registries; consumers (LSP, CLI, batch tools) each construct one and
//! operate the engine through it. There's no `Clone` and no `Default`-
//! based duplication: a second `Beans` is a second workspace, not a
//! shared view of the first.
//!
//! This is the entry point for the library's public API. Rather than
//! constructing `Graph::new()` and `Registries::new()` separately and
//! threading both through every call, consumers hold a `Beans` and use
//! its public fields (or future helper methods) for direct access.
//!
//! Today the struct is a thin wrapper. Engine-wide state that doesn't
//! belong to either the graph or any single registry (workspace root,
//! file → roots map, future generation counter, future snapshot
//! metadata) lands here as the runtime grows.

use crate::graph::Graph;
use crate::payload::NodePayload;
use crate::registries::Registries;

/// The top-level engine instance. Per workspace, exactly one. Owns the
/// graph + registries and any future engine-wide state.
pub struct Beans {
    pub graph: Graph<NodePayload>,
    pub registries: Registries,
}

impl Beans {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            registries: Registries::new(),
        }
    }
}

impl Default for Beans {
    fn default() -> Self {
        Self::new()
    }
}
