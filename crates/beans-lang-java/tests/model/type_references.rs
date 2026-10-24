use beans_core::ranges::ByteRange;
use beans_lang_java::lower_into;

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
        let range = file.type_reference_at(at(name)).unwrap();
        assert_eq!(&text[range.start()..range.end()], selected);
        assert!(file.type_reference_at(range.end() - 1).is_some());
    }

    for (character, component) in [
        ("<String>", "Outer<String>"),
        (">.Inner", "Outer<String>"),
        ("<Integer>", "Inner<Integer>"),
    ] {
        let range = file.type_reference_at(at(character)).unwrap();
        assert_eq!(&text[range.start()..range.end()], component);
    }

    for character in ["class", "C<", ".", "[]", "field", " "] {
        assert!(
            file.type_reference_at(at(character)).is_none(),
            "{character}"
        );
    }
    assert!(file.type_reference_at(text.len()).is_none());
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
        let range = file.type_reference_at(position).unwrap();
        assert_eq!(
            &text[range.start()..range.end()],
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
        file.type_reference_at(number).unwrap(),
        ByteRange::new(number, number + "Number".len())
    );
    let wildcard = file.type_reference_at(text.find('?').unwrap()).unwrap();
    assert_eq!(
        &text[wildcard.start()..wildcard.end()],
        "Box<? extends Number>"
    );
}
