// JLS §§7.5.1 and 6.4.1.
//
// What a single-type import names is decided by `resolve_exact_imports`, so
// those cases live in `resolution/tests/imports.rs`: a duplicate import of one
// type, a canonical member type, a unit importing its own type, and an import
// that must not be in scope in a later import. One case stays here for the trip
// out to a user.
//
// The checks below are on the import declaration itself rather than on what it
// names, which is why they are pending against a diagnostic and not against a
// resolution.
//
// Dropped as unprovable for now: a single-type and a single-static import of one
// type are deduplicated. Static imports contribute nothing, so only one half of
// that ever ran.

use beans_acceptance::fixture::fixture;

#[test]
fn single_type_import_provides_simple_name() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.X")
        .run();
}

#[test]
fn distinct_single_imports_with_same_simple_name_collide() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file("r/X.java", "package r; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; import r.X; class Test {}",
        )
        .analyze("p/Test.java")
        .expect("import-collision")
        .expected_failure("import declaration checks are not implemented")
        .run();
}

#[test]
fn single_import_colliding_with_current_unit_type_is_error() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; class X {} class Test {}",
        )
        .analyze("p/Test.java")
        .expect("import-collision")
        .expected_failure("import declaration checks are not implemented")
        .run();
}

#[test]
fn single_import_of_missing_type_is_error() {
    fixture()
        .file("p/Test.java", "package p; import q.Missing; class Test {}")
        .analyze("p/Test.java")
        .expect("unresolvable-import")
        .expected_failure("import declaration checks are not implemented")
        .run();
}

#[test]
fn single_import_cannot_name_package() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file("p/Test.java", "package p; import q; class Test {}")
        .analyze("p/Test.java")
        .expect("unresolvable-import")
        .expected_failure("import declaration checks are not implemented")
        .run();
}

#[test]
fn single_import_of_inaccessible_type_is_error() {
    fixture()
        .file("q/Hidden.java", "package q; class Hidden {}")
        .file("p/Test.java", "package p; import q.Hidden; class Test {}")
        .analyze("p/Test.java")
        .expect("inaccessible-import")
        .expected_failure("import declaration checks are not implemented")
        .run();
}

#[test]
fn inherited_qualified_alias_is_not_a_canonical_import_name() {
    fixture()
        .file(
            "q/Base.java",
            "package q; public class Base { public static class Inner {} }",
        )
        .file("q/Sub.java", "package q; public class Sub extends Base {}")
        .file(
            "p/Test.java",
            "package p; import q.Sub.Inner; class Test {}",
        )
        .analyze("p/Test.java")
        .expect("non-canonical-import")
        .expected_failure("import declaration checks are not implemented")
        .run();
}

#[test]
fn distinct_types_from_single_type_and_single_static_import_collide() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "r/Host.java",
            "package r; public class Host { public static class X {} }",
        )
        .file(
            "p/Test.java",
            "package p; import q.X; import static r.Host.X; class Test {}",
        )
        .analyze("p/Test.java")
        .expect("import-collision")
        .expected_failure("import declaration checks are not implemented")
        .run();
}

#[test]
fn named_package_cannot_import_type_from_unnamed_package() {
    fixture()
        .file("X.java", "public class X {}")
        .file("p/Test.java", "package p; import X; class Test {}")
        .analyze("p/Test.java")
        .expect("unresolvable-import")
        .expected_failure("import declaration checks are not implemented")
        .run();
}
