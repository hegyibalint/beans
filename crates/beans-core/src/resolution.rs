use crate::names::Name;

pub trait TypeDefinitionQuery<Handle> {
    fn find_types(&self, name: &Name) -> Result<Vec<Handle>, String>;
}
