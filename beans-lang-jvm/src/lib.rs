//! beans-lang-jvm — the shared JVM model every language vertical
//! projects into.
//!
//! Per ADR-0004 each language has its own rich model (in its
//! `beans-lang-<language>` crate); cross-language interop goes through
//! the JVM projection defined here. The types in this crate describe
//! what the JVM itself can express: classes, methods, fields,
//! modifiers, generic signatures, structural type references, and
//! annotations — plus the promoted enrichments (nullability today)
//! that cross-language consumers benefit from.
//!
//! This crate also owns [`JvmRegistries`] — the shared registry bag.
//! It is the only registry surface visible across verticals: each
//! vertical registers its projections here and queries other
//! languages' symbols exclusively through it. Language-specific
//! refinements live on the per-language node types, not on these JVM
//! types.

pub mod completion;
pub mod container;
pub mod model;
pub mod registries;

pub use completion::{CompletionCandidate, CompletionCandidates};
pub use container::{ContainerError, Jar, Jmod};
pub use model::{
    AnnotationInstance, AnnotationValue, ConstantValue, Fqn, JvmConstructorKey, JvmFieldKey,
    JvmMethodKey, JvmTypeKey, Modifier, PackageKey, PrimitiveKind, RecordComponent, SymbolKind,
    TypeParam, TypeRef, WildcardBound,
};
pub use model::{
    AsJvm, JvmAnnotationElementNode, JvmConstructorNode, JvmDeclHeader, JvmEnrichments,
    JvmEnumConstantNode, JvmFieldNode, JvmMethodNode, JvmNodePayload, JvmPackageNode, JvmParameter,
    JvmTypeKind, JvmTypeNode, NullabilityInfo,
};
pub use registries::JvmRegistries;

// Compatibility module aliases. Keep these until the facade/API cleanup
// removes broad root module paths.
pub use model::annotation;
pub use model::constant;
pub use model::fqn;
pub use model::keys;
pub use model::modifier;
pub use model::payload;
pub use model::record;
pub use model::symbol_kind;
pub use model::type_ref;
