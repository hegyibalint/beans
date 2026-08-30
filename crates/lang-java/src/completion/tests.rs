mod agreement;
mod handles;
mod imports;
mod java_lang;
mod lexical;
mod members;
mod methods;
mod same_package;
mod shadowing;
mod variables;

use std::path::PathBuf;

use beans_core::language::LanguageProcessing;
use beans_core::storage::Revision;
use beans_platform_jvm as jvm;
use beans_test_support::markers::strip_markers;

use crate::Language;

use super::*;

/// Files processed into a fresh language, exactly one of them marked `<cur>`.
/// Unscoped, because scoping is settled in `resolution.rs`; what a caret is
/// offered is the same question asked backwards, and the stages it goes through
/// are the ones under test.
struct Workspace {
    java: Language,
    jvm: jvm::Platform,
    revision: Revision,
    caret: jvm::model::Source,
    offset: Offset,
    text: String,
}

impl Workspace {
    fn of(files: &[(&str, &str)]) -> Workspace {
        let mut java = Language::new();
        let mut jvm = jvm::Platform::new();
        let mut revision = Revision::default();
        let mut caret = None;

        for (path, contents) in files {
            let path = PathBuf::from(path);
            let stripped = strip_markers(contents, &path);
            let source = jvm::model::Source::SourceFile { path };
            if let Some(cursor) = stripped.cursors.first() {
                caret = Some((
                    source.clone(),
                    Offset(cursor.offset),
                    stripped.clean.clone(),
                ));
            }
            let at = revision.bump();
            java.process(source, at, &mut jvm, &stripped.clean);
        }

        let (caret, offset, text) = caret.expect("a fixture marks exactly one caret");
        Workspace {
            java,
            jvm,
            revision,
            caret,
            offset,
            text,
        }
    }

    /// A class the lake holds with no Java model behind it: what a jar, a jmod
    /// or a runtime image contributes, and the only way to put something in
    /// `java.lang` without reading a JDK.
    fn compiled(mut self, path: &str, fqn: &str) -> Workspace {
        let at = self.revision.bump();
        self.jvm.register(
            at,
            jvm::model::Source::ClassFile {
                path: PathBuf::from(path),
            },
            vec![jvm::model::Class {
                fqn: jvm::model::BinaryName::new(fqn),
                kind: jvm::model::TypeKind::Class,
                access: Some(jvm::model::AccessLevel::Public),
                enclosing: None,
                superclass: None,
                interfaces: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
            }],
        );
        self
    }

    fn complete(&self) -> Vec<CompletionItem<jvm::model::Source>> {
        let file = self
            .java
            .model_at(&self.caret, self.revision)
            .expect("the caret's file was parsed");
        let query = Query::new(
            jvm::query::Query::new(&self.jvm, jvm::query::ScopeQuery::unscoped(), self.revision),
            &self.java,
        );
        let point = Point::at(&self.caret, file, self.offset, &self.text, &query)
            .expect("the caret is inside the compilation unit");

        complete(&point, &query, self.revision)
    }
}

/// One file, no lake beyond what it projects itself.
fn complete_at_cursor(contents: &str) -> Vec<CompletionItem<jvm::model::Source>> {
    Workspace::of(&[("p/Test.java", contents)]).complete()
}

/// Only the rows §6.5.5 produced, for a claim that is about types while the
/// other two namespaces share the same list.
fn type_labels(items: &[CompletionItem<jvm::model::Source>]) -> Vec<&str> {
    items
        .iter()
        .filter(|item| {
            matches!(
                item.kind,
                CompletionItemKind::Class
                    | CompletionItemKind::Interface
                    | CompletionItemKind::Enum
                    | CompletionItemKind::Record
                    | CompletionItemKind::AnnotationInterface
                    | CompletionItemKind::TypeParameter
            )
        })
        .map(|item| item.label.as_str())
        .collect()
}

fn labels(items: &[CompletionItem<jvm::model::Source>]) -> Vec<&str> {
    items.iter().map(|item| item.label.as_str()).collect()
}

/// Found by what it writes, not by what it reads. A method's label carries its
/// parameters and its insertion does not, so the insertion is the half that
/// names a declaration.
fn item<'a>(
    items: &'a [CompletionItem<jvm::model::Source>],
    name: &str,
) -> &'a CompletionItem<jvm::model::Source> {
    items
        .iter()
        .find(|item| item.insert == name)
        .unwrap_or_else(|| panic!("nothing inserts {name:?} in {:?}", labels(items)))
}
