use beans_core::{compilation_scope::CompilationScopes, location::Location};

use beans_lang_java::{
    diagnostics::{Diagnostic, DiagnosticSeverity},
    engine::JavaLanguage,
    lower_into,
};

mod support;
use support::Models;

fn diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut java = Models::new();
    let source = Location::file("/Example.java");
    java.store(lower_into(text), source.clone());
    JavaLanguage::new()
        .diagnostics(
            &source,
            &CompilationScopes::unrestricted(),
            &java.connection,
        )
        .unwrap()
}

#[test]
fn unresolved_field_type_has_a_name_range_and_message() {
    let text = "class Example { Missing value; }";
    let result = diagnostics(text);
    assert_eq!(result.len(), 1);
    let diagnostic = &result[0];
    assert_eq!(
        &text[diagnostic.range.start()..diagnostic.range.end()],
        "Missing"
    );
    assert_eq!(diagnostic.code, "cannot-find-symbol");
    assert_eq!(diagnostic.message, "cannot find type: Missing");
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
}

#[test]
fn resolved_types_type_parameters_and_primitives_are_silent() {
    assert!(
        diagnostics(
            "class Known {} class Example<T> { Known value; T item; int[] counts; boolean flag; }"
        )
        .is_empty()
    );
}

#[test]
fn generic_arguments_wildcard_bounds_and_arrays_are_checked() {
    let text = "class Box<T> {} class Example { Box<Missing[]>[] a; Box<? extends Absent> b; Box<? super Unknown> c; Box<?> d; }";
    let result = diagnostics(text);
    let names: Vec<_> = result
        .iter()
        .map(|diagnostic| &text[diagnostic.range.start()..diagnostic.range.end()])
        .collect();
    assert_eq!(names, ["Missing", "Absent", "Unknown"]);
}

#[test]
fn unresolved_generic_type_range_excludes_its_arguments() {
    let text = "class Known {} class Example { Missing<Known>[] values; }";
    let result = diagnostics(text);
    assert_eq!(result.len(), 1);
    assert_eq!(
        &text[result[0].range.start()..result[0].range.end()],
        "Missing"
    );
}

#[test]
fn partial_ambiguous_and_unmodeled_positions_are_silent() {
    assert!(diagnostics("class Known {} class Example { Known.Missing value; }").is_empty());
    assert!(diagnostics("class Same {} class Same {} class Example { Same value; }").is_empty());
    assert!(diagnostics("class Example extends Missing {}").is_empty());
}

#[test]
fn diagnostics_follow_cross_file_replacement_and_deletion() {
    let mut java = Models::new();
    let source = Location::file("/Example.java");
    let other = Location::file("/Missing.java");
    java.store(
        lower_into("class Example { Missing value; }"),
        source.clone(),
    );
    let count = |java: &Models| {
        JavaLanguage::new()
            .diagnostics(
                &source,
                &CompilationScopes::unrestricted(),
                &java.connection,
            )
            .unwrap()
            .len()
    };
    assert_eq!(count(&java), 1);
    java.store(lower_into("class Missing {}"), other.clone());
    assert_eq!(count(&java), 0);
    java.store(lower_into("class Renamed {}"), other.clone());
    assert_eq!(count(&java), 1);
    java.store(lower_into("class Missing {}"), other.clone());
    assert_eq!(count(&java), 0);
    java.remove(other);
    assert_eq!(count(&java), 1);
    java.remove(source.clone());
    assert_eq!(count(&java), 0);
}
