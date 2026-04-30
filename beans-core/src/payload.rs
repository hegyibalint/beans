//! Cross-layer node payload union.
//!
//! Per ADR-0019 every variant the engine can store lives in one enum.
//! Per ADR-0006 a single graph holds both per-language nodes and their
//! JVM projections, so [`NodePayload`] unions the per-language payloads
//! (gated by their language feature) with the shared JVM payload.
//! Per ADR-0021 this replaces the prototype's monolithic `Symbol`
//! (deleted in step 7 of the graph migration).
//!
//! Variants:
//! - `Jvm` — a JVM-projection node ([`JvmNodePayload`]). **Always
//!   present**, never feature-gated. Per ADR-0004 each language node
//!   hard-links a JVM projection as its descendant; cross-file resolution
//!   between languages goes through the JVM layer (the only vocabulary
//!   shared by all five). Gating `Jvm` behind a feature would dissolve
//!   the cross-language interop story the entire architecture is built
//!   around — even a Kotlin-only build of beans needs the JVM payload
//!   variant to represent the projection of the Kotlin source nodes.
//! - `Java` — a Java-side node ([`JavaNodePayload`]). Hard-links its
//!   `Jvm` projection child per ADR-0004. Gated by `feature = "java"`.
//!
//! New language variants land alongside their feature-gated module.
//!
//! Per ADR-0014 RAII handles for each variant's registrations live on
//! [`NodeData::handles`](crate::graph::NodeData::handles), not on the
//! payload itself. The payload values are therefore plain data —
//! `Send + Sync`-eligible — which lets pre-integration parse plans
//! travel between rayon workers (ADR-0005).

use crate::graph::NodeBehavior;
use crate::graph::arena::{NodeHandle, NodeId};
use crate::jvm::payload::JvmNodePayload;
use crate::registries::Registries;

#[cfg(feature = "java")]
use crate::languages::java::payload::JavaNodePayload;

/// Union of every node payload the engine can store. Variants are
/// feature-gated to match their owning language module.
#[derive(Debug, Clone, PartialEq)]
pub enum NodePayload {
    Jvm(JvmNodePayload),

    #[cfg(feature = "java")]
    Java(JavaNodePayload),
}

impl NodeBehavior for NodePayload {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        match self {
            NodePayload::Jvm(p) => p.on_created(id, ctx),

            #[cfg(feature = "java")]
            NodePayload::Java(p) => p.on_created(id, ctx),
        }
    }
}
