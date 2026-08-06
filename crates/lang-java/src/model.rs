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

    /// The type annotation owned by this declaration, if any.
    pub fn type_ref(&self) -> Option<&TypeRef> {
        match self {
            Self::Field(declaration) => declaration.referenced_type.as_ref(),
            Self::Method(declaration) => declaration.return_type.as_ref(),
            Self::Parameter(declaration) => declaration.ty.as_ref(),
            Self::Local(declaration) => declaration.ty.as_ref(),
            Self::Type(declaration) => declaration.superclass.as_ref(),
            _ => None,
        }
    }
}

/// A type as written in source. Resolution against the model happens later.
#[derive(Debug, Clone)]
pub struct TypeRef {
    pub span: OffsetSpan,
    /// The erased head of the type: `List` for `List<String>`, `String` for `String[]`.
    pub name: Name,
    /// Primitives and `void` never resolve to declarations.
    pub primitive: bool,
}

#[derive(Debug, Clone)]
pub struct TypeDeclaration {
    pub span: OffsetSpan,
    pub name: Option<Identifier>,
    pub kind: TypeKind,
    /// `None` where §8.1.1 says access control does not apply: local and
    /// anonymous classes.
    pub access: Option<Access>,
    pub superclass: Option<TypeRef>,
    pub declaring_scope: LexicalScopeId,
    pub body_scope: LexicalScopeId,
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
    /// method return type, superclass).
    TypeRef(DeclarationId),
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
