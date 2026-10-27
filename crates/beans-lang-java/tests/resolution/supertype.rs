use beans_core::location::Location;
use beans_lang_java::{
    lower_into,
    model::{File, nodes::NodeIndex},
    semantics::resolution::{Context, JavaTypeCandidate, TypeCandidate, resolve_supertype},
};

use super::type_index;

fn resolve_declared_superclass(file: &File, owner: NodeIndex) -> TypeCandidate {
    let declaration = file.node(owner).unwrap().kind().as_type().unwrap();
    let source = Location::file("/Test.java");
    let ctx = Context::new(
        file,
        &source,
        owner,
        declaration.declared_superclass.as_ref().unwrap().value(),
    );

    resolve_supertype(&ctx).unwrap()
}

#[test]
fn members_declared_in_the_owners_body_are_not_in_supertype_scope() {
    let file =
        lower_into("class Outer { class Base {} class Child extends Base { class Base {} } }");
    let child = type_index(&file, &["Outer", "Child"]);

    let result = resolve_declared_superclass(&file, child);

    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = result else {
        panic!("expected one Java declaration, got {result:?}");
    };
    assert_eq!(handle.node_index(), type_index(&file, &["Outer", "Base"]));
}

#[test]
fn owner_type_parameters_are_in_supertype_scope() {
    let file = lower_into("class Outer { class Base {} class Child<Base> extends Base {} }");
    let child = type_index(&file, &["Outer", "Child"]);

    let result = resolve_declared_superclass(&file, child);

    let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(handle)) = result else {
        panic!("expected one Java type parameter, got {result:?}");
    };
    assert_eq!(handle.owner().node_index(), child);
    assert_eq!(handle.parameter(&file).unwrap().name.value(), "Base");
}
