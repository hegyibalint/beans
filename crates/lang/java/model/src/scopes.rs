use crate::{File, declarations};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeIndex(usize);

#[derive(Debug, Clone, Copy)]
pub struct IndexedScope<'a> {
    pub index: ScopeIndex,
    pub scope: &'a Scope,
}

impl ScopeIndex {
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) const fn as_usize(self) -> usize {
        self.0
    }
}

/// Scopes hold what at a given location (or technically span) we have access to.
#[derive(Debug)]
pub struct Scope {
    parent_scope: Option<ScopeIndex>,
    child_scopes: Vec<ScopeIndex>,
    declarations: Vec<declarations::DeclarationIndex>,
}

impl Scope {
    pub(crate) fn new(parent_scope: Option<ScopeIndex>) -> Self {
        Self {
            parent_scope,
            child_scopes: Vec::new(),
            declarations: Vec::new(),
        }
    }

    pub fn parent_scope(&self) -> Option<ScopeIndex> {
        self.parent_scope
    }

    pub(crate) fn add_child_scope(&mut self, scope: ScopeIndex) {
        self.child_scopes.push(scope);
    }

    pub fn child_scopes(&self) -> &[ScopeIndex] {
        &self.child_scopes
    }

    pub(crate) fn add_declaration(&mut self, declaration: declarations::DeclarationIndex) {
        self.declarations.push(declaration);
    }

    pub fn iter_declarations<'a>(
        &'a self,
        file: &'a File,
    ) -> impl Iterator<Item = declarations::IndexedDeclaration<'a>> + 'a {
        self.declarations
            .iter()
            .copied()
            .map(move |index| declarations::IndexedDeclaration {
                index,
                declaration: file
                    .declaration(index)
                    .expect("scope contains an invalid declaration index"),
            })
    }
}
