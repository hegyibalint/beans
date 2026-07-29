use super::*;

#[test]
fn a_static_import_does_not_introduce_a_type_name_yet() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let current_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import static p.Outer.Inner; class Test {}",
    );
    let current_file = file_model(&java, revision, &current_source);

    assert_eq!(
        resolve_exact_imports(
            &identifier("Inner"),
            current_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Unresolved
    );
}

#[test]
fn an_exact_import_resolves_a_top_level_type() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let imported_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/X.java",
        "package p; class X {}",
    );
    let importing_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.X; class Test {}",
    );
    let imported_declaration =
        file_model(&java, revision, &imported_source).top_level_declarations[0];
    let importing_file = file_model(&java, revision, &importing_source);

    assert_eq!(
        resolve_exact_imports(
            &identifier("X"),
            importing_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source: imported_source,
            declaration: imported_declaration,
        })
    );
}

#[test]
fn an_exact_import_resolves_a_member_type() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let outer_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Outer.java",
        "package p; class Outer { class Inner {} }",
    );
    let importing_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.Outer.Inner; class Test {}",
    );
    let outer_file = file_model(&java, revision, &outer_source);
    let outer_scope = type_declaration(outer_file, outer_file.top_level_declarations[0]).body_scope;
    let inner = type_in_scope(outer_file, outer_scope, "Inner");
    let importing_file = file_model(&java, revision, &importing_source);

    assert_eq!(
        resolve_exact_imports(
            &identifier("Inner"),
            importing_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source: outer_source,
            declaration: inner,
        })
    );
}

#[test]
fn an_exact_import_does_not_skip_an_intermediate_name_segment() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    process(
        &mut java,
        &mut jvm,
        revision,
        "p/Inner.java",
        "package p; class Inner {}",
    );
    let importing_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.Outer.Inner; class Test {}",
    );
    let importing_file = file_model(&java, revision, &importing_source);

    assert_eq!(
        resolve_exact_imports(
            &identifier("Inner"),
            importing_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Unresolved
    );
}

#[test]
fn an_exact_import_uses_the_file_package_as_the_type_boundary() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let imported_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Outer/Inner.java",
        "package p.Outer; class Inner {}",
    );
    let importing_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.Outer.Inner; class Test {}",
    );
    let imported_declaration =
        file_model(&java, revision, &imported_source).top_level_declarations[0];
    let importing_file = file_model(&java, revision, &importing_source);

    assert_eq!(
        resolve_exact_imports(
            &identifier("Inner"),
            importing_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source: imported_source,
            declaration: imported_declaration,
        })
    );
}
