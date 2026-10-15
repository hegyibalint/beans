use std::{path::PathBuf, process::Command};

use serde::Deserialize;

#[derive(Deserialize)]
struct Installation {
    version: String,
    install_path: PathBuf,
}

pub fn list(version_prefix: &str) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("mise")
        .args(["ls", "java", "--installed", "--json"])
        .output()
        .map_err(|error| format!("failed to run mise: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "mise ls java failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let installations: Vec<Installation> = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid mise JDK list: {error}"))?;
    Ok(installations
        .into_iter()
        .filter(|installation| installation.version.starts_with(version_prefix))
        .map(|installation| installation.install_path)
        .collect())
}
