//! §6.5.6.2 and §6.5.7.2: what a name after a `.` may be.

use super::*;

/// The receiver's declared type decides the list, so a local of another type
/// offers that type's members and none of the caret's own.
#[test]
fn a_local_offers_the_members_of_its_declared_type() {
    let items = Workspace::of(&[
        (
            "p/Other.java",
            "package p;
public class Other {
    public int shared;
    public void greet() {}
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    int mine;
    void m() {
        Other other = null;
        other.<cur>
    }
}
",
        ),
    ])
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"shared"));
    assert!(labels.contains(&"greet()"));
    // Nothing from the scope chain: §6.5.6.2 asks about a member, not a name.
    assert!(!labels.contains(&"mine"));
    assert!(!labels.contains(&"other"));
}

/// §6.6.1 is a relation between a declaration and the place reaching for it,
/// and a member of another top level class is the case it exists for.
#[test]
fn a_private_member_of_another_type_is_not_offered() {
    let items = Workspace::of(&[
        (
            "p/Other.java",
            "package p;
public class Other {
    public int open;
    private int hidden;
    private void secret() {}
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    void m() {
        Other other = null;
        other.<cur>
    }
}
",
        ),
    ])
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"open"));
    assert!(!labels.contains(&"hidden"));
    assert!(!labels.contains(&"secret()"));
}

/// §15.8.3: `this` denotes the instance whose body encloses the caret, so the
/// list is the enclosing type's own members.
///
/// Both halves of `TODO.md`'s runaway-dot entry are asserted here, because the
/// wrong half is what a fix has to turn red. Everything above the caret is
/// offered; `below` is not, having been consumed as the invocation's name. The
/// trailing member also has to exist at all — with `m` last, the whole
/// `class_declaration` collapses into an `ERROR` and there is no type to ask.
#[test]
fn this_offers_the_members_above_the_caret_and_loses_those_below() {
    let items = complete_at_cursor(
        "package p;
class Test {
    int count;
    void helper() {}
    void m() {
        this.<cur>
    }
    void below() {}
}
",
    );

    let labels = labels(&items);
    assert!(labels.contains(&"count"));
    assert!(labels.contains(&"helper()"));
    assert!(!labels.contains(&"below()"));
}

/// §6.5.2.1 classifies a chain left to right: `outer` is a variable, and `inner`
/// is a field of what `outer` denotes.
#[test]
fn a_dotted_chain_walks_one_field_at_a_time() {
    let items = Workspace::of(&[
        (
            "p/Leaf.java",
            "package p;
public class Leaf {
    public int value;
}
",
        ),
        (
            "p/Branch.java",
            "package p;
public class Branch {
    public Leaf leaf;
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    void m() {
        Branch branch = null;
        branch.leaf.<cur>
    }
}
",
        ),
    ])
    .complete();

    assert!(labels(&items).contains(&"value"));
}

/// A receiver that names no variable is a type name instead, which is what
/// reaches a static member (§6.5.6.2's *ExpressionName* against §6.5.5's
/// *TypeName*).
#[test]
fn a_type_name_receiver_offers_that_types_members() {
    let items = Workspace::of(&[
        (
            "p/Config.java",
            "package p;
public class Config {
    public static int limit;
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    void m() {
        Config.<cur>
    }
}
",
        ),
    ])
    .complete();

    assert!(labels(&items).contains(&"limit"));
}

/// A method row after a dot reads the way one before a dot does — the same
/// renderer, so the two lists cannot drift apart.
#[test]
fn a_member_method_row_carries_its_parameters() {
    let items = Workspace::of(&[
        (
            "p/Other.java",
            "package p;
public class Other {
    public void describe(int factor) {}
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    void m() {
        Other other = null;
        other.<cur>
    }
}
",
        ),
    ])
    .complete();

    assert_eq!(item(&items, "describe").label, "describe(int factor)");
}

/// Lookbehind reads a chain of identifiers and stops at anything else, so a
/// call receiver yields nothing. Still qualified, though: §6.5.6.2 rules out
/// answering with the names in scope, so the list is empty rather than wrong.
#[test]
fn a_receiver_that_is_not_a_name_offers_nothing() {
    let items = complete_at_cursor(
        "package p;
class Test {
    int count;
    Test make() { return null; }
    void m() {
        make().<cur>
    }
}
",
    );

    assert!(items.is_empty());
}

/// A prefix after the dot filters the members, the way it filters every other
/// enumeration.
#[test]
fn a_prefix_after_the_dot_filters_the_members() {
    let items = Workspace::of(&[
        (
            "p/Other.java",
            "package p;
public class Other {
    public int alpha;
    public int beta;
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    void m() {
        Other other = null;
        other.al<cur>
    }
}
",
        ),
    ])
    .complete();

    assert_eq!(labels(&items), vec!["alpha"]);
}
