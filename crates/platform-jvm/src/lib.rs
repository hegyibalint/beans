use beans_core::{Revision, storage::RevisionedStorage};

use crate::model::{JvmClass, JvmSource};
use crate::scope::FileScope;

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

    /// The source is unused until classpath scoping exists; passing it
    /// stakes the shape of the query.
    pub fn scope(&self, _jvm_source: &JvmSource, revision: Revision) -> FileScope<'_> {
        FileScope::new(&self.class_lake, revision)
    }
}
