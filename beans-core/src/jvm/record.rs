//! Record components.
//!
//! `record Point(int x, int y)` declares two components (JLS §8.10.1);
//! each becomes a [`RecordComponent`] on the type's payload, plus a
//! synthesized field with the same name and type. Per ADR-0021 the
//! component list lives on the language node ([`JavaTypeNode`] / its
//! JVM projection [`JvmTypeNode`]); accessor generation reads from it.
//!
//! [`JavaTypeNode`]: crate::languages::java::JavaTypeNode
//! [`JvmTypeNode`]: crate::jvm::JvmTypeNode

use crate::jvm::type_ref::TypeRef;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordComponent {
    /// Component name; also the name of the auto-generated accessor.
    pub name: String,
    /// Declared component type.
    pub component_type: TypeRef,
}
