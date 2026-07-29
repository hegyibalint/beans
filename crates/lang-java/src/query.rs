use beans_platform_jvm::model::{JvmClass, JvmQualifiedName, JvmSource};
use beans_platform_jvm::query::JvmQuery;

use crate::LanguageJava;
use crate::model::{JavaDeclaration, JavaFile};
use crate::resolution::JavaTypeTarget;

/// The JVM query plus this vertical's own models. The platform has already
/// narrowed to what the scope can see; the only thing it cannot know is
/// whether Java holds a richer view of a name than the projection.
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

    /// Everything declaring this binary name, each seen through the best view
    /// we hold of it. Several answers means the name is contested.
    pub fn types_named(&self, fqn: &JvmQualifiedName) -> Vec<JavaTypeTarget> {
        self.jvm
            .classes_named(fqn)
            .into_iter()
            .map(|(source, class)| self.view_of(source, class))
            .collect()
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
            None => JavaTypeTarget::Jvm(class.fqn.clone()),
        }
    }
}
