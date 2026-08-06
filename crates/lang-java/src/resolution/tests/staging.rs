// Which stage wins when several of them could answer. Each test below sets two
// declarations against each other and then removes the winner, so the assertion
// is about precedence rather than about only one candidate existing. Without
// that second half a stage that never runs looks exactly like a stage that runs
// and loses.
//
// JLS §6.4.1 is the rule; `resolve_type_name` is the reading of it, and the
// order of its stages is the whole of that reading.

use super::*;

/// `resolve_type_name` as a reference in a class body reaches it, answered with
/// the label a user would be shown.
fn resolve(
    java: &Language,
    jvm: &jvm::Platform,
    source: &jvm::model::Source,
    name: &str,
) -> Option<String> {
    let revision = Revision::default();
    let file = file_model(java, revision, source);
    let body = type_declaration(file, file.top_level_declarations[0]).body_scope;

    let resolution = resolve_type_name(
        &model::Name::Simple(identifier(name)),
        source,
        file,
        body,
        &java_query(java, jvm, revision),
    );

    let TypeResolution::Resolved(TypeTarget::Parsed {
        source,
        declaration,
    }) = resolution
    else {
        return None;
    };
    file_model(java, revision, &source).declaration_label(declaration)
}

/// A world holding `files`, with the last one as the file doing the asking.
fn asking(files: &[(&str, &str)]) -> (Language, jvm::Platform, jvm::model::Source) {
    let revision = Revision::default();
    let mut java = Language::new();
    let mut jvm = jvm::Platform::new();
    let mut asker = None;
    for (path, contents) in files {
        asker = Some(process(&mut java, &mut jvm, revision, path, contents));
    }
    (java, jvm, asker.expect("at least one file"))
}

/// Stage 1 over stage 3. §6.4.1: a type declaration shadows every other type of
/// that name in scope where it occurs.
#[test]
fn a_member_type_shadows_a_type_of_the_same_package() {
    let (java, jvm, asker) = asking(&[
        ("p/X.java", "package p; class X {}"),
        ("p/Test.java", "package p; class Test { class X {} }"),
    ]);
    assert_eq!(
        resolve(&java, &jvm, &asker, "X").as_deref(),
        Some("p.Test.X")
    );

    // Without the member type, stage 3 is what answers, so the assertion above
    // is about the order of the two and not about `p.X` being unreachable.
    let (java, jvm, asker) = asking(&[
        ("p/X.java", "package p; class X {}"),
        ("p/Test.java", "package p; class Test {}"),
    ]);
    assert_eq!(resolve(&java, &jvm, &asker, "X").as_deref(), Some("p.X"));
}

/// Stage 1 over stage 2.
#[test]
fn a_member_type_shadows_a_single_type_import() {
    let (java, jvm, asker) = asking(&[
        ("q/X.java", "package q; public class X {}"),
        (
            "p/Test.java",
            "package p; import q.X; class Test { class X {} }",
        ),
    ]);
    assert_eq!(
        resolve(&java, &jvm, &asker, "X").as_deref(),
        Some("p.Test.X")
    );

    let (java, jvm, asker) = asking(&[
        ("q/X.java", "package q; public class X {}"),
        ("p/Test.java", "package p; import q.X; class Test {}"),
    ]);
    assert_eq!(resolve(&java, &jvm, &asker, "X").as_deref(), Some("q.X"));
}

/// Stage 2 over stage 3. §6.4.1 has a single-type import shadow a top-level type
/// of that name declared in *another* compilation unit of this package, which is
/// why the sibling below loses and a member type above would not.
#[test]
fn a_single_type_import_shadows_a_sibling_of_the_same_package() {
    let (java, jvm, asker) = asking(&[
        ("q/X.java", "package q; public class X {}"),
        ("p/X.java", "package p; class X {}"),
        ("p/Test.java", "package p; import q.X; class Test {}"),
    ]);
    assert_eq!(resolve(&java, &jvm, &asker, "X").as_deref(), Some("q.X"));

    let (java, jvm, asker) = asking(&[
        ("q/X.java", "package q; public class X {}"),
        ("p/X.java", "package p; class X {}"),
        ("p/Test.java", "package p; class Test {}"),
    ]);
    assert_eq!(resolve(&java, &jvm, &asker, "X").as_deref(), Some("p.X"));
}

