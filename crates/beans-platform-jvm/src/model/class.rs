use std::io::Read;

use serde::{Deserialize, Serialize};

use crate::{ClassAccessFlags, Field, Method};

/// Stored declarations of one class file (JVMS §4.1), without bytecode or attributes.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Class {
    /// Internal binary name, with `/` separating package components (JVMS §4.2.1).
    pub name: String,
    /// SHA-256 of the complete class-file bytes used to produce this model.
    pub fingerprint: [u8; 32],
    #[serde(with = "super::access_flags::class")]
    pub access_flags: ClassAccessFlags,
    pub superclass: Option<String>,
    pub interfaces: Vec<String>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
}

impl Class {
    /// Reads a class-file snapshot. Unrecognized formats return `Ok(None)`.
    pub fn read(reader: impl Read) -> Result<Option<Self>, String> {
        crate::parser::parse(reader)
    }
}
