//! JVM node payloads — the typed shape every JVM-projection node carries.
//!
//! Per ADR-0021 the prototype's `Symbol`-with-`Option<Signature>` shape is
//! retired in favour of typed payload variants. A method node carries its
//! return type and parameters as fields of [`JvmMethodNode`]; consumers
//! pattern-match on the [`JvmNodePayload`] variant rather than on
//! `Option`s.
//!
//! Per ADR-0004 the JVM layer carries promoted enrichments — language-
//! sourced facts (Kotlin nullability, Scala property origins, default-
//! parameter flags) lifted onto the JVM projection so cross-language
//! consumers can read them without crossing into the source language's
//! rich model. [`JvmEnrichments`] holds those, with `nullability` the
//! only field we model today; the rest land alongside their first
//! consumer per ADR-0017 (no central pipeline, utilities-on-demand).
//!
//! Per ADR-0014 each payload variant that registers in a registry holds
//! its [`ProviderHandle`] in an `Option<...>` field on itself. Dropping
//! the payload — which the graph's hard-link GC walk does on destroy —
//! drops the handle, which removes the registry entry. Each variant's
//! [`NodeBehavior::on_created`] populates the field from the relevant
//! registry; the integrate path in `parser.rs` is the canonical caller.

use crate::graph::NodeBehavior;
use crate::graph::arena::NodeId;
use crate::graph::registry::ProviderHandle;
use crate::jvm::annotation::AnnotationInstance;
use crate::jvm::fqn::Fqn;
use crate::jvm::keys::{
    JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey,
};
use crate::jvm::modifier::Modifier;
use crate::jvm::signature::{ConstantValue, RecordComponent};
use crate::jvm::type_ref::{TypeParam, TypeRef};
use crate::primitives::Location;
use crate::registries::Registries;

/// What category of JVM declaration a [`JvmTypeNode`] represents. Records
/// and annotations have their own variants because their JVM projection
/// has structural quirks (record components, annotation elements) that
/// matter to consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JvmTypeKind {
    /// Plain `class`.
    Class,
    /// `interface`.
    Interface,
    /// `enum` (JLS §8.9).
    Enum,
    /// `record` (JLS §8.10).
    Record,
    /// `@interface` annotation type (JLS §9.6).
    Annotation,
}

/// Promoted enrichments lifted onto the JVM projection from the source
/// language model.
///
/// Per ADR-0004 promotion is explicit and minimal. Today only
/// [`nullability`](Self::nullability) is modelled; defaulting to `None`
/// for Java sources matches "the JVM has no opinion on nullability."
/// Other ARCHITECTURE.md candidates (`property_origin`, `has_defaults`)
/// land alongside their first cross-language consumer and not before.
/// When the second non-uniform field lands, the bag splits per
/// backlog #029.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JvmEnrichments {
    /// `Some(NullabilityInfo::NonNull)` for Kotlin non-nullable types or
    /// Java `@NonNull`-annotated declarations; `Some(NullabilityInfo::Nullable)`
    /// for Kotlin `T?` or Java `@Nullable`; `None` when no source
    /// language has opined.
    pub nullability: Option<NullabilityInfo>,
}

/// Nullability fact that the JVM projection promotes from a source
/// language. Distinct from "we don't know" (modelled as
/// `Option<NullabilityInfo>::None` on the enrichment) so that an
/// explicit "the source language said nullable" is not collapsed with
/// "no information available."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NullabilityInfo {
    NonNull,
    Nullable,
}

/// Common fields every named JVM declaration carries. Inlined as a flat
/// header struct to avoid repeating the same five fields across each
/// payload variant.
#[derive(Debug, Clone, PartialEq)]
pub struct JvmDeclHeader {
    pub name: String,
    pub fqn: Fqn,
    pub location: Option<Location>,
    pub modifiers: Vec<Modifier>,
    pub annotations: Vec<AnnotationInstance>,
}

impl JvmDeclHeader {
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

/// A JVM type declaration (class / interface / enum / record /
/// annotation type).
#[derive(Debug)]
pub struct JvmTypeNode {
    pub header: JvmDeclHeader,
    pub kind: JvmTypeKind,
    pub type_parameters: Vec<TypeParam>,
    /// Record components, present iff `kind == JvmTypeKind::Record`. The
    /// component list is the source-of-truth for accessor generation
    /// (JLS §8.10.3); empty for non-records.
    pub record_components: Vec<RecordComponent>,
    pub enrichments: JvmEnrichments,
    /// RAII registration in `Registries::jvm.types`. Populated by
    /// [`NodeBehavior::on_created`]; cleared on payload drop, which
    /// removes the registry entry per ADR-0014.
    pub type_provider: Option<ProviderHandle<JvmTypeKey>>,
}

impl NodeBehavior for JvmTypeNode {
    type Ctx = Registries;
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx) {
        let key = JvmTypeKey::new(self.header.fqn.clone());
        self.type_provider = Some(ctx.jvm.types.register(key, id));
    }
}

