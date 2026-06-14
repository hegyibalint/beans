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

pub mod annotation;
pub mod completion;
pub mod constant;
pub mod container;
pub mod fqn;
pub mod keys;
pub mod modifier;
pub mod payload;
pub mod record;
pub mod registries;
pub mod symbol_kind;
pub mod type_ref;

pub use annotation::{AnnotationInstance, AnnotationValue};
pub use completion::{CompletionCandidate, CompletionCandidates};
pub use constant::ConstantValue;
pub use container::{ContainerError, Jar, Jmod};
pub use fqn::Fqn;
pub use keys::{JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey};
pub use modifier::Modifier;
pub use payload::{
    AsJvm, JvmAnnotationElementNode, JvmConstructorNode, JvmDeclHeader, JvmEnrichments,
    JvmEnumConstantNode, JvmFieldNode, JvmMethodNode, JvmNodePayload, JvmPackageNode, JvmParameter,
    JvmTypeKind, JvmTypeNode, NullabilityInfo,
};
pub use record::RecordComponent;
pub use registries::JvmRegistries;
pub use symbol_kind::SymbolKind;
pub use type_ref::{PrimitiveKind, TypeParam, TypeRef, WildcardBound};
