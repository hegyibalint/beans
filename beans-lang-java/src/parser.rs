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

use beans_core::graph::NodeBehavior;
use beans_core::graph::arena::{Graph, NodeId};
use beans_lang_jvm::fqn::Fqn;
use beans_lang_jvm::payload::{
    JvmConstructorNode, JvmDeclHeader, JvmEnrichments, JvmEnumConstantNode, JvmFieldNode,
    JvmMethodNode, JvmNodePayload, JvmParameter, JvmTypeKind, JvmTypeNode,
};
use crate::payload::{
    JavaConstructorNode, JavaDeclHeader, JavaEnumConstantNode, JavaFieldNode, JavaMethodNode,
    JavaNodePayload, JavaParameter, JavaTypeKind, JavaTypeNode, JavaTypeUseNode, JavaUseHeader,
};
use crate::syntax::{extract_imports, Import};
use crate::types::TypeRef as ParsedTypeRef;
use beans_core::primitives::Location;
use beans_lang_jvm::{Modifier, TypeParam, TypeRef};

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
    payload: JavaPlanPayload,
    parent: Option<usize>,
}

/// The two payload species this vertical emits: Java source nodes and
/// their JVM projections. The facade's graph payload union is above
/// this crate, so the plan carries a local enum; [`integrate`] converts
/// at the boundary through the consumer's `From` impls.
#[derive(Debug)]
pub enum JavaPlanPayload {
    Java(JavaNodePayload),
    Jvm(JvmNodePayload),
}

impl ParsedJavaFile {
    /// Re-key every qualified name in the plan onto the workspace's
    /// canonical buffers (backlog #037). Runs at the serial integrate
    /// boundary — parsing stays interner-free so it can fan out across
    /// rayon workers with self-contained outputs (ADR-0005). Every
    /// downstream copy (registry key, RAII handle, projection) clones
    /// from these payloads, so one pass here collapses them all.
    pub fn intern(&mut self, interner: &beans_core::Interner) {
        for pending in &mut self.plan {
            match &mut pending.payload {
                JavaPlanPayload::Java(java) => match java {
                    JavaNodePayload::Type(n) => n.header.fqn.intern_in(interner),
                    JavaNodePayload::Method(n) => n.header.fqn.intern_in(interner),
                    JavaNodePayload::Constructor(n) => n.header.fqn.intern_in(interner),
                    JavaNodePayload::Field(n) => n.header.fqn.intern_in(interner),
                    JavaNodePayload::EnumConstant(n) => {
                        n.header.fqn.intern_in(interner);
                        n.enum_owner.intern_in(interner);
                    }
                    JavaNodePayload::AnnotationElement(n) => n.header.fqn.intern_in(interner),
                    JavaNodePayload::Package(n) => n.header.fqn.intern_in(interner),
                    JavaNodePayload::TypeUse(n) => {
                        for fqn in &mut n.header.candidate_fqns {
                            fqn.intern_in(interner);
                        }
                    }
                    JavaNodePayload::Parameter(_) | JavaNodePayload::Import(_) => {}
                },
                JavaPlanPayload::Jvm(jvm) => match jvm {
                    JvmNodePayload::Type(n) => n.header.fqn.intern_in(interner),
                    JvmNodePayload::Method(n) => {
                        n.header.fqn.intern_in(interner);
                        n.owner.intern_in(interner);
                    }
                    JvmNodePayload::Constructor(n) => {
                        n.header.fqn.intern_in(interner);
                        n.owner.intern_in(interner);
                    }
                    JvmNodePayload::Field(n) => {
                        n.header.fqn.intern_in(interner);
                        n.owner.intern_in(interner);
                    }
                    JvmNodePayload::EnumConstant(n) => {
                        n.header.fqn.intern_in(interner);
                        n.enum_owner.intern_in(interner);
                    }
                    JvmNodePayload::AnnotationElement(n) => {
                        n.header.fqn.intern_in(interner);
                        n.owner.intern_in(interner);
                    }
                    JvmNodePayload::Package(n) => n.header.fqn.intern_in(interner),
                    JvmNodePayload::Parameter(_) => {}
                },
            }
        }
    }
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

    // Imports are captured before the symbol pass so the walker can
    // resolve `JavaTypeUseNode::candidate_fqns` per ADR-0029. The
    // line-based `extract_imports` is robust to malformed surrounding
    // code; tree-sitter would do the same job but we preserve the
    // existing line-based path for now.
    let imports = extract_imports(path, source);

