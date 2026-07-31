use std::fs;
use std::path::{Path, PathBuf};

use beans_platform_jvm::model::JvmSource;
use beans_platform_jvm::query::{JvmContainer, JvmScope};
use beans_workspace::model::{Selector, Unit, Workspace};

/// The dependency graph, flattened into one scope per unit and then forgotten.
/// This is the only place edges are followed; nothing downstream ever sees one.
#[derive(Default)]
pub(crate) struct Scopes {
    units: Vec<ScopedUnit>,
}

struct ScopedUnit {
    /// The trees the unit declares as its own, which is what decides whether a
    /// file belongs to it. Narrower than `scope`, which also holds everything
    /// the unit may look at.
    owns: Vec<PathBuf>,
    scope: JvmScope,
}

impl Scopes {
    pub(crate) fn of(workspace: &Workspace) -> Scopes {
        Scopes {
            units: workspace
                .units
                .iter()
                .map(|unit| ScopedUnit {
                    owns: owned_trees(unit),
                    scope: JvmScope::of(visible_containers(unit, workspace)),
                })
                .collect(),
        }
    }

    /// Every scope that claims `source`. Two units declaring one tree means a
    /// file under it is compiled by both, so it belongs to both.
    ///
    /// Empty is not "sees nothing": a caller must leave such a source
    /// unregistered so it stays unscoped.
    pub(crate) fn of_source(&self, source: &JvmSource) -> Vec<JvmScope> {
        let JvmSource::SourceFile { path } = source else {
            // Only hand written sources are placed by a workspace. A jar entry
            // is reached through a unit's classpath, never owned by one.
            return Vec::new();
        };

        self.units
            .iter()
            .filter(|unit| unit.owns.iter().any(|base| path.starts_with(base)))
            .map(|unit| unit.scope.clone())
            .collect()
    }
}

fn owned_trees(unit: &Unit) -> Vec<PathBuf> {
    unit.sources
        .iter()
        .flat_map(|selector| match selector {
            Selector::Tree { base, .. } => vec![base.clone()],
            // A path is a prefix of itself, so a listed file is a tree of one.
            Selector::Files { files, .. } => files.clone(),
        })
        .collect()
}

/// What a unit may look at: its own sources, whatever it links against, and
/// the sources of the units it names.
///
/// Direct edges only. Whether `depends_on` chains is a question about what a
/// descriptor means, and it belongs to the workspace layer rather than here;
/// with `app -> lib -> core` this gives `app` the sources of `lib` and not
/// those of `core`.
fn visible_containers(unit: &Unit, workspace: &Workspace) -> Vec<JvmContainer> {
    let mut containers: Vec<JvmContainer> = owned_trees(unit)
        .into_iter()
        .map(JvmContainer::Source)
        .collect();

    containers.extend(unit.classpath.iter().cloned().map(JvmContainer::Artifact));

    // A JDK is one image, so this says "the whole runtime is visible", which
    // JPMS says is not true. Splitting it needs the lake to hold modules.
    if let Some(jdk_home) = &unit.jdk_home {
        containers.push(JvmContainer::Artifact(jdk_home.join("lib").join("modules")));
    }

    for dependency in &unit.depends_on {
        let Some(dependency) = workspace.units.iter().find(|u| u.id == *dependency) else {
            // An id naming no unit is a descriptor typo. Nothing validates
            // that yet, and dropping it beats failing the whole import.
            continue;
        };
        containers.extend(
            owned_trees(dependency)
                .into_iter()
                .map(JvmContainer::Source),
        );
    }

    containers
}

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
