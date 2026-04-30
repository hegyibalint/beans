//! Graph engine skeleton.
//!
//! Graph is structurally pure: nodes (arena) + hard links (containment
//! tree) + cache state + behavior trait. It owns no indexing or
//! notification machinery — that lives in [`crate::registry`] and is
//! wired in via the [`NodeHandle`] marker trait.
//!
//! - `arena`: [`Graph<P>`], [`NodeData<P>`], [`NodeId`], the [`NodeHandle`]
//!   marker. Flat arena with free-list slot reuse and recursive hard-link
//!   destroy.
//! - `cache_state`: [`CacheState`], [`Generation`]. Per ADR-0009 / ADR-0010.
//! - `dynamic_link`: [`RegistryQuery`] trait. Per ADR-0006 / ADR-0008 the
//!   use-site stores *the question*, not the answer; this trait is its
//!   shape. The richer "list of priority-ordered queries with combine
//!   modes" abstraction is deferred until per-language rich models and
//!   cached cross-file resolutions make it load-bearing.
//! - `behavior`: [`NodeBehavior`] trait for payload-driven lifecycle hooks.
//!
//! See ARCHITECTURE.md and the cited ADRs for the full rationale.

pub mod arena;
pub mod behavior;
pub mod cache_state;
pub mod dynamic_link;

pub use arena::{Graph, NodeData, NodeHandle, NodeId};
pub use behavior::NodeBehavior;
pub use cache_state::{CacheState, Generation};
pub use dynamic_link::RegistryQuery;
