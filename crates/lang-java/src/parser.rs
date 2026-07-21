use beans_core::model::Span;
use tree_sitter::{Node, Parser};

use crate::model::{
    JavaBody, JavaBodyNode, JavaBodyNodeId, JavaBodyNodeKind, JavaConstructorDeclaration,
    JavaDeclaration, JavaDeclarationId, JavaExpression, JavaFieldDeclaration, JavaFile,
    JavaIdentifier, JavaImport, JavaImportKind, JavaLexicalScope, JavaLexicalScopeId,
    JavaLocalDeclaration, JavaMethodDeclaration, JavaName, JavaParameterDeclaration,
    JavaPositionIndex, JavaQualifiedName, JavaStatement, JavaTypeDeclaration, JavaTypeKind,
    JavaTypeRef,
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
                    file.top_level_declarations.push(declaration);
                }
            }
            "interface_declaration" => {
                if let Some(declaration) =
                    parse_interface_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_declarations.push(declaration);
                }
            }
            "enum_declaration" => {
                if let Some(declaration) =
                    parse_enum_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_declarations.push(declaration);
                }
            }
            "record_declaration" => {
                if let Some(declaration) =
                    parse_record_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_declarations.push(declaration);
                }
            }
            "annotation_type_declaration" => {
                if let Some(declaration) =
                    parse_annotation_type_declaration(child, compilation_unit_scope, src, &mut file)
                {
                    file.top_level_declarations.push(declaration);
                }
            }
            "module_declaration" | "line_comment" | "block_comment" => {}
            _ => {}
        }
    }

    file.lexical_scopes[compilation_unit_scope.0].span = Span {
        start: 0,
        end: src.len(),
    };
    file.position_index = JavaPositionIndex::build(&file);
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

fn new_scope(
    file: &mut JavaFile,
    parent: JavaLexicalScopeId,
    owner: Option<JavaDeclarationId>,
    span: Span,
) -> JavaLexicalScopeId {
    let scope_id = JavaLexicalScopeId(file.lexical_scopes.len());
    file.lexical_scopes.push(JavaLexicalScope {
        parent: Some(parent),
        owner,
        declarations: Vec::new(),
        span,
    });
    scope_id
}

fn add_declaration(
    file: &mut JavaFile,
    declaring_scope: JavaLexicalScopeId,
    declaration: JavaDeclaration,
) -> JavaDeclarationId {
    let declaration_id = JavaDeclarationId(file.declarations.len());
    file.declarations.push(declaration);
    file.lexical_scopes[declaring_scope.0]
        .declarations
        .push(declaration_id);
    declaration_id
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
    let superclass = node
        .child_by_field_name("superclass")
        .and_then(|superclass| superclass.named_child(0))
        .and_then(|ty| parse_type_ref(ty, src));
    let body_scope = new_scope(file, declaring_scope, None, body.byte_range().into());

    let declaration = add_declaration(
        file,
        declaring_scope,
        JavaDeclaration::Type(JavaTypeDeclaration {
            span: node.byte_range().into(),
            name: Some(name),
            kind,
            superclass,
            declaring_scope,
            body_scope,
        }),
    );
    file.lexical_scopes[body_scope.0].owner = Some(declaration);

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
            "field_declaration" => parse_field_declaration(child, scope, src, file),
            "method_declaration" => {
                parse_method_declaration(child, scope, src, file);
            }
            "constructor_declaration" => {
                parse_constructor_declaration(child, scope, src, file);
            }
            "enum_body_declarations" => walk_type_body(child, scope, src, file),
            _ => {}
        }
    }
}

fn parse_field_declaration(node: Node, scope: JavaLexicalScopeId, src: &str, file: &mut JavaFile) {
    let ty = node
        .child_by_field_name("type")
        .and_then(|ty| parse_type_ref(ty, src));

    let mut cursor = node.walk();
    for declarator in node.children_by_field_name("declarator", &mut cursor) {
        let name = declarator
            .child_by_field_name("name")
            .and_then(|name| parse_identifier(name, src));
        add_declaration(
            file,
            scope,
            JavaDeclaration::Field(JavaFieldDeclaration {
                span: declarator.byte_range().into(),
                name,
                referenced_type: ty.clone(),
                declaring_scope: scope,
            }),
        );
    }
}

