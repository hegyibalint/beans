use super::{find_type_declaration, raw_type};
use crate::lower_into;
use beans_lang_java_model::nodes::NodeKind;

#[test]
fn field_preserves_its_name_and_declared_type() {
    let file = lower_into("class Owner { Value target; }");
    let owner = find_type_declaration(&file, "Owner");
    let fields = file
        .iter_children(owner.index)
        .filter_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) => Some(field),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [field] = fields.as_slice() else {
        panic!("expected one field declaration, found {}", fields.len());
    };

    assert_eq!(field.name, "target");
    assert_eq!(field.declared_type, raw_type(&["Value"]));
}
