//! §8.2 and §9.2: the members a receiver has that its own body never declared.

use super::*;

/// A subclass has its superclass's members as its own, so the list is longer
/// than what the receiver's body spells.
#[test]
fn a_superclass_contributes_its_members() {
    let items = Workspace::of(&[
        (
            "p/Base.java",
            "package p;
public class Base {
    public int inherited;
    public void shared() {}
}
",
        ),
        (
            "p/Widget.java",
            "package p;
public class Widget extends Base {
    public int own;
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    void m() {
        Widget widget = null;
        widget.<cur>
    }
}
",
        ),
    ])
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"own"));
    assert!(labels.contains(&"inherited"));
    assert!(labels.contains(&"shared()"));
}

/// §9.2 does for a superinterface what §8.2 does for a superclass, and
/// `implements` is the other clause the walk has to read.
#[test]
fn a_superinterface_contributes_its_members() {
    let items = Workspace::of(&[
        (
            "p/Marker.java",
            "package p;
public interface Marker {
    void mark();
}
",
        ),
        (
            "p/Widget.java",
            "package p;
public class Widget implements Marker {
    public int own;
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    void m() {
        Widget widget = null;
        widget.<cur>
    }
}
",
        ),
    ])
    .complete();

    assert!(labels(&items).contains(&"mark()"));
}

/// The walk is transitive, because §8.2 defines a class's members in terms of
/// its superclass's *members* and not of its superclass's declarations.
#[test]
fn a_grandparents_members_arrive_too() {
    let items = complete_at_cursor(
        "package p;
class Grand {
    int fromGrand;
}
class Parent extends Grand {
}
class Child extends Parent {
}
class Test {
    void m() {
        Child child = null;
        child.<cur>
    }
}
",
    );

    assert!(labels(&items).contains(&"fromGrand"));
}

/// §8.3: a field declared in a subclass *hides* one of the same name above it.
/// Two declarations, one name, and the row that survives has to be the nearer
/// one — which is what the detail says.
#[test]
fn a_hidden_field_is_offered_once_by_its_nearest_declaration() {
    let items = complete_at_cursor(
        "package p;
class Base {
    String value;
}
class Widget extends Base {
    int value;
}
class Test {
    void m() {
        Widget widget = null;
        widget.<cur>
    }
}
",
    );

    assert_eq!(labels(&items).iter().filter(|l| **l == "value").count(), 1);
    assert_eq!(item(&items, "value").detail.as_deref(), Some("int"));
}

/// §8.2 excludes a private member from what a subclass inherits, and §6.6.1 is
/// where that falls out rather than a rule of its own: the declaration is in
/// another top level type from the caret, so the filter every member already
/// goes through drops it.
#[test]
fn a_private_member_of_a_superclass_is_not_inherited() {
    let items = Workspace::of(&[
        (
            "p/Base.java",
            "package p;
public class Base {
    public int open;
    private int secret;
}
",
        ),
        (
            "p/Widget.java",
            "package p;
public class Widget extends Base {
}
",
        ),
        (
            "p/Test.java",
            "package p;
class Test {
    void m() {
        Widget widget = null;
        widget.<cur>
    }
}
",
        ),
    ])
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"open"));
    assert!(!labels.contains(&"secret"));
}

/// §8.4.8's overriding against §8.4.9's overloading. One name, three
/// declarations, and only the pair that differ in §8.4.2's parameter types are
/// two things a user can pick between.
#[test]
fn an_override_collapses_and_an_overload_does_not() {
    let items = complete_at_cursor(
        "package p;
class Base {
    void describe(int factor) {}
    void describe(String label) {}
}
class Widget extends Base {
    void describe(int factor) {}
}
class Test {
    void m() {
        Widget widget = null;
        widget.<cur>
    }
}
",
    );

    let mut described: Vec<&str> = labels(&items)
        .into_iter()
        .filter(|label| label.starts_with("describe"))
        .collect();
    described.sort();

    assert_eq!(
        described,
        ["describe(String label)", "describe(int factor)"]
    );
}

/// §8.1.5 lets two superinterfaces share an ancestor, so one type is reached
/// twice and its members still have to be offered once.
#[test]
fn a_diamond_offers_a_shared_ancestors_member_once() {
    let items = complete_at_cursor(
        "package p;
interface Top {
    void top();
}
interface Left extends Top {
}
interface Right extends Top {
}
class Widget implements Left, Right {
}
class Test {
    void m() {
        Widget widget = null;
        widget.<cur>
    }
}
",
    );

    assert_eq!(labels(&items).iter().filter(|l| **l == "top()").count(), 1);
}

/// §8.1.4 forbids a class being its own superclass, and an editor sees the
/// forbidden state on the way to a legal one. Which members come back does not
/// matter here; terminating does.
#[test]
fn a_cycle_in_the_hierarchy_terminates() {
    let items = complete_at_cursor(
        "package p;
class A extends B {
    int fromA;
}
class B extends A {
    int fromB;
}
class Test {
    void m() {
        A a = null;
        a.<cur>
    }
}
",
    );

    let labels = labels(&items);
    assert!(labels.contains(&"fromA"));
    assert!(labels.contains(&"fromB"));
}

/// A chain steps through an inherited field the same way it steps through a
/// declared one: §6.5.2.1 asks what the segment denotes, and §8.2 has already
/// made the answer a member of the type.
#[test]
fn a_chain_walks_through_an_inherited_field() {
    let items = complete_at_cursor(
        "package p;
class Leaf {
    int value;
}
class Base {
    Leaf leaf;
}
class Branch extends Base {
}
class Test {
    void m() {
        Branch branch = null;
        branch.leaf.<cur>
    }
}
",
    );

    assert!(labels(&items).contains(&"value"));
}

/// §8.5 inherits a member type too, which is the third namespace and the one
/// with a hole behind it: the row is offered and `resolve_type_name` cannot
/// follow it, because its member-type stage reads one body scope. `TODO.md`
/// carries that half.
#[test]
fn a_member_type_of_a_superclass_is_offered() {
    let items = complete_at_cursor(
        "package p;
class Base {
    class Nested {}
}
class Widget extends Base {
}
class Test {
    void m() {
        Widget.<cur>
    }
}
",
    );

    assert!(labels(&items).contains(&"Nested"));
}
