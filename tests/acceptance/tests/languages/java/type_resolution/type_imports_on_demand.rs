// JLS §§7.3, 7.5.2, and 6.4.1.

use beans_acceptance::fixture::fixture;

#[test]
fn package_on_demand_import_provides_top_level_type() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.*; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.X")
        .expected_failure("on-demand imports are not resolved")
        .run();
}

#[test]
fn type_on_demand_import_provides_member_type() {
    fixture()
        .file(
            "q/Outer.java",
            "package q; public class Outer { public class Inner {} }",
        )
        .file(
            "p/Test.java",
            "package p; import q.Outer.*; class Test { <cur:target>Inner f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.Outer.Inner")
        .expected_failure("on-demand imports are not resolved")
        .run();
}

#[test]
fn type_on_demand_import_provides_inherited_member_type() {
    fixture()
        .file(
            "q/Base.java",
            "package q; public class Base { public static class Inner {} }",
        )
        .file("q/Sub.java", "package q; public class Sub extends Base {}")
        .file(
            "p/Test.java",
            "package p; import q.Sub.*; class Test { <cur:target>Inner f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.Base.Inner")
        .expected_failure("on-demand imports are not resolved")
        .run();
}

#[test]
fn package_on_demand_import_excludes_inaccessible_type() {
    fixture()
        .file("q/X.java", "package q; class X {}")
        .file(
            "p/Test.java",
            "package p; import q.*; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .expect_at("target", "unresolvable-type")
        .expected_failure("on-demand imports are not resolved")
        .run();
}

#[test]
fn duplicate_package_on_demand_import_is_redundant() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.*; import q.*; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.X")
        .expected_failure("on-demand imports are not resolved")
        .run();
}

#[test]
fn on_demand_import_of_current_package_is_ignored() {
    fixture()
        .file(
            "p/Test.java",
            "package p; import p.*; class X {} class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.X")
        .run();
}

#[test]
fn explicit_on_demand_import_of_java_lang_is_ignored() {
    fixture()
        .file(
            "java/lang/String.java",
            "package java.lang; public class String {}",
        )
        .file(
            "p/Test.java",
            "package p; import java.lang.*; class Test { <cur:target>String f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "java.lang.String")
        .expected_failure("on-demand imports are not resolved")
        .run();
}
