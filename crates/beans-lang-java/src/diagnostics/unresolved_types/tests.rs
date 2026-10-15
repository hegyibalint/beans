use beans_core::{
    classpath::Unrestricted, location::Location, query_scope::QueryScope, revision::Revision,
};
use beans_platform_jvm::JvmPlatform;

use crate::{
    diagnostics::{Diagnostic, DiagnosticSeverity},
    engine::JavaLanguage,
    lower_into,
    testing::Models,
};

fn diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut java = Models::new();
    let source = Location::file("/Example.java");
    java.store(Revision::new(1), lower_into(text), source.clone());
    JavaLanguage::new()
        .diagnostics(
            &java.connection,
            QueryScope::new(Revision::new(1), &Unrestricted),
            &JvmPlatform::default(),
            &source,
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
fn diagnostics_follow_the_requested_revision_and_cross_file_declarations() {
    let mut java = Models::new();
    let source = Location::file("/Example.java");
    let other = Location::file("/Missing.java");
    java.store(
        Revision::new(1),
        lower_into("class Example { Missing value; }"),
        source.clone(),
    );
    java.store(
        Revision::new(2),
        lower_into("class Missing {}"),
        other.clone(),
    );
    java.remove(Revision::new(3), other);
    java.remove(Revision::new(4), source.clone());
    for (revision, expected) in [(1, 1), (2, 0), (3, 1), (4, 0)] {
        assert_eq!(
            JavaLanguage::new()
                .diagnostics(
                    &java.connection,
                    QueryScope::new(Revision::new(revision), &Unrestricted),
                    &JvmPlatform::default(),
                    &source,
                )
                .unwrap()
                .len(),
            expected,
            "revision {revision}"
        );
    }
}