fn parse_method_declaration(
    node: Node,
    declaring_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    let name = parse_identifier(node.child_by_field_name("name")?, src)?;
    let return_type = node
        .child_by_field_name("type")
        .and_then(|ty| parse_type_ref(ty, src));
    let method_scope = new_scope(file, declaring_scope, None, node.byte_range().into());

    // Declare before parsing the contents so declaration ids follow source order.
    let declaration = add_declaration(
        file,
        declaring_scope,
        JavaDeclaration::Method(JavaMethodDeclaration {
            span: node.byte_range().into(),
            name: Some(name),
            return_type,
            parameters: Vec::new(),
            declaring_scope,
            body_scope: method_scope,
            body: None,
        }),
    );
    file.lexical_scopes[method_scope.0].owner = Some(declaration);

    let parameters = parse_formal_parameters(node, method_scope, src, file);
    let body = parse_body(node, method_scope, src, file);
    let JavaDeclaration::Method(method) = &mut file.declarations[declaration.0] else {
        unreachable!();
    };
    method.parameters = parameters;
    method.body = body;
    Some(declaration)
}

fn parse_constructor_declaration(
    node: Node,
    declaring_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    let constructor_scope = new_scope(file, declaring_scope, None, node.byte_range().into());

    let declaration = add_declaration(
        file,
        declaring_scope,
        JavaDeclaration::Constructor(JavaConstructorDeclaration {
            span: node.byte_range().into(),
            parameters: Vec::new(),
            declaring_scope,
            body_scope: constructor_scope,
            body: None,
        }),
    );
    file.lexical_scopes[constructor_scope.0].owner = Some(declaration);

    let parameters = parse_formal_parameters(node, constructor_scope, src, file);
    let body = parse_body(node, constructor_scope, src, file);
    let JavaDeclaration::Constructor(constructor) = &mut file.declarations[declaration.0] else {
        unreachable!();
    };
    constructor.parameters = parameters;
    constructor.body = body;
    Some(declaration)
}

