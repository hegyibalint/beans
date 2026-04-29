//! Java language module.
//!
//! Per ADR-0004 / ADR-0019 each JVM language has its own module under
//! [`crate::languages`]. Java's surface is small — Java's source model
//! maps cleanly to its JVM projection — so the module today carries
//! [`JavaSymbolKey`] (the single per-language registry key),
//! [`JavaRegistries`] (the bag), and [`JavaNodePayload`] (the typed
//! per-kind payload). The Java parser, type resolution, and rule set
//! land in subsequent migration steps; the types here exist now so the
//! engine and registry tests can exercise the language layer end-to-end
//! without waiting for the parser port.

pub mod keys;
pub mod payload;
pub mod registries;

pub use keys::JavaSymbolKey;
pub use payload::{
    JavaAnnotationElementNode, JavaConstructorNode, JavaDeclHeader, JavaEnumConstantNode,
    JavaFieldNode, JavaMethodNode, JavaNodePayload, JavaPackageNode, JavaParameter,
    JavaTypeKind, JavaTypeNode,
};
pub use registries::JavaRegistries;
