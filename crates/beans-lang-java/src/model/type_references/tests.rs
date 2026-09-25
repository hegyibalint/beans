use beans_core::model::{names::Name, ranges::ByteRange};

use super::type_reference_at;
use crate::lowering::lower_into;

#[test]
fn occurrence_lookup_selects_identifiers_and_nested_type_arguments() {
    let text = "class C<T extends Bound> extends Outer<String>.Inner<Integer> { int[] field; }";
    let file = lower_into(text);
    let at = |name: &str| text.find(name).unwrap();

    for name in ["Bound", "Outer", "String", "Inner", "Integer", "int"] {
        let occurrence = type_reference_at(&file, at(name)).unwrap();
        assert_eq!(
            &text[occurrence.range.start()..occurrence.range.end()],
            name
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

    for character in ["class", "C<", ".", "<String>", "[]", "field", " "] {
        let offset = at(character);
        // The whitespace in this fixture may be inside the surrounding type range,
        // but not inside an identifier.
        assert!(type_reference_at(&file, offset).is_none(), "{character}");
    }
    assert!(type_reference_at(&file, at("Outer") + "Outer".len()).is_none());
    assert!(type_reference_at(&file, text.len()).is_none());
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
    assert!(type_reference_at(&file, text.find('?').unwrap()).is_none());
}