fn parse_formal_parameters(
    node: Node,
    scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Vec<JavaDeclarationId> {
    let Some(parameters) = node.child_by_field_name("parameters") else {
        return Vec::new();
    };

    let mut result = Vec::new();
    let mut cursor = parameters.walk();
    for parameter in parameters.named_children(&mut cursor) {
        if parameter.kind() != "formal_parameter" && parameter.kind() != "spread_parameter" {
            continue;
        }
        let name = parameter
            .child_by_field_name("name")
            .and_then(|name| parse_identifier(name, src));
        let ty = parameter
            .child_by_field_name("type")
            .and_then(|ty| parse_type_ref(ty, src));
        let declaration = add_declaration(
            file,
            scope,
            JavaDeclaration::Parameter(JavaParameterDeclaration {
                span: parameter.byte_range().into(),
                name,
                ty,
                declaring_scope: scope,
            }),
        );
        result.push(declaration);
    }
    result
}

fn parse_body(
    node: Node,
    scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<crate::model::JavaBodyId> {
    let block = node.child_by_field_name("body")?;
    let mut builder = BodyBuilder::default();
    let (root, block_scope) = parse_block(block, scope, src, file, &mut builder);

    let body_id = crate::model::JavaBodyId(file.bodies.len());
    file.bodies.push(JavaBody {
        scope: block_scope,
        root,
        nodes: builder.nodes,
    });
    Some(body_id)
}

#[derive(Default)]
struct BodyBuilder {
    nodes: Vec<JavaBodyNode>,
}

impl BodyBuilder {
    fn add_statement(
        &mut self,
        statement: JavaStatement,
        span: Span,
        scope: JavaLexicalScopeId,
    ) -> JavaBodyNodeId {
        self.add(JavaBodyNodeKind::Statement(statement), span, scope)
    }

    fn add_expression(
        &mut self,
        expression: JavaExpression,
        span: Span,
        scope: JavaLexicalScopeId,
    ) -> JavaBodyNodeId {
        self.add(JavaBodyNodeKind::Expression(expression), span, scope)
    }

    fn add(
        &mut self,
        kind: JavaBodyNodeKind,
        span: Span,
        scope: JavaLexicalScopeId,
    ) -> JavaBodyNodeId {
        let id = JavaBodyNodeId(self.nodes.len());
        self.nodes.push(JavaBodyNode { span, scope, kind });
        id
    }
}

fn parse_block(
    node: Node,
    parent_scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
    builder: &mut BodyBuilder,
) -> (JavaBodyNodeId, JavaLexicalScopeId) {
    debug_assert_eq!(node.kind(), "block");

    let block_scope = new_scope(file, parent_scope, None, node.byte_range().into());
    let mut statements = Vec::new();

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "class_declaration" | "interface_declaration" | "enum_declaration"
            | "record_declaration" | "annotation_type_declaration" => {
                if let Some(declaration) =
                    parse_local_type_declaration(child, block_scope, src, file)
                {
                    statements.push(builder.add_statement(
                        JavaStatement::TypeDeclaration(declaration),
                        child.byte_range().into(),
                        block_scope,
                    ));
                }
            }
            "local_variable_declaration" => {
                parse_local_variable_declaration(
                    child,
                    block_scope,
                    src,
                    file,
                    builder,
                    &mut statements,
                );
            }
            "expression_statement" => {
                if let Some(expression) = child
                    .named_child(0)
                    .and_then(|expression| parse_expression(expression, block_scope, src, file, builder))
                {
                    statements.push(builder.add_statement(
                        JavaStatement::Expression(expression),
                        child.byte_range().into(),
                        block_scope,
                    ));
                }
            }
            "block" => {
                let (block, _) = parse_block(child, block_scope, src, file, builder);
                statements.push(block);
            }
            "return_statement" => {
                let value = child
                    .named_child(0)
                    .and_then(|expression| parse_expression(expression, block_scope, src, file, builder));
                statements.push(builder.add_statement(
                    JavaStatement::Return(value),
                    child.byte_range().into(),
                    block_scope,
                ));
            }
            _ => {}
        }
    }

    let block = builder.add_statement(
        JavaStatement::Block {
            scope: block_scope,
            statements,
        },
        node.byte_range().into(),
        parent_scope,
    );
    (block, block_scope)
}

fn parse_local_type_declaration(
    node: Node,
    scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
) -> Option<JavaDeclarationId> {
    match node.kind() {
        "class_declaration" => parse_class_declaration(node, scope, src, file),
        "interface_declaration" => parse_interface_declaration(node, scope, src, file),
        "enum_declaration" => parse_enum_declaration(node, scope, src, file),
        "record_declaration" => parse_record_declaration(node, scope, src, file),
        "annotation_type_declaration" => {
            parse_annotation_type_declaration(node, scope, src, file)
        }
        _ => None,
    }
}

fn parse_local_variable_declaration(
    node: Node,
    scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
    builder: &mut BodyBuilder,
    statements: &mut Vec<JavaBodyNodeId>,
) {
    let ty = node
        .child_by_field_name("type")
        .and_then(|ty| parse_type_ref(ty, src));

    let mut cursor = node.walk();
    for declarator in node.children_by_field_name("declarator", &mut cursor) {
        let name = declarator
            .child_by_field_name("name")
            .and_then(|name| parse_identifier(name, src));
        let declaration = add_declaration(
            file,
            scope,
            JavaDeclaration::Local(JavaLocalDeclaration {
                span: declarator.byte_range().into(),
                name,
                ty: ty.clone(),
                declaring_scope: scope,
            }),
        );
        let initializer = declarator
            .child_by_field_name("value")
            .and_then(|value| parse_expression(value, scope, src, file, builder));
        statements.push(builder.add_statement(
            JavaStatement::LocalDeclaration {
                declaration,
                initializer,
            },
            declarator.byte_range().into(),
            scope,
        ));
    }
}

