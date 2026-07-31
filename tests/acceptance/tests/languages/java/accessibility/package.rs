// §6.6.1: "access is permitted only when the access occurs from within the
// package in which the class, interface, type parameter, or reference type is
// declared", which a declaration without a modifier gets by default.

use beans_acceptance::fixture::fixture;

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
