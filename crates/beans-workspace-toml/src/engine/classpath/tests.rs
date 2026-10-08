mod artifacts;
mod separation;
mod sources;

use crate::model::{Project, Unit};

use super::SourceClasspath;

fn unit(sources: &[&str]) -> Unit {
    Unit {
        sources: sources.iter().map(Into::into).collect(),
        depends_on: Vec::new(),
        classpath: Vec::new(),
        jdk_home: None,
    }
}

fn classpath(unit: Unit, dependencies: &[(&str, Unit)]) -> SourceClasspath {
    let mut project = Project {
        root: "/project".into(),
        units: dependencies
            .iter()
            .map(|(id, unit)| ((*id).into(), unit.clone()))
            .collect(),
    };
    project.units.insert("app".into(), unit);
    let mut classpath = SourceClasspath::default();
    classpath.include(&project.units["app"], &project);
    classpath
}
