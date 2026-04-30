//! Annotation instances attached to declarations.
//!
//! An annotation is a compile-time-evaluated value attached to a
//! declaration: `@Override`, `@Retention(RUNTIME)`,
//! `@Target({TYPE, METHOD})`. Annotations are first-class on every
//! payload variant that can carry them
//! ([`crate::jvm::JvmDeclHeader::annotations`],
//! [`crate::languages::java::JavaDeclHeader::annotations`]); diagnostic
//! rules and JVM enrichments (e.g., promoting Kotlin nullability to the
//! JVM projection) read them directly.
//!
//! The shape of an annotation value is constrained by JLS §9.6.1 — the
//! [`AnnotationValue`] variants enumerate exactly what is permitted.

use crate::{ConstantValue, TypeRef};

/// An annotation applied to a declaration: `@Override`,
/// `@Retention(RUNTIME)`, etc.
/// (JLS §9.7).
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationInstance {
    /// FQN of the annotation type, e.g., "java.lang.Override"
    pub fqn: String,
    /// Element name-value pairs. For marker annotations, this is empty
    /// (JLS §9.7.2). For single-element annotations, the element name
    /// is "value" (JLS §9.7.3).
    pub elements: Vec<(String, AnnotationValue)>,
}

/// A value in an annotation element.
///
/// Annotation element values are restricted by JLS 9.6.1 to:
/// primitives, String, Class, enums, annotations, and arrays of these.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationValue {
    /// A compile-time constant: `42`, `"hello"`, `true`
    Const(ConstantValue),
    /// A class literal: `String.class`, `int.class` (JLS §9.6.1).
    ClassLiteral(TypeRef),
    /// An enum constant reference: `RetentionPolicy.RUNTIME` (JLS §9.6.1).
    EnumRef {
        /// FQN of the enum type
        type_fqn: String,
        /// Name of the constant
        constant: String,
    },
    /// A nested annotation: `@Target(@Foo)`
    Annotation(Box<AnnotationInstance>),
    /// An array of values: `{ElementType.TYPE, ElementType.METHOD}`
    Array(Vec<AnnotationValue>),
}
