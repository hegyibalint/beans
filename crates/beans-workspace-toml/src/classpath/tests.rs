mod artifacts;
mod sources;

use crate::model::{Module, Project};

use super::SourceClasspath;

fn module(sources: &[&str]) -> Module {
    Module {
        sources: sources.iter().map(Into::into).collect(),
        depends_on: Vec::new(),
    }
}

fn classpath(module: Module, dependencies: &[(&str, Module)]) -> SourceClasspath {
    let mut project = Project {
        root: "/project".into(),
        jdk_home: None,
        modules: dependencies
            .iter()
            .map(|(id, module)| ((*id).into(), module.clone()))
            .collect(),
    };
    project.modules.insert("app".into(), module);
    let mut classpath = SourceClasspath::default();
    classpath.include(&project.modules["app"], &project);
    classpath
}
