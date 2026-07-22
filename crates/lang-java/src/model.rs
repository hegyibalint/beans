use beans_core::model::{Offset, OffsetSpan};

#[derive(Debug, Clone)]
pub struct JavaFile {
    pub package: Option<JavaName>,
    pub imports: Vec<JavaImport>,

    pub declarations: Vec<JavaDeclaration>,
    pub lexical_scopes: Vec<JavaLexicalScope>,
    pub bodies: Vec<JavaBody>,

    pub compilation_unit_scope: JavaLexicalScopeId,
    pub top_level_declarations: Vec<JavaDeclarationId>,

    /// Derived from the rest of the model; rebuilt after parsing.
    pub position_index: JavaPositionIndex,
}

impl JavaFile {
    pub fn new() -> Self {
        Self {
            package: None,
            imports: Vec::new(),
            declarations: Vec::new(),
            lexical_scopes: vec![JavaLexicalScope {
                parent: None,
                owner: None,
                declarations: Vec::new(),
                span: OffsetSpan {
                    start: Offset(0),
                    end: Offset(0),
                },
            }],
            bodies: Vec::new(),
            compilation_unit_scope: JavaLexicalScopeId(0),
            top_level_declarations: Vec::new(),
            position_index: JavaPositionIndex::default(),
        }
    }

