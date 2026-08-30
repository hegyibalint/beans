use std::fmt;

use beans_core::model::{Offset, OffsetSpan};

#[derive(Debug, Clone)]
pub struct File {
    pub package: Option<Name>,
    pub imports: Vec<Import>,

    pub declarations: Vec<Declaration>,
    pub lexical_scopes: Vec<LexicalScope>,
    pub bodies: Vec<Body>,

    pub compilation_unit_scope: LexicalScopeId,
    pub top_level_declarations: Vec<DeclarationId>,

    /// Derived from the rest of the model; rebuilt after parsing.
    pub position_index: PositionIndex,
}

impl File {
    pub fn new() -> Self {
        Self {
            package: None,
            imports: Vec::new(),
            declarations: Vec::new(),
            lexical_scopes: vec![LexicalScope {
                parent: None,
                owner: None,
                declarations: Vec::new(),
                span: OffsetSpan {
                    start: Offset(0),
                    end: Offset(0),
                },
            }],
            bodies: Vec::new(),
            compilation_unit_scope: LexicalScopeId(0),
            top_level_declarations: Vec::new(),
            position_index: PositionIndex::default(),
        }
    }

    pub fn iter_scope_chain<'file>(
        &'file self,
        start: LexicalScopeId,
    ) -> impl Iterator<Item = (LexicalScopeId, &'file LexicalScope)> + 'file {
        std::iter::successors(Some(start), move |scope_id| {
            self.lexical_scopes.get(scope_id.0).unwrap().parent
        })
        .map(move |scope_id| (scope_id, self.lexical_scopes.get(scope_id.0).unwrap()))
    }

    /// The nearest type whose body encloses `scope`: what `this` refers to.
    pub fn enclosing_type_declaration(&self, scope: LexicalScopeId) -> Option<DeclarationId> {
        self.iter_scope_chain(scope)
            .filter_map(|(_, scope)| scope.owner)
            .find(|owner| matches!(self.declarations[owner.0], Declaration::Type(_)))
    }

    /// A display name for a declaration: dotted for types (`p.Outer.Inner`),
    /// the bare name for everything else.
    pub fn declaration_label(&self, declaration: DeclarationId) -> Option<String> {
        let name = self.declarations[declaration.0].name()?;
        let Declaration::Type(_) = self.declarations[declaration.0] else {
            return Some(name.text.clone());
        };

        let mut segments = vec![name.text.clone()];
        let mut declaring = match &self.declarations[declaration.0] {
            Declaration::Type(declaration) => declaration.declaring_scope,
            _ => unreachable!(),
        };
        while let Some(owner) = self.lexical_scopes[declaring.0].owner {
            let Declaration::Type(owner_type) = &self.declarations[owner.0] else {
                break;
            };
            segments.push(owner_type.name.as_ref()?.text.clone());
            declaring = owner_type.declaring_scope;
        }
        segments.reverse();

        match &self.package {
            Some(package) => Some(format!("{}.{}", package.dotted(), segments.join("."))),
            None => Some(segments.join(".")),
        }
    }
}

impl Default for File {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclarationId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LexicalScopeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyNodeId(pub usize);

/// JLS 6.1: different entities can share a spelling; resolution filters by this axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    Type,
    Variable,
    Method,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub text: String,
    pub span: OffsetSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Name {
    Simple(Identifier),
    Qualified(QualifiedName),
}

impl Name {
    pub fn segments(&self) -> &[Identifier] {
        match self {
            Self::Simple(identifier) => std::slice::from_ref(identifier),
            Self::Qualified(name) => name.segments(),
        }
    }

    pub fn span(&self) -> OffsetSpan {
        match self {
            Self::Simple(identifier) => identifier.span,
            Self::Qualified(name) => name.span,
        }
    }

    pub fn dotted(&self) -> String {
        dotted(self.segments())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    segments: Vec<Identifier>,
    pub span: OffsetSpan,
}

impl QualifiedName {
    pub(crate) fn new(segments: Vec<Identifier>, span: OffsetSpan) -> Self {
        assert!(
            segments.len() >= 2,
            "a qualified Java name has at least two identifiers"
        );
        Self { segments, span }
    }

