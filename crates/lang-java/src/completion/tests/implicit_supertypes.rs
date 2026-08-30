//! §8.1.4's supertype that nobody writes, and §9.6's.
//!
//! Each fixture puts the implicit target in the lake by hand. Without one there
//! is nothing to reach, which is exactly what `examples/beans` looks like with
//! `jdk_home` commented out.

use super::*;

fn object_with_to_string() -> jvm::model::Class {
    let mut object = compiled_class("java.lang.Object");
    object.methods = vec![compiled_method("toString", Vec::new(), "java.lang.String")];
    object
}

/// The one everybody meets. §8.1.4: *"For a class other than `Object` with a
/// normal class declaration, the direct superclass type is `Object`."* Nothing
/// in the source says so, so the walk has to add it.
#[test]
fn a_class_with_no_extends_clause_reaches_object() {
    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Widget {
    int own;
}
class Test {
    void m(Widget widget) {
        widget.<cur>
    }
}
",
    )])
    .compiled_class("Object.class", object_with_to_string())
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"own"));
    assert!(labels.contains(&"toString()"));
}

/// §8.1.4 again, one hop further out: the implicit edge is followed at the top
/// of a written chain, not at every step, so a subclass reaches `Object` through
/// its superclass rather than beside it.
#[test]
fn a_subclass_reaches_object_through_the_top_of_its_chain() {
    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Base {
}
class Widget extends Base {
}
class Test {
    void m(Widget widget) {
        widget.<cur>
    }
}
",
    )])
    .compiled_class("Object.class", object_with_to_string())
    .complete();

    assert!(labels(&items).contains(&"toString()"));
}

/// §8.1.4: *"For an enum class E, the direct superclass type is `Enum<E>`."*
/// Not a longer way of saying `Object` — `Enum` is where `name()` and
/// `ordinal()` live, and jumping past it would lose them.
#[test]
fn an_enum_reaches_enum_before_object() {
    let mut enum_class = compiled_class("java.lang.Enum");
    enum_class.superclass = Some(jvm::model::BinaryName::new("java.lang.Object"));
    enum_class.methods = vec![compiled_method("name", Vec::new(), "java.lang.String")];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
enum Season {
    WINTER
}
class Test {
    void m(Season season) {
        season.<cur>
    }
}
",
    )])
    .compiled_class("Object.class", object_with_to_string())
    .compiled_class("Enum.class", enum_class)
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"name()"));
    assert!(labels.contains(&"toString()"));
}

/// §8.1.4: *"For a record class R, the direct superclass type is `Record`."*
/// §8.10 adds that a record declaration has no `extends` clause at all, so this
/// edge is the only way a record has a supertype.
#[test]
fn a_record_reaches_record_before_object() {
    let mut record_class = compiled_class("java.lang.Record");
    record_class.superclass = Some(jvm::model::BinaryName::new("java.lang.Object"));

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
record Point(int i, int j) {
}
class Test {
    void m(Point point) {
        point.<cur>
    }
}
",
    )])
    .compiled_class("Object.class", object_with_to_string())
    .compiled_class("Record.class", record_class)
    .complete();

    assert!(labels(&items).contains(&"toString()"));
}

/// §9.6 gives an annotation interface `java.lang.annotation.Annotation`, which
/// is the fifth arm and the only one that is not in `java.lang`.
#[test]
fn an_annotation_interface_reaches_annotation() {
    let mut annotation = compiled_class("java.lang.annotation.Annotation");
    annotation.kind = jvm::model::TypeKind::Interface;
    annotation.methods = vec![compiled_method(
        "annotationType",
        Vec::new(),
        "java.lang.Class",
    )];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
@interface Marker {
}
class Test {
    void m(Marker marker) {
        marker.<cur>
    }
}
",
    )])
    .compiled_class("Annotation.class", annotation)
    .complete();

    assert!(labels(&items).contains(&"annotationType()"));
}

/// §9.2 says an interface does *not* inherit from `Object`, it implicitly
/// declares members matching `Object`'s `public` ones. We reach them by the
/// edge rather than by declaring them, which shows the same rows for every
/// `public` method and is wrong only for the `protected` ones. `TODO.md` has it.
#[test]
fn an_interface_receiver_is_offered_objects_public_methods() {
    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
interface Marker {
    void mark();
}
class Test {
    void m(Marker marker) {
        marker.<cur>
    }
}
",
    )])
    .compiled_class("Object.class", object_with_to_string())
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"mark()"));
    assert!(labels.contains(&"toString()"));
}

/// A written `extends` clause replaces the implicit edge rather than joining it,
/// so `Object` is reached once and through the superclass.
#[test]
fn a_written_extends_clause_replaces_the_implicit_one() {
    let mut base = compiled_class("p.Base");
    base.superclass = Some(jvm::model::BinaryName::new("java.lang.Object"));

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Widget extends Base {
}
class Test {
    void m(Widget widget) {
        widget.<cur>
    }
}
",
    )])
    .compiled_class("Object.class", object_with_to_string())
    .compiled_class("Base.class", base)
    .complete();

    assert_eq!(
        labels(&items)
            .iter()
            .filter(|label| **label == "toString()")
            .count(),
        1
    );
}

/// With no JDK in the lake the edge points at nothing, which is what
/// `examples/beans` looks like and why nothing regressed when it landed.
#[test]
fn an_empty_lake_offers_no_object_members() {
    let items = complete_at_cursor(
        "package p;
class Widget {
    int own;
}
class Test {
    void m(Widget widget) {
        widget.<cur>
    }
}
",
    );

    assert_eq!(labels(&items), vec!["own"]);
}
