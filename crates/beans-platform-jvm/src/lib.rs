mod container;
mod model;
mod parser;
pub mod query;
pub mod storage;

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

    /// Recognized resources report classes and failures; unsupported formats return an error.
    pub fn process(&self, path: &Path) -> Result<ProcessedResource, String> {
        log::debug!("Processing JVM resource {}", path.display());
        let mut file =
            std::fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let resource_id = Location::file(path);

        if let Some(processed) = container::class::process(&resource_id, &mut file)? {
            return Ok(processed);
        }
        if let Some(processed) = container::jar::process(&resource_id, &mut file)? {
            return Ok(processed);
        }
        if let Some(processed) = container::jimage::process(&resource_id, &mut file)? {
            return Ok(processed);
        }

        Err(format!(
            "{}: unsupported JVM resource format",
            path.display()
        ))
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

#[derive(Debug, Default)]
pub struct ProcessedResource {
    pub classes: Vec<(Location, Class)>,
    pub failures: Vec<(Location, String)>,
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

fn artifact_exists(path: &Path) -> Result<bool, String> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(format!("{}: expected a JDK artifact file", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}
