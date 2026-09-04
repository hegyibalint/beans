use super::{find_type_declaration, named_segment, named_segments, raw_type};
use crate::lower_into;
use beans_lang_java_model::references::{TypeBound, TypeNameComponent, TypeRef};

fn first_superinterface_bound<'a>(
    file: &'a beans_lang_java_model::File,
    declaration: &str,
) -> &'a TypeBound {
    &named_segment(
        &find_type_declaration(file, declaration)
            .declaration
            .declared_superinterfaces[0],
    )
    .bounds[0]
}

#[test]
fn simple_and_qualified_type_names_preserve_their_segments() {
    let file = lower_into("class Simple extends Base {} class Qualified extends outer.Base {}");

    assert_eq!(
        find_type_declaration(&file, "Simple")
            .declaration
            .declared_superclass,
        Some(raw_type(&["Base"]))
    );
    assert_eq!(
        find_type_declaration(&file, "Qualified")
            .declaration
            .declared_superclass,
        Some(raw_type(&["outer", "Base"]))
    );
}

#[test]
fn zero_one_and_two_type_arguments_are_preserved() {
    let file = lower_into(
        "class Zero implements Box {}
         class One implements Box<Value> {}
         class Two implements Pair<First, Second> {}",
    );

    assert_eq!(
        named_segment(
            &find_type_declaration(&file, "Zero")
                .declaration
                .declared_superinterfaces[0],
        )
        .bounds,
        []
    );
    assert_eq!(
        named_segment(
            &find_type_declaration(&file, "One")
                .declaration
                .declared_superinterfaces[0],
        )
        .bounds,
        [TypeBound::Exact {
            primary: raw_type(&["Value"]),
        }]
    );
    assert_eq!(
        named_segment(
            &find_type_declaration(&file, "Two")
                .declaration
                .declared_superinterfaces[0],
        )
        .bounds,
        [
            TypeBound::Exact {
                primary: raw_type(&["First"]),
            },
            TypeBound::Exact {
                primary: raw_type(&["Second"]),
            },
        ]
    );
}

#[test]
fn exact_and_wildcard_type_arguments_are_distinguished() {
    let file = lower_into(
        "class Exact implements Box<Value> {}
         class Upper implements Box<? extends Number> {}
         class Lower implements Box<? super Number> {}
         class Any implements Box<?> {}",
    );

    assert_eq!(
        first_superinterface_bound(&file, "Exact"),
        &TypeBound::Exact {
            primary: raw_type(&["Value"]),
        }
    );
    assert_eq!(
        first_superinterface_bound(&file, "Upper"),
        &TypeBound::Extends {
            primary: raw_type(&["Number"]),
            additional: vec![],
        }
    );
    assert_eq!(
        first_superinterface_bound(&file, "Lower"),
        &TypeBound::Super {
            primary: raw_type(&["Number"]),
        }
    );
    assert_eq!(
        first_superinterface_bound(&file, "Any"),
        &TypeBound::Unbounded
    );
}

#[test]
fn type_arguments_are_attached_to_their_name_component() {
    let file = lower_into("class C implements Outer<String>.Inner<Integer> {}");
    let segments = named_segments(
        &find_type_declaration(&file, "C")
            .declaration
            .declared_superinterfaces[0],
    );

    assert_eq!(
        segments,
        [
            TypeNameComponent {
                name: "Outer".to_owned(),
                bounds: vec![TypeBound::Exact {
                    primary: raw_type(&["String"]),
                }],
            },
            TypeNameComponent {
                name: "Inner".to_owned(),
                bounds: vec![TypeBound::Exact {
                    primary: raw_type(&["Integer"]),
                }],
            },
        ]
    );
}

#[test]
fn nested_type_arguments_preserve_their_shape() {
    let file = lower_into("class C implements Map<String, List<Integer>> {}");
    let segment = named_segment(
        &find_type_declaration(&file, "C")
            .declaration
            .declared_superinterfaces[0],
    );

    assert_eq!(
        segment.bounds,
        [
            TypeBound::Exact {
                primary: raw_type(&["String"]),
            },
            TypeBound::Exact {
                primary: TypeRef::Named {
                    segments: vec![TypeNameComponent {
                        name: "List".to_owned(),
                        bounds: vec![TypeBound::Exact {
                            primary: raw_type(&["Integer"]),
                        }],
                    }],
                },
            },
        ]
    );
}

#[test]
fn array_type_arguments_preserve_one_and_two_dimensions() {
    let file = lower_into(
        "class One implements Box<String[]> {}
         class Two implements Box<String[][]> {}",
    );

    assert_eq!(
        first_superinterface_bound(&file, "One"),
        &TypeBound::Exact {
            primary: TypeRef::Array {
                element: Box::new(raw_type(&["String"])),
                dimensions: 1,
            },
        }
    );
    assert_eq!(
        first_superinterface_bound(&file, "Two"),
        &TypeBound::Exact {
            primary: TypeRef::Array {
                element: Box::new(raw_type(&["String"])),
                dimensions: 2,
            },
        }
    );
}