/// A JVM parameter on a method or constructor (JLS §8.4.1).
#[derive(Debug, Clone, PartialEq)]
pub struct JvmParameter {
    pub name: String,
    pub param_type: TypeRef,
    pub is_varargs: bool,
    pub enrichments: JvmEnrichments,
}

/// A JVM method declaration (JLS §8.4).
#[derive(Debug)]
pub struct JvmMethodNode {
    pub header: JvmDeclHeader,
    /// FQN of the declaring type. Redundant with `header.fqn`'s parent
    /// segment, but keeping it explicit avoids string-parsing every
    /// registration call.
    pub owner: Fqn,
    pub return_type: TypeRef,
    pub parameters: Vec<JvmParameter>,
    pub type_parameters: Vec<TypeParam>,
    pub throws: Vec<TypeRef>,
    pub enrichments: JvmEnrichments,
    /// RAII registration in `Registries::jvm.methods`.
    pub method_provider: Option<ProviderHandle<JvmMethodKey>>,
}

impl JvmMethodNode {
    /// Build the [`JvmMethodKey`] for this method. Per ADR-0012 the
    /// param types must be erased and fully-qualified; the producer
    /// (typically the parser's integration step) is responsible for
    /// pre-erasing them via [`TypeRef::erasure`] before the payload is
    /// constructed, so this just clones the stored types.
    pub fn key(&self) -> JvmMethodKey {
        JvmMethodKey::new(
            self.owner.clone(),
            self.header.name.clone(),
            self.parameters.iter().map(|p| p.param_type.clone()).collect(),
        )
    }
}

impl NodeBehavior for JvmMethodNode {
    type Ctx = Registries;
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx) {
        let key = self.key();
        self.method_provider = Some(ctx.jvm.methods.register(key, id));
    }
}

/// A JVM constructor declaration (JLS §8.8). Distinct from
/// [`JvmMethodNode`] because dispatch and resolution differ at the JVM
/// level (constructors are `<init>` methods, not named).
///
/// No `enrichments` field. The promoted enrichments [`JvmEnrichments`]
/// models today (nullability) live on the things they describe — the
/// constructor's *parameters* carry their own [`JvmParameter::enrichments`]
/// for nullability, and a constructor's "return value" is the enclosing
/// type, which is the [`JvmTypeNode`] this constructor hangs off and
/// already carries its own enrichments. Adding a constructor-level bag
/// would just duplicate the type's enrichments.
#[derive(Debug)]
pub struct JvmConstructorNode {
    pub header: JvmDeclHeader,
    pub owner: Fqn,
    pub parameters: Vec<JvmParameter>,
    pub type_parameters: Vec<TypeParam>,
    pub throws: Vec<TypeRef>,
    pub constructor_provider: Option<ProviderHandle<JvmConstructorKey>>,
}

impl JvmConstructorNode {
    pub fn key(&self) -> JvmConstructorKey {
        JvmConstructorKey::new(
            self.owner.clone(),
            self.parameters.iter().map(|p| p.param_type.clone()).collect(),
        )
    }
}

impl NodeBehavior for JvmConstructorNode {
    type Ctx = Registries;
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx) {
        let key = self.key();
        self.constructor_provider = Some(ctx.jvm.constructors.register(key, id));
    }
}

/// A JVM field declaration (JLS §8.3). Includes static-final constants
/// via [`constant_value`](Self::constant_value).
#[derive(Debug)]
pub struct JvmFieldNode {
    pub header: JvmDeclHeader,
    pub owner: Fqn,
    pub field_type: TypeRef,
    pub constant_value: Option<ConstantValue>,
    pub initialized: bool,
    pub enrichments: JvmEnrichments,
    pub field_provider: Option<ProviderHandle<JvmFieldKey>>,
}

impl JvmFieldNode {
    pub fn key(&self) -> JvmFieldKey {
        JvmFieldKey::new(self.owner.clone(), self.header.name.clone())
    }
}

impl NodeBehavior for JvmFieldNode {
    type Ctx = Registries;
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx) {
        let key = self.key();
        self.field_provider = Some(ctx.jvm.fields.register(key, id));
    }
}

