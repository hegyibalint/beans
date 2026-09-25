use crate::model::names::Name;

pub trait TypeDefinitionQuery<Handle> {
    fn find_types<'query>(
        &'query self,
        name: &'query Name,
    ) -> impl Iterator<Item = Handle> + 'query;
}
