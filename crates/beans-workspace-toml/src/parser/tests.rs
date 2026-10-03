mod jdk_override;
mod loading;
mod paths;
mod schema;

use std::path::Path;

use crate::model::Project;

fn project(contents: &str) -> Project {
    super::parse(contents, Path::new("/project"))
        .unwrap()
        .model()
        .clone()
}
