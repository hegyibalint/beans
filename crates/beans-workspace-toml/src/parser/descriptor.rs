use std::{fs, io, path::Path};

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
/// A nonempty override replaces `jdk_home` for every unit. Relative override
/// paths are resolved against the workspace root.
pub fn load(root: &Path, jdk_override: Option<&Path>) -> Result<Option<TomlWorkspace>, LoadError> {
    let path = root.join(DESCRIPTOR);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            log::debug!("No descriptor at {}", path.display());
            return Ok(None);
        }
        Err(error) => return Err(LoadError::Read { path, error }),
    };
    let mut project = parse_project(&contents, root).map_err(|error| LoadError::Parse {
        path: path.clone(),
        error,
    })?;
    if let Some(home) = jdk_override.filter(|path| !path.as_os_str().is_empty()) {
        let home = normalize(&project.root.join(home));
        for unit in project.units.values_mut() {
            unit.jdk_home = Some(home.clone());
        }
        log::info!(
            "JDK override active for {}: jdk_home={}",
            path.display(),
            home.display()
        );
    }
    log::info!("Loaded {}: units={}", path.display(), project.units.len());
    Ok(Some(TomlWorkspace::new(project)))
}

/// Parse the v0.2 descriptor format and resolve paths against `root`.
/// A relative root is made absolute using the current directory; inputs need
/// not exist. Unknown fields and dependencies naming absent units are errors.
pub fn parse(contents: &str, root: &Path) -> Result<TomlWorkspace, ParseError> {
    parse_project(contents, root).map(TomlWorkspace::new)
}

fn parse_project(contents: &str, root: &Path) -> Result<Project, ParseError> {
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
                jdk_home: unit
                    .jdk_home
                    .as_deref()
                    .or(jdk_home.as_deref())
                    .map(resolve),
            };
            (id, unit)
        })
        .collect();
    Ok(Project { root, units })
}
