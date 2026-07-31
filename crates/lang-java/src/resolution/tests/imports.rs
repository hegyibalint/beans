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

/// §6.5.4.2 walks the whole dotted name, so nesting below the package boundary
/// is followed to the end rather than stopping at the first type.
#[test]
fn an_exact_import_walks_nesting_below_the_boundary_to_the_end() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let outer_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/A.java",
        "package p; public class A { public static class B { public static class C {} } }",
    );
    let importing_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.A.B.C; class Test {}",
    );
    let outer_file = file_model(&java, revision, &outer_source);
    let a_scope = type_declaration(outer_file, outer_file.top_level_declarations[0]).body_scope;
    let b = type_in_scope(outer_file, a_scope, "B");
    let c = type_in_scope(outer_file, type_declaration(outer_file, b).body_scope, "C");
    let importing_file = file_model(&java, revision, &importing_source);

    assert_eq!(
        resolve_exact_imports(
            &identifier("C"),
            importing_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source: outer_source,
            declaration: c,
        })
    );
}

/// §7.5.1: an import is in scope for the compilation unit's types, not for the
/// import declarations themselves. `Vector` here names a package, and the type
/// imported on the line above must not obscure it.
#[test]
fn an_import_is_not_in_scope_in_a_later_import() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    process(
        &mut java,
        &mut jvm,
        revision,
        "java/util/Vector.java",
        "package java.util; public class Vector {}",
    );
    let mosquito_source = process(
        &mut java,
        &mut jvm,
        revision,
        "Vector/Mosquito.java",
        "package Vector; public class Mosquito {}",
    );
    let importing_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; import java.util.Vector; import Vector.Mosquito; class Test {}",
    );
    let mosquito = file_model(&java, revision, &mosquito_source).top_level_declarations[0];
    let importing_file = file_model(&java, revision, &importing_source);

    assert_eq!(
        resolve_exact_imports(
            &identifier("Mosquito"),
            importing_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source: mosquito_source,
            declaration: mosquito,
        })
    );
}

/// §7.5.1 has a compilation unit importing one of its own types be permitted and
/// have no effect. It resolves, and to the same declaration either way.
#[test]
fn an_import_of_the_compilation_units_own_type_names_that_type() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let own_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/X.java",
        "package p; import p.X; class X {}",
    );
    let own_file = file_model(&java, revision, &own_source);
    let own_declaration = own_file.top_level_declarations[0];

    assert_eq!(
        resolve_exact_imports(
            &identifier("X"),
            own_file,
            &java_query(&java, &jvm, revision)
        ),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source: own_source,
            declaration: own_declaration,
        })
    );
}