    pub fn segments(&self) -> &[Identifier] {
        &self.segments
    }
}

pub(crate) fn dotted(segments: &[Identifier]) -> String {
    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(".")
}

#[derive(Debug, Clone)]
pub struct Import {
    pub name: Name,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    Type,           // import a.b.C;
    TypeOnDemand,   // import a.b.*;
    Static,         // import static a.b.C.member;
    StaticOnDemand, // import static a.b.C.*;
    Module,         // import module a.b;  (Java 25)
}

#[derive(Debug, Clone)]
pub enum Declaration {
    Type(TypeDeclaration),
    TypeParameter(TypeParameterDeclaration),
    Field(FieldDeclaration),
    Constructor(ConstructorDeclaration),
    Method(MethodDeclaration),
    Parameter(ParameterDeclaration),
    Local(LocalDeclaration),
}

impl Declaration {
    pub fn name(&self) -> Option<&Identifier> {
        match self {
            Self::Type(declaration) => declaration.name.as_ref(),
            Self::TypeParameter(declaration) => Some(&declaration.name),
            Self::Field(declaration) => declaration.name.as_ref(),
            Self::Constructor(_) => None,
            Self::Method(declaration) => declaration.name.as_ref(),
            Self::Parameter(declaration) => declaration.name.as_ref(),
            Self::Local(declaration) => declaration.name.as_ref(),
        }
    }

    pub fn name_span(&self) -> Option<OffsetSpan> {
        self.name().map(|name| name.span)
    }

    /// Total: every declaration is rooted in written source.
    pub fn span(&self) -> OffsetSpan {
        match self {
            Self::Type(declaration) => declaration.span,
            Self::TypeParameter(declaration) => declaration.name.span,
            Self::Field(declaration) => declaration.span,
            Self::Constructor(declaration) => declaration.span,
            Self::Method(declaration) => declaration.span,
            Self::Parameter(declaration) => declaration.span,
            Self::Local(declaration) => declaration.span,
        }
    }

    pub fn namespace(&self) -> Namespace {
        match self {
            Self::Type(_) | Self::TypeParameter(_) => Namespace::Type,
            Self::Field(_) | Self::Parameter(_) | Self::Local(_) => Namespace::Variable,
            Self::Constructor(_) | Self::Method(_) => Namespace::Method,
        }
    }

    pub fn declaring_scope(&self) -> LexicalScopeId {
        match self {
            Self::Type(declaration) => declaration.declaring_scope,
            // Type parameters are not parsed yet; when they are, they must
            // carry the scope of their generic declaration.
            Self::TypeParameter(_) => LexicalScopeId(0),
            Self::Field(declaration) => declaration.declaring_scope,
            Self::Constructor(declaration) => declaration.declaring_scope,
            Self::Method(declaration) => declaration.declaring_scope,
            Self::Parameter(declaration) => declaration.declaring_scope,
            Self::Local(declaration) => declaration.declaring_scope,
        }
    }

    /// The type annotation owned by this declaration, if any. A type
    /// declaration has none: its `extends` and `implements` clauses are a list
    /// rather than an annotation, and `TypeDeclaration::supertypes` answers
    /// for them.
    pub fn type_ref(&self) -> Option<&TypeRef> {
        match self {
            Self::Field(declaration) => declaration.referenced_type.as_ref(),
            Self::Method(declaration) => declaration.return_type.as_ref(),
            Self::Parameter(declaration) => declaration.ty.as_ref(),
            Self::Local(declaration) => declaration.ty.as_ref(),
            _ => None,
        }
    }
}

/// One place a type was written. §10.2 lets a single type come from up to
/// three places — `int[] f[]` is `int[][]`, with brackets on the type and on
/// the declarator — so the span covers the occurrence rather than the type.
#[derive(Debug, Clone)]
pub struct TypeRef {
    pub span: OffsetSpan,
    pub ty: Type,
}

/// JLS 26 §4.3's *Type*, as written. Unresolved on purpose: `Named` holds a
/// spelling and what it denotes is §6.5.5's business.
///
/// Type arguments are dropped. §4.6 erases them and the lake holds erased
/// types, so `List<String>` and `List` are one thing here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// §4.2.
    Primitive(Primitive),
    /// §4.3 lists *ClassOrInterfaceType* and *TypeVariable* separately, but
    /// they are one spelling — `T` and `String` are the same syntax and only
    /// resolution can tell them apart.
    Named(Name),
    /// §10.1: "The component type of an array may itself be an array type."
    /// The grammar spells this flat (a base plus *Dims*) while the type system
    /// is recursive; this follows the type system, because §10.2 can spread
    /// one type's brackets across the text with no contiguous middle to point
    /// at.
    Array(Box<Type>),
    /// §8.4.5's *Result* is `UnannType` or `void`, so `void` is not a *Type*
    /// at all. It lives here anyway rather than in a second enum used by one
    /// field; the cost is that `Array(Void)` is representable, and the grammar
    /// cannot produce it.
    Void,
}

/// §4.2's primitive types. `void` is not among them (§4.2, §8.4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primitive {
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
}

