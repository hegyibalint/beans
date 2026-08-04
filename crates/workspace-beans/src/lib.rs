use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use beans_workspace::model::Workspace;

use crate::model::Descriptor;

mod model;

/// The file this producer reads, at the workspace root.
pub const DESCRIPTOR: &str = "beans.toml";

pub const TOOL: &str = "beans";

/// Read `beans.toml` from `root`, if there is one.
///
/// `Ok(None)` is a workspace that never asked for one, which is the common
/// case and not a problem. A descriptor that exists and cannot be read or
/// parsed is an error: we were told to follow it and cannot.
pub fn load(root: &Path) -> Result<Option<Workspace>, LoadError> {
    let path = root.join(DESCRIPTOR);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(LoadError::Read { path, error }),
    };

    parse(&contents, root)
        .map(Some)
        .map_err(|error| LoadError::Parse { path, error })
}

/// The descriptor spells paths relative to the root, because a file a person
/// writes and commits cannot hold absolute ones. Resolving them here is what
/// lets `Workspace` promise its consumers that every path is absolute.
fn parse(contents: &str, root: &Path) -> Result<Workspace, toml::de::Error> {
    let descriptor: Descriptor = toml::from_str(contents)?;

    Ok(Workspace {
        tool: TOOL.to_string(),
        units: descriptor
            .unit
            .into_iter()
            .map(|(id, unit)| unit.resolve(id, root))
            .collect(),
    })
}

#[derive(Debug)]
pub enum LoadError {
    Read {
        path: PathBuf,
        error: io::Error,
    },
    Parse {
        path: PathBuf,
        error: toml::de::Error,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Read { path, error } => write!(f, "cannot read {}: {error}", path.display()),
            LoadError::Parse { path, error } => {
                write!(f, "cannot parse {}: {error}", path.display())
            }
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Read { error, .. } => Some(error),
            LoadError::Parse { error, .. } => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::JAVA_SOURCES;
    use beans_workspace::model::Selector;

    fn workspace(contents: &str) -> Workspace {
        parse(contents, Path::new("/project")).expect("the descriptor parses")
    }

    #[test]
    fn a_unit_is_named_by_its_key() {
        let workspace = workspace("[unit.lib]\n[unit.app]\n");

        let ids: Vec<&str> = workspace
            .units
            .iter()
            .map(|unit| unit.id.as_str())
            .collect();
        assert_eq!(ids, ["app", "lib"]);
    }

    #[test]
    fn a_source_is_a_tree_of_java_files_under_the_root() {
        let workspace = workspace("[unit.lib]\nsources = [\"lib/src\"]\n");

        assert_eq!(
            workspace.units[0].sources,
            [Selector::Tree {
                base: PathBuf::from("/project/lib/src"),
                includes: vec![JAVA_SOURCES.to_string()],
                excludes: Vec::new(),
                generated: false,
            }]
        );
    }

    #[test]
    fn a_unit_carries_the_units_it_depends_on() {
        let workspace = workspace("[unit.app]\ndepends_on = [\"lib\"]\n\n[unit.lib]\n");

        assert_eq!(workspace.units[0].depends_on, ["lib"]);
        assert!(workspace.units[1].depends_on.is_empty());
    }

    #[test]
    fn classpath_elements_are_resolved_from_the_workspace_root() {
        let workspace =
            workspace("[unit.app]\nclasspath = [\"lib/Feature.class\", \"lib/dependency.jar\"]\n");

        assert_eq!(
            workspace.units[0].classpath,
            [
                PathBuf::from("/project/lib/Feature.class"),
                PathBuf::from("/project/lib/dependency.jar"),
            ]
        );
    }

    #[test]
    fn a_descriptor_declaring_nothing_is_an_empty_workspace() {
        assert!(workspace("").units.is_empty());
    }

    // A typo in a file nobody validates is a silent wrong answer, so the parse
    // refuses rather than ignoring what it does not know.
    #[test]
    fn an_unknown_field_is_rejected() {
        assert!(
            parse(
                "[unit.lib]\nsauces = [\"lib/src\"]\n",
                Path::new("/project")
            )
            .is_err()
        );
    }

    #[test]
    fn a_root_without_a_descriptor_has_no_workspace() {
        let without = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

        assert_eq!(load(&without).expect("absence is not an error"), None);
    }
}
