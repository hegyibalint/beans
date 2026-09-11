use crate::query::JavaTypeEntry;
use beans_core_model::source::Source;
use beans_lang_java_model::{File, nodes::NodeKind};

#[test]
fn an_empty_member_path_returns_the_starting_target_unchanged() {
    let file = crate::lower_into("class Outer {}");
    let source = Source::SourceFile {
        path: "Outer.java".into(),
    };
    let entry = file.iter_children(File::ROOT_NODE_ID).next().unwrap();
    let NodeKind::Type(declaration) = entry.node.kind() else {
        panic!("expected a type declaration");
    };
    let target = JavaTypeEntry {
        source: &source,
        file: &file,
        node_index: entry.index,
        declaration,
    };

    let resolved = target.resolve_member_path(&[]).unwrap();

    assert!(std::ptr::eq(resolved.source, target.source));
    assert!(std::ptr::eq(resolved.file, target.file));
    assert!(std::ptr::eq(resolved.declaration, target.declaration));
    assert_eq!(resolved.node_index, target.node_index);
}
