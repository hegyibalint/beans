use super::*;

#[test]
fn resolves_a_top_level_type_by_its_package_spelling() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let resolved_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/X.java",
        "/* shift the package span */ package p; class X {}",
    );
    let current_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; class Test {}",
    );
    let resolved_declaration =
        file_model(&java, revision, &resolved_source).top_level_declarations[0];
    let current_file = file_model(&java, revision, &current_source);

    assert_eq!(
        resolve_from_same_package(
            &identifier("X"),
            current_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source: resolved_source,
            declaration: resolved_declaration,
        })
    );
}

#[test]
fn ignores_a_type_from_another_package() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    process(
        &mut java,
        &mut jvm,
        revision,
        "p/X.java",
        "package p; class X {}",
    );
    let current_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; class Test {}",
    );
    let current_file = file_model(&java, revision, &current_source);

    assert_eq!(
        resolve_from_same_package(
            &identifier("X"),
            current_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Unresolved
    );
}

#[test]
fn resolves_a_type_from_the_unnamed_package() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let resolved_source = process(&mut java, &mut jvm, revision, "X.java", "class X {}");
    let current_source = process(&mut java, &mut jvm, revision, "Test.java", "class Test {}");
    let resolved_declaration =
        file_model(&java, revision, &resolved_source).top_level_declarations[0];
    let current_file = file_model(&java, revision, &current_source);

    assert_eq!(
        resolve_from_same_package(
            &identifier("X"),
            current_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source: resolved_source,
            declaration: resolved_declaration,
        })
    );
}
