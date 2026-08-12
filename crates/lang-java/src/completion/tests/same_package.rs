//! Stage 3, §6.3: the top-level types of this compilation unit's package are in
//! scope without an import.

use super::*;

#[test]
fn a_sibling_top_level_type_is_offered() {
    let items = Workspace::of(&[
        (
            "p/Test.java",
            "package p;
class Test {
    <cur> field;
}
",
        ),
        ("p/Sibling.java", "package p;\nclass Sibling {}\n"),
    ])
    .complete();

    assert!(labels(&items).contains(&"Sibling"));
    assert_eq!(item(&items, "Sibling").detail.as_deref(), Some("p.Sibling"));
}

#[test]
fn another_packages_type_is_not_offered() {
    let items = Workspace::of(&[
        (
            "p/Test.java",
            "package p;
class Test {
    <cur> field;
}
",
        ),
        ("q/Elsewhere.java", "package q;\nclass Elsewhere {}\n"),
    ])
    .complete();

    assert!(!labels(&items).contains(&"Elsewhere"));
}

/// §6.5.5.1 puts a member type in scope by its simple name inside its enclosing
/// type or through an import, never by sharing a package. The lake cannot make
/// the distinction — a binary name carries the package and nothing else — so
/// this stage drops what §13.1 spelled with a `$`.
#[test]
fn a_nested_type_of_a_sibling_is_not_offered_by_its_simple_name() {
    let items = Workspace::of(&[
        (
            "p/Test.java",
            "package p;
class Test {
    <cur> field;
}
",
        ),
        ("p/Sibling.java", "package p;\nclass Sibling {}\n"),
    ])
    .compiled("out/p/Sibling$Buried.class", "p.Sibling$Buried")
    .complete();

    assert!(labels(&items).contains(&"Sibling"));
    assert!(!labels(&items).contains(&"Buried"));
}
