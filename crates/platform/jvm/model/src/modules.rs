use crate::names::ModuleName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDescriptor {
    pub name: ModuleName,
}

impl ModuleDescriptor {
    pub fn new(name: ModuleName) -> Self {
        Self { name }
    }
}
