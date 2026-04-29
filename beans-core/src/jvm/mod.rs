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
pub mod modifier;
pub mod relation;
pub mod signature;
pub mod symbol;
pub mod type_ref;

pub use annotation::{AnnotationInstance, AnnotationValue};
pub use modifier::Modifier;
pub use relation::{Relation, RelationKind};
pub use signature::{ConstantValue, MethodParam, RecordComponent, Signature};
pub use symbol::Symbol;
pub use type_ref::{PrimitiveKind, TypeParam, TypeRef, WildcardBound};
