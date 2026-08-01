use beans_core::storage::Revision;
use std::path::PathBuf;
use std::sync::Arc;

use crate::PlatformJvm;
use crate::model::{JvmClass, JvmQualifiedName, JvmSource};

/// One place classes come from. A source names its own container for every
/// kind but a source file, which is why membership is a cheap check rather
/// than a stored list of everything a scope can see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JvmContainer {
    /// Where hand-written code lives. A directory for now, which is the widest
    /// reading of a build tool's tree selector: its include and exclude
    /// patterns are dropped.
    Source(PathBuf),
    /// Where compiled code lives: a jar, a jmod, or a runtime image.
    Artifact(PathBuf),
}

impl JvmContainer {
    fn holds(&self, source: &JvmSource) -> bool {
        match (self, source) {
            (JvmContainer::Source(base), JvmSource::SourceFile { path }) => path.starts_with(base),
            (JvmContainer::Artifact(artifact), JvmSource::JarEntry { jar_path, .. }) => {
                jar_path == artifact
            }
            (JvmContainer::Artifact(artifact), JvmSource::JmodEntry { jmod_path, .. }) => {
                jmod_path == artifact
            }
            (JvmContainer::Artifact(artifact), JvmSource::JimageEntry { jimage_path, .. }) => {
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
pub struct JvmScope(Arc<Vec<JvmContainer>>);

impl JvmScope {
    pub fn of(containers: Vec<JvmContainer>) -> JvmScope {
        JvmScope(Arc::new(containers))
    }

    fn holds(&self, source: &JvmSource) -> bool {
        self.0.iter().any(|container| container.holds(source))
    }
}

/// What one source can see. Opaque on purpose: nothing outside reads its
/// shape, so it can become a handle into a persisted scope table later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmScopeQuery(Scope);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Scope {
    /// Nothing has told us what this source can see. Everything in the lake
    /// is visible, which is what a cold start serves before the first import.
    Unscoped,
    /// More than one because a tree can be claimed by two units, and a file
    /// checked into both is compiled by both.
    Scopes(Vec<JvmScope>),
}

impl JvmScopeQuery {
    pub fn unscoped() -> JvmScopeQuery {
        JvmScopeQuery(Scope::Unscoped)
    }

    pub fn of(scopes: Vec<JvmScope>) -> JvmScopeQuery {
        JvmScopeQuery(Scope::Scopes(scopes))
    }

    /// Visible under any one scope is visible. That is the permissive reading
    /// of shared code: the strict one analyses the file once per scope and
    /// reports per context, which needs an analysis to have a context first.
    pub fn contains(&self, source: &JvmSource) -> bool {
        match &self.0 {
            Scope::Unscoped => true,
            Scope::Scopes(scopes) => scopes.iter().any(|scope| scope.holds(source)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JvmScopeMembership {
    InScope,
    OutsideScope,
}

/// One read of the JVM world: what exists, seen through which scope, as of
/// which revision. The engine builds it per request and hands it to the
/// language verticals, so resolution asks about names instead of about jars
/// and classpath order.
pub struct JvmQuery<'jvm> {
    jvm: &'jvm PlatformJvm,
    scope: JvmScopeQuery,
    pub revision: Revision,
}

impl<'jvm> JvmQuery<'jvm> {
    pub fn new(jvm: &'jvm PlatformJvm, scope: JvmScopeQuery, revision: Revision) -> JvmQuery<'jvm> {
        JvmQuery {
            jvm,
            scope,
            revision,
        }
    }

    /// Every declaration of this binary name in the lake at this query's
    /// revision. Scope membership is a separate question.
    pub fn classes_named(&self, fqn: &JvmQualifiedName) -> Vec<(&'jvm JvmSource, &'jvm JvmClass)> {
        self.all_classes()
            .filter(|(_, class)| class.fqn == *fqn)
            .collect()
    }

    /// Whether `candidate_source` is visible from this query's viewpoint.
    pub fn scope_membership(&self, candidate_source: &JvmSource) -> JvmScopeMembership {
        if self.scope.contains(candidate_source) {
            JvmScopeMembership::InScope
        } else {
            JvmScopeMembership::OutsideScope
        }
    }

    /// Package in the binary-name sense: `p.Outer$Inner` lives in `p`, so
    /// nested classes come back too; languages filter by `enclosing`.
    pub fn classes_in_package(&self, package: &str) -> Vec<(&'jvm JvmSource, &'jvm JvmClass)> {
        self.all_classes()
            .filter(|(_, class)| class.fqn.package() == package)
            .collect()
    }

    fn all_classes(&self) -> impl Iterator<Item = (&'jvm JvmSource, &'jvm JvmClass)> {
        self.jvm
            .class_lake
            .iter_at(self.revision)
            .flat_map(|(source, classes)| classes.iter().map(move |class| (source, class)))
    }
}
