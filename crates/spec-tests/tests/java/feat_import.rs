use beans_spec_tests::fixture::fixture;

// Type import resolution is one observable feature assembled from rules across the JLS.
// Every expectation starts as an expected failure and is promoted independently.

// JLS §§6.3, 6.5.5.1, 8.2, and 9.2.
mod scope_of_declarations {
    use super::*;

    #[test]
    fn top_level_scope_is_not_order_dependent() {
        fixture()
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>Later f; } class Later {}",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "p.Later")
            .run();
    }

    #[test]
    fn same_package_top_level_type_is_in_scope() {
        fixture()
            .file("p/X.java", "package p; class X {}")
            .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
            .analyze("p/Test.java")
            .resolves_to("target", "p.X")
            .run();
    }

    #[test]
    fn member_type_scope_is_not_order_dependent() {
        fixture()
            .file(
                "p/Outer.java",
                "package p; class Outer { <cur:target>Inner f; class Inner {} }",
            )
            .analyze("p/Outer.java")
            .resolves_to("target", "p.Outer.Inner")
            .run();
    }

    #[test]
    fn enclosing_member_type_is_in_scope_in_a_nested_type() {
        fixture()
            .file(
                "p/Outer.java",
                "package p; class Outer { class X {} class Inner { <cur:target>X f; } }",
            )
            .analyze("p/Outer.java")
            .resolves_to("target", "p.Outer.X")
            .run();
    }

    #[test]
    fn superclass_member_type_is_inherited() {
        fixture()
            .file("p/Base.java", "package p; class Base { class X {} }")
            .file(
                "p/Sub.java",
                "package p; class Sub extends Base { <cur:target>X f; }",
            )
            .analyze("p/Sub.java")
            .resolves_to("target", "p.Base.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn superinterface_member_type_is_inherited() {
        fixture()
            .file(
                "p/Types.java",
                "package p; interface Types { interface X {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test implements Types { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "p.Types.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn class_type_parameter_is_in_scope_in_the_class_body() {
        fixture()
            .file(
                "p/Box.java",
                "package p; class Box<T> { <cur:target>T value; }",
            )
            .analyze("p/Box.java")
            .resolves_to_type_param("target", "T")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn method_type_parameter_is_in_scope_in_the_method_declaration() {
        fixture()
            .file(
                "p/Box.java",
                "package p; class Box { <T> <cur:target>T pick(T value) { return value; } }",
            )
            .analyze("p/Box.java")
            .resolves_to_type_param("target", "T")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn method_type_parameter_shadows_class_type_parameter() {
        fixture()
            .file(
                "p/Box.java",
                "package p; class Box<T> { <T> <cur:target>T pick(T value) { return value; } }",
            )
            .analyze("p/Box.java")
            .resolves_to_type_param("target", "T")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn local_type_is_in_scope_after_its_declaration() {
        fixture()
            .file(
                "p/Test.java",
                "package p; class Test { void m() { class Local {} <cur:target>Local value; } }",
            )
            .analyze("p/Test.java")
            .resolves_to_local_type("target", "Local")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn local_type_is_not_in_scope_before_its_declaration() {
        fixture()
            .file("p/Test.java", "package p; class Local {} class Test { void m() { <cur:target>Local before; class Local {} } }")
            .analyze("p/Test.java")
            .resolves_to("target", "p.Local")
            .run();
    }

    #[test]
    fn local_type_is_not_in_scope_after_its_block() {
        fixture()
            .file("p/Test.java", "package p; class Local {} class Test { void m() { { class Local {} } <cur:target>Local after; } }")
            .analyze("p/Test.java")
            .resolves_to("target", "p.Local")
            .run();
    }

    #[test]
    fn class_type_parameter_cannot_be_used_in_a_static_context() {
        fixture()
            .file(
                "p/Box.java",
                "package p; class Box<T> { static <cur:target>T value; }",
            )
            .analyze("p/Box.java")
            .expect_at("target", "illegal-type-parameter-use")
            .expected_failure("type parameter use checks are not implemented")
            .run();
    }

    #[test]
    fn class_type_parameter_cannot_cross_a_static_nested_type() {
        fixture()
            .file(
                "p/Box.java",
                "package p; class Box<T> { static class Nested { <cur:target>T value; } }",
            )
            .analyze("p/Box.java")
            .expect_at("target", "illegal-type-parameter-use")
            .expected_failure("type parameter use checks are not implemented")
            .run();
    }
}

// JLS §6.4.1.
mod shadowing {
    use super::*;

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
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn type_parameter_shadows_on_demand_import() {
        fixture()
            .file("q/X.java", "package q; public class X {}")
            .file(
                "p/Box.java",
                "package p; import q.*; class Box<X> { <cur:target>X value; }",
            )
            .analyze("p/Box.java")
            .resolves_to_type_param("target", "X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn local_type_shadows_member_type() {
        fixture()
            .file(
                "p/Test.java",
                "package p; class Test { class X {} void m() { class X {} <cur:target>X value; } }",
            )
            .analyze("p/Test.java")
            .resolves_to_local_type("target", "X")
            .expected_failure("resolution is not implemented")
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
            .expected_failure("resolution is not implemented")
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
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// JLS §6.5.5.1.
mod simple_type_names {
    use super::*;

    #[test]
    fn missing_simple_type_is_unresolvable() {
        fixture()
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>Missing f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn unimported_accessible_type_is_importable() {
        fixture()
            .file("q/X.java", "package q; public class X {}")
            .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
            .analyze("p/Test.java")
            .offers_imports("target", &["q.X"])
            .expected_failure("import suggestions are not implemented")
            .run();
    }

    #[test]
    fn all_accessible_auto_import_candidates_are_reported() {
        fixture()
            .file("q/X.java", "package q; public class X {}")
            .file("r/X.java", "package r; public class X {}")
            .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
            .analyze("p/Test.java")
            .offers_imports("target", &["q.X", "r.X"])
            .expected_failure("import suggestions are not implemented")
            .run();
    }

    #[test]
    fn distinct_type_imports_on_demand_are_ambiguous() {
        fixture()
            .file("q/X.java", "package q; public class X {}")
            .file("r/X.java", "package r; public class X {}")
            .file(
                "p/Test.java",
                "package p; import q.*; import r.*; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .ambiguous_between("target", &["q.X", "r.X"])
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn explicit_on_demand_import_can_collide_with_java_lang() {
        fixture()
            .file(
                "java/lang/String.java",
                "package java.lang; public class String {}",
            )
            .file("q/String.java", "package q; public class String {}")
            .file(
                "p/Test.java",
                "package p; import q.*; class Test { <cur:target>String f; }",
            )
            .analyze("p/Test.java")
            .ambiguous_between("target", &["java.lang.String", "q.String"])
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn duplicate_on_demand_paths_to_same_declaration_are_deduplicated() {
        fixture()
            .file("q/X.java", "package q; public class X {}")
            .file(
                "p/Test.java",
                "package p; import q.*; import q.*; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn type_and_static_on_demand_paths_to_same_type_are_deduplicated() {
        fixture()
            .file("q/Host.java", "package q; public class Host { public static class X {} }")
            .file("p/Test.java", "package p; import q.Host.*; import static q.Host.*; class Test { <cur:target>X f; }")
            .analyze("p/Test.java")
            .resolves_to("target", "q.Host.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn distinct_inherited_member_types_are_ambiguous() {
        fixture()
            .file("p/A.java", "package p; interface A { class X {} }")
            .file("p/B.java", "package p; interface B { class X {} }")
            .file(
                "p/Test.java",
                "package p; class Test implements A, B { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .ambiguous_between("target", &["p.A.X", "p.B.X"])
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn diamond_paths_to_same_member_type_are_deduplicated() {
        fixture()
            .file("p/Top.java", "package p; interface Top { class X {} }")
            .file("p/Left.java", "package p; interface Left extends Top {}")
            .file("p/Right.java", "package p; interface Right extends Top {}")
            .file(
                "p/Test.java",
                "package p; class Test implements Left, Right { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "p.Top.X")
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// JLS §§6.5.2, 6.5.4, and 6.5.5.2.
mod qualified_type_names {
    use super::*;

    #[test]
    fn fully_qualified_top_level_type_needs_no_import() {
        fixture()
            .file("q/X.java", "package q; public class X {}")
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn fully_qualified_member_type_resolves() {
        fixture()
            .file(
                "q/Outer.java",
                "package q; public class Outer { public class Inner {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Outer.Inner f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Outer.Inner")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn imported_outer_type_can_qualify_member_type() {
        fixture()
            .file(
                "q/Outer.java",
                "package q; public class Outer { public class Inner {} }",
            )
            .file(
                "p/Test.java",
                "package p; import q.Outer; class Test { <cur:target>Outer.Inner f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Outer.Inner")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn qualified_inherited_member_denotes_declaring_type_member() {
        fixture()
            .file(
                "q/Base.java",
                "package q; public class Base { public static class Inner {} }",
            )
            .file("q/Sub.java", "package q; public class Sub extends Base {}")
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Sub.Inner f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Base.Inner")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn type_parameter_can_qualify_a_bound_member_type() {
        fixture()
            .file(
                "q/Outer.java",
                "package q; public class Outer { public class Inner {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test<T extends q.Outer> { <cur:target>T.Inner f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Outer.Inner")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn missing_type_in_existing_package_is_unresolvable() {
        fixture()
            .file("q/Present.java", "package q; public class Present {}")
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Missing f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn missing_member_of_existing_type_is_unresolvable() {
        fixture()
            .file("q/Outer.java", "package q; public class Outer {}")
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Outer.Missing f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn inaccessible_qualified_top_level_type_is_rejected() {
        fixture()
            .file("q/Hidden.java", "package q; class Hidden {}")
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Hidden f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "inaccessible-type")
            .expected_failure("access checks are not implemented")
            .run();
    }

    #[test]
    fn inaccessible_qualified_member_type_is_rejected() {
        fixture()
            .file(
                "q/Outer.java",
                "package q; public class Outer { private class Hidden {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Outer.Hidden f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "inaccessible-type")
            .expected_failure("access checks are not implemented")
            .run();
    }

    #[test]
    fn in_scope_type_prefix_obscures_same_named_package() {
        fixture()
            .file("q/X.java", "package q; public class X {}")
            .file("p/q.java", "package p; class q {}")
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.X f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn source_member_name_maps_to_jvm_binary_identity() {
        fixture()
            .file(
                "q/Outer.java",
                "package q; public class Outer { public static class Inner {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Outer.Inner f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Outer.Inner")
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// JLS §§7.5.1 and 6.4.1.
mod single_type_imports {
    use super::*;

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
            .file("q/Host.java", "package q; public class Host { public static class X {} }")
            .file("p/Test.java", "package p; import q.Host.X; import static q.Host.X; class Test { <cur:target>X f; }")
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
}

// JLS §§7.3, 7.5.2, and 6.4.1.
mod type_imports_on_demand {
    use super::*;

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
            .expected_failure("resolution is not implemented")
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
            .expected_failure("resolution is not implemented")
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
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn package_on_demand_import_does_not_reach_subpackages() {
        fixture()
            .file("q/r/X.java", "package q.r; public class X {}")
            .file(
                "p/Test.java",
                "package p; import q.*; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .offers_imports("target", &["q.r.X"])
            .expected_failure("import suggestions are not implemented")
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
            .expected_failure("resolution is not implemented")
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
            .expected_failure("resolution is not implemented")
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
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// JLS §§7.5.3, 7.5.4, and 6.4.1.
mod static_type_imports {
    use super::*;

    #[test]
    fn single_static_import_provides_static_member_type() {
        fixture()
            .file(
                "q/Host.java",
                "package q; public class Host { public static class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; import static q.Host.X; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Host.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn static_on_demand_import_provides_static_member_type() {
        fixture()
            .file(
                "q/Host.java",
                "package q; public class Host { public static class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; import static q.Host.*; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Host.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn single_static_import_rejects_non_static_inner_type() {
        fixture()
            .file(
                "q/Host.java",
                "package q; public class Host { public class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; import static q.Host.X; class Test {}",
            )
            .analyze("p/Test.java")
            .expect("invalid-static-import")
            .expected_failure("import declaration checks are not implemented")
            .run();
    }

    #[test]
    fn static_on_demand_import_excludes_non_static_inner_type() {
        fixture()
            .file(
                "q/Host.java",
                "package q; public class Host { public class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; import static q.Host.*; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .offers_imports("target", &["q.Host.X"])
            .expected_failure("import suggestions are not implemented")
            .run();
    }

    #[test]
    fn static_import_can_reach_inherited_static_member_type() {
        fixture()
            .file(
                "q/Base.java",
                "package q; public class Base { public static class X {} }",
            )
            .file(
                "q/Host.java",
                "package q; public class Host extends Base {}",
            )
            .file(
                "p/Test.java",
                "package p; import static q.Host.X; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Base.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn single_static_import_of_missing_member_is_error() {
        fixture()
            .file("q/Host.java", "package q; public class Host {}")
            .file(
                "p/Test.java",
                "package p; import static q.Host.X; class Test {}",
            )
            .analyze("p/Test.java")
            .expect("unresolvable-import")
            .expected_failure("import declaration checks are not implemented")
            .run();
    }

    #[test]
    fn single_static_import_of_inaccessible_member_is_error() {
        fixture()
            .file(
                "q/Host.java",
                "package q; public class Host { private static class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; import static q.Host.X; class Test {}",
            )
            .analyze("p/Test.java")
            .expect("inaccessible-import")
            .expected_failure("import declaration checks are not implemented")
            .run();
    }

    #[test]
    fn one_single_static_import_may_expose_ambiguous_inherited_types() {
        fixture()
            .file("q/A.java", "package q; public interface A { class X {} }")
            .file("q/B.java", "package q; public interface B { class X {} }")
            .file(
                "q/Host.java",
                "package q; public class Host implements A, B {}",
            )
            .file(
                "p/Test.java",
                "package p; import static q.Host.X; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .ambiguous_between("target", &["q.A.X", "q.B.X"])
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn type_and_static_on_demand_imports_of_distinct_types_are_ambiguous() {
        fixture()
            .file("q/X.java", "package q; public class X {}")
            .file(
                "r/Host.java",
                "package r; public class Host { public static class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; import q.*; import static r.Host.*; class Test { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .ambiguous_between("target", &["q.X", "r.Host.X"])
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn duplicate_static_on_demand_import_is_redundant() {
        fixture()
            .file("q/Host.java", "package q; public class Host { public static class X {} }")
            .file("p/Test.java", "package p; import static q.Host.*; import static q.Host.*; class Test { <cur:target>X f; }")
            .analyze("p/Test.java")
            .resolves_to("target", "q.Host.X")
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// JLS §6.6.
mod accessibility {
    use super::*;

    #[test]
    fn package_access_top_level_type_is_accessible_in_same_package() {
        fixture()
            .file("p/X.java", "package p; class X {}")
            .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
            .analyze("p/Test.java")
            .resolves_to("target", "p.X")
            .run();
    }

    #[test]
    fn package_access_top_level_type_is_not_importable_from_other_package() {
        fixture()
            .file("q/X.java", "package q; class X {}")
            .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
            .analyze("p/Test.java")
            .expect_at("target", "unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn private_member_type_is_accessible_within_same_top_level_nest() {
        fixture()
            .file(
                "p/Outer.java",
                "package p; class Outer { private class X {} class Inner { <cur:target>X f; } }",
            )
            .analyze("p/Outer.java")
            .resolves_to("target", "p.Outer.X")
            .run();
    }

    #[test]
    fn private_member_type_is_inaccessible_outside_top_level_nest() {
        fixture()
            .file(
                "q/Outer.java",
                "package q; public class Outer { private class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Outer.X f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "inaccessible-type")
            .expected_failure("access checks are not implemented")
            .run();
    }

    #[test]
    fn protected_member_type_is_inherited_by_cross_package_subclass() {
        fixture()
            .file(
                "q/Base.java",
                "package q; public class Base { protected static class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test extends q.Base { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "q.Base.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn package_access_member_type_is_not_inherited_across_packages() {
        fixture()
            .file(
                "q/Base.java",
                "package q; public class Base { static class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test extends q.Base { <cur:target>X f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn accessible_member_of_inaccessible_enclosing_type_is_inaccessible() {
        fixture()
            .file(
                "q/Hidden.java",
                "package q; class Hidden { public static class X {} }",
            )
            .file(
                "p/Test.java",
                "package p; class Test { <cur:target>q.Hidden.X f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "inaccessible-type")
            .expected_failure("access checks are not implemented")
            .run();
    }
}

// JLS §§7.5.5 and 6.4.1.
mod module_imports {
    use super::*;

    #[test]
    fn module_import_provides_exported_public_type() {
        fixture()
            .file("lib/module-info.java", "module m.lib { exports api; }")
            .file("lib/api/X.java", "package api; public class X {}")
            .file("app/module-info.java", "module m.app { requires m.lib; }")
            .file(
                "app/p/Test.java",
                "package p; import module m.lib; class Test { <cur:target>X f; }",
            )
            .analyze("app/p/Test.java")
            .resolves_to("target", "api.X")
            .expected_failure("module imports and fixture module roots are not implemented")
            .run();
    }

    #[test]
    fn module_import_includes_transitively_read_exported_packages() {
        fixture()
            .file(
                "base/module-info.java",
                "module m.base { exports base.api; }",
            )
            .file(
                "base/base/api/X.java",
                "package base.api; public class X {}",
            )
            .file(
                "facade/module-info.java",
                "module m.facade { requires transitive m.base; }",
            )
            .file(
                "app/module-info.java",
                "module m.app { requires m.facade; }",
            )
            .file(
                "app/p/Test.java",
                "package p; import module m.facade; class Test { <cur:target>X f; }",
            )
            .analyze("app/p/Test.java")
            .resolves_to("target", "base.api.X")
            .expected_failure("module imports and fixture module roots are not implemented")
            .run();
    }

    #[test]
    fn one_module_import_can_introduce_ambiguous_simple_name() {
        fixture()
            .file(
                "lib/module-info.java",
                "module m.lib { exports a; exports b; }",
            )
            .file("lib/a/X.java", "package a; public class X {}")
            .file("lib/b/X.java", "package b; public class X {}")
            .file("app/module-info.java", "module m.app { requires m.lib; }")
            .file(
                "app/p/Test.java",
                "package p; import module m.lib; class Test { <cur:target>X f; }",
            )
            .analyze("app/p/Test.java")
            .ambiguous_between("target", &["a.X", "b.X"])
            .expected_failure("module imports and fixture module roots are not implemented")
            .run();
    }

    #[test]
    fn single_type_import_shadows_module_import() {
        fixture()
            .file("lib/module-info.java", "module m.lib { exports a; }")
            .file("lib/a/X.java", "package a; public class X {}")
            .file("app/module-info.java", "module m.app { requires m.lib; }")
            .file("app/q/X.java", "package q; public class X {}")
            .file(
                "app/p/Test.java",
                "package p; import module m.lib; import q.X; class Test { <cur:target>X f; }",
            )
            .analyze("app/p/Test.java")
            .resolves_to("target", "q.X")
            .run();
    }

    #[test]
    fn type_import_on_demand_shadows_module_import() {
        fixture()
            .file("lib/module-info.java", "module m.lib { exports a; }")
            .file("lib/a/X.java", "package a; public class X {}")
            .file("app/module-info.java", "module m.app { requires m.lib; }")
            .file("app/q/X.java", "package q; public class X {}")
            .file(
                "app/p/Test.java",
                "package p; import module m.lib; import q.*; class Test { <cur:target>X f; }",
            )
            .analyze("app/p/Test.java")
            .resolves_to("target", "q.X")
            .expected_failure("module imports and fixture module roots are not implemented")
            .run();
    }

    #[test]
    fn static_import_on_demand_shadows_module_import() {
        fixture()
            .file("lib/module-info.java", "module m.lib { exports a; }")
            .file("lib/a/X.java", "package a; public class X {}")
            .file("app/module-info.java", "module m.app { requires m.lib; }")
            .file("app/q/Host.java", "package q; public class Host { public static class X {} }")
            .file("app/p/Test.java", "package p; import module m.lib; import static q.Host.*; class Test { <cur:target>X f; }")
            .analyze("app/p/Test.java")
            .resolves_to("target", "q.Host.X")
            .expected_failure("module imports and fixture module roots are not implemented")
            .run();
    }

    #[test]
    fn current_package_type_shadows_module_import() {
        fixture()
            .file("lib/module-info.java", "module m.lib { exports a; }")
            .file("lib/a/X.java", "package a; public class X {}")
            .file("app/module-info.java", "module m.app { requires m.lib; }")
            .file("app/p/X.java", "package p; class X {}")
            .file(
                "app/p/Test.java",
                "package p; import module m.lib; class Test { <cur:target>X f; }",
            )
            .analyze("app/p/Test.java")
            .resolves_to("target", "p.X")
            .run();
    }

    #[test]
    fn implicit_java_lang_import_shadows_module_import() {
        fixture()
            .file(
                "java/java/lang/String.java",
                "package java.lang; public class String {}",
            )
            .file("lib/module-info.java", "module m.lib { exports a; }")
            .file("lib/a/String.java", "package a; public class String {}")
            .file("app/module-info.java", "module m.app { requires m.lib; }")
            .file(
                "app/p/Test.java",
                "package p; import module m.lib; class Test { <cur:target>String f; }",
            )
            .analyze("app/p/Test.java")
            .resolves_to("target", "java.lang.String")
            .expected_failure("module imports and fixture module roots are not implemented")
            .run();
    }

    #[test]
    fn importing_unread_module_is_error() {
        fixture()
            .file("lib/module-info.java", "module m.lib { exports api; }")
            .file("lib/api/X.java", "package api; public class X {}")
            .file("app/module-info.java", "module m.app {}")
            .file(
                "app/p/Test.java",
                "package p; import module m.lib; class Test {}",
            )
            .analyze("app/p/Test.java")
            .expect("unread-module-import")
            .expected_failure("module imports and fixture module roots are not implemented")
            .run();
    }
}
