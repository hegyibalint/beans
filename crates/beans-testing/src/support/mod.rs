pub mod jdks;

use std::io::{Cursor, Write};

use beans_platform_jvm::{Class, ClassAccessFlags};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

/// A minimal public class model with a dummy fingerprint and no backing artifact.
pub fn class_model(name: &str) -> Class {
    Class {
        name: name.into(),
        fingerprint: [0; 32],
        access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::SUPER,
        superclass: (name != "java/lang/Object").then(|| "java/lang/Object".into()),
        interfaces: Vec::new(),
        fields: Vec::new(),
        methods: Vec::new(),
    }
}

pub fn class_bytes(name: &str) -> Vec<u8> {
    let mut bytes = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52, 0, 5];
    for (name, index) in [(name, 1u16), ("java/lang/Object", 3)] {
        bytes.push(1);
        bytes.extend((name.len() as u16).to_be_bytes());
        bytes.extend(name.as_bytes());
        bytes.push(7);
        bytes.extend(index.to_be_bytes());
    }
    for value in [0x21u16, 2, 4, 0, 0, 0, 0] {
        bytes.extend(value.to_be_bytes());
    }
    bytes
}

pub fn jar_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
