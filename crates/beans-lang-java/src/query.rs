use crate::model::{
    File,
    nodes::{NodeIndex, types::TypeDeclaration},
};
use crate::model_query::JavaModelQuery;
pub use crate::semantics::DeclarationHandle;
use beans_core::{classpath::Classpath, names::Name, resource::ResourceId};
use beans_core::{query_scope::QueryScope, resolution::TypeDefinitionQuery, revision::Revision};

pub struct JavaQuery<'a, Q: JavaModelQuery> {
    models: &'a Q,
    scope: QueryScope<'a>,
}

impl<'a, Q: JavaModelQuery> JavaQuery<'a, Q> {
    pub fn new(models: &'a Q, scope: QueryScope<'a>) -> Self {
        Self { models, scope }
    }

    pub fn revision(&self) -> Revision {
        self.scope.revision()
    }

    pub fn classpath(&self) -> &'a dyn Classpath {
        self.scope.classpath()
    }

    pub fn file(&self, source: &ResourceId) -> Result<Option<File>, String> {
        self.models.java_file(source)
    }

    pub fn declaration_handle(
        &self,
        source: &ResourceId,
        node_index: NodeIndex,
    ) -> DeclarationHandle {
        DeclarationHandle::new(self.scope.revision(), source.clone(), node_index)
    }

    pub fn declaration(
        &self,
        handle: &DeclarationHandle,
    ) -> Result<Option<TypeDeclaration>, String> {
        if handle.revision() != self.scope.revision() {
            return Ok(None);
        }
        let Some(file) = self.file(handle.source())? else {
            return Ok(None);
        };
        Ok(file
            .node(handle.node_index())
            .and_then(|node| node.kind().as_type())
            .cloned())
    }

    /// Finds declaration handles by canonical name (JLS §6.7) in visible source files.
    /// Does not check Java accessibility or search inherited members.
    pub fn find_type(&self, name: &Name) -> Result<Vec<DeclarationHandle>, String> {
        let mut candidates = Vec::new();
        for (source, file) in self.models.java_files()? {
            if !self.scope.classpath().contains_source(&source) {
                continue;
            }
            for (node_index, _) in file.find_type(name) {
                candidates.push(self.declaration_handle(&source, node_index));
            }
        }
        Ok(candidates)
    }
}

impl<Q: JavaModelQuery> TypeDefinitionQuery<DeclarationHandle> for JavaQuery<'_, Q> {
    fn find_types(&self, name: &Name) -> Result<Vec<DeclarationHandle>, String> {
        self.find_type(name)
    }
}

#[cfg(test)]
mod tests;