    let mut ctx = ParseContext {
        path,
        shared_path: std::sync::Arc::from(path),
        source: source_bytes,
        plan: Vec::new(),
        package: String::new(),
        enclosing_stack: Vec::new(),
        imports: imports.clone(),
    };

    // First pass: find the package declaration so `build_fqn` works for
    // every following symbol, and so use-site candidate FQNs can include
    // the same-package candidate.
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
/// stored on [`NodeData::handles`](beans_core::graph::NodeData::handles); the
/// engine drops them when [`Graph::destroy`] frees the slot, removing
/// each registry entry as a side effect.
pub fn integrate<P, C>(
    graph: &mut Graph<P>,
    registries: &C,
    parsed: ParsedJavaFile,
) -> Vec<NodeId>
where
    P: From<JavaNodePayload> + From<JvmNodePayload> + NodeBehavior<Ctx = C>,
{
    let mut inserted: Vec<NodeId> = Vec::with_capacity(parsed.plan.len());
    for pending in parsed.plan {
        let parent = pending.parent.and_then(|idx| inserted.get(idx).copied());
        let payload = match pending.payload {
            JavaPlanPayload::Java(j) => P::from(j),
            JavaPlanPayload::Jvm(v) => P::from(v),
        };
        let id = graph.insert(payload, parent);
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
    /// One shared buffer for this file's path; every emitted
    /// [`Location`] clones it (pointer bump) instead of copying the
    /// path text per node (backlog #037).
    shared_path: std::sync::Arc<Path>,
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
    /// File-level imports. Per ADR-0029 the walker computes
    /// `JavaUseHeader::candidate_fqns` for each use site at parse time,
    /// using `imports + same-package + java.lang` in priority order.
    imports: Vec<Import>,
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
        file: std::sync::Arc::clone(&ctx.shared_path),
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
            match child.kind() {
                "formal_parameter" => {
                    // tree-sitter-java exposes `name` and `type` as
                    // named fields on `formal_parameter`.
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| node_text(n, source).to_string())
                        .unwrap_or_default();
                    let ty = child
                        .child_by_field_name("type")
                        .map(|n| parse_type_ref(n, source).to_core())
                        .unwrap_or_else(|| TypeRef::simple("unknown"));
                    params.push((name, ty, false));
                }
                "spread_parameter" => {
                    // tree-sitter-java models `String... xs` as a
                    // `spread_parameter` whose children are the type
                    // node, the `...` token, and a `variable_declarator`
                    // carrying the name. The named `name`/`type`
                    // fields aren't exposed; walk children manually.
                    let mut name = String::new();
                    let mut ty: Option<TypeRef> = None;
                    for j in 0..child.child_count() {
                        let part = child.child(j).unwrap();
                        match part.kind() {
                            "variable_declarator" => {
                                if let Some(n) = part.child_by_field_name("name") {
                                    name = node_text(n, source).to_string();
                                }
                            }
                            "..." | "modifiers" => {}
                            _ => {
                                // Anything else is the parameter type.
                                if ty.is_none() {
                                    ty = Some(parse_type_ref(part, source).to_core());
                                }
                            }
                        }
                    }
                    params.push((
                        name,
                        ty.unwrap_or_else(|| TypeRef::simple("unknown")),
                        true,
                    ));
                }
                _ => {}
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

// ---- Type-use emission (ADR-0029) ----

/// Walk a tree-sitter type expression and emit one
/// [`JavaTypeUseNode`] per named identifier, hard-linked under
/// `parent_plan_idx`. Per ADR-0029 the use-site's `location` spans
/// the identifier text only — for `com.example.Service` the span is
/// `Service`, not the qualifier prefix; for `Repository<User>` the
/// outer raw type and each type argument emit one flat node each.
///
/// Primitive types (`int`, `boolean`, ...) and `void` produce no use
/// sites — they don't need resolution.
fn emit_type_use_sites(ctx: &mut ParseContext, type_node: Node, parent_plan_idx: usize) {
    match type_node.kind() {
        "void_type" | "integral_type" | "floating_point_type" | "boolean_type" => {
            // Primitive: no use site.
        }
        "type_identifier" => {
            let name = node_text(type_node, ctx.source).to_string();
            let location = make_location(ctx, type_node);
            emit_type_use(ctx, name, location, parent_plan_idx);
        }
        "scoped_type_identifier" => {
            // Span the rightmost identifier only — that's the
            // refactor-meaningful token. The qualifier is recoverable
            // from context if any consumer needs it.
            let ident = rightmost_type_identifier(type_node);
            let name = node_text(ident, ctx.source).to_string();
            let location = make_location(ctx, ident);
            emit_type_use(ctx, name, location, parent_plan_idx);
        }
        "generic_type" => {
            // Outer raw type (first child): one use site.
            // Type arguments (children of `arguments` field): one each.
            if let Some(base) = type_node.child(0) {
                emit_type_use_sites(ctx, base, parent_plan_idx);
            }
            let args_node = type_node.child_by_field_name("arguments").or_else(|| {
                (0..type_node.child_count())
                    .filter_map(|i| type_node.child(i))
                    .find(|c| c.kind() == "type_arguments")
            });
            if let Some(args) = args_node {
                for i in 0..args.child_count() {
                    let arg = args.child(i).unwrap();
                    if matches!(arg.kind(), "<" | ">" | ",") {
                        continue;
                    }
                    emit_type_use_sites(ctx, arg, parent_plan_idx);
                }
            }
        }
        "array_type" => {
            let elem = type_node
                .child_by_field_name("element")
                .or_else(|| type_node.child(0));
            if let Some(elem) = elem {
                emit_type_use_sites(ctx, elem, parent_plan_idx);
            }
        }
        "wildcard" => {
            // `?`, `? extends T`, `? super T` — the bound (when present)
            // is a type identifier we want.
            for i in 0..type_node.child_count() {
                let child = type_node.child(i).unwrap();
                if matches!(child.kind(), "?" | "extends" | "super") {
                    continue;
                }
                emit_type_use_sites(ctx, child, parent_plan_idx);
            }
        }
        _ => {
            // Unknown type-position node kind. Skip silently rather
            // than panic; tree-sitter-java may surface annotated_type
            // and other shapes we'll add as they appear in tests.
        }
    }
}

/// Descend a `scoped_type_identifier` to the rightmost `type_identifier`
/// child. For `com.example.Service`, returns the `Service` token. For
/// a plain `type_identifier`, returns it unchanged.
fn rightmost_type_identifier(node: Node<'_>) -> Node<'_> {
    if node.kind() != "scoped_type_identifier" {
        return node;
    }
    for i in (0..node.child_count()).rev() {
        let child = match node.child(i) {
            Some(c) => c,
            None => continue,
        };
        match child.kind() {
            "type_identifier" => return child,
            "scoped_type_identifier" => return rightmost_type_identifier(child),
            _ => {}
        }
    }
    node
}

fn emit_type_use(
    ctx: &mut ParseContext,
    name: String,
    location: Location,
    parent_plan_idx: usize,
) {
    let candidate_fqns = build_candidate_fqns(ctx, &name);
    let payload = JavaNodePayload::from(JavaTypeUseNode {
        header: JavaUseHeader {
            name,
            location,
            candidate_fqns,
        },
    });
    ctx.plan.push(PendingNode {
        payload: JavaPlanPayload::Java(payload),
        parent: Some(parent_plan_idx),
    });
}

/// Compute the priority-ordered FQN candidates for a simple type name,
/// per ADR-0029 / Java's classpath shadowing rules.
///
/// Order: explicit single-imports → same-package → `java.lang` →
/// wildcard imports. Static imports do not contribute (they bind
/// member names, not type names).
fn build_candidate_fqns(ctx: &ParseContext, simple_name: &str) -> Vec<Fqn> {
    let mut out: Vec<Fqn> = Vec::new();

    for imp in &ctx.imports {
        if let Import::Single(fqn, _) = imp {
            if fqn.rsplit('.').next() == Some(simple_name) {
                out.push(Fqn::new(fqn.clone()));
            }
        }
    }

    let same_pkg_fqn = if ctx.package.is_empty() {
        simple_name.to_string()
    } else {
        format!("{}.{}", ctx.package, simple_name)
    };
    if !out.iter().any(|f| f.as_str() == same_pkg_fqn) {
        out.push(Fqn::new(same_pkg_fqn));
    }

    let java_lang = format!("java.lang.{}", simple_name);
    if !out.iter().any(|f| f.as_str() == java_lang) {
        out.push(Fqn::new(java_lang));
    }

    for imp in &ctx.imports {
        if let Import::Wildcard(pkg, _) = imp {
            let candidate = format!("{}.{}", pkg, simple_name);
            if !out.iter().any(|f| f.as_str() == candidate) {
                out.push(Fqn::new(candidate));
            }
        }
    }

    out
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
        payload: JavaPlanPayload::Java(java),
        parent,
    });
    ctx.plan.push(PendingNode {
        payload: JavaPlanPayload::Jvm(jvm),
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

    let java_payload = JavaNodePayload::from(JavaTypeNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        kind: java_kind,
        type_parameters: type_parameters.clone(),
        record_components: Vec::new(),
    });
    let jvm_payload = JvmNodePayload::from(JvmTypeNode {
        header: jvm_header(ctx, &name, node, modifiers),
        kind: jvm_kind,
        type_parameters,
        record_components: Vec::new(),
        enrichments: JvmEnrichments::default(),
    });
    let java_idx = emit_pair(ctx, java_payload, jvm_payload);
    emit_class_extends_implements(ctx, node, java_idx);

    ctx.enclosing_stack.push(EnclosingFrame { java_idx, name });
    extract_body_members(ctx, node);
    ctx.enclosing_stack.pop();
}

/// Walk a class-like declaration's `superclass` and `interfaces` fields
/// and emit one [`JavaTypeUseNode`] per named supertype identifier,
/// hard-linked under the class declaration.
fn emit_class_extends_implements(ctx: &mut ParseContext, node: Node, parent_idx: usize) {
    if let Some(sc) = node.child_by_field_name("superclass") {
        for i in 0..sc.child_count() {
            let child = sc.child(i).unwrap();
            if child.kind() == "extends" {
                continue;
            }
            emit_type_use_sites(ctx, child, parent_idx);
        }
    }
    if let Some(impls) = node.child_by_field_name("interfaces") {
        for i in 0..impls.child_count() {
            let child = impls.child(i).unwrap();
            if child.kind() == "type_list" {
                for j in 0..child.child_count() {
                    let t = child.child(j).unwrap();
                    if matches!(t.kind(), "," | "implements" | "extends") {
                        continue;
                    }
                    emit_type_use_sites(ctx, t, parent_idx);
                }
            }
        }
    }
}

/// Walk a method or constructor's signature (return type, parameter
/// types, throws clause) and emit type-use nodes hard-linked under
/// `parent_idx`.
fn emit_method_signature_uses(ctx: &mut ParseContext, node: Node, parent_idx: usize) {
    if let Some(t) = node.child_by_field_name("type") {
        emit_type_use_sites(ctx, t, parent_idx);
    }
    if let Some(params) = node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            let p = params.child(i).unwrap();
            match p.kind() {
                "formal_parameter" => {
                    if let Some(t) = p.child_by_field_name("type") {
                        emit_type_use_sites(ctx, t, parent_idx);
                    }
                }
                "spread_parameter" => {
                    // tree-sitter-java's `spread_parameter` doesn't
                    // expose a `type` field; the type is the first
                    // child that isn't modifiers/`...`/declarator.
                    for j in 0..p.child_count() {
                        let part = p.child(j).unwrap();
                        if matches!(
                            part.kind(),
                            "..." | "modifiers" | "variable_declarator"
                        ) {
                            continue;
                        }
                        emit_type_use_sites(ctx, part, parent_idx);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "throws" {
            for j in 0..child.child_count() {
                let t = child.child(j).unwrap();
                if matches!(t.kind(), "throws" | ",") {
                    continue;
                }
                emit_type_use_sites(ctx, t, parent_idx);
            }
        }
    }
}

fn extract_enum(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let modifiers = extract_modifiers(node, ctx.source);

    let java_payload = JavaNodePayload::from(JavaTypeNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        kind: JavaTypeKind::Enum,
        type_parameters: Vec::new(),
        record_components: Vec::new(),
    });
    let jvm_payload = JvmNodePayload::from(JvmTypeNode {
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

    let java_payload = JavaNodePayload::from(JavaEnumConstantNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        enum_owner: enum_owner.clone(),
    });
    let jvm_payload = JvmNodePayload::from(JvmEnumConstantNode {
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

    let has_body = node.child_by_field_name("body").is_some();

    let java_payload = JavaNodePayload::from(JavaMethodNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        return_type: return_type.clone(),
        parameters: java_parameters,
        type_parameters: type_parameters.clone(),
        throws: Vec::new(),
        has_body,
    });
    let jvm_payload = JvmNodePayload::from(JvmMethodNode {
        header: jvm_header(ctx, &name, node, modifiers),
        owner,
        return_type: return_type.erasure(),
        parameters: jvm_parameters,
        type_parameters,
        throws: Vec::new(),
        enrichments: JvmEnrichments::default(),
    });
    let java_idx = emit_pair(ctx, java_payload, jvm_payload);
    emit_method_signature_uses(ctx, node, java_idx);
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

    let java_payload = JavaNodePayload::from(JavaConstructorNode {
        header: java_header(ctx, &name, node, modifiers.clone()),
        parameters: java_parameters,
        type_parameters: type_parameters.clone(),
        throws: Vec::new(),
    });
    let jvm_payload = JvmNodePayload::from(JvmConstructorNode {
        header: jvm_header(ctx, &name, node, modifiers),
        owner,
        parameters: jvm_parameters,
        type_parameters,
        throws: Vec::new(),
    });
    let java_idx = emit_pair(ctx, java_payload, jvm_payload);
    emit_method_signature_uses(ctx, node, java_idx);
}

fn extract_fields(ctx: &mut ParseContext, node: Node) {
    let modifiers = extract_modifiers(node, ctx.source);
    let type_node = node.child_by_field_name("type");
    let field_type = type_node
        .map(|n| parse_type_ref(n, ctx.source).to_core())
        .unwrap_or_else(|| TypeRef::simple("unknown"));
    let owner = parent_owner_fqn(ctx);

    // Field declarations can have multiple declarators: `int a, b, c;`
    // Each declarator emits its own JavaFieldNode + JvmFieldNode pair.
    // Per ADR-0029 each field also gets one JavaTypeUseNode per named
    // identifier in its type, hard-linked under the field. With shared
    // declarators each declarator gets an independent set of use-site
    // children — the use sites are field-local, not declaration-shared.
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "variable_declarator" {
            let name = match child.child_by_field_name("name") {
                Some(n) => node_text(n, ctx.source).to_string(),
                None => continue,
            };

            let java_payload = JavaNodePayload::from(JavaFieldNode {
                header: java_header(ctx, &name, child, modifiers.clone()),
                field_type: field_type.clone(),
                constant_value: None,
                initialized: false,
            });
            let jvm_payload = JvmNodePayload::from(JvmFieldNode {
                header: jvm_header(ctx, &name, child, modifiers.clone()),
                owner: owner.clone(),
                field_type: field_type.clone(),
                constant_value: None,
                initialized: false,
                enrichments: JvmEnrichments::default(),
            });
            let java_idx = emit_pair(ctx, java_payload, jvm_payload);
            if let Some(t) = type_node {
                emit_type_use_sites(ctx, t, java_idx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Per ADR-0005 the parse phase must be runnable on a rayon worker
    /// — meaning the parse output type has to be `Send`. Per ADR-0014
    /// RAII registration handles live on
    /// [`NodeData::handles`](beans_core::graph::NodeData::handles), not on
    /// the payload, which keeps every payload variant free of
    /// `Rc`-flavoured `!Send` taints. This is a static check that the
    /// invariant holds — a regression here would silently break
    /// workspace indexing parallelism.
    fn _assert_parse_output_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ParsedJavaFile>();
        assert_send::<JavaPlanPayload>();
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
                JavaPlanPayload::Java(j) => Some(j),
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

        // Plan layout (declarations only): Dog(Java), Dog(Jvm),
        // name(Java), name(Jvm), Dog ctor(Java), Dog ctor(Jvm),
        // getName(Java), getName(Jvm). Per ADR-0029 use-site nodes
        // (`JavaTypeUseNode`) interleave under their containing
        // declaration; their parent is the declaration, not the class.
        let dog_idx = 0;
        assert_eq!(parsed.plan[dog_idx].parent, None);
        assert_eq!(parsed.plan[dog_idx + 1].parent, Some(dog_idx));

        // Every *declaration* Java payload has the class as parent.
        // Use sites have their own (non-class) Java parents — the
        // declaration that contains them.
        for member in parsed.plan.iter().skip(2) {
            if let JavaPlanPayload::Java(j) = &member.payload {
                if j.header().is_some() {
                    assert_eq!(member.parent, Some(dog_idx));
                }
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

    /// Collect every JavaTypeUseNode in the plan.
    fn type_uses(plan: &[PendingNode]) -> Vec<&JavaTypeUseNode> {
        plan.iter()
            .filter_map(|p| match &p.payload {
                JavaPlanPayload::Java(JavaNodePayload::TypeUse(t)) => Some(t),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn type_uses_emit_for_supertype_and_field_and_method_signatures() {
        // Per ADR-0029 every named type identifier in a declaration
        // header emits one JavaTypeUseNode hard-linked under the
        // containing declaration. This test pins:
        //   1. Supertype + implements emit one use site each.
        //   2. Field type emits one use site.
        //   3. Method return type and parameter types emit one each.
        //   4. Throws clauses emit one per thrown type.
        //   5. Generic type arguments emit flat sibling use sites
        //      (Repository<User> → 2 nodes).
        let source = r#"
package com.example;
public class UserService extends BaseService implements Auditable {
    private Repository<User> users;
    public UserService(Repository<User> users) {}
    public Optional<User> findById(long id) throws NotFoundException { return null; }
}
"#;
        let parsed = parse(source);
        let names: Vec<&str> = type_uses(&parsed.plan)
            .iter()
            .map(|t| t.header.name.as_str())
            .collect();

        // Slice 1 is signatures-only — no body content. The expected
        // multiset:
        //   class header:        BaseService, Auditable
        //   field:               Repository, User
        //   constructor params:  Repository, User
        //   method return type:  Optional, User
        //   method param:        (none — `long` is primitive)
        //   method throws:       NotFoundException
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec![
                "Auditable",
                "BaseService",
                "NotFoundException",
                "Optional",
                "Repository",
                "Repository",
                "User",
                "User",
                "User",
            ],
            "unexpected use-site name multiset; full list: {:?}",
            names
        );
    }

    #[test]
    fn type_use_span_is_identifier_only_for_scoped_types() {
        // Per ADR-0029 the load-bearing invariant: a JavaTypeUseNode's
        // `location` spans the identifier text only — for
        // `com.example.Service`, the span is `Service`, never the full
        // qualified expression. Mechanical rename rewrites the span.
        let source = "package com.example;\npublic class A extends com.other.Base {}\n";
        let parsed = parse(source);

        let uses = type_uses(&parsed.plan);
        let base = uses
            .iter()
            .find(|t| t.header.name == "Base")
            .expect("Base use site emitted");

        // The span text on the source must be exactly "Base", not
        // "com.other.Base". Use the location's row/col to extract.
        let loc = &base.header.location;
        assert_eq!(loc.start_line, 1, "Base is on the second source line (0-indexed)");
        let line: &str = source.lines().nth(loc.start_line as usize).unwrap();
        let span_text = &line[loc.start_col as usize..loc.end_col as usize];
        assert_eq!(span_text, "Base", "span must cover only the rightmost identifier");
    }

    #[test]
    fn type_use_candidate_fqns_prefer_explicit_imports() {
        // Per ADR-0029 candidate FQN order: explicit single-imports
        // first, then same-package, then java.lang, then wildcard
        // imports.
        let source = r#"
package com.example;
import com.other.Service;
import java.util.*;
public class App {
    private Service svc;
    private List<String> items;
}
"#;
        let parsed = parse(source);
        let uses = type_uses(&parsed.plan);

        let svc = uses
            .iter()
            .find(|t| t.header.name == "Service")
            .expect("Service use site emitted");
        let svc_fqns: Vec<&str> = svc.header.candidate_fqns.iter().map(|f| f.as_str()).collect();
        assert_eq!(svc_fqns[0], "com.other.Service", "explicit import wins");

        let list = uses
            .iter()
            .find(|t| t.header.name == "List")
            .expect("List use site emitted");
        let list_fqns: Vec<&str> = list.header.candidate_fqns.iter().map(|f| f.as_str()).collect();
        // No explicit import for List → same-package, then java.lang,
        // then wildcard import (java.util.*).
        assert_eq!(list_fqns[0], "com.example.List");
        assert_eq!(list_fqns[1], "java.lang.List");
        assert_eq!(list_fqns[2], "java.util.List");
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
                JavaPlanPayload::Jvm(JvmNodePayload::Method(m)) if m.header.name == "process" => {
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
            .position(|p| matches!(&p.payload, JavaPlanPayload::Java(j) if j.header().is_some_and(|h| h.name == "Inner")))
            .expect("inner not found");
        assert_eq!(parsed.plan[inner_idx].parent, Some(outer_idx));
        let value_idx = parsed
            .plan
            .iter()
            .position(|p| matches!(&p.payload, JavaPlanPayload::Java(j) if j.header().is_some_and(|h| h.name == "value")))
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
