mod jdk_override;
mod loading;
mod paths;
mod schema;

use std::path::Path;

use beans_workspace_toml::{model::Project, parse};

fn project(contents: &str) -> Project {
    parse(contents, Path::new("/project"))
        .unwrap()
        .model()
        .clone()
}
