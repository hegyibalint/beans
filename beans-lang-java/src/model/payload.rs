//! Java node payloads.
//!
//! Per ADR-0004 each language has its own rich model and projects to JVM
//! for cross-language interop. Java's source model is the closest to its
//! JVM projection of any of the five JVM languages, so [`JavaNodePayload`]
//! mirrors [`beans_lang_jvm::JvmNodePayload`] structurally — at this stage
//! the Java payload carries the same per-kind data. The split is still
//! load-bearing: a Java node hard-links its JVM-projection child node
//! (per ADR-0004's "each language-model node hard-links a JVM
//! projection"), and Java-specific facts that don't travel through JVM
//! (when we add them) attach here, not on the JVM payload.
//!
//! Per ADR-0014 RAII registration handles live on
//! [`NodeData::handles`](beans_core::graph::NodeData::handles), not on the
//! payload variants. Each variant's [`NodeBehavior::on_created`] returns
//! the registered handles boxed; the engine stores them on the node and
//! drops them when the slot is freed. Per ADR-0012 every Java-side
//! declaration shares one registry — `JavaRegistries::symbols`,
//! keyed by [`JavaSymbolKey`] (FQN-only). Method overload disambiguation
//! happens at the JVM layer.

use crate::model::keys::JavaSymbolKey;
use crate::model::registries::JavaRegistries;
use beans_core::Interner;
use beans_core::graph::NodeBehavior;
use beans_core::graph::arena::{NodeHandle, NodeId};
use beans_core::primitives::Location;
use beans_lang_jvm::model::annotation::AnnotationInstance;
use beans_lang_jvm::model::constant::ConstantValue;
use beans_lang_jvm::model::fqn::Fqn;
use beans_lang_jvm::model::modifier::Modifier;
use beans_lang_jvm::model::record::RecordComponent;
use beans_lang_jvm::model::type_ref::{TypeParam, TypeRef};

/// What category of Java declaration a [`JavaTypeNode`] represents.
/// Mirrors [`beans_lang_jvm::JvmTypeKind`] one-for-one today; the split
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
/// [`beans_lang_jvm::JvmDeclHeader`]; duplicated rather than re-used so
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

/// Header carried by every Java *use-site* node. Per ADR-0029 the IR's
/// second half — references to declarations — is built around this
/// shape, in symmetry with [`JavaDeclHeader`] for declarations.
///
/// `name` is the identifier text exactly as it appears in source. For
/// `com.example.Service`, only `Service` is captured — the qualifier
/// is structural and recoverable from context. `location` spans the
/// identifier text only, never a surrounding expression. This is the
/// load-bearing invariant for refactor-readiness: a rename rewrites
/// `location.range` mechanically.
///
/// `candidate_fqns` are the resolution candidates the parser computed
/// from imports + same-package + `java.lang` + same-file types, in
/// priority order. Resolution at use time is "first FQN whose
/// `JavaSymbolKey` has a provider in `java_symbols`."
#[derive(Debug, Clone, PartialEq)]
pub struct JavaUseHeader {
    pub name: String,
    pub location: Location,
    pub candidate_fqns: Vec<Fqn>,
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
    type Ctx = JavaRegistries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.symbols.register(key, id))]
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
    /// True iff the method declaration carries a `{ ... }` body. False
    /// for abstract methods, interface methods without `default`, and
    /// `native` methods. Read by the `abstract-method-with-body` rule;
    /// future flow-sensitive rules will read it too.
    pub has_body: bool,
}

impl NodeBehavior for JavaMethodNode {
    type Ctx = JavaRegistries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.symbols.register(key, id))]
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
    type Ctx = JavaRegistries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.symbols.register(key, id))]
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
    type Ctx = JavaRegistries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.symbols.register(key, id))]
    }
}

/// A Java enum constant.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaEnumConstantNode {
    pub header: JavaDeclHeader,
    pub enum_owner: Fqn,
}

impl NodeBehavior for JavaEnumConstantNode {
    type Ctx = JavaRegistries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.symbols.register(key, id))]
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
    type Ctx = JavaRegistries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.symbols.register(key, id))]
    }
}

/// A Java package declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaPackageNode {
    pub header: JavaDeclHeader,
}

impl NodeBehavior for JavaPackageNode {
    type Ctx = JavaRegistries;
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        let key = JavaSymbolKey::new(self.header.fqn.clone());
        vec![Box::new(ctx.symbols.register(key, id))]
    }
}

