//! Actor bridge between the async LSP boundary and the single-threaded
//! graph core.
//!
//! Per ADR-0018 the graph (`Graph<NodePayload>` + `Registries`) is
//! `!Send + !Sync` — it uses `Rc<RefCell<...>>` because each LSP
//! workspace has its own state and atomic synchronization is wasted
//! work. tower-lsp's async server, by contrast, requires the backend
//! to be `Send + Sync`.
//!
//! The bridge that ARCHITECTURE.md describes ("no async colours leak
//! inwards") is implemented here as an actor: a single dedicated
//! thread owns the [`ServerState`] for the LSP's lifetime and pulls
//! commands from a [`tokio::sync::mpsc`] channel. The async LSP
//! handlers serialize each request into a [`Cmd`], send it via the
//! channel's `Sender`, and `.await` the reply on a per-command
//! [`tokio::sync::oneshot`] channel.
//!
//! Why this shape:
//! - The state stays genuinely single-threaded (ADR-0018).
//! - Registries keep their cheap `Rc<RefCell<...>>` storage (ADR-0015).
//! - The async surface is `Send + Sync` because `mpsc::Sender` and
//!   tower-lsp's `Client` are both `Send + Sync`.
//! - Worker shutdown is automatic: when every `Sender` drops on server
//!   exit, the `Receiver::recv()` loop returns `None` and the thread
//!   exits cleanly.

use std::path::{Path, PathBuf};

use beans_core::diagnostics::compute_diagnostics;
use beans_core::languages::java;
use beans_core::payload::NodePayload;
use beans_core::{Modifier, SymbolKind};
use tokio::sync::{mpsc, oneshot};
use tower_lsp::lsp_types::SymbolKind as LspSymbolKind;
use tower_lsp::lsp_types::*;

use crate::backend::ServerState;
use crate::hover;
use crate::workspace;

/// Bounded channel size between async handlers and the worker. Sized
/// to "comfortably more than any plausible in-flight request burst."
/// Editors with multi-cursor / find-all-references can fan a few
/// dozen `references` requests at once; 64 leaves headroom without
/// being so large it hides actor stalls.
const COMMAND_CHANNEL_SIZE: usize = 64;

/// One LSP request, packaged for the worker. Each variant carries
/// inputs plus a `oneshot::Sender` for the reply. Variants for
/// `did_*` notifications return [`DiagnosticsForFile`] so the async
/// handler can publish without re-locking.
pub enum Cmd {
    Initialize {
        root: Option<PathBuf>,
        reply: oneshot::Sender<usize>,
    },
    DidOpen {
        uri: Url,
        text: String,
        reply: oneshot::Sender<DiagnosticsForFile>,
    },
    DidChange {
        uri: Url,
        text: String,
        reply: oneshot::Sender<DiagnosticsForFile>,
    },
    DidSave {
        uri: Url,
        reply: oneshot::Sender<DiagnosticsForFile>,
    },
    GotoDefinition {
        uri: Url,
        pos: Position,
        reply: oneshot::Sender<Option<GotoDefinitionResponse>>,
    },
    References {
        uri: Url,
        pos: Position,
        reply: oneshot::Sender<Option<Vec<Location>>>,
    },
    Hover {
        uri: Url,
        pos: Position,
        reply: oneshot::Sender<Option<Hover>>,
    },
    DocumentSymbol {
        uri: Url,
        reply: oneshot::Sender<Option<DocumentSymbolResponse>>,
    },
}

/// Diagnostics computed for one file, paired with that file's URI so
/// the async handler can publish without consulting state again.
pub struct DiagnosticsForFile {
    pub uri: Url,
    pub diagnostics: Vec<Diagnostic>,
}

/// Owning handle for the worker thread. Cloning the inner sender is
/// cheap (`Sender: Clone`) and is how the LSP backend hands access to
/// async handlers.
#[derive(Clone)]
pub struct WorkerHandle {
    sender: mpsc::Sender<Cmd>,
}

