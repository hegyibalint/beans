//! Graph engine skeleton.
//!
//! Per ADR-0027 the layer-1 graph is a typed arena with a hard-link
//! forest and RAII handles, and nothing more. Lazy recomputation,
//! push-stale propagation, and stable-vs-volatile lifecycle are layer-2
//! consumer concerns.
//!
//! - `arena`: [`Graph<P>`], [`NodeData<P>`], [`NodeId`], the [`NodeHandle`]
//!   marker. Flat arena with free-list slot reuse and recursive hard-link
//!   destroy.
//! - `behavior`: [`NodeBehavior`] trait for payload-driven lifecycle hooks.
//!
//! See ARCHITECTURE.md and the cited ADRs for the full rationale.

pub mod arena;
pub mod behavior;

pub use arena::{Graph, NodeData, NodeHandle, NodeId};
pub use behavior::NodeBehavior;