/// What an `import` declaration introduces.
///
/// Per ADR-0029 (amended): imports are first-class graph citizens
/// because they participate in cross-file refactor flow — renaming a
/// target FQN must invalidate every importing file, which is the
/// graph + registry layer's job. The variants mirror
/// [`crate::source::Import`] but without carrying
/// location data twice (the location lives on [`JavaImportNode`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JavaImportKind {
    /// `import com.example.MyClass;`
    Single,
    /// `import com.example.*;`
    Wildcard,
    /// `import static com.example.Util.MAX;`
    Static,
}

/// A Java `import` declaration as a graph node.
///
/// Hard-linked at the file root (parent: `None`), alongside the file's
/// top-level type declarations. `target` is the FQN being imported for
/// `Single`/`Static`, or the package prefix for `Wildcard`.
/// `location` spans the whole `import …;` statement so unused-import
/// can squiggle the right line.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaImportNode {
    pub kind: JavaImportKind,
    pub target: String,
    pub location: Location,
}

impl NodeBehavior for JavaImportNode {
    type Ctx = JavaRegistries;
    fn on_created(&self, _id: NodeId, _ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        // Slice 1: no registry registration. The natural follow-up is
        // a `java_imports: Registry<JavaImportKey>` so that
        // "find all importers of FQN X" becomes O(1) — useful for
        // cross-file rename refactors. Deferred until the second
        // consumer (rename) lands.
        Vec::new()
    }
}

/// A Java type-use site: a named type reference in source position.
///
/// Per ADR-0029 every type identifier appearing in a declaration
/// header — supertype, implements, field type, parameter type, return
/// type, throws, type-bound — emits one [`JavaTypeUseNode`] hard-linked
/// under the containing declaration. Multi-identifier expressions
/// (`Repository<User>`, `Map.Entry`) emit one node per identifier with
/// each node's `location` spanning only its identifier text.
///
/// `on_created` is a no-op for slice 1: resolution happens at rule-run
/// time by trying each candidate FQN against `java_symbols`. A future
/// `FallbackSubscription` per use site (when layer-2 caching needs
/// precise invalidation) will install registry watches in
/// `NodeData::handles`.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaTypeUseNode {
    pub header: JavaUseHeader,
}

impl NodeBehavior for JavaTypeUseNode {
    type Ctx = JavaRegistries;
    fn on_created(&self, _id: NodeId, _ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>> {
        // Slice 1: no registry subscription. Resolution is a per-rule
        // walk. ADR-0027 reserves the slot for a FallbackSubscription
        // when the layer-2 caching design has a real driver.
        Vec::new()
    }
}

/// Union of every Java-side node payload variant.
/// The graph stores one `NodePayload` per slot, sized to its widest
/// variant. Declaration nodes (method/type/field/...) are 130–250 B
/// while the most *numerous* nodes — use sites — are ~80 B, so the
/// decl variants are `Box`ed (backlog #037): the arena floor drops to
/// the small inline variants' width, and a method's bytes live on the
/// heap only for slots that are actually methods. `Box` derefs
/// transparently, so accessors and traversals are unaffected;
/// construction goes through the [`From`] impls below.
#[derive(Debug, Clone, PartialEq)]
pub enum JavaNodePayload {
    Type(Box<JavaTypeNode>),
    Method(Box<JavaMethodNode>),
    Constructor(Box<JavaConstructorNode>),
    Field(Box<JavaFieldNode>),
    EnumConstant(Box<JavaEnumConstantNode>),
    AnnotationElement(Box<JavaAnnotationElementNode>),
    Package(Box<JavaPackageNode>),
    // Inline: small and/or numerous. TypeUse is the most common node;
    // an extra allocation per use-site is not worth its ~80 B.
    Parameter(JavaParameter),
    TypeUse(JavaTypeUseNode),
    Import(JavaImportNode),
}

impl From<JavaTypeNode> for JavaNodePayload {
    fn from(n: JavaTypeNode) -> Self {
        Self::Type(Box::new(n))
    }
}
impl From<JavaMethodNode> for JavaNodePayload {
    fn from(n: JavaMethodNode) -> Self {
        Self::Method(Box::new(n))
    }
}
impl From<JavaConstructorNode> for JavaNodePayload {
    fn from(n: JavaConstructorNode) -> Self {
        Self::Constructor(Box::new(n))
    }
}
impl From<JavaFieldNode> for JavaNodePayload {
    fn from(n: JavaFieldNode) -> Self {
        Self::Field(Box::new(n))
    }
}
impl From<JavaEnumConstantNode> for JavaNodePayload {
    fn from(n: JavaEnumConstantNode) -> Self {
        Self::EnumConstant(Box::new(n))
    }
}
impl From<JavaAnnotationElementNode> for JavaNodePayload {
    fn from(n: JavaAnnotationElementNode) -> Self {
        Self::AnnotationElement(Box::new(n))
    }
}
impl From<JavaPackageNode> for JavaNodePayload {
    fn from(n: JavaPackageNode) -> Self {
        Self::Package(Box::new(n))
    }
}
impl From<JavaParameter> for JavaNodePayload {
    fn from(n: JavaParameter) -> Self {
        Self::Parameter(n)
    }
}
impl From<JavaTypeUseNode> for JavaNodePayload {
    fn from(n: JavaTypeUseNode) -> Self {
        Self::TypeUse(n)
    }
}
impl From<JavaImportNode> for JavaNodePayload {
    fn from(n: JavaImportNode) -> Self {
        Self::Import(n)
    }
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
            JavaNodePayload::TypeUse(_) => None,
            JavaNodePayload::Import(_) => None,
        }
    }

    /// Borrow the per-payload [`JavaUseHeader`] uniformly. `None` for
    /// every declaration variant; `Some` only for use-site variants.
    /// Symmetric with [`Self::header`].
    pub fn use_header(&self) -> Option<&JavaUseHeader> {
        match self {
            JavaNodePayload::TypeUse(n) => Some(&n.header),
            _ => None,
        }
    }
}

