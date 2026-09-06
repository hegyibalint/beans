use crate::scopes::{ScopeIndex, ScopeKind};

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
pub struct ScopeEntry<'a> {
    pub scope_index: ScopeIndex,
    pub scope: &'a scopes::Scope,
}

#[derive(Debug, Clone, Copy)]
pub struct DeclarationEntry<'a> {
    pub scope_index: ScopeIndex,
    pub scope: &'a scopes::Scope,
    pub declaration_index: declarations::DeclarationIndex,
    pub declaration: &'a declarations::Declaration,
}

impl File {
    pub const ROOT_SCOPE_ID: ScopeIndex = ScopeIndex::new(0);

    pub fn new() -> File {
        Self {
            package_name: None,
            imports: Vec::new(),

            declarations: Vec::new(),
            scopes: vec![scopes::Scope::new(ScopeKind::CompilationUnit, None)],
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

    pub fn iter_scopes(&self) -> impl Iterator<Item = ScopeEntry<'_>> + '_ {
        self.scopes.iter().enumerate().map(|entry| ScopeEntry {
            scope_index: ScopeIndex::new(entry.0),
            scope: entry.1,
        })
    }

    pub fn iter_declarations(&self) -> impl Iterator<Item = DeclarationEntry<'_>> + '_ {
        self.iter_scopes()
            .flat_map(move |entry| self.iter_declarations_in(entry.scope_index))
    }

    pub fn iter_declarations_in(
        &self,
        scope_index: ScopeIndex,
    ) -> impl Iterator<Item = DeclarationEntry<'_>> + '_ {
        let scope = self.scope(scope_index).expect("invalid scope index");
        scope
            .iter_declaration_indices()
            .map(move |entry| DeclarationEntry {
                scope_index,
                scope,
                declaration_index: entry,
                declaration: self
                    .declaration(entry)
                    .expect("scope contains an invalid declaration index"),
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

    pub fn new_child_scope(
        &mut self,
        parent_scope: ScopeIndex,
        kind: ScopeKind,
    ) -> scopes::ScopeIndex {
        let parent_index = parent_scope.as_usize();
        assert!(
            parent_index < self.scopes.len(),
            "invalid parent scope index"
        );

        let index = ScopeIndex::new(self.scopes.len());
        self.scopes
            .push(scopes::Scope::new(kind, Some(parent_scope)));
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
        scopes::{ScopeIndex, ScopeKind},
    };

    #[test]
    fn new_child_scope_links_parent_child_and_owner() {
        let mut file = File::new();
        let declaration = file.add_declaration(
            File::ROOT_SCOPE_ID,
            Declaration::Type(TypeDeclaration::new(Kind::Class)),
        );

        let child = file.new_child_scope(
            File::ROOT_SCOPE_ID,
            ScopeKind::TypeBody { owner: declaration },
        );

        assert_eq!(
            file.scope(child).unwrap().parent_scope(),
            Some(File::ROOT_SCOPE_ID)
        );
        assert_eq!(
            file.scope(child).unwrap().kind(),
            ScopeKind::TypeBody { owner: declaration }
        );
        assert_eq!(
            file.scope(File::ROOT_SCOPE_ID)
                .unwrap()
                .iter_child_scopes()
                .collect::<Vec<_>>(),
            [child]
        );
    }

    #[test]
    #[should_panic(expected = "invalid parent scope index")]
    fn new_child_scope_rejects_an_unknown_parent() {
        let mut file = File::new();
        let declaration = file.add_declaration(
            File::ROOT_SCOPE_ID,
            Declaration::Type(TypeDeclaration::new(Kind::Class)),
        );

        file.new_child_scope(
            ScopeIndex::new(10),
            ScopeKind::TypeBody { owner: declaration },
        );
    }

    #[test]
    fn scoped_iteration_exposes_only_the_selected_scopes_declarations() {
        let mut file = File::new();
        let owner = file.add_declaration(
            File::ROOT_SCOPE_ID,
            Declaration::Type(TypeDeclaration::new(Kind::Class)),
        );
        let body = file.new_child_scope(File::ROOT_SCOPE_ID, ScopeKind::TypeBody { owner });
        let member =
            file.add_declaration(body, Declaration::Type(TypeDeclaration::new(Kind::Class)));

        let mut entries = file.iter_declarations_in(body);
        let entry = entries.next().expect("expected the member declaration");
        assert!(entries.next().is_none());
        assert_eq!(entry.scope_index, body);
        assert_eq!(entry.declaration_index, member);
        assert!(std::ptr::eq(entry.scope, file.scope(body).unwrap()));
        assert!(std::ptr::eq(
            entry.declaration,
            file.declaration(member).unwrap()
        ));

        assert_eq!(
            file.iter_declarations()
                .map(|entry| (entry.scope_index, entry.declaration_index))
                .collect::<Vec<_>>(),
            [(File::ROOT_SCOPE_ID, owner), (body, member)]
        );
        for entry in file.iter_scopes() {
            assert!(std::ptr::eq(
                entry.scope,
                file.scope(entry.scope_index).unwrap()
            ));
        }
    }

    #[test]
    #[should_panic(expected = "invalid scope index")]
    fn scoped_iteration_rejects_an_unknown_scope() {
        let file = File::new();
        let _ = file.iter_declarations_in(ScopeIndex::new(10));
    }

    #[test]
    fn add_declaration_registers_it_with_its_scope() {
        let mut file = File::new();
        let declaration = Declaration::Type(TypeDeclaration::new(Kind::Class));

        let declaration = file.add_declaration(File::ROOT_SCOPE_ID, declaration);

        let mut entries = file.iter_declarations();
        let entry = entries.next().expect("expected one declaration");
        assert!(entries.next().is_none());
        assert_eq!(entry.scope_index, File::ROOT_SCOPE_ID);
        assert_eq!(entry.declaration_index, declaration);
        assert!(std::ptr::eq(
            entry.scope,
            file.scope(entry.scope_index).unwrap()
        ));
        assert!(std::ptr::eq(
            entry.declaration,
            file.declaration(entry.declaration_index).unwrap()
        ));
        assert_eq!(
            file.iter_declarations_in(File::ROOT_SCOPE_ID)
                .map(|entry| entry.declaration_index)
                .collect::<Vec<_>>(),
            [declaration]
        );
    }
}
