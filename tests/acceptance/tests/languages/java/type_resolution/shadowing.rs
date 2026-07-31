// JLS §6.4.1.

use beans_acceptance::fixture::fixture;

#[test]
fn member_type_shadows_same_package_top_level_type() {
    fixture()
        .file(
            "p/Test.java",
            "package p; class X {} class Test { class X {} <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.Test.X")
        .run();
}

#[test]
fn member_type_shadows_single_type_import() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; class Test { class X {} <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.Test.X")
        .run();
}

#[test]
fn inherited_member_type_shadows_single_type_import() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file("p/Base.java", "package p; class Base { static class X {} }")
        .file(
            "p/Test.java",
            "package p; import q.X; class Test extends Base { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.Base.X")
        .expected_failure("inherited member types are not resolved")
        .run();
}

#[test]
fn single_type_import_shadows_same_package_sibling() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file("p/X.java", "package p; class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.X")
        .run();
}

#[test]
fn same_package_type_shadows_type_import_on_demand() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file("p/X.java", "package p; class X {}")
        .file(
            "p/Test.java",
            "package p; import q.*; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.X")
        .run();
}

#[test]
fn same_package_type_shadows_static_import_on_demand() {
    fixture()
        .file(
            "q/Host.java",
            "package q; public class Host { public static class X {} }",
        )
        .file("p/X.java", "package p; class X {}")
        .file(
            "p/Test.java",
            "package p; import static q.Host.*; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.X")
        .run();
}

#[test]
fn same_package_type_shadows_implicit_java_lang() {
    fixture()
        .file(
            "java/lang/String.java",
            "package java.lang; public class String {}",
        )
        .file("p/String.java", "package p; class String {}")
        .file(
            "p/Test.java",
            "package p; class Test { <cur:target>String f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.String")
        .run();
}

#[test]
fn single_type_import_shadows_type_import_on_demand() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file("r/X.java", "package r; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; import r.*; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.X")
        .run();
}

#[test]
fn single_type_import_shadows_static_import_on_demand() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "r/Host.java",
            "package r; public class Host { public static class X {} }",
        )
        .file(
            "p/Test.java",
            "package p; import q.X; import static r.Host.*; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.X")
        .run();
}

#[test]
fn single_static_type_import_shadows_same_package_sibling() {
    fixture()
        .file(
            "q/Host.java",
            "package q; public class Host { public static class X {} }",
        )
        .file("p/X.java", "package p; class X {}")
        .file(
            "p/Test.java",
            "package p; import static q.Host.X; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.Host.X")
        .expected_failure("static single-type imports are not resolved")
        .run();
}

#[test]
fn single_static_type_import_shadows_type_import_on_demand() {
    fixture()
        .file(
            "q/Host.java",
            "package q; public class Host { public static class X {} }",
        )
        .file("r/X.java", "package r; public class X {}")
        .file(
            "p/Test.java",
            "package p; import static q.Host.X; import r.*; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.Host.X")
        .expected_failure("static single-type imports are not resolved")
        .run();
}
