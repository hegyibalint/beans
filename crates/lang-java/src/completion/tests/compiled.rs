//! What a class file contributes: its own members, and its half of a hierarchy.
//!
//! Everything here is a hand-built lake rather than a real JDK, because the
//! claim is about reading `jvm::model::Class` and not about reading a jimage.

use super::*;

/// A receiver whose type we never parsed still has members. JVMS §4.5 and §4.6
/// put them in the class file and `class_file.rs` decoded them; this is the
/// first thing that reads them.
#[test]
fn a_compiled_receiver_offers_its_own_members() {
    let mut instant = compiled_class("p.Instant");
    instant.methods = vec![compiled_method("toString", Vec::new(), "java.lang.String")];
    instant.fields = vec![jvm::model::Field {
        name: "EPOCH".to_string(),
        access: jvm::model::AccessLevel::Public,
        jvm_type: class_type("p.Instant"),
    }];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    void m() {
        Instant seen = null;
        seen.<cur>
    }
}
",
    )])
    .compiled_class("Instant.class", instant)
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"toString()"));
    assert!(labels.contains(&"EPOCH"));
}

/// A method we decoded has parameter types and no parameter names, JVMS §4.7.24
/// making `MethodParameters` optional, so the row shows the types simple-named.
#[test]
fn a_compiled_method_row_shows_types_without_names() {
    let mut owner = compiled_class("p.Owner");
    owner.methods = vec![compiled_method(
        "join",
        vec![class_type("java.lang.String"), class_type("p.Owner")],
        "java.lang.String",
    )];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    void m() {
        Owner owner = null;
        owner.<cur>
    }
}
",
    )])
    .compiled_class("Owner.class", owner)
    .complete();

    assert_eq!(item(&items, "join").label, "join(String, Owner)");
    assert_eq!(item(&items, "join").detail.as_deref(), Some("String"));
}

/// The hop the walk could not make before. `Widget extends Base` where `Base`
/// is a class file: §8.2 does not care which half of the lake a supertype is in,
/// and neither does the walk any more.
#[test]
fn a_parsed_class_inherits_from_a_compiled_superclass() {
    let mut base = compiled_class("p.Base");
    base.methods = vec![compiled_method("inherited", Vec::new(), "java.lang.String")];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Widget extends Base {
    int own;
}
class Test {
    void m(Widget widget) {
        widget.<cur>
    }
}
",
    )])
    .compiled_class("Base.class", base)
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"own"));
    assert!(labels.contains(&"inherited()"));
}

/// And the hop after it. JVMS §4.1 puts a class file's own superclass in the
/// class file, so a compiled type walks on to its compiled supertype with no
/// resolution at all — which is how a record reaches `Object` through `Record`.
#[test]
fn a_compiled_class_walks_on_to_its_own_supertype() {
    let mut object = compiled_class("java.lang.Object");
    object.methods = vec![compiled_method("toString", Vec::new(), "java.lang.String")];

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
    .compiled_class("Object.class", object)
    .compiled_class("Base.class", base)
    .complete();

    assert!(labels(&items).contains(&"toString()"));
}

/// §6.6.1 with one end in a class file. The level is decoded (JVMS §4.5, §4.6)
/// and `is_compiled_accessible` reads it the same way it reads a type's.
#[test]
fn a_private_member_of_a_compiled_type_is_not_offered() {
    let mut owner = compiled_class("p.Owner");
    owner.fields = vec![
        jvm::model::Field {
            name: "open".to_string(),
            access: jvm::model::AccessLevel::Public,
            jvm_type: class_type("java.lang.String"),
        },
        jvm::model::Field {
            name: "secret".to_string(),
            access: jvm::model::AccessLevel::Private,
            jvm_type: class_type("java.lang.String"),
        },
    ];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    void m() {
        Owner owner = null;
        owner.<cur>
    }
}
",
    )])
    .compiled_class("Owner.class", owner)
    .complete();

    let labels = labels(&items);
    assert!(labels.contains(&"open"));
    assert!(!labels.contains(&"secret"));
}

/// JVMS §2.9 names a constructor `<init>`, which is in the method table and is
/// not a member (§8.2) — nor a name anybody could type, `<` not being an
/// identifier character (§3.8).
#[test]
fn a_compiled_constructor_is_not_a_member() {
    let mut owner = compiled_class("p.Owner");
    owner.methods = vec![
        compiled_method("<init>", Vec::new(), "java.lang.Object"),
        compiled_method("real", Vec::new(), "java.lang.String"),
    ];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    void m() {
        Owner owner = null;
        owner.<cur>
    }
}
",
    )])
    .compiled_class("Owner.class", owner)
    .complete();

    assert_eq!(labels(&items), vec!["real()"]);
}

/// A chain steps through a compiled member's type the same way it steps through
/// a parsed one: JVMS §4.3.2 already decoded the descriptor to a binary name, so
/// there is nothing left for §6.5.5 to resolve.
#[test]
fn a_chain_walks_through_a_compiled_members_type() {
    let mut string = compiled_class("java.lang.String");
    string.methods = vec![compiled_method("strip", Vec::new(), "java.lang.String")];

    let mut owner = compiled_class("p.Owner");
    owner.fields = vec![jvm::model::Field {
        name: "label".to_string(),
        access: jvm::model::AccessLevel::Public,
        jvm_type: class_type("java.lang.String"),
    }];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    void m() {
        Owner owner = null;
        owner.label.<cur>
    }
}
",
    )])
    .compiled_class("String.class", string)
    .compiled_class("Owner.class", owner)
    .complete();

    assert!(labels(&items).contains(&"strip()"));
}

/// §8.4.8: an override collapses onto the nearest declaration even when the two
/// halves live in different halves of the lake.
#[test]
fn a_parsed_override_hides_the_compiled_method_it_overrides() {
    let mut base = compiled_class("p.Base");
    base.methods = vec![compiled_method("toString", Vec::new(), "java.lang.String")];

    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Widget extends Base {
    public String toString() { return null; }
}
class Test {
    void m(Widget widget) {
        widget.<cur>
    }
}
",
    )])
    .compiled_class("Base.class", base)
    .complete();

    assert_eq!(
        labels(&items)
            .iter()
            .filter(|label| label.starts_with("toString"))
            .count(),
        1
    );
}
