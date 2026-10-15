use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// Source directories, including files added after the descriptor was loaded.
    pub sources: Vec<PathBuf>,
    /// Dependencies expose their sources, but not their dependencies.
    pub depends_on: Vec<String>,
}
