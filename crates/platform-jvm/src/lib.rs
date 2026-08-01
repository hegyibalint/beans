use beans_core::storage::{Revision, RevisionedStorage};

use crate::model::{JvmClass, JvmSource};
use crate::query::{JvmQuery, JvmScope, JvmScopeQuery};

pub mod model;
pub mod query;

pub struct PlatformJvm {
    /// A source's value is its whole contribution, so re-registering a
    /// source replaces everything it previously declared.
    class_lake: RevisionedStorage<JvmSource, Vec<JvmClass>>,
    /// Keyed by the source doing the asking, because visibility runs one way:
    /// an app sees its library and the library does not see back.
    scopes: RevisionedStorage<JvmSource, Vec<JvmScope>>,
}

impl PlatformJvm {
    pub fn new() -> PlatformJvm {
        PlatformJvm {
            class_lake: RevisionedStorage::new(),
            scopes: RevisionedStorage::new(),
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

    /// Declare what `jvm_source` may see. Whoever flattened the project builds
    /// the scopes; this side stores them and never asks where they came from.
    pub fn register_scopes(
        &mut self,
        revision: Revision,
        jvm_source: JvmSource,
        jvm_scopes: Vec<JvmScope>,
    ) {
        self.scopes.put(revision, jvm_source, jvm_scopes);
    }

    /// One revision of the lake paired with `jvm_source`'s viewpoint.
    /// Discovery remains unfiltered; the query uses the paired scope to answer
    /// whether another source is visible from that viewpoint.
    ///
    /// The scope is looked up here rather than passed in, so a caller says
    /// where it is standing instead of carrying a scope from the wrong file.
    pub fn query_from(&self, jvm_source: &JvmSource, revision: Revision) -> JvmQuery<'_> {
        JvmQuery::new(self, self.scope_of(jvm_source, revision), revision)
    }

    /// A source nobody has scoped sees everything. That is the one answer that
    /// keeps a scratch file, an unopened project and a half-finished import
    /// working, and it makes scoping something a project opts into rather than
    /// something every caller has to remember to set up.
    fn scope_of(&self, jvm_source: &JvmSource, revision: Revision) -> JvmScopeQuery {
        match self.scopes.get(jvm_source, revision) {
            Some(scopes) => JvmScopeQuery::of(scopes.clone()),
            None => JvmScopeQuery::unscoped(),
        }
    }
}
