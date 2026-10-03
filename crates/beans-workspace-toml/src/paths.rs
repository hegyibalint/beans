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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_and_parent_components_are_collapsed_lexically() {
        assert_eq!(
            normalize(Path::new("/project/./app/../src")),
            Path::new("/project/src")
        );
    }

    #[test]
    fn parent_components_cannot_escape_the_filesystem_root() {
        assert_eq!(
            normalize(Path::new("/project/../../src")),
            Path::new("/src")
        );
        assert_eq!(normalize(Path::new("/../..")), Path::new("/"));
    }

    #[test]
    fn non_file_uris_cannot_masquerade_as_local_paths() {
        for uri in [
            "beans-jvm:///project/src/Example.class",
            "custom:///project/src/Example.java",
        ] {
            assert!(document_path(uri).is_none(), "{uri}");
        }
    }

    #[test]
    fn file_uri_paths_are_decoded_and_normalized() {
        assert_eq!(
            document_path("file:///project/old/../my%20sources/%C3%A9.java"),
            Some(PathBuf::from("/project/my sources/é.java"))
        );
    }
}
