//! §6.5.7's namespace: what an unqualified call can reach.

use super::*;

#[test]
fn offers_the_methods_of_the_enclosing_type() {
    let items = complete_at_cursor(
        "package p;
class Test {
    void sibling() {}
    void m() {
        <cur>
    }
}
",
    );

    let labels = labels(&items);
    assert!(labels.contains(&"sibling"));
    assert!(labels.contains(&"m"));
}

/// §15.12.2 picks an overload from the types of the arguments, and a caret has
/// typed none. So both are offered and the user chooses — which is the one
/// namespace here where a name is more than one row.
#[test]
fn every_overload_is_its_own_row() {
    let items = complete_at_cursor(
        "package p;
class Test {
    void over(int x) {}
    void over(String x) {}
    void m() {
        <cur>
    }
}
",
    );

    let details: Vec<&str> = items
        .iter()
        .filter(|item| item.label == "over")
        .filter_map(|item| item.detail.as_deref())
        .collect();
    assert_eq!(details.len(), 2);
    assert!(details.contains(&"(int) -> void"));
    assert!(details.contains(&"(String) -> void"));
}

/// The signature is the only thing telling two overloads apart in the list, so
/// a parameter has to carry the brackets §10.2 gave it — including the pair an
/// ellipsis stands for (§8.4.1). Written both ways round because they are the
/// same declared type, and §8.4.2 would call them override-equivalent.
#[test]
fn a_signature_carries_the_brackets_of_its_parameters() {
    let items = complete_at_cursor(
        "package p;
class Test {
    int[] spread(String... args) { return null; }
    int[] array(String[] args) { return null; }
    void m() {
        <cur>
    }
}
",
    );

    assert_eq!(
        item(&items, "spread").detail.as_deref(),
        Some("(String[]) -> int[]")
    );
    assert_eq!(
        item(&items, "array").detail.as_deref(),
        Some("(String[]) -> int[]")
    );
}

#[test]
fn a_method_of_another_type_in_the_file_is_not_offered() {
    let items = complete_at_cursor(
        "package p;
class Test {
    void m() {
        <cur>
    }
}
class Elsewhere {
    void hidden() {}
}
",
    );

    assert!(!labels(&items).contains(&"hidden"));
}
