use crate::{ConstantValue, TypeRef};

/// An annotation applied to a symbol: `@Override`, `@Retention(RUNTIME)`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationInstance {
    /// FQN of the annotation type, e.g., "java.lang.Override"
    pub fqn: String,
    /// Element name-value pairs. For marker annotations, this is empty.
    /// For single-element annotations, the element name is "value".
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
    /// A class literal: `String.class`, `int.class`
    ClassLiteral(TypeRef),
    /// An enum constant reference: `RetentionPolicy.RUNTIME`
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
