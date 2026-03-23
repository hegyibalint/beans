use std::path::{Path, PathBuf};

use beans_core::SymbolTable;
use beans_lang_java::parse_java_file;
use walkdir::WalkDir;

use crate::resolve::Import;

/// Scan workspace for .java files, skipping hidden and build directories.
pub fn scan_workspace(root: &Path) -> Vec<PathBuf> {
    let skip_dirs = ["target", "build", "out", "bin", ".git", ".gradle", ".idea", "node_modules"];

    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            if entry.file_type().is_dir() {
                !name.starts_with('.') && !skip_dirs.contains(&name.as_ref())
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .is_some_and(|ext| ext == "java")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Parse all .java files in the workspace and insert symbols into the table.
/// Returns a map of file -> imports for name resolution.
pub fn index_workspace(
    root: &Path,
    table: &mut SymbolTable,
) -> std::collections::HashMap<PathBuf, Vec<Import>> {
    let files = scan_workspace(root);
    let mut file_imports = std::collections::HashMap::new();

    for file in &files {
        index_file(file, table, &mut file_imports);
    }

    file_imports
}

/// Parse a single file and insert its symbols into the table.
pub fn index_file(
    file: &Path,
    table: &mut SymbolTable,
    file_imports: &mut std::collections::HashMap<PathBuf, Vec<Import>>,
) {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(_) => return,
    };

    index_file_with_content(file, &source, table, file_imports);
}

/// Parse a file from in-memory content and insert its symbols into the table.
pub fn index_file_with_content(
    file: &Path,
    source: &str,
    table: &mut SymbolTable,
    file_imports: &mut std::collections::HashMap<PathBuf, Vec<Import>>,
) {
    // Remove old symbols for this file
    table.remove_by_file(file);

    let symbols = parse_java_file(file, source);
    table.insert_parsed_symbols(symbols);

    // Extract imports
    let imports = extract_imports(source);
    file_imports.insert(file.to_path_buf(), imports);
}

/// Extract import statements from Java source.
/// This is a simple line-based parser — no need for tree-sitter here.
fn extract_imports(source: &str) -> Vec<Import> {
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

/// Extract the package declaration from Java source.
pub fn extract_package(source: &str) -> String {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("package ") {
            return rest.trim_end_matches(';').trim().to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_imports_single() {
        let source = "import com.example.MyClass;\nimport java.util.List;\n";
        let imports = extract_imports(source);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0], Import::Single("com.example.MyClass".to_string()));
        assert_eq!(imports[1], Import::Single("java.util.List".to_string()));
    }

    #[test]
    fn test_extract_imports_wildcard() {
        let source = "import java.util.*;\n";
        let imports = extract_imports(source);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0], Import::Wildcard("java.util".to_string()));
    }

    #[test]
    fn test_extract_imports_static() {
        let source = "import static com.example.Utils.MAX;\n";
        let imports = extract_imports(source);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0], Import::Static("com.example.Utils.MAX".to_string()));
    }

    #[test]
    fn test_extract_package() {
        assert_eq!(extract_package("package com.example;\n"), "com.example");
        assert_eq!(extract_package("// no package\npublic class Foo {}"), "");
    }

    #[test]
    fn test_scan_workspace_skips_hidden_dirs() {
        use std::fs;
        let tmp = std::env::temp_dir().join("beans_test_scan");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".hidden")).unwrap();
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(tmp.join(".hidden/Hidden.java"), "class Hidden {}").unwrap();
        fs::write(tmp.join("src/Visible.java"), "class Visible {}").unwrap();

        let files = scan_workspace(&tmp);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("Visible.java"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_index_file_with_content() {
        let mut table = SymbolTable::new();
        let mut imports = std::collections::HashMap::new();
        let source = "package com.test;\nimport java.util.List;\npublic class Foo { int x; }";
        let path = Path::new("Foo.java");

        index_file_with_content(path, source, &mut table, &mut imports);

        // Should have symbols for Foo and x
        assert!(table.lookup_by_fqn("com.test.Foo").is_some());
        assert!(table.lookup_by_fqn("com.test.Foo.x").is_some());

        // Should have extracted imports
        let file_imports = imports.get(path).unwrap();
        assert_eq!(file_imports.len(), 1);
        assert_eq!(file_imports[0], Import::Single("java.util.List".to_string()));
    }

    #[test]
    fn test_reindex_file_replaces_old_symbols() {
        let mut table = SymbolTable::new();
        let mut imports = std::collections::HashMap::new();
        let path = Path::new("Foo.java");

        // First index
        index_file_with_content(path, "package com.test;\npublic class Foo { int x; }", &mut table, &mut imports);
        assert!(table.lookup_by_fqn("com.test.Foo.x").is_some());

        // Re-index with different content (field removed, method added)
        index_file_with_content(path, "package com.test;\npublic class Foo { void doWork() {} }", &mut table, &mut imports);
        assert!(table.lookup_by_fqn("com.test.Foo.x").is_none());
        assert!(table.lookup_by_fqn("com.test.Foo.doWork").is_some());
    }
}
