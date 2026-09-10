use crate::query::{JavaQuery, JavaTypeEntry};
use beans_lang_java_model::{
    self as java_model, ScopeEntry,
    declarations::{Declaration::Type, types::AccessLevel},
    imports::ImportType,
    references::{TypeNameComponent, TypeRef},
    scopes::{ScopeIndex, ScopeKind},
};

/// The answer to what a `TypeRef` actually responds to, given the classpath
pub enum ResolutionResult {
    Resolved,
    Ambigous,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeLookupError {
    NotFound,
    Ambiguous,
}

impl<'a> JavaTypeEntry<'a> {
    /// An empty path denotes this type; nonempty paths require member lookup (JLS §6.5.5.2).
    pub fn resolve_member_path(
        self,
        remaining: &[TypeNameComponent],
    ) -> Result<Self, TypeLookupError> {
        if remaining.is_empty() {
            return Ok(self);
        }

        todo!("resolve member types with accessibility, inheritance, and ambiguity checks")
    }
}

pub struct Resolver {}

impl Resolver {
    pub fn resolve(
        &self,
        file: &java_model::File,
        scope_index: ScopeIndex,
        type_ref: &TypeRef,
        query: &JavaQuery<'_>,
    ) -> ResolutionResult {
        ResolutionInstance::new(self, file, scope_index, type_ref, query).resolve()
    }
}

struct ResolutionInstance<'a> {
    resolver: &'a Resolver,
    file: &'a java_model::File,
    scope_index: ScopeIndex,
    type_ref: &'a TypeRef,
    query: &'a JavaQuery<'a>,
}

impl<'a> ResolutionInstance<'a> {
    fn new(
        resolver: &'a Resolver,
        file: &'a java_model::File,
        scope_index: ScopeIndex,
        type_ref: &'a TypeRef,
        query: &'a JavaQuery<'a>,
    ) -> Self {
        Self {
            resolver,
            file,
            scope_index,
            type_ref,
            query,
        }
    }

    fn resolve(&self) -> ResolutionResult {
        for entry in self.file.iter_scopes_from(self.scope_index) {
            let result = self.lookup_simple_type(entry);

            match result {
                ResolutionResult::NotFound => continue,
                result => return result,
            }
        }

        match self.type_ref {
            TypeRef::Named { segments } if segments.len() == 1 => {
                self.find_single_imported_type(self.type_ref)
            }
            _ => ResolutionResult::NotFound,
        }
    }

    /// JLS §6.3, §6.4.1, §6.5.5.1.
    fn lookup_simple_type(&self, entry: ScopeEntry<'_>) -> ResolutionResult {
        // Not a name, we are not interested
        let TypeRef::Named { segments } = self.type_ref else {
            return ResolutionResult::NotFound;
        };
        // Not a simple name, we are not interested
        let [component] = segments.as_slice() else {
            return ResolutionResult::NotFound;
        };

        self.find_first(
            entry,
            &component.name,
            &[
                Self::lookup_simple_type_declaration,
                Self::lookup_simple_type_parameter,
            ],
        )
    }

    /// JLS §6.3, §6.4.1.
    fn lookup_simple_type_declaration(
        &self,
        entry: ScopeEntry<'_>,
        name: &str,
    ) -> ResolutionResult {
        let mut matches = self
            .file
            .iter_declarations_in_scope(entry.scope_index)
            .filter_map(|entry| match entry.declaration {
                Type(typ) if typ.name.as_deref() == Some(name) => Some(typ),
                _ => None,
            });

        match (matches.next(), matches.next()) {
            (None, _) => ResolutionResult::NotFound,
            (Some(_), None) => ResolutionResult::Resolved,
            (Some(_), Some(_)) => ResolutionResult::Ambigous,
        }
    }

    /// JLS §6.3: a class's type parameters are in scope in its own body.
    fn lookup_simple_type_parameter(&self, entry: ScopeEntry<'_>, name: &str) -> ResolutionResult {
        let ScopeKind::TypeBody { owner } = entry.scope.kind() else {
            return ResolutionResult::NotFound;
        };
        let Type(typ) = self.file.declaration(owner).expect("invalid scope owner") else {
            unreachable!("type-body scope must have a type owner");
        };

        match typ.type_parameter_named(name) {
            Some(_) => ResolutionResult::Resolved,
            None => ResolutionResult::NotFound,
        }
    }

    fn find_first(
        &self,
        entry: ScopeEntry<'_>,
        name: &str,
        stages: &[fn(&Self, ScopeEntry<'_>, &str) -> ResolutionResult],
    ) -> ResolutionResult {
        for lookup in stages {
            match lookup(self, entry, name) {
                ResolutionResult::NotFound => continue,
                result => return result,
            }
        }

        ResolutionResult::NotFound
    }

    /// JLS §7.5.1: ordinary single imports bind the imported type's simple name.
    fn find_single_imported_type(&self, type_ref: &TypeRef) -> ResolutionResult {
        /// TODOs:
        /// This method is partially complete. What is missing is
        ///  - Accessibility checks
        let TypeRef::Named { segments } = type_ref else {
            return ResolutionResult::NotFound;
        };
        let Some((first, remaining)) = segments.split_first() else {
            return ResolutionResult::NotFound;
        };

        

        let candidates = self
            .file
            .imports
            .iter()
            .filter(|i| i.has_valid_name_shape())
            .filter(|i| i.typ() == ImportType::SingleType)
            // When using inner classes like `Outer.Inner`, we cannot directly look for this.
            // What we will store is the types in the compilation unit, i.e. `Outer`
            // Resolving `Inner` is the returned type's job.
            .filter(|i| i.name().last() == Some(&first.name))
            // We resolve the typename by looking into the processed symbols
            .flat_map(|i| self.query.finWhat d_type(i.name()))
            .filter(|t| self.is_accessible_top_level_type(t));

        let result = Self::select_unique_type(candidates)
            .and_then(|target| target.resolve_member_path(remaining));

        match result {
            Ok(_) => ResolutionResult::Resolved,
            Err(TypeLookupError::NotFound) => ResolutionResult::NotFound,
            Err(TypeLookupError::Ambiguous) => ResolutionResult::Ambigous,
        }
    }

    /// JLS §6.6.1: top-level access in the current non-modular compilation model.
    fn is_accessible_top_level_type(&self, target: &JavaTypeEntry<'_>) -> bool {
        if target.file.package_name == self.file.package_name {
            target.declaration.access.is_empty()
                || target.declaration.access.contains(&AccessLevel::Public)
        } else {
            target.declaration.access.contains(&AccessLevel::Public)
        }
    }

    /// JLS §7.5.1: repeated imports of one target are not conflicting imports.
    fn select_unique_type(
        mut candidates: impl Iterator<Item = JavaTypeEntry<'a>>,
    ) -> Result<JavaTypeEntry<'a>, TypeLookupError> {
        let first = candidates.next().ok_or(TypeLookupError::NotFound)?;
        for candidate in candidates {
            if candidate.source != first.source
                || candidate.declaration_index != first.declaration_index
            {
                return Err(TypeLookupError::Ambiguous);
            }
        }
        Ok(first)
    }

    fn find_package_type(&self, _type_ref: &TypeRef) -> ResolutionResult {
        todo!()
    }

    fn find_on_demand_imported_type(&self, _type_ref: &TypeRef) -> ResolutionResult {
        todo!()
    }
}

#[cfg(test)]
mod tests;
