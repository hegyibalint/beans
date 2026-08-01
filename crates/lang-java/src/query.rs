use beans_platform_jvm::model::{JvmClass, JvmQualifiedName, JvmSource};
use beans_platform_jvm::query::{JvmQuery, JvmScopeMembership};

use crate::LanguageJava;
use crate::model::{JavaDeclaration, JavaFile};
use crate::resolution::JavaTypeTarget;

/// The JVM query plus this vertical's own models. Candidate discovery remains
/// broad; Java resolution asks this query about scope when a stage needs it.
pub struct JavaQuery<'a> {
    jvm: JvmQuery<'a>,
    java: &'a LanguageJava,
}

impl<'a> JavaQuery<'a> {
    pub fn new(jvm: JvmQuery<'a>, java: &'a LanguageJava) -> JavaQuery<'a> {
        JavaQuery { jvm, java }
    }

    pub fn model_of(&self, source: &JvmSource) -> Option<&'a JavaFile> {
        self.java.model_at(source, self.jvm.revision)
    }

    /// Every declaration of this binary name, each seen through the best view
    /// we hold of it. Resolution applies scope before deciding whether the name
    /// is contested.
    pub fn types_named(&self, fqn: &JvmQualifiedName) -> Vec<JavaTypeTarget> {
        self.jvm
            .classes_named(fqn)
            .into_iter()
            .map(|(source, class)| self.view_of(source, class))
            .collect()
    }

    pub fn scope_membership(&self, target: &JavaTypeTarget) -> JvmScopeMembership {
        self.jvm.scope_membership(target.source())
    }

    /// A file this vertical parsed gives a declaration to navigate to.
    /// Anything else, a class file or a Kotlin source, only ever has the
    /// lossy projection the lake holds.
    fn view_of(&self, source: &JvmSource, class: &JvmClass) -> JavaTypeTarget {
        let declaration = self.model_of(source).and_then(|file| {
            file.top_level_declarations.iter().copied().find(|id| {
                let JavaDeclaration::Type(declaration) = &file.declarations[id.0] else {
                    return false;
                };
                declaration
                    .name
                    .as_ref()
                    .is_some_and(|name| name.text == class.fqn.simple_name())
            })
        });

        match declaration {
            Some(declaration) => JavaTypeTarget::Java {
                source: source.clone(),
                declaration,
            },
            None => JavaTypeTarget::Jvm {
                source: source.clone(),
                fqn: class.fqn.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use beans_core::storage::Revision;
    use beans_platform_jvm::{
        PlatformJvm,
        model::{JvmKind, JvmSource},
        query::{JvmContainer, JvmScope},
    };

    use super::*;

    #[test]
    fn a_projected_target_retains_its_source_for_scope_membership() {
        let revision = Revision::default();
        let class_source = JvmSource::JarEntry {
            jar_path: PathBuf::from("dependency.jar"),
            entry_path: "p/X.class".to_string(),
        };
        let class = JvmClass {
            fqn: JvmQualifiedName::new("p.X"),
            kind: JvmKind::Class,
            enclosing: None,
            superclass: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let asking_source = JvmSource::SourceFile {
            path: PathBuf::from("app/p/Test.java"),
        };
        let mut jvm = PlatformJvm::new();
        jvm.register(revision, class_source.clone(), vec![class]);
        jvm.register_scopes(
            revision,
            asking_source.clone(),
            vec![JvmScope::of(vec![JvmContainer::Source(PathBuf::from(
                "app",
            ))])],
        );
        let java = LanguageJava::new();
        let query = JavaQuery::new(jvm.query_from(&asking_source, revision), &java);

        let targets = query.types_named(&JvmQualifiedName::new("p.X"));

        assert_eq!(
            targets,
            vec![JavaTypeTarget::Jvm {
                source: class_source,
                fqn: JvmQualifiedName::new("p.X"),
            }]
        );
        assert_eq!(
            query.scope_membership(&targets[0]),
            JvmScopeMembership::OutsideScope
        );
    }
}
