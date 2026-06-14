//! Java-local syntactic helpers — imports, package, identifier extraction.
//!
//! These were previously methods on a `JavaLanguage` trait object. Per
//! ADR-0021 the trait dispatch goes; in its place each language module
//! exposes plain functions that the fixture harness and the LSP dispatch
//! to via a `match ext { "java" => ..., "kt" => ..., ... }` at the
//! consumer's edge. No central registry.

use std::path::Path;

use beans_core::primitives::Location;

/// One Java `import` statement, paired with its source location.
///
/// Lives here rather than in a generic `Import` because Java's import
/// shapes (single, wildcard, static) are language-specific. Other JVM
/// languages (Kotlin, Scala) have different import syntax and will
/// surface their own `Import` shapes when their parsers land.
///
/// Per ADR-0029 each variant carries a [`Location`] so diagnostics —
/// notably `unused-import` — can squiggle the offending statement.
/// The location spans the whole `import …;` line.
#[derive(Debug, Clone, PartialEq)]
pub enum Import {
    /// `import com.example.MyClass;`
    Single(String, Location),
    /// `import com.example.*;`
    Wildcard(String, Location),
    /// `import static com.example.Utils.MAX;`
    Static(String, Location),
}

impl Import {
    /// Source location of the whole import statement. Used by
    /// diagnostic rules to anchor squiggles.
    pub fn location(&self) -> &Location {
        match self {
            Import::Single(_, loc) | Import::Wildcard(_, loc) | Import::Static(_, loc) => loc,
        }
    }
}

/// Extract `import` statements from a Java source. Recognises:
/// - `import com.example.Foo;` → [`Import::Single`]
/// - `import com.example.*;` → [`Import::Wildcard`]
/// - `import static com.example.Utils.MAX;` → [`Import::Static`]
///
/// Line-based, not parser-based — robust to malformed surrounding code.
/// Each returned `Import` carries a [`Location`] spanning its line so
/// diagnostic rules can squiggle the right place.
///
/// Takes the file's shared `Arc<Path>` (minted once in
/// `parse_java_to_graph`) so import locations point at the *same* path
/// buffer as the file's declaration locations — one buffer per file,
/// not one per producer (backlog #037).
pub fn extract_imports(file: &std::sync::Arc<Path>, source: &str) -> Vec<Import> {
    let mut imports = Vec::new();
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("import ") else {
            continue;
        };
        let rest = rest.trim_end_matches(';').trim();

        // The "whole import statement" span: from the leading `import`
        // keyword to the trailing `;` (or end-of-line if `;` missing).
        // Computed in code-unit (byte) columns to match what tree-sitter
        // produces elsewhere; ASCII-only Java import statements make
        // this safe for the cases we care about.
        let start_col = line.find("import").unwrap_or(0) as u32;
        let end_col = line
            .rfind(';')
            .map(|i| (i + 1) as u32)
            .unwrap_or(line.len() as u32);
        let location = Location {
            file: std::sync::Arc::clone(file),
            start_line: line_idx as u32,
            start_col,
            end_line: line_idx as u32,
            end_col,
        };

        if let Some(fqn) = rest.strip_prefix("static ") {
            imports.push(Import::Static(fqn.trim().to_string(), location));
        } else if rest.ends_with(".*") {
            let package = rest.trim_end_matches(".*");
            imports.push(Import::Wildcard(package.to_string(), location));
        } else {
            imports.push(Import::Single(rest.to_string(), location));
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
