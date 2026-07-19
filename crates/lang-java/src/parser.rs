use tree_sitter::{Node, Parser};

use crate::model::{
    JavaDeclaration, JavaDeclarationId, JavaFile, JavaIdentifier, JavaImport, JavaImportKind,
    JavaLexicalScope, JavaLexicalScopeId, JavaName, JavaQualifiedName, JavaTypeDeclaration,
    JavaTypeKind,
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
        parse_program(tree.root_node(), contents)
    }
}

fn parse_program(root: Node, src: &str) -> JavaFile {
    debug_assert_eq!(root.kind(), "program");

    let mut file = JavaFile::new();
    let compilation_unit_scope = file.compilation_unit_scope;

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
                if let Some(declaration) =
                    parse_class_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_types.push(declaration);
                }
            }
            "interface_declaration" => {
                if let Some(declaration) =
                    parse_interface_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_types.push(declaration);
                }
            }
            "enum_declaration" => {
                if let Some(declaration) =
                    parse_enum_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_types.push(declaration);
                }
            }
            "record_declaration" => {
                if let Some(declaration) =
                    parse_record_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_types.push(declaration);
                }
            }
            "annotation_type_declaration" => {
                if let Some(declaration) =
                    parse_annotation_type_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_types.push(declaration);
                }
            }
            "module_declaration" | "line_comment" | "block_comment" => {}
            _ => {}
        }
    }

    file
}

fn parse_package_declaration(node: Node, src: &str) -> Option<JavaName> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| match child.kind() {
            "identifier" | "scoped_identifier" => parse_name(child, src),
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
                name = parse_name(child, src);
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

fn parse_class_declaration(
    node: Node,
    declaring_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    add_type_declaration(node, JavaTypeKind::Class, declaring_scope, src, file)
}

fn parse_interface_declaration(
    node: Node,
    declaring_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    add_type_declaration(node, JavaTypeKind::Interface, declaring_scope, src, file)
}

fn parse_enum_declaration(
    node: Node,
    declaring_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    add_type_declaration(node, JavaTypeKind::Enum, declaring_scope, src, file)
}

fn parse_record_declaration(
    node: Node,
    declaring_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    add_type_declaration(node, JavaTypeKind::Record, declaring_scope, src, file)
}

fn parse_annotation_type_declaration(
    node: Node,
    declaring_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    add_type_declaration(
        node,
        JavaTypeKind::AnnotationInterface,
        declaring_scope,
        src,
        file,
    )
}

fn add_type_declaration(
    node: Node,
    kind: JavaTypeKind,
    declaring_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    let name = parse_identifier(node.child_by_field_name("name")?, src)?;
    let body = node.child_by_field_name("body")?;
    let body_scope = JavaLexicalScopeId(file.lexical_scopes.len());
    file.lexical_scopes.push(JavaLexicalScope {
        parent: Some(declaring_scope),
        declarations: Vec::new(),
    });

    let declaration = JavaDeclarationId(file.declarations.len());
    file.declarations
        .push(JavaDeclaration::Type(JavaTypeDeclaration {
            name: Some(name),
            kind,
            declaring_scope,
            body_scope,
        }));
    file.lexical_scopes[declaring_scope.0]
        .declarations
        .push(declaration);

    walk_type_body(body, body_scope, src, file);

    Some(declaration)
}

fn walk_type_body(node: Node, scope: JavaLexicalScopeId, src: &str, file: &mut JavaFile) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "class_declaration" => {
                parse_class_declaration(child, scope, src, file);
            }
            "interface_declaration" => {
                parse_interface_declaration(child, scope, src, file);
            }
            "enum_declaration" => {
                parse_enum_declaration(child, scope, src, file);
            }
            "record_declaration" => {
                parse_record_declaration(child, scope, src, file);
            }
            "annotation_type_declaration" => {
                parse_annotation_type_declaration(child, scope, src, file);
            }
            "enum_body_declarations" => walk_type_body(child, scope, src, file),
            _ => {}
        }
    }
}

fn parse_name(node: Node, src: &str) -> Option<JavaName> {
    match node.kind() {
        "identifier" => Some(JavaName::Simple(parse_identifier(node, src)?)),
        "scoped_identifier" => Some(JavaName::Qualified(parse_scoped_identifier(node, src)?)),
        kind => panic!("uncovered name node kind: {kind}"),
    }
}

fn parse_scoped_identifier(node: Node, src: &str) -> Option<JavaQualifiedName> {
    let mut identifiers = Vec::new();
    collect_scoped_identifier(node, src, &mut identifiers)?;
    Some(JavaQualifiedName::new(
        identifiers,
        node.byte_range().into(),
    ))
}

