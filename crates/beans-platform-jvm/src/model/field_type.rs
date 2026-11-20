use serde::{Deserialize, Serialize};

/// A field, parameter, or non-void return descriptor (JVMS §4.3).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldType {
    pub dimensions: u8,
    pub element: ElementType,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
    Reference(String),
}
