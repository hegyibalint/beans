use beans_core::{
    classpath::Unrestricted,
    features::navigation::{DefinitionProvider, DefinitionRequest},
    location::{Location, LocationEntry},
    query_scope::QueryScope,
    ranges::ByteRange,
    source::SourceSpan,
};
use beans_lang_java::{engine::JavaLanguage, features::JavaFeatureContext, lower_into};

mod support;
use support::Models;

#[test]
fn resolved_field_type_projects_to_its_declaration_name() {
    let text = "class Outer { class Member {} Member field; }";
    let source = source_id();
    let mut java = Models::new();
    java.store(lower_into(text), source.clone());
    let name_start = text.find("Member").unwrap();

    assert_eq!(
        definition_at(&java, &source, text.rfind("Member").unwrap()),
        Some(SourceSpan::new(
            source,
            ByteRange::new(name_start, name_start + "Member".len())
        ))
    );
}

#[test]
fn qualified_field_type_navigates_each_component_to_its_own_declaration() {
    let text = "class Box<T> { class Slot<U> {} } class Example { Box<String>.Slot<Integer> nested; Box<? extends Number> numbers; }";
    let source = source_id();
    let mut java = Models::new();
    java.store(lower_into(text), source.clone());
    let box_name = text.find("class Box").unwrap() + "class ".len();
    let slot_name = text.find("class Slot").unwrap() + "class ".len();

    for (use_offset, target_offset, name) in [
        (text.find("Box<String>").unwrap(), box_name, "Box"),
        (text.find("Slot<Integer>").unwrap(), slot_name, "Slot"),
        (text.find("Box<?").unwrap(), box_name, "Box"),
    ] {
        assert_eq!(
            definition_at(&java, &source, use_offset),
            Some(SourceSpan::new(
                source.clone(),
                ByteRange::new(target_offset, target_offset + name.len())
            )),
            "{name} at {use_offset}"
        );
    }
    assert_eq!(
        definition_at(&java, &source, text.find(".Slot").unwrap()),
        None
    );
}

#[test]
fn generic_components_and_their_arguments_navigate_independently() {
    let text = "class A {} class B {} class First<T> { class Second<U> {} } class Use { First<A>.Second<B> field; }";
    let source = source_id();
    let mut java = Models::new();
    java.store(lower_into(text), source.clone());
    let first = text.find("First<A>").unwrap();
    let second = text.find("Second<B>").unwrap();
    let declaration = |name: &str| {
        let start = text.find(&format!("class {name}")).unwrap() + "class ".len();
        Some(SourceSpan::new(
            source.clone(),
            ByteRange::new(start, start + name.len()),
        ))
    };

    for (offset, name, origin) in [
        (first, "First", "First<A>"),
        (first + "First".len(), "First", "First<A>"),
        (first + "First<A>".len() - 1, "First", "First<A>"),
        (first + "First<".len(), "A", "A"),
        (second, "Second", "Second<B>"),
        (second + "Second".len(), "Second", "Second<B>"),
        (second + "Second<B>".len() - 1, "Second", "Second<B>"),
        (second + "Second<".len(), "B", "B"),
    ] {
        let definition = JavaLanguage::new()
            .goto_definition(
                &JavaFeatureContext {
                    connection: &java.connection,
                    scope: QueryScope::new(&Unrestricted),
                    workspace_root: None,
                },
                &DefinitionRequest {
                    source: &source,
                    offset,
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(Some(definition.target), declaration(name));
        assert_eq!(
            &text[definition.origin_range.start()..definition.origin_range.end()],
            origin
        );
    }
    assert_eq!(
        definition_at(&java, &source, first + "First<A>".len()),
        None
    );
}

#[test]
fn field_type_parameter_navigates_to_its_declaration_name() {
    // JLS §6.3 and §6.5.5.1: the class type parameter is in scope in its body.
    let text = "class Example<T extends Labelled> { T label; }";
    let source = source_id();
    let mut java = Models::new();
    java.store(lower_into(text), source.clone());
    let declaration = text.find("<T").unwrap() + 1;
    let use_offset = text.find("T label").unwrap();

    assert_eq!(
        definition_at(&java, &source, use_offset),
        Some(SourceSpan::new(
            source,
            ByteRange::new(declaration, declaration + 1)
        ))
    );
}

#[test]
fn type_parameter_declaration_names_navigate_to_themselves() {
    let text = "class Box<T, U extends Labelled> { U label; }";
    let source = source_id();
    let mut java = Models::new();
    java.store(lower_into(text), source.clone());

    for offset in [
        text.find("<T").unwrap() + 1,
        text.find("U extends").unwrap(),
    ] {
        assert_eq!(
            definition_at(&java, &source, offset),
            Some(SourceSpan::new(
                source.clone(),
                ByteRange::new(offset, offset + 1)
            ))
        );
    }
    assert_eq!(
        definition_at(&java, &source, text.find("<T").unwrap() + 2),
        Some(SourceSpan::new(
            source.clone(),
            ByteRange::new(text.find("<T").unwrap() + 1, text.find("<T").unwrap() + 2)
        ))
    );
    for offset in [
        text.find("<T").unwrap(),
        text.find("U extends").unwrap() + 2,
    ] {
        assert_eq!(definition_at(&java, &source, offset), None);
    }
}

#[test]
fn field_type_end_positions_navigate_when_entered_from_the_right() {
    let text = "class Box<T> { class Slot<U> {} T value; Box<String>.Slot<Integer> nested; }";
    let source = source_id();
    let mut java = Models::new();
    java.store(lower_into(text), source.clone());

    for (use_start, target_start, name) in [
        (
            text.find("T value").unwrap(),
            text.find("<T").unwrap() + 1,
            "T",
        ),
        (
            text.find("Box<String>").unwrap(),
            text.find("class Box").unwrap() + 6,
            "Box",
        ),
        (
            text.find("Slot<Integer>").unwrap(),
            text.find("class Slot").unwrap() + 6,
            "Slot",
        ),
    ] {
        assert_eq!(
            definition_at(&java, &source, use_start + name.len()),
            Some(SourceSpan::new(
                source.clone(),
                ByteRange::new(target_start, target_start + name.len())
            )),
            "end of {name}"
        );
    }
}

#[test]
fn unrelated_positions_do_not_resolve_to_field_types() {
    let text = "class Outer { class Member {} Member field; int count; }";
    let source = source_id();
    let mut java = Models::new();
    java.store(lower_into(text), source.clone());

    for offset in [
        text.find("class").unwrap(),
        text.find("Member").unwrap(),
        text.find("field").unwrap(),
        text.find("int").unwrap(),
        text.find("count").unwrap(),
        text.len(),
    ] {
        assert_eq!(
            definition_at(&java, &source, offset),
            None,
            "offset {offset}"
        );
    }
}

#[test]
fn an_unresolved_field_type_has_no_definition() {
    let text = "class Outer { Missing field; }";
    let source = source_id();
    let mut java = Models::new();
    java.store(lower_into(text), source.clone());

    assert_eq!(
        definition_at(&java, &source, text.find("Missing").unwrap()),
        None
    );
}

fn source_id() -> Location {
    Location::new(LocationEntry::Virtual("untitled:Example.java".into()))
}

fn definition_at(java: &Models, source: &Location, offset: usize) -> Option<SourceSpan> {
    JavaLanguage::new()
        .goto_definition(
            &JavaFeatureContext {
                connection: &java.connection,
                scope: QueryScope::new(&Unrestricted),
                workspace_root: None,
            },
            &DefinitionRequest { source, offset },
        )
        .unwrap()
        .map(|definition| definition.target)
}
