mod container;
mod model;
mod parser;
pub mod query;
pub mod storage;

#[cfg(test)]
mod tests;

use std::{
    io,
    path::{Path, PathBuf},
};

use beans_core::location::Location;
use rusqlite::Transaction;

pub use crate::model::class::Class;

#[derive(Default)]
pub struct JvmPlatform;

impl JvmPlatform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(&self, path: &Path) -> Result<Vec<(Location, Class)>, String> {
        log::debug!("Processing JVM resource {}", path.display());
        let mut file =
            std::fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let resource_id = Location::file(path);

        if let Some(entries) = container::class::process(&resource_id, &mut file)? {
            return collect(path, entries);
        }
        if let Some(entries) = container::jar::process(&resource_id, &mut file)? {
            return collect(path, entries);
        }
        if let Some(entries) = container::jimage::process(&resource_id, &mut file)? {
            return collect(path, entries);
        }

        Ok(Vec::new())
    }

    /// Replaces a resource; the caller must roll back the transaction on error.
    pub fn store(
        &self,
        transaction: &Transaction<'_>,
        path: &Path,
        classes: Vec<(Location, Class)>,
    ) -> Result<(), String> {
        storage::remove(transaction, path)?;
        for (location, class) in classes {
            storage::store(transaction, &class, &location)?;
        }
        Ok(())
    }
}

/// Selects the first supported layout, including tools.jar only for legacy homes.
pub fn jdk_artifacts(home: &Path) -> Result<Vec<PathBuf>, String> {
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

fn collect(
    path: &Path,
    entries: impl Iterator<Item = Result<(Location, Class), String>>,
) -> Result<Vec<(Location, Class)>, String> {
    let mut classes = Vec::new();
    for entry in entries {
        classes.push(entry?);
        if classes.len() % 1000 == 0 {
            log::debug!("Parsed {} JVM classes from {}", classes.len(), path.display());
        }
    }
    log::debug!(
        "Finished parsing {} JVM classes from {}",
        classes.len(),
        path.display()
    );
    Ok(classes)
}

fn artifact_exists(path: &Path) -> Result<bool, String> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(format!("{}: expected a JDK artifact file", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}
