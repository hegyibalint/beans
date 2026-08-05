use std::fmt;

use cafebabe::attributes::{AttributeData, AttributeInfo, InnerClassEntry};
use cafebabe::descriptors::{ClassName, FieldDescriptor, FieldType, ReturnDescriptor};
use cafebabe::{AccessFlags, ClassAccessFlags, ParseOptions};

use crate::model::{
    JvmAccessLevel, JvmClass, JvmField, JvmKind, JvmMethod, JvmPrimitive, JvmQualifiedName,
    JvmReturnType, JvmType,
};

#[derive(Debug)]
pub(crate) struct ParseError(cafebabe::ParseError);

#[derive(Debug)]
pub(crate) enum ParseOutcome {
    Class(JvmClass),
    ModuleDescriptor,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<ParseOutcome, ParseError> {
    let mut options = ParseOptions::default();
    options.parse_bytecode(false);
    let class = cafebabe::parse_class_with_options(bytes, &options).map_err(ParseError)?;

    if class.access_flags.contains(ClassAccessFlags::MODULE) {
        return Ok(ParseOutcome::ModuleDescriptor);
    }

    let fqn = qualified_name(&class.this_class);
    let enclosing = enclosing_class(&class.this_class, &class.attributes);
    let access = class_access(&class.this_class, class.access_flags, &class.attributes);
    let kind = class_kind(class.access_flags, &class.attributes);
    let superclass = class.super_class.as_ref().map(|name| qualified_name(name));
    let interfaces = class
        .interfaces
        .iter()
        .map(|name| qualified_name(name))
        .collect();
    let fields = class
        .fields
        .iter()
        .map(|field| JvmField {
            name: field.name.to_string(),
            access: access_level(field.access_flags.bits()),
            jvm_type: jvm_type(&field.descriptor),
        })
        .collect();
    let methods = class
        .methods
        .iter()
        .map(|method| JvmMethod {
            name: method.name.to_string(),
            access: access_level(method.access_flags.bits()),
            params: method.descriptor.parameters.iter().map(jvm_type).collect(),
            return_type: match &method.descriptor.return_type {
                ReturnDescriptor::Return(descriptor) => JvmReturnType::Value(jvm_type(descriptor)),
                ReturnDescriptor::Void => JvmReturnType::Void,
            },
        })
        .collect();

    Ok(ParseOutcome::Class(JvmClass {
        fqn,
        kind,
        access,
        enclosing,
        superclass,
        interfaces,
        fields,
        methods,
    }))
}

fn class_kind(flags: ClassAccessFlags, attributes: &[AttributeInfo<'_>]) -> JvmKind {
    if flags.contains(ClassAccessFlags::ANNOTATION) {
        JvmKind::AnnotationInterface
    } else if flags.contains(ClassAccessFlags::ENUM) {
        JvmKind::Enum
    } else if attributes
        .iter()
        .any(|attribute| matches!(attribute.data, AttributeData::Record(_)))
    {
        JvmKind::Record
    } else if flags.contains(ClassAccessFlags::INTERFACE) {
        JvmKind::Interface
    } else {
        JvmKind::Class
    }
}

fn enclosing_class(
    this_class: &ClassName<'_>,
    attributes: &[AttributeInfo<'_>],
) -> Option<JvmQualifiedName> {
    let lexical_enclosing = attributes
        .iter()
        .find_map(|attribute| match &attribute.data {
            AttributeData::EnclosingMethod { class_name, .. } => Some(qualified_name(class_name)),
            _ => None,
        });

    lexical_enclosing.or_else(|| {
        inner_class_entry(this_class, attributes)
            .and_then(|entry| entry.outer_class_info.as_deref())
            .map(qualified_name)
    })
}

/// JVMS §4.1's Table 4.1-B has neither `ACC_PRIVATE` nor `ACC_PROTECTED`, so a
/// nested class's own header cannot spell what its source said: javac widens
/// `protected` to `ACC_PUBLIC` and drops `private` for nothing at all. §4.7.6
/// keeps the original for precisely this reason — its flags are "used by a
/// compiler to recover the original information when source code is not
/// available" — so the entry naming this class wins wherever there is one. A
/// top-level class is a package member and §4.7.6 gives it none, which is also
/// the only case where the header is complete.
fn class_access(
    this_class: &ClassName<'_>,
    flags: ClassAccessFlags,
    attributes: &[AttributeInfo<'_>],
) -> Option<JvmAccessLevel> {
    let Some(entry) = inner_class_entry(this_class, attributes) else {
        return Some(access_level(flags.bits()));
    };

    // §4.7.6 zeroes `outer_class_info_index` for a local or anonymous class,
    // which JLS §8.1.1 leaves outside access control altogether.
    entry
        .outer_class_info
        .is_some()
        .then(|| access_level(entry.access_flags.bits()))
}

fn inner_class_entry<'a, 'class>(
    this_class: &ClassName<'_>,
    attributes: &'a [AttributeInfo<'class>],
) -> Option<&'a InnerClassEntry<'class>> {
    attributes.iter().find_map(|attribute| {
        let AttributeData::InnerClasses(classes) = &attribute.data else {
            return None;
        };
        classes
            .iter()
            .find(|entry| entry.inner_class_info.as_ref() == &**this_class)
    })
}

/// The three access bits, which JVMS gives one meaning and one value in every
/// table that carries them (§4.1, §4.5, §4.6, §4.7.6). None of them set is
/// package access, so the answer is total and a caller never combines bits.
fn access_level(flags: u16) -> JvmAccessLevel {
    let is_set = |flag: AccessFlags| flags & flag.bits() != 0;

    if is_set(AccessFlags::PUBLIC) {
        JvmAccessLevel::Public
    } else if is_set(AccessFlags::PROTECTED) {
        JvmAccessLevel::Protected
    } else if is_set(AccessFlags::PRIVATE) {
        JvmAccessLevel::Private
    } else {
        JvmAccessLevel::Package
    }
}

fn qualified_name(internal_name: &str) -> JvmQualifiedName {
    JvmQualifiedName::new(internal_name.replace('/', "."))
}

fn jvm_type(descriptor: &FieldDescriptor<'_>) -> JvmType {
    let mut jvm_type = match &descriptor.field_type {
        FieldType::Byte => JvmType::Primitive(JvmPrimitive::Byte),
        FieldType::Char => JvmType::Primitive(JvmPrimitive::Char),
        FieldType::Double => JvmType::Primitive(JvmPrimitive::Double),
        FieldType::Float => JvmType::Primitive(JvmPrimitive::Float),
        FieldType::Integer => JvmType::Primitive(JvmPrimitive::Int),
        FieldType::Long => JvmType::Primitive(JvmPrimitive::Long),
        FieldType::Short => JvmType::Primitive(JvmPrimitive::Short),
        FieldType::Boolean => JvmType::Primitive(JvmPrimitive::Boolean),
        FieldType::Object(name) => JvmType::Class(qualified_name(name)),
    };
    for _ in 0..descriptor.dimensions {
        jvm_type = JvmType::Array(Box::new(jvm_type));
    }
    jvm_type
}

#[cfg(test)]
mod tests;
