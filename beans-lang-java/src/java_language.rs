use std::path::Path;

use beans_core::language::Language;
use beans_core::resolve::Import;
use beans_core::Symbol;

use crate::parse_java_file;

/// Java language implementation.
pub struct JavaLanguage;

impl Language for JavaLanguage {
    fn extensions(&self) -> &[&str] {
        &["java"]
    }

    fn parse(&self, path: &Path, source: &str) -> Vec<Symbol> {
        parse_java_file(path, source)
    }

    fn extract_imports(&self, source: &str) -> Vec<Import> {
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

    fn extract_package(&self, source: &str) -> String {
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("package ") {
                return rest.trim_end_matches(';').trim().to_string();
            }
        }
        String::new()
    }

    fn word_at_position(&self, source: &str, line: u32, col: u32) -> Option<String> {
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
}

fn is_java_identifier_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