impl NodeBehavior for JavaNodePayload {
    type Ctx = JavaRegistries;
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
            JavaNodePayload::TypeUse(n) => n.on_created(id, ctx),
            // Slice 1 (ADR-0029): imports are passive location carriers;
            // no registry registration until cross-file import rename lands.
            JavaNodePayload::Import(_) => Vec::new(),
        }
    }
}

/// Payload projection: "does this payload carry a Java node?"
///
/// Vertical code is generic over the graph's payload type (the union
/// lives in the `beans` facade, above every vertical). The facade
/// implements this for its union; rules and resolution helpers bound
/// on `P: AsJava` to match Java payloads without seeing the union.
pub trait AsJava {
    fn as_java(&self) -> Option<&JavaNodePayload>;
}

impl AsJava for JavaNodePayload {
    fn as_java(&self) -> Option<&JavaNodePayload> {
        Some(self)
    }
}

// ---- FQN interning (backlog #037) ----
//
// Per-node `intern_fqns`, forwarded by the payload enum like
// `on_created`/`header()`. Co-locating the "which fields are FQNs"
// knowledge with each node means a new FQN field is interned where it
// is declared. Called from `integrate` at the serial boundary
// (ADR-0005); the parse phase stays interner-free.

impl JavaDeclHeader {
    /// Re-key this header's FQN onto the workspace's interned buffers.
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.fqn.intern_in(interner);
    }
}

impl JavaUseHeader {
    /// Re-key every candidate FQN onto the interned buffers.
    pub fn intern_fqns(&mut self, interner: &Interner) {
        for fqn in &mut self.candidate_fqns {
            fqn.intern_in(interner);
        }
    }
}

impl JavaTypeNode {
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.header.intern_fqns(interner);
    }
}
impl JavaMethodNode {
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.header.intern_fqns(interner);
    }
}
impl JavaConstructorNode {
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.header.intern_fqns(interner);
    }
}
impl JavaFieldNode {
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.header.intern_fqns(interner);
    }
}
impl JavaEnumConstantNode {
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.header.intern_fqns(interner);
        self.enum_owner.intern_in(interner);
    }
}
impl JavaAnnotationElementNode {
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.header.intern_fqns(interner);
    }
}
impl JavaPackageNode {
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.header.intern_fqns(interner);
    }
}
impl JavaTypeUseNode {
    pub fn intern_fqns(&mut self, interner: &Interner) {
        self.header.intern_fqns(interner);
    }
}

impl JavaNodePayload {
    /// Intern every FQN this payload owns. Forwarded per-variant; the
    /// match is exhaustive, so a new variant is a compile error here.
    pub fn intern_fqns(&mut self, interner: &Interner) {
        match self {
            JavaNodePayload::Type(n) => n.intern_fqns(interner),
            JavaNodePayload::Method(n) => n.intern_fqns(interner),
            JavaNodePayload::Constructor(n) => n.intern_fqns(interner),
            JavaNodePayload::Field(n) => n.intern_fqns(interner),
            JavaNodePayload::EnumConstant(n) => n.intern_fqns(interner),
            JavaNodePayload::AnnotationElement(n) => n.intern_fqns(interner),
            JavaNodePayload::Package(n) => n.intern_fqns(interner),
            JavaNodePayload::TypeUse(n) => n.intern_fqns(interner),
            // Parameters and imports carry no FQN.
            JavaNodePayload::Parameter(_) | JavaNodePayload::Import(_) => {}
        }
    }
}
