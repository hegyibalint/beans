use beans_platform_jvm_model::classes::{AccessLevel, ClassKind};

use super::lower_class;

const OUTER: &[u8] = include_bytes!("fixtures/classes/fixture/Outer.class");
const ANONYMOUS: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$1.class");
const LOCAL: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$1Local.class");
const ANNOTATION: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$MemberAnnotation.class");
const ENUM: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$MemberEnum.class");
const INTERFACE: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$MemberInterface.class");
const RECORD: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$MemberRecord.class");
const PACKAGE: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$PackageMember.class");
const PRIVATE: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$PrivateMember.class");
const PROTECTED: &[u8] = include_bytes!("fixtures/classes/fixture/Outer$ProtectedMember.class");

#[test]
fn binary_names_and_class_kinds_are_lowered() {
    for (bytes, name, kind) in [
        (OUTER, "fixture.Outer", ClassKind::Class),
        (
            INTERFACE,
            "fixture.Outer$MemberInterface",
            ClassKind::Interface,
        ),
        (
            ANNOTATION,
            "fixture.Outer$MemberAnnotation",
            ClassKind::AnnotationInterface,
        ),
        (ENUM, "fixture.Outer$MemberEnum", ClassKind::Enum),
        (RECORD, "fixture.Outer$MemberRecord", ClassKind::Record),
    ] {
        let class = lower_class(bytes);

        assert_eq!(class.binary_name.as_str(), name);
        assert_eq!(class.kind, kind);
    }
}

#[test]
fn nested_access_comes_from_the_inner_classes_attribute() {
    for (bytes, access) in [
        (PROTECTED, AccessLevel::Protected),
        (PRIVATE, AccessLevel::Private),
        (PACKAGE, AccessLevel::Package),
        (INTERFACE, AccessLevel::Public),
    ] {
        assert_eq!(lower_class(bytes).access, access);
    }
}

#[test]
fn only_direct_member_classes_are_attached_to_their_owner() {
    let outer = lower_class(OUTER);
    let mut members: Vec<_> = outer
        .member_classes
        .iter()
        .map(|member| (member.simple_name.as_str(), member.binary_name.as_str()))
        .collect();
    members.sort_unstable();

    assert_eq!(
        members,
        [
            ("MemberAnnotation", "fixture.Outer$MemberAnnotation"),
            ("MemberEnum", "fixture.Outer$MemberEnum"),
            ("MemberInterface", "fixture.Outer$MemberInterface"),
            ("MemberRecord", "fixture.Outer$MemberRecord"),
            ("PackageMember", "fixture.Outer$PackageMember"),
            ("PrivateMember", "fixture.Outer$PrivateMember"),
            ("ProtectedMember", "fixture.Outer$ProtectedMember"),
        ]
    );
}

#[test]
fn local_and_anonymous_class_files_are_classes() {
    for (bytes, name) in [
        (LOCAL, "fixture.Outer$1Local"),
        (ANONYMOUS, "fixture.Outer$1"),
    ] {
        assert_eq!(lower_class(bytes).binary_name.as_str(), name);
    }
}
