use super::*;

#[test]
fn single_import_shadows_a_same_package_type_from_another_file() {
    let fixture = example();
    let results = fixture.superclass("src/app/Foo.java", "Foo");
    fixture.assert_type(&results[0], "src/library/Base.java", "library.Base");
}

#[test]
fn repeated_imports_of_the_same_declaration_are_deduplicated() {
    let fixture = Fixture::new(&[
        (
            "src/Use.java",
            "import p.Bar; import p.Bar; class Use { Bar target; }",
        ),
        ("src/p/Bar.java", "package p; public class Bar {}"),
    ]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/p/Bar.java", "p.Bar");
}

#[test]
fn equal_canonical_names_from_distinct_origins_remain_ambiguous() {
    let fixture = Fixture::new(&[
        ("src/Use.java", "import p.Bar; class Use { Bar target; }"),
        ("src/one/Bar.java", "package p; public class Bar {}"),
        ("src/two/Bar.java", "package p; public class Bar {}"),
    ]);
    let results = fixture.field("src/Use.java");
    let ResolutionResult::Ambiguous(candidates) = &results[0] else {
        panic!("expected ambiguity: {results:?}");
    };
    assert_eq!(candidates.len(), 2);
    assert_ne!(candidates[0].declaration, candidates[1].declaration);
}

#[test]
fn broken_single_import_does_not_fall_back_to_a_same_package_type() {
    let fixture = Fixture::new(&[
        (
            "src/app/Use.java",
            "package app; import missing.Bar; class Use { Bar target; }",
        ),
        ("src/app/Bar.java", "package app; class Bar {}"),
    ]);
    let results = fixture.field("src/app/Use.java");
    let ResolutionResult::Blocked {
        candidates,
        problems,
    } = &results[0]
    else {
        panic!("expected broken import: {results:?}");
    };
    assert!(candidates.is_empty());
    assert!(matches!(
        problems.as_slice(),
        [LookupProblem::InvalidImport(_)]
    ));
}

#[test]
fn canonical_member_import_checks_enclosing_type_accessibility() {
    let fixture = Fixture::new(&[
        (
            "src/Use.java",
            "import p.Outer.Hidden.Bar; class Use { Bar target; }",
        ),
        (
            "src/p/Outer.java",
            "package p; public class Outer { private static class Hidden { public static class Bar {} } }",
        ),
    ]);
    let results = fixture.field("src/Use.java");
    let ResolutionResult::Blocked {
        candidates,
        problems,
    } = &results[0]
    else {
        panic!("expected inaccessible enclosing type: {results:?}");
    };
    assert_eq!(candidates.len(), 1);
    assert!(matches!(
        problems.as_slice(),
        [LookupProblem::Inaccessible(_)]
    ));
}

#[test]
fn package_access_depends_on_the_use_site_package() {
    for (package, accessible) in [("p", true), ("other", false)] {
        let text = format!("package {package}; import p.Bar; class Use {{ Bar target; }}");
        let fixture = Fixture::new(&[
            ("src/Use.java", &text),
            ("src/p/Bar.java", "package p; class Bar {}"),
        ]);
        let results = fixture.field("src/Use.java");
        if accessible {
            fixture.assert_type(&results[0], "src/p/Bar.java", "p.Bar");
        } else {
            assert!(matches!(results[0], ResolutionResult::Blocked { .. }));
        }
    }
}

#[test]
fn imports_cannot_discover_sources_outside_the_classpath() {
    let fixture = Fixture::new(&[
        ("src/Use.java", "import p.Bar; class Use { Bar target; }"),
        ("hidden/Bar.java", "package p; public class Bar {}"),
    ]);
    let results = fixture.field("src/Use.java");
    assert!(matches!(results[0], ResolutionResult::Blocked { .. }));
}

#[test]
fn java_lang_lookup_uses_java_source_models_too() {
    let fixture = Fixture::new(&[
        ("src/Use.java", "class Use { String target; }"),
        (
            "src/java/lang/String.java",
            "package java.lang; public class String {}",
        ),
    ]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/java/lang/String.java", "java.lang.String");
}
