use beans_core::model::{Offset, OffsetSpan};
use tree_sitter::{Node, Parser as TreeSitterParser};

use crate::model;

pub struct Parser {
    parser: TreeSitterParser,
}

impl Parser {
    pub fn new() -> Parser {
        let mut parser = TreeSitterParser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .expect("java grammar is compatible with the linked tree-sitter");
        Parser { parser }
    }

    pub fn parse(&mut self, contents: &str) -> model::File {
        let tree = self
            .parser
            .parse(contents, None)
            .expect("parse returns a tree when a language is set");
        parse_program(tree.root_node(), contents)
    }
}

fn parse_program(root: Node, src: &str) -> model::File {
    debug_assert_eq!(root.kind(), "program");

    let mut file = model::File::new();
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

    file.lexical_scopes[compilation_unit_scope.0].span = OffsetSpan {
        start: Offset(0),
        end: Offset(src.len()),
    };
    file.position_index = model::PositionIndex::build(&file);
    file
}

fn parse_package_declaration(node: Node, src: &str) -> Option<model::Name> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| match child.kind() {
            "identifier" | "scoped_identifier" => parse_name(child, src),
            _ => None,
        })
}

fn parse_import_declaration(node: Node, src: &str) -> Option<model::Import> {
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
        (false, false) => model::ImportKind::Type,
        (false, true) => model::ImportKind::TypeOnDemand,
        (true, false) => model::ImportKind::Static,
        (true, true) => model::ImportKind::StaticOnDemand,
    };

    Some(model::Import { name: name?, kind })
}

fn new_scope(
    file: &mut model::File,
    parent: model::LexicalScopeId,
    owner: Option<model::DeclarationId>,
    span: OffsetSpan,
) -> model::LexicalScopeId {
    let scope_id = model::LexicalScopeId(file.lexical_scopes.len());
    file.lexical_scopes.push(model::LexicalScope {
        parent: Some(parent),
        owner,
        declarations: Vec::new(),
        span,
    });
    scope_id
}

fn add_declaration(
    file: &mut model::File,
    declaring_scope: model::LexicalScopeId,
    declaration: model::Declaration,
) -> model::DeclarationId {
    let declaration_id = model::DeclarationId(file.declarations.len());
    file.declarations.push(declaration);
    file.lexical_scopes[declaring_scope.0]
        .declarations
        .push(declaration_id);
    declaration_id
}

fn parse_class_declaration(
    node: Node,
    declaring_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    add_type_declaration(node, model::TypeKind::Class, declaring_scope, src, file)
}

fn parse_interface_declaration(
    node: Node,
    declaring_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    add_type_declaration(node, model::TypeKind::Interface, declaring_scope, src, file)
}

fn parse_enum_declaration(
    node: Node,
    declaring_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    add_type_declaration(node, model::TypeKind::Enum, declaring_scope, src, file)
}

fn parse_record_declaration(
    node: Node,
    declaring_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    add_type_declaration(node, model::TypeKind::Record, declaring_scope, src, file)
}

fn parse_annotation_type_declaration(
    node: Node,
    declaring_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    add_type_declaration(
        node,
        model::TypeKind::AnnotationInterface,
        declaring_scope,
        src,
        file,
    )
}

fn parse_access(node: Node) -> Option<model::Access> {
    let implicit = implicit_access_level(node)?;
    let declared = declared_access_level(node);

    Some(model::Access {
        level: declared.map_or(implicit, |(level, _)| level),
        declared_at: declared.map(|(_, span)| span),
    })
}

/// What the position means when nothing is written.
fn implicit_access_level(node: Node) -> Option<model::AccessLevel> {
    match node.parent()?.kind() {
        // §6.6.1. A record body and an anonymous class body are both
        // `class_body`; an enum's members sit after its constants.
        "program" | "class_body" | "enum_body_declarations" => Some(model::AccessLevel::Package),
        // §9.5, which §9.6 extends to annotation interfaces.
        "interface_body" | "annotation_type_body" => Some(model::AccessLevel::Public),
        // §8.1.1: no access level here at all. A local or anonymous class is
        // reached through its scope (§6.3), never through access control.
        _ => None,
    }
}

fn declared_access_level(node: Node) -> Option<(model::AccessLevel, OffsetSpan)> {
    let mut cursor = node.walk();
    let modifiers = node
        .children(&mut cursor)
        .find(|child| child.kind() == "modifiers")?;

    let mut cursor = modifiers.walk();
    modifiers.children(&mut cursor).find_map(|child| {
        let level = match child.kind() {
            "public" => model::AccessLevel::Public,
            "protected" => model::AccessLevel::Protected,
            "private" => model::AccessLevel::Private,
            _ => return None,
        };
        Some((level, child.byte_range().into()))
    })
}

fn add_type_declaration(
    node: Node,
    kind: model::TypeKind,
    declaring_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    let name = parse_identifier(node.child_by_field_name("name")?, src)?;
    let body = node.child_by_field_name("body")?;
    let superclass = node
        .child_by_field_name("superclass")
        .and_then(|superclass| superclass.named_child(0))
        .and_then(|ty| parse_type_ref(ty, src));
    let interfaces = parse_super_interfaces(node, src);
    let body_scope = new_scope(file, declaring_scope, None, body.byte_range().into());

    let declaration = add_declaration(
        file,
        declaring_scope,
        model::Declaration::Type(model::TypeDeclaration {
            span: node.byte_range().into(),
            name: Some(name),
            kind,
            access: parse_access(node),
            superclass,
            interfaces,
            declaring_scope,
            body_scope,
        }),
    );
    file.lexical_scopes[body_scope.0].owner = Some(declaration);

    walk_type_body(body, body_scope, src, file);

    Some(declaration)
}

