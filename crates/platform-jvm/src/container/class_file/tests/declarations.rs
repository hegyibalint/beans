use crate::model::{JvmKind, JvmPrimitive, JvmQualifiedName, JvmReturnType, JvmType};

use super::parse_type;

const FEATURE: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature.class");
const MEMBER: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature$Member.class");
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
