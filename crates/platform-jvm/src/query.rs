use beans_core::storage::Revision;
use std::path::PathBuf;

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

/// What a source can see. Opaque on purpose: it carries its containers today,
/// and can become a handle into a persisted scope table without any caller
/// noticing, because nothing outside reads its shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmScopeQuery(Scope);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Scope {
    /// Nothing has told us what this source can see. Everything in the lake
    /// is visible, which is what a cold start serves before the first import.
    Unscoped,
    Containers(Vec<JvmContainer>),
}

impl JvmScopeQuery {
    pub fn unscoped() -> JvmScopeQuery {
        JvmScopeQuery(Scope::Unscoped)
    }

    pub fn of(containers: Vec<JvmContainer>) -> JvmScopeQuery {
        JvmScopeQuery(Scope::Containers(containers))
    }

    pub fn contains(&self, source: &JvmSource) -> bool {
        match &self.0 {
            Scope::Unscoped => true,
            Scope::Containers(containers) => {
                containers.iter().any(|container| container.holds(source))
            }
        }
    }
}

/// One read of the JVM world: what exists, seen through which scope, as of
/// which revision. The engine builds it per request and hands it to the
/// language verticals, so resolution asks about names instead of about jars
/// and classpath order.
pub struct JvmQuery<'jvm> {
    pub jvm: &'jvm PlatformJvm,
    pub scope: JvmScopeQuery,
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

    /// Everything the scope can see declaring this binary name. Several
    /// answers means the name is contested; ordering the containers is what
    /// will decide between them.
    pub fn classes_named(&self, fqn: &JvmQualifiedName) -> Vec<(&'jvm JvmSource, &'jvm JvmClass)> {
        self.classes()
            .filter(|(_, class)| class.fqn == *fqn)
            .collect()
    }

    /// Package in the binary-name sense: `p.Outer$Inner` lives in `p`, so
    /// nested classes come back too; languages filter by `enclosing`.
    pub fn classes_in_package(&self, package: &str) -> Vec<(&'jvm JvmSource, &'jvm JvmClass)> {
        self.classes()
            .filter(|(_, class)| class.fqn.package() == package)
            .collect()
    }

    // TODO: rank rather than filter. Two containers claiming a name is
    // shadowing, not ambiguity, and the order here is a HashMap's.
    fn classes(&self) -> impl Iterator<Item = (&'jvm JvmSource, &'jvm JvmClass)> {
        let scope = &self.scope;
        self.jvm
            .class_lake
            .iter_at(self.revision)
            .filter(move |(source, _)| scope.contains(source))
            .flat_map(|(source, classes)| classes.iter().map(move |class| (source, class)))
    }
}