/// The direct superinterfaces of a declaration, which two clauses spell.
///
/// A class, enum or record writes `implements` (§8.1.5) and the grammar hands
/// it over as the `interfaces` field. An interface writes `extends` (§9.1.3)
/// for the same list, and the grammar gives it no field at all — an
/// `extends_interfaces` child sits among the modifiers. Both wrap exactly one
/// `type_list`, which is the only reason one function can answer for both.
///
/// An annotation interface reaches here and has neither, which is right: §9.6
/// gives it no supertype clause to write.
fn parse_super_interfaces(node: Node, src: &str) -> Vec<model::TypeRef> {
    let mut cursor = node.walk();
    let clause = node.child_by_field_name("interfaces").or_else(|| {
        node.named_children(&mut cursor)
            .find(|child| child.kind() == "extends_interfaces")
    });
    let Some(list) = clause.and_then(|clause| clause.named_child(0)) else {
        return Vec::new();
    };

    let mut cursor = list.walk();
    list.named_children(&mut cursor)
        .filter_map(|ty| parse_type_ref(ty, src))
        .collect()
}

fn walk_type_body(node: Node, scope: model::LexicalScopeId, src: &str, file: &mut model::File) {
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

fn parse_field_declaration(
    node: Node,
    scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) {
    let ty = node
        .child_by_field_name("type")
        .and_then(|ty| parse_type_ref(ty, src));

    // The modifiers sit on the declaration, so every declarator in
    // `int a, b;` shares one access level.
    let access = parse_access(node);

    let mut cursor = node.walk();
    for declarator in node.children_by_field_name("declarator", &mut cursor) {
        let name = declarator
            .child_by_field_name("name")
            .and_then(|name| parse_identifier(name, src));
        let extra = dimensions_of(declarator, src);
        add_declaration(
            file,
            scope,
            model::Declaration::Field(model::FieldDeclaration {
                span: declarator.byte_range().into(),
                name,
                access,
                referenced_type: ty.clone().map(|ty| with_extra_dimensions(ty, extra)),
                declaring_scope: scope,
            }),
        );
    }
}

fn parse_method_declaration(
    node: Node,
    declaring_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    let name = parse_identifier(node.child_by_field_name("name")?, src)?;
    // §10.2 lets the brackets sit after the parameter list: `int m()[]` returns
    // `int[]`. Obsolete style, still legal.
    let trailing = dimensions_of(node, src);
    let return_type = node
        .child_by_field_name("type")
        .and_then(|ty| parse_type_ref(ty, src))
        .map(|ty| with_extra_dimensions(ty, trailing));
    let method_scope = new_scope(file, declaring_scope, None, node.byte_range().into());

    // Declare before parsing the contents so declaration ids follow source order.
    let declaration = add_declaration(
        file,
        declaring_scope,
        model::Declaration::Method(model::MethodDeclaration {
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
    let model::Declaration::Method(method) = &mut file.declarations[declaration.0] else {
        unreachable!();
    };
    method.parameters = parameters;
    method.body = body;
    Some(declaration)
}

fn parse_constructor_declaration(
    node: Node,
    declaring_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    let constructor_scope = new_scope(file, declaring_scope, None, node.byte_range().into());

    let declaration = add_declaration(
        file,
        declaring_scope,
        model::Declaration::Constructor(model::ConstructorDeclaration {
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
    let model::Declaration::Constructor(constructor) = &mut file.declarations[declaration.0] else {
        unreachable!();
    };
    constructor.parameters = parameters;
    constructor.body = body;
    Some(declaration)
}

fn parse_formal_parameters(
    node: Node,
    scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Vec<model::DeclarationId> {
    let Some(parameters) = node.child_by_field_name("parameters") else {
        return Vec::new();
    };

    let mut result = Vec::new();
    let mut cursor = parameters.walk();
    for parameter in parameters.named_children(&mut cursor) {
        let (name, ty) = match parameter.kind() {
            "formal_parameter" => (
                parameter
                    .child_by_field_name("name")
                    .and_then(|name| parse_identifier(name, src)),
                parameter
                    .child_by_field_name("type")
                    .and_then(|ty| parse_type_ref(ty, src))
                    .map(|ty| with_extra_dimensions(ty, dimensions_of(parameter, src))),
            ),
            "spread_parameter" => parse_spread_parameter(parameter, src),
            _ => continue,
        };
        let declaration = add_declaration(
            file,
            scope,
            model::Declaration::Parameter(model::ParameterDeclaration {
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

/// A variable arity parameter, `String... args`.
///
/// §8.4.1 makes its declared type an array type, and §10.2 says why: "the
/// ellipsis of a variable arity parameter is treated as a bracket pair". So
/// `foo(String...)` and `foo(String[])` declare the same parameter type and are
/// override-equivalent (§8.4.2) — which is the whole reason this cannot be
/// skipped.
///
/// The grammar gives this node no fields at all. Its type is a plain child, and
/// its name lives inside a nested `variable_declarator`, which may carry
/// brackets of its own.
fn parse_spread_parameter(
    node: Node,
    src: &str,
) -> (Option<model::Identifier>, Option<model::TypeRef>) {
    let mut cursor = node.walk();
    let mut declarator = None;
    let mut ty = None;
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "variable_declarator" => declarator = Some(child),
            "modifiers" | "annotation" | "marker_annotation" => {}
            _ => ty = ty.or_else(|| parse_type_ref(child, src)),
        }
    }

    let name = declarator
        .and_then(|declarator| declarator.child_by_field_name("name"))
        .and_then(|name| parse_identifier(name, src));
    let extra = 1 + declarator.map_or(0, |declarator| dimensions_of(declarator, src));

    (name, ty.map(|ty| with_extra_dimensions(ty, extra)))
}

fn parse_body(
    node: Node,
    scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<crate::model::BodyId> {
    let block = node.child_by_field_name("body")?;
    let mut builder = BodyBuilder::default();
    let (root, block_scope) = parse_block(block, scope, src, file, &mut builder);

    let body_id = crate::model::BodyId(file.bodies.len());
    file.bodies.push(model::Body {
        scope: block_scope,
        root,
        nodes: builder.nodes,
    });
    Some(body_id)
}

#[derive(Default)]
struct BodyBuilder {
    nodes: Vec<model::BodyNode>,
}

impl BodyBuilder {
    fn add_statement(
        &mut self,
        statement: model::Statement,
        span: OffsetSpan,
        scope: model::LexicalScopeId,
    ) -> model::BodyNodeId {
        self.add(model::BodyNodeKind::Statement(statement), span, scope)
    }

    fn add_expression(
        &mut self,
        expression: model::Expression,
        span: OffsetSpan,
        scope: model::LexicalScopeId,
    ) -> model::BodyNodeId {
        self.add(model::BodyNodeKind::Expression(expression), span, scope)
    }

    fn add(
        &mut self,
        kind: model::BodyNodeKind,
        span: OffsetSpan,
        scope: model::LexicalScopeId,
    ) -> model::BodyNodeId {
        let id = model::BodyNodeId(self.nodes.len());
        self.nodes.push(model::BodyNode { span, scope, kind });
        id
    }
}

fn parse_block(
    node: Node,
    parent_scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
    builder: &mut BodyBuilder,
) -> (model::BodyNodeId, model::LexicalScopeId) {
    debug_assert!(
        matches!(node.kind(), "block" | "constructor_body"),
        "expected block or constructor_body, got {}",
        node.kind(),
    );

    let block_scope = new_scope(file, parent_scope, None, node.byte_range().into());
    let mut statements = Vec::new();

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration" => {
                if let Some(declaration) =
                    parse_local_type_declaration(child, block_scope, src, file)
                {
                    statements.push(builder.add_statement(
                        model::Statement::TypeDeclaration(declaration),
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
                if let Some(expression) = child.named_child(0).and_then(|expression| {
                    parse_expression(expression, block_scope, src, file, builder)
                }) {
                    statements.push(builder.add_statement(
                        model::Statement::Expression(expression),
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
                let value = child.named_child(0).and_then(|expression| {
                    parse_expression(expression, block_scope, src, file, builder)
                });
                statements.push(builder.add_statement(
                    model::Statement::Return(value),
                    child.byte_range().into(),
                    block_scope,
                ));
            }
            _ => {}
        }
    }

    let block = builder.add_statement(
        model::Statement::Block {
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
    scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
) -> Option<model::DeclarationId> {
    match node.kind() {
        "class_declaration" => parse_class_declaration(node, scope, src, file),
        "interface_declaration" => parse_interface_declaration(node, scope, src, file),
        "enum_declaration" => parse_enum_declaration(node, scope, src, file),
        "record_declaration" => parse_record_declaration(node, scope, src, file),
        "annotation_type_declaration" => parse_annotation_type_declaration(node, scope, src, file),
        _ => None,
    }
}

fn parse_local_variable_declaration(
    node: Node,
    scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
    builder: &mut BodyBuilder,
    statements: &mut Vec<model::BodyNodeId>,
) {
    let ty = node
        .child_by_field_name("type")
        .and_then(|ty| parse_type_ref(ty, src));

    let mut cursor = node.walk();
    for declarator in node.children_by_field_name("declarator", &mut cursor) {
        let name = declarator
            .child_by_field_name("name")
            .and_then(|name| parse_identifier(name, src));
        let extra = dimensions_of(declarator, src);
        let declaration = add_declaration(
            file,
            scope,
            model::Declaration::Local(model::LocalDeclaration {
                span: declarator.byte_range().into(),
                name,
                ty: ty.clone().map(|ty| with_extra_dimensions(ty, extra)),
                declaring_scope: scope,
            }),
        );
        let initializer = declarator
            .child_by_field_name("value")
            .and_then(|value| parse_expression(value, scope, src, file, builder));
        statements.push(builder.add_statement(
            model::Statement::LocalDeclaration {
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
    scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
    builder: &mut BodyBuilder,
) -> Option<model::BodyNodeId> {
    let span = node.byte_range().into();
    let expression = match node.kind() {
        "identifier" => model::Expression::NameRef {
            name: parse_identifier(node, src)?,
        },
        "this" => model::Expression::This,
        "field_access" => {
            let receiver = parse_expression(
                node.child_by_field_name("object")?,
                scope,
                src,
                file,
                builder,
            )?;
            let name = parse_identifier(node.child_by_field_name("field")?, src)?;
            model::Expression::FieldAccess { receiver, name }
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
            model::Expression::MethodCall {
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
            model::Expression::ObjectCreation { ty, arguments }
        }
        "assignment_expression" => {
            let target =
                parse_expression(node.child_by_field_name("left")?, scope, src, file, builder)?;
            let value = parse_expression(
                node.child_by_field_name("right")?,
                scope,
                src,
                file,
                builder,
            )?;
            model::Expression::Assign { target, value }
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
        | "null_literal" => model::Expression::Literal,
        _ => return None,
    };
    Some(builder.add_expression(expression, span, scope))
}

fn parse_argument_list(
    node: Node,
    scope: model::LexicalScopeId,
    src: &str,
    file: &mut model::File,
    builder: &mut BodyBuilder,
) -> Vec<model::BodyNodeId> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter_map(|argument| parse_expression(argument, scope, src, file, builder))
        .collect()
}

fn parse_type_ref(node: Node, src: &str) -> Option<model::TypeRef> {
    Some(model::TypeRef {
        span: node.byte_range().into(),
        ty: parse_type(node, src)?,
    })
}

fn parse_type(node: Node, src: &str) -> Option<model::Type> {
    match node.kind() {
        "type_identifier" => Some(model::Type::Named(model::Name::Simple(parse_identifier(
            node, src,
        )?))),
        // §8.4.5: not a Type, and only a method result.
        "void_type" => Some(model::Type::Void),
        "integral_type" | "floating_point_type" | "boolean_type" => {
            model::Primitive::from_keyword(&util_copy_source(node, src)).map(model::Type::Primitive)
        }
        "generic_type" | "scoped_type_identifier" | "scoped_identifier" => {
            let span = node.byte_range().into();
            let mut segments = Vec::new();
            collect_type_segments(node, src, &mut segments);
            let name = match segments.len() {
                0 => return None,
                1 => model::Name::Simple(segments.pop().unwrap()),
                _ => model::Name::Qualified(model::QualifiedName::new(segments, span)),
            };
            Some(model::Type::Named(name))
        }
        "array_type" => {
            let element = parse_type(node.child_by_field_name("element")?, src)?;
            Some(arrayed(element, dimensions_of(node, src)))
        }
        _ => None,
    }
}

/// §10.1's bracket pairs. A `dimensions` node holds all of them at once and may
/// carry an annotation before each (§9.7.4), so the brackets are counted rather
/// than the children.
fn dimensions_of(node: Node, src: &str) -> usize {
    node.child_by_field_name("dimensions")
        .map_or(0, |dims| src[dims.byte_range()].matches('[').count())
}

fn arrayed(ty: model::Type, dimensions: usize) -> model::Type {
    (0..dimensions).fold(ty, |ty, _| model::Type::Array(Box::new(ty)))
}

/// §10.2: the array type of a variable is its element type, then the bracket
/// pairs following its identifier in the declarator, then the bracket pairs on
/// the type at the head of the declaration. A method composes the same way with
/// the pairs after its parameter list.
///
/// Which bracket contributed which level cannot be observed — every level of an
/// array is the same thing — so they simply add. What matters is that they are
/// counted per *declarator*: §10.2's `short s, aas[][]` declares a `short` and a
/// `short[][]` from one type.
fn with_extra_dimensions(type_ref: model::TypeRef, extra: usize) -> model::TypeRef {
    model::TypeRef {
        span: type_ref.span,
        ty: arrayed(type_ref.ty, extra),
    }
}

/// Segments of a possibly qualified type name, skipping type arguments:
/// `java.util.List<String>` contributes `java.util.List`.
fn collect_type_segments(node: Node, src: &str, segments: &mut Vec<model::Identifier>) {
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

fn parse_name(node: Node, src: &str) -> Option<model::Name> {
    match node.kind() {
        "identifier" => Some(model::Name::Simple(parse_identifier(node, src)?)),
        "scoped_identifier" => Some(model::Name::Qualified(parse_scoped_identifier(node, src)?)),
        kind => panic!("uncovered name node kind: {kind}"),
    }
}

fn parse_scoped_identifier(node: Node, src: &str) -> Option<model::QualifiedName> {
    let mut identifiers = Vec::new();
    collect_scoped_identifier(node, src, &mut identifiers)?;
    Some(model::QualifiedName::new(
        identifiers,
        node.byte_range().into(),
    ))
}

fn collect_scoped_identifier(
    node: Node,
    src: &str,
    identifiers: &mut Vec<model::Identifier>,
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

fn parse_identifier(node: Node, src: &str) -> Option<model::Identifier> {
    match node.kind() {
        "identifier" | "type_identifier" => Some(model::Identifier {
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

    fn type_declaration(file: &model::File, id: model::DeclarationId) -> &model::TypeDeclaration {
        let model::Declaration::Type(declaration) = &file.declarations[id.0] else {
            panic!("expected a type declaration");
        };
        declaration
    }

    #[test]
    fn parses_compilation_unit_declarations() {
        let content = "package org.beans.test;\nimport java.util.List;\nclass Foo {}\n";
        let mut parser = Parser::new();
        let file = parser.parse(content);

        assert_eq!(
            file.package.as_ref().map(model::Name::dotted),
            Some("org.beans.test".to_string())
        );
        assert!(matches!(&file.package, Some(model::Name::Qualified(_))));
        assert_eq!(file.imports.len(), 1);
        assert_eq!(file.imports[0].name.dotted(), "java.util.List");
        assert!(matches!(&file.imports[0].name, model::Name::Qualified(_)));
        assert_eq!(file.imports[0].kind, model::ImportKind::Type);

        assert_eq!(file.top_level_declarations, [model::DeclarationId(0)]);
        assert_eq!(
            file.lexical_scopes[file.compilation_unit_scope.0].declarations,
            [model::DeclarationId(0)]
        );

        let declaration = type_declaration(&file, model::DeclarationId(0));
        assert_eq!(
            declaration.name.as_ref().map(|name| name.text.as_str()),
            Some("Foo")
        );
        assert_eq!(declaration.kind, model::TypeKind::Class);
        assert_eq!(declaration.declaring_scope, file.compilation_unit_scope);
        assert_eq!(
            file.lexical_scopes[declaration.body_scope.0].parent,
            Some(file.compilation_unit_scope)
        );
    }

    #[test]
    fn parses_a_single_identifier_as_a_simple_name() {
        let mut parser = Parser::new();
        let file = parser.parse("package example; class Example {}");

        let Some(model::Name::Simple(identifier)) = file.package else {
            panic!("expected a simple package name");
        };
        assert_eq!(identifier.text, "example");
    }

    #[test]
    fn parses_each_named_type_kind() {
        let content = "class C {} interface I {} enum E {} record R() {} @interface A {}";
        let mut parser = Parser::new();
        let file = parser.parse(content);

        let kinds: Vec<_> = file
            .top_level_declarations
            .iter()
            .map(|id| type_declaration(&file, *id).kind)
            .collect();
        assert_eq!(
            kinds,
            [
                model::TypeKind::Class,
                model::TypeKind::Interface,
                model::TypeKind::Enum,
                model::TypeKind::Record,
                model::TypeKind::AnnotationInterface,
            ]
        );
    }

    // JLS 26 §8.1.4 gives a class its `extends` clause and §8.1.5 its
    // `implements`; §9.1.3 spells an interface's superinterfaces with
    // `extends`, which is the same list under the other keyword. The model
    // keeps the two clauses apart, so each case below asks which half a
    // spelling landed in.
    mod supertypes {
        use super::*;

        fn supertypes_of(
            file: &model::File,
            id: model::DeclarationId,
        ) -> (Option<String>, Vec<String>) {
            let declaration = type_declaration(file, id);
            (
                declaration.superclass.as_ref().map(|ty| ty.ty.to_string()),
                declaration
                    .interfaces
                    .iter()
                    .map(|ty| ty.ty.to_string())
                    .collect(),
            )
        }

        #[test]
        fn a_class_keeps_its_superclass_apart_from_its_interfaces() {
            let mut parser = Parser::new();
            let file = parser.parse("class C extends B implements A, D {}");

            assert_eq!(
                supertypes_of(&file, model::DeclarationId(0)),
                (
                    Some("B".to_string()),
                    vec!["A".to_string(), "D".to_string()]
                )
            );
        }

        /// §9.1.3: an interface `extends` its superinterfaces, so the keyword
        /// says superclass and the meaning says interface. The grammar agrees —
        /// it gives `interface_declaration` no `superclass` field at all — and
        /// this is the case that would break if we read one.
        #[test]
        fn an_interface_extends_into_the_interface_list() {
            let mut parser = Parser::new();
            let file = parser.parse("interface I extends A, B {}");

            assert_eq!(
                supertypes_of(&file, model::DeclarationId(0)),
                (None, vec!["A".to_string(), "B".to_string()])
            );
        }

        /// §8.1.4 gives an enum `Enum` and a record `Record` implicitly, so
        /// neither may write `extends` and both may write `implements`.
        #[test]
        fn an_enum_and_a_record_implement_without_extending() {
            let mut parser = Parser::new();
            let file = parser.parse("enum E implements A {} record R() implements A {}");

            for id in [model::DeclarationId(0), model::DeclarationId(1)] {
                assert_eq!(supertypes_of(&file, id), (None, vec!["A".to_string()]));
            }
        }

        /// Nothing written is no supertype, not an implicit one. §8.1.4's
        /// `Object` and §9.2's borrowed `Object` methods are resolution's to
        /// supply, and an annotation interface (§9.6) has no clause to write.
        #[test]
        fn a_declaration_with_no_clause_names_no_supertype() {
            let mut parser = Parser::new();
            let file = parser.parse("class C {} interface I {} @interface A {}");

            for id in 0..3 {
                assert_eq!(
                    supertypes_of(&file, model::DeclarationId(id)),
                    (None, Vec::new())
                );
            }
        }

        /// §8.2 takes members from the direct superclass before the direct
        /// superinterfaces, and this iterator is where that order is stated.
        #[test]
        fn the_superclass_is_enumerated_before_the_interfaces() {
            let mut parser = Parser::new();
            let file = parser.parse("class C extends B implements A, D {}");

            let enumerated: Vec<_> = type_declaration(&file, model::DeclarationId(0))
                .supertypes()
                .map(|(id, ty)| (id, ty.ty.to_string()))
                .collect();

            assert_eq!(
                enumerated,
                [
                    (model::SupertypeId::Superclass, "B".to_string()),
                    (model::SupertypeId::Interface(0), "A".to_string()),
                    (model::SupertypeId::Interface(1), "D".to_string()),
                ]
            );
        }

        /// A supertype is a type reference like any other, so `TypeRef` keeps
        /// the erased head and drops the type arguments — the same erasure
        /// `jvm::model::Type` commits to. Nothing pinned that anywhere before,
        /// and the supertype clauses are now a second place relying on it.
        #[test]
        fn a_supertype_keeps_its_erased_head() {
            let mut parser = Parser::new();
            let file = parser
                .parse("class C<T> extends Base<T> implements Plain, java.util.List<String> {}");

            assert_eq!(
                supertypes_of(&file, model::DeclarationId(0)),
                (
                    Some("Base".to_string()),
                    vec!["Plain".to_string(), "java.util.List".to_string()]
                )
            );
        }

        /// Two names in one clause have to be told apart: a caret in `A` and a
        /// caret in `D` are different references, and `EntityId::TypeRef` names
        /// only the declaration that owns them.
        #[test]
        fn a_caret_lands_on_the_supertype_it_is_inside() {
            let content = "class C extends B implements A, D {}";
            let mut parser = Parser::new();
            let file = parser.parse(content);

            let at = |needle: &str| {
                let offset = Offset(content.find(needle).expect("spelled once"));
                file.position_index.tightest_containing(offset).unwrap().1
            };

            let owner = model::DeclarationId(0);
            assert_eq!(
                at("B"),
                model::EntityId::Supertype(owner, model::SupertypeId::Superclass)
            );
            assert_eq!(
                at("A"),
                model::EntityId::Supertype(owner, model::SupertypeId::Interface(0))
            );
            assert_eq!(
                at("D"),
                model::EntityId::Supertype(owner, model::SupertypeId::Interface(1))
            );
        }
    }

    // JLS 26 §10.2 builds one type out of up to three places: the element type
    // at the head of the declaration, the bracket pairs after the declarator's
    // identifier, and the bracket pairs on the type. Each case below writes the
    // brackets somewhere different and asks what type came out.
    mod types {
        use super::*;

        /// Every declaration that names a type, rendered as a reader sees it.
        fn written(file: &model::File) -> Vec<(String, String)> {
            file.declarations
                .iter()
                .filter_map(|declaration| {
                    Some((
                        declaration.name()?.text.clone(),
                        declaration.type_ref()?.ty.to_string(),
                    ))
                })
                .collect()
        }

        fn parse(src: &str) -> model::File {
            Parser::new().parse(src)
        }

        #[test]
        fn brackets_on_the_type_are_kept() {
            assert_eq!(
                written(&parse("class A { int[][] f; }")),
                [("f".to_string(), "int[][]".to_string())]
            );
        }

        /// §10.2's second source: "any bracket pairs that follow the variable's
        /// *Identifier* in the declarator".
        #[test]
        fn brackets_after_the_identifier_are_kept() {
            assert_eq!(
                written(&parse("class A { int f[]; }")),
                [("f".to_string(), "int[]".to_string())]
            );
        }

        /// Mixed notation, which §10.2 permits and advises against.
        #[test]
        fn brackets_on_both_sides_add_up() {
            assert_eq!(
                written(&parse("class A { int[] f[]; }")),
                [("f".to_string(), "int[][]".to_string())]
            );
        }

        /// §10.2's own Example 10.2-1: one declaration, one element type, two
        /// declarators, two different types. This is why the brackets belong to
        /// the declarator and not to the declaration.
        #[test]
        fn one_declaration_can_declare_two_different_types() {
            assert_eq!(
                written(&parse("class A { short s, aas[][]; }")),
                [
                    ("s".to_string(), "short".to_string()),
                    ("aas".to_string(), "short[][]".to_string()),
                ]
            );
        }

        /// §10.2 again: "the element type in the *Result*, then any bracket
        /// pairs that follow the formal parameter list". Obsolete style, legal.
        #[test]
        fn a_method_may_write_its_brackets_after_the_parameter_list() {
            assert_eq!(
                written(&parse("class A { int m()[] { return null; } }")),
                [("m".to_string(), "int[]".to_string())]
            );
        }

        /// §8.4.1 makes a variable arity parameter's declared type an array
        /// type, and §10.2 says why: the ellipsis "is treated as a bracket
        /// pair". So this and `String[]` are the same parameter type, which
        /// §8.4.2 needs in order to call them override-equivalent.
        #[test]
        fn a_variable_arity_parameter_is_an_array() {
            assert_eq!(
                written(&parse("class A { void v(String... args) {} }")),
                [
                    ("v".to_string(), "void".to_string()),
                    ("args".to_string(), "String[]".to_string()),
                ]
            );
            assert_eq!(
                written(&parse("class A { void v(String[] args) {} }")),
                [
                    ("v".to_string(), "void".to_string()),
                    ("args".to_string(), "String[]".to_string()),
                ]
            );
        }

        /// §4.2 lists eight primitives and `void` is not among them; §8.4.5
        /// makes it a *Result* and nothing else. The model says so rather than
        /// carrying a flag beside a `Name` holding a keyword (§3.9).
        #[test]
        fn void_is_its_own_thing() {
            let file = parse("class A { void m() {} int n() { return 0; } }");
            let types: Vec<_> = file
                .declarations
                .iter()
                .filter_map(|declaration| declaration.type_ref().map(|ty| ty.ty.clone()))
                .collect();

            assert_eq!(
                types,
                [
                    model::Type::Void,
                    model::Type::Primitive(model::Primitive::Int),
                ]
            );
        }

        /// §4.6 erases type arguments and the lake holds erased types, so the
        /// array wrapping survives and the arguments do not.
        #[test]
        fn a_generic_array_keeps_the_brackets_and_drops_the_arguments() {
            assert_eq!(
                written(&parse("class A { java.util.List<String>[] g; }")),
                [("g".to_string(), "java.util.List[]".to_string())]
            );
        }
    }

    // JLS 26 §6.6.1 gives four levels and no fifth for "nothing was written".
    // What absence means is decided by the position, so each case below fixes a
    // position and asks what the parser resolved it to.
    mod access {
        use super::*;

        fn access_of(file: &model::File, name: &str) -> Option<model::Access> {
            file.declarations
                .iter()
                .find_map(|declaration| match declaration {
                    model::Declaration::Type(ty)
                        if ty.name.as_ref().is_some_and(|it| it.text == name) =>
                    {
                        Some(ty.access)
                    }
                    _ => None,
                })
                .expect("the fixture declares this type")
        }

        fn package_private(file: &model::File, name: &str) {
            assert_eq!(
                access_of(file, name),
                Some(model::Access {
                    level: model::AccessLevel::Package,
                    declared_at: None,
                })
            );
        }

        fn implicitly_public(file: &model::File, name: &str) {
            assert_eq!(
                access_of(file, name),
                Some(model::Access {
                    level: model::AccessLevel::Public,
                    declared_at: None,
                })
            );
        }

        #[test]
        fn a_top_level_type_without_a_modifier_is_package_private() {
            let mut parser = Parser::new();
            let file = parser.parse("class C {}");

            package_private(&file, "C");
        }

        // A record body and an anonymous class body are both `class_body`, and
        // an enum's members sit in `enum_body_declarations` after the constants,
        // so each container is its own case even though all four are classes.
        #[test]
        fn a_member_of_any_class_without_a_modifier_is_package_private() {
            let mut parser = Parser::new();
            let file = parser.parse(
                "class C { class InClass {} }\
                 record R() { class InRecord {} }\
                 enum E { A; class InEnum {} }",
            );

            package_private(&file, "InClass");
            package_private(&file, "InRecord");
            package_private(&file, "InEnum");
        }

        // §9.5: every member class or interface in an interface body is
        // implicitly public, and §9.6 hands the same rule to annotation
        // interfaces. The same absence means the opposite of the case above,
        // which is why the position has to be part of the answer.
        #[test]
        fn a_member_of_any_interface_without_a_modifier_is_public() {
            let mut parser = Parser::new();
            let file = parser.parse(
                "interface I { class InInterface {} }\
                 @interface A { class InAnnotation {} }",
            );

            implicitly_public(&file, "InInterface");
            implicitly_public(&file, "InAnnotation");
        }

        // §8.1.1: public pertains only to top level and member classes, and
        // protected and private only to member classes. A local class has no
        // level to carry; it is reached through its scope or not at all.
        #[test]
        fn a_local_class_has_no_access_level() {
            let mut parser = Parser::new();
            let file = parser.parse("class C { void m() { class L {} } }");

            assert_eq!(access_of(&file, "L"), None);
        }

        #[test]
        fn every_level_is_recognised_where_it_is_legal() {
            let mut parser = Parser::new();
            let file = parser.parse(
                "class C { public class Pub {} protected class Prot {} private class Priv {} }",
            );

            let levels = ["Pub", "Prot", "Priv"]
                .map(|name| access_of(&file, name).map(|access| access.level));
            assert_eq!(
                levels,
                [
                    Some(model::AccessLevel::Public),
                    Some(model::AccessLevel::Protected),
                    Some(model::AccessLevel::Private),
                ]
            );
        }

        #[test]
        fn an_explicit_modifier_records_where_it_was_written() {
            let mut parser = Parser::new();
            let file = parser.parse("public class C {}");

            assert_eq!(
                access_of(&file, "C"),
                Some(model::Access {
                    level: model::AccessLevel::Public,
                    declared_at: Some(OffsetSpan {
                        start: Offset(0),
                        end: Offset(6),
                    }),
                })
            );
        }

        // §9.5 permits redundantly writing what the position already implies,
        // so the level is unchanged and only the provenance differs.
        #[test]
        fn a_redundant_modifier_is_still_recorded_as_written() {
            let mut parser = Parser::new();
            let file = parser.parse("interface I { public class Inner {} }");

            assert_eq!(
                access_of(&file, "Inner"),
                Some(model::Access {
                    level: model::AccessLevel::Public,
                    declared_at: Some(OffsetSpan {
                        start: Offset(14),
                        end: Offset(20),
                    }),
                })
            );
        }
    }

    #[test]
    fn recursively_parses_member_types() {
        let content = "class Outer { class Member { interface Deep {} } }";
        let mut parser = Parser::new();
        let file = parser.parse(content);

        assert_eq!(file.top_level_declarations, [model::DeclarationId(0)]);
        assert_eq!(file.declarations.len(), 3);

        let outer = type_declaration(&file, model::DeclarationId(0));
        let member = type_declaration(&file, model::DeclarationId(1));
        let deep = type_declaration(&file, model::DeclarationId(2));

        assert_eq!(
            file.lexical_scopes[outer.body_scope.0].declarations,
            [model::DeclarationId(1)]
        );
        assert_eq!(member.declaring_scope, outer.body_scope);
        assert_eq!(
            file.lexical_scopes[member.body_scope.0].declarations,
            [model::DeclarationId(2)]
        );
        assert_eq!(deep.declaring_scope, member.body_scope);
    }

    // The worked example from PLAN.md; offsets are load-bearing.
    const WORKED: &str = "class A {\n    int a;\n\n    void b(B c) {\n        int d = c.a;\n        this.a = d;\n        b(c);\n    }\n}\n";

    #[test]
    fn parses_the_worked_example_model() {
        let mut parser = Parser::new();
        let file = parser.parse(WORKED);

        // D0 class A, D1 field a, D2 method b, D3 param c, D4 local d
        assert_eq!(file.declarations.len(), 5);
        let model::Declaration::Type(class) = &file.declarations[0] else {
            panic!("D0 is the class");
        };
        assert_eq!(
            class.span,
            OffsetSpan {
                start: Offset(0),
                end: Offset(102)
            }
        );
        assert_eq!(
            class.name.as_ref().unwrap().span,
            OffsetSpan {
                start: Offset(6),
                end: Offset(7)
            }
        );

        let model::Declaration::Field(field) = &file.declarations[1] else {
            panic!("D1 is the field");
        };
        assert_eq!(
            field.name.as_ref().unwrap().span,
            OffsetSpan {
                start: Offset(18),
                end: Offset(19)
            }
        );
        assert_eq!(
            field.referenced_type.as_ref().unwrap().span,
            OffsetSpan {
                start: Offset(14),
                end: Offset(17)
            }
        );
        assert!(matches!(
            field.referenced_type.as_ref().unwrap().ty,
            model::Type::Primitive(model::Primitive::Int)
        ));

        let model::Declaration::Method(method) = &file.declarations[2] else {
            panic!("D2 is the method");
        };
        assert_eq!(
            method.name.as_ref().unwrap().span,
            OffsetSpan {
                start: Offset(31),
                end: Offset(32)
            }
        );
        assert_eq!(method.parameters, [model::DeclarationId(3)]);
        assert!(method.body.is_some());

        let model::Declaration::Parameter(parameter) = &file.declarations[3] else {
            panic!("D3 is the parameter");
        };
        assert_eq!(
            parameter.name.as_ref().unwrap().span,
            OffsetSpan {
                start: Offset(35),
                end: Offset(36)
            }
        );
        let param_ty = parameter.ty.as_ref().unwrap();
        assert!(matches!(param_ty.ty, model::Type::Named(_)));
        assert_eq!(
            param_ty.span,
            OffsetSpan {
                start: Offset(33),
                end: Offset(34)
            }
        );

        let model::Declaration::Local(local) = &file.declarations[4] else {
            panic!("D4 is the local");
        };
        assert_eq!(
            local.name.as_ref().unwrap().span,
            OffsetSpan {
                start: Offset(52),
                end: Offset(53)
            }
        );

        // Scopes: S0 compilation unit, S1 type body, S2 method, S3 block.
        assert_eq!(file.lexical_scopes.len(), 4);
        assert_eq!(file.lexical_scopes[1].owner, Some(model::DeclarationId(0)));
        assert_eq!(file.lexical_scopes[2].owner, Some(model::DeclarationId(2)));
        assert_eq!(
            file.lexical_scopes[3].span,
            OffsetSpan {
                start: Offset(38),
                end: Offset(100),
            }
        );
        assert_eq!(
            file.lexical_scopes[3].parent,
            Some(model::LexicalScopeId(2))
        );

        // model::Body: 12 nodes — expressions and statements share one arena.
        let body = &file.bodies[0];
        assert_eq!(body.nodes.len(), 12);

        let model::BodyNodeKind::Statement(model::Statement::Block { statements, scope }) =
            &body.node(body.root).kind
        else {
            panic!("root is a block");
        };
        assert_eq!(*scope, model::LexicalScopeId(3));
        assert_eq!(statements.len(), 3);

        // Every node is stamped with the scope it lives in.
        assert!(
            body.nodes
                .iter()
                .all(|node| node.scope == model::LexicalScopeId(3)
                    || node.scope == model::LexicalScopeId(2))
        );

        // N2: int d = c.a;
        let model::BodyNodeKind::Statement(model::Statement::LocalDeclaration {
            declaration,
            initializer: Some(initializer),
        }) = &body.nodes[2].kind
        else {
            panic!("N2 declares d with an initializer");
        };
        assert_eq!(*declaration, model::DeclarationId(4));
        let model::Expression::FieldAccess { receiver, name } =
            body.expression(*initializer).unwrap()
        else {
            panic!("initializer is c.a");
        };
        assert_eq!(
            name.span,
            OffsetSpan {
                start: Offset(58),
                end: Offset(59)
            }
        );
        let model::Expression::NameRef { name } = body.expression(*receiver).unwrap() else {
            panic!("receiver is c");
        };
        assert_eq!(
            name.span,
            OffsetSpan {
                start: Offset(56),
                end: Offset(57)
            }
        );

        // N7: this.a = d;
        let model::BodyNodeKind::Statement(model::Statement::Expression(assign)) =
            &body.nodes[7].kind
        else {
            panic!("N7 is an expression statement");
        };
        let model::Expression::Assign { target, value } = body.expression(*assign).unwrap() else {
            panic!("N7 is an assignment");
        };
        let model::Expression::FieldAccess { receiver, name } = body.expression(*target).unwrap()
        else {
            panic!("target is this.a");
        };
        assert!(matches!(
            body.expression(*receiver),
            Some(model::Expression::This)
        ));
        assert_eq!(
            name.span,
            OffsetSpan {
                start: Offset(74),
                end: Offset(75)
            }
        );
        let model::Expression::NameRef { name } = body.expression(*value).unwrap() else {
            panic!("value is d");
        };
        assert_eq!(
            name.span,
            OffsetSpan {
                start: Offset(78),
                end: Offset(79)
            }
        );

        // N10: b(c);
        let model::BodyNodeKind::Statement(model::Statement::Expression(call)) =
            &body.nodes[10].kind
        else {
            panic!("N10 is an expression statement");
        };
        let model::Expression::MethodCall {
            receiver,
            name,
            arguments,
        } = body.expression(*call).unwrap()
        else {
            panic!("N10 is a method call");
        };
        assert!(receiver.is_none());
        assert_eq!(
            name.span,
            OffsetSpan {
                start: Offset(89),
                end: Offset(90)
            }
        );
        assert_eq!(arguments.len(), 1);
    }

    #[test]
    fn worked_example_position_index_resolves_offsets() {
        let mut parser = Parser::new();
        let file = parser.parse(WORKED);
        let index = &file.position_index;

        // (6) the `c` in `c.a`
        let (_, entity) = index.tightest_containing(Offset(56)).unwrap();
        assert!(matches!(entity, model::EntityId::BodyNode(_, id) if id == model::BodyNodeId(0)));
        // (7) the `a` in `c.a`
        let (_, entity) = index.tightest_containing(Offset(58)).unwrap();
        assert!(matches!(entity, model::EntityId::BodyNode(_, id) if id == model::BodyNodeId(1)));
        // (8) this
        let (_, entity) = index.tightest_containing(Offset(70)).unwrap();
        assert!(matches!(entity, model::EntityId::BodyNode(_, id) if id == model::BodyNodeId(3)));
        // (10) the `d` value
        let (_, entity) = index.tightest_containing(Offset(78)).unwrap();
        assert!(matches!(entity, model::EntityId::BodyNode(_, id) if id == model::BodyNodeId(5)));
        // (11) the `b` call name
        let (_, entity) = index.tightest_containing(Offset(89)).unwrap();
        assert!(matches!(entity, model::EntityId::BodyNode(_, id) if id == model::BodyNodeId(9)));
        // (3) the parameter type `B`
        let (_, entity) = index.tightest_containing(Offset(33)).unwrap();
        assert_eq!(entity, model::EntityId::TypeRef(model::DeclarationId(3)));
        // (4) the parameter name `c`
        let (_, entity) = index.tightest_containing(Offset(35)).unwrap();
        assert_eq!(
            entity,
            model::EntityId::Declaration(model::DeclarationId(3))
        );
        // (5) the local name `d`
        let (_, entity) = index.tightest_containing(Offset(52)).unwrap();
        assert_eq!(
            entity,
            model::EntityId::Declaration(model::DeclarationId(4))
        );
        // (1) the field name `a`
        let (_, entity) = index.tightest_containing(Offset(18)).unwrap();
        assert_eq!(
            entity,
            model::EntityId::Declaration(model::DeclarationId(1))
        );
    }

    #[test]
    fn parses_a_constructor_body() {
        // tree-sitter-java models a constructor's body as `constructor_body`,
        // not `block`. The parser must still walk its statements so references
        // inside a constructor resolve.
        let mut parser = Parser::new();
        let file =
            parser.parse("class A {\n    int a;\n    A(int c) {\n        this.a = c;\n    }\n}\n");

        // D0 class A, D1 field a, D2 constructor A, D3 param c.
        let model::Declaration::Constructor(constructor) = &file.declarations[2] else {
            panic!("D2 is the constructor");
        };
        assert_eq!(constructor.parameters, [model::DeclarationId(3)]);
        let body_id = constructor.body.expect("the constructor has a body");

        let body = &file.bodies[body_id.0];
        let model::BodyNodeKind::Statement(model::Statement::Block { statements, .. }) =
            &body.node(body.root).kind
        else {
            panic!("root is a block");
        };
        assert_eq!(statements.len(), 1);

        // `this.a = c;` inside the constructor body parses into an assignment.
        let model::BodyNodeKind::Statement(model::Statement::Expression(assign)) =
            &body.node(statements[0]).kind
        else {
            panic!("the sole statement is an expression statement");
        };
        let model::Expression::Assign { target, value } = body.expression(*assign).unwrap() else {
            panic!("the expression is an assignment");
        };
        let model::Expression::FieldAccess { receiver, name } = body.expression(*target).unwrap()
        else {
            panic!("the target is this.a");
        };
        assert!(matches!(
            body.expression(*receiver),
            Some(model::Expression::This)
        ));
        assert_eq!(name.text, "a");
        let model::Expression::NameRef { name } = body.expression(*value).unwrap() else {
            panic!("the value is the parameter c");
        };
        assert_eq!(name.text, "c");
    }
}
