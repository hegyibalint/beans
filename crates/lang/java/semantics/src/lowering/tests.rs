use beans_lang_java_model::{
    File,
    nodes::{NodeIndex, NodeKind, types::TypeDeclaration},
    references::{TypeNameComponent, TypeRef},
};

struct TypeEntry<'a> {
    parent: NodeIndex,
    index: NodeIndex,
    declaration: &'a TypeDeclaration,
}

fn find_type_declarations<'a>(file: &'a File, name: &str) -> Vec<TypeEntry<'a>> {
    file.iter_nodes()
        .filter_map(|entry| {
            let NodeKind::Type(declaration) = entry.node.kind() else {
                return None;
            };

            (declaration.name.as_deref() == Some(name)).then_some(TypeEntry {
                parent: entry.node.parent().expect("type has a parent"),
                index: entry.index,
                declaration,
            })
        })
        .collect()
}

fn find_type_declaration<'a>(file: &'a File, name: &str) -> TypeEntry<'a> {
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
mod fields;
mod imports;
mod nodes;
mod type_declarations;
mod type_parameters;
mod type_references;
