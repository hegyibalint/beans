//! Stage 1: the scope chain, which is the inverse of
//! `candidates_from_lexical_scopes`.

use super::*;

#[test]
fn offers_every_type_up_the_scope_chain() {
    let items = complete_at_cursor(
        "package p;
class Test {
    class Inner {}
    void m() {
        <cur>
    }
}
class Sibling {}
",
    );

    // §6.4.1's nearest-first, which is also the order resolution stops in.
    assert_eq!(labels(&items), ["Inner", "Test", "Sibling"]);
}

#[test]
fn the_prefix_filters_by_what_is_typed() {
    let items = complete_at_cursor(
        "package p;
class Test {
    class Alpha {}
    class Beta {}
    Al<cur> field;
}
",
    );

    assert_eq!(labels(&items), ["Alpha"]);
}

#[test]
fn the_replace_span_covers_the_typed_prefix() {
    let items = complete_at_cursor(
        "package p;
class Test {
    class Alpha {}
    Al<cur> field;
}
",
    );

    assert_eq!(item(&items, "Alpha").replace.len(), "Al".len());
}

#[test]
fn a_caret_that_has_typed_nothing_replaces_nothing() {
    let items = complete_at_cursor(
        "package p;
class Test {
    class Alpha {}
    void m() {
        <cur>
    }
}
",
    );

    assert!(item(&items, "Alpha").replace.is_empty());
}

#[test]
fn a_nearer_declaration_wins_a_name_outright() {
    let items = complete_at_cursor(
        "package p;
class Test {
    class Shadowed {}
    void m() {
        <cur>
    }
}
class Shadowed {}
",
    );

    assert_eq!(
        labels(&items).iter().filter(|l| **l == "Shadowed").count(),
        1
    );
    // Which of the two won, said in the only way the item can say it.
    assert_eq!(
        item(&items, "Shadowed").detail.as_deref(),
        Some("p.Test.Shadowed")
    );
}

#[test]
fn a_qualified_caret_offers_nothing() {
    let items = complete_at_cursor(
        "package p;
class Test {
    class Inner {}
    void m() {
        String s = \"\";
        s.<cur>
    }
}
",
    );

    assert_eq!(labels(&items), Vec::<&str>::new());
}

#[test]
fn a_member_type_carries_a_handle_and_a_local_class_does_not() {
    let items = complete_at_cursor(
        "package p;
class Test {
    class Member {}
    void m() {
        class Local {}
        <cur>
    }
}
",
    );

    assert!(item(&items, "Member").handle.is_some());
    assert!(item(&items, "Local").handle.is_none());
}

#[test]
fn only_types_are_offered_yet() {
    let items = complete_at_cursor(
        "package p;
class Test {
    int field;
    void method() {
        <cur>
    }
}
",
    );

    assert!(!labels(&items).contains(&"field"));
    assert!(!labels(&items).contains(&"method"));
}
