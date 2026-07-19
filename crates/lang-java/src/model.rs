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
    pub top_level_types: Vec<JavaDeclarationId>,
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
            top_level_types: Vec::new(),
        }
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

#[derive(Debug, Clone)]
pub struct JavaTypeDeclaration {
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
