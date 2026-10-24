use beans_core::ranges::{ByteRange, Spanned};

use beans_lang_java::model::{
    File,
    nodes::{
        NodeKind,
        fields::FieldDeclaration,
        methods::MethodDeclaration,
        types::{Kind, TypeDeclaration},
    },
    references::{PrimitiveType, TypeRef},
};

fn span<T>(value: T) -> Spanned<T> {
    Spanned::new(value, ByteRange::new(0, 0))
}

#[test]
fn insertion_preserves_payloads_and_links_in_both_directions() {
    let mut file = File::new();
    let typ = file.add_node(
        File::ROOT_NODE_ID,
        NodeKind::Type(TypeDeclaration::new(Kind::Class)),
    );
    let field = file.add_node(
        typ,
        NodeKind::Field(FieldDeclaration {
            name: "value".into(),
            declared_type: span(TypeRef::Primitive(PrimitiveType::Int)),
        }),
    );
    let method = file.add_node(typ, NodeKind::Method(MethodDeclaration {}));
    let block = file.add_node(method, NodeKind::Block);

    assert_eq!(file.node(typ).unwrap().parent(), Some(File::ROOT_NODE_ID));
    assert_eq!(file.node(field).unwrap().parent(), Some(typ));
    assert_eq!(file.node(method).unwrap().parent(), Some(typ));
    assert_eq!(file.node(block).unwrap().parent(), Some(method));
    assert_eq!(
        file.node(typ).unwrap().iter_children().collect::<Vec<_>>(),
        [field, method]
    );
    assert_eq!(
        file.node(method)
            .unwrap()
            .iter_children()
            .collect::<Vec<_>>(),
        [block]
    );
    let NodeKind::Field(payload) = file.node(field).unwrap().kind() else {
        panic!("expected field");
    };
    assert_eq!(payload.name, "value");
    assert_eq!(
        payload.declared_type,
        span(TypeRef::Primitive(PrimitiveType::Int))
    );
    assert!(matches!(
        file.node(method).unwrap().kind(),
        NodeKind::Method(_)
    ));
    assert!(matches!(file.node(block).unwrap().kind(), NodeKind::Block));
}

#[test]
fn ancestors_include_start_and_root_but_not_siblings() {
    let mut file = File::new();
    let outer = file.add_node(File::ROOT_NODE_ID, NodeKind::Block);
    let inner = file.add_node(outer, NodeKind::Block);
    file.add_node(outer, NodeKind::Block);
    assert_eq!(
        file.iter_ancestors(inner)
            .map(|entry| entry.index)
            .collect::<Vec<_>>(),
        [inner, outer, File::ROOT_NODE_ID]
    );
    assert_eq!(file.iter_ancestors(File::ROOT_NODE_ID).count(), 1);
    for entry in file.iter_ancestors(inner) {
        assert!(std::ptr::eq(entry.node, file.node(entry.index).unwrap()));
    }
}

#[test]
fn children_are_direct_and_ordered_even_when_insertions_are_interleaved() {
    let mut file = File::new();
    let first = file.add_node(File::ROOT_NODE_ID, NodeKind::Block);
    let nested = file.add_node(first, NodeKind::Block);
    let second = file.add_node(File::ROOT_NODE_ID, NodeKind::Block);
    assert_eq!(
        file.iter_children(File::ROOT_NODE_ID)
            .map(|entry| entry.index)
            .collect::<Vec<_>>(),
        [first, second]
    );
    assert_eq!(
        file.iter_children(first)
            .map(|entry| entry.index)
            .collect::<Vec<_>>(),
        [nested]
    );
    assert!(file.iter_children(second).next().is_none());
    for entry in file.iter_children(File::ROOT_NODE_ID) {
        assert!(std::ptr::eq(entry.node, file.node(entry.index).unwrap()));
    }
}

#[test]
fn arena_iteration_visits_each_stored_node_once_in_insertion_order() {
    let mut file = File::new();
    let first = file.add_node(File::ROOT_NODE_ID, NodeKind::Block);
    let nested = file.add_node(first, NodeKind::Block);
    let second = file.add_node(File::ROOT_NODE_ID, NodeKind::Block);
    assert_eq!(
        file.iter_nodes()
            .map(|entry| entry.index)
            .collect::<Vec<_>>(),
        [File::ROOT_NODE_ID, first, nested, second]
    );
    for entry in file.iter_nodes() {
        assert!(std::ptr::eq(entry.node, file.node(entry.index).unwrap()));
    }
}
