use super::{find_type_declaration, raw_type};
use crate::lower_into;
use beans_lang_java_model::references::TypeBound;

#[test]
fn zero_one_and_two_type_parameters_are_preserved() {
    let file = lower_into("class Zero {} class One<T> {} class Two<T, U> {}");

    assert_eq!(
        find_type_declaration(&file, "Zero")
            .declaration
            .type_parameters,
        []
    );
    assert_eq!(
        find_type_declaration(&file, "One")
            .declaration
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>(),
        ["T"]
    );
    assert_eq!(
        find_type_declaration(&file, "Two")
            .declaration
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>(),
        ["T", "U"]
    );
}

#[test]
fn duplicate_type_parameter_names_are_preserved() {
    let file = lower_into("class A<T, T, T> {}");

    assert_eq!(
        find_type_declaration(&file, "A")
            .declaration
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>(),
        ["T", "T", "T"]
    );
}

#[test]
fn omitted_and_intersection_bounds_preserve_their_shape() {
    let file = lower_into(
        "class Bounds<
            Unbounded,
            Single extends A,
            Pair extends A & B,
            Triple extends A & B & C
        > {}",
    );
    let parameters = &find_type_declaration(&file, "Bounds")
        .declaration
        .type_parameters;

    assert_eq!(parameters[0].bounds, []);
    assert_eq!(
        parameters[1].bounds,
        [TypeBound::Extends {
            primary: raw_type(&["A"]),
            additional: vec![],
        }]
    );
    assert_eq!(
        parameters[2].bounds,
        [TypeBound::Extends {
            primary: raw_type(&["A"]),
            additional: vec![raw_type(&["B"])],
        }]
    );
    assert_eq!(
        parameters[3].bounds,
        [TypeBound::Extends {
            primary: raw_type(&["A"]),
            additional: vec![raw_type(&["B"]), raw_type(&["C"])],
        }]
    );
}

#[test]
fn annotations_do_not_obscure_a_type_parameter_bound() {
    let file = lower_into("class C<T extends @Marker package.Bound> {}");

    assert_eq!(
        find_type_declaration(&file, "C")
            .declaration
            .type_parameters[0]
            .bounds,
        [TypeBound::Extends {
            primary: raw_type(&["package", "Bound"]),
            additional: vec![],
        }]
    );
}
