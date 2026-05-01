//! Graph engine skeleton.
//!
//! Graph is structurally pure: nodes (arena) + hard links (containment
//! tree) + cache state + behavior trait. It owns no indexing or
//! notification machinery — that lives in [`crate::registry`] and is
//! wired in via the [`NodeHandle`] marker trait. Cross-file lookup
//! lives in [`crate::query`] (the [`Queryable`](crate::Queryable)
//! trait + [`first_match`](crate::first_match) helpers) and
//! [`crate::multi_query`] (subscription-backed cached queries).
//!
//! - `arena`: [`Graph<P>`], [`NodeData<P>`], [`NodeId`], the [`NodeHandle`]
//!   marker. Flat arena with free-list slot reuse and recursive hard-link
//!   destroy.
//! - `cache_state`: [`CacheState`], [`Generation`]. Per ADR-0009 / ADR-0010.
//! - `behavior`: [`NodeBehavior`] trait for payload-driven lifecycle hooks.
//!
//! See ARCHITECTURE.md and the cited ADRs for the full rationale.

pub mod arena;
pub mod behavior;
pub mod cache_state;

pub use arena::{Graph, NodeData, NodeHandle, NodeId};
pub use behavior::NodeBehavior;
pub use cache_state::{CacheState, Generation};
