// Placing a file in the project, which decides what it may look at. Two things
// have to hold that no specification mentions, because both come from an editor
// session rather than from a compiler run: a file the project never mentions has
// to keep working, and the project may turn up after the files do.
//
// Which sources a unit reaches is decided in the `crates/engine/src/workspace.rs`
// and its cases live in the `workspace/tests/scopes.rs`. One of them comes back
// here, to say a unit edge reaches a user of Beans.

use beans_acceptance::fixture::fixture;

/// Every file below declares `package p`, so a package boundary cannot be what
/// stops a reference and the unit boundary is the only thing left. That is what
/// a main and a test source set look like.
#[test]
fn a_unit_sees_the_unit_it_depends_on() {
    fixture()
        .unit("a", &["a"], &[])
        .unit("b", &["b"], &["a"])
        .file("a/p/A.java", "package p; public class A {}")
        .file(
            "b/p/B.java",
            "package p; public class B { <cur:down>A field; }",
        )
        .analyze("b/p/B.java")
        .resolves_to("down", "p.A")
        .run();
}

/// The case every editor session hits: a file open next to a project, under no
/// declared tree. Nobody placed it, so nothing may be hidden from it. Placing it
/// nowhere and calling that an empty scope would leave it seeing nothing, and a
/// scratch file would go dark the moment a project loaded.
#[test]
fn a_file_no_unit_declares_still_sees_the_project() {
    fixture()
        .unit("a", &["a"], &[])
        .file("a/p/A.java", "package p; public class A {}")
        .file(
            "scratch/p/Scratch.java",
            "package p; public class Scratch { <cur:reach>A field; }",
        )
        .analyze("scratch/p/Scratch.java")
        .resolves_to("reach", "p.A")
        .run();
}

/// The same file, once a unit does declare it, is placed and the unit boundary
/// applies. Without this the test above would pass for a project that never
/// scopes anything.
#[test]
fn a_file_a_unit_declares_is_held_to_its_boundary() {
    fixture()
        .unit("a", &["a"], &[])
        .unit("scratch", &["scratch"], &[])
        .file("a/p/A.java", "package p; public class A {}")
        .file(
            "scratch/p/Scratch.java",
            "package p; public class Scratch { <cur:reach>A field; }",
        )
        .analyze("scratch/p/Scratch.java")
        .does_not_resolve("reach")
        .run();
}

/// An editor opens files first and reads the project afterwards. A scope is not
/// parse input, so the project arriving late has to re-place what we already
/// hold rather than wait for the files to be sent again.
#[test]
fn a_project_arriving_after_its_files_still_places_them() {
    fixture()
        .workspace_arrives_last()
        .unit("a", &["a"], &[])
        .unit("b", &["b"], &[])
        .file("a/p/A.java", "package p; public class A {}")
        .file(
            "b/p/B.java",
            "package p; public class B { <cur:across>A field; }",
        )
        .analyze("b/p/B.java")
        .does_not_resolve("across")
        .run();
}

/// The same shape without a project: nothing is placed, so the reference that
/// the unit boundary stopped above resolves here. This is what says the test
/// above is about the late arrival and not about `b` never seeing `a`.
#[test]
fn without_a_project_nothing_is_placed_at_all() {
    fixture()
        .file("a/p/A.java", "package p; public class A {}")
        .file(
            "b/p/B.java",
            "package p; public class B { <cur:across>A field; }",
        )
        .analyze("b/p/B.java")
        .resolves_to("across", "p.A")
        .run();
}
