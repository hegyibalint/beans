use beans_core_model as core_model;
use beans_lang_java_model::{
    self as java_model, ScopeEntry,
    declarations::Declaration::Type,
    imports::ImportType,
    references::TypeRef,
    scopes::{ScopeIndex, ScopeKind},
};

/// The answer to what a `TypeRef` actually responds to, given the classpath
pub enum ResolutionResult {
    Resolved,
    Ambigous,
    NotFound,
}

pub struct Resolver {}

impl Resolver {
    pub fn resolve(
        &self,
        file: &java_model::File,
        scope_index: ScopeIndex,
        type_ref: &TypeRef,
        classpath: &core_model::classpath::Classpath,
    ) -> ResolutionResult {
        ResolutionInstance::new(self, file, scope_index, type_ref, classpath).resolve()
    }
}

struct ResolutionInstance<'a> {
    resolver: &'a Resolver,
    file: &'a java_model::File,
    scope_index: ScopeIndex,
    type_ref: &'a TypeRef,
    classpath: &'a core_model::classpath::Classpath,
}

impl<'a> ResolutionInstance<'a> {
    fn new(
        resolver: &'a Resolver,
        file: &'a java_model::File,
        scope_index: ScopeIndex,
        type_ref: &'a TypeRef,
        classpath: &'a core_model::classpath::Classpath,
    ) -> Self {
        Self {
            resolver,
            file,
            scope_index,
            type_ref,
            classpath,
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

        ResolutionResult::NotFound
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

    /// JLS §7.5.1, §7.5.3: single imports bind the imported member's simple name.
    fn find_single_imported_type(&self, type_ref: &TypeRef) -> ResolutionResult {
        let TypeRef::Named { segments } = type_ref else {
            return ResolutionResult::NotFound;
        };
        let Some((first, _remaining)) = segments.split_first() else {
            return ResolutionResult::NotFound;
        };

        let candidates: Vec<_> = self
            .file
            .imports
            .iter()
            .filter(|import| {
                matches!(
                    import.typ(),
                    ImportType::SingleType | ImportType::SingleStaticType
                )
            })
            .filter(|import| import.name().last() == Some(&first.name))
            .collect();

        if candidates.is_empty() {
            return ResolutionResult::NotFound;
        }

        todo!(
            "resolve each candidate according to its import kind, check accessibility/staticness, \
             deduplicate targets, and return the unique starting type with one component consumed"
        )
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
