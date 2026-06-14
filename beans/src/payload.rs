//! Cross-layer node payload union.
//!
//! Per the vertical crate layout this union lives in the facade — the
//! one crate that sees every language. Per ADR-0006 a single graph
//! holds both per-language nodes and their JVM projections, so
//! [`NodePayload`] unions the per-language payloads with the shared JVM
//! payload.
//!
//! Variants:
//! - `Jvm` — a JVM-projection node ([`JvmNodePayload`]). Per ADR-0004
//!   each language node hard-links a JVM projection as its descendant;
//!   cross-file resolution between languages goes through the JVM layer
//!   (the only vocabulary shared by all five). It is the shared spine of
//!   the cross-language interop story the whole architecture is built
//!   around.
//! - `Java` — a Java-side node ([`JavaNodePayload`]). Hard-links its
//!   `Jvm` projection child per ADR-0004.
//!
//! New language variants land alongside their vertical crate (ADR-0033:
//! the facade composes every vertical unconditionally — no Cargo
//! features gate the arms).
//!
//! Vertical code never sees this union. Walkers construct payloads
//! through the `From` impls below (generic `P: From<JavaNodePayload>`
//! bounds); rules match through the projection traits
//! ([`AsJvm`], [`AsJava`]). The facade is where both meet.
//!
//! Per ADR-0014 RAII handles for each variant's registrations live on
//! `NodeData::handles`, not on the payload itself. The payload values
//! are therefore plain data — `Send + Sync`-eligible — which lets
//! pre-integration parse plans travel between rayon workers (ADR-0005).

use beans_core::graph::NodeBehavior;
use beans_core::graph::arena::{NodeHandle, NodeId};
use beans_lang_java::payload::{AsJava, JavaNodePayload};
use beans_lang_jvm::payload::{AsJvm, JvmNodePayload};

use crate::registries::Registries;

/// Union of every node payload the engine can store: one variant per
/// vertical crate, plus the shared JVM projection.
#[derive(Debug, Clone, PartialEq)]
pub enum NodePayload {
    Jvm(JvmNodePayload),
    Java(JavaNodePayload),
}

impl NodeBehavior for NodePayload {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        match self {
            NodePayload::Jvm(p) => p.on_created(id, &ctx.jvm),
            NodePayload::Java(p) => p.on_created(id, &ctx.java),
        }
    }
}

impl From<JvmNodePayload> for NodePayload {
    fn from(p: JvmNodePayload) -> Self {
        NodePayload::Jvm(p)
    }
}

impl From<JavaNodePayload> for NodePayload {
    fn from(p: JavaNodePayload) -> Self {
        NodePayload::Java(p)
    }
}

impl AsJvm for NodePayload {
    fn as_jvm(&self) -> Option<&JvmNodePayload> {
        // Arms explicit (not `_`) so a new `NodePayload` variant is a
        // compile error here, not a silent `None` (ADR-0030).
        match self {
            NodePayload::Jvm(p) => Some(p),
            NodePayload::Java(_) => None,
        }
    }
}

impl AsJava for NodePayload {
    fn as_java(&self) -> Option<&JavaNodePayload> {
        match self {
            NodePayload::Java(p) => Some(p),
            NodePayload::Jvm(_) => None,
        }
    }
}
