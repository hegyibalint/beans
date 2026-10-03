use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    engine::TomlWorkspace,
    model::{Descriptor, Project, Unit},
    paths::normalize,
};

use super::{LoadError, ParseError};

/// The descriptor filename, relative to the workspace root.
pub const DESCRIPTOR: &str = "beans.toml";

/// Read a workspace's descriptor. Absence is not an error; a present but
/// unreadable or invalid descriptor is never silently replaced with a fallback.
/// A nonempty `BEANS_WORKSPACE_TOML_JAVA_HOME` overrides `jdk_home` for every
/// unit. Relative override paths are resolved against the workspace root.
pub fn load(root: &Path) -> Result<Option<TomlWorkspace>, LoadError> {
    let path = root.join(DESCRIPTOR);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            log::debug!("No descriptor at {}", path.display());
            return Ok(None);
        }
        Err(error) => return Err(LoadError::Read { path, error }),
    };
    let jdk_override = std::env::var_os("BEANS_WORKSPACE_TOML_JAVA_HOME").map(PathBuf::from);
    let workspace =
        parse_with_jdk_override(&contents, root, jdk_override.as_deref()).map_err(|error| {
            LoadError::Parse {
                path: path.clone(),
                error,
            }
        })?;
    if let Some(home) = jdk_override.filter(|path| !path.as_os_str().is_empty()) {
        log::info!(
            "BEANS_WORKSPACE_TOML_JAVA_HOME override active for {}: jdk_home={}",
            path.display(),
            normalize(&workspace.model().root.join(home)).display()
        );
    }
    log::info!(
        "Loaded {}: units={}",
        path.display(),
        workspace.model().units.len()
    );
    Ok(Some(workspace))
}

/// Parse the v0.2 descriptor format and resolve paths against `root`.
/// A relative root is made absolute using the current directory; inputs need
/// not exist. Unknown fields and dependencies naming absent units are errors.
/// Unlike [`load`], this does not read environment overrides.
pub fn parse(contents: &str, root: &Path) -> Result<TomlWorkspace, ParseError> {
    parse_with_jdk_override(contents, root, None)
}

pub(super) fn parse_with_jdk_override(
    contents: &str,
    root: &Path,
    jdk_override: Option<&Path>,
) -> Result<TomlWorkspace, ParseError> {
    let jdk_override = jdk_override.filter(|path| !path.as_os_str().is_empty());
    let Descriptor { jdk_home, unit } = toml::from_str(contents).map_err(ParseError::Toml)?;
    for (id, descriptor) in &unit {
        for dependency in &descriptor.depends_on {
            if !unit.contains_key(dependency) {
                return Err(ParseError::UnknownDependency {
                    unit: id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    let root = normalize(
        &std::path::absolute(root).map_err(|error| ParseError::Root {
            path: root.to_path_buf(),
            error,
        })?,
    );
    let resolve = |path: &Path| normalize(&root.join(path));
    let units = unit
        .into_iter()
        .map(|(id, unit)| {
            let unit = Unit {
                sources: unit.sources.iter().map(|path| resolve(path)).collect(),
                depends_on: unit.depends_on,
                classpath: unit.classpath.iter().map(|path| resolve(path)).collect(),
                jdk_home: jdk_override
                    .or(unit.jdk_home.as_deref())
                    .or(jdk_home.as_deref())
                    .map(resolve),
            };
            (id, unit)
        })
        .collect();
    Ok(TomlWorkspace::new(Project { root, units }))
}
