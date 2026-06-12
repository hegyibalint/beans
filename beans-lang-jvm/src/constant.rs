//! Compile-time constant values.
//!
//! Used by `static final` field initializers ([`crate::JvmFieldNode::constant_value`])
//! and annotation element defaults
//! ([`crate::JvmAnnotationElementNode::default_value`]). The
//! variants cover JLS §15.29 constant expression types plus `Null`
//! (which is not technically a constant expression but is the only
//! sensible representation of a null annotation default).
//!
//! Numeric literals collapse to two variants — [`Int`](ConstantValue::Int)
//! for integral types and [`Float`](ConstantValue::Float) for
//! floating-point — because downstream consumers care about the value,
//! not the source type's width. The original type is recoverable from
//! the field's `field_type`.
//!
//! Per backlog #030 this enum cannot represent every JLS §9.6.1
//! annotation default form (class literals, enum refs, nested
//! annotations, arrays). Extending it is on the post-migration audit
//! list.

#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    /// An integral literal: `byte`, `short`, `int`, `long`. Stored as
    /// `i64` to fit `long` without precision loss.
    Int(i64),
    /// A floating-point literal: `float`, `double`. Stored as `f64`.
    Float(f64),
    /// A `String` literal.
    String(String),
    /// A `boolean` literal: `true` / `false`.
    Bool(bool),
    /// A `char` literal.
    Char(char),
    /// The `null` literal — used for annotation element defaults that
    /// are explicitly nullable; not a real Java constant expression.
    Null,
}
