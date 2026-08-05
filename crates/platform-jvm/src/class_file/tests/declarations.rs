use crate::model::{
    JvmAccessLevel, JvmKind, JvmPrimitive, JvmQualifiedName, JvmReturnType, JvmType,
};

use super::parse_type;

const FEATURE: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature.class");
const MEMBER: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature$Member.class");
const GUARDED: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature$Guarded.class");
const SHARED: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature$Shared.class");
const HIDDEN: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature$Hidden.class");
const LOCAL: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature$1Local.class");
const POINT: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Point.class");
const MARKER: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Marker.class");
const MODE: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Mode.class");
const CONTRACT: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Contract.class");
const MODULE: &[u8] = include_bytes!("fixtures/classes/module-info.class");

#[test]
fn internal_names_and_descriptors_become_jvm_model_types() {
    let class = parse_type(FEATURE);

    assert_eq!(class.fqn.as_str(), "beans.fixture.Feature");
    assert_eq!(
        class.superclass.as_ref().map(JvmQualifiedName::as_str),
        Some("java.lang.Object")
    );
    assert_eq!(class.interfaces[0].as_str(), "java.io.Serializable");

    assert_eq!(class.fields[0].name, "values");
    assert_eq!(
        class.fields[0].jvm_type,
        JvmType::Array(Box::new(JvmType::Primitive(JvmPrimitive::Int)))
    );

    let combine = class
        .methods
        .iter()
        .find(|method| method.name == "combine")
        .expect("fixture declares combine");
    assert_eq!(
        combine.params,
        [
            JvmType::Class(JvmQualifiedName::new("java.lang.String")),
            JvmType::Primitive(JvmPrimitive::Long),
        ]
    );
    assert_eq!(
        combine.return_type,
        JvmReturnType::Value(JvmType::Array(Box::new(JvmType::Array(Box::new(
            JvmType::Class(JvmQualifiedName::new("java.lang.Object")),
        )))))
    );
}

#[test]
fn class_flags_and_the_record_attribute_determine_type_kind() {
    for (bytes, expected) in [
        (FEATURE, JvmKind::Class),
        (POINT, JvmKind::Record),
        (MARKER, JvmKind::AnnotationInterface),
        (MODE, JvmKind::Enum),
        (CONTRACT, JvmKind::Interface),
    ] {
        assert_eq!(parse_type(bytes).kind, expected);
    }
}

/// A nested class is the case the header cannot answer: JVMS Table 4.1-B has no
/// `ACC_PRIVATE` and no `ACC_PROTECTED`, so javac widens `protected` to
/// `ACC_PUBLIC` and writes nothing at all for `private`. Reading §4.7.6's entry
/// instead is what keeps `Guarded` and `Hidden` apart from `Member` and
/// `Shared`, and this test is the only thing that says so.
#[test]
fn a_nested_class_keeps_the_access_level_of_its_source() {
    for (bytes, expected) in [
        (FEATURE, Some(JvmAccessLevel::Public)),
        (CONTRACT, Some(JvmAccessLevel::Package)),
        (MEMBER, Some(JvmAccessLevel::Public)),
        (GUARDED, Some(JvmAccessLevel::Protected)),
        (SHARED, Some(JvmAccessLevel::Package)),
        (HIDDEN, Some(JvmAccessLevel::Private)),
        // JLS §8.1.1: access control does not reach a local class.
        (LOCAL, None),
    ] {
        assert_eq!(parse_type(bytes).access, expected);
    }
}

/// JVMS §4.5 and §4.6 spell all three bits, so a member needs none of the
/// recovery above; none of them set is package access.
#[test]
fn fields_and_methods_carry_their_own_access_level() {
    let class = parse_type(FEATURE);
    let field = |name: &str| {
        let field = class.fields.iter().find(|field| field.name == name);
        field.expect("the fixture declares it").access
    };
    let method = |name: &str| {
        let method = class.methods.iter().find(|method| method.name == name);
        method.expect("the fixture declares it").access
    };

    assert_eq!(field("values"), JvmAccessLevel::Public);
    assert_eq!(field("guarded"), JvmAccessLevel::Protected);
    assert_eq!(field("shared"), JvmAccessLevel::Package);
    assert_eq!(field("hidden"), JvmAccessLevel::Private);

    assert_eq!(method("combine"), JvmAccessLevel::Public);
    assert_eq!(method("guard"), JvmAccessLevel::Protected);
    assert_eq!(method("share"), JvmAccessLevel::Package);
    assert_eq!(method("hide"), JvmAccessLevel::Private);
}

#[test]
fn member_and_local_classes_retain_their_enclosing_class() {
    for bytes in [MEMBER, LOCAL] {
        let class = parse_type(bytes);
        assert_eq!(
            class.enclosing.as_ref().map(JvmQualifiedName::as_str),
            Some("beans.fixture.Feature")
        );
    }
}

#[test]
fn a_module_descriptor_is_not_projected_as_a_class() {
    assert!(matches!(
        super::super::parse(MODULE).unwrap(),
        super::super::ParseOutcome::ModuleDescriptor
    ));
}
