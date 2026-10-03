use beans_core::{names::Name, ranges::ByteRange};

use super::type_reference_at;
use crate::lowering::lower_into;

#[test]
fn occurrence_lookup_selects_components_and_nested_type_arguments() {
    let text = "class C<T extends Bound> extends Outer<String>.Inner<Integer> { int[] field; }";
    let file = lower_into(text);
    let at = |name: &str| text.find(name).unwrap();

    for (name, selected) in [
        ("Bound", "Bound"),
        ("Outer", "Outer<String>"),
        ("String", "String"),
        ("Inner", "Inner<Integer>"),
        ("Integer", "Integer"),
        ("int", "int"),
    ] {
        let occurrence = type_reference_at(&file, at(name)).unwrap();
        assert_eq!(
            &text[occurrence.range.start()..occurrence.range.end()],
            selected
        );
        assert!(type_reference_at(&file, occurrence.range.end() - 1).is_some());
    }

    let declaration = file.find_type(&Name::new(vec!["C".into()]))[0].1;
    let superclass = declaration.declared_superclass.as_ref().unwrap();
    for name in ["Outer", "Inner"] {
        let occurrence = type_reference_at(&file, at(name)).unwrap();
        assert!(std::ptr::eq(occurrence.reference, superclass.value()));
        assert_eq!(occurrence.type_range, superclass.range());
    }
    let nested = type_reference_at(&file, at("String")).unwrap();
    assert!(!std::ptr::eq(nested.reference, superclass.value()));
    assert_eq!(
        &text[nested.type_range.start()..nested.type_range.end()],
        "String"
    );
    let array = type_reference_at(&file, at("int")).unwrap();
    assert_eq!(
        &text[array.type_range.start()..array.type_range.end()],
        "int[]"
    );

    for (character, component) in [
        ("<String>", "Outer<String>"),
        (">.Inner", "Outer<String>"),
        ("<Integer>", "Inner<Integer>"),
    ] {
        let occurrence = type_reference_at(&file, at(character)).unwrap();
        assert_eq!(
            &text[occurrence.range.start()..occurrence.range.end()],
            component
        );
    }

    for character in ["class", "C<", ".", "[]", "field", " "] {
        let offset = at(character);
        // The whitespace in this fixture is outside the modeled components.
        assert!(type_reference_at(&file, offset).is_none(), "{character}");
    }
    assert!(type_reference_at(&file, text.len()).is_none());
}

#[test]
fn nested_generic_arguments_take_priority_over_enclosing_components() {
    let text = "class C { First<List<A>> field; }";
    let file = lower_into(text);

    for (position, selected) in [
        (text.find("First").unwrap(), "First<List<A>>"),
        (
            text.find("First<").unwrap() + "First".len(),
            "First<List<A>>",
        ),
        (text.find("List").unwrap(), "List<A>"),
        (text.find("List<").unwrap() + "List".len(), "List<A>"),
        (text.find("A>>").unwrap(), "A"),
        (text.find("A>>").unwrap() + 1, "List<A>"),
        (text.find("A>>").unwrap() + 2, "First<List<A>>"),
    ] {
        let occurrence = type_reference_at(&file, position).unwrap();
        assert_eq!(
            &text[occurrence.range.start()..occurrence.range.end()],
            selected,
            "offset {position}"
        );
    }
}

#[test]
fn wildcard_bounds_are_type_references_but_wildcards_are_not() {
    let text = "class C implements Box<? extends Number> {}";
    let file = lower_into(text);
    let number = text.find("Number").unwrap();

    assert_eq!(
        type_reference_at(&file, number).unwrap().range,
        ByteRange::new(number, number + "Number".len())
    );
    let wildcard = type_reference_at(&file, text.find('?').unwrap()).unwrap();
    assert_eq!(
        &text[wildcard.range.start()..wildcard.range.end()],
        "Box<? extends Number>"
    );
}
