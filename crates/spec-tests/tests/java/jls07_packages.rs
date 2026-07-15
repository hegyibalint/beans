use beans_spec_tests::fixture::fixture;

// §7.3 — Compilation Units: every compilation unit implicitly contains
// `import java.lang.*;`.
mod jls_7_3_compilation_units {
    use super::*;

    #[test]
    fn java_lang_is_implicitly_imported() {
        fixture()
            .file("java/lang/String.java", "package java.lang;\npublic class String {}")
            .file("p/Test.java", "package p;\nclass Test { <cur:s>String s; }")
            .analyze("p/Test.java")
            .resolves_to("s", "java.lang.String")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // The implicit import is an ordinary on-demand import; a user's
    // on-demand import supplying the same name collides with it
    // (§6.4.1 has no shadow rule between on-demand imports).
    #[test]
    fn implicit_java_lang_collides_with_a_users_on_demand_import() {
        fixture()
            .file("java/lang/String.java", "package java.lang;\npublic class String {}")
            .file("q/String.java", "package q;\npublic class String {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.*;\n\nclass Test { <cur:s>String s; }",
            )
            .analyze("p/Test.java")
            .expect("ambiguous-type")
            .expected_failure("resolution is not implemented")
            .run();
    }
}

// §7.5.1 — Single-Type-Import Declarations.
mod jls_7_5_1_single_type_imports {
    use super::*;

    // "If two single-type-import declarations ... attempt to import
    // classes or interfaces with the same simple name, then a
    // compile-time error occurs, unless the two ... are the same."
    #[test]
    fn colliding_single_type_imports_are_an_error() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file("r/X.java", "package r;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.X;\nimport r.X;\n\nclass Test {}",
            )
            .analyze("p/Test.java")
            .expect("import-collision")
            .expected_failure("import declaration checks are not implemented")
            .run();
    }

    // "... in which case the duplicate declaration is ignored."
    #[test]
    fn duplicate_imports_of_the_same_type_are_ignored() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.X;\nimport q.X;\n\nclass Test { <cur:x>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("x", "q.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // "If a single-type-import declaration imports a class or interface
    // whose simple name is x, and the compilation unit also declares a
    // top level class or interface whose simple name is x, a
    // compile-time error occurs."
    #[test]
    fn an_import_colliding_with_a_file_local_type_is_an_error() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.X;\n\nclass X {}",
            )
            .analyze("p/Test.java")
            .expect("import-collision")
            .expected_failure("import declaration checks are not implemented")
            .run();
    }

    // "If the class or interface imported ... is declared as a top level
    // class or interface in the compilation unit that contains the
    // import declaration, then the import declaration is ignored."
    #[test]
    fn importing_the_files_own_type_is_ignored() {
        fixture()
            .file(
                "p/X.java",
                "package p;\n\nimport p.X;\n\nclass X { <cur:x>X self; }",
            )
            .analyze("p/X.java")
            .resolves_to("x", "p.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // "The TypeName must be the canonical name of a class or interface"
    // and the named type must be accessible.
    #[test]
    fn an_import_naming_a_missing_type_is_an_error() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.Missing;\n\nclass Test {}",
            )
            .analyze("p/Test.java")
            .expect("unresolvable-import")
            .expected_failure("import declaration checks are not implemented")
            .run();
    }

    // Example 7.5.1-3: an import cannot name a package.
    #[test]
    fn an_import_naming_a_package_is_an_error() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file("p/Test.java", "package p;\n\nimport q;\n\nclass Test {}")
            .analyze("p/Test.java")
            .expect("unresolvable-import")
            .expected_failure("import declaration checks are not implemented")
            .run();
    }
}

// §7.5.2 — Type-Import-on-Demand Declarations.
mod jls_7_5_2_on_demand_imports {
    use super::*;

    #[test]
    fn an_on_demand_import_provides_the_packages_types() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.*;\n\nclass Test { <cur:x>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("x", "q.X")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // The PackageOrTypeName may denote a class: its member types are
    // imported.
    #[test]
    fn an_on_demand_import_of_a_class_provides_its_member_types() {
        fixture()
            .file(
                "q/Outer.java",
                "package q;\npublic class Outer { public static class Inner {} }",
            )
            .file(
                "p/Test.java",
                "package p;\n\nimport q.Outer.*;\n\nclass Test { <cur:i>Inner f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("i", "q.Outer.Inner")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // Packages are not hierarchical for imports: q.* does not reach q.r.
    #[test]
    fn on_demand_imports_do_not_reach_subpackages() {
        fixture()
            .file("q/r/X.java", "package q.r;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.*;\n\nclass Test { <cur:x>X f; }",
            )
            .analyze("p/Test.java")
            .expect("unresolvable-type")
            .expected_failure("resolution is not implemented")
            .run();
    }

    // "Two or more type-import-on-demand declarations ... may name the
    // same package ... All but one ... are considered redundant."
    #[test]
    fn redundant_on_demand_imports_are_legal() {
        fixture()
            .file("q/X.java", "package q;\npublic class X {}")
            .file(
                "p/Test.java",
                "package p;\n\nimport q.*;\nimport q.*;\n\nclass Test { <cur:x>X f; }",
            )
            .analyze("p/Test.java")
            .resolves_to("x", "q.X")
            .expected_failure("resolution is not implemented")
            .run();
    }
}