fn parse_expression(
    node: Node,
    scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
    builder: &mut BodyBuilder,
) -> Option<JavaBodyNodeId> {
    let span = node.byte_range().into();
    let expression = match node.kind() {
        "identifier" => JavaExpression::NameRef {
            name: parse_identifier(node, src)?,
        },
        "this" => JavaExpression::This,
        "field_access" => {
            let receiver =
                parse_expression(node.child_by_field_name("object")?, scope, src, file, builder)?;
            let name = parse_identifier(node.child_by_field_name("field")?, src)?;
            JavaExpression::FieldAccess { receiver, name }
        }
        "method_invocation" => {
            let receiver = node
                .child_by_field_name("object")
                .and_then(|object| parse_expression(object, scope, src, file, builder));
            let name = parse_identifier(node.child_by_field_name("name")?, src)?;
            let arguments = node
                .child_by_field_name("arguments")
                .map(|arguments| parse_argument_list(arguments, scope, src, file, builder))
                .unwrap_or_default();
            JavaExpression::MethodCall {
                receiver,
                name,
                arguments,
            }
        }
        "object_creation_expression" => {
            let ty = parse_type_ref(node.child_by_field_name("type")?, src)?;
            let arguments = node
                .child_by_field_name("arguments")
                .map(|arguments| parse_argument_list(arguments, scope, src, file, builder))
                .unwrap_or_default();
            JavaExpression::ObjectCreation { ty, arguments }
        }
        "assignment_expression" => {
            let target =
                parse_expression(node.child_by_field_name("left")?, scope, src, file, builder)?;
            let value =
                parse_expression(node.child_by_field_name("right")?, scope, src, file, builder)?;
            JavaExpression::Assign { target, value }
        }
        "parenthesized_expression" => {
            return parse_expression(node.named_child(0)?, scope, src, file, builder);
        }
        "decimal_integer_literal"
        | "hex_integer_literal"
        | "octal_integer_literal"
        | "binary_integer_literal"
        | "decimal_floating_point_literal"
        | "hex_floating_point_literal"
        | "string_literal"
        | "character_literal"
        | "true"
        | "false"
        | "null_literal" => JavaExpression::Literal,
        _ => return None,
    };
    Some(builder.add_expression(expression, span, scope))
}

fn parse_argument_list(
    node: Node,
    scope: JavaLexicalScopeId,
    src: &str,
    file: &mut JavaFile,
    builder: &mut BodyBuilder,
) -> Vec<JavaBodyNodeId> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter_map(|argument| parse_expression(argument, scope, src, file, builder))
        .collect()
}

fn parse_type_ref(node: Node, src: &str) -> Option<JavaTypeRef> {
    let span = node.byte_range().into();
    match node.kind() {
        "type_identifier" => Some(JavaTypeRef {
            span,
            name: JavaName::Simple(parse_identifier(node, src)?),
            primitive: false,
        }),
        "integral_type" | "floating_point_type" | "boolean_type" | "void_type" => {
            Some(JavaTypeRef {
                span,
                name: JavaName::Simple(JavaIdentifier {
                    text: util_copy_source(node, src),
                    span,
                }),
                primitive: true,
            })
        }
        "generic_type" | "scoped_type_identifier" | "scoped_identifier" => {
            let mut segments = Vec::new();
            collect_type_segments(node, src, &mut segments);
            let name = match segments.len() {
                0 => return None,
                1 => JavaName::Simple(segments.pop().unwrap()),
                _ => JavaName::Qualified(JavaQualifiedName::new(segments, span)),
            };
            Some(JavaTypeRef {
                span,
                name,
                primitive: false,
            })
        }
        "array_type" => node
            .child_by_field_name("element")
            .and_then(|element| parse_type_ref(element, src)),
        _ => None,
    }
}

