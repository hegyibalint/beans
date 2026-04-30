//! Tree-sitter walker for Java source files.
//!
//! Per ADR-0021 the walker structure (which tree-sitter nodes it visits,
//! which fields it reads) is preserved verbatim from the prototype
//! parser; the emit shape changed in step 7 of the graph migration. The
//! walker now pushes a `PendingNode` pair — one Java payload plus a
//! hard-linked JVM projection per declaration — directly into a
//! [`ParsedJavaFile`]'s plan, which `integrate` later inserts into the
//! graph and registers against `Registries`.
//!
//! Per ADR-0005 the parse phase produces a self-contained
//! [`ParsedJavaFile`] (verified `Send` per the static check at the
//! bottom of this file) so it can run on a rayon worker; integration
//! is serial on the graph thread.

use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser};

use crate::graph::NodeBehavior;
use crate::graph::arena::{Graph, NodeId};
use crate::jvm::fqn::Fqn;
use crate::jvm::payload::{
    JvmConstructorNode, JvmDeclHeader, JvmEnrichments, JvmEnumConstantNode, JvmFieldNode,
    JvmMethodNode, JvmNodePayload, JvmParameter, JvmTypeKind, JvmTypeNode,
};
use crate::languages::java::payload::{
    JavaConstructorNode, JavaDeclHeader, JavaEnumConstantNode, JavaFieldNode, JavaMethodNode,
    JavaNodePayload, JavaParameter, JavaTypeKind, JavaTypeNode,
};
use crate::languages::java::syntax::Import;
use crate::languages::java::types::TypeRef as ParsedTypeRef;
use crate::payload::NodePayload;
use crate::primitives::Location;
use crate::registries::Registries;
use crate::{Modifier, TypeParam, TypeRef};

// ---- Public surface ----

/// Pre-computed integration plan for one parsed Java file.
///
/// Per ADR-0005 a `ParsedFile` is "self-contained, with no graph
/// references" so it can be produced on a rayon worker. The plan stores
/// payloads and parent indices into its own internal vector;
/// [`integrate`] resolves indices to real [`NodeId`]s as it inserts
/// each payload into the graph.
#[derive(Debug)]
pub struct ParsedJavaFile {
    pub path: PathBuf,
    pub package: String,
    pub imports: Vec<Import>,
    plan: Vec<PendingNode>,
}

/// One unit of work in a parsed file's plan: a payload and its parent
/// index into the same plan (`None` for a root). The plan is in
/// topological order — parents are always pushed before children — so
/// integration can resolve indices linearly.
#[derive(Debug)]
pub(crate) struct PendingNode {
    payload: NodePayload,
    parent: Option<usize>,
}

/// Parse a Java source file into a self-contained [`ParsedJavaFile`].
///
/// Performs no graph mutation — runs on its own thread, suitable for
/// rayon parallel parsing. The returned plan is then consumed by
/// [`integrate`] on the graph thread.
pub fn parse_java_to_graph(path: &Path, source: &str) -> ParsedJavaFile {
    let mut parser = Parser::new();
    let language = tree_sitter_java::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("failed to set Java language");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return ParsedJavaFile {
                path: path.to_path_buf(),
                package: String::new(),
                imports: Vec::new(),
                plan: Vec::new(),
            };
        }
    };

    let root = tree.root_node();
    let source_bytes = source.as_bytes();

    let mut ctx = ParseContext {
        path,
        source: source_bytes,
        plan: Vec::new(),
        package: String::new(),
        enclosing_stack: Vec::new(),
    };

    // First pass: find the package declaration so `build_fqn` works for
    // every following symbol.
    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        if child.kind() == "package_declaration" {
            ctx.package = extract_package_name(child, source_bytes);
        }
    }

    // Second pass: extract symbols.
    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        extract_symbol(&mut ctx, child);
    }

    let imports = crate::languages::java::syntax::extract_imports(source);

    ParsedJavaFile {
        path: path.to_path_buf(),
        package: ctx.package,
        imports,
        plan: ctx.plan,
    }
}

