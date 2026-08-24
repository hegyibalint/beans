use beans_platform_jvm as jvm;

use crate::Language;
use crate::model;
use crate::resolution::TypeTarget;

/// The JVM query plus this vertical's own models. Candidate discovery remains
/// broad; Java resolution asks this query about scope when a stage needs it.
pub struct Query<'a> {
    jvm: jvm::query::Query<'a>,
    java: &'a Language,
}

impl<'a> Query<'a> {
    pub fn new(jvm: jvm::query::Query<'a>, java: &'a Language) -> Query<'a> {
        Query { jvm, java }
    }

    pub fn model_of(&self, source: &jvm::model::Source) -> Option<&'a model::File> {
        self.java.model_at(source, self.jvm.revision)
    }

    /// Every declaration of this binary name, each seen through the best view
    /// we hold of it. Resolution applies scope before deciding whether the name
    /// is contested.
    pub fn types_named(&self, fqn: &jvm::model::BinaryName) -> Vec<TypeTarget> {
        self.jvm
            .classes_named(fqn)
            .into_iter()
            .map(|(source, class)| self.view_of(source, class))
            .collect()
    }

    /// The top-level types a package declares, each with the best view we hold
    /// of it, paired with the simple name it is in scope under.
    ///
    /// Nested types are dropped rather than returned: §6.5.5.1 puts a member
    /// type in scope by its simple name only inside its enclosing type or
    /// through an import, never by living in the same package. The lake cannot
    /// make that distinction for us — `classes_in_package` says so itself —
    /// because a binary name carries the package and nothing else about where
    /// the type sits, so §13.1's `$` is what is left to read.
    pub fn top_level_types_in_package<'q>(
        &'q self,
        package: &'q str,
    ) -> impl Iterator<Item = (String, TypeTarget)> + 'q {
        self.jvm
            .classes_in_package(package)
            .filter(|(_, class)| !class.fqn.is_nested())
            .map(move |(source, class)| {
                (
                    class.fqn.simple_name().to_string(),
                    self.view_of(source, class),
                )
            })
    }

    pub fn scope_membership(&self, target: &TypeTarget) -> jvm::query::ScopeMembership {
        self.jvm.scope_membership(target.source())
    }

    /// What a compiled type's declaration said about access. A
    /// `TypeTarget::Compiled` names a binary name and where it came from, never
    /// the class itself, so §6.6.1 has to come back here for the level.
    ///
    /// `None` is every answer we cannot give: a local or anonymous class, which
    /// §8.1.1 leaves outside access control, and a name this source no longer
    /// declares. Both make accessibility a question with no evidence, which
    /// resolution reads as permission.
    pub fn class_access(
        &self,
        source: &jvm::model::Source,
        fqn: &jvm::model::BinaryName,
    ) -> Option<jvm::model::AccessLevel> {
        self.jvm
            .classes_named(fqn)
            .into_iter()
            .find(|(class_source, _)| *class_source == source)
            .and_then(|(_, class)| class.access)
    }

    /// A file this vertical parsed gives a declaration to navigate to.
    /// Anything else, a class file or a Kotlin source, only ever has the
    /// lossy projection the lake holds.
    fn view_of(&self, source: &jvm::model::Source, class: &jvm::model::Class) -> TypeTarget {
        let declaration = self.model_of(source).and_then(|file| {
            file.top_level_declarations.iter().copied().find(|id| {
                let model::Declaration::Type(declaration) = &file.declarations[id.0] else {
                    return false;
                };
                declaration
                    .name
                    .as_ref()
                    .is_some_and(|name| name.text == class.fqn.simple_name())
            })
        });

        match declaration {
            Some(declaration) => TypeTarget::Parsed {
                source: source.clone(),
                declaration,
            },
            None => TypeTarget::Compiled {
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
    use beans_platform_jvm as jvm;

    use super::*;

    #[test]
    fn a_projected_target_retains_its_source_for_scope_membership() {
        let revision = Revision::default();
        let class_source = jvm::model::Source::JarEntry {
            jar_path: PathBuf::from("dependency.jar"),
            entry_path: "p/X.class".to_string(),
        };
        let class = jvm::model::Class {
            fqn: jvm::model::BinaryName::new("p.X"),
            kind: jvm::model::TypeKind::Class,
            access: Some(jvm::model::AccessLevel::Public),
            enclosing: None,
            superclass: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let asking_source = jvm::model::Source::SourceFile {
            path: PathBuf::from("app/p/Test.java"),
        };
        let mut jvm = jvm::Platform::new();
        jvm.register(revision, class_source.clone(), vec![class]);
        jvm.register_scopes(
            revision,
            asking_source.clone(),
            vec![jvm::query::Scope::of(vec![jvm::query::Container::Source(
                PathBuf::from("app"),
            )])],
        );
        let java = Language::new();
        let query = Query::new(jvm.query_from(&asking_source, revision), &java);

        let targets = query.types_named(&jvm::model::BinaryName::new("p.X"));

        assert_eq!(
            targets,
            vec![TypeTarget::Compiled {
                source: class_source,
                fqn: jvm::model::BinaryName::new("p.X"),
            }]
        );
        assert_eq!(
            query.scope_membership(&targets[0]),
            jvm::query::ScopeMembership::OutsideScope
        );
    }
}
