//! Per-kind structural information attached to a [`Symbol`](crate::Symbol).
//!
//! `Signature` is the prototype symbol model's escape hatch for the parts
//! of a declaration that vary by kind: a method has a return type and
//! parameters, a field has a type and an optional constant value, a class
//! has type parameters. Rather than baking those into [`Symbol`](crate::Symbol) directly,
//! the prototype carries `signature: Option<Signature>` and pattern-matches
//! at consumer sites.
//!
//! Per ADR-0021 the graph-engine replacement does *not* take this shape:
//! payload variants are typed (`MethodPayload`, `FieldPayload`, etc.) so
//! consumers don't pattern-match `Option`. `Signature` will go away with
//! the prototype.

use crate::jvm::type_ref::{TypeParam, TypeRef};

/// Kind-specific structural data for a symbol.
///
/// Pattern-match on the variant that corresponds to the [`SymbolKind`](crate::SymbolKind)
/// of the symbol carrying this signature. A symbol's `signature` is `None`
/// when the kind has no structural information of its own (e.g., a package).
///
/// Each variant only exists because the corresponding declaration form
/// surfaces information that doesn't fit the common `Symbol` fields. If a
/// new construct doesn't add anything beyond name/kind/parent/children, it
/// goes in without a `Signature`.
#[derive(Debug, Clone, PartialEq)]
pub enum Signature {
    /// Methods and constructors (JLS §8.4, §8.8).
    Method {
        /// Declared return type. For constructors this is the enclosing
        /// class type; for `void` methods it is `TypeRef::Void`.
        return_type: TypeRef,
        /// Formal parameters in source order (JLS §8.4.1).
        parameters: Vec<MethodParam>,
        /// Generic type parameters declared on the method:
        /// `<T> T identity(T x)` (JLS §8.4.4).
        type_parameters: Vec<TypeParam>,
        /// Checked exception types in the `throws` clause (JLS §8.4.6,
        /// §11.2). Empty for methods that declare no checked exceptions.
        throws: Vec<TypeRef>,
    },
    /// Fields, including enum constants and `static final` constants
    /// (JLS §8.3).
    Field {
        /// Declared type of the field.
        field_type: TypeRef,
        /// Compile-time constant value, if the field is `static final`
        /// and bound to a literal expression. Powers `ConstantValue`-aware
        /// hover/completion and the JLS §15.29 constant-expression rules.
        constant_value: Option<ConstantValue>,
        /// Whether the declaration carries an initializer expression
        /// (JLS §8.3.2). Distinct from `constant_value.is_some()` — a
        /// field can have an initializer that is not a compile-time
        /// constant.
        initialized: bool,
    },
    /// Classes, interfaces, and enums (per [`SymbolKind`](crate::SymbolKind))
    /// (JLS §8.1, §9.1).
    Class {
        /// Generic type parameters: `class List<E>` (JLS §8.1.2, §9.1.2).
        type_parameters: Vec<TypeParam>,
    },
    /// Records (JLS §8.10) — distinct from `Class` because the components
    /// participate in synthesized accessors and `equals`/`hashCode`.
    Record {
        /// Generic type parameters declared on the record (JLS §8.10.1).
        type_parameters: Vec<TypeParam>,
        /// Components in source order (JLS §8.10.1). Each component is
        /// also represented as a child [`Field`](crate::SymbolKind::Field)
        /// symbol; this list is the canonical source for accessor
        /// generation and component-order-dependent diagnostics.
        components: Vec<RecordComponent>,
    },
    /// Annotation type elements: the methods declared inside an
    /// `@interface` body that consumers of the annotation supply values
    /// for (JLS §9.6.1). Distinct from `Method` because of the
    /// default-value mechanism.
    AnnotationElement {
        /// Declared element type (must be primitive, `String`, `Class`,
        /// enum, annotation, or array-of-those per JLS §9.6.1).
        element_type: TypeRef,
        /// Default value supplied via the `default` clause, if any
        /// (JLS §9.6.2).
        default_value: Option<ConstantValue>,
    },
}

/// A formal parameter on a method or constructor.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodParam {
    /// Parameter name as written. Required by the symbol model even
    /// though the JVM does not preserve it without `-parameters` —
    /// source-derived symbols always have names; bytecode-derived
    /// symbols use synthetic names (`arg0`, `arg1`) when missing.
    pub name: String,
    /// Declared parameter type.
    pub param_type: TypeRef,
    /// Whether this parameter is a varargs (`...`) parameter
    /// (JLS §8.4.1). Only the *last* parameter may set this; the
    /// parser is responsible for enforcing that invariant.
    pub is_varargs: bool,
}

/// One component of a record declaration: `record Point(int x, int y)`
/// has two components (JLS §8.10.1).
#[derive(Debug, Clone, PartialEq)]
pub struct RecordComponent {
    /// Component name; also the name of the auto-generated accessor.
    pub name: String,
    /// Declared component type.
    pub component_type: TypeRef,
}

/// A compile-time constant value.
///
/// Used both for `static final` field initializers and for annotation
/// element values. The variants cover JLS §15.29 constant expression
/// types plus `Null` (which is not technically a constant expression but
/// is the only sensible representation of a null annotation default).
///
/// Numeric literals collapse to two variants — [`Int`](Self::Int) for
/// integral types and [`Float`](Self::Float) for floating-point — because
/// downstream consumers care about the value, not the source type's
/// width. The original type is recoverable from the field's
/// `field_type`.
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
