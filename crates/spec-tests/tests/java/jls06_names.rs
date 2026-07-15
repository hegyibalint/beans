use beans_spec_tests::fixture::fixture;

// §6.3 — Scope of a Declaration. Member types reach subclasses through
// inheritance (§8.2), so resolution cannot finish without the hierarchy.
mod jls_6_3_scope_of_declaration {
    use super::*;

    #[test]
    fn inherited_member_type_is_in_scope_in_the_subclass() {
        fixture()
            .file("p/A.java", "package p;\nclass A { class X {} }")
            .file("p/B.java", "package p;\nclass B extends A { <cur:x>X f; }")
            .analyze("p/B.java")
            .resolves_to("x", "p.A.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn interface_member_type_is_inherited_by_implementers() {
        fixture()
            .file("p/M.java", "package p;\ninterface M { interface E {} }")
            .file("p/C.java", "package p;\nclass C implements M { <cur:e>E f; }")
            .analyze("p/C.java")
            .resolves_to("e", "p.M.E")
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// §6.4.1 — Shadowing, sentence by sentence.
mod jls_6_4_1_shadowing {
    use super::*;

    // "A declaration d of a type named n shadows the declarations of any
    // other types named n that are in scope at the point where d occurs."
    #[test]
    fn member_type_shadows_a_same_file_top_level_type() {
        fixture()
            .file(
                "p/T.java",
                r#"
package p;

class X {}

class Outer {
    class X {}
    <cur:x>X f;
}
"#,
            )
            .analyze("p/T.java")
            .resolves_to("x", "p.Outer.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // Example 6.4.1-2: the file-local Vector wins over java.util.Vector.
    #[test]
    fn file_local_type_shadows_an_on_demand_import() {
        fixture()
            .file("q/Vector.java", "package q;\npublic class Vector {}")
            .file(
                "p/Test.java",
                r#"
package p;

import q.*;

class Vector {}

class Test {
    <cur:v>Vector v;
}
"#,
            )
            .analyze("p/Test.java")
            .resolves_to("v", "p.Vector")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // "A single-type-import declaration d ... shadows ... any top level
    // type named n declared in another compilation unit of p."
    #[test]
    fn single_type_import_shadows_a_same_package_sibling() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file("p/X.java", "package p;\nclass X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.X;\n\nclass Test { <cur:x>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("x", "q.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // "... any type named n imported by a type-import-on-demand
    // declaration in c."
    #[test]
    fn single_type_import_shadows_an_on_demand_import() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file("r/X.java", "package r;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.X;\nimport r.*;\n\nclass Test { <cur:x>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("x", "q.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // §7.5.2: an on-demand import "might be shadowed by ... a class or
    // interface ... declared in the package to which the compilation
    // unit belongs".
    #[test]
    fn same_package_sibling_shadows_an_on_demand_import() {
        fixture()
            .file("p/X.java", "package p;\nclass X {}")
            .file("q/X.java", "package q;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.*;\n\nclass Test { <cur:x>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("x", "p.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // java.lang is only an implicit on-demand import (§7.3), so a
    // same-package type shadows it like any other.
    #[test]
    fn same_package_sibling_shadows_implicit_java_lang() {
        fixture()
            .file("java/lang/String.java", "package java.lang;\npublic class String {}")
            .file("p/String.java", "package p;\nclass String {}")
            .file("p/Test.java", "package p;\nclass Test { <cur:s>String s; }")
            .analyze("p/Test.java")
            .resolves_to("s", "p.String")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn type_parameter_shadows_an_on_demand_import() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file(
                "p/Box.java",
                "package p;\n\nimport q.*;\n\nclass Box<X> { <cur:x>X val; }",
            )
            .analyze("p/Box.java")
            .resolves_to_type_param("x", "X")
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// §6.5.5.1 — "the identifier must occur in the scope of exactly one
// declaration of a class, interface, or type parameter with this name,
// or a compile-time error occurs."
mod jls_6_5_5_1_simple_type_names {
    use super::*;

    // Zero declarations in scope.
    #[test]
    fn type_name_without_declaration_is_unresolvable() {
        fixture()
            .file(
                "p/Foo.java",
                "package p;\nclass Foo { <cur:missing>Missing field; }",
            )
            .analyze("p/Foo.java")
            .expect("unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // Existing elsewhere is not in scope: spec-wise the same error as
    // not existing at all. "Importable" is a Beans refinement on top.
    #[test]
    fn a_type_declared_in_an_unimported_package_is_not_in_scope() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file("p/Test.java", "package p;\nclass Test { <cur:x>X f; }")
            .analyze("p/Test.java")
            .expect("unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // More than one declaration in scope: nothing in §6.4.1 lets one
    // on-demand import shadow another.
    #[test]
    fn colliding_on_demand_imports_make_a_use_ambiguous() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file("r/X.java", "package r;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.*;\nimport r.*;\n\nclass Test { <cur:x>X f; }",
            )
            .analyze("p/Test.java")
            .expect("ambiguous-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // A type name may denote "a class, interface, or type parameter" —
    // type parameters are not classes and have no Fqn.
    #[test]
    fn a_type_parameter_is_a_valid_meaning_for_a_type_name() {
        fixture()
            .file("p/Box.java", "package p;\nclass Box<T> { <cur:t>T val; }")
            .analyze("p/Box.java")
            .resolves_to_type_param("t", "T")
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// §6.5.5.2 — Qualified Type Names.
mod jls_6_5_5_2_qualified_type_names {
    use super::*;

    #[test]
    fn a_fully_qualified_name_resolves_without_an_import() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file("p/Test.java", "package p;\nclass Test { <cur:x>q.X f; }")
            .analyze("p/Test.java")
            .resolves_to("x", "q.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    #[test]
    fn a_qualified_member_type_resolves() {
        fixture()
            .file(
                "q/Outer.java",
                "package q;\npublic class Outer { public class Inner {} }",
            )
            .file(
                "p/Test.java",
                "package p;\nclass Test { <cur:i>q.Outer.Inner f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("i", "q.Outer.Inner")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // "If Id does not name a member class or interface within Q ...
    // a compile-time error occurs."
    #[test]
    fn a_qualified_name_without_such_member_is_an_error() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\nclass Test { <cur:m>q.Missing f; }",
            )
            .analyze("p/Test.java")
            .expect("unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }
}
