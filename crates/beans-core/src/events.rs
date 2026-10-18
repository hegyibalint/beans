use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    JavaHomeFound(JavaHomeFound),
    Stop,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JavaHomeFound {
    pub home: PathBuf,
}