impl Type {
    /// The name this type is rooted in, if it is rooted in one at all. An
    /// array defers to its component type (§10.1), so `String[][]` answers
    /// `String`.
    pub fn named(&self) -> Option<&Name> {
        match self {
            Self::Named(name) => Some(name),
            Self::Array(component) => component.named(),
            Self::Primitive(_) | Self::Void => None,
        }
    }
}

impl Primitive {
    /// The keyword, which §3.9 makes a keyword and not an identifier — the
    /// reason these are an enum rather than a `Name` with a flag beside it.
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Boolean => "boolean",
            Self::Byte => "byte",
            Self::Char => "char",
            Self::Short => "short",
            Self::Int => "int",
            Self::Long => "long",
            Self::Float => "float",
            Self::Double => "double",
        }
    }

    pub fn from_keyword(keyword: &str) -> Option<Primitive> {
        Some(match keyword {
            "boolean" => Self::Boolean,
            "byte" => Self::Byte,
            "char" => Self::Char,
            "short" => Self::Short,
            "int" => Self::Int,
            "long" => Self::Long,
            "float" => Self::Float,
            "double" => Self::Double,
            _ => return None,
        })
    }
}

/// The type as a reader of the source would see it, which is what an editor
/// shows beside a completion row. Not the resolved type: `String` reads
/// `String` whether or not anything answers that name.
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(primitive) => f.write_str(primitive.keyword()),
            Self::Named(name) => f.write_str(&name.dotted()),
            Self::Array(component) => write!(f, "{component}[]"),
            Self::Void => f.write_str("void"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeDeclaration {
    pub span: OffsetSpan,
    pub name: Option<Identifier>,
    pub kind: TypeKind,
    /// `None` where §8.1.1 says access control does not apply: local and
    /// anonymous classes.
    pub access: Option<Access>,
    /// §8.1.4's `extends` clause. Only a normal class declaration has one, so
    /// this is `None` for an interface, an enum and a record however they were
    /// written.
    pub superclass: Option<TypeRef>,
    /// §8.1.5's `implements` clause, and §9.1.3's `extends` clause — which is
    /// the same list under a different keyword, and why an interface's
    /// supertypes land here rather than in `superclass`.
    pub interfaces: Vec<TypeRef>,
    pub declaring_scope: LexicalScopeId,
    pub body_scope: LexicalScopeId,
}

/// Which supertype of a declaration, for something holding on to one.
///
/// Two clauses rather than one flat index, because a flat index means
/// different things depending on what was written: position 0 is the
/// superclass in `class C extends B implements A` and a superinterface in
/// `interface I extends A`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupertypeId {
    Superclass,
    Interface(usize),
}

impl TypeDeclaration {
    /// Every supertype this declaration names, in §8.2's order: what it
    /// inherits from its direct superclass, then from its direct
    /// superinterfaces.
    ///
    /// Only what was written. §8.1.4 gives a class with no `extends` clause
    /// `Object` as its direct superclass type, and §9.2 does the equivalent for
    /// a bare interface, but neither appears in the source and neither belongs
    /// in a model of it.
    pub fn supertypes(&self) -> impl Iterator<Item = (SupertypeId, &TypeRef)> {
        self.superclass
            .iter()
            .map(|type_ref| (SupertypeId::Superclass, type_ref))
            .chain(
                self.interfaces
                    .iter()
                    .enumerate()
                    .map(|(index, type_ref)| (SupertypeId::Interface(index), type_ref)),
            )
    }

    pub fn supertype(&self, id: SupertypeId) -> Option<&TypeRef> {
        match id {
            SupertypeId::Superclass => self.superclass.as_ref(),
            SupertypeId::Interface(index) => self.interfaces.get(index),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Class,
    Interface,
    Enum,
    Record,
    AnnotationInterface,
}

/// JLS 26 §6.6.1's four levels. Exactly one applies: §8.1.1 makes more than one
/// access modifier a compile-time error, so this is an enum and not a flag set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    Public,
    Protected,
    Package,
    Private,
}

/// The level that applies, and where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Access {
    pub level: AccessLevel,
    /// Where the modifier was written. `None` means the position supplied it:
    ///
    /// - top-level type, or class member: package (§6.6.1)
    /// - anything in an interface body: `public` (§9.3, §9.4, §9.5)
    /// - normal class constructor: package (§8.8.3)
    /// - enum constructor: `private` (§8.9)
    pub declared_at: Option<OffsetSpan>,
}

