use beans_acceptance::fixture::fixture;

// Type import resolution is one observable feature assembled from rules across the JLS.
//
// A pending expectation carries the reason it is pending, and that reason is part
// of the assertion; when it stops being true the marker is wrong, even while the
// test still passes. Where a whole rule is out of reach we keep the citation and
// the claims we would make, rather than tests that all fail for one reason.

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
            .expected_failure("inherited member types are not resolved")
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
            .expected_failure("inherited member types are not resolved")
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
}

// JLS §6.5.5.1.
//
// Every case here needs on-demand imports and the ambiguity rules that come with
// them, and none of that resolves yet; written out, they would be nine tests
// failing for one reason. The claims to make once it lands:
//
//  - a missing simple type is unresolvable
//  - two on-demand imports offering one simple name are ambiguous
//  - an explicit on-demand import can collide with `java.lang`
//  - two on-demand paths to one declaration are deduplicated
//  - a type import and a static import reaching one type are deduplicated
//  - two inherited member types sharing a name are ambiguous
//  - diamond paths to one member type are deduplicated
//
// Two more belong to code actions rather than to resolution, i.e. the tier that
// is allowed to see past the scope: an unimported accessible type is importable,
// and every accessible candidate is offered, not just the first.

// JLS §§6.5.2, 6.5.4, and 6.5.5.2.
//
// A qualified type reference is not resolved at all, so there is nothing here to
// be pending about yet; all eleven cases would fail for that one reason. The
// claims to make once it lands:
//
//  - a fully qualified top level type needs no import
//  - a fully qualified member type resolves
//  - an imported outer type can qualify a member type
//  - a qualified inherited member denotes the member of its declaring type
//  - a type parameter can qualify a member type of its bound
//  - a missing type in an existing package is unresolvable
//  - a missing member of an existing type is unresolvable
//  - an inaccessible qualified type is rejected, top level and member alike
//  - an in-scope type prefix obscures a same-named package
//  - a source member name maps to its JVM binary identity

// Where a dotted name stops being a package and starts being a type.
// JLS §§6.5.4.1 and 6.5.4.2 walk it left to right and switch the first time the
// package to the left declares a type of that name; §7.1 forbids the two from
// colliding. A single-type import is the only surface where that walk is
// observable today, because a qualified type reference is not resolved yet.
mod package_type_boundary {
    use super::*;

    #[test]
    fn a_package_prefix_walks_through_to_the_type() {
        fixture()
            .file(
                "java/util/Date.java",
                "package java.util; public class Date {}",
            )
            .file(
                "p/Test.java",
                "package p; import java.util.Date; class Test { <cur:target>Date f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("target", "java.util.Date")
            .run();
    }

    #[test]
    fn nesting_below_the_boundary_is_walked_to_the_end() {
        fixture()
            .file(
                "p/A.java",
                "package p; public class A { public static class B { public static class C {} } }",
            )
            .file(
                "q/Test.java",
                "package q; import p.A.B.C; class Test { <cur:target>C f; }",
            )
            .analyze("q/Test.java")
            .resolves_to("target", "p.A.B.C")
            .run();
    }

    #[test]
    fn a_type_prefix_leaves_no_way_back_to_a_package() {
        fixture()
            .file("p/Outer.java", "package p; public class Outer {}")
            .file(
                "p/Outer/Inner.java",
                "package p.Outer; public class Inner {}",
            )
            .file(
                "q/Test.java",
                "package q; import p.Outer.Inner; class Test { <cur:target>Inner f; }",
            )
            .analyze("q/Test.java")
            .expect_at("target", "unresolvable-type")
            .expected_failure("unresolvable type references produce no diagnostic yet")
            .run();
    }

    #[test]
    fn a_prefix_that_names_nothing_is_unresolvable() {
        fixture()
            .file(
                "p/Test.java",
                "package p; import com.example.Missing; class Test { <cur:target>Missing f; }",
            )
            .analyze("p/Test.java")
            .expect_at("target", "unresolvable-type")
            .expected_failure("unresolvable type references produce no diagnostic yet")
            .run();
    }

    #[test]
    fn the_unnamed_package_contributes_no_prefix() {
        fixture()
            .file(
                "Outer.java",
                "public class Outer { public static class Inner {} }",
            )
            .file("Test.java", "class Test { <cur:target>Outer.Inner f; }")
            .analyze("Test.java")
            .resolves_to("target", "Outer.Inner")
            .expected_failure("qualified type references are not resolved yet")
            .run();
    }

    // Both declare `p.B`, so the label cannot tell them apart even though the
    // targets are distinct. Twice the same spelling is the honest expectation.
    #[test]
    fn one_name_declared_in_two_trees_is_ambiguous() {
        fixture()
            .file("app/p/B.java", "package p; public class B {}")
            .file("lib/p/B.java", "package p; public class B {}")
            .file(
                "q/Test.java",
                "package q; import p.B; class Test { <cur:target>B f; }",
            )
            .analyze("q/Test.java")
            .ambiguous_between("target", &["p.B", "p.B"])
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
}

// JLS §§7.5.3, 7.5.4, and 6.4.1.
//
// Static imports are not resolved, neither single nor on-demand, so all ten cases
// would fail for that one reason. The claims to make once they land:
//
//  - a single static import provides a static member type
//  - a static on-demand import provides a static member type
//  - a single static import rejects a non-static inner type
//  - a static on-demand import excludes a non-static inner type
//  - a static import can reach an inherited static member type
//  - importing a missing member is an error
//  - importing an inaccessible member is an error
//  - one single static import may expose ambiguous inherited types
//  - a type import and a static on-demand import of distinct types are ambiguous
//  - a duplicate static on-demand import is redundant

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
            .expected_failure("type accessibility is not checked when resolving a simple name")
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
            .expected_failure("qualified type references are not resolved yet")
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
            .expected_failure("inherited member types are not resolved")
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
            .expected_failure("inherited member types are not resolved")
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
            .expected_failure("qualified type references are not resolved yet")
            .run();
    }
}

// JLS §§7.5.5 and 6.4.1.
mod module_imports {
    use super::*;

    // Everything that needs a module to actually exist is out of reach: the
    // fixture has no module roots, and module imports are not resolved. What is
    // left below are the two cases where a module import loses to something
    // else, which hold today because the module import contributes nothing.
    //
    // The claims to make once module roots and §7.5.5 land:
    //
    //  - a module import provides an exported public type
    //  - it includes the packages exported by transitively read modules
    //  - one module import can introduce an ambiguous simple name
    //  - a type import on demand shadows a module import
    //  - a static import on demand shadows a module import
    //  - the implicit `java.lang` import shadows a module import
    //  - importing a module we do not read is an error

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
}
