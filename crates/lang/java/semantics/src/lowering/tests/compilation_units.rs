use crate::lower_into;
use beans_lang_java_model::{File, nodes::NodeKind};

#[test]
fn empty_compilation_unit_has_only_the_root_node() {
    let file = lower_into("");
    let root = file.node(File::ROOT_NODE_ID).unwrap();

    assert!(file.package_name.is_empty());
    assert!(file.imports.is_empty());
    assert_eq!(file.iter_nodes().count(), 1);
    assert!(matches!(root.kind(), NodeKind::CompilationUnit));
    assert_eq!(root.parent(), None);
    assert!(root.iter_children().next().is_none());
}

#[test]
fn simple_package_name_is_preserved() {
    let file = lower_into("package inventory;");

    assert_eq!(file.package_name.as_slice(), ["inventory"]);
}

#[test]
fn qualified_package_name_preserves_its_components() {
    let file = lower_into("package example.inventory;");

    assert_eq!(file.package_name.as_slice(), ["example", "inventory"]);
}