/// Stage 2 over stage 4. §6.4.1: a single-type import shadows "any type named n
/// imported by a type-import-on-demand declaration in c", and §7.3 makes the
/// implicit `java.lang` one of those.
#[test]
fn a_single_type_import_shadows_the_implicit_java_lang_import() {
    let (java, jvm, asker) = asking(&[
        ("java/lang/X.java", "package java.lang; public class X {}"),
        ("q/X.java", "package q; public class X {}"),
        ("p/Test.java", "package p; import q.X; class Test {}"),
    ]);
    assert_eq!(resolve(&java, &jvm, &asker, "X").as_deref(), Some("q.X"));

    let (java, jvm, asker) = asking(&[
        ("java/lang/X.java", "package java.lang; public class X {}"),
        ("p/Test.java", "package p; class Test {}"),
    ]);
    assert_eq!(
        resolve(&java, &jvm, &asker, "X").as_deref(),
        Some("java.lang.X")
    );
}

/// Stage 3 over stage 4. §7.5.2's Example 7.5.2-1 says an on-demand import
/// "might be shadowed [...] by a class or interface named `Vector` and declared
/// in the package to which the compilation unit belongs".
#[test]
fn a_type_of_the_same_package_shadows_the_implicit_java_lang_import() {
    let (java, jvm, asker) = asking(&[
        ("java/lang/X.java", "package java.lang; public class X {}"),
        ("p/X.java", "package p; class X {}"),
        ("p/Test.java", "package p; class Test {}"),
    ]);
    assert_eq!(resolve(&java, &jvm, &asker, "X").as_deref(), Some("p.X"));

    let (java, jvm, asker) = asking(&[
        ("java/lang/X.java", "package java.lang; public class X {}"),
        ("p/Test.java", "package p; class Test {}"),
    ]);
    assert_eq!(
        resolve(&java, &jvm, &asker, "X").as_deref(),
        Some("java.lang.X")
    );
}

#[test]
fn an_inaccessible_type_is_returned_with_an_unresolved_name() {
    let (java, jvm, asker) = asking(&[
        ("q/X.java", "package q; class X {}"),
        ("p/Test.java", "package p; import q.X; class Test {}"),
    ]);
    let file = file_model(&java, Revision::default(), &asker);
    let body = type_declaration(file, file.top_level_declarations[0]).body_scope;

    let TypeResolution::Unresolved { invalid_candidates } = resolve_type_name(
        &model::Name::Simple(identifier("X")),
        &asker,
        file,
        body,
        &java_query(&java, &jvm, Revision::default()),
    ) else {
        panic!("an inaccessible type must not resolve");
    };

    assert_eq!(invalid_candidates.len(), 1);
    assert!(invalid_candidates[0].has_invalidity(TypeInvalidity::Inaccessible));
}

/// §7.5.1 makes the inaccessible import a compile-time error. The JLS need not
/// recover meaning for the broken compilation unit; javac retains that error
/// while resolving the later simple name from the same package, and Beans does
/// the same.
#[test]
fn an_inaccessible_import_does_not_hide_an_accessible_same_package_type() {
    let (java, jvm, asker) = asking(&[
        ("q/X.java", "package q; class X {}"),
        ("p/X.java", "package p; class X {}"),
        ("p/Test.java", "package p; import q.X; class Test {}"),
    ]);

    assert_eq!(resolve(&java, &jvm, &asker, "X").as_deref(), Some("p.X"));

    let file = file_model(&java, Revision::default(), &asker);
    let body = type_declaration(file, file.top_level_declarations[0]).body_scope;
    let candidates = resolve_type_candidates(
        &model::Name::Simple(identifier("X")),
        &asker,
        file,
        body,
        &java_query(&java, &jvm, Revision::default()),
    );
    assert_eq!(candidates.valid.len(), 1);
    assert!(candidates.has_invalidity(TypeInvalidity::Inaccessible));
}

