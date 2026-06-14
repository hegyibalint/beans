//! Actor bridge between the async LSP boundary and the single-threaded
//! graph core.
//!
//! Per ADR-0018 the engine ([`beans::Workspace`], holding the graph +
//! registries) is `!Send + !Sync` — it uses `Rc<RefCell<...>>` because
//! each LSP workspace has its own state and atomic synchronization is
//! wasted work. tower-lsp's async server, by contrast, requires the
//! backend to be `Send + Sync`.
//!
//! The bridge that ARCHITECTURE.md describes ("no async colours leak
//! inwards") is implemented here as an actor: a single dedicated
//! thread owns the [`ServerState`] for the LSP's lifetime and pulls
//! commands from a [`tokio::sync::mpsc`] channel. The async LSP
//! handlers serialize each request into a [`Cmd`], send it via the
//! channel's `Sender`, and `.await` the reply on a per-command
//! [`tokio::sync::oneshot`] channel.
//!
//! Each handler is now a thin translation layer: convert the protocol
//! request (`Url`, `Position`) into facade inputs (path, line, column),
//! call the matching [`beans::Workspace`] method, and map the domain
//! result (`NodeId`, `beans::Location`, `beans::DocSymbol`,
//! `beans::Fix`, `beans::Diagnostic`) back onto the wire types. The
//! indexing and resolution mechanics live in the facade.
//!
//! Why this shape:
//! - The state stays genuinely single-threaded (ADR-0018).
//! - Registries keep their cheap `Rc<RefCell<...>>` storage (ADR-0015).
//! - The async surface is `Send + Sync` because `mpsc::Sender` and
//!   tower-lsp's `Client` are both `Send + Sync`.
//! - Worker shutdown is automatic: when every `Sender` drops on server
//!   exit, the `Receiver::recv()` loop returns `None` and the thread
//!   exits cleanly.

use std::path::PathBuf;

use beans::{DocSymbol, Location as BeansLocation, SymbolKind};
use tokio::sync::{mpsc, oneshot};
use tower_lsp::lsp_types::SymbolKind as LspSymbolKind;
use tower_lsp::lsp_types::*;

use crate::backend::ServerState;
use crate::hover;

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
    CodeAction {
        uri: Url,
        pos: Position,
        reply: oneshot::Sender<Option<Vec<CodeActionOrCommand>>>,
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
    pub async fn send<R>(&self, build_cmd: impl FnOnce(oneshot::Sender<R>) -> Cmd) -> Option<R> {
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
/// blocking parses inside the facade per ADR-0005.
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
            let resolved = root
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let count = state.workspace.index_workspace(&resolved);
            let _ = reply.send(count);
        }
        Cmd::DidOpen { uri, text, reply } => {
            let diagnostics = reindex_and_diagnose(state, &uri, Some(text));
            let _ = reply.send(DiagnosticsForFile { uri, diagnostics });
        }
        Cmd::DidChange { uri, text, reply } => {
            let diagnostics = reindex_and_diagnose(state, &uri, Some(text));
            let _ = reply.send(DiagnosticsForFile { uri, diagnostics });
        }
        Cmd::DidSave { uri, reply } => {
            // No new text supplied; the facade re-reads from disk.
            let diagnostics = reindex_and_diagnose(state, &uri, None);
            let _ = reply.send(DiagnosticsForFile { uri, diagnostics });
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
        Cmd::CodeAction { uri, pos, reply } => {
            let _ = reply.send(handle_code_action(state, &uri, pos));
        }
        Cmd::DocumentSymbol { uri, reply } => {
            let _ = reply.send(handle_document_symbol(state, &uri));
        }
    }
}

// ---- Command handlers (sync, run on the worker thread) ----
//
// Each handler is protocol-in / protocol-out around one facade call.

fn reindex_and_diagnose(
    state: &mut ServerState,
    uri: &Url,
    text: Option<String>,
) -> Vec<Diagnostic> {
    let Ok(path) = uri.to_file_path() else {
        return Vec::new();
    };
    match text {
        Some(t) => {
            state.workspace.update_file(&path, &t);
        }
        None => {
            state.workspace.reindex_from_disk(&path);
        }
    }
    state
        .workspace
        .diagnostics(&path)
        .into_iter()
        .map(to_lsp_diagnostic)
        .collect()
}

