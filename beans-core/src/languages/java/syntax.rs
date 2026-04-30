//! Java-local syntactic helpers — imports, package, identifier extraction.
//!
//! These were previously methods on a `JavaLanguage` trait object. Per
//! ADR-0021 the trait dispatch goes; in its place each language module
//! exposes plain functions that the fixture harness and the LSP dispatch
//! to via a `match ext { "java" => ..., "kt" => ..., ... }` at the
//! consumer's edge. No central registry.

/// One Java `import` statement.
///
/// Lives here rather than in a generic `Import` because Java's import
/// shapes (single, wildcard, static) are language-specific. Other JVM
/// languages (Kotlin, Scala) have different import syntax and will
/// surface their own `Import` shapes when their parsers land.
#[derive(Debug, Clone, PartialEq)]
pub enum Import {
    /// `import com.example.MyClass;`
    Single(String),
    /// `import com.example.*;`
    Wildcard(String),
    /// `import static com.example.Utils.MAX;`
    Static(String),
}

/// Extract `import` statements from a Java source. Recognises:
/// - `import com.example.Foo;` → [`Import::Single`]
/// - `import com.example.*;` → [`Import::Wildcard`]
/// - `import static com.example.Util.MAX;` → [`Import::Static`]
///
/// Line-based, not parser-based — robust to malformed surrounding code,
/// matches the pre-migration implementation.
pub fn extract_imports(source: &str) -> Vec<Import> {
    let mut imports = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest.trim_end_matches(';').trim();
            if let Some(fqn) = rest.strip_prefix("static ") {
                imports.push(Import::Static(fqn.trim().to_string()));
            } else if rest.ends_with(".*") {
                let package = rest.trim_end_matches(".*");
                imports.push(Import::Wildcard(package.to_string()));
            } else {
                imports.push(Import::Single(rest.to_string()));
            }
        }
    }
    imports
}

/// Extract the `package` declaration from a Java source. Empty string
/// when the file is in the default (unnamed) package.
pub fn extract_package(source: &str) -> String {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("package ") {
            return rest.trim_end_matches(';').trim().to_string();
        }
    }
    String::new()
}

/// Extract the Java identifier at a given (line, column) position. Used
/// by go-to-definition and hover to figure out what word the cursor sits
/// on. `None` if the position is past the line end or not on an
/// identifier character.
pub fn word_at_position(source: &str, line: u32, col: u32) -> Option<String> {
    let target_line = source.lines().nth(line as usize)?;
    let col = col as usize;
    if col > target_line.len() {
        return None;
    }
    let bytes = target_line.as_bytes();
    let mut start = col;
    while start > 0 && is_java_identifier_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && is_java_identifier_char(bytes[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    Some(target_line[start..end].to_string())
}

fn is_java_identifier_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
