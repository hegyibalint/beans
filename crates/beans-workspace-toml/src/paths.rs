use std::path::PathBuf;

pub(crate) use beans_core::paths::normalize;
use url::Url;

pub(crate) fn document_path(uri: &str) -> Option<PathBuf> {
    // Url::to_file_path does not check the scheme itself.
    let uri = Url::parse(uri).ok().filter(|uri| uri.scheme() == "file")?;
    let path = uri.to_file_path().ok()?;
    Some(normalize(&path))
}