fn handle_goto_definition(
    state: &ServerState,
    uri: &Url,
    pos: Position,
) -> Option<GotoDefinitionResponse> {
    let path = uri.to_file_path().ok()?;
    let loc = state
        .workspace
        .definition_at(&path, pos.line, pos.character)?;
    location_to_lsp(&loc).map(GotoDefinitionResponse::Scalar)
}

fn handle_references(state: &ServerState, uri: &Url, pos: Position) -> Option<Vec<Location>> {
    let path = uri.to_file_path().ok()?;
    let locations: Vec<Location> = state
        .workspace
        .references_at(&path, pos.line, pos.character)
        .iter()
        .filter_map(location_to_lsp)
        .collect();
    if locations.is_empty() {
        None
    } else {
        Some(locations)
    }
}

fn handle_hover(state: &ServerState, uri: &Url, pos: Position) -> Option<Hover> {
    let path = uri.to_file_path().ok()?;
    let payload = state.workspace.hover_at(&path, pos.line, pos.character)?;
    hover::build_hover_text(payload).map(|markdown| Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: None,
    })
}

/// Pull half of auto-import: stateless request-time recompute. The
/// facade re-derives the fixes from the *current* graph, so the action
/// can never act on stale state (ADR-0028's rule, structural).
fn handle_code_action(
    state: &ServerState,
    uri: &Url,
    pos: Position,
) -> Option<Vec<CodeActionOrCommand>> {
    let path = uri.to_file_path().ok()?;
    let fixes = state
        .workspace
        .quick_fixes_at(&path, pos.line, pos.character);
    if fixes.is_empty() {
        return None;
    }
    Some(
        fixes
            .into_iter()
            .map(|f| fix_to_code_action(uri, f))
            .collect(),
    )
}

fn handle_document_symbol(state: &ServerState, uri: &Url) -> Option<DocumentSymbolResponse> {
    let path = uri.to_file_path().ok()?;
    let symbols = state.workspace.document_symbols(&path);
    if symbols.is_empty() {
        return None;
    }
    Some(DocumentSymbolResponse::Nested(
        symbols.iter().map(doc_symbol_to_lsp).collect(),
    ))
}

// ---- Domain -> protocol conversions (the LSP rim) ----

fn to_lsp_diagnostic(d: beans::Diagnostic) -> Diagnostic {
    Diagnostic {
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
            beans::diagnostics::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
            beans::diagnostics::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
            beans::diagnostics::DiagnosticSeverity::Information => DiagnosticSeverity::INFORMATION,
            beans::diagnostics::DiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
        }),
        code: d.code.map(NumberOrString::String),
        message: d.message,
        ..Default::default()
    }
}

fn location_to_lsp(loc: &BeansLocation) -> Option<Location> {
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

/// Map a domain [`beans::Fix`] onto the protocol envelope. The only
/// LSP-aware step: `SourceEdit` spans become `TextEdit`s in a
/// `WorkspaceEdit` keyed by the document's URI.
fn fix_to_code_action(uri: &Url, fix: beans::Fix) -> CodeActionOrCommand {
    let edits: Vec<TextEdit> = fix
        .edits
        .iter()
        .map(|e| TextEdit {
            range: Range {
                start: Position {
                    line: e.location.start_line,
                    character: e.location.start_col,
                },
                end: Position {
                    line: e.location.end_line,
                    character: e.location.end_col,
                },
            },
            new_text: e.new_text.clone(),
        })
        .collect();
    let changes = std::collections::HashMap::from([(uri.clone(), edits)]);
    CodeActionOrCommand::CodeAction(CodeAction {
        title: fix.label,
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }),
        ..CodeAction::default()
    })
}

