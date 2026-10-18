use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    JavaHomeFound(JavaHomeFound),
    Stop,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JavaHomeFound {
    pub home: PathBuf,
}