impl WorkerHandle {
    /// Send a command and await its reply. Returns `None` if the
    /// worker has exited (e.g., during shutdown after the channel
    /// closes); LSP handlers treat that as "no result" rather than
    /// surfacing the error.
    pub async fn send<R>(
        &self,
        build_cmd: impl FnOnce(oneshot::Sender<R>) -> Cmd,
    ) -> Option<R> {
        let (tx, rx) = oneshot::channel();
        let cmd = build_cmd(tx);
        if self.sender.send(cmd).await.is_err() {
            return None;
        }
        rx.await.ok()
    }
}

/// Spawn the worker thread that owns the [`ServerState`]. Returns a
/// handle whose senders the LSP backend clones to dispatch commands.
///
/// The worker runs on a dedicated `std::thread` (not a tokio blocking
/// task) so it is cleanly distinct from the async runtime and never
/// shares its allocator state with rayon — which is itself driven by
/// blocking parses inside the worker per ADR-0005.
///
/// Panic policy: if a single command handler panics, it tears down the
/// worker thread; the next command via `WorkerHandle::send` returns
/// `None` because the channel close races the panic. Per ADR-0018
/// "RefCell borrow violations are bugs to fix, not recoverable
/// conditions"; we don't catch_unwind around individual commands.
pub fn spawn_worker() -> WorkerHandle {
    let (tx, rx) = mpsc::channel(COMMAND_CHANNEL_SIZE);
    std::thread::Builder::new()
        .name("beans-lsp-worker".to_string())
        .spawn(move || worker_loop(rx))
        .expect("failed to spawn beans-lsp-worker thread");
    WorkerHandle { sender: tx }
}

fn worker_loop(mut rx: mpsc::Receiver<Cmd>) {
    let mut state = ServerState::new();
    while let Some(cmd) = rx.blocking_recv() {
        handle(cmd, &mut state);
    }
}

fn handle(cmd: Cmd, state: &mut ServerState) {
    match cmd {
        Cmd::Initialize { root, reply } => {
            state.workspace_root = root.clone();
            let resolved = root
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            workspace::index_workspace(&resolved, state);
            let _ = reply.send(state.file_roots.len());
        }
        Cmd::DidOpen { uri, text, reply } => {
            // Update open_files first, then reindex against the same
            // text. If reindex panics the open_files state is still
            // consistent with the graph that existed before — the
            // panic kills the worker thread per ADR-0018, so there's
            // no recovery path that would observe the partial state.
            state.open_files.insert(uri.clone(), text.clone());
            let diagnostics = reindex_and_diagnose(state, &uri, Some(text));
            let _ = reply.send(DiagnosticsForFile { uri, diagnostics });
        }
        Cmd::DidChange { uri, text, reply } => {
            state.open_files.insert(uri.clone(), text.clone());
            let diagnostics = reindex_and_diagnose(state, &uri, Some(text));
            let _ = reply.send(DiagnosticsForFile { uri, diagnostics });
        }
        Cmd::DidSave { uri, reply } => {
            // No new text supplied; re-read from disk.
            let diagnostics = reindex_and_diagnose(state, &uri, None);
            let _ = reply.send(DiagnosticsForFile {
                uri,
                diagnostics,
            });
        }
        Cmd::GotoDefinition { uri, pos, reply } => {
            let _ = reply.send(handle_goto_definition(state, &uri, pos));
        }
        Cmd::References { uri, pos, reply } => {
            let _ = reply.send(handle_references(state, &uri, pos));
        }
        Cmd::Hover { uri, pos, reply } => {
            let _ = reply.send(handle_hover(state, &uri, pos));
        }
        Cmd::DocumentSymbol { uri, reply } => {
            let _ = reply.send(handle_document_symbol(state, &uri));
        }
    }
}

// ---- Command handlers (sync, run on the worker thread) ----

