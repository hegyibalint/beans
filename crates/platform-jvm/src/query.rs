use beans_core::storage::Revision;
use std::path::PathBuf;
use std::sync::Arc;

use crate::Platform;
use crate::model;

/// One place classes come from. A source names its own container for every
/// kind but a source file, which is why membership is a cheap check rather
/// than a stored list of everything a scope can see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Container {
    /// Where hand-written code lives. A directory for now, which is the widest
    /// reading of a build tool's tree selector: its include and exclude
    /// patterns are dropped.
    Source(PathBuf),
    /// Where compiled code lives: a class file, a directory of them, a jar,
    /// jmod, or runtime image.
    Artifact(PathBuf),
}

impl Container {
    fn holds(&self, source: &model::Source) -> bool {
        match (self, source) {
            (Container::Source(base), model::Source::SourceFile { path }) => path.starts_with(base),
            // A class file names itself, so an artifact that is a directory has
            // to hold every class under it rather than only one equal to it.
            (Container::Artifact(artifact), model::Source::ClassFile { path }) => {
                path.starts_with(artifact)
            }
            (Container::Artifact(artifact), model::Source::JarEntry { jar_path, .. }) => {
                jar_path == artifact
            }
            (Container::Artifact(artifact), model::Source::JmodEntry { jmod_path, .. }) => {
                jmod_path == artifact
            }
            (Container::Artifact(artifact), model::Source::JimageEntry { jimage_path, .. }) => {
                jimage_path == artifact
            }
            _ => false,
        }
    }
}

/// One flattened answer to "what can be seen from here". Dependency edges,
/// transitivity and build tools are all resolved before this exists, so the
/// filter never walks a graph.
///
/// Shared rather than copied, because every file in a source set sees exactly
/// the same thing: a unit builds one of these and all of its files point at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope(Arc<Vec<Container>>);

impl Scope {
    pub fn of(containers: Vec<Container>) -> Scope {
        Scope(Arc::new(containers))
    }

    fn holds(&self, source: &model::Source) -> bool {
        self.0.iter().any(|container| container.holds(source))
    }
}

/// What one source can see. Opaque on purpose: nothing outside reads its
/// shape, so it can become a handle into a persisted scope table later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeQuery(Selection);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Selection {
    /// Nothing has told us what this source can see. Everything in the lake
    /// is visible, which is what a cold start serves before the first import.
    Unscoped,
    /// More than one because a tree can be claimed by two units, and a file
    /// checked into both is compiled by both.
    Scopes(Vec<Scope>),
}

impl ScopeQuery {
    pub fn unscoped() -> ScopeQuery {
        ScopeQuery(Selection::Unscoped)
    }

    pub fn of(scopes: Vec<Scope>) -> ScopeQuery {
        ScopeQuery(Selection::Scopes(scopes))
    }

    /// Visible under any one scope is visible. That is the permissive reading
    /// of shared code: the strict one analyses the file once per scope and
    /// reports per context, which needs an analysis to have a context first.
    pub fn contains(&self, source: &model::Source) -> bool {
        match &self.0 {
            Selection::Unscoped => true,
            Selection::Scopes(scopes) => scopes.iter().any(|scope| scope.holds(source)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeMembership {
    InScope,
    OutsideScope,
}

/// One read of the JVM world: what exists, seen through which scope, as of
/// which revision. The engine builds it per request and hands it to the
/// language verticals, so resolution asks about names instead of about jars
/// and classpath order.
pub struct Query<'jvm> {
    jvm: &'jvm Platform,
    scope: ScopeQuery,
    pub revision: Revision,
}

impl<'jvm> Query<'jvm> {
    pub fn new(jvm: &'jvm Platform, scope: ScopeQuery, revision: Revision) -> Query<'jvm> {
        Query {
            jvm,
            scope,
            revision,
        }
    }

    /// Every declaration of this binary name in the lake at this query's
    /// revision. Scope membership is a separate question.
    pub fn classes_named(
        &self,
        fqn: &model::BinaryName,
    ) -> Vec<(&'jvm model::Source, &'jvm model::Class)> {
        self.all_classes()
            .filter(|(_, class)| class.fqn == *fqn)
            .collect()
    }

    /// Whether `candidate_source` is visible from this query's viewpoint.
    pub fn scope_membership(&self, candidate_source: &model::Source) -> ScopeMembership {
        if self.scope.contains(candidate_source) {
            ScopeMembership::InScope
        } else {
            ScopeMembership::OutsideScope
        }
    }

    /// Package in the binary-name sense: `p.Outer$Inner` lives in `p`, so
    /// nested classes come back too; languages filter by `enclosing`.
    pub fn classes_in_package(
        &self,
        package: &str,
    ) -> Vec<(&'jvm model::Source, &'jvm model::Class)> {
        self.all_classes()
            .filter(|(_, class)| class.fqn.package() == package)
            .collect()
    }

    fn all_classes(&self) -> impl Iterator<Item = (&'jvm model::Source, &'jvm model::Class)> {
        self.jvm
            .classes
            .iter_at(self.revision)
            .flat_map(|(source, classes)| classes.iter().map(move |class| (source, class)))
    }
}
