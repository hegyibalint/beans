//! Who gets a handle, and whether it finds its way back.
//!
//! `agreement.rs` checks the half of an item a user reads. This checks the half
//! only we read: the note completion writes to itself so that
//! `completionItem/resolve` can find the same declaration again later.

use super::*;

/// The rule, stated once: `None` means this thing's identity is file-local.
#[test]
fn only_what_another_file_could_name_carries_one() {
    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    class Member {}
    int field;
    void m(String parameter) {
        class Local {}
        int local = 1;
        <cur>
    }
}
",
    )])
    .complete();

    for (label, expected) in [
        ("Member", true), // a member type: p.Test.Member names it from anywhere
        ("Test", true),   // a top-level type
        ("Local", false), // §13.1 gives it compiler-chosen digits
        ("local", false), // §6.3 confines it to one block
        ("parameter", false),
        ("field", false), // has an identity, but naming it needs the enclosing type
        ("m", false),     // and a descriptor, for overloads
    ] {
        assert_eq!(
            item(&items, label).handle.is_some(),
            expected,
            "handle on {label:?}"
        );
    }
}

/// A handle is only useful if it comes back to what it was minted from.
#[test]
fn a_handle_names_the_declaration_it_was_minted_from() {
    let workspace = Workspace::of(&[
        (
            "p/Test.java",
            "package p;
class Test {
    <cur> field;
}
",
        ),
        ("p/Sibling.java", "package p;\nclass Sibling {}\n"),
    ]);
    let items = workspace.complete();

    let handle = item(&items, "Sibling")
        .handle
        .as_ref()
        .expect("a sibling in this package is nameable from elsewhere");

    // The source routes it back to the vertical that minted it, and the payload
    // is what that vertical needs to find the declaration again.
    assert_eq!(
        handle.source,
        jvm::model::Source::SourceFile {
            path: std::path::PathBuf::from("p/Sibling.java")
        }
    );
    assert_eq!(handle.payload, "p.Sibling");
    assert_eq!(handle.revision, workspace.revision);
}

/// A compiled type has no model behind it, so the payload is the binary name
/// the lake holds rather than a label walked out of a parse.
#[test]
fn a_compiled_type_is_named_by_its_binary_name() {
    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    <cur> field;
}
",
    )])
    .compiled("jdk/java/lang/String.class", "java.lang.String")
    .complete();

    let handle = item(&items, "String").handle.as_ref().expect("a handle");
    assert_eq!(handle.payload, "java.lang.String");
}
