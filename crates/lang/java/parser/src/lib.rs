use tree_sitter::{Parser, Tree};

pub fn parse(content: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .expect("tree-sitter-java must be compatible with tree-sitter");
    parser
        .parse(content, None)
        .expect("tree-sitter must produce a syntax tree")
}