/// Insert every node in the parsed plan into `graph`, register each
/// via its [`NodeBehavior::on_created`], and return the resulting
/// [`NodeId`]s in plan order.
///
/// Hard-link parent/child relationships are reconstructed from the
/// plan's `parent` indices. Per ADR-0014 the registration handles are
/// stored on [`NodeData::handles`](crate::graph::NodeData::handles); the
/// engine drops them when [`Graph::destroy`] frees the slot, removing
/// each registry entry as a side effect.
pub fn integrate(
    graph: &mut Graph<NodePayload>,
    registries: &Registries,
    parsed: ParsedJavaFile,
) -> Vec<NodeId> {
    let mut inserted: Vec<NodeId> = Vec::with_capacity(parsed.plan.len());
    for pending in parsed.plan {
        let parent = pending.parent.and_then(|idx| inserted.get(idx).copied());
        let id = graph.insert(pending.payload, parent);
        inserted.push(id);
    }

    for &id in &inserted {
        let handles = graph
            .get(id)
            .map(|node| node.payload.on_created(id, registries))
            .unwrap_or_default();
        if let Some(node) = graph.get_mut(id) {
            node.handles = handles;
        }
    }

    inserted
}

// ---- Walker context ----

struct ParseContext<'a> {
    path: &'a Path,
    source: &'a [u8],
    /// The plan being built. Every [`emit_pair`] call pushes two
    /// entries — Java then JVM — and the JVM entry is hard-linked off
    /// the Java one. `enclosing_stack` records Java-payload plan
    /// indices, so when a nested member fixes up its parent it always
    /// points at the Java side.
    plan: Vec<PendingNode>,
    package: String,
    /// Plan-indices of currently-open enclosing declarations (Java
    /// payloads). The walker pushes/pops as it descends/ascends class
    /// bodies.
    enclosing_stack: Vec<EnclosingFrame>,
}

struct EnclosingFrame {
    /// Plan index of the enclosing declaration's Java payload.
    java_idx: usize,
    /// The simple name, used to build child FQNs.
    name: String,
}

// ---- Tree-sitter helpers (unchanged from the prototype walker) ----

fn extract_package_name(node: Node, source: &[u8]) -> String {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "scoped_identifier" | "identifier" => {
                return node_text(child, source).to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn node_text<'a>(node: Node, source: &'a [u8]) -> &'a str {
    std::str::from_utf8(&source[node.byte_range()]).unwrap_or("")
}

fn build_fqn(ctx: &ParseContext, name: &str) -> String {
    let mut parts = Vec::new();
    if !ctx.package.is_empty() {
        parts.push(ctx.package.as_str());
    }
    for frame in &ctx.enclosing_stack {
        parts.push(frame.name.as_str());
    }
    parts.push(name);
    parts.join(".")
}

fn parent_owner_fqn(ctx: &ParseContext) -> Fqn {
    let mut parts = Vec::new();
    if !ctx.package.is_empty() {
        parts.push(ctx.package.as_str());
    }
    for frame in &ctx.enclosing_stack {
        parts.push(frame.name.as_str());
    }
    Fqn::new(parts.join("."))
}

fn make_location(ctx: &ParseContext, node: Node) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        file: ctx.path.to_path_buf(),
        start_line: start.row as u32,
        start_col: start.column as u32,
        end_line: end.row as u32,
        end_col: end.column as u32,
    }
}

fn extract_modifiers(node: Node, _source: &[u8]) -> Vec<Modifier> {
    let mut modifiers = Vec::new();
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "modifiers" {
            for j in 0..child.child_count() {
                let modifier_node = child.child(j).unwrap();
                if let Some(m) = parse_modifier(modifier_node.kind()) {
                    modifiers.push(m);
                }
            }
            break;
        }
    }
    modifiers
}

fn parse_modifier(text: &str) -> Option<Modifier> {
    match text {
        "public" => Some(Modifier::Public),
        "private" => Some(Modifier::Private),
        "protected" => Some(Modifier::Protected),
        "static" => Some(Modifier::Static),
        "abstract" => Some(Modifier::Abstract),
        "final" => Some(Modifier::Final),
        "sealed" => Some(Modifier::Sealed),
        "non-sealed" => Some(Modifier::NonSealed),
        "default" => Some(Modifier::Default),
        "synchronized" => Some(Modifier::Synchronized),
        "volatile" => Some(Modifier::Volatile),
        "transient" => Some(Modifier::Transient),
        "native" => Some(Modifier::Native),
        "strictfp" => Some(Modifier::Strictfp),
        _ => None,
    }
}