fn collect_scoped_identifier(
    node: Node,
    src: &str,
    identifiers: &mut Vec<JavaIdentifier>,
) -> Option<()> {
    let scope = node.child_by_field_name("scope")?;
    match scope.kind() {
        "identifier" => identifiers.push(parse_identifier(scope, src)?),
        "scoped_identifier" => collect_scoped_identifier(scope, src, identifiers)?,
        kind => panic!("uncovered scoped identifier scope kind: {kind}"),
    }
    identifiers.push(parse_identifier(node.child_by_field_name("name")?, src)?);
    Some(())
}

fn parse_identifier(node: Node, src: &str) -> Option<JavaIdentifier> {
    match node.kind() {
        "identifier" => Some(JavaIdentifier {
            text: util_copy_source(node, src),
            span: node.byte_range().into(),
        }),
        kind => panic!("uncovered identifier node kind: {kind}"),
    }
}

fn util_copy_source(node: Node, src: &str) -> String {
    src[node.byte_range()].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_declaration(file: &JavaFile, id: JavaDeclarationId) -> &JavaTypeDeclaration {
        let JavaDeclaration::Type(declaration) = &file.declarations[id.0] else {
            panic!("expected a type declaration");
        };
        declaration
    }

    #[test]
    fn parses_compilation_unit_declarations() {
        let content = "package org.beans.test;\nimport java.util.List;\nclass Foo {}\n";
        let mut parser = JavaParser::new();
        let file = parser.parse(content);

        assert_eq!(
            file.package.as_ref().map(JavaName::dotted),
            Some("org.beans.test".to_string())
        );
        assert!(matches!(&file.package, Some(JavaName::Qualified(_))));
        assert_eq!(file.imports.len(), 1);
        assert_eq!(file.imports[0].name.dotted(), "java.util.List");
        assert!(matches!(&file.imports[0].name, JavaName::Qualified(_)));
        assert_eq!(file.imports[0].kind, JavaImportKind::Type);

        assert_eq!(file.top_level_types, [JavaDeclarationId(0)]);
        assert_eq!(
            file.lexical_scopes[file.compilation_unit_scope.0].declarations,
            [JavaDeclarationId(0)]
        );

        let declaration = type_declaration(&file, JavaDeclarationId(0));
        assert_eq!(
            declaration.name.as_ref().map(|name| name.text.as_str()),
            Some("Foo")
        );
        assert_eq!(declaration.kind, JavaTypeKind::Class);
        assert_eq!(declaration.declaring_scope, file.compilation_unit_scope);
        assert_eq!(
            file.lexical_scopes[declaration.body_scope.0].parent,
            Some(file.compilation_unit_scope)
        );
    }

    #[test]
    fn parses_a_single_identifier_as_a_simple_name() {
        let mut parser = JavaParser::new();
        let file = parser.parse("package example; class Example {}");

        let Some(JavaName::Simple(identifier)) = file.package else {
            panic!("expected a simple package name");
        };
        assert_eq!(identifier.text, "example");
    }

    #[test]
    fn parses_each_named_type_kind() {
        let content = "class C {} interface I {} enum E {} record R() {} @interface A {}";
        let mut parser = JavaParser::new();
        let file = parser.parse(content);

        let kinds: Vec<_> = file
            .top_level_types
            .iter()
            .map(|id| type_declaration(&file, *id).kind)
            .collect();
        assert_eq!(
            kinds,
            [
                JavaTypeKind::Class,
                JavaTypeKind::Interface,
                JavaTypeKind::Enum,
                JavaTypeKind::Record,
                JavaTypeKind::AnnotationInterface,
            ]
        );
    }

    #[test]
    fn recursively_parses_member_types() {
        let content = "class Outer { class Member { interface Deep {} } }";
        let mut parser = JavaParser::new();
        let file = parser.parse(content);

        assert_eq!(file.top_level_types, [JavaDeclarationId(0)]);
        assert_eq!(file.declarations.len(), 3);

        let outer = type_declaration(&file, JavaDeclarationId(0));
        let member = type_declaration(&file, JavaDeclarationId(1));
        let deep = type_declaration(&file, JavaDeclarationId(2));

        assert_eq!(
            file.lexical_scopes[outer.body_scope.0].declarations,
            [JavaDeclarationId(1)]
        );
        assert_eq!(member.declaring_scope, outer.body_scope);
        assert_eq!(
            file.lexical_scopes[member.body_scope.0].declarations,
            [JavaDeclarationId(2)]
        );
        assert_eq!(deep.declaring_scope, member.body_scope);
    }
}
