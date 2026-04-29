//! Java node payloads.
//!
//! Per ADR-0004 each language has its own rich model and projects to JVM
//! for cross-language interop. Java's source model is the closest to its
//! JVM projection of any of the five JVM languages, so [`JavaNodePayload`]
//! mirrors [`crate::jvm::JvmNodePayload`] structurally — at this stage
//! the Java payload carries the same per-kind data. The split is still
//! load-bearing: a Java node hard-links its JVM-projection child node
//! (per ADR-0004's "each language-model node hard-links a JVM
//! projection"), and Java-specific facts that don't travel through JVM
//! (when we add them) attach here, not on the JVM payload.
//!
//! For step 3 of the graph migration the variants exist as types only;
//! the parser populates them in step 4.

use crate::jvm::annotation::AnnotationInstance;
use crate::jvm::fqn::Fqn;
use crate::jvm::modifier::Modifier;
use crate::jvm::signature::{ConstantValue, RecordComponent};
use crate::jvm::type_ref::{TypeParam, TypeRef};
use crate::primitives::Location;

/// What category of Java declaration a [`JavaTypeNode`] represents.
/// Mirrors [`crate::jvm::JvmTypeKind`] one-for-one today; the split
/// exists so Java-specific kinds (none yet) can land here without
/// touching the JVM enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JavaTypeKind {
    Class,
    Interface,
    Enum,
    Record,
    Annotation,
}

/// Common header for every named Java declaration. Symmetric with
/// [`crate::jvm::JvmDeclHeader`]; duplicated rather than re-used so
/// that future Java-specific header fields don't ripple into the JVM
/// projection.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaDeclHeader {
    pub name: String,
    pub fqn: Fqn,
    pub location: Option<Location>,
    pub modifiers: Vec<Modifier>,
    pub annotations: Vec<AnnotationInstance>,
}

impl JavaDeclHeader {
    pub fn new(name: impl Into<String>, fqn: impl Into<Fqn>) -> Self {
        Self {
            name: name.into(),
            fqn: fqn.into(),
            location: None,
            modifiers: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

/// A Java parameter on a method or constructor.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaParameter {
    pub name: String,
    pub param_type: TypeRef,
    pub is_varargs: bool,
}

/// A Java type declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaTypeNode {
    pub header: JavaDeclHeader,
    pub kind: JavaTypeKind,
    pub type_parameters: Vec<TypeParam>,
    /// Record components, present iff `kind == JavaTypeKind::Record`.
    pub record_components: Vec<RecordComponent>,
}

/// A Java method declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaMethodNode {
    pub header: JavaDeclHeader,
    pub return_type: TypeRef,
    pub parameters: Vec<JavaParameter>,
    pub type_parameters: Vec<TypeParam>,
    pub throws: Vec<TypeRef>,
}

/// A Java constructor declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaConstructorNode {
    pub header: JavaDeclHeader,
    pub parameters: Vec<JavaParameter>,
    pub type_parameters: Vec<TypeParam>,
    pub throws: Vec<TypeRef>,
}

/// A Java field declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaFieldNode {
    pub header: JavaDeclHeader,
    pub field_type: TypeRef,
    pub constant_value: Option<ConstantValue>,
    pub initialized: bool,
}

/// A Java enum constant.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaEnumConstantNode {
    pub header: JavaDeclHeader,
    pub enum_owner: Fqn,
}

/// A Java annotation-type element (JLS §9.6.1).
#[derive(Debug, Clone, PartialEq)]
pub struct JavaAnnotationElementNode {
    pub header: JavaDeclHeader,
    pub element_type: TypeRef,
    pub default_value: Option<ConstantValue>,
}

/// A Java package declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaPackageNode {
    pub header: JavaDeclHeader,
}

/// Union of every Java-side node payload variant.
#[derive(Debug, Clone, PartialEq)]
pub enum JavaNodePayload {
    Type(JavaTypeNode),
    Method(JavaMethodNode),
    Constructor(JavaConstructorNode),
    Field(JavaFieldNode),
    EnumConstant(JavaEnumConstantNode),
    AnnotationElement(JavaAnnotationElementNode),
    Parameter(JavaParameter),
    Package(JavaPackageNode),
}

impl JavaNodePayload {
    /// Borrow the per-payload [`JavaDeclHeader`] uniformly. `Parameter`
    /// has no header.
    pub fn header(&self) -> Option<&JavaDeclHeader> {
        match self {
            JavaNodePayload::Type(n) => Some(&n.header),
            JavaNodePayload::Method(n) => Some(&n.header),
            JavaNodePayload::Constructor(n) => Some(&n.header),
            JavaNodePayload::Field(n) => Some(&n.header),
            JavaNodePayload::EnumConstant(n) => Some(&n.header),
            JavaNodePayload::AnnotationElement(n) => Some(&n.header),
            JavaNodePayload::Package(n) => Some(&n.header),
            JavaNodePayload::Parameter(_) => None,
        }
    }
}