#[derive(Debug, Clone)]
pub struct TypeParameterDeclaration {
    /// Total: the name is the entire payload of a type parameter today.
    pub name: Identifier,
}

#[derive(Debug, Clone)]
pub struct FieldDeclaration {
    pub span: OffsetSpan,
    pub name: Option<Identifier>,
    pub access: Option<Access>,
    pub referenced_type: Option<TypeRef>,
    pub declaring_scope: LexicalScopeId,
}

#[derive(Debug, Clone)]
pub struct ConstructorDeclaration {
    pub span: OffsetSpan,
    pub parameters: Vec<DeclarationId>,
    pub declaring_scope: LexicalScopeId,
    pub body_scope: LexicalScopeId,
    pub body: Option<BodyId>,
}

#[derive(Debug, Clone)]
pub struct MethodDeclaration {
    pub span: OffsetSpan,
    pub name: Option<Identifier>,
    /// §8.4.3's access modifier, read the same way a field's is. `None` where
    /// §8.1.1 puts a declaration outside access control, which for a method
    /// means one declared in a local or anonymous class.
    pub access: Option<Access>,
    pub return_type: Option<TypeRef>,
    pub parameters: Vec<DeclarationId>,
    pub declaring_scope: LexicalScopeId,
    pub body_scope: LexicalScopeId,
    pub body: Option<BodyId>,
}

#[derive(Debug, Clone)]
pub struct ParameterDeclaration {
    pub span: OffsetSpan,
    pub name: Option<Identifier>,
    pub ty: Option<TypeRef>,
    pub declaring_scope: LexicalScopeId,
}

#[derive(Debug, Clone)]
pub struct LocalDeclaration {
    pub span: OffsetSpan,
    pub name: Option<Identifier>,
    pub ty: Option<TypeRef>,
    pub declaring_scope: LexicalScopeId,
}

#[derive(Debug, Clone)]
pub struct LexicalScope {
    pub parent: Option<LexicalScopeId>,
    /// The declaration that introduced this scope (type bodies, methods), if any.
    pub owner: Option<DeclarationId>,
    pub declarations: Vec<DeclarationId>,
    pub span: OffsetSpan,
}

/// The executable content of a method, constructor, or initializer.
/// Statements and expressions share one arena; span and enclosing scope
/// are inline on every node.
#[derive(Debug, Clone)]
pub struct Body {
    /// The scope of the root block.
    pub scope: LexicalScopeId,
    pub root: BodyNodeId,
    pub nodes: Vec<BodyNode>,
}

impl Body {
    pub fn node(&self, id: BodyNodeId) -> &BodyNode {
        &self.nodes[id.0]
    }

    pub fn expression(&self, id: BodyNodeId) -> Option<&Expression> {
        match &self.node(id).kind {
            BodyNodeKind::Expression(expression) => Some(expression),
            _ => None,
        }
    }
}

/// OffsetSpan and scope are stamped at extraction: the parser knows both, and
/// later queries should never re-derive what was free at parse time.
#[derive(Debug, Clone)]
pub struct BodyNode {
    pub span: OffsetSpan,
    /// The scope this node lives in. Note a `Block` *introduces* a deeper
    /// scope (its payload), but is stamped with the scope it lives in.
    pub scope: LexicalScopeId,
    pub kind: BodyNodeKind,
}

#[derive(Debug, Clone)]
pub enum BodyNodeKind {
    Statement(Statement),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Block {
        scope: LexicalScopeId,
        statements: Vec<BodyNodeId>,
    },
    TypeDeclaration(DeclarationId),
    LocalDeclaration {
        declaration: DeclarationId,
        initializer: Option<BodyNodeId>,
    },
    Expression(BodyNodeId),
    Return(Option<BodyNodeId>),
}

#[derive(Debug, Clone)]
pub enum Expression {
    NameRef {
        name: Identifier,
    },
    FieldAccess {
        receiver: BodyNodeId,
        name: Identifier,
    },
    MethodCall {
        /// `None` is the implicit `this` receiver.
        receiver: Option<BodyNodeId>,
        name: Identifier,
        arguments: Vec<BodyNodeId>,
    },
    ObjectCreation {
        ty: TypeRef,
        arguments: Vec<BodyNodeId>,
    },
    This,
    Assign {
        target: BodyNodeId,
        value: BodyNodeId,
    },
    Literal,
}