fn reindex_and_diagnose(
    state: &mut ServerState,
    uri: &Url,
    text: Option<String>,
) -> Vec<Diagnostic> {
    let path = match uri.to_file_path() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let source = match text {
        Some(t) => t,
        None => match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        },
    };
    workspace::integrate_source(state, &path, &source);
    diagnostics_for_path(state, &path)
}

fn diagnostics_for_path(state: &ServerState, path: &Path) -> Vec<Diagnostic> {
    compute_diagnostics(&state.graph, &state.registries, path)
        .into_iter()
        .map(|d| Diagnostic {
            range: Range {
                start: Position {
                    line: d.location.start_line,
                    character: d.location.start_col,
                },
                end: Position {
                    line: d.location.end_line,
                    character: d.location.end_col,
                },
            },
            severity: Some(match d.severity {
                beans_core::diagnostics::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
                beans_core::diagnostics::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
                beans_core::diagnostics::DiagnosticSeverity::Information => {
                    DiagnosticSeverity::INFORMATION
                }
                beans_core::diagnostics::DiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
            }),
            code: d.code.map(NumberOrString::String),
            message: d.message,
            ..Default::default()
        })
        .collect()
}

fn handle_goto_definition(
    state: &ServerState,
    uri: &Url,
    pos: Position,
) -> Option<GotoDefinitionResponse> {
    let text = document_text(state, uri)?;
    let file = uri.to_file_path().ok()?;
    let id = resolve_at_cursor(state, &file, &text, pos)?;
    let node = state.graph.get(id)?;
    let view = payload_view(&node.payload)?;
    let lsp_loc = view.location.and_then(location_to_lsp)?;
    Some(GotoDefinitionResponse::Scalar(lsp_loc))
}

fn handle_references(
    state: &ServerState,
    uri: &Url,
    pos: Position,
) -> Option<Vec<Location>> {
    let text = document_text(state, uri)?;
    let word = word_at_position(&text, pos.line, pos.character)?;

    // Mirror prototype `find_references_by_name`: every Java payload
    // whose simple name matches.
    let mut locations = Vec::new();
    for (_id, node) in state.graph.iter() {
        let view = match payload_view(&node.payload) {
            Some(v) => v,
            None => continue,
        };
        if view.name == word
            && let Some(loc) = view.location.and_then(location_to_lsp)
        {
            locations.push(loc);
        }
    }
    if locations.is_empty() {
        None
    } else {
        Some(locations)
    }
}

fn handle_hover(state: &ServerState, uri: &Url, pos: Position) -> Option<Hover> {
    let text = document_text(state, uri)?;
    let file = uri.to_file_path().ok()?;
    let id = resolve_at_cursor(state, &file, &text, pos)?;
    let payload = &state.graph.get(id)?.payload;
    hover::build_hover_text(payload).map(|markdown| Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: None,
    })
}

