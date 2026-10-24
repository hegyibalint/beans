mod artifacts;
mod separation;
mod sources;

use std::path::Path;

use beans_workspace_toml::{TomlWorkspace, parse};

fn workspace(contents: &str) -> TomlWorkspace {
    parse(contents, Path::new("/project")).unwrap()
}
