use serde::{Deserialize, Serialize};

use crate::{FieldType, MethodAccessFlags};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Method {
    /// The JVM name, including `<init>` and `<clinit>` (JVMS §4.2.2).
    pub name: String,
    #[serde(with = "super::access_flags::method")]
    pub access_flags: MethodAccessFlags,
    pub parameters: Vec<FieldType>,
    /// `None` represents the void return descriptor.
    pub return_type: Option<FieldType>,
}
