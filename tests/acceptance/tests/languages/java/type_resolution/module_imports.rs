// JLS §§7.5.5 and 6.4.1.

use beans_acceptance::fixture::fixture;

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
