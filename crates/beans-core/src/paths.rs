use std::path::{Component, Path, PathBuf};

/// Normalize absolute paths without requiring declared inputs to exist.
/// This is lexical identity, not filesystem canonicalization through symlinks.
pub fn normalize(path: &Path) -> PathBuf {
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