#[allow(deprecated)] // DocumentSymbol::deprecated is the lsp_types field name.
fn doc_symbol_to_lsp(sym: &DocSymbol) -> DocumentSymbol {
    let range = sym
        .location
        .as_ref()
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

    let children: Vec<DocumentSymbol> = sym.children.iter().map(doc_symbol_to_lsp).collect();

    DocumentSymbol {
        name: sym.name.clone(),
        detail: sym.detail.clone(),
        kind: symbol_kind_to_lsp(sym.kind),
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

/// Map a JVM-shaped `beans::SymbolKind` to the LSP wire's `SymbolKind`.
/// Per-language kinds (Kotlin's `Object`, Scala's `Trait`, Clojure's
/// `Namespace`, etc.) will need their own mapping when those payloads
/// land.
///
/// The `EnumConstant` arm is reachable in principle but unreachable
/// today: the facade's `payload_view` collapses `EnumConstant` into
/// `SymbolKind::Field` for spec-test stability. Backlog #032 tracks
/// whether to surface `EnumConstant` distinctly.
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
    fn symbol_kind_to_lsp_covers_canonical_kinds() {
        assert_eq!(symbol_kind_to_lsp(SymbolKind::Class), LspSymbolKind::CLASS);
        assert_eq!(
            symbol_kind_to_lsp(SymbolKind::Interface),
            LspSymbolKind::INTERFACE
        );
        assert_eq!(
            symbol_kind_to_lsp(SymbolKind::Method),
            LspSymbolKind::METHOD
        );
        assert_eq!(symbol_kind_to_lsp(SymbolKind::Field), LspSymbolKind::FIELD);
        assert_eq!(
            symbol_kind_to_lsp(SymbolKind::Constructor),
            LspSymbolKind::CONSTRUCTOR
        );
    }

    /// Boundary smoke test: an indexed file resolves go-to-definition to
    /// a Scalar location with the declaration's range. Verifies the
    /// Url->path->facade->lsp round trip, not the resolution itself
    /// (that's covered in `beans`).
    #[test]
    fn goto_definition_maps_to_scalar_location() {
        let mut state = ServerState::new();
        let path = std::path::Path::new("/tmp/beans-lsp-gd/Dog.java");
        let source = "package com.example;\npublic class Dog {\n    public String getName() { return null; }\n}\n";
        state.workspace.update_file(path, source);

        let uri = Url::from_file_path(path).unwrap();
        // Cursor on the `Dog` class name (line 1).
        let resp = handle_goto_definition(
            &state,
            &uri,
            Position {
                line: 1,
                character: 13,
            },
        )
        .expect("Dog should resolve to a definition");
        let GotoDefinitionResponse::Scalar(loc) = resp else {
            panic!("expected a Scalar definition response");
        };
        assert_eq!(loc.uri, uri);
        assert_eq!(loc.range.start.line, 1);
    }

    /// Boundary smoke test: a quick fix becomes a QUICKFIX CodeAction
    /// whose WorkspaceEdit inserts the import under the document's URI.
    #[test]
    fn code_action_maps_fix_to_workspace_edit() {
        let mut state = ServerState::new();
        let model = std::path::Path::new("/tmp/beans-lsp-ca/Service.java");
        let app = std::path::Path::new("/tmp/beans-lsp-ca/App.java");
        state.workspace.update_file(
            model,
            "package com.example.model;\npublic class Service {}\n",
        );
        let app_text =
            "package com.example.app;\npublic class App {\n    private Service service;\n}\n";
        state.workspace.update_file(app, app_text);

        let uri = Url::from_file_path(app).unwrap();
        let actions = handle_code_action(
            &state,
            &uri,
            Position {
                line: 2,
                character: 14,
            },
        )
        .expect("an import fix should be offered");
        assert_eq!(actions.len(), 1);
        let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
            panic!("expected a CodeAction");
        };
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        let changes = action.edit.as_ref().unwrap().changes.as_ref().unwrap();
        let edits = changes.get(&uri).unwrap();
        assert_eq!(edits.len(), 1);
        assert!(
            edits[0]
                .new_text
                .contains("import com.example.model.Service;")
        );

        // A resolved position offers nothing.
        assert!(
            handle_code_action(
                &state,
                &uri,
                Position {
                    line: 1,
                    character: 0
                }
            )
            .is_none()
        );
    }

    /// Boundary smoke test: document symbols map to a nested response
    /// with the class as the top-level symbol.
    #[test]
    fn document_symbol_maps_to_nested_response() {
        let mut state = ServerState::new();
        let path = std::path::Path::new("/tmp/beans-lsp-ds/Dog.java");
        state.workspace.update_file(
            path,
            "package com.example;\npublic class Dog {\n    public String getName() { return null; }\n}\n",
        );
        let uri = Url::from_file_path(path).unwrap();
        let resp = handle_document_symbol(&state, &uri).expect("outline expected");
        let DocumentSymbolResponse::Nested(symbols) = resp else {
            panic!("expected a Nested document-symbol response");
        };
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Dog");
        assert_eq!(symbols[0].kind, LspSymbolKind::CLASS);
    }
}
