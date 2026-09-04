use beans_lang_java_model::{
    File,
    declarations::{Declaration, DeclarationIndex, types::TypeDeclaration},
    references::{TypeNameComponent, TypeRef},
    scopes::{Scope, ScopeIndex},
};

struct ScopedTypeDeclaration<'a> {
    scope_id: ScopeIndex,
    scope: &'a Scope,
    declaration_id: DeclarationIndex,
    declaration: &'a TypeDeclaration,
}

fn find_type_declarations<'a>(file: &'a File, name: &str) -> Vec<ScopedTypeDeclaration<'a>> {
    file.iter_declarations()
        .filter_map(|scoped_declaration| {
            let Declaration::Type(declaration) = scoped_declaration.declaration.declaration else {
                return None;
            };

            (declaration.name.as_deref() == Some(name)).then_some(ScopedTypeDeclaration {
                scope_id: scoped_declaration.scope.index,
                scope: scoped_declaration.scope.scope,
                declaration_id: scoped_declaration.declaration.index,
                declaration,
            })
        })
        .collect()
}

fn find_type_declaration<'a>(file: &'a File, name: &str) -> ScopedTypeDeclaration<'a> {
    let mut findings = find_type_declarations(file, name);

    match findings.len() {
        1 => findings.pop().unwrap(),
        0 => panic!("expected one type declaration named `{name}`, found none"),
        count => panic!("expected one type declaration named `{name}`, found {count}"),
    }
}

fn raw_type(names: &[&str]) -> TypeRef {
    TypeRef::Named {
        segments: names
            .iter()
            .map(|name| TypeNameComponent {
                name: (*name).to_owned(),
                bounds: Vec::new(),
            })
            .collect(),
    }
}

fn named_segments(reference: &TypeRef) -> &[TypeNameComponent] {
    let TypeRef::Named { segments } = reference else {
        panic!("expected a named type reference");
    };

    segments
}

fn named_segment(reference: &TypeRef) -> &TypeNameComponent {
    let segments = named_segments(reference);
    let [segment] = segments else {
        panic!(
            "expected a named type reference with exactly one segment, found {}",
            segments.len()
        );
    };

    segment
}

mod compilation_units;
mod imports;
mod scopes;
mod type_declarations;
mod type_parameters;
mod type_references;
