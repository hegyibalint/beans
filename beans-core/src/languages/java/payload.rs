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
//! Per ADR-0014 RAII registration handles live on
//! [`NodeData::handles`](crate::graph::NodeData::handles), not on the
//! payload variants. Each variant's [`NodeBehavior::on_created`] returns
//! the registered handles boxed; the engine stores them on the node and
//! drops them when the slot is freed. Per ADR-0012 every Java-side
//! declaration shares one registry — `Registries::java.symbols`,
//! keyed by [`JavaSymbolKey`] (FQN-only). Method overload disambiguation
//! happens at the JVM layer.

use crate::graph::NodeBehavior;
use crate::graph::arena::{NodeHandle, NodeId};
use crate::jvm::annotation::AnnotationInstance;
use crate::jvm::fqn::Fqn;
use crate::jvm::modifier::Modifier;
use crate::jvm::constant::ConstantValue;
use crate::jvm::record::RecordComponent;
use crate::jvm::type_ref::{TypeParam, TypeRef};
use crate::languages::java::keys::JavaSymbolKey;
use crate::primitives::Location;
use crate::registries::Registries;

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

impl NodeBehavior for JavaTypeNode {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.java.symbols.register(key, id))]
    }
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

impl NodeBehavior for JavaMethodNode {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.java.symbols.register(key, id))]
    }
}

/// A Java constructor declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaConstructorNode {
    pub header: JavaDeclHeader,
    pub parameters: Vec<JavaParameter>,
    pub type_parameters: Vec<TypeParam>,
    pub throws: Vec<TypeRef>,
}

impl NodeBehavior for JavaConstructorNode {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.java.symbols.register(key, id))]
    }
}

/// A Java field declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaFieldNode {
    pub header: JavaDeclHeader,
    pub field_type: TypeRef,
    pub constant_value: Option<ConstantValue>,
    pub initialized: bool,
}

impl NodeBehavior for JavaFieldNode {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.java.symbols.register(key, id))]
    }
}

/// A Java enum constant.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaEnumConstantNode {
    pub header: JavaDeclHeader,
    pub enum_owner: Fqn,
}

impl NodeBehavior for JavaEnumConstantNode {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.java.symbols.register(key, id))]
    }
}

/// A Java annotation-type element (JLS §9.6.1).
#[derive(Debug, Clone, PartialEq)]
pub struct JavaAnnotationElementNode {
    pub header: JavaDeclHeader,
    pub element_type: TypeRef,
    pub default_value: Option<ConstantValue>,
}

impl NodeBehavior for JavaAnnotationElementNode {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.java.symbols.register(key, id))]
    }
}

/// A Java package declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaPackageNode {
    pub header: JavaDeclHeader,
}

impl NodeBehavior for JavaPackageNode {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.java.symbols.register(key, id))]
    }
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

impl NodeBehavior for JavaNodePayload {
    type Ctx = Registries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        match self {
            JavaNodePayload::Type(n) => n.on_created(id, ctx),
            JavaNodePayload::Method(n) => n.on_created(id, ctx),
            JavaNodePayload::Constructor(n) => n.on_created(id, ctx),
            JavaNodePayload::Field(n) => n.on_created(id, ctx),
            JavaNodePayload::EnumConstant(n) => n.on_created(id, ctx),
            JavaNodePayload::AnnotationElement(n) => n.on_created(id, ctx),
            JavaNodePayload::Package(n) => n.on_created(id, ctx),
            JavaNodePayload::Parameter(_) => Vec::new(),
        }
    }
}
