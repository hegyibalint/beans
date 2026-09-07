use crate::{
    lower_into,
    resolution::{ResolutionInstance, ResolutionResult, Resolver},
};
use beans_lang_java_model::declarations::Declaration;

fn lookup_nested_field(type_name: &str) -> ResolutionResult {
    lookup_field(&format!(
        "class Outer<A> {{
             class Inner<B> {{
                 {type_name} target;
             }}
         }}"
    ))
}

fn lookup_field(source: &str) -> ResolutionResult {
    let file = lower_into(source);
    let (scope_index, field) = file
        .iter_declarations()
        .find_map(|entry| match entry.declaration {
            Declaration::Field(field) if field.name == "target" => {
                Some((entry.scope_index, field))
            }
            _ => None,
        })
        .expect("expected target field");
    let resolver = Resolver {};
    let classpath = beans_core_model::classpath::Classpath::default();

    ResolutionInstance::new(
        &resolver,
        &file,
        scope_index,
        &field.declared_type,
        &classpath,
    )
    .lookup_simple_type()
}

#[test]
fn nested_field_resolves_enclosing_type_parameter() {
    assert!(matches!(
        lookup_nested_field("A"),
        ResolutionResult::Resolved
    ));
}

#[test]
fn type_parameter_and_top_level_type_with_same_name_resolve_without_ambiguity() {
    let result = lookup_field(
        "class A {}
         class Outer<A> {
             class Inner<B> {
                 A target;
             }
         }",
    );

    assert!(matches!(result, ResolutionResult::Resolved));
}

#[test]
fn nested_field_with_unknown_type_is_not_found() {
    assert!(matches!(
        lookup_nested_field("X"),
        ResolutionResult::NotFound
    ));
}
