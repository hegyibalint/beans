use crate::model::Project;

/// A resolved workspace descriptor.
#[derive(Debug)]
pub struct TomlWorkspace {
    project: Project,
}

impl TomlWorkspace {
    pub(crate) fn new(project: Project) -> Self {
        Self { project }
    }

    pub fn model(&self) -> &Project {
        &self.project
    }
}
