use super::{find_type_declaration, find_type_declarations, raw_type};
use crate::lower_into;
use beans_lang_java_model::declarations::{
    Declaration,
    types::{AccessLevel, Kind, Modifier},
};

#[test]
fn declaration_kinds_are_preserved() {
    let file =
        lower_into("class C {} enum E {} record R() {} interface I {} @interface Annotation {}");

    assert_eq!(
        ["C", "E", "R", "I", "Annotation"]
            .map(|name| find_type_declaration(&file, name).declaration.kind),
        [
            Kind::Class,
            Kind::Enum,
            Kind::Record,
            Kind::Interface,
            Kind::AnnotationInterface,
        ]
    );
}

#[test]
fn two_declarations_preserve_their_names_and_source_order() {
    let file = lower_into("class First {} interface Second {}");

    assert_eq!(
        file.iter_declarations()
            .map(|scoped_declaration| {
                let Declaration::Type(declaration) = scoped_declaration.declaration.declaration
                else {
                    panic!("expected a type declaration");
                };
                declaration.name.as_deref()
            })
            .collect::<Vec<_>>(),
        [Some("First"), Some("Second")]
    );
}

#[test]
fn duplicate_declaration_names_remain_independently_findable() {
    let file = lower_into("class Duplicate {} interface Duplicate {}");
    let findings = find_type_declarations(&file, "Duplicate");
    let [first, second] = findings.as_slice() else {
        panic!("expected two declarations named `Duplicate`");
    };

    assert_eq!(first.declaring_scope_id, second.declaring_scope_id);
    assert_ne!(first.declaration_id, second.declaration_id);
    assert!(
        first
            .declaring_scope
            .iter_declarations(&file)
            .any(|indexed_declaration| indexed_declaration.index == first.declaration_id)
    );
    assert!(
        second
            .declaring_scope
            .iter_declarations(&file)
            .any(|indexed_declaration| indexed_declaration.index == second.declaration_id)
    );
    assert_eq!(first.declaration.kind, Kind::Class);
    assert_eq!(second.declaration.kind, Kind::Interface);
}

#[test]
fn superclass_can_be_absent_or_present() {
    let file = lower_into("class Plain {} class Child extends parent.Base {}");

    assert_eq!(
        find_type_declaration(&file, "Plain")
            .declaration
            .declared_superclass,
        None
    );
    assert_eq!(
        find_type_declaration(&file, "Child")
            .declaration
            .declared_superclass,
        Some(raw_type(&["parent", "Base"]))
    );
}

#[test]
fn zero_one_and_two_superinterfaces_are_preserved() {
    let file = lower_into(
        "class Zero {}
         class One implements First {}
         class Two implements First, second.Second {}",
    );

    assert_eq!(
        find_type_declaration(&file, "Zero")
            .declaration
            .declared_superinterfaces,
        []
    );
    assert_eq!(
        find_type_declaration(&file, "One")
            .declaration
            .declared_superinterfaces,
        [raw_type(&["First"])]
    );
    assert_eq!(
        find_type_declaration(&file, "Two")
            .declaration
            .declared_superinterfaces,
        [raw_type(&["First"]), raw_type(&["second", "Second"])]
    );
}

#[test]
fn declaration_kinds_share_the_superinterface_model() {
    let file = lower_into(
        "class C implements ClassInterface {}
         enum E implements EnumInterface {;}
         record R() implements RecordInterface {}
         interface I extends ParentInterface {}",
    );

    assert_eq!(
        find_type_declaration(&file, "C")
            .declaration
            .declared_superinterfaces,
        [raw_type(&["ClassInterface"])]
    );
    assert_eq!(
        find_type_declaration(&file, "E")
            .declaration
            .declared_superinterfaces,
        [raw_type(&["EnumInterface"])]
    );
    assert_eq!(
        find_type_declaration(&file, "R")
            .declaration
            .declared_superinterfaces,
        [raw_type(&["RecordInterface"])]
    );
    assert_eq!(
        find_type_declaration(&file, "I")
            .declaration
            .declared_superinterfaces,
        [raw_type(&["ParentInterface"])]
    );
}

#[test]
fn zero_one_and_two_access_levels_and_modifiers_are_preserved() {
    let file = lower_into(
        "class Zero {}
         public final class One {}
         public private abstract final class Two {}",
    );

    let zero = find_type_declaration(&file, "Zero");
    let one = find_type_declaration(&file, "One");
    let two = find_type_declaration(&file, "Two");

    assert_eq!(zero.declaration.access, []);
    assert_eq!(zero.declaration.modifiers, []);
    assert_eq!(one.declaration.access, [AccessLevel::Public]);
    assert_eq!(one.declaration.modifiers, [Modifier::Final]);
    assert_eq!(
        two.declaration.access,
        [AccessLevel::Public, AccessLevel::Private]
    );
    assert_eq!(
        two.declaration.modifiers,
        [Modifier::Abstract, Modifier::Final]
    );
}

#[test]
fn duplicate_conflicting_and_unusual_modifiers_preserve_source_order() {
    let file = lower_into(
        "public public protected private abstract abstract static final sealed non-sealed strictfp class C {}",
    );
    let declaration = find_type_declaration(&file, "C").declaration;

    assert_eq!(
        declaration.access,
        [
            AccessLevel::Public,
            AccessLevel::Public,
            AccessLevel::Protected,
            AccessLevel::Private,
        ]
    );
    assert_eq!(
        declaration.modifiers,
        [
            Modifier::Abstract,
            Modifier::Abstract,
            Modifier::Static,
            Modifier::Final,
            Modifier::Sealed,
            Modifier::NonSealed,
            Modifier::Strictfp,
        ]
    );
}
