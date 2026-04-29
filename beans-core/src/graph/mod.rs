//! Graph engine skeleton.
//!
//! - `arena`: `Graph<P>`, `NodeData<P>`, `NodeId`. Flat arena with free-list
//!   slot reuse and recursive hard-link destroy.
//! - `cache_state`: `CacheState`, `Generation`. Per ADR-0009 / ADR-0010.
//! - `registry`: `Registry<K>`, `ProviderHandle<K>`, `SubscriptionHandle<K>`.
//!   Per ADR-0013 / ADR-0014 / ADR-0015.
//! - `behavior`: `NodeBehavior` trait for payload-driven lifecycle hooks.
//!
//! See ARCHITECTURE.md and the cited ADRs for the full rationale.

pub mod arena;
pub mod behavior;
pub mod cache_state;
pub mod registry;

pub use arena::{Graph, NodeData, NodeId};
pub use behavior::NodeBehavior;
pub use cache_state::{CacheState, Generation};
pub use registry::{Callback, ProviderHandle, Registry, SubscriptionHandle, SubscriptionId};

#[cfg(test)]
mod tests;
