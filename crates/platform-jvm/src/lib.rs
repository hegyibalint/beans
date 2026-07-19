use beans_core::storage::{Revision, RevisionedStorage};

use crate::model::{JvmClass, JvmQualifiedName, JvmSource};
use crate::scope::JvmScope;

pub mod model;
pub mod scope;

pub struct PlatformJvm {
    /// A source's value is its whole contribution, so re-registering a
    /// source replaces everything it previously declared.
    class_lake: RevisionedStorage<JvmSource, Vec<JvmClass>>,
}

impl PlatformJvm {
    pub fn new() -> PlatformJvm {
        PlatformJvm {
            class_lake: RevisionedStorage::new(),
        }
    }

    pub fn register(
        &mut self,
        revision: Revision,
        jvm_source: JvmSource,
        jvm_classes: Vec<JvmClass>,
    ) -> &[JvmClass] {
        self.class_lake.put(revision, jvm_source, jvm_classes)
    }

    pub fn class(
        &self,
        fqn: &JvmQualifiedName,
        scope: &dyn JvmScope,
        revision: Revision,
    ) -> Option<&JvmClass> {
        self.classes(scope, revision)
            .find(|class| class.fqn == *fqn)
    }

    /// Package in the binary-name sense: `p.Outer$Inner` lives in `p`,
    /// so nested classes come back too; languages filter by `enclosing`.
    pub fn classes_in_package(
        &self,
        package: &str,
        scope: &dyn JvmScope,
        revision: Revision,
    ) -> impl Iterator<Item = &JvmClass> {
        self.classes(scope, revision)
            .filter(move |class| class.fqn.package() == package)
    }

    fn classes(&self, scope: &dyn JvmScope, revision: Revision) -> impl Iterator<Item = &JvmClass> {
        self.class_lake
            .iter_at(revision)
            .filter(move |(source, _)| scope.in_scope(source))
            .flat_map(|(_source, classes)| classes)
    }
}