fn extract_type_parameters(node: Node, source: &[u8]) -> Vec<TypeParam> {
    let mut type_params = Vec::new();
    if let Some(tp_node) = node.child_by_field_name("type_parameters") {
        for i in 0..tp_node.child_count() {
            let child = tp_node.child(i).unwrap();
            if child.kind() == "type_parameter" {
                type_params.push(TypeParam::new(node_text(child, source).to_string()));
            }
        }
    }
    type_params
}

fn extract_formal_parameters(node: Node, source: &[u8]) -> Vec<(String, TypeRef, bool)> {
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        for i in 0..params_node.child_count() {
            let child = params_node.child(i).unwrap();
            let is_varargs = child.kind() == "spread_parameter";
            if child.kind() == "formal_parameter" || is_varargs {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source).to_string())
                    .unwrap_or_default();
                let ty = child
                    .child_by_field_name("type")
                    .map(|n| parse_type_ref(n, source).to_core())
                    .unwrap_or_else(|| TypeRef::simple("unknown"));
                params.push((name, ty, is_varargs));
            }
        }
    }
    params
}

fn parse_type_ref(node: Node, source: &[u8]) -> ParsedTypeRef {
    match node.kind() {
        "void_type" => ParsedTypeRef::Void,
        "integral_type" | "floating_point_type" | "boolean_type" => {
            ParsedTypeRef::Primitive(node_text(node, source).to_string())
        }
        "type_identifier" | "identifier" => {
            ParsedTypeRef::Simple(node_text(node, source).to_string())
        }
        "scoped_type_identifier" => {
            ParsedTypeRef::Qualified(node_text(node, source).to_string())
        }
        "generic_type" => {
            let base = node
                .child(0)
                .map(|n| node_text(n, source).to_string())
                .unwrap_or_default();
            let mut args = Vec::new();
            if let Some(type_args) = node.child_by_field_name("arguments") {
                for i in 0..type_args.child_count() {
                    let child = type_args.child(i).unwrap();
                    if child.kind() != "<" && child.kind() != ">" && child.kind() != "," {
                        args.push(parse_type_ref(child, source));
                    }
                }
            } else {
                for i in 0..node.child_count() {
                    let child = node.child(i).unwrap();
                    if child.kind() == "type_arguments" {
                        for j in 0..child.child_count() {
                            let arg = child.child(j).unwrap();
                            if arg.kind() != "<" && arg.kind() != ">" && arg.kind() != "," {
                                args.push(parse_type_ref(arg, source));
                            }
                        }
                    }
                }
            }
            ParsedTypeRef::Parameterized(base, args)
        }
        "array_type" => {
            if let Some(element) = node.child_by_field_name("element") {
                ParsedTypeRef::Array(Box::new(parse_type_ref(element, source)))
            } else if let Some(first_child) = node.child(0) {
                ParsedTypeRef::Array(Box::new(parse_type_ref(first_child, source)))
            } else {
                ParsedTypeRef::Array(Box::new(ParsedTypeRef::Simple(
                    node_text(node, source).to_string(),
                )))
            }
        }
        "wildcard" => ParsedTypeRef::Wildcard,
        _ => ParsedTypeRef::Simple(node_text(node, source).to_string()),
    }
}

// ---- Plan emission helpers ----

/// Push a (Java, JVM) pair into the plan. Returns the Java plan index;
/// the JVM projection sits at `index + 1` and is hard-linked off the
/// Java node per ADR-0004 ("each language-model node hard-links a JVM
/// projection — projections are leaves, not relays").
fn emit_pair(
    ctx: &mut ParseContext,
    java: JavaNodePayload,
    jvm: JvmNodePayload,
) -> usize {
    let parent = ctx.enclosing_stack.last().map(|f| f.java_idx);
    let java_idx = ctx.plan.len();
    ctx.plan.push(PendingNode {
        payload: NodePayload::Java(java),
        parent,
    });
    ctx.plan.push(PendingNode {
        payload: NodePayload::Jvm(jvm),
        parent: Some(java_idx),
    });
    java_idx
}

fn java_header(ctx: &ParseContext, name: &str, node: Node, modifiers: Vec<Modifier>) -> JavaDeclHeader {
    JavaDeclHeader {
        name: name.to_string(),
        fqn: Fqn::new(build_fqn(ctx, name)),
        location: Some(make_location(ctx, node)),
        modifiers,
        annotations: Vec::new(),
    }
}

