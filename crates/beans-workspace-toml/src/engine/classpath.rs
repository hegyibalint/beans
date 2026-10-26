use beans_core::{classpath::Classpath, query_scope::QueryScope};

use crate::model::{Project, Unit};

#[derive(Debug, Default)]
pub(super) struct SourceClasspath {
    source_roots: Vec<Classpath>,
    artifacts: Vec<Classpath>,
}

impl SourceClasspath {
    pub(super) fn include(&mut self, unit: &Unit, project: &Project) {
        self.source_roots
            .extend(unit.sources.iter().map(Classpath::file));
        for dependency in &unit.depends_on {
            self.source_roots.extend(
                project.units[dependency]
                    .sources
                    .iter()
                    .map(Classpath::file),
            );
        }
        self.artifacts
            .extend(unit.classpath.iter().map(Classpath::file));
        if let Some(home) = &unit.jdk_home {
            self.artifacts
                .push(Classpath::file(home.join("lib/modules")));
        }
    }

    pub(super) fn scope(&self) -> QueryScope<'_> {
        QueryScope::new(&self.source_roots, &self.artifacts)
    }
}
