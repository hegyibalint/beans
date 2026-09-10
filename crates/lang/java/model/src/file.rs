use crate::{
    declarations::{self, Declaration, DeclarationIndex, types::TypeDeclaration},
    imports,
    names::Name,
    scopes::{self, ScopeIndex, ScopeKind},
};

/// Represents a whole `.java` file.
#[derive(Debug)]
pub struct File {
    /// Package components; empty for the unnamed package (JLS §7.4.2).
    pub package_name: Name,
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
            package_name: Name::default(),
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

    pub fn iter_scopes_from(
        &self,
        scope_index: ScopeIndex,
    ) -> impl Iterator<Item = ScopeEntry<'_>> + '_ {
        std::iter::successors(Some(scope_index), |&index| {
            self.scope(index)
                .expect("invalid scope index")
                .parent_scope()
        })
        .map(move |index| ScopeEntry {
            scope_index: index,
            scope: self.scope(index).expect("invalid scope index"),
        })
    }

    pub fn iter_declarations(&self) -> impl Iterator<Item = DeclarationEntry<'_>> + '_ {
        self.iter_scopes()
            .flat_map(move |entry| self.iter_declarations_in_scope(entry.scope_index))
    }

    pub fn iter_declarations_from(
        &self,
        scope_index: ScopeIndex,
    ) -> impl Iterator<Item = DeclarationEntry<'_>> + '_ {
        self.iter_scopes_from(scope_index)
            .flat_map(move |entry| self.iter_declarations_in_scope(entry.scope_index))
    }

    pub fn iter_declarations_in_scope(
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

    /// Finds declarations by canonical name (JLS §6.7), retaining duplicate declarations.
    /// Follows declared members only; does not check accessibility or search inherited members.
    pub fn find_type(&self, name: &[String]) -> Vec<(DeclarationIndex, &TypeDeclaration)> {
        if !self.package_name.is_valid() {
            return Vec::new();
        }
        if name.is_empty() || name.iter().any(|component| component.is_empty()) {
            return Vec::new();
        }

        let Some(type_path) = name.strip_prefix(self.package_name.as_slice()) else {
            return Vec::new();
        };

        let mut search_scopes = vec![Self::ROOT_SCOPE_ID];
        let mut matches = Vec::new();

        for component in type_path {
            matches.clear();
            let mut member_scopes = Vec::new();

            for scope_index in search_scopes {
                for entry in self.iter_declarations_in_scope(scope_index) {
                    let Declaration::Type(declaration) = entry.declaration else {
                        continue;
                    };
                    if declaration.name.as_ref() != Some(component) {
                        continue;
                    }

                    matches.push((entry.declaration_index, declaration));

                    let body_kind = ScopeKind::TypeBody {
                        owner: entry.declaration_index,
                    };
                    for child_index in entry.scope.iter_child_scopes() {
                        let child = self.scope(child_index).expect("invalid child scope index");
                        if child.kind() == body_kind {
                            member_scopes.push(child_index);
                        }
                    }
                }
            }

            if matches.is_empty() {
                return matches;
            }
            search_scopes = member_scopes;
        }

        matches
    }
}

#[cfg(test)]
mod tests;
