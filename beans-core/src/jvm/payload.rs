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

use crate::jvm::annotation::AnnotationInstance;
use crate::jvm::fqn::Fqn;
use crate::jvm::modifier::Modifier;
use crate::jvm::signature::{ConstantValue, RecordComponent};
use crate::jvm::type_ref::{TypeParam, TypeRef};
use crate::primitives::Location;

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
#[derive(Debug, Clone, PartialEq)]
pub struct JvmTypeNode {
    pub header: JvmDeclHeader,
    pub kind: JvmTypeKind,
    pub type_parameters: Vec<TypeParam>,
    /// Record components, present iff `kind == JvmTypeKind::Record`. The
    /// component list is the source-of-truth for accessor generation
    /// (JLS §8.10.3); empty for non-records.
    pub record_components: Vec<RecordComponent>,
    pub enrichments: JvmEnrichments,
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
#[derive(Debug, Clone, PartialEq)]
pub struct JvmMethodNode {
    pub header: JvmDeclHeader,
    pub return_type: TypeRef,
    pub parameters: Vec<JvmParameter>,
    pub type_parameters: Vec<TypeParam>,
    pub throws: Vec<TypeRef>,
    pub enrichments: JvmEnrichments,
}

/// A JVM constructor declaration (JLS §8.8). Distinct from
/// [`JvmMethodNode`] because dispatch and resolution differ at the JVM
/// level (constructors are `<init>` methods, not named).
#[derive(Debug, Clone, PartialEq)]
pub struct JvmConstructorNode {
    pub header: JvmDeclHeader,
    pub parameters: Vec<JvmParameter>,
    pub type_parameters: Vec<TypeParam>,
    pub throws: Vec<TypeRef>,
}

/// A JVM field declaration (JLS §8.3). Includes static-final constants
/// via [`constant_value`](Self::constant_value).
#[derive(Debug, Clone, PartialEq)]
pub struct JvmFieldNode {
    pub header: JvmDeclHeader,
    pub field_type: TypeRef,
    pub constant_value: Option<ConstantValue>,
    pub initialized: bool,
    pub enrichments: JvmEnrichments,
}

/// A JVM enum constant (JLS §8.9.1). Modelled separately from
/// [`JvmFieldNode`] because the JVM projection treats it as a synthetic
/// `static final` with a known declaring enum, and consumers commonly
/// dispatch on enum-constant-ness directly.
#[derive(Debug, Clone, PartialEq)]
pub struct JvmEnumConstantNode {
    pub header: JvmDeclHeader,
    /// FQN of the enclosing enum type. Redundant with `header.fqn`'s
    /// parent, but keeping it explicit avoids a parse on every consumer.
    pub enum_owner: Fqn,
}

/// A JVM annotation-type element (JLS §9.6.1). Distinct from a method
/// because of the `default` value mechanism.
#[derive(Debug, Clone, PartialEq)]
pub struct JvmAnnotationElementNode {
    pub header: JvmDeclHeader,
    pub element_type: TypeRef,
    pub default_value: Option<ConstantValue>,
}

/// A package declaration (JLS §7.4). One node per package; the package
/// FQN is its dotted name and is also stored on `header.fqn` for
/// uniformity.
#[derive(Debug, Clone, PartialEq)]
pub struct JvmPackageNode {
    pub header: JvmDeclHeader,
}

/// Union of every JVM-projection node payload variant.
#[derive(Debug, Clone, PartialEq)]
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