#[test]
fn an_outside_scope_import_does_not_hide_an_in_scope_package_type() {
    let revision = Revision::default();
    let mut java = Language::new();
    let mut jvm = jvm::Platform::new();
    process(
        &mut java,
        &mut jvm,
        revision,
        "dependency/q/X.java",
        "package q; public class X {}",
    );
    let package_source = process(
        &mut java,
        &mut jvm,
        revision,
        "app/p/X.java",
        "package p; class X {}",
    );
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "app/p/Test.java",
        "package p; import q.X; class Test {}",
    );
    jvm.register_scopes(
        revision,
        asker.clone(),
        vec![jvm::query::Scope::of(vec![jvm::query::Container::Source(
            PathBuf::from("app"),
        )])],
    );
    let file = file_model(&java, revision, &asker);
    let body = type_declaration(file, file.top_level_declarations[0]).body_scope;
    let package_declaration =
        file_model(&java, revision, &package_source).top_level_declarations[0];
    let query = Query::new(jvm.query_from(&asker, revision), &java);

    let candidates = resolve_type_candidates(
        &model::Name::Simple(identifier("X")),
        &asker,
        file,
        body,
        &query,
    );
    assert_eq!(
        candidates.clone().into_resolution(),
        TypeResolution::Resolved(TypeTarget::Parsed {
            source: package_source,
            declaration: package_declaration,
        })
    );
    assert!(candidates.has_invalidity(TypeInvalidity::OutsideScope));
}

#[test]
fn a_name_known_only_outside_scope_is_unresolved() {
    let revision = Revision::default();
    let mut java = Language::new();
    let mut jvm = jvm::Platform::new();
    let outside_source = process(
        &mut java,
        &mut jvm,
        revision,
        "dependency/p/X.java",
        "package p; class X {}",
    );
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "app/p/Test.java",
        "package p; class Test {}",
    );
    jvm.register_scopes(
        revision,
        asker.clone(),
        vec![jvm::query::Scope::of(vec![jvm::query::Container::Source(
            PathBuf::from("app"),
        )])],
    );
    let file = file_model(&java, revision, &asker);
    let body = type_declaration(file, file.top_level_declarations[0]).body_scope;
    let outside_declaration =
        file_model(&java, revision, &outside_source).top_level_declarations[0];
    let query = Query::new(jvm.query_from(&asker, revision), &java);

    let candidates = query.types_named(&jvm::model::BinaryName::new("p.X"));
    assert_eq!(
        candidates,
        vec![TypeTarget::Parsed {
            source: outside_source,
            declaration: outside_declaration,
        }]
    );
    assert_eq!(
        query.scope_membership(&candidates[0]),
        jvm::query::ScopeMembership::OutsideScope
    );
    let candidates = resolve_type_candidates(
        &model::Name::Simple(identifier("X")),
        &asker,
        file,
        body,
        &query,
    );
    let TypeResolution::Unresolved { invalid_candidates } = candidates.into_resolution() else {
        panic!("an outside-scope candidate must not resolve");
    };
    assert_eq!(invalid_candidates.len(), 1);
    assert!(invalid_candidates[0].has_invalidity(TypeInvalidity::OutsideScope));
}

/// Nothing in scope at all. The stages run out rather than guessing, which is
/// what keeps a missing import from resolving to whatever else is in the lake.
#[test]
fn a_name_no_stage_answers_is_unresolved() {
    let (java, jvm, asker) = asking(&[
        ("q/X.java", "package q; public class X {}"),
        ("p/Test.java", "package p; class Test {}"),
    ]);

    assert_eq!(resolve(&java, &jvm, &asker, "X"), None);
}

/// A qualified name is not staged at all yet; §6.5.5.2 classifies its prefix
/// first, and `resolve_type_name` returns before stage 1 rather than treating
/// the last segment as a simple name.
#[test]
fn a_qualified_name_does_not_fall_through_to_the_stages() {
    let revision = Revision::default();
    let mut java = Language::new();
    let mut jvm = jvm::Platform::new();
    process(
        &mut java,
        &mut jvm,
        revision,
        "q/X.java",
        "package q; public class X {}",
    );
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; class Test {}",
    );

    let file = file_model(&java, revision, &asker);
    let body = type_declaration(file, file.top_level_declarations[0]).body_scope;
    let qualified = model::Name::Qualified(model::QualifiedName::new(
        vec![identifier("q"), identifier("X")],
        OffsetSpan {
            start: Offset(0),
            end: Offset(3),
        },
    ));

    assert_eq!(
        resolve_type_name(
            &qualified,
            &asker,
            file,
            body,
            &java_query(&java, &jvm, revision)
        ),
        TypeResolution::Unresolved {
            invalid_candidates: Vec::new(),
        }
    );
}
