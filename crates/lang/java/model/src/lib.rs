use crate::scopes::ScopeIndex;

pub mod declarations;
pub mod imports;
pub mod references;
pub mod scopes;

/// Represents a whole `.java` file.
#[derive(Debug)]
pub struct File {
    /// The package name, if exists
    pub package_name: Option<references::NameRef>,
    pub imports: Vec<imports::Import>,

    declarations: Vec<declarations::Declaration>,
    scopes: Vec<scopes::Scope>,
}

#[derive(Debug, Clone, Copy)]
pub struct ScopedDeclaration<'a> {
    pub scope: scopes::IndexedScope<'a>,
    pub declaration: declarations::IndexedDeclaration<'a>,
}

impl File {
    pub const ROOT_SCOPE_ID: ScopeIndex = ScopeIndex::new(0);

    pub fn new() -> File {
        Self {
            package_name: None,
            imports: Vec::new(),

            declarations: Vec::new(),
            scopes: vec![
                // In scopes, we create the 0th element, the root scope
                scopes::Scope::new(None),
            ],
        }
    }

    pub fn scope(&self, index: ScopeIndex) -> Option<&scopes::Scope> {
        self.scopes.get(index.as_usize())
    }

    pub fn declaration(
        &self,
        index: declarations::DeclarationIndex,
    ) -> Option<&declarations::Declaration> {
        self.declarations.get(index.as_usize())
    }

    pub fn iter_scopes(&self) -> impl Iterator<Item = scopes::IndexedScope<'_>> + '_ {
        self.scopes
            .iter()
            .enumerate()
            .map(|(index, scope)| scopes::IndexedScope {
                index: ScopeIndex::new(index),
                scope,
            })
    }

    pub fn iter_declarations(&self) -> impl Iterator<Item = ScopedDeclaration<'_>> + '_ {
        self.iter_scopes().flat_map(move |scope| {
            scope
                .scope
                .iter_declarations(self)
                .map(move |declaration| ScopedDeclaration { scope, declaration })
        })
    }

    pub fn add_declaration(
        &mut self,
        scope: ScopeIndex,
        declaration: declarations::Declaration,
    ) -> declarations::DeclarationIndex {
        let scope_index = scope.as_usize();
        assert!(scope_index < self.scopes.len(), "invalid scope index");

        let declaration_index = declarations::DeclarationIndex::new(self.declarations.len());
        self.declarations.push(declaration);
        self.scopes[scope_index].add_declaration(declaration_index);
        declaration_index
    }

    pub fn new_scope(&mut self, parent_scope: ScopeIndex) -> scopes::ScopeIndex {
        let parent_index = parent_scope.as_usize();
        assert!(
            parent_index < self.scopes.len(),
            "invalid parent scope index"
        );

        let index = ScopeIndex::new(self.scopes.len());
        self.scopes.push(scopes::Scope::new(Some(parent_scope)));
        self.scopes[parent_index].add_child_scope(index);
        index
    }
}

#[cfg(test)]
mod tests {
    use super::{
        File,
        declarations::{
            Declaration,
            types::{Kind, TypeDeclaration},
        },
        scopes::ScopeIndex,
    };

    #[test]
    fn new_scope_links_parent_and_child() {
        let mut file = File::new();

        let child = file.new_scope(File::ROOT_SCOPE_ID);

        assert_eq!(
            file.scope(child).unwrap().parent_scope(),
            Some(File::ROOT_SCOPE_ID)
        );
        assert_eq!(
            file.scope(File::ROOT_SCOPE_ID).unwrap().child_scopes(),
            [child]
        );
    }

    #[test]
    #[should_panic(expected = "invalid parent scope index")]
    fn new_scope_rejects_an_unknown_parent() {
        let mut file = File::new();

        file.new_scope(ScopeIndex::new(10));
    }

    #[test]
    fn add_declaration_registers_it_with_its_scope() {
        let mut file = File::new();
        let declaration = Declaration::Type(TypeDeclaration::new(Kind::Class));

        let declaration = file.add_declaration(File::ROOT_SCOPE_ID, declaration);

        assert!(file.declaration(declaration).is_some());
        assert_eq!(
            file.scope(File::ROOT_SCOPE_ID)
                .unwrap()
                .iter_declarations(&file)
                .map(|indexed_declaration| indexed_declaration.index)
                .collect::<Vec<_>>(),
            [declaration]
        );
    }
}
