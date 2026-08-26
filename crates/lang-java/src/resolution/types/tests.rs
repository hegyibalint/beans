mod ambiguity;
mod compiled;
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
use beans_platform_jvm as jvm;

use crate::{Language, model};

use super::*;

fn identifier(text: &str) -> model::Identifier {
    model::Identifier {
        text: text.into(),
        span: OffsetSpan {
            start: Offset(0),
            end: Offset(text.len()),
        },
    }
}

fn source(path: &str) -> jvm::model::Source {
    jvm::model::Source::SourceFile {
        path: PathBuf::from(path),
    }
}

fn type_declaration(
    file: &model::File,
    declaration_id: model::DeclarationId,
) -> &model::TypeDeclaration {
    let model::Declaration::Type(declaration) = &file.declarations[declaration_id.0] else {
        panic!("expected a type declaration");
    };
    declaration
}

fn type_in_scope(
    file: &model::File,
    scope_id: model::LexicalScopeId,
    name: &str,
) -> model::DeclarationId {
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
fn declaring_scope_of(file: &model::File, name: &str) -> model::LexicalScopeId {
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

fn java_query<'a>(java: &'a Language, jvm: &'a jvm::Platform, revision: Revision) -> Query<'a> {
    Query::new(
        jvm::query::Query::new(jvm, jvm::query::ScopeQuery::unscoped(), revision),
        java,
    )
}

fn file_model<'java>(
    java: &'java Language,
    revision: Revision,
    source: &jvm::model::Source,
) -> &'java model::File {
    java.model_at(source, revision).unwrap()
}

fn compilation_unit_site<'a>(source: &'a jvm::model::Source, file: &'a model::File) -> Site<'a> {
    Site {
        source,
        file,
        scope: model::LexicalScopeId(0),
    }
}

fn process(
    java: &mut Language,
    jvm: &mut jvm::Platform,
    revision: Revision,
    path: &str,
    contents: &str,
) -> jvm::model::Source {
    let source = source(path);
    java.process(source.clone(), revision, jvm, contents);
    source
}

/// A class the lake holds with no Java model behind it, which is what every
/// container but a source file contributes. Resolution can only ever answer with
/// a `TypeTarget::Compiled` for one, so what it knows about the declaration is
/// the binary name and the access level.
fn compiled_class(
    jvm: &mut jvm::Platform,
    revision: Revision,
    source: jvm::model::Source,
    fqn: &str,
    access: jvm::model::AccessLevel,
) -> jvm::model::Source {
    jvm.register(
        revision,
        source.clone(),
        vec![jvm::model::Class {
            fqn: jvm::model::BinaryName::new(fqn),
            kind: jvm::model::TypeKind::Class,
            access: Some(access),
            enclosing: None,
            superclass: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
        }],
    );
    source
}
