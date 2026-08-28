use std::fs;
use std::path::{Path, PathBuf};

use beans_platform_jvm as jvm;
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
    scope: jvm::query::Scope,
}

impl Scopes {
    pub(crate) fn of(workspace: &Workspace) -> Scopes {
        Scopes {
            units: workspace
                .units
                .iter()
                .map(|unit| ScopedUnit {
                    owns: owned_trees(unit),
                    scope: jvm::query::Scope::of(visible_containers(unit, workspace)),
                })
                .collect(),
        }
    }

    /// Every scope that claims `source`. Two units declaring one tree means a
    /// file under it is compiled by both, so it belongs to both.
    ///
    /// Empty is not "sees nothing": a caller must leave such a source
    /// unregistered so it stays unscoped.
    pub(crate) fn of_source(&self, source: &jvm::model::Source) -> Vec<jvm::query::Scope> {
        let jvm::model::Source::SourceFile { path } = source else {
            // Only hand written sources are placed by a workspace. Compiled
            // inputs are reached through a unit's classpath, never owned by one.
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
fn visible_containers(unit: &Unit, workspace: &Workspace) -> Vec<jvm::query::Container> {
    let mut containers: Vec<jvm::query::Container> = owned_trees(unit)
        .into_iter()
        .map(jvm::query::Container::Source)
        .collect();

    containers.extend(
        unit.classpath
            .iter()
            .cloned()
            .map(jvm::query::Container::Artifact),
    );

    // A JDK is one image, so this says "the whole runtime is visible", which
    // JPMS says is not true. Splitting it needs the lake to hold modules.
    containers.extend(runtime_image(unit).map(jvm::query::Container::Artifact));

    for dependency in &unit.depends_on {
        let Some(dependency) = workspace.units.iter().find(|u| u.id == *dependency) else {
            // An id naming no unit is a descriptor typo. Nothing validates
            // that yet, and dropping it beats failing the whole import.
            continue;
        };
        containers.extend(
            owned_trees(dependency)
                .into_iter()
                .map(jvm::query::Container::Source),
        );
    }

    containers
}

/// The one file a JDK contributes. A `jdk_home` is a setting of its own rather
/// than a classpath element because a runtime is not a dependency: a project
/// has exactly one, it is what every other name is resolved against, and
/// `javac` takes it as `--release` or `--system` rather than as `-cp`.
fn runtime_image(unit: &Unit) -> Option<PathBuf> {
    unit.jdk_home
        .as_ref()
        .map(|home| home.join("lib").join("modules"))
}

/// Every compiled input a workspace declares, in unit and classpath order,
/// each named once.
///
/// A JDK is separate at the descriptor and one file here, because from this
/// point on it is read exactly like a jar. Deduplicated because four units
/// sharing one runtime is the normal case, and reading it four times is 27,000
/// classes three times over.
pub(crate) fn compiled_inputs(workspace: &Workspace) -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = Vec::new();
    for unit in &workspace.units {
        for path in unit.classpath.iter().cloned().chain(runtime_image(unit)) {
            if !inputs.contains(&path) {
                inputs.push(path);
            }
        }
    }
    inputs
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
    // A selector may name one file rather than a tree, which `owned_trees`
    // already accounts for on the scoping side: a path is a prefix of itself,
    // so such a file belongs to the unit that named it. Reading has to agree,
    // or the file is in scope and never loaded.
    if directory.is_file() {
        if is_java(directory) {
            into.push(directory.to_path_buf());
        }
        return;
    }

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
mod tests;
