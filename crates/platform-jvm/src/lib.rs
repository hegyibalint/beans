use std::path::PathBuf;

use beans_core::storage::{Revision, RevisionedStorage};

use crate::query::{Query, Scope, ScopeQuery};

mod class_file;
mod container;
pub mod model;
pub mod query;

pub struct Platform {
    /// A source's value is its whole contribution, so re-registering a
    /// source replaces everything it previously declared.
    classes: RevisionedStorage<model::Source, Vec<model::Class>>,
    /// Keyed by the source doing the asking, because visibility runs one way:
    /// an app sees its library and the library does not see back.
    scopes: RevisionedStorage<model::Source, Vec<Scope>>,
}

impl Platform {
    pub fn new() -> Platform {
        Platform {
            classes: RevisionedStorage::new(),
            scopes: RevisionedStorage::new(),
        }
    }

    pub fn register(
        &mut self,
        revision: Revision,
        jvm_source: model::Source,
        jvm_classes: Vec<model::Class>,
    ) -> &[model::Class] {
        self.classes.put(revision, jvm_source, jvm_classes)
    }

    /// Declare what `jvm_source` may see. Whoever flattened the project builds
    /// the scopes; this side stores them and never asks where they came from.
    pub fn register_scopes(
        &mut self,
        revision: Revision,
        jvm_source: model::Source,
        jvm_scopes: Vec<Scope>,
    ) {
        self.scopes.put(revision, jvm_source, jvm_scopes);
    }

    /// One revision of the lake paired with `jvm_source`'s viewpoint.
    /// Discovery remains unfiltered; the query uses the paired scope to answer
    /// whether another source is visible from that viewpoint.
    ///
    /// The scope is looked up here rather than passed in, so a caller says
    /// where it is standing instead of carrying a scope from the wrong file.
    pub fn query_from(&self, jvm_source: &model::Source, revision: Revision) -> Query<'_> {
        Query::new(self, self.scope_of(jvm_source, revision), revision)
    }

    /// A source nobody has scoped sees everything. That is the one answer that
    /// keeps a scratch file, an unopened project and a half-finished import
    /// working, and it makes scoping something a project opts into rather than
    /// something every caller has to remember to set up.
    fn scope_of(&self, jvm_source: &model::Source, revision: Revision) -> ScopeQuery {
        match self.scopes.get(jvm_source, revision) {
            Some(scopes) => ScopeQuery::of(scopes.clone()),
            None => ScopeQuery::unscoped(),
        }
    }

    pub fn process_classpath(&mut self, classpath: &[PathBuf], revision: Revision) {
        for element in classpath {
            for processed in container::process(element) {
                let Ok((source, class)) = processed else {
                    continue;
                };
                self.register(revision, source, vec![class]);
            }
        }
    }
}