fn handle_document_symbol(state: &ServerState, uri: &Url) -> Option<DocumentSymbolResponse> {
    let file = uri.to_file_path().ok()?;
    let roots = state.file_roots.get(&file)?.clone();
    let mut result = Vec::new();
    for root in roots {
        if let Some(node) = state.graph.get(root)
            && matches!(node.payload, NodePayload::Jvm(_))
        {
            // JVM projection siblings live alongside their Java
            // counterparts; only the Java root becomes a top-level
            // document symbol.
            continue;
        }
        if let Some(sym) = build_document_symbol(state, &file, root) {
            result.push(sym);
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(DocumentSymbolResponse::Nested(result))
    }
}

#[allow(deprecated)] // DocumentSymbol::deprecated is the lsp_types field name.
fn build_document_symbol(
    state: &ServerState,
    file: &Path,
    id: beans_core::graph::NodeId,
) -> Option<DocumentSymbol> {
    let node = state.graph.get(id)?;
    let view = payload_view(&node.payload)?;
    let range = view
        .location
        .map(|loc| Range {
            start: Position {
                line: loc.start_line,
                character: loc.start_col,
            },
            end: Position {
                line: loc.end_line,
                character: loc.end_col,
            },
        })
        .unwrap_or_default();

    let children: Vec<DocumentSymbol> = node
        .children
        .iter()
        .copied()
        .filter_map(|child_id| {
            let child = state.graph.get(child_id)?;
            if matches!(child.payload, NodePayload::Jvm(_)) {
                return None;
            }
            if let Some(child_view) = payload_view(&child.payload)
                && let Some(loc) = child_view.location
                && loc.file != file
            {
                return None;
            }
            build_document_symbol(state, file, child_id)
        })
        .collect();

    let detail = match &node.payload {
        NodePayload::Java(java::JavaNodePayload::Method(m)) => {
            let params: Vec<String> =
                m.parameters.iter().map(|p| p.param_type.to_string()).collect();
            Some(format!("({}) -> {}", params.join(", "), m.return_type))
        }
        NodePayload::Java(java::JavaNodePayload::Field(f)) => Some(f.field_type.to_string()),
        _ => None,
    };

    Some(DocumentSymbol {
        name: view.name.to_string(),
        detail,
        kind: symbol_kind_to_lsp(view.kind),
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    })
}

// ---- Local helpers (mirror the legacy LSP shape) ----

fn document_text(state: &ServerState, uri: &Url) -> Option<String> {
    if let Some(t) = state.open_files.get(uri) {
        return Some(t.clone());
    }
    let path = uri.to_file_path().ok()?;
    std::fs::read_to_string(path).ok()
}

fn resolve_at_cursor(
    state: &ServerState,
    file: &Path,
    text: &str,
    pos: Position,
) -> Option<beans_core::graph::NodeId> {
    let imports = state
        .file_imports
        .get(file)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let pkg = state
        .file_packages
        .get(file)
        .map(|s| s.as_str())
        .unwrap_or("");

    if let Some(compound) = compound_at_position(text, pos.line, pos.character)
        && let Some(id) = java::resolve_compound_name(
            &compound,
            imports,
            pkg,
            &state.registries,
            &state.graph,
        )
    {
        return Some(id);
    }

    let word = word_at_position(text, pos.line, pos.character)?;
    java::resolve_name(&word, imports, pkg, &state.registries, &state.graph)
}

fn word_at_position(text: &str, line: u32, character: u32) -> Option<String> {
    let target_line = text.lines().nth(line as usize)?;
    let col = character as usize;
    if col > target_line.len() {
        return None;
    }
    let bytes = target_line.as_bytes();
    let mut start = col;
    while start > 0 && is_identifier_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && is_identifier_char(bytes[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    Some(target_line[start..end].to_string())
}

fn compound_at_position(text: &str, line: u32, character: u32) -> Option<String> {
    let target_line = text.lines().nth(line as usize)?;
    let col = character as usize;
    if col > target_line.len() {
        return None;
    }
    let bytes = target_line.as_bytes();
    let mut start = col;
    while start > 0 && (is_identifier_char(bytes[start - 1]) || bytes[start - 1] == b'.') {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && (is_identifier_char(bytes[end]) || bytes[end] == b'.') {
        end += 1;
    }
    if start == end {
        return None;
    }
    let text = target_line[start..end].trim_matches('.');
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn is_identifier_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

fn location_to_lsp(loc: &beans_core::Location) -> Option<Location> {
    let uri = Url::from_file_path(&loc.file).ok()?;
    Some(Location {
        uri,
        range: Range {
            start: Position {
                line: loc.start_line,
                character: loc.start_col,
            },
            end: Position {
                line: loc.end_line,
                character: loc.end_col,
            },
        },
    })
}

/// Map a JVM-shaped `beans_core::SymbolKind` to the LSP wire's
/// `SymbolKind`. Per-language kinds (Kotlin's `Object`, Scala's
/// `Trait`, Clojure's `Namespace`, etc.) live in their respective
/// per-language enums (`crate::languages::<lang>::SymbolKind`); when
/// those payloads land they'll need their own `to_lsp` mapping.
///
/// The `EnumConstant` arm is reachable in principle but unreachable
/// today: `payload_view` collapses `JavaNodePayload::EnumConstant`
/// into `SymbolKind::Field` for spec-test stability. Backlog #032
/// tracks whether to surface `EnumConstant` distinctly.
fn symbol_kind_to_lsp(kind: SymbolKind) -> LspSymbolKind {
    match kind {
        SymbolKind::Class => LspSymbolKind::CLASS,
        SymbolKind::Interface => LspSymbolKind::INTERFACE,
        SymbolKind::Enum => LspSymbolKind::ENUM,
        SymbolKind::Record => LspSymbolKind::STRUCT,
        SymbolKind::Annotation => LspSymbolKind::CLASS,
        SymbolKind::Method => LspSymbolKind::METHOD,
        SymbolKind::Constructor => LspSymbolKind::CONSTRUCTOR,
        SymbolKind::Field | SymbolKind::EnumConstant => LspSymbolKind::FIELD,
        SymbolKind::Parameter => LspSymbolKind::VARIABLE,
        SymbolKind::Package => LspSymbolKind::NAMESPACE,
    }
}

/// Project a Java payload onto the (kind, fqn, name, location, modifiers)
/// view every LSP handler needs. JVM-projection nodes return `None`;
/// resolution always lands on the Java side.
struct PayloadView<'a> {
    kind: SymbolKind,
    name: &'a str,
    #[allow(dead_code)] // not yet read by handlers; kept for symmetry.
    fqn: &'a str,
    location: Option<&'a beans_core::Location>,
    #[allow(dead_code)] // not yet read by handlers; kept for symmetry.
    modifiers: &'a [Modifier],
}

fn payload_view(payload: &NodePayload) -> Option<PayloadView<'_>> {
    use java::{JavaNodePayload, JavaTypeKind};
    let java = match payload {
        NodePayload::Java(j) => j,
        NodePayload::Jvm(_) => return None,
    };
    let view = match java {
        JavaNodePayload::Type(n) => {
            let kind = match n.kind {
                JavaTypeKind::Class => SymbolKind::Class,
                JavaTypeKind::Interface => SymbolKind::Interface,
                JavaTypeKind::Enum => SymbolKind::Enum,
                JavaTypeKind::Record => SymbolKind::Record,
                JavaTypeKind::Annotation => SymbolKind::Annotation,
            };
            PayloadView {
                kind,
                name: &n.header.name,
                fqn: n.header.fqn.as_str(),
                location: n.header.location.as_ref(),
                modifiers: &n.header.modifiers,
            }
        }
        JavaNodePayload::Method(n) => PayloadView {
            kind: SymbolKind::Method,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::Constructor(n) => PayloadView {
            kind: SymbolKind::Constructor,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::Field(n) => PayloadView {
            kind: SymbolKind::Field,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::EnumConstant(n) => PayloadView {
            kind: SymbolKind::Field,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::AnnotationElement(n) => PayloadView {
            kind: SymbolKind::Method,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::Parameter(p) => PayloadView {
            kind: SymbolKind::Parameter,
            name: &p.name,
            fqn: "",
            location: None,
            modifiers: &[],
        },
        JavaNodePayload::Package(n) => PayloadView {
            kind: SymbolKind::Package,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
    };
    Some(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `WorkerHandle` and `Cmd` are the surface tower-lsp sees through
    /// the backend. They must be `Send + Sync` so `BeanBackend` can be
    /// `Send + Sync`. The state owned by the worker thread is `!Send`
    /// per ADR-0018, but it never crosses thread boundaries.
    fn _assert_handle_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WorkerHandle>();
    }

    #[test]
    fn word_at_position_single_line() {
        let text = "public class MyClass extends Base {\n    private String name;\n}";
        assert_eq!(word_at_position(text, 0, 15), Some("MyClass".to_string()));
        assert_eq!(word_at_position(text, 0, 29), Some("Base".to_string()));
        assert_eq!(word_at_position(text, 1, 19), Some("name".to_string()));
    }

    #[test]
    fn word_at_position_edge_cases() {
        let text = "int x;";
        assert_eq!(word_at_position(text, 0, 0), Some("int".to_string()));
        assert_eq!(word_at_position(text, 0, 4), Some("x".to_string()));
        assert_eq!(word_at_position(text, 0, 5), Some("x".to_string()));
        assert_eq!(word_at_position(text, 0, 100), None);
    }

    #[test]
    fn compound_at_position_picks_dotted_name() {
        let text = "MyClass.doWork()";
        assert_eq!(
            compound_at_position(text, 0, 10),
            Some("MyClass.doWork".to_string())
        );
        assert_eq!(
            compound_at_position(text, 0, 3),
            Some("MyClass.doWork".to_string())
        );
    }

    #[test]
    fn symbol_kind_to_lsp_covers_canonical_kinds() {
        assert_eq!(symbol_kind_to_lsp(SymbolKind::Class), LspSymbolKind::CLASS);
        assert_eq!(
            symbol_kind_to_lsp(SymbolKind::Interface),
            LspSymbolKind::INTERFACE
        );
        assert_eq!(symbol_kind_to_lsp(SymbolKind::Method), LspSymbolKind::METHOD);
        assert_eq!(symbol_kind_to_lsp(SymbolKind::Field), LspSymbolKind::FIELD);
        assert_eq!(
            symbol_kind_to_lsp(SymbolKind::Constructor),
            LspSymbolKind::CONSTRUCTOR
        );
    }

    #[test]
    fn integrate_then_resolve_within_file() {
        let mut state = ServerState::new();
        let path = std::path::Path::new("src/Dog.java");
        let source = r#"
package com.example;
public class Dog {
    private String name;
    public String getName() { return name; }
}
"#;
        workspace::integrate_source(&mut state, path, source);

        let imports = state.file_imports.get(path).cloned().unwrap_or_default();
        let pkg = state.file_packages.get(path).cloned().unwrap_or_default();

        let dog_id = java::resolve_name(
            "Dog",
            &imports,
            &pkg,
            &state.registries,
            &state.graph,
        )
        .expect("Dog should resolve in com.example");
        let dog_node = state.graph.get(dog_id).unwrap();
        let dog_view = payload_view(&dog_node.payload).unwrap();
        assert_eq!(dog_view.fqn, "com.example.Dog");
        assert_eq!(dog_view.kind, SymbolKind::Class);

        let getter_id = java::resolve_compound_name(
            "Dog.getName",
            &imports,
            &pkg,
            &state.registries,
            &state.graph,
        )
        .expect("Dog.getName should resolve");
        let getter_view = payload_view(&state.graph.get(getter_id).unwrap().payload).unwrap();
        assert_eq!(getter_view.fqn, "com.example.Dog.getName");
        assert_eq!(getter_view.kind, SymbolKind::Method);
    }

    #[test]
    fn cross_file_resolution_via_imports() {
        let mut state = ServerState::new();
        let model = std::path::Path::new("src/User.java");
        let svc = std::path::Path::new("src/UserService.java");

        workspace::integrate_source(
            &mut state,
            model,
            "package com.example.model;\npublic class User {\n    public String getName() { return null; }\n}\n",
        );
        workspace::integrate_source(
            &mut state,
            svc,
            "package com.example.service;\nimport com.example.model.User;\npublic class UserService {\n    public User findUser() { return null; }\n}\n",
        );

        let imports = state.file_imports.get(svc).cloned().unwrap_or_default();
        let pkg = state.file_packages.get(svc).cloned().unwrap_or_default();
        let user_id = java::resolve_name(
            "User",
            &imports,
            &pkg,
            &state.registries,
            &state.graph,
        )
        .expect("import-resolved User");
        let view = payload_view(&state.graph.get(user_id).unwrap().payload).unwrap();
        assert_eq!(view.fqn, "com.example.model.User");
    }

    #[test]
    fn reindex_replaces_old_symbols() {
        let mut state = ServerState::new();
        let path = std::path::Path::new("src/Foo.java");
        workspace::integrate_source(
            &mut state,
            path,
            "package com.test;\npublic class Foo { public void oldMethod() {} }",
        );
        assert!(java::lookup_fqn(&state.registries, "com.test.Foo.oldMethod").is_some());

        workspace::integrate_source(
            &mut state,
            path,
            "package com.test;\npublic class Foo { public void newMethod() {} }",
        );
        assert!(
            java::lookup_fqn(&state.registries, "com.test.Foo.oldMethod").is_none(),
            "old method should be unregistered after reindex"
        );
        assert!(java::lookup_fqn(&state.registries, "com.test.Foo.newMethod").is_some());
    }

    #[test]
    fn references_walks_graph_and_returns_locations() {
        // The references handler walks every Java payload and returns
        // every location whose simple name matches. This test verifies
        // both the walk and the location-mapping.
        let mut state = ServerState::new();
        let path = std::path::Path::new("/tmp/Service.java");
        workspace::integrate_source(
            &mut state,
            path,
            "package com.example;\npublic class Service {\n    public void process() {}\n    public void process(String s) {}\n}\n",
        );

        // Two methods named `process` exist; references should find
        // both. We use a temp /tmp path so URL conversion succeeds.
        let uri = Url::from_file_path(path).unwrap();
        let text = std::fs::read_to_string(path).unwrap_or_else(|_| {
            // The file isn't really on disk; seed open_files so
            // document_text() finds it.
            String::new()
        });
        if !text.is_empty() {
            // path existed; read worked
        }
        // Seed open_files explicitly so document_text() returns
        // something even though we never wrote to disk.
        state.open_files.insert(
            uri.clone(),
            "package com.example;\npublic class Service {\n    public void process() {}\n    public void process(String s) {}\n}\n".to_string(),
        );

        // Cursor on the first `process` declaration.
        let result = handle_references(&state, &uri, Position { line: 2, character: 16 });
        let locations = result.expect("references should hit");
        assert_eq!(locations.len(), 2, "two `process` methods expected");
    }

    #[test]
    fn diagnostics_handler_returns_empty_for_step_6_plumbing() {
        // Step 6 plumbing: rules are not implemented; compute_diagnostics
        // returns Vec::new(). This test pins that contract so the
        // eventual rule-engine landing (backlog #015) makes the
        // semantic shift explicit.
        let state = ServerState::new();
        let path = std::path::Path::new("/tmp/anything.java");
        let diagnostics = diagnostics_for_path(&state, path);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn document_symbol_outline_for_class() {
        let mut state = ServerState::new();
        let path = std::path::Path::new("src/Dog.java");
        workspace::integrate_source(
            &mut state,
            path,
            r#"
package com.example;
public class Dog {
    private String name;
    public Dog(String name) { this.name = name; }
    public String getName() { return name; }
}
"#,
        );
        let roots = state.file_roots.get(path).cloned().unwrap_or_default();
        let mut symbols = Vec::new();
        for root in roots {
            if let Some(node) = state.graph.get(root)
                && matches!(node.payload, NodePayload::Jvm(_))
            {
                continue;
            }
            if let Some(sym) = build_document_symbol(&state, path, root) {
                symbols.push(sym);
            }
        }
        assert_eq!(symbols.len(), 1);
        let dog = &symbols[0];
        assert_eq!(dog.name, "Dog");
        assert_eq!(dog.kind, LspSymbolKind::CLASS);

        let children = dog.children.as_ref().expect("Dog has children");
        let names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"name"), "name field expected");
        assert!(names.contains(&"Dog"), "constructor expected");
        assert!(names.contains(&"getName"), "getName method expected");

        let getter = children.iter().find(|c| c.name == "getName").unwrap();
        assert_eq!(getter.kind, LspSymbolKind::METHOD);
        assert_eq!(getter.detail.as_deref(), Some("() -> String"));
    }
}
