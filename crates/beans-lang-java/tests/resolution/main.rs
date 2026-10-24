use beans_core::{location::Location, names::Name};
use beans_lang_java::{
    model::{
        File,
        nodes::{NodeIndex, NodeKind},
    },
    semantics::resolution::{
        Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, resolve,
    },
};

#[path = "../support/mod.rs"]
mod support;

mod compilation_unit;
mod external;
mod inheritance;
mod lexical;
mod supertype;
mod type_parameter_path;
mod validation;

fn type_index(file: &File, components: &[&str]) -> NodeIndex {
    let name = Name::new(
        components
            .iter()
            .map(|component| (*component).to_owned())
            .collect(),
    );
    let matches = file.find_type(&name);
    assert_eq!(matches.len(), 1);
    matches[0].0
}

fn resolve_field(file: &File, field_name: &str) -> Result<TypeCandidate, ResolutionFailure> {
    let (index, field) = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) if field.name == field_name => Some((entry.index, field)),
            _ => None,
        })
        .expect("expected field");
    let source = Location::file("/Test.java");
    let ctx = Context::new(&source, file, index, field.declared_type.value());
    resolve(&ctx)
}

fn resolved_java_index(result: TypeCandidate) -> NodeIndex {
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = result else {
        panic!("expected one Java declaration");
    };
    handle.node_index()
}
