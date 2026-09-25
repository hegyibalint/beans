use super::{classes::Class, modules::ModuleDescriptor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassFile {
    Class(Class),
    ModuleDescriptor(ModuleDescriptor),
}
