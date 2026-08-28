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

/// §10.2 composes one type from the brackets on the type and the brackets on
/// the declarator, and the row shows what was written. All three of these read
/// as their element type before arrays reached the model.
#[test]
fn an_array_is_offered_with_its_brackets() {
    let items = complete_at_cursor(
        "package p;
class Test {
    String[] field;
    void m(int grid[][]) {
        String[][] local = null;
        <cur>
    }
}
",
    );

    assert_eq!(item(&items, "field").detail.as_deref(), Some("String[]"));
    assert_eq!(item(&items, "grid").detail.as_deref(), Some("int[][]"));
    assert_eq!(item(&items, "local").detail.as_deref(), Some("String[][]"));
}

/// §8.4.1 makes a variable arity parameter's declared type an array type, and
/// §10.2 says why: "the ellipsis of a variable arity parameter is treated as a
/// bracket pair". The grammar gives that node no fields at all, so this
/// parameter used to reach the model with no name, and a caret in the body was
/// offered nothing for it.
#[test]
fn a_variable_arity_parameter_is_offered_as_an_array() {
    let items = complete_at_cursor(
        "package p;
class Test {
    void m(String... args) {
        <cur>
    }
}
",
    );

    assert!(labels(&items).contains(&"args"));
    assert_eq!(item(&items, "args").detail.as_deref(), Some("String[]"));
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
