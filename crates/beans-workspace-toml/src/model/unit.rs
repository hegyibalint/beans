use std::path::PathBuf;

/// One compilation scope, such as a project's main or test source set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// Source directories, including files added after the descriptor was loaded.
    pub sources: Vec<PathBuf>,
    /// Direct dependencies: their sources are visible, but their dependencies are not.
    pub depends_on: Vec<String>,
    /// Compiled inputs: directories, standalone class files, or archives.
    pub classpath: Vec<PathBuf>,
    /// The unit's JDK, overriding the descriptor default when provided.
    pub jdk_home: Option<PathBuf>,
}
