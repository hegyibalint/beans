use std::io::Read;

use cafebabe::{
    ClassFile,
    descriptors::{FieldDescriptor, ReturnDescriptor},
};
use sha2::{Digest, Sha256};

use crate::{Class, ElementType, Field, FieldType, Method, MethodAccessFlags};

pub fn parse(mut reader: impl Read) -> Result<Option<Class>, String> {
    let mut options = cafebabe::ParseOptions::default();
    options.parse_bytecode(false);

    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;

    // See JVMS §4: the class-file magic is 0xCAFEBABE.
    if !bytes.starts_with(&[0xCA, 0xFE, 0xBA, 0xBE]) {
        return Ok(None);
    }

    let class =
        cafebabe::parse_class_with_options(&bytes, &options).map_err(|error| error.to_string())?;
    Ok(Some(class_model(class, Sha256::digest(&bytes).into())))
}

fn class_model(class: ClassFile<'_>, fingerprint: [u8; 32]) -> Class {
    let fields = class
        .fields
        .into_iter()
        .map(|field| Field {
            name: field.name.into_owned(),
            access_flags: field.access_flags,
            field_type: field_type(field.descriptor),
        })
        .collect();
    let methods = class
        .methods
        .into_iter()
        .map(|method| {
            let mut access_flags = method.access_flags;
            // Outside these versions this bit is reserved (JVMS §4.6).
            if !(46..=60).contains(&class.major_version) {
                access_flags.remove(MethodAccessFlags::STRICT);
            }
            Method {
                name: method.name.into_owned(),
                access_flags,
                parameters: method
                    .descriptor
                    .parameters
                    .into_iter()
                    .map(field_type)
                    .collect(),
                return_type: match method.descriptor.return_type {
                    ReturnDescriptor::Void => None,
                    ReturnDescriptor::Return(descriptor) => Some(field_type(descriptor)),
                },
            }
        })
        .collect();

    Class {
        name: class.this_class.to_string(),
        fingerprint,
        access_flags: class.access_flags,
        superclass: class.super_class.map(|name| name.to_string()),
        interfaces: class
            .interfaces
            .into_iter()
            .map(|name| name.to_string())
            .collect(),
        fields,
        methods,
    }
}

fn field_type(descriptor: FieldDescriptor<'_>) -> FieldType {
    use cafebabe::descriptors::FieldType as ParsedType;

    let element = match descriptor.field_type {
        ParsedType::Byte => ElementType::Byte,
        ParsedType::Char => ElementType::Char,
        ParsedType::Double => ElementType::Double,
        ParsedType::Float => ElementType::Float,
        ParsedType::Integer => ElementType::Int,
        ParsedType::Long => ElementType::Long,
        ParsedType::Short => ElementType::Short,
        ParsedType::Boolean => ElementType::Boolean,
        ParsedType::Object(name) => ElementType::Reference(name.to_string()),
    };
    FieldType {
        dimensions: descriptor.dimensions,
        element,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use super::parse;

    #[test]
    fn unrecognized_contents_return_none() {
        for bytes in [b"".as_slice(), b"not a class", &[0xCA, 0xFE, 0xBA]] {
            assert!(parse(bytes).unwrap().is_none());
        }
    }

    #[test]
    fn reader_failures_return_an_error() {
        struct BrokenReader;

        impl Read for BrokenReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("read failed"))
            }
        }

        assert!(matches!(parse(BrokenReader), Err(error) if error == "read failed"));
    }
}
