use tree_sitter::{Node, Parser};

use crate::model::{
    JavaClass, JavaField, JavaFile, JavaImport, JavaImportKind, JavaMethod, JavaMethodParameter,
    JavaQualifiedName, JavaSimpleName,
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

    pub fn parse(&mut self, contents: &str) -> JavaFile {
        let tree = self
            .parser
            .parse(contents, None)
            .expect("parse returns a tree when a language is set");
        parse_file(tree.root_node(), contents)
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

    let mut fields = Vec::new();
    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.named_children(&mut cursor) {
            match member.kind() {
                "field_declaration" => fields.extend(parse_field_declaration(member, src)),
                "method_declaration" => methods.extend(parse_method_declaration(member, src)),
                _ => {}
            }
        }
    }

    Some(JavaClass {
        name,
        fields,
        methods,
    })
}

fn parse_field_declaration(node: Node, src: &str) -> Vec<JavaField> {
    let Some(java_type) = node
        .child_by_field_name("type")
        .and_then(|t| parse_type(t, src))
    else {
        return Vec::new();
    };

    let mut cursor = node.walk();
    node.children_by_field_name("declarator", &mut cursor)
        .filter_map(|decl| {
            let name = parse_simple_name(decl.child_by_field_name("name")?, src)?;
            Some(JavaField {
                name,
                java_type: java_type.clone(),
            })
        })
        .collect()
}

fn parse_method_declaration(node: Node, src: &str) -> Option<JavaMethod> {
    let name = parse_simple_name(node.child_by_field_name("name")?, src)?;
    let return_type = parse_type(node.child_by_field_name("type")?, src)?;

    let mut cursor = node.walk();
    let params = node
        .child_by_field_name("parameters")
        .map(|list| {
            list.named_children(&mut cursor)
                .filter(|c| c.kind() == "formal_parameter")
                .filter_map(|param| parse_formal_parameter(param, src))
                .collect()
        })
        .unwrap_or_default();

    Some(JavaMethod {
        name,
        params,
        return_type,
    })
}

fn parse_formal_parameter(node: Node, src: &str) -> Option<JavaMethodParameter> {
    let name = parse_simple_name(node.child_by_field_name("name")?, src)?;
    let java_type = parse_type(node.child_by_field_name("type")?, src)?;
    Some(JavaMethodParameter { name, java_type })
}

/// Pull the named type out of a type node. Reference types (`Bar`,
/// `java.util.List`, `List<T>`) yield a qualified name; primitives,
/// arrays and `void` are not modelled yet and drop out as `None`.
fn parse_type(node: Node, src: &str) -> Option<JavaQualifiedName> {
    match node.kind() {
        "type_identifier" => Some(JavaQualifiedName {
            segments: vec![parse_simple_name(node, src)?],
            span: node.byte_range().into(),
        }),
        "scoped_type_identifier" => {
            let mut cursor = node.walk();
            let mut segments = Vec::new();
            for child in node.named_children(&mut cursor) {
                match child.kind() {
                    "type_identifier" => segments.push(parse_simple_name(child, src)?),
                    "scoped_type_identifier" => segments.extend(parse_type(child, src)?.segments),
                    _ => {}
                }
            }
            (!segments.is_empty()).then(|| JavaQualifiedName {
                segments,
                span: node.byte_range().into(),
            })
        }
        "generic_type" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .find_map(|child| match child.kind() {
                    "type_identifier" | "scoped_type_identifier" => parse_type(child, src),
                    _ => None,
                })
        }
        _ => None,
    }
}

fn parse_qualified_name(node: Node, src: &str) -> Option<JavaQualifiedName> {
    match node.kind() {
        "identifier" | "type_identifier" => Some(JavaQualifiedName {
            segments: vec![parse_simple_name(node, src)?],
            span: node.byte_range().into(),
        }),
        "scoped_identifier" => {
            let mut qualified = parse_qualified_name(node.child_by_field_name("scope")?, src)?;
            let last = parse_simple_name(node.child_by_field_name("name")?, src)?;
            qualified.segments.push(last);
            qualified.span = node.byte_range().into();
            Some(qualified)
        }
        kind => panic!("uncovered name node kind: {kind}"),
    }
}

fn parse_simple_name(node: Node, src: &str) -> Option<JavaSimpleName> {
    match node.kind() {
        "identifier" | "type_identifier" => Some(JavaSimpleName {
            text: util_copy_source(node, src),
            span: node.byte_range().into(),
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
        let model = parser.parse(content.as_str());

        let package = model.package.as_ref().expect("package is parsed");
        let segments: Vec<&str> = package.segments.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(segments, ["org", "beans", "test"]);

        assert_eq!(model.imports.len(), 1);
        let import = &model.imports[0];
        let segments: Vec<&str> = import
            .name
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(segments, ["java", "util", "List"]);
        assert!(matches!(import.kind, JavaImportKind::Type));

        assert_eq!(model.classes.len(), 1);
        let class = &model.classes[0];
        assert_eq!(class.name.text, "Foo");

        let span = &class.name.span;
        assert_eq!(&content[span.start as usize..span.end as usize], "Foo");

        eprintln!("{model:#?}");
    }

    #[test]
    fn parse_member_type_references() {
        let content = r#"package org.beans.test.asd;

class Foo {
    Bar bar;

    Baz make(Qux q) {
    }
}
"#
        .to_string();

        let mut parser = JavaParser::new();
        let model = parser.parse(content.as_str());

        let class = &model.classes[0];

        let field_type: Vec<&str> = class.fields[0]
            .java_type
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(class.fields[0].name.text, "bar");
        assert_eq!(field_type, ["Bar"]);

        let method = &class.methods[0];
        assert_eq!(method.name.text, "make");
        let return_type: Vec<&str> =
            method.return_type.segments.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(return_type, ["Baz"]);
        assert_eq!(method.params[0].name.text, "q");
        let param_type: Vec<&str> = method.params[0]
            .java_type
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(param_type, ["Qux"]);
    }
}
