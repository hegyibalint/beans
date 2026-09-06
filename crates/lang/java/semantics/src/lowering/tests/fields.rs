use super::{find_type_body_scope, find_type_declaration, raw_type};
use crate::lower_into;
use beans_lang_java_model::declarations::Declaration;

#[test]
fn field_preserves_its_name_and_declared_type() {
    let file = lower_into("class Owner { Value target; }");
    let owner = find_type_declaration(&file, "Owner");
    let owner_body = find_type_body_scope(&file, owner.declaration_id);
    let fields = file
        .iter_declarations_in(owner_body.scope_index)
        .filter_map(|entry| match entry.declaration {
            Declaration::Field(field) => Some(field),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [field] = fields.as_slice() else {
        panic!("expected one field declaration, found {}", fields.len());
    };

    assert_eq!(field.name, "target");
    assert_eq!(field.declared_type, raw_type(&["Value"]));
}
