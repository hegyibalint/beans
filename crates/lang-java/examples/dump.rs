use std::fs;

use tree_sitter::{Node, Parser};

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump <file.java>");
    let src = fs::read_to_string(&path).expect("readable file");

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .expect("java grammar is compatible with the linked tree-sitter");
    let tree = parser.parse(&src, None).unwrap();

    print(tree.root_node(), &src, 0, None);
}

fn print(node: Node, src: &str, depth: usize, field: Option<&str>) {
    let indent = "  ".repeat(depth);
    let field = field.map(|f| format!("{f}: ")).unwrap_or_default();
    let marker = if node.is_error() || node.is_missing() { " ✗" } else { "" };
    let text = if node.child_count() == 0 {
        format!(" {:?}", &src[node.byte_range()])
    } else {
        String::new()
    };
    println!(
        "{indent}{field}{}{marker} [{}..{}]{text}",
        node.kind(),
        node.start_byte(),
        node.end_byte()
    );

    let mut cursor = node.walk();
    for (i, child) in node.children(&mut cursor).enumerate() {
        print(child, src, depth + 1, node.field_name_for_child(i as u32));
    }
}
