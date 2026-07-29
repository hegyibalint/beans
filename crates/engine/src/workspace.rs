use std::fs;
use std::path::{Path, PathBuf};

use beans_workspace::model::{Selector, Workspace};

/// Every Java source a workspace declares, in the order its units and
/// selectors name them, and sorted within a directory so the same tree always
/// loads the same way.
///
/// Only the `.java` extension is honoured, not `includes` or `excludes`. The
/// only pattern anything emits today is `**/*.java`; a build tool import
/// producing real ones needs a matcher here rather than this shortcut.
pub(crate) fn java_sources(workspace: &Workspace) -> Vec<PathBuf> {
    let mut sources = Vec::new();

    for unit in &workspace.units {
        for selector in &unit.sources {
            match selector {
                Selector::Tree { base, .. } => collect(base, &mut sources),
                Selector::Files { files, .. } => {
                    sources.extend(files.iter().filter(|path| is_java(path)).cloned());
                }
            }
        }
    }

    sources
}

/// A declared root that does not exist is not an error. Build tools name
/// source sets that were never created, and a missing directory contributes
/// nothing rather than failing the whole import.
fn collect(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect(&path, into);
        } else if is_java(&path) {
            into.push(path);
        }
    }
}

fn is_java(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "java")
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_workspace::model::Unit;

    fn unit(sources: Vec<Selector>) -> Unit {
        Unit {
            id: "unit".to_string(),
            sources,
            depends_on: Vec::new(),
            classpath: Vec::new(),
            jdk_home: None,
        }
    }

    fn workspace(units: Vec<Unit>) -> Workspace {
        Workspace {
            tool: "test".to_string(),
            units,
        }
    }

    /// This crate's own tree stands in for a project: it holds nested
    /// directories and not one `.java` file.
    fn rust_crate_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    #[test]
    fn a_tree_that_does_not_exist_contributes_nothing() {
        let workspace = workspace(vec![unit(vec![Selector::Tree {
            base: rust_crate_root().join("no-such-directory"),
            includes: vec!["**/*.java".to_string()],
            excludes: Vec::new(),
            generated: false,
        }])]);

        assert!(java_sources(&workspace).is_empty());
    }

    #[test]
    fn a_tree_contributes_only_java_files() {
        let workspace = workspace(vec![unit(vec![Selector::Tree {
            base: rust_crate_root(),
            includes: vec!["**/*.java".to_string()],
            excludes: Vec::new(),
            generated: false,
        }])]);

        assert!(java_sources(&workspace).is_empty(), "this tree is all Rust");
    }

    #[test]
    fn listed_files_are_filtered_by_extension() {
        let workspace = workspace(vec![unit(vec![Selector::Files {
            files: vec![PathBuf::from("A.java"), PathBuf::from("build.gradle.kts")],
            generated: false,
        }])]);

        assert_eq!(java_sources(&workspace), [PathBuf::from("A.java")]);
    }
}
