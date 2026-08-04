use std::fmt;

use cafebabe::attributes::{AttributeData, AttributeInfo};
use cafebabe::descriptors::{ClassName, FieldDescriptor, FieldType, ReturnDescriptor};
use cafebabe::{ClassAccessFlags, ParseOptions};

use crate::model::{
    JvmClass, JvmField, JvmKind, JvmMethod, JvmPrimitive, JvmQualifiedName, JvmReturnType, JvmType,
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
            jvm_type: jvm_type(&field.descriptor),
        })
        .collect();
    let methods = class
        .methods
        .iter()
        .map(|method| JvmMethod {
            name: method.name.to_string(),
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
        attributes.iter().find_map(|attribute| {
            let AttributeData::InnerClasses(classes) = &attribute.data else {
                return None;
            };
            classes
                .iter()
                .find(|entry| entry.inner_class_info.as_ref() == &**this_class)
                .and_then(|entry| entry.outer_class_info.as_deref())
                .map(qualified_name)
        })
    })
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
