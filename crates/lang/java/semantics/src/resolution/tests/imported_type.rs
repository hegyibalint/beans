use super::{ResolutionInstance, ResolutionResult, lookup_field, lookup_field_with_query};
use crate::query::JavaQuery;
use beans_core_engine::{Revision, storage::RevisionedStorage};
use beans_core_model::{classpath::{Classpath, ClasspathElement}, source::Source};
use beans_lang_java_model::ScopeEntry;

fn with_dependencies(
    source: &str,
    dependencies: &[(&str, &str)],
    lookup: impl FnOnce(&ResolutionInstance<'_>, ScopeEntry<'_>) -> ResolutionResult,
) -> ResolutionResult {
    let revision = Revision::new(1);
    let mut files = RevisionedStorage::default();
    for (path, content) in dependencies {
        files.put(revision, Source::SourceFile { path: (*path).into() }, crate::lower_into(content));
    }
    let classpath = Classpath::new(vec![ClasspathElement::new("src".into(), [0; 32].into())]);
    let query = JavaQuery::new(&files, revision, &classpath);
    lookup_field_with_query(source, &query, lookup)
}

fn find(source: &str, dependencies: &[(&str, &str)]) -> ResolutionResult {
    with_dependencies(source, dependencies, |instance, _| {
        instance.find_single_imported_type(instance.type_ref)
    })
}

#[test]
fn a_single_import_resolves_its_public_top_level_target() {
    assert!(matches!(find(
        "package app; import p.Example; class Use { Example target; }",
        &[("src/p/Example.java", "package p; public class Example {}")],
    ), ResolutionResult::Resolved));
}

#[test]
fn an_import_spelling_alone_does_not_resolve_a_type() {
    assert!(matches!(find(
        "import p.Example; class Use { Example target; }",
        &[("src/q/Example.java", "package q; public class Example {}")],
    ), ResolutionResult::NotFound));
}

#[test]
fn package_access_requires_the_importing_file_to_be_in_the_same_package() {
    let dependencies = [("src/p/Example.java", "package p; class Example {}")];
    assert!(matches!(find(
        "package p; import p.Example; class Use { Example target; }", &dependencies,
    ), ResolutionResult::Resolved));
    assert!(matches!(find(
        "package other; import p.Example; class Use { Example target; }", &dependencies,
    ), ResolutionResult::NotFound));
}

#[test]
fn a_public_member_does_not_bypass_its_enclosing_types_access() {
    let result = find(
        "package app; import p.Outer.Member; class Use { Member target; }",
        &[(
            "src/p/Outer.java",
            "package p; class Outer { public class Member {} }",
        )],
    );
    assert!(matches!(result, ResolutionResult::NotFound));
}

#[test]
fn repeated_imports_of_the_same_declaration_are_not_ambiguous() {
    assert!(matches!(find(
        "import p.Example; import p.Example; class Use { Example target; }",
        &[("src/p/Example.java", "package p; public class Example {}")],
    ), ResolutionResult::Resolved));
}

#[test]
fn conflicting_imports_remain_ambiguous_during_error_recovery() {
    assert!(matches!(find(
        "import p.Example; import q.Example; class Use { Example target; }",
        &[
            ("src/p/Example.java", "package p; public class Example {}"),
            ("src/q/Example.java", "package q; public class Example {}"),
        ],
    ), ResolutionResult::Ambigous));
}

#[test]
fn duplicate_origins_are_not_collapsed_by_qualified_name() {
    assert!(matches!(find(
        "import p.Example; class Use { Example target; }",
        &[
            ("src/a/Example.java", "package p; public class Example {}"),
            ("src/b/Example.java", "package p; public class Example {}"),
        ],
    ), ResolutionResult::Ambigous));
}

#[test]
fn duplicate_declarations_are_not_collapsed_by_source() {
    assert!(matches!(find(
        "import p.Example; class Use { Example target; }",
        &[("src/p/Example.java", "package p; public class Example {} public class Example {}")],
    ), ResolutionResult::Ambigous));
}

#[test]
fn unnamed_package_types_cannot_be_imported() {
    assert!(matches!(find(
        "package app; import Example; class Use { Example target; }",
        &[("src/Example.java", "public class Example {}")],
    ), ResolutionResult::NotFound));
}

#[test]
fn resolving_a_prefix_does_not_resolve_an_unchecked_member_suffix() {
    let source = "import p.Example; class Use { Example.Missing target; }";
    let dependencies = [("src/p/Example.java", "package p; public class Example {}")];
    assert!(matches!(with_dependencies(source, &dependencies, |instance, _| instance.resolve()),
        ResolutionResult::NotFound));
}

#[test]
fn conflicting_starting_types_are_rejected_before_member_lookup() {
    assert!(matches!(find(
        "import p.Example; import q.Example; class Use { Example.Inner target; }",
        &[
            ("src/p/Example.java", "package p; public class Example {}"),
            ("src/q/Example.java", "package q; public class Example { public class Inner {} }"),
        ],
    ), ResolutionResult::Ambigous));
}

#[test]
fn lexical_hits_stop_before_conflicting_imports_during_error_recovery() {
    let result = with_dependencies(
        "import p.Example; import q.Example; class Use<Example> { Example target; }",
        &[
            ("src/p/Example.java", "package p; public class Example {}"),
            ("src/q/Example.java", "package q; public class Example {}"),
        ],
        |instance, _| instance.resolve(),
    );
    assert!(matches!(result, ResolutionResult::Resolved));
}

#[test]
fn single_import_lookup_ignores_on_demand_imports() {
    for import in ["import p.*;", "import static p.Container.*;"] {
        let source = format!("{import} class Use {{ Outer.Inner target; }}");
        let result = lookup_field(&source, |instance, _| {
            instance.find_single_imported_type(instance.type_ref)
        });

        assert!(matches!(result, ResolutionResult::NotFound));
    }
}

#[test]
fn single_import_lookup_leaves_non_named_references_to_other_resolution() {
    for field_type in ["int", "Outer[]"] {
        let source = format!("import p.Outer; import p.*; class Use {{ {field_type} target; }}");
        lookup_field(&source, |instance, _| {
            assert!(matches!(
                instance.find_single_imported_type(instance.type_ref),
                ResolutionResult::NotFound
            ));
            ResolutionResult::NotFound
        });
    }
}
