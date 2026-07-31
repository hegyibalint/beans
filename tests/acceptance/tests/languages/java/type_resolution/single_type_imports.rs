// JLS §§7.5.1 and 6.4.1.

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
fn duplicate_single_import_of_same_type_is_ignored() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; import q.X; class Test { <cur:target>X f; }",
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
fn importing_compilation_units_own_type_is_ignored() {
    fixture()
        .file(
            "p/X.java",
            "package p; import p.X; class X { <cur:target>X self; }",
        )
        .analyze("p/X.java")
        .resolves_to("target", "p.X")
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
fn canonical_member_type_can_be_imported() {
    fixture()
        .file(
            "q/Outer.java",
            "package q; public class Outer { public static class Inner {} }",
        )
        .file(
            "p/Test.java",
            "package p; import q.Outer.Inner; class Test { <cur:target>Inner f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.Outer.Inner")
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
fn same_type_from_single_type_and_single_static_import_is_deduplicated() {
    fixture()
        .file(
            "q/Host.java",
            "package q; public class Host { public static class X {} }",
        )
        .file(
            "p/Test.java",
            "package p; import q.Host.X; import static q.Host.X; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.Host.X")
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
fn imports_are_not_in_scope_in_later_import_declarations() {
    fixture()
        .file("java/util/Vector.java", "package java.util; public class Vector {}")
        .file("Vector/Mosquito.java", "package Vector; public class Mosquito {}")
        .file("p/Test.java", "package p; import java.util.Vector; import Vector.Mosquito; class Test { <cur:target>Mosquito f; }")
        .analyze("p/Test.java")
        .resolves_to("target", "Vector.Mosquito")
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
