use std::path::PathBuf;

#[derive(Debug)]
pub enum Event {
    JavaHomeFound(JavaHomeFound),
    Stop,
}

#[derive(Debug)]
pub struct JavaHomeFound {
    pub home: PathBuf,
}
