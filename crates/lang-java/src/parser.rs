use beans_core::VirtualFile;
use tree_sitter::{Node, Parser};

use crate::model::{
    JavaClass, JavaFile, JavaImport, JavaImportKind, JavaQualifiedName, JavaSimpleName,
};

pub struct JavaParser {
    parser: Parser,
}

impl JavaParser {
    pub fn new() -> JavaParser {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .expect("java grammar is compatible with the linked tree-sitter");
        JavaParser { parser }
    }

    pub fn parse(&mut self, file: VirtualFile) -> JavaFile {
        let tree = self
            .parser
            .parse(&file.content, None)
            .expect("parse returns a tree when a language is set");
        parse_file(tree.root_node(), &file.content)
    }
}

fn parse_file(root: Node, src: &str) -> JavaFile {
    let mut file = JavaFile::default();

    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "package_declaration" => {
                file.package = parse_package_declaration(child, src);
            }
            "import_declaration" => {
                if let Some(import) = parse_import_declaration(child, src) {
                    file.imports.push(import);
                }
            }
            "class_declaration" => {
                if let Some(class) = parse_class_declaration(child, src) {
                    file.classes.push(class);
                }
            }
            _ => {}
        }
    }

    file
}

fn parse_package_declaration(node: Node, src: &str) -> Option<JavaQualifiedName> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| match child.kind() {
            "identifier" | "scoped_identifier" => parse_qualified_name(child, src),
            _ => None,
        })
}

fn parse_import_declaration(node: Node, src: &str) -> Option<JavaImport> {
    let mut name = None;
    let mut is_static = false;
    let mut on_demand = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "scoped_identifier" => {
                name = parse_qualified_name(child, src);
            }
            "static" => is_static = true,
            "asterisk" => on_demand = true,
            _ => {}
        }
    }

    let kind = match (is_static, on_demand) {
        (false, false) => JavaImportKind::Type,
        (false, true) => JavaImportKind::TypeOnDemand,
        (true, false) => JavaImportKind::Static,
        (true, true) => JavaImportKind::StaticOnDemand,
    };

    Some(JavaImport { name: name?, kind })
}

fn parse_class_declaration(node: Node, src: &str) -> Option<JavaClass> {
    let name = parse_simple_name(node.child_by_field_name("name")?, src)?;
    Some(JavaClass { name })
}

fn parse_qualified_name(node: Node, src: &str) -> Option<JavaQualifiedName> {
    match node.kind() {
        "identifier" | "type_identifier" => Some(JavaQualifiedName {
            segments: vec![parse_simple_name(node, src)?],
            span: node.byte_range(),
        }),
        "scoped_identifier" => {
            let mut qualified = parse_qualified_name(node.child_by_field_name("scope")?, src)?;
            let last = parse_simple_name(node.child_by_field_name("name")?, src)?;
            qualified.segments.push(last);
            qualified.span = node.byte_range();
            Some(qualified)
        }
        kind => panic!("uncovered name node kind: {kind}"),
    }
}

fn parse_simple_name(node: Node, src: &str) -> Option<JavaSimpleName> {
    match node.kind() {
        "identifier" | "type_identifier" => Some(JavaSimpleName {
            text: util_copy_source(node, src),
            span: node.byte_range(),
        }),
        kind => panic!("uncovered simple name node kind: {kind}"),
    }
}

fn util_copy_source(node: Node, src: &str) -> String {
    src[node.byte_range()].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::JavaImportKind;

    #[test]
    fn parse_minimal_file() {
        let content = r#"package org.beans.test;

import java.util.List;

class Foo {
}
"#
        .to_string();

        let mut parser = JavaParser::new();
        let model = parser.parse(VirtualFile {
            uri: "file:///Foo.java".to_string(),
            content: content.clone(),
        });

        let package = model.package.as_ref().expect("package is parsed");
        let segments: Vec<&str> = package.segments.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(segments, ["org", "beans", "test"]);

        assert_eq!(model.imports.len(), 1);
        let import = &model.imports[0];
        let segments: Vec<&str> = import.name.segments.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(segments, ["java", "util", "List"]);
        assert!(matches!(import.kind, JavaImportKind::Type));

        assert_eq!(model.classes.len(), 1);
        let class = &model.classes[0];
        assert_eq!(class.name.text, "Foo");

        let span = &class.name.span;
        assert_eq!(&content[span.start as usize..span.end as usize], "Foo");

        eprintln!("{model:#?}");
    }
}
