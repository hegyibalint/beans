use std::path::Path;

use crate::resolve::Import;
use crate::Symbol;

/// Trait for language-specific operations.
/// Each JVM language (Java, Kotlin, Scala, etc.) implements this trait.
/// The test harness and LSP use it to dispatch per-file operations.
pub trait Language: Send + Sync {
    /// File extensions this language handles (e.g., &["java"] or &["kt", "kts"])
    fn extensions(&self) -> &[&str];

    /// Parse source into symbols
    fn parse(&self, path: &Path, source: &str) -> Vec<Symbol>;

    /// Extract import statements from source
    fn extract_imports(&self, source: &str) -> Vec<Import>;

    /// Extract package/namespace declaration from source
    fn extract_package(&self, source: &str) -> String;

    /// Extract the identifier at a given position (language-specific identifier rules)
    fn word_at_position(&self, source: &str, line: u32, col: u32) -> Option<String>;
}
