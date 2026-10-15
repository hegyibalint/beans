use beans_core::names::Name;

use super::*;
use crate::model::{
    imports::Import,
    nodes::{Node, NodeIndex, NodeKind},
};

#[test]
fn models_round_trip_all_fields_and_ranges_including_incomplete_source() {
    for source in [
        "",
        "class Example {",
        "package p; import java.util.Map; public final class Example<T extends Number & Runnable> extends Base implements Interface { Map<String, ? super T>[] items; private class Inner {} }",
        "record Example(String value) {}",
        "enum Example { A, B }",
        "@interface Example {}",
        "class Example { void run() { { } } }",
        "package café; class Élément { Élément field; }",
    ] {
        let original = crate::lower_into(source);
        let bytes = encode(&original).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(encode(&decoded).unwrap(), bytes);
        assert_eq!(format!("{decoded:?}"), format!("{original:?}"));
        for offset in 0..source.len() {
            assert_eq!(
                decoded.type_reference_at(offset),
                original.type_reference_at(offset)
            );
        }
    }
}

#[test]
fn version_one_has_a_fixed_empty_file_encoding() {
    let bytes = [1, 0, 0, 1, 0, 0, 0];
    assert_eq!(encode(&File::new()).unwrap(), bytes);
    assert_eq!(encode(&decode(&bytes).unwrap()).unwrap(), bytes);
}

#[test]
fn decoded_models_own_their_data() {
    let decoded = {
        let file = crate::lower_into("package p; class Example { class Inner {} }");
        let bytes = encode(&file).unwrap();
        decode(&bytes).unwrap()
    };
    let name = Name::new(vec!["p".into(), "Example".into(), "Inner".into()]);
    assert_eq!(decoded.find_type(&name)[0].1.name(), Some("Inner"));
}

#[test]
fn missing_unsupported_truncated_and_trailing_payloads_are_rejected() {
    assert_eq!(decode(&[]).unwrap_err(), "missing Java model version");
    assert_eq!(
        decode(&[2]).unwrap_err(),
        "unsupported Java model version: 2"
    );
    let bytes = encode(&File::new()).unwrap();
    for end in 1..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut bytes = bytes;
    bytes.push(0);
    assert_eq!(decode(&bytes).unwrap_err(), "trailing bytes in Java model");
}

#[test]
fn invalid_node_graphs_are_rejected_before_semantic_traversal() {
    for nodes in [
        vec![],
        vec![Node::new(None, NodeKind::Block)],
        vec![Node::new(
            Some(NodeIndex::new(0)),
            NodeKind::CompilationUnit,
        )],
        vec![root(&[9])],
        vec![root(&[0])],
        vec![root(&[1, 1]), child(0, NodeKind::Block)],
        vec![root(&[]), child(0, NodeKind::Block)],
        vec![root(&[1]), child(1, NodeKind::Block)],
        vec![root(&[1]), child(0, NodeKind::CompilationUnit)],
        vec![root(&[1]), Node::new(None, NodeKind::Block)],
    ] {
        let bytes = postcard::to_extend(
            &(Name::default(), Vec::<Import>::new(), nodes),
            vec![MODEL_VERSION],
        )
        .unwrap();
        assert!(decode(&bytes).is_err());
    }
}

fn root(children: &[usize]) -> Node {
    let mut node = Node::new(None, NodeKind::CompilationUnit);
    for &child in children {
        node.add_child(NodeIndex::new(child));
    }
    node
}

fn child(parent: usize, kind: NodeKind) -> Node {
    Node::new(Some(NodeIndex::new(parent)), kind)
}
