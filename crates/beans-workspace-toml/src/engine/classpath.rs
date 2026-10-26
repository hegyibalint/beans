use beans_core::{query_scope::QueryScope, resource_filter::ResourceFilter};

use crate::model::{Project, Unit};

#[derive(Debug, Default)]
pub(super) struct SourceClasspath {
    source_roots: Vec<ResourceFilter>,
    artifacts: Vec<ResourceFilter>,
}

impl SourceClasspath {
    pub(super) fn include(&mut self, unit: &Unit, project: &Project) {
        self.source_roots
            .extend(unit.sources.iter().map(ResourceFilter::file));
        for dependency in &unit.depends_on {
            self.source_roots.extend(
                project.units[dependency]
                    .sources
                    .iter()
                    .map(ResourceFilter::file),
            );
        }
        self.artifacts
            .extend(unit.classpath.iter().map(ResourceFilter::file));
        if let Some(home) = &unit.jdk_home {
            self.artifacts
                .push(ResourceFilter::file(home.join("lib/modules")));
        }
    }

    pub(super) fn scope(&self) -> QueryScope<'_> {
        QueryScope::new(&self.source_roots, &self.artifacts)
    }
}
