use beans_acceptance::fixture::fixture;

// JLS 26 §6.6.1 decides whether a name a file resolved is a name it may use.
// The two are separate questions here: resolution stays permissive so that
// navigation still reaches the declaration, and the diagnostic is what says it
// may not be touched.

// §6.6.1: private access is permitted when it "occurs from within the body of
// the top level class or interface that encloses the declaration". The unit is
// the outermost body (§7.6), which is wider than the declaring class and
// narrower than the file.
mod private_access {
    use super::*;

    #[test]
    fn a_sibling_top_level_class_cannot_reach_a_private_field() {
        fixture()
            .file(
                "p/Vault.java",
                "package p;\
                 class Vault { private int secret; }\
                 class Thief { int steal(Vault v) { return v.<cur:theft>secret; } }",
            )
            .analyze("p/Vault.java")
            .expect_at("theft", "inaccessible-member")
            .run();
    }

    // The same theft, one file further away. Nothing about the rule changes,
    // so nothing about the answer may change either.
    #[test]
    fn another_file_cannot_reach_a_private_field() {
        fixture()
            .file(
                "p/Vault.java",
                "package p; class Vault { private int secret; }",
            )
            .file(
                "p/Thief.java",
                "package p; class Thief { int steal(Vault v) { return v.<cur:theft>secret; } }",
            )
            .analyze("p/Thief.java")
            .expect_at("theft", "inaccessible-member")
            .run();
    }

    // The other side of the same rule: a nested class is inside the enclosing
    // top level body, so it reaches the private field and must stay quiet.
    #[test]
    fn a_nested_class_reaches_a_private_field_of_its_enclosing_type() {
        fixture()
            .file(
                "p/Vault.java",
                "package p;\
                 class Vault {\
                     private int secret;\
                     static class Inner { int peek(Vault v) { return v.<cur:legal>secret; } }\
                 }",
            )
            .analyze("p/Vault.java")
            .expect_no("inaccessible-member")
            .run();
    }
}

// §6.6.1: "access is permitted only when the access occurs from within the
// package in which the class, interface, type parameter, or reference type is
// declared", which a declaration without a modifier gets by default.
mod package_access {
    use super::*;

    #[test]
    fn another_package_cannot_reach_a_package_private_field() {
        fixture()
            .file(
                "a/Vault.java",
                "package a; public class Vault { int secret; }",
            )
            // Imported rather than written `a.Vault`, because a qualified type
            // reference does not resolve yet and the receiver has to.
            .file(
                "b/Thief.java",
                "package b; import a.Vault;\
                 class Thief { int steal(Vault v) { return v.<cur:theft>secret; } }",
            )
            .analyze("b/Thief.java")
            .expect_at("theft", "inaccessible-member")
            .run();
    }

    #[test]
    fn the_declaring_package_reaches_a_package_private_field() {
        fixture()
            .file(
                "p/Vault.java",
                "package p; public class Vault { int secret; }",
            )
            .file(
                "p/Neighbour.java",
                "package p; class Neighbour { int read(Vault v) { return v.<cur:legal>secret; } }",
            )
            .analyze("p/Neighbour.java")
            .expect_no("inaccessible-member")
            .run();
    }
}