fn jvm_header(ctx: &ParseContext, name: &str, node: Node, modifiers: Vec<Modifier>) -> JvmDeclHeader {
    JvmDeclHeader {
        name: name.to_string(),
        fqn: Fqn::new(build_fqn(ctx, name)),
        location: Some(make_location(ctx, node)),
        modifiers,
        annotations: Vec::new(),
    }
}

// ---- Walker (extract_*) ----
//
// Each function visits one tree-sitter node category and emits a
// (Java, JVM) plan-pair for it. Per ADR-0021 the walker structure is
// preserved from the prototype; only the emission body differs.

fn extract_symbol(ctx: &mut ParseContext, node: Node) {
    match node.kind() {
        "class_declaration" => extract_class_like(ctx, node, JavaTypeKind::Class, JvmTypeKind::Class),
        "interface_declaration" => {
            extract_class_like(ctx, node, JavaTypeKind::Interface, JvmTypeKind::Interface)
        }
        "enum_declaration" => extract_enum(ctx, node),
        "record_declaration" => extract_class_like(ctx, node, JavaTypeKind::Record, JvmTypeKind::Record),
        "annotation_type_declaration" => {
            extract_class_like(ctx, node, JavaTypeKind::Annotation, JvmTypeKind::Annotation)
        }
        _ => {}
    }
}

fn extract_class_like(
    ctx: &mut ParseContext,
    node: Node,
    java_kind: JavaTypeKind,
    jvm_kind: JvmTypeKind,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let modifiers = extract_modifiers(node, ctx.source);
    let type_parameters = extract_type_parameters(node, ctx.source);

    let java_payload = JavaNodePayload::Type(JavaTypeNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        kind: java_kind,
        type_parameters: type_parameters.clone(),
        record_components: Vec::new(),
    });
    let jvm_payload = JvmNodePayload::Type(JvmTypeNode {
        header: jvm_header(ctx, &name, node, modifiers),
        kind: jvm_kind,
        type_parameters,
        record_components: Vec::new(),
        enrichments: JvmEnrichments::default(),
    });
    let java_idx = emit_pair(ctx, java_payload, jvm_payload);

    ctx.enclosing_stack.push(EnclosingFrame { java_idx, name });
    extract_body_members(ctx, node);
    ctx.enclosing_stack.pop();
}

fn extract_enum(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let modifiers = extract_modifiers(node, ctx.source);

    let java_payload = JavaNodePayload::Type(JavaTypeNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        kind: JavaTypeKind::Enum,
        type_parameters: Vec::new(),
        record_components: Vec::new(),
    });
    let jvm_payload = JvmNodePayload::Type(JvmTypeNode {
        header: jvm_header(ctx, &name, node, modifiers),
        kind: JvmTypeKind::Enum,
        type_parameters: Vec::new(),
        record_components: Vec::new(),
        enrichments: JvmEnrichments::default(),
    });
    let java_idx = emit_pair(ctx, java_payload, jvm_payload);

    ctx.enclosing_stack.push(EnclosingFrame {
        java_idx,
        name: name.clone(),
    });
    if let Some(body) = node.child_by_field_name("body") {
        for i in 0..body.child_count() {
            let child = body.child(i).unwrap();
            match child.kind() {
                "enum_constant" => extract_enum_constant(ctx, child),
                "enum_body_declarations" => {
                    for j in 0..child.child_count() {
                        let decl = child.child(j).unwrap();
                        extract_body_member(ctx, decl);
                    }
                }
                _ => extract_body_member(ctx, child),
            }
        }
    }
    ctx.enclosing_stack.pop();
}

fn extract_enum_constant(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let modifiers = vec![Modifier::Public, Modifier::Static, Modifier::Final];
    let enum_owner = parent_owner_fqn(ctx);

    let java_payload = JavaNodePayload::EnumConstant(JavaEnumConstantNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        enum_owner: enum_owner.clone(),
    });
    let jvm_payload = JvmNodePayload::EnumConstant(JvmEnumConstantNode {
        header: jvm_header(ctx, &name, node, modifiers),
        enum_owner,
    });
    emit_pair(ctx, java_payload, jvm_payload);
}