/// A JVM enum constant (JLS §8.9.1). Modelled separately from
/// [`JvmFieldNode`] because the JVM projection treats it as a synthetic
/// `static final` with a known declaring enum, and consumers commonly
/// dispatch on enum-constant-ness directly. Registers under the same
/// [`JvmFieldKey`] registry as regular fields.
#[derive(Debug)]
pub struct JvmEnumConstantNode {
    pub header: JvmDeclHeader,
    /// FQN of the enclosing enum type. Redundant with `header.fqn`'s
    /// parent, but keeping it explicit avoids a parse on every consumer
    /// and removes ambiguity with the `JvmFieldKey::owner` semantics.
    pub enum_owner: Fqn,
    pub field_provider: Option<ProviderHandle<JvmFieldKey>>,
}

impl JvmEnumConstantNode {
    pub fn key(&self) -> JvmFieldKey {
        JvmFieldKey::new(self.enum_owner.clone(), self.header.name.clone())
    }
}

impl NodeBehavior for JvmEnumConstantNode {
    type Ctx = Registries;
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx) {
        let key = self.key();
        self.field_provider = Some(ctx.jvm.fields.register(key, id));
    }
}

/// A JVM annotation-type element (JLS §9.6.1). Distinct from a method
/// because of the `default` value mechanism. Registered as a zero-arg
/// method on the annotation type since that is how the JVM models them.
#[derive(Debug)]
pub struct JvmAnnotationElementNode {
    pub header: JvmDeclHeader,
    /// FQN of the enclosing annotation type.
    pub owner: Fqn,
    pub element_type: TypeRef,
    pub default_value: Option<ConstantValue>,
    pub method_provider: Option<ProviderHandle<JvmMethodKey>>,
}

impl JvmAnnotationElementNode {
    pub fn key(&self) -> JvmMethodKey {
        JvmMethodKey::new(self.owner.clone(), self.header.name.clone(), Vec::new())
    }
}

impl NodeBehavior for JvmAnnotationElementNode {
    type Ctx = Registries;
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx) {
        let key = self.key();
        self.method_provider = Some(ctx.jvm.methods.register(key, id));
    }
}

/// A package declaration (JLS §7.4). One node per package; the package
/// FQN is its dotted name and is also stored on `header.fqn` for
/// uniformity.
#[derive(Debug)]
pub struct JvmPackageNode {
    pub header: JvmDeclHeader,
    pub package_provider: Option<ProviderHandle<PackageKey>>,
}

impl JvmPackageNode {
    pub fn key(&self) -> PackageKey {
        PackageKey::new(self.header.fqn.clone())
    }
}

impl NodeBehavior for JvmPackageNode {
    type Ctx = Registries;
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx) {
        let key = self.key();
        self.package_provider = Some(ctx.jvm.packages.register(key, id));
    }
}

/// Union of every JVM-projection node payload variant.
#[derive(Debug)]
pub enum JvmNodePayload {
    Type(JvmTypeNode),
    Method(JvmMethodNode),
    Constructor(JvmConstructorNode),
    Field(JvmFieldNode),
    EnumConstant(JvmEnumConstantNode),
    AnnotationElement(JvmAnnotationElementNode),
    Parameter(JvmParameter),
    Package(JvmPackageNode),
}

impl JvmNodePayload {
    /// Borrow the per-payload [`JvmDeclHeader`] uniformly. `Parameter`
    /// has no header (parameters carry only `name`); the variant returns
    /// `None`.
    pub fn header(&self) -> Option<&JvmDeclHeader> {
        match self {
            JvmNodePayload::Type(n) => Some(&n.header),
            JvmNodePayload::Method(n) => Some(&n.header),
            JvmNodePayload::Constructor(n) => Some(&n.header),
            JvmNodePayload::Field(n) => Some(&n.header),
            JvmNodePayload::EnumConstant(n) => Some(&n.header),
            JvmNodePayload::AnnotationElement(n) => Some(&n.header),
            JvmNodePayload::Package(n) => Some(&n.header),
            JvmNodePayload::Parameter(_) => None,
        }
    }
}

impl NodeBehavior for JvmNodePayload {
    type Ctx = Registries;
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx) {
        match self {
            JvmNodePayload::Type(n) => n.on_created(id, ctx),
            JvmNodePayload::Method(n) => n.on_created(id, ctx),
            JvmNodePayload::Constructor(n) => n.on_created(id, ctx),
            JvmNodePayload::Field(n) => n.on_created(id, ctx),
            JvmNodePayload::EnumConstant(n) => n.on_created(id, ctx),
            JvmNodePayload::AnnotationElement(n) => n.on_created(id, ctx),
            JvmNodePayload::Package(n) => n.on_created(id, ctx),
            // Parameters are hard-linked under their method/constructor;
            // they don't have their own registry slot.
            JvmNodePayload::Parameter(_) => {}
        }
    }
}
