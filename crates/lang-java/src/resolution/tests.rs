mod ambiguity;
mod imports;
mod lexical;
mod on_demand;
mod same_package;
mod staging;

use std::path::PathBuf;

use beans_core::{
    language::LanguageProcessing,
    model::{Offset, OffsetSpan},
    storage::Revision,
};
use beans_platform_jvm::{
    PlatformJvm,
    query::{JvmContainer, JvmQuery, JvmScope, JvmScopeQuery},
};

use crate::{
    LanguageJava,
    model::{JavaQualifiedName, JavaTypeDeclaration},
};

use super::*;

fn identifier(text: &str) -> JavaIdentifier {
    JavaIdentifier {
        text: text.into(),
        span: OffsetSpan {
            start: Offset(0),
            end: Offset(text.len()),
        },
    }
}

fn source(path: &str) -> JvmSource {
    JvmSource::SourceFile {
        path: PathBuf::from(path),
    }
}

fn type_declaration(file: &JavaFile, declaration_id: JavaDeclarationId) -> &JavaTypeDeclaration {
    let JavaDeclaration::Type(declaration) = &file.declarations[declaration_id.0] else {
        panic!("expected a type declaration");
    };
    declaration
}

fn type_in_scope(file: &JavaFile, scope_id: JavaLexicalScopeId, name: &str) -> JavaDeclarationId {
    file.lexical_scopes[scope_id.0]
        .declarations
        .iter()
        .copied()
        .find(|declaration_id| {
            type_declaration(file, *declaration_id)
                .name
                .as_ref()
                .is_some_and(|identifier| identifier.text == name)
        })
        .unwrap()
}

/// The scope a declaration's own type reference is resolved in, which is what
/// `resolve_occurrence_at` hands over when a caret lands on one. Naming the
/// declaration rather than an offset keeps a test from counting bytes.
fn declaring_scope_of(file: &JavaFile, name: &str) -> JavaLexicalScopeId {
    file.declarations
        .iter()
        .find(|declaration| {
            declaration
                .name()
                .is_some_and(|identifier| identifier.text == name)
        })
        .expect("no declaration by that name")
        .declaring_scope()
}

fn java_query<'a>(
    java: &'a LanguageJava,
    jvm: &'a PlatformJvm,
    revision: Revision,
) -> JavaQuery<'a> {
    JavaQuery::new(
        JvmQuery::new(jvm, JvmScopeQuery::unscoped(), revision),
        java,
    )
}

fn file_model<'java>(
    java: &'java LanguageJava,
    revision: Revision,
    source: &JvmSource,
) -> &'java JavaFile {
    java.model_at(source, revision).unwrap()
}

fn compilation_unit_site<'a>(source: &'a JvmSource, file: &'a JavaFile) -> JavaSite<'a> {
    JavaSite {
        source,
        file,
        scope: JavaLexicalScopeId(0),
    }
}

fn process(
    java: &mut LanguageJava,
    jvm: &mut PlatformJvm,
    revision: Revision,
    path: &str,
    contents: &str,
) -> JvmSource {
    let source = source(path);
    java.process(source.clone(), revision, jvm, contents);
    source
}
