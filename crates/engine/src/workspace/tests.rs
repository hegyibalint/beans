mod scopes;
mod sources;

use std::path::PathBuf;

use beans_platform_jvm as jvm;
use beans_workspace::model::{Selector, Unit, Workspace};

use super::*;

fn unit(id: &str, sources: Vec<Selector>) -> Unit {
    Unit {
        id: id.to_string(),
        sources,
        depends_on: Vec::new(),
        classpath: Vec::new(),
        jdk_home: None,
    }
}

fn tree(base: &str) -> Selector {
    Selector::Tree {
        base: PathBuf::from(base),
        includes: vec!["**/*.java".to_string()],
        excludes: Vec::new(),
        generated: false,
    }
}

fn workspace(units: Vec<Unit>) -> Workspace {
    Workspace {
        tool: "test".to_string(),
        units,
    }
}

fn source_file(path: &str) -> jvm::model::Source {
    jvm::model::Source::SourceFile {
        path: PathBuf::from(path),
    }
}
