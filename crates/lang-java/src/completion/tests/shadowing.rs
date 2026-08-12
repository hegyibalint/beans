//! §6.4.1 across stages. `or_stage` stops at the first stage with a valid
//! candidate; completion cannot, so the same rule is applied per name instead.

use super::*;

/// A member type and a single-type import competing for one spelling. The
/// member type wins in `resolve_type_candidates`, and here it wins the row.
#[test]
fn a_member_type_wins_the_name_from_an_import() {
    let items = complete_at_cursor(
        "package p;
import java.util.List;
class Test {
    class List {}
    <cur> field;
}
",
    );

    assert_eq!(labels(&items).iter().filter(|l| **l == "List").count(), 1);
    assert_eq!(item(&items, "List").detail.as_deref(), Some("p.Test.List"));
}

/// An import against a sibling in the same package: §7.5.1 comes before §6.3.
#[test]
fn an_import_wins_the_name_from_the_same_package() {
    let items = Workspace::of(&[
        (
            "p/Test.java",
            "package p;
import q.Contested;
class Test {
    <cur> field;
}
",
        ),
        ("p/Contested.java", "package p;\nclass Contested {}\n"),
        ("q/Contested.java", "package q;\nclass Contested {}\n"),
    ])
    .complete();

    assert_eq!(
        labels(&items).iter().filter(|l| **l == "Contested").count(),
        1
    );
    assert_eq!(
        item(&items, "Contested").detail.as_deref(),
        Some("q.Contested")
    );
}

/// A type in this package against `java.lang`: §6.3 comes before §7.3, which is
/// why a project may declare its own `String` and mean it.
#[test]
fn the_current_package_wins_the_name_from_java_lang() {
    let items = Workspace::of(&[
        (
            "p/Test.java",
            "package p;
class Test {
    <cur> field;
}
",
        ),
        ("p/String.java", "package p;\npublic class String {}\n"),
    ])
    .compiled("jdk/java/lang/String.class", "java.lang.String")
    .complete();

    assert_eq!(labels(&items).iter().filter(|l| **l == "String").count(), 1);
    assert_eq!(item(&items, "String").detail.as_deref(), Some("p.String"));
}
