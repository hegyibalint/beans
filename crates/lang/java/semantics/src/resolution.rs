use beans_core_model as core_model;
use beans_lang_java_model as java_model;

/// The answer to what a `TypeRef` actually responds to, given the classpath
pub enum ResolutionResult {
    Resolved,
    Ambigous,
    NotFound,
}

impl ResolutionResult {
    fn or_else(self, next: impl FnOnce() -> Self) -> Self {
        match self {
            Self::NotFound => next(),
            result => result,
        }
    }
}

pub struct Resolver {}

impl Resolver {
    pub fn resolve(
        &self,
        type_ref: &java_model::references::TypeRef,
        classpath: &core_model::Classpath,
    ) -> ResolutionResult {
        ResolutionInstance::new(self, type_ref, classpath).resolve()
    }
}

struct ResolutionInstance<'a> {
    resolver: &'a Resolver,
    file: &'a java_model::File,
    type_ref: &'a java_model::references::TypeRef,
    classpath: &'a core_model::Classpath,
}

impl<'a> ResolutionInstance<'a> {
    fn new(
        resolver: &'a Resolver,
        file: &'a java_model::File,
        type_ref: &'a java_model::references::TypeRef,
        classpath: &'a core_model::Classpath,
    ) -> Self {
        Self {
            resolver,
            file,
            type_ref,
            classpath,
        }
    }

    fn resolve(&self) -> ResolutionResult {
        // Placeholder wiring, not final Java lookup precedence or branching.
        self.lookup_simple_type()
            .or_else(|| self.lookup_inherited_member_types())
            .or_else(|| self.lookup_with_owner_parameters())
            .or_else(|| self.lookup_compilation_environment())
            .or_else(|| self.lookup_all_sources_in_tier())
            .or_else(|| self.lookup_qualified_type())
            .or_else(|| self.unique())
    }

    /// JLS §6.3, §6.4.1, §6.5.5.1.
    ///
    /// A simple type name is a `TypeRef::Named` with exactly one segment. It
    /// may resolve to:
    ///
    /// - a type parameter, such as `T`;
    /// - a local, member, or top-level `Declaration::Type`;
    /// - an inherited member type;
    /// - an imported or same-package type.
    fn lookup_simple_type(&self) -> ResolutionResult {
        self.file.
    }

    /// JLS §8.5, §9.5.
    fn lookup_inherited_member_types(&self) -> ResolutionResult {
        todo!()
    }

    /// JLS §6.3.
    fn lookup_with_owner_parameters(&self) -> ResolutionResult {
        todo!()
    }

    /// JLS §6.4.1, §7.5.
    fn lookup_compilation_environment(&self) -> ResolutionResult {
        todo!()
    }

    /// JLS §6.4.1, §7.5.
    fn lookup_all_sources_in_tier(&self) -> ResolutionResult {
        todo!()
    }

    /// JLS §6.5.3, §6.5.4, §6.5.5.2.
    fn lookup_qualified_type(&self) -> ResolutionResult {
        todo!()
    }

    fn unique(&self) -> ResolutionResult {
        todo!()
    }
}