/// Segments of a possibly qualified type name, skipping type arguments:
/// `java.util.List<String>` contributes `java.util.List`.
fn collect_type_segments(node: Node, src: &str, segments: &mut Vec<JavaIdentifier>) {
    if node.kind() == "type_arguments" {
        return;
    }
    if node.kind() == "type_identifier" {
        if let Some(identifier) = parse_identifier(node, src) {
            segments.push(identifier);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_type_segments(child, src, segments);
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
        "identifier" | "type_identifier" => Some(JavaIdentifier {
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

        assert_eq!(file.top_level_declarations, [JavaDeclarationId(0)]);
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
            .top_level_declarations
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

        assert_eq!(file.top_level_declarations, [JavaDeclarationId(0)]);
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

    // The worked example from PLAN.md; offsets are load-bearing.
    const WORKED: &str = "class A {\n    int a;\n\n    void b(B c) {\n        int d = c.a;\n        this.a = d;\n        b(c);\n    }\n}\n";

    #[test]
    fn parses_the_worked_example_model() {
        let mut parser = JavaParser::new();
        let file = parser.parse(WORKED);

        // D0 class A, D1 field a, D2 method b, D3 param c, D4 local d
        assert_eq!(file.declarations.len(), 5);
        let JavaDeclaration::Type(class) = &file.declarations[0] else {
            panic!("D0 is the class");
        };
        assert_eq!(class.span, Span { start: 0, end: 102 });
        assert_eq!(class.name.as_ref().unwrap().span, Span { start: 6, end: 7 });

        let JavaDeclaration::Field(field) = &file.declarations[1] else {
            panic!("D1 is the field");
        };
        assert_eq!(
            field.name.as_ref().unwrap().span,
            Span { start: 18, end: 19 }
        );
        assert_eq!(
            field.referenced_type.as_ref().unwrap().span,
            Span { start: 14, end: 17 }
        );
        assert!(field.referenced_type.as_ref().unwrap().primitive);

        let JavaDeclaration::Method(method) = &file.declarations[2] else {
            panic!("D2 is the method");
        };
        assert_eq!(
            method.name.as_ref().unwrap().span,
            Span { start: 31, end: 32 }
        );
        assert_eq!(method.parameters, [JavaDeclarationId(3)]);
        assert!(method.body.is_some());

        let JavaDeclaration::Parameter(parameter) = &file.declarations[3] else {
            panic!("D3 is the parameter");
        };
        assert_eq!(
            parameter.name.as_ref().unwrap().span,
            Span { start: 35, end: 36 }
        );
        let param_ty = parameter.ty.as_ref().unwrap();
        assert!(!param_ty.primitive);
        assert_eq!(param_ty.span, Span { start: 33, end: 34 });

        let JavaDeclaration::Local(local) = &file.declarations[4] else {
            panic!("D4 is the local");
        };
        assert_eq!(
            local.name.as_ref().unwrap().span,
            Span { start: 52, end: 53 }
        );

        // Scopes: S0 compilation unit, S1 type body, S2 method, S3 block.
        assert_eq!(file.lexical_scopes.len(), 4);
        assert_eq!(file.lexical_scopes[1].owner, Some(JavaDeclarationId(0)));
        assert_eq!(file.lexical_scopes[2].owner, Some(JavaDeclarationId(2)));
        assert_eq!(
            file.lexical_scopes[3].span,
            Span {
                start: 38,
                end: 100
            }
        );
        assert_eq!(file.lexical_scopes[3].parent, Some(JavaLexicalScopeId(2)));

        // Body: 12 nodes — expressions and statements share one arena.
        let body = &file.bodies[0];
        assert_eq!(body.nodes.len(), 12);

        let JavaBodyNodeKind::Statement(JavaStatement::Block { statements, scope }) =
            &body.node(body.root).kind
        else {
            panic!("root is a block");
        };
        assert_eq!(*scope, JavaLexicalScopeId(3));
        assert_eq!(statements.len(), 3);

        // Every node is stamped with the scope it lives in.
        assert!(body.nodes.iter().all(|node| node.scope == JavaLexicalScopeId(3)
            || node.scope == JavaLexicalScopeId(2)));

        // N2: int d = c.a;
        let JavaBodyNodeKind::Statement(JavaStatement::LocalDeclaration {
            declaration,
            initializer: Some(initializer),
        }) = &body.nodes[2].kind
        else {
            panic!("N2 declares d with an initializer");
        };
        assert_eq!(*declaration, JavaDeclarationId(4));
        let JavaExpression::FieldAccess { receiver, name } =
            body.expression(*initializer).unwrap()
        else {
            panic!("initializer is c.a");
        };
        assert_eq!(name.span, Span { start: 58, end: 59 });
        let JavaExpression::NameRef { name } = body.expression(*receiver).unwrap() else {
            panic!("receiver is c");
        };
        assert_eq!(name.span, Span { start: 56, end: 57 });

        // N7: this.a = d;
        let JavaBodyNodeKind::Statement(JavaStatement::Expression(assign)) = &body.nodes[7].kind
        else {
            panic!("N7 is an expression statement");
        };
        let JavaExpression::Assign { target, value } = body.expression(*assign).unwrap() else {
            panic!("N7 is an assignment");
        };
        let JavaExpression::FieldAccess { receiver, name } = body.expression(*target).unwrap()
        else {
            panic!("target is this.a");
        };
        assert!(matches!(
            body.expression(*receiver),
            Some(JavaExpression::This)
        ));
        assert_eq!(name.span, Span { start: 74, end: 75 });
        let JavaExpression::NameRef { name } = body.expression(*value).unwrap() else {
            panic!("value is d");
        };
        assert_eq!(name.span, Span { start: 78, end: 79 });

        // N10: b(c);
        let JavaBodyNodeKind::Statement(JavaStatement::Expression(call)) = &body.nodes[10].kind
        else {
            panic!("N10 is an expression statement");
        };
        let JavaExpression::MethodCall {
            receiver,
            name,
            arguments,
        } = body.expression(*call).unwrap()
        else {
            panic!("N10 is a method call");
        };
        assert!(receiver.is_none());
        assert_eq!(name.span, Span { start: 89, end: 90 });
        assert_eq!(arguments.len(), 1);
    }

    #[test]
    fn worked_example_position_index_resolves_offsets() {
        let mut parser = JavaParser::new();
        let file = parser.parse(WORKED);
        let index = &file.position_index;

        use crate::model::JavaEntityId;

        // (6) the `c` in `c.a`
        let (_, entity) = index.tightest_containing(56).unwrap();
        assert!(matches!(entity, JavaEntityId::BodyNode(_, id) if id == JavaBodyNodeId(0)));
        // (7) the `a` in `c.a`
        let (_, entity) = index.tightest_containing(58).unwrap();
        assert!(matches!(entity, JavaEntityId::BodyNode(_, id) if id == JavaBodyNodeId(1)));
        // (8) this
        let (_, entity) = index.tightest_containing(70).unwrap();
        assert!(matches!(entity, JavaEntityId::BodyNode(_, id) if id == JavaBodyNodeId(3)));
        // (10) the `d` value
        let (_, entity) = index.tightest_containing(78).unwrap();
        assert!(matches!(entity, JavaEntityId::BodyNode(_, id) if id == JavaBodyNodeId(5)));
        // (11) the `b` call name
        let (_, entity) = index.tightest_containing(89).unwrap();
        assert!(matches!(entity, JavaEntityId::BodyNode(_, id) if id == JavaBodyNodeId(9)));
        // (3) the parameter type `B`
        let (_, entity) = index.tightest_containing(33).unwrap();
        assert_eq!(entity, JavaEntityId::TypeRef(JavaDeclarationId(3)));
        // (4) the parameter name `c`
        let (_, entity) = index.tightest_containing(35).unwrap();
        assert_eq!(entity, JavaEntityId::Declaration(JavaDeclarationId(3)));
        // (5) the local name `d`
        let (_, entity) = index.tightest_containing(52).unwrap();
        assert_eq!(entity, JavaEntityId::Declaration(JavaDeclarationId(4)));
        // (1) the field name `a`
        let (_, entity) = index.tightest_containing(18).unwrap();
        assert_eq!(entity, JavaEntityId::Declaration(JavaDeclarationId(1)));
    }
}