    pub fn strip_package<'name>(&self, name: &'name JavaName) -> Option<&'name [JavaIdentifier]> {
        let name_segments = name.segments();
        let Some(package) = &self.package else {
            // In an unnamed package, there is nothing to strip (the prefix is [])
            // We can return the whole name as an identifier
            return Some(name_segments);
        };

        let package_segments = package.segments();
        if name_segments.len() < package_segments.len() {
            // The prefix we want to strip off is longer than what we are stripping from.
            // This makes no sense, we can return nothing
            return None;
        }

        for index in 0..package_segments.len() {
            if name_segments[index].text != package_segments[index].text {
                // The prefix is mismatched, we can give up and return nothing
                return None;
            }
        }

        // If we survived until here, we are sure that the prefix exists
        // We can just trim it off from the name segments, and return it back
        Some(&name_segments[package_segments.len()..])
    }

    pub fn iter_scope_chain<'file>(
        &'file self,
        start: JavaLexicalScopeId,
    ) -> impl Iterator<Item = (JavaLexicalScopeId, &'file JavaLexicalScope)> + 'file {
        std::iter::successors(Some(start), move |scope_id| {
            self.lexical_scopes.get(scope_id.0).unwrap().parent
        })
        .map(move |scope_id| (scope_id, self.lexical_scopes.get(scope_id.0).unwrap()))
    }

    pub fn iter_declaration_chain<'file>(
        &'file self,
        start: JavaLexicalScopeId,
    ) -> impl Iterator<
        Item = (
            JavaLexicalScopeId,
            JavaDeclarationId,
            &'file JavaDeclaration,
        ),
    > + 'file {
        self.iter_scope_chain(start)
            .flat_map(move |(scope_id, scope)| {
                scope.declarations.iter().copied().map(move |decl_id| {
                    (scope_id, decl_id, self.declarations.get(decl_id.0).unwrap())
                })
            })
    }

    pub(crate) fn iter_declarations<'file>(
        &'file self,
        ids: &'file [JavaDeclarationId],
    ) -> impl Iterator<Item = (JavaDeclarationId, &'file JavaDeclaration)> + 'file {
        ids.iter().copied().map(|id| (id, &self.declarations[id.0]))
    }

    /// The nearest type whose body encloses `scope`: what `this` refers to.
    pub fn enclosing_type_declaration(
        &self,
        scope: JavaLexicalScopeId,
    ) -> Option<JavaDeclarationId> {
        self.iter_scope_chain(scope)
            .filter_map(|(_, scope)| scope.owner)
            .find(|owner| matches!(self.declarations[owner.0], JavaDeclaration::Type(_)))
    }

    /// A display name for a declaration: dotted for types (`p.Outer.Inner`),
    /// the bare name for everything else.
    pub fn declaration_label(&self, declaration: JavaDeclarationId) -> Option<String> {
        let name = self.declarations[declaration.0].name()?;
        let JavaDeclaration::Type(_) = self.declarations[declaration.0] else {
            return Some(name.text.clone());
        };

        let mut segments = vec![name.text.clone()];
        let mut declaring = match &self.declarations[declaration.0] {
            JavaDeclaration::Type(declaration) => declaration.declaring_scope,
            _ => unreachable!(),
        };
        while let Some(owner) = self.lexical_scopes[declaring.0].owner {
            let JavaDeclaration::Type(owner_type) = &self.declarations[owner.0] else {
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

impl Default for JavaFile {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JavaDeclarationId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JavaLexicalScopeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JavaBodyId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JavaBodyNodeId(pub usize);

/// JLS 6.1: different entities can share a spelling; resolution filters by this axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaNamespace {
    Type,
    Variable,
    Method,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JavaIdentifier {
    pub text: String,
    pub span: OffsetSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JavaName {
    Simple(JavaIdentifier),
    Qualified(JavaQualifiedName),
}

impl JavaName {
    pub fn segments(&self) -> &[JavaIdentifier] {
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
pub struct JavaQualifiedName {
    segments: Vec<JavaIdentifier>,
    pub span: OffsetSpan,
}

impl JavaQualifiedName {
    pub(crate) fn new(segments: Vec<JavaIdentifier>, span: OffsetSpan) -> Self {
        assert!(
            segments.len() >= 2,
            "a qualified Java name has at least two identifiers"
        );
        Self { segments, span }
    }

    pub fn segments(&self) -> &[JavaIdentifier] {
        &self.segments
    }

    pub fn dotted(&self) -> String {
        dotted(&self.segments)
    }
}

fn dotted(segments: &[JavaIdentifier]) -> String {
    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(".")
}

#[derive(Debug, Clone)]
pub struct JavaImport {
    pub name: JavaName,
    pub kind: JavaImportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaImportKind {
    Type,           // import a.b.C;
    TypeOnDemand,   // import a.b.*;
    Static,         // import static a.b.C.member;
    StaticOnDemand, // import static a.b.C.*;
    Module,         // import module a.b;  (Java 25)
}

#[derive(Debug, Clone)]
pub enum JavaDeclaration {
    Type(JavaTypeDeclaration),
    TypeParameter(JavaTypeParameterDeclaration),
    Field(JavaFieldDeclaration),
    Constructor(JavaConstructorDeclaration),
    Method(JavaMethodDeclaration),
    Parameter(JavaParameterDeclaration),
    Local(JavaLocalDeclaration),
}

impl JavaDeclaration {
    pub fn name(&self) -> Option<&JavaIdentifier> {
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

    pub fn namespace(&self) -> JavaNamespace {
        match self {
            Self::Type(_) | Self::TypeParameter(_) => JavaNamespace::Type,
            Self::Field(_) | Self::Parameter(_) | Self::Local(_) => JavaNamespace::Variable,
            Self::Constructor(_) | Self::Method(_) => JavaNamespace::Method,
        }
    }

    pub fn declaring_scope(&self) -> JavaLexicalScopeId {
        match self {
            Self::Type(declaration) => declaration.declaring_scope,
            // Type parameters are not parsed yet; when they are, they must
            // carry the scope of their generic declaration.
            Self::TypeParameter(_) => JavaLexicalScopeId(0),
            Self::Field(declaration) => declaration.declaring_scope,
            Self::Constructor(declaration) => declaration.declaring_scope,
            Self::Method(declaration) => declaration.declaring_scope,
            Self::Parameter(declaration) => declaration.declaring_scope,
            Self::Local(declaration) => declaration.declaring_scope,
        }
    }

    /// The type annotation owned by this declaration, if any.
    pub fn type_ref(&self) -> Option<&JavaTypeRef> {
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
pub struct JavaTypeRef {
    pub span: OffsetSpan,
    /// The erased head of the type: `List` for `List<String>`, `String` for `String[]`.
    pub name: JavaName,
    /// Primitives and `void` never resolve to declarations.
    pub primitive: bool,
}

#[derive(Debug, Clone)]
pub struct JavaTypeDeclaration {
    pub span: OffsetSpan,
    pub name: Option<JavaIdentifier>,
    pub kind: JavaTypeKind,
    pub superclass: Option<JavaTypeRef>,
    pub declaring_scope: JavaLexicalScopeId,
    pub body_scope: JavaLexicalScopeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaTypeKind {
    Class,
    Interface,
    Enum,
    Record,
    AnnotationInterface,
}

#[derive(Debug, Clone)]
pub struct JavaTypeParameterDeclaration {
    /// Total: the name is the entire payload of a type parameter today.
    pub name: JavaIdentifier,
}

#[derive(Debug, Clone)]
pub struct JavaFieldDeclaration {
    pub span: OffsetSpan,
    pub name: Option<JavaIdentifier>,
    pub referenced_type: Option<JavaTypeRef>,
    pub declaring_scope: JavaLexicalScopeId,
}

#[derive(Debug, Clone)]
pub struct JavaConstructorDeclaration {
    pub span: OffsetSpan,
    pub parameters: Vec<JavaDeclarationId>,
    pub declaring_scope: JavaLexicalScopeId,
    pub body_scope: JavaLexicalScopeId,
    pub body: Option<JavaBodyId>,
}

#[derive(Debug, Clone)]
pub struct JavaMethodDeclaration {
    pub span: OffsetSpan,
    pub name: Option<JavaIdentifier>,
    pub return_type: Option<JavaTypeRef>,
    pub parameters: Vec<JavaDeclarationId>,
    pub declaring_scope: JavaLexicalScopeId,
    pub body_scope: JavaLexicalScopeId,
    pub body: Option<JavaBodyId>,
}

#[derive(Debug, Clone)]
pub struct JavaParameterDeclaration {
    pub span: OffsetSpan,
    pub name: Option<JavaIdentifier>,
    pub ty: Option<JavaTypeRef>,
    pub declaring_scope: JavaLexicalScopeId,
}

#[derive(Debug, Clone)]
pub struct JavaLocalDeclaration {
    pub span: OffsetSpan,
    pub name: Option<JavaIdentifier>,
    pub ty: Option<JavaTypeRef>,
    pub declaring_scope: JavaLexicalScopeId,
}

#[derive(Debug, Clone)]
pub struct JavaLexicalScope {
    pub parent: Option<JavaLexicalScopeId>,
    /// The declaration that introduced this scope (type bodies, methods), if any.
    pub owner: Option<JavaDeclarationId>,
    pub declarations: Vec<JavaDeclarationId>,
    pub span: OffsetSpan,
}

/// The executable content of a method, constructor, or initializer.
/// Statements and expressions share one arena; span and enclosing scope
/// are inline on every node.
#[derive(Debug, Clone)]
pub struct JavaBody {
    /// The scope of the root block.
    pub scope: JavaLexicalScopeId,
    pub root: JavaBodyNodeId,
    pub nodes: Vec<JavaBodyNode>,
}

impl JavaBody {
    pub fn node(&self, id: JavaBodyNodeId) -> &JavaBodyNode {
        &self.nodes[id.0]
    }

    pub fn expression(&self, id: JavaBodyNodeId) -> Option<&JavaExpression> {
        match &self.node(id).kind {
            JavaBodyNodeKind::Expression(expression) => Some(expression),
            _ => None,
        }
    }
}

/// OffsetSpan and scope are stamped at extraction: the parser knows both, and
/// later queries should never re-derive what was free at parse time.
#[derive(Debug, Clone)]
pub struct JavaBodyNode {
    pub span: OffsetSpan,
    /// The scope this node lives in. Note a `Block` *introduces* a deeper
    /// scope (its payload), but is stamped with the scope it lives in.
    pub scope: JavaLexicalScopeId,
    pub kind: JavaBodyNodeKind,
}

#[derive(Debug, Clone)]
pub enum JavaBodyNodeKind {
    Statement(JavaStatement),
    Expression(JavaExpression),
}

#[derive(Debug, Clone)]
pub enum JavaStatement {
    Block {
        scope: JavaLexicalScopeId,
        statements: Vec<JavaBodyNodeId>,
    },
    TypeDeclaration(JavaDeclarationId),
    LocalDeclaration {
        declaration: JavaDeclarationId,
        initializer: Option<JavaBodyNodeId>,
    },
    Expression(JavaBodyNodeId),
    Return(Option<JavaBodyNodeId>),
}

#[derive(Debug, Clone)]
pub enum JavaExpression {
    NameRef {
        name: JavaIdentifier,
    },
    FieldAccess {
        receiver: JavaBodyNodeId,
        name: JavaIdentifier,
    },
    MethodCall {
        /// `None` is the implicit `this` receiver.
        receiver: Option<JavaBodyNodeId>,
        name: JavaIdentifier,
        arguments: Vec<JavaBodyNodeId>,
    },
    ObjectCreation {
        ty: JavaTypeRef,
        arguments: Vec<JavaBodyNodeId>,
    },
    This,
    Assign {
        target: JavaBodyNodeId,
        value: JavaBodyNodeId,
    },
    Literal,
}

/// Anything the position index can hand back for an offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JavaEntityId {
    Declaration(JavaDeclarationId),
    /// The type annotation owned by a declaration (field/parameter/local type,
    /// method return type, superclass).
    TypeRef(JavaDeclarationId),
    BodyNode(JavaBodyId, JavaBodyNodeId),
    Scope(JavaLexicalScopeId),
    Import(usize),
}

/// Per-file position index: the persistent skeleton of the syntax tree.
/// Derived data, rebuilt from the model after every parse.
#[derive(Debug, Clone, Default)]
pub struct JavaPositionIndex {
    /// Sorted by span start, then end. Spans from one parse are well-nested.
    entries: Vec<(OffsetSpan, JavaEntityId)>,
}

impl JavaPositionIndex {
    pub fn build(file: &JavaFile) -> Self {
        let mut entries = Vec::new();

        for (index, declaration) in file.declarations.iter().enumerate() {
            let id = JavaDeclarationId(index);
            entries.push((declaration.span(), JavaEntityId::Declaration(id)));
            if let Some(name_span) = declaration.name_span() {
                entries.push((name_span, JavaEntityId::Declaration(id)));
            }
            if let Some(type_ref) = declaration.type_ref() {
                entries.push((type_ref.span, JavaEntityId::TypeRef(id)));
            }
        }

        for (index, scope) in file.lexical_scopes.iter().enumerate() {
            entries.push((scope.span, JavaEntityId::Scope(JavaLexicalScopeId(index))));
        }

        for (index, import) in file.imports.iter().enumerate() {
            entries.push((import.name.span(), JavaEntityId::Import(index)));
        }

        for (body_index, body) in file.bodies.iter().enumerate() {
            let body_id = JavaBodyId(body_index);
            for (index, node) in body.nodes.iter().enumerate() {
                let id = JavaBodyNodeId(index);
                entries.push((node.span, JavaEntityId::BodyNode(body_id, id)));
                // Name segments are the F12 surface of chains: `c` and `a` in
                // `c.a` are separate occurrences of one expression.
                let JavaBodyNodeKind::Expression(expression) = &node.kind else {
                    continue;
                };
                let name_span = match expression {
                    JavaExpression::FieldAccess { name, .. } => Some(name.span),
                    JavaExpression::MethodCall { name, .. } => Some(name.span),
                    JavaExpression::ObjectCreation { ty, .. } => Some(ty.span),
                    _ => None,
                };
                if let Some(span) = name_span {
                    entries.push((span, JavaEntityId::BodyNode(body_id, id)));
                }
            }
        }

        entries.sort_by_key(|(span, _)| (span.start, span.end));
        Self { entries }
    }

    pub fn tightest_containing(&self, offset: Offset) -> Option<(OffsetSpan, JavaEntityId)> {
        self.iter_containing(offset).into_iter().next()
    }

    /// Every entry containing `offset`, tightest first.
    pub fn iter_containing(&self, offset: Offset) -> Vec<(OffsetSpan, JavaEntityId)> {
        let mut containing: Vec<_> = self
            .entries
            .iter()
            .filter(|(span, _)| span.start <= offset && offset < span.end)
            .copied()
            .collect();
        containing.sort_by_key(|(span, _)| (span.len(), span.start));
        containing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_lexical_scope(file: &mut JavaFile, parent: JavaLexicalScopeId) -> JavaLexicalScopeId {
        let scope_id = JavaLexicalScopeId(file.lexical_scopes.len());
        file.lexical_scopes.push(JavaLexicalScope {
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

    fn identifier(text: &str, start: usize) -> JavaIdentifier {
        JavaIdentifier {
            text: text.into(),
            span: OffsetSpan {
                start: Offset(start),
                end: Offset(start + text.len()),
            },
        }
    }

    fn type_declaration(
        name: JavaIdentifier,
        declaring: JavaLexicalScopeId,
        body: JavaLexicalScopeId,
    ) -> JavaDeclaration {
        JavaDeclaration::Type(JavaTypeDeclaration {
            span: OffsetSpan {
                start: Offset(0),
                end: Offset(20),
            },
            name: Some(name),
            kind: JavaTypeKind::Class,
            superclass: None,
            declaring_scope: declaring,
            body_scope: body,
        })
    }

    #[test]
    fn declarations_expose_their_names_and_name_spans() {
        let name = identifier("Named", 7);
        let declarations = [
            type_declaration(name.clone(), JavaLexicalScopeId(0), JavaLexicalScopeId(1)),
            JavaDeclaration::TypeParameter(JavaTypeParameterDeclaration { name: name.clone() }),
            JavaDeclaration::Field(JavaFieldDeclaration {
                span: OffsetSpan {
                    start: Offset(0),
                    end: Offset(10),
                },
                name: Some(name.clone()),
                referenced_type: None,
                declaring_scope: JavaLexicalScopeId(0),
            }),
            JavaDeclaration::Method(JavaMethodDeclaration {
                span: OffsetSpan {
                    start: Offset(0),
                    end: Offset(10),
                },
                name: Some(name.clone()),
                return_type: None,
                parameters: Vec::new(),
                declaring_scope: JavaLexicalScopeId(0),
                body_scope: JavaLexicalScopeId(1),
                body: None,
            }),
        ];

        for declaration in declarations {
            assert_eq!(declaration.name(), Some(&name));
            assert_eq!(declaration.name_span(), Some(name.span));
        }

        let constructor = JavaDeclaration::Constructor(JavaConstructorDeclaration {
            span: OffsetSpan {
                start: Offset(0),
                end: Offset(10),
            },
            parameters: Vec::new(),
            declaring_scope: JavaLexicalScopeId(0),
            body_scope: JavaLexicalScopeId(1),
            body: None,
        });
        assert_eq!(constructor.name(), None);
        assert_eq!(constructor.name_span(), None);
    }

    #[test]
    fn strip_package_returns_the_type_name_segments() {
        let mut file = JavaFile::new();
        file.package = Some(JavaName::Simple(identifier("p", 0)));
        let name = JavaName::Qualified(JavaQualifiedName::new(
            vec![
                identifier("p", 10),
                identifier("Outer", 12),
                identifier("Inner", 18),
            ],
            OffsetSpan {
                start: Offset(10),
                end: Offset(23),
            },
        ));

        let type_segments = file.strip_package(&name).unwrap();
        let type_names: Vec<_> = type_segments
            .iter()
            .map(|identifier| identifier.text.as_str())
            .collect();

        assert_eq!(type_names, ["Outer", "Inner"]);
    }

    #[test]
    fn strip_package_rejects_a_different_package() {
        let mut file = JavaFile::new();
        file.package = Some(JavaName::Simple(identifier("p", 0)));
        let name = JavaName::Qualified(JavaQualifiedName::new(
            vec![identifier("q", 10), identifier("Outer", 12)],
            OffsetSpan {
                start: Offset(10),
                end: Offset(17),
            },
        ));

        assert_eq!(file.strip_package(&name), None);
    }

    #[test]
    fn strip_package_preserves_the_whole_name_in_the_default_package() {
        let file = JavaFile::new();
        let name = JavaName::Simple(identifier("Outer", 0));

        assert_eq!(file.strip_package(&name), Some(name.segments()));
    }

    #[test]
    fn iter_scope_chain_from_the_compilation_unit_contains_only_itself() {
        let file = JavaFile::new();
        let entries: Vec<_> = file.iter_scope_chain(file.compilation_unit_scope).collect();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, file.compilation_unit_scope);
        assert!(std::ptr::eq(
            entries[0].1,
            &file.lexical_scopes[file.compilation_unit_scope.0]
        ));
    }

    #[test]
    fn iter_scope_chain_walks_from_innermost_to_outermost() {
        let mut file = JavaFile::new();
        let compilation_unit = file.compilation_unit_scope;
        let outer = add_lexical_scope(&mut file, compilation_unit);
        let sibling = add_lexical_scope(&mut file, compilation_unit);
        let inner = add_lexical_scope(&mut file, outer);

        let entries: Vec<_> = file.iter_scope_chain(inner).collect();
        let scope_ids: Vec<_> = entries.iter().map(|(scope_id, _)| *scope_id).collect();

        assert_eq!(scope_ids, [inner, outer, compilation_unit]);
        assert!(!scope_ids.contains(&sibling));
        assert!(
            entries
                .iter()
                .all(|(scope_id, scope)| std::ptr::eq(*scope, &file.lexical_scopes[scope_id.0]))
        );
    }

    #[test]
    fn position_index_returns_tightest_first() {
        let mut file = JavaFile::new();
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
        file.declarations
            .push(JavaDeclaration::Local(JavaLocalDeclaration {
                span: OffsetSpan {
                    start: Offset(10),
                    end: Offset(15),
                },
                name: Some(identifier("x", 10)),
                ty: None,
                declaring_scope: outer,
            }));
        file.position_index = JavaPositionIndex::build(&file);

        let entries = file.position_index.iter_containing(Offset(10));
        assert_eq!(
            entries[0],
            (
                OffsetSpan {
                    start: Offset(10),
                    end: Offset(11),
                },
                JavaEntityId::Declaration(JavaDeclarationId(0))
            )
        );
        assert!(entries.iter().any(|(_, entity)| matches!(
            entity,
            JavaEntityId::Scope(scope) if *scope == outer
        )));
    }
}
