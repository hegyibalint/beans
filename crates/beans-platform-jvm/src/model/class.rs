use serde::{Deserialize, Serialize};

/// Stored as JSON in `models.model` for JVM rows (`model_kind = 0`).
#[derive(Debug, Serialize, Deserialize)]
pub struct Class {
    pub name: String,
}