fn extract_body_members(ctx: &mut ParseContext, node: Node) {
    let body = node.child_by_field_name("body").or_else(|| {
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            if child.kind().ends_with("_body") {
                return Some(child);
            }
        }
        None
    });

    if let Some(body) = body {
        for i in 0..body.child_count() {
            let child = body.child(i).unwrap();
            extract_body_member(ctx, child);
        }
    }
}

fn extract_body_member(ctx: &mut ParseContext, node: Node) {
    match node.kind() {
        "method_declaration" => extract_method(ctx, node),
        "constructor_declaration" => extract_constructor(ctx, node),
        "field_declaration" => extract_fields(ctx, node),
        "class_declaration" => extract_class_like(ctx, node, JavaTypeKind::Class, JvmTypeKind::Class),
        "interface_declaration" => {
            extract_class_like(ctx, node, JavaTypeKind::Interface, JvmTypeKind::Interface)
        }
        "enum_declaration" => extract_enum(ctx, node),
        "record_declaration" => {
            extract_class_like(ctx, node, JavaTypeKind::Record, JvmTypeKind::Record)
        }
        "annotation_type_declaration" => {
            extract_class_like(ctx, node, JavaTypeKind::Annotation, JvmTypeKind::Annotation)
        }
        _ => {}
    }
}

fn extract_method(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let modifiers = extract_modifiers(node, ctx.source);
    let return_type = node
        .child_by_field_name("type")
        .map(|n| parse_type_ref(n, ctx.source).to_core())
        .unwrap_or(TypeRef::Void);
    let parameters_raw = extract_formal_parameters(node, ctx.source);
    let type_parameters = extract_type_parameters(node, ctx.source);

    let java_parameters: Vec<JavaParameter> = parameters_raw
        .iter()
        .map(|(pname, pty, varargs)| JavaParameter {
            name: pname.clone(),
            param_type: pty.clone(),
            is_varargs: *varargs,
        })
        .collect();
    let owner = parent_owner_fqn(ctx);
    let jvm_parameters: Vec<JvmParameter> = parameters_raw
        .iter()
        .map(|(pname, pty, varargs)| JvmParameter {
            name: pname.clone(),
            // Per ADR-0012 JvmMethodKey requires erased parameter
            // types; pre-erase here at construction.
            param_type: pty.erasure(),
            is_varargs: *varargs,
            enrichments: JvmEnrichments::default(),
        })
        .collect();

    let java_payload = JavaNodePayload::Method(JavaMethodNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        return_type: return_type.clone(),
        parameters: java_parameters,
        type_parameters: type_parameters.clone(),
        throws: Vec::new(),
    });
    let jvm_payload = JvmNodePayload::Method(JvmMethodNode {
        header: jvm_header(ctx, &name, node, modifiers),
        owner,
        return_type: return_type.erasure(),
        parameters: jvm_parameters,
        type_parameters,
        throws: Vec::new(),
        enrichments: JvmEnrichments::default(),
    });
    emit_pair(ctx, java_payload, jvm_payload);
}

fn extract_constructor(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let modifiers = extract_modifiers(node, ctx.source);
    let parameters_raw = extract_formal_parameters(node, ctx.source);
    let type_parameters = extract_type_parameters(node, ctx.source);

    let java_parameters: Vec<JavaParameter> = parameters_raw
        .iter()
        .map(|(pname, pty, varargs)| JavaParameter {
            name: pname.clone(),
            param_type: pty.clone(),
            is_varargs: *varargs,
        })
        .collect();
    let owner = parent_owner_fqn(ctx);
    let jvm_parameters: Vec<JvmParameter> = parameters_raw
        .iter()
        .map(|(pname, pty, varargs)| JvmParameter {
            name: pname.clone(),
            param_type: pty.erasure(),
            is_varargs: *varargs,
            enrichments: JvmEnrichments::default(),
        })
        .collect();

    let java_payload = JavaNodePayload::Constructor(JavaConstructorNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        parameters: java_parameters,
        type_parameters: type_parameters.clone(),
        throws: Vec::new(),
    });
    let jvm_payload = JvmNodePayload::Constructor(JvmConstructorNode {
        header: jvm_header(ctx, &name, node, modifiers),
        owner,
        parameters: jvm_parameters,
        type_parameters,
        throws: Vec::new(),
    });
    emit_pair(ctx, java_payload, jvm_payload);
}

