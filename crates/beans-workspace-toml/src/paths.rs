use std::path::{Component, Path, PathBuf};

use url::Url;

/// Normalize absolute paths without requiring declared inputs to exist.
/// This is lexical identity, not filesystem canonicalization through symlinks.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    debug_assert!(path.is_absolute());
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

pub(crate) fn document_path(uri: &str) -> Option<PathBuf> {
    // Url::to_file_path does not check the scheme itself.
    let uri = Url::parse(uri).ok().filter(|uri| uri.scheme() == "file")?;
    let path = uri.to_file_path().ok()?;
    Some(normalize(&path))
}
