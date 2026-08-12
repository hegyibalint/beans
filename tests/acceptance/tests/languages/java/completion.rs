//! What a user of Beans is offered at a caret. The rules live in
//! `lang-java`'s `completion` module; these establish that the answer reaches
//! the facade at all.

use beans_acceptance::fixture::fixture;

/// JLS §6.3: a declaration is in scope at a point if its scope includes that
/// point. Walking outwards from the caret is how that gets answered.
#[test]
fn a_caret_is_offered_the_types_in_scope_around_it() {
    fixture()
        .file(
            "com/example/Foo.java",
            "package com.example;
class Foo {
    class Inner {}
    void m() {
        <cur:body>
    }
}
class Sibling {}
",
        )
        .analyze("com/example/Foo.java")
        .completes_with("body", &["Inner", "Foo", "Sibling"])
        .run();
}

/// §6.5.6.2 makes a name after a `.` a member of the receiver, so the names in
/// scope are exactly the wrong answer there.
#[test]
fn a_caret_after_a_dot_is_offered_nothing_in_scope() {
    fixture()
        .file(
            "com/example/Foo.java",
            "package com.example;
class Foo {
    class Inner {}
    void m() {
        String s = \"\";
        s.<cur:member>
    }
}
",
        )
        .analyze("com/example/Foo.java")
        .does_not_complete("member", "Inner")
        .run();
}