/// Anything the position index can hand back for an offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityId {
    Declaration(DeclarationId),
    /// The type annotation owned by a declaration (field/parameter/local type,
    /// method return type).
    TypeRef(DeclarationId),
    /// One name written in a type declaration's `extends` or `implements`
    /// clause. Separate from `TypeRef` because there can be several and a
    /// caret has to land on the one it is inside.
    Supertype(DeclarationId, SupertypeId),
    BodyNode(BodyId, BodyNodeId),
    Scope(LexicalScopeId),
    Import(usize),
}

/// When the user clicks around, we need to answer one question fast: what is the tightest, most fitting _something_ at this offset?
/// The position index is a fast answer to that question.
#[derive(Debug, Clone, Default)]
pub struct PositionIndex {
    /// Sorted by span start, then end. Spans from one parse are well-nested.
    entries: Vec<(OffsetSpan, EntityId)>,
}

impl PositionIndex {
    pub fn build(file: &File) -> Self {
        let mut entries = Vec::new();

        for (index, declaration) in file.declarations.iter().enumerate() {
            let id = DeclarationId(index);
            entries.push((declaration.span(), EntityId::Declaration(id)));
            if let Some(name_span) = declaration.name_span() {
                entries.push((name_span, EntityId::Declaration(id)));
            }
            if let Some(type_ref) = declaration.type_ref() {
                entries.push((type_ref.span, EntityId::TypeRef(id)));
            }
            if let Declaration::Type(declaration) = declaration {
                for (supertype, type_ref) in declaration.supertypes() {
                    entries.push((type_ref.span, EntityId::Supertype(id, supertype)));
                }
            }
        }

        for (index, scope) in file.lexical_scopes.iter().enumerate() {
            entries.push((scope.span, EntityId::Scope(LexicalScopeId(index))));
        }

        for (index, import) in file.imports.iter().enumerate() {
            entries.push((import.name.span(), EntityId::Import(index)));
        }

        for (body_index, body) in file.bodies.iter().enumerate() {
            let body_id = BodyId(body_index);
            for (index, node) in body.nodes.iter().enumerate() {
                let id = BodyNodeId(index);
                entries.push((node.span, EntityId::BodyNode(body_id, id)));
                // Name segments are the F12 surface of chains: `c` and `a` in
                // `c.a` are separate occurrences of one expression.
                let BodyNodeKind::Expression(expression) = &node.kind else {
                    continue;
                };
                let name_span = match expression {
                    Expression::FieldAccess { name, .. } => Some(name.span),
                    Expression::MethodCall { name, .. } => Some(name.span),
                    Expression::ObjectCreation { ty, .. } => Some(ty.span),
                    _ => None,
                };
                if let Some(span) = name_span {
                    entries.push((span, EntityId::BodyNode(body_id, id)));
                }
            }
        }

        entries.sort_by_key(|(span, _)| (span.start, span.end));
        Self { entries }
    }

    pub fn tightest_containing(&self, offset: Offset) -> Option<(OffsetSpan, EntityId)> {
        self.iter_containing(offset).into_iter().next()
    }

