use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Class {
    pub name: String,
}
