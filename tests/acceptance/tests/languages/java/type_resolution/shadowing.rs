// JLS §6.4.1.
//
// Precedence between the stages of `resolve_type_name` is decided in
// `resolution.rs`, so its cases live in `resolution/tests/staging.rs`. Each one
// there also removes the winner, which is what makes it about an order rather
// than about a single candidate existing. One case stays here, to show the order
// survives the trip out to a user.
//
// The rest of §6.4.1 sets a stage we have built against one we have not. A test
// whose loser does not exist passes without proving anything, which is worse
// than a pending one, so these stay claims until stages 4 and 5 land:
//
//  - a same package type shadows a type-import-on-demand
//  - a same package type shadows a static-import-on-demand
//  - a same package type shadows the implicit `java.lang` import
//  - a single-type import shadows a type-import-on-demand
//  - a single-type import shadows a static-import-on-demand
//  - a single static import shadows a same package sibling
//  - a single static import shadows a type-import-on-demand

use beans_acceptance::fixture::fixture;

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
