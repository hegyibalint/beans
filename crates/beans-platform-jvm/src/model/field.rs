use serde::{Deserialize, Serialize};

use crate::{FieldAccessFlags, FieldType};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(with = "super::access_flags::field")]
    pub access_flags: FieldAccessFlags,
    pub field_type: FieldType,
}
