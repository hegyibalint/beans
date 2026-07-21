use beans_core::model::Span;

#[derive(Debug, Clone)]
pub struct JavaFile {
    pub package: Option<JavaName>,
    pub imports: Vec<JavaImport>,

    /// Declarations that appear in this file, including types, fields, methods, and parameters.
    /// Indexed by their order of appearance in the file, newtyped as `JavaDeclarationId`.
    pub declarations: Vec<JavaDeclaration>,
    /// Lexical scopes that appear in this file, including the compilation unit scope and nested scopes.
    /// Indexed by their order of appearance in the file, newtyped as `JavaLexicalScopeId`.
    /// The first lexical scope is always the compilation unit scope.
    pub lexical_scopes: Vec<JavaLexicalScope>,

    pub compilation_unit_scope: JavaLexicalScopeId,
    pub top_level_declarations: Vec<JavaDeclarationId>,
}

impl JavaFile {
    pub fn new() -> Self {
        Self {
            package: None,
            imports: Vec::new(),
            declarations: Vec::new(),
            lexical_scopes: vec![JavaLexicalScope {
                parent: None,
                declarations: Vec::new(),
            }],
            compilation_unit_scope: JavaLexicalScopeId(0),
            top_level_declarations: Vec::new(),
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

    pub fn lexical_scope_chain<'file>(
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
        self.lexical_scope_chain(start)
            .flat_map(move |(scope_id, scope)| {
                scope.declarations.iter().copied().map(move |decl_id| {
                    (scope_id, decl_id, self.declarations.get(decl_id.0).unwrap())
                })
            })
    }

    pub(crate) fn closest_declaration(
        &self,
        offset: usize,
    ) -> Option<(JavaDeclarationId, &JavaDeclaration)> {
        self.declarations
            .iter()
            .enumerate()
            .filter_map(|(index, declaration)| {
                let span = declaration.span()?;
                if span.start <= offset && offset < span.end {
                    Some((JavaDeclarationId(index), declaration, span.end - span.start))
                } else {
                    None
                }
            })
            .min_by_key(|(_, _, length)| *length)
            .map(|(id, declaration, _)| (id, declaration))
    }

    pub(crate) fn iter_declarations<'file>(
        &'file self,
        ids: &'file [JavaDeclarationId],
    ) -> impl Iterator<Item = (JavaDeclarationId, &'file JavaDeclaration)> + 'file {
        ids.iter().copied().map(|id| (id, &self.declarations[id.0]))
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JavaIdentifier {
    pub text: String,
    pub span: Span,
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

    pub fn span(&self) -> Span {
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
    pub span: Span,
}

impl JavaQualifiedName {
    pub(crate) fn new(segments: Vec<JavaIdentifier>, span: Span) -> Self {
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
}

impl JavaDeclaration {
    pub fn name(&self) -> Option<&JavaIdentifier> {
        match self {
            Self::Type(declaration) => declaration.name.as_ref(),
            Self::TypeParameter(declaration) => declaration.name.as_ref(),
            Self::Field(declaration) => declaration.name.as_ref(),
            Self::Constructor(_) => None,
            Self::Method(declaration) => declaration.name.as_ref(),
        }
    }

    pub fn name_span(&self) -> Option<Span> {
        self.name().map(|name| name.span)
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Type(declaration) => Some(declaration.span),
            _ => self.name_span(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JavaTypeDeclaration {
    pub span: Span,
    pub name: Option<JavaIdentifier>,
    pub kind: JavaTypeKind,
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
    pub name: Option<JavaIdentifier>,
}

#[derive(Debug, Clone)]
pub struct JavaFieldDeclaration {
    pub name: Option<JavaIdentifier>,
}

#[derive(Debug, Clone)]
pub struct JavaConstructorDeclaration {}

#[derive(Debug, Clone)]
pub struct JavaMethodDeclaration {
    pub name: Option<JavaIdentifier>,
}

#[derive(Debug, Clone)]
pub struct JavaLexicalScope {
    pub parent: Option<JavaLexicalScopeId>,
    pub declarations: Vec<JavaDeclarationId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_lexical_scope(file: &mut JavaFile, parent: JavaLexicalScopeId) -> JavaLexicalScopeId {
        let scope_id = JavaLexicalScopeId(file.lexical_scopes.len());
        file.lexical_scopes.push(JavaLexicalScope {
            parent: Some(parent),
            declarations: Vec::new(),
        });
        scope_id
    }

    fn identifier(text: &str, start: usize) -> JavaIdentifier {
        JavaIdentifier {
            text: text.into(),
            span: Span {
                start,
                end: start + text.len(),
            },
        }
    }

    #[test]
    fn declarations_expose_their_names_and_name_spans() {
        let name = identifier("Named", 7);
        let declarations = [
            JavaDeclaration::Type(JavaTypeDeclaration {
                span: Span { start: 0, end: 20 },
                name: Some(name.clone()),
                kind: JavaTypeKind::Class,
                declaring_scope: JavaLexicalScopeId(0),
                body_scope: JavaLexicalScopeId(1),
            }),
            JavaDeclaration::TypeParameter(JavaTypeParameterDeclaration {
                name: Some(name.clone()),
            }),
            JavaDeclaration::Field(JavaFieldDeclaration {
                name: Some(name.clone()),
            }),
            JavaDeclaration::Method(JavaMethodDeclaration {
                name: Some(name.clone()),
            }),
        ];

        for declaration in declarations {
            assert_eq!(declaration.name(), Some(&name));
            assert_eq!(declaration.name_span(), Some(name.span));
        }

        let constructor = JavaDeclaration::Constructor(JavaConstructorDeclaration {});
        assert_eq!(constructor.name(), None);
        assert_eq!(constructor.name_span(), None);
    }

    #[test]
    fn finds_the_tightest_declaration_at_an_offset() {
        let mut file = JavaFile::new();
        file.declarations
            .push(JavaDeclaration::Method(JavaMethodDeclaration {
                name: Some(identifier("outer", 7)),
            }));
        file.declarations
            .push(JavaDeclaration::Method(JavaMethodDeclaration {
                name: Some(identifier("in", 8)),
            }));

        assert_eq!(
            file.closest_declaration(8).map(|(id, _)| id),
            Some(JavaDeclarationId(1))
        );
        assert!(file.closest_declaration(12).is_none());
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
            Span { start: 10, end: 23 },
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
            Span { start: 10, end: 17 },
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
    fn lexical_scope_chain_from_the_compilation_unit_contains_only_itself() {
        let file = JavaFile::new();
        let entries: Vec<_> = file
            .lexical_scope_chain(file.compilation_unit_scope)
            .collect();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, file.compilation_unit_scope);
        assert!(std::ptr::eq(
            entries[0].1,
            &file.lexical_scopes[file.compilation_unit_scope.0]
        ));
    }

    #[test]
    fn lexical_scope_chain_walks_from_innermost_to_outermost() {
        let mut file = JavaFile::new();
        let compilation_unit = file.compilation_unit_scope;
        let outer = add_lexical_scope(&mut file, compilation_unit);
        let sibling = add_lexical_scope(&mut file, compilation_unit);
        let inner = add_lexical_scope(&mut file, outer);

        let entries: Vec<_> = file.lexical_scope_chain(inner).collect();
        let scope_ids: Vec<_> = entries.iter().map(|(scope_id, _)| *scope_id).collect();

        assert_eq!(scope_ids, [inner, outer, compilation_unit]);
        assert!(!scope_ids.contains(&sibling));
        assert!(
            entries
                .iter()
                .all(|(scope_id, scope)| std::ptr::eq(*scope, &file.lexical_scopes[scope_id.0]))
        );
    }
}