    /// Every entry containing `offset`, tightest first.
    pub fn iter_containing(&self, offset: Offset) -> Vec<(OffsetSpan, EntityId)> {
        let mut containing: Vec<_> = self
            .entries
            .iter()
            .filter(|(span, _)| span.start <= offset && offset <= span.end)
            .copied()
            .collect();
        containing.sort_by_key(|(span, _)| (span.len(), span.start));
        containing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_lexical_scope(file: &mut File, parent: LexicalScopeId) -> LexicalScopeId {
        let scope_id = LexicalScopeId(file.lexical_scopes.len());
        file.lexical_scopes.push(LexicalScope {
            parent: Some(parent),
            owner: None,
            declarations: Vec::new(),
            span: OffsetSpan {
                start: Offset(0),
                end: Offset(0),
            },
        });
        scope_id
    }

    fn identifier(text: &str, start: usize) -> Identifier {
        Identifier {
            text: text.into(),
            span: OffsetSpan {
                start: Offset(start),
                end: Offset(start + text.len()),
            },
        }
    }

    // What every kind of declaration answers about its own name.
    mod declarations {
        use super::*;

        fn type_declaration(
            name: Identifier,
            declaring: LexicalScopeId,
            body: LexicalScopeId,
        ) -> Declaration {
            Declaration::Type(TypeDeclaration {
                span: OffsetSpan {
                    start: Offset(0),
                    end: Offset(20),
                },
                name: Some(name),
                kind: TypeKind::Class,
                access: None,
                superclass: None,
                interfaces: Vec::new(),
                declaring_scope: declaring,
                body_scope: body,
            })
        }

        #[test]
        fn expose_their_names_and_name_spans() {
            let name = identifier("Named", 7);
            let declarations = [
                type_declaration(name.clone(), LexicalScopeId(0), LexicalScopeId(1)),
                Declaration::TypeParameter(TypeParameterDeclaration { name: name.clone() }),
                Declaration::Field(FieldDeclaration {
                    span: OffsetSpan {
                        start: Offset(0),
                        end: Offset(10),
                    },
                    name: Some(name.clone()),
                    access: None,
                    referenced_type: None,
                    declaring_scope: LexicalScopeId(0),
                }),
                Declaration::Method(MethodDeclaration {
                    span: OffsetSpan {
                        start: Offset(0),
                        end: Offset(10),
                    },
                    name: Some(name.clone()),
                    access: None,
                    return_type: None,
                    parameters: Vec::new(),
                    declaring_scope: LexicalScopeId(0),
                    body_scope: LexicalScopeId(1),
                    body: None,
                }),
            ];

            for declaration in declarations {
                assert_eq!(declaration.name(), Some(&name));
                assert_eq!(declaration.name_span(), Some(name.span));
            }

            let constructor = Declaration::Constructor(ConstructorDeclaration {
                span: OffsetSpan {
                    start: Offset(0),
                    end: Offset(10),
                },
                parameters: Vec::new(),
                declaring_scope: LexicalScopeId(0),
                body_scope: LexicalScopeId(1),
                body: None,
            });
            assert_eq!(constructor.name(), None);
            assert_eq!(constructor.name_span(), None);
        }
    }

    // Walking outwards from a scope, which is how a name is looked up.
    mod scope_chain {
        use super::*;

        #[test]
        fn the_compilation_unit_contains_only_itself() {
            let file = File::new();
            let entries: Vec<_> = file.iter_scope_chain(file.compilation_unit_scope).collect();

            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].0, file.compilation_unit_scope);
            assert!(std::ptr::eq(
                entries[0].1,
                &file.lexical_scopes[file.compilation_unit_scope.0]
            ));
        }

        #[test]
        fn walks_from_innermost_to_outermost() {
            let mut file = File::new();
            let compilation_unit = file.compilation_unit_scope;
            let outer = add_lexical_scope(&mut file, compilation_unit);
            let sibling = add_lexical_scope(&mut file, compilation_unit);
            let inner = add_lexical_scope(&mut file, outer);

            let entries: Vec<_> = file.iter_scope_chain(inner).collect();
            let scope_ids: Vec<_> = entries.iter().map(|(scope_id, _)| *scope_id).collect();

            assert_eq!(scope_ids, [inner, outer, compilation_unit]);
            assert!(!scope_ids.contains(&sibling));
            assert!(
                entries.iter().all(|(scope_id, scope)| std::ptr::eq(
                    *scope,
                    &file.lexical_scopes[scope_id.0]
                ))
            );
        }
    }

    // Finding what encloses an offset, which is how a caret becomes an entity.
    mod position_index {
        use super::*;

        #[test]
        fn returns_tightest_first() {
            let mut file = File::new();
            let compilation_unit = file.compilation_unit_scope;
            let outer = add_lexical_scope(&mut file, compilation_unit);
            file.lexical_scopes[outer.0].span = OffsetSpan {
                start: Offset(5),
                end: Offset(50),
            };
            file.lexical_scopes[compilation_unit.0].span = OffsetSpan {
                start: Offset(0),
                end: Offset(100),
            };
            file.declarations.push(Declaration::Local(LocalDeclaration {
                span: OffsetSpan {
                    start: Offset(10),
                    end: Offset(15),
                },
                name: Some(identifier("x", 10)),
                ty: None,
                declaring_scope: outer,
            }));
            file.position_index = PositionIndex::build(&file);

            let entries = file.position_index.iter_containing(Offset(10));
            assert_eq!(
                entries[0],
                (
                    OffsetSpan {
                        start: Offset(10),
                        end: Offset(11),
                    },
                    EntityId::Declaration(DeclarationId(0))
                )
            );
            assert!(entries.iter().any(|(_, entity)| matches!(
                entity,
                EntityId::Scope(scope) if *scope == outer
            )));
        }
    }
}
