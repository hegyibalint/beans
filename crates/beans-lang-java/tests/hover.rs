use beans_core::{
    features::hover::{HoverProvider, HoverRequest, HoverResponse},
    location::{Location, LocationEntry},
    query_scope::QueryScope,
};
use beans_lang_java::{engine::JavaLanguage, features::JavaFeatureContext, lower_into};

mod support;
use support::Models;

#[test]
fn jvm_type_hover_shows_its_name_and_container_location() {
    let contents = "class C { String value; }";
    let source = source_id();
    let mut models = Models::new();
    models.store(lower_into(contents), source.clone());
    let location = Location::file("/jdk/lib/modules")
        .entry(LocationEntry::Container("java/lang/String.class".into()));
    let transaction = models.connection.transaction().unwrap();
    beans_platform_jvm::storage::store(
        &transaction,
        &beans_platform_jvm::Class {
            name: "java/lang/String".into(),
        },
        &location,
    )
    .unwrap();
    transaction.commit().unwrap();

    let response = hover(&models, &source, contents, contents.find("String").unwrap()).unwrap();

    assert_eq!(
        response.contents().plaintext(),
        "Type: java.lang.String\nLocated in: /jdk/lib/modules → java/lang/String.class"
    );
    assert_eq!(
        &contents[response.range().start()..response.range().end()],
        "String"
    );
}

#[test]
fn java_type_hover_shows_its_qualified_name_and_source() {
    let contents =
        "package demo; class Box<T> { class Slot {} } class C { Box<String>.Slot value; }";
    let source = Location::file("/src/demo/C.java");
    let mut models = Models::new();
    models.store(lower_into(contents), source.clone());

    for (component, name) in [("Box<String>", "demo.Box"), ("Slot value", "demo.Box.Slot")] {
        let response = hover(
            &models,
            &source,
            contents,
            contents.find(component).unwrap(),
        )
        .unwrap();
        assert_eq!(
            response.contents().plaintext(),
            format!("Type: {name}\nLocated in: /src/demo/C.java")
        );
    }
}

#[test]
fn hover_follows_replacement_and_removal() {
    let source = source_id();
    let mut models = Models::new();
    for (contents, expected) in [
        ("class C extends Former {}", "Former"),
        ("class C extends New {}", "New"),
    ] {
        models.store(lower_into(contents), source.clone());
        let response = hover(&models, &source, contents, 16).unwrap();
        assert_eq!(response.contents().plaintext(), expected);
    }
    models.remove(source.clone());
    assert!(hover(&models, &source, "class C extends New {}", 16).is_none());
}

#[test]
fn nested_type_argument_has_its_own_hover() {
    let contents = "class C extends Outer<String>.Inner {}";
    let mut models = Models::new();
    let source = source_id();
    models.store(lower_into(contents), source.clone());
    let offset = contents.find("String").unwrap();

    let response = hover(&models, &source, contents, offset).unwrap();

    assert_eq!(response.contents().plaintext(), "String");
    assert_eq!(
        &contents[response.range().start()..response.range().end()],
        "String"
    );
    assert!(hover(&models, &Location::file("C.class"), contents, offset).is_none());
}

#[test]
fn simple_type_hover_includes_its_type_arguments() {
    let contents = "class C extends Simple<Whatever> {}";
    let mut models = Models::new();
    let source = source_id();
    models.store(lower_into(contents), source.clone());

    let response = hover(&models, &source, contents, contents.find("Simple").unwrap()).unwrap();

    assert_eq!(response.contents().plaintext(), "Simple<Whatever>");
    assert_eq!(
        &contents[response.range().start()..response.range().end()],
        "Simple<Whatever>"
    );
}

#[test]
fn qualified_type_hover_includes_every_segment_and_its_arguments() {
    let contents = "class C extends Qualified<Asd>.Type<Asd2> {}";
    let mut models = Models::new();
    let source = source_id();
    models.store(lower_into(contents), source.clone());

    for (name, component) in [("Qualified", "Qualified<Asd>"), ("Type", "Type<Asd2>")] {
        let response = hover(&models, &source, contents, contents.find(name).unwrap()).unwrap();
        assert_eq!(response.contents().plaintext(), "Qualified<Asd>.Type<Asd2>");
        assert_eq!(
            &contents[response.range().start()..response.range().end()],
            component
        );
    }
    let nested = hover(&models, &source, contents, contents.find("Asd2").unwrap()).unwrap();
    assert_eq!(nested.contents().plaintext(), "Asd2");
}

#[test]
fn array_type_hover_includes_its_dimensions() {
    let contents = "class C { int[] counts; Simple<Whatever>[] values; }";
    let mut models = Models::new();
    let source = source_id();
    models.store(lower_into(contents), source.clone());

    for (name, expected, component) in [
        ("int", "int[]", "int"),
        ("Simple", "Simple<Whatever>[]", "Simple<Whatever>"),
        ("Whatever", "Whatever", "Whatever"),
    ] {
        let response = hover(&models, &source, contents, contents.find(name).unwrap()).unwrap();
        assert_eq!(response.contents().plaintext(), expected);
        assert_eq!(
            &contents[response.range().start()..response.range().end()],
            component
        );
    }
}

fn source_id() -> Location {
    Location::new(LocationEntry::Virtual("untitled:Example.java".into()))
}

fn hover(
    models: &Models,
    source: &Location,
    contents: &str,
    offset: usize,
) -> Option<HoverResponse> {
    JavaLanguage::new()
        .hover(
            &JavaFeatureContext {
                connection: &models.connection,
                scope: QueryScope::unrestricted(),
                workspace_root: None,
            },
            &HoverRequest {
                source,
                contents,
                offset,
            },
        )
        .unwrap()
}
