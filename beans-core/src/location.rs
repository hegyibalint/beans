use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub file: PathBuf,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}
