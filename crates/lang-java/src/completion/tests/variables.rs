//! §6.5.6's namespace: locals, parameters and fields, up the scope chain.

use super::*;

#[test]
fn offers_locals_parameters_and_fields() {
    let items = complete_at_cursor(
        "package p;
class Test {
    int field;
    void m(String parameter) {
        int local = 1;
        <cur>
    }
}
",
    );

    let labels = labels(&items);
    assert!(labels.contains(&"local"));
    assert!(labels.contains(&"parameter"));
    assert!(labels.contains(&"field"));
}

/// §6.3: "the scope of a local variable declaration in a block is the rest of
/// the block", so a caret above the declarator does not see it.
#[test]
fn a_local_is_not_offered_above_its_own_declarator() {
    let items = complete_at_cursor(
        "package p;
class Test {
    void m() {
        <cur>
        int later = 1;
    }
}
",
    );

    assert!(!labels(&items).contains(&"later"));
}

/// §6.4.1: a local shadows a field of the same name, and the row has to be the
/// one the user will actually reach.
#[test]
fn a_local_takes_the_name_from_a_field() {
    let items = complete_at_cursor(
        "package p;
class Test {
    String shadowed;
    void m() {
        int shadowed = 1;
        <cur>
    }
}
",
    );

    assert_eq!(
        labels(&items).iter().filter(|l| **l == "shadowed").count(),
        1
    );
    assert_eq!(item(&items, "shadowed").detail.as_deref(), Some("int"));
}

/// The type as written, which is what an editor shows and what the arena
/// already holds. Resolving it would cost a lake lookup per row.
#[test]
fn a_variable_is_detailed_with_its_written_type() {
    let items = complete_at_cursor(
        "package p;
class Test {
    void m(java.util.List given) {
        <cur>
    }
}
",
    );

    assert_eq!(
        item(&items, "given").detail.as_deref(),
        Some("java.util.List")
    );
}

#[test]
fn a_variable_carries_no_handle() {
    let items = complete_at_cursor(
        "package p;
class Test {
    void m() {
        int local = 1;
        <cur>
    }
}
",
    );

    assert!(item(&items, "local").handle.is_none());
}
