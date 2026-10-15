use std::{
    io,
    path::{Path, PathBuf},
};

pub(super) fn artifacts(home: &Path) -> Result<Vec<PathBuf>, String> {
    for (required, optional) in [
        ("lib/modules", None),
        ("jre/lib/rt.jar", Some("lib/tools.jar")),
        ("lib/rt.jar", Some("lib/tools.jar")),
    ] {
        let runtime = home.join(required);
        if !artifact_exists(&runtime)? {
            continue;
        }

        let mut artifacts = vec![runtime];
        if let Some(relative) = optional {
            let tools = home.join(relative);
            if artifact_exists(&tools)? {
                artifacts.push(tools);
            }
        }
        return Ok(artifacts);
    }
    Err(format!(
        "{}: no supported JDK runtime found",
        home.display()
    ))
}

fn artifact_exists(path: &Path) -> Result<bool, String> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(format!("{}: expected a JDK artifact file", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

#[cfg(test)]
mod tests;