fn extract_fields(ctx: &mut ParseContext, node: Node) {
    let modifiers = extract_modifiers(node, ctx.source);
    let field_type = node
        .child_by_field_name("type")
        .map(|n| parse_type_ref(n, ctx.source).to_core())
        .unwrap_or_else(|| TypeRef::simple("unknown"));
    let owner = parent_owner_fqn(ctx);

    // Field declarations can have multiple declarators: `int a, b, c;`
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "variable_declarator" {
            let name = match child.child_by_field_name("name") {
                Some(n) => node_text(n, ctx.source).to_string(),
                None => continue,
            };

            let java_payload = JavaNodePayload::Field(JavaFieldNode {
                header: java_header(ctx, &name, child, modifiers.clone()),
                field_type: field_type.clone(),
                constant_value: None,
                initialized: false,
            });
            let jvm_payload = JvmNodePayload::Field(JvmFieldNode {
                header: jvm_header(ctx, &name, child, modifiers.clone()),
                owner: owner.clone(),
                field_type: field_type.clone(),
                constant_value: None,
                initialized: false,
                enrichments: JvmEnrichments::default(),
            });
            emit_pair(ctx, java_payload, jvm_payload);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Per ADR-0005 the parse phase must be runnable on a rayon worker
    /// — meaning the parse output type has to be `Send`. Per ADR-0014
    /// RAII registration handles live on
    /// [`NodeData::handles`](crate::graph::NodeData::handles), not on
    /// the payload, which keeps every payload variant free of
    /// `Rc`-flavoured `!Send` taints. This is a static check that the
    /// invariant holds — a regression here would silently break
    /// workspace indexing parallelism.
    fn _assert_parse_output_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ParsedJavaFile>();
        assert_send::<NodePayload>();
    }

    fn parse(source: &str) -> ParsedJavaFile {
        parse_java_to_graph(Path::new("Test.java"), source)
    }

    /// Find a Java payload by simple name. Walks the plan in order so
    /// the test asserts on the first hit (matches the walker's
    /// declaration order).
    fn find_java<'a>(plan: &'a [PendingNode], name: &str) -> &'a JavaNodePayload {
        plan.iter()
            .filter_map(|p| match &p.payload {
                NodePayload::Java(j) => Some(j),
                _ => None,
            })
            .find(|j| j.header().is_some_and(|h| h.name == name))
            .unwrap_or_else(|| panic!("no Java payload named '{}'", name))
    }

    #[test]
    fn simple_class() {
        let parsed = parse("package com.example;\npublic class Dog {}\n");
        let dog = find_java(&parsed.plan, "Dog");
        if let JavaNodePayload::Type(t) = dog {
            assert_eq!(t.header.fqn.as_str(), "com.example.Dog");
            assert_eq!(t.kind, JavaTypeKind::Class);
            assert!(t.header.modifiers.contains(&Modifier::Public));
        } else {
            panic!("expected Type payload");
        }
    }

    #[test]
    fn class_with_members_emits_pairs_in_topological_order() {
        let source = r#"
package com.example;
public class Dog {
    private String name;
    public Dog(String name) { this.name = name; }
    public String getName() { return name; }
}
"#;
        let parsed = parse(source);

        // Plan layout: Dog(Java), Dog(Jvm), name(Java), name(Jvm),
        // Dog ctor(Java), Dog ctor(Jvm), getName(Java), getName(Jvm).
        // Each Java payload's parent points at its enclosing Java
        // payload's plan index.
        let dog_idx = 0;
        assert_eq!(parsed.plan[dog_idx].parent, None);
        assert_eq!(parsed.plan[dog_idx + 1].parent, Some(dog_idx));

        // Members of Dog: their Java parent is `dog_idx`.
        for member in parsed.plan.iter().skip(2) {
            if let NodePayload::Java(_) = member.payload {
                assert_eq!(member.parent, Some(dog_idx));
            }
        }

        let name_field = find_java(&parsed.plan, "name");
        if let JavaNodePayload::Field(f) = name_field {
            assert_eq!(f.field_type.to_string(), "String");
        } else {
            panic!("expected Field");
        }

        let getter = find_java(&parsed.plan, "getName");
        if let JavaNodePayload::Method(m) = getter {
            assert_eq!(m.return_type.to_string(), "String");
        } else {
            panic!("expected Method");
        }
    }

    #[test]
    fn enum_constants_emit_as_enum_constant_not_field() {
        let source = r#"
package com.example;
public enum Color { RED, GREEN, BLUE; public String label() { return name(); } }
"#;
        let parsed = parse(source);

        let red = find_java(&parsed.plan, "RED");
        assert!(matches!(red, JavaNodePayload::EnumConstant(_)));

        // The `label` method should still be a Method.
        let label = find_java(&parsed.plan, "label");
        assert!(matches!(label, JavaNodePayload::Method(_)));
    }

    #[test]
    fn jvm_method_keys_carry_erased_param_types() {
        let source = r#"
package com.example;
public class Service {
    public void process(java.util.List<String> items) {}
}
"#;
        let parsed = parse(source);
        // Find the JVM Method payload, check that the parameter type's
        // erasure has been applied (parameterized type → raw).
        let process_jvm = parsed
            .plan
            .iter()
            .filter_map(|p| match &p.payload {
                NodePayload::Jvm(JvmNodePayload::Method(m)) if m.header.name == "process" => {
                    Some(m)
                }
                _ => None,
            })
            .next()
            .expect("JVM process method");
        assert_eq!(process_jvm.parameters.len(), 1);
        let erased = &process_jvm.parameters[0].param_type;
        // Erasure of `java.util.List<String>` is `java.util.List`.
        assert_eq!(erased.to_string(), "java.util.List");
    }

    #[test]
    fn varargs_parameter_flagged() {
        let parsed = parse(
            "package com.example;\npublic class V { public void f(String... xs) {} }\n",
        );
        let f = find_java(&parsed.plan, "f");
        if let JavaNodePayload::Method(m) = f {
            assert_eq!(m.parameters.len(), 1);
            assert!(m.parameters[0].is_varargs);
        } else {
            panic!("expected Method payload");
        }
    }

    #[test]
    fn nested_class_parent_points_at_outer() {
        let source = r#"
package com.example;
public class Outer {
    public class Inner { private int value; }
}
"#;
        let parsed = parse(source);
        // Outer at plan_idx 0; Inner's Java payload should have
        // parent == 0.
        let outer_idx = 0;
        let inner_idx = parsed
            .plan
            .iter()
            .position(|p| matches!(&p.payload, NodePayload::Java(j) if j.header().is_some_and(|h| h.name == "Inner")))
            .expect("inner not found");
        assert_eq!(parsed.plan[inner_idx].parent, Some(outer_idx));
        let value_idx = parsed
            .plan
            .iter()
            .position(|p| matches!(&p.payload, NodePayload::Java(j) if j.header().is_some_and(|h| h.name == "value")))
            .expect("value field not found");
        // value's parent is Inner.
        assert_eq!(parsed.plan[value_idx].parent, Some(inner_idx));

        let inner = find_java(&parsed.plan, "Inner");
        if let JavaNodePayload::Type(t) = inner {
            assert_eq!(t.header.fqn.as_str(), "com.example.Outer.Inner");
        }
    }

    #[test]
    fn modifiers_propagate_through_pair() {
        let source = r#"
package com.example;
public abstract class Base {
    protected static final int MAX = 100;
    public abstract void doWork();
}
"#;
        let parsed = parse(source);

        let base = find_java(&parsed.plan, "Base");
        let mods = match base {
            JavaNodePayload::Type(t) => &t.header.modifiers,
            _ => panic!(),
        };
        assert!(mods.contains(&Modifier::Public));
        assert!(mods.contains(&Modifier::Abstract));

        let max = find_java(&parsed.plan, "MAX");
        let mods = max.header().unwrap().modifiers.clone();
        assert!(mods.contains(&Modifier::Protected));
        assert!(mods.contains(&Modifier::Static));
        assert!(mods.contains(&Modifier::Final));
    }

    #[test]
    fn package_extracted_into_parsed_file() {
        let parsed = parse("package com.example;\npublic class Foo {}\n");
        assert_eq!(parsed.package, "com.example");
    }

    #[test]
    fn no_package_yields_empty_string() {
        let parsed = parse("public class Foo {}\n");
        assert_eq!(parsed.package, "");
        let foo = find_java(&parsed.plan, "Foo");
        if let JavaNodePayload::Type(t) = foo {
            assert_eq!(t.header.fqn.as_str(), "Foo");
        }
    }

    #[test]
    fn multiple_field_declarators() {
        let parsed = parse("package com.example;\npublic class M { private int a, b, c; }\n");
        for n in &["a", "b", "c"] {
            let f = find_java(&parsed.plan, n);
            if let JavaNodePayload::Field(field) = f {
                assert_eq!(field.header.fqn.as_str(), &format!("com.example.M.{n}"));
            } else {
                panic!("expected Field payload for {n}");
            }
        }
    }

    #[test]
    fn annotation_type_parsed_as_annotation_kind() {
        let parsed = parse(
            "package com.example;\npublic @interface MyAnnotation { String value(); }\n",
        );
        let annot = find_java(&parsed.plan, "MyAnnotation");
        if let JavaNodePayload::Type(t) = annot {
            assert_eq!(t.kind, JavaTypeKind::Annotation);
            assert_eq!(t.header.fqn.as_str(), "com.example.MyAnnotation");
        } else {
            panic!("expected Type payload");
        }
    }

    #[test]
    fn interface_body_emits_methods_as_children() {
        let parsed = parse(
            "package com.example;\npublic interface Foo { void bar(); }\n",
        );
        let foo = find_java(&parsed.plan, "Foo");
        match foo {
            JavaNodePayload::Type(t) => assert_eq!(t.kind, JavaTypeKind::Interface),
            _ => panic!("expected Type payload"),
        }
        let bar = find_java(&parsed.plan, "bar");
        assert!(matches!(bar, JavaNodePayload::Method(_)));
    }

    #[test]
    fn generic_class_carries_type_parameters() {
        let parsed = parse("package com.example;\npublic class Box<T> { T value; }\n");
        let boxed = find_java(&parsed.plan, "Box");
        if let JavaNodePayload::Type(t) = boxed {
            assert_eq!(t.type_parameters.len(), 1);
            assert_eq!(t.type_parameters[0].name, "T");
        } else {
            panic!("expected Type payload");
        }
    }

    #[test]
    fn java_field_preserves_parameterized_type() {
        // Java-side `field_type` keeps the parameterized form `List<String>`.
        // The JVM-side erasure to `List` is covered by
        // `jvm_method_keys_carry_erased_param_types`; this test pins the
        // pre-erasure preservation.
        let parsed = parse(
            "package com.example;\npublic class C { java.util.List<String> xs; }\n",
        );
        let xs = find_java(&parsed.plan, "xs");
        if let JavaNodePayload::Field(f) = xs {
            let ty = f.field_type.to_string();
            assert!(
                ty.contains("List") && ty.contains("String"),
                "expected Java-side type to retain List<String>, got `{ty}`"
            );
        } else {
            panic!("expected Field payload");
        }
    }

    #[test]
    fn array_types_parse_through_fallback_chain() {
        // `parse_type_ref`'s `array_type` branch tries
        // `child_by_field_name("element")` first, then `node.child(0)`,
        // then text-of-the-whole-node. This test exercises both nested
        // array shapes so a regression in the fallback chain surfaces.
        let parsed = parse(
            "package com.example;\npublic class C { int[] a; String[][] b; }\n",
        );
        let a = find_java(&parsed.plan, "a");
        if let JavaNodePayload::Field(f) = a {
            let ty = f.field_type.to_string();
            assert!(
                ty.contains("[]"),
                "expected `int[]` to render as an array shape, got `{ty}`"
            );
        } else {
            panic!("expected Field payload for `a`");
        }
        let b = find_java(&parsed.plan, "b");
        if let JavaNodePayload::Field(f) = b {
            let ty = f.field_type.to_string();
            assert!(
                ty.contains("[]"),
                "expected `String[][]` to render as an array shape, got `{ty}`"
            );
        } else {
            panic!("expected Field payload for `b`");
        }
    }

    #[test]
    fn declaration_location_tracks_source_line() {
        // Multi-line source with the class declaration on a known line.
        // tree-sitter rows are 0-indexed; `package com.example;` is row 0,
        // blank line is row 1, the class declaration spans starting at
        // row 2.
        let source = "package com.example;\n\npublic class Foo {\n}\n";
        let parsed = parse(source);
        let foo = find_java(&parsed.plan, "Foo");
        if let JavaNodePayload::Type(t) = foo {
            let loc = t
                .header
                .location
                .as_ref()
                .expect("source-derived payloads carry locations");
            assert_eq!(loc.start_line, 2, "Foo declaration should start on row 2");
        } else {
            panic!("expected Type payload");
        }
    }
}
