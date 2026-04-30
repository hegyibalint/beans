//! JVM interop layer — the shared vocabulary every JVM language projects into.
//!
//! Per ADR-0004 each language has its own rich model (under
//! [`crate::languages`]); cross-language interop goes through the JVM
//! projection defined here. The types in this module describe what the JVM
//! itself can express: classes, methods, fields, modifiers, generic
//! signatures, structural type references, and annotations.
//!
//! Language-specific refinements (Kotlin nullability, Scala HKT, Clojure
//! protocols) live on the per-language node types, not on these JVM types.
//! See [`ARCHITECTURE.md`](../../../ARCHITECTURE.md) for the full layering
//! story.

pub mod annotation;
pub mod constant;
pub mod fqn;
pub mod keys;
pub mod modifier;
pub mod payload;
pub mod record;
pub mod registries;
pub mod symbol_kind;
pub mod type_ref;

pub use annotation::{AnnotationInstance, AnnotationValue};
pub use constant::ConstantValue;
pub use fqn::Fqn;
pub use keys::{JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey};
pub use modifier::Modifier;
pub use payload::{
    JvmAnnotationElementNode, JvmConstructorNode, JvmDeclHeader, JvmEnrichments,
    JvmEnumConstantNode, JvmFieldNode, JvmMethodNode, JvmNodePayload, JvmPackageNode,
    JvmParameter, JvmTypeKind, JvmTypeNode, NullabilityInfo,
};
pub use record::RecordComponent;
pub use registries::JvmRegistries;
pub use symbol_kind::SymbolKind;
pub use type_ref::{PrimitiveKind, TypeParam, TypeRef, WildcardBound};
