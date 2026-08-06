//! The LSP facet: it translates between the protocol and the `Beans` facade and
//! carries messages, and it decides nothing about a language. Everything it is
//! asked is answered one layer down, so its tests are about what goes on the
//! wire rather than about what the answer should be.

pub mod translation;

use beans::Beans;
use beans_platform_jvm as jvm;
use lsp_server::{
    Connection, Message, Notification as ServerNotification, Request as ServerRequest,
    Response as ServerResponse,
};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    PublishDiagnostics,
};
use lsp_types::request::{
    GotoDeclaration, GotoDeclarationParams, GotoDeclarationResponse, GotoDefinition, HoverRequest,
    Request as _,
};
use lsp_types::{
    DeclarationCapability, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
    HoverParams, HoverProviderCapability, MarkedString, OneOf, PublishDiagnosticsParams,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

use std::fs::OpenOptions;
use std::io::{LineWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::translation::{
    position_to_line_column, source_to_uri, text_range_to_range, translate_diagnostics,
    uri_to_path, uri_to_source,
};

/// A process-global JSONL sink for raw protocol traffic, in the shape of a
/// logger: set once from the environment, then written from the single message
/// loop. Absent because there is nothing per-connection to carry.
static TRACE: OnceLock<Mutex<LineWriter<std::fs::File>>> = OnceLock::new();

/// Opens the trace file named by `BEANS_TRACE`, if set. An unset variable or an
/// unopenable path leaves the sink empty, so [`trace`] stays a silent no-op —
/// tracing is opt-in and never a reason to fail startup.
pub fn init_trace() {
    let Some(path) = std::env::var_os("BEANS_TRACE") else {
        return;
    };
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = TRACE.set(Mutex::new(LineWriter::new(file)));
    }
}

/// Appends one message to the trace as a single JSONL record tagged with its
/// direction and a millisecond timestamp. Every step is best-effort: a broken
/// trace must never take the server down.
fn trace(dir: &str, msg: &Message) {
    let Some(sink) = TRACE.get() else {
        return;
    };
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0);
    let record = serde_json::json!({ "dir": dir, "at": at, "msg": msg });
    if let Ok(mut sink) = sink.lock() {
        let _ = writeln!(sink, "{record}");
    }
}

/// Sends an outbound message, tracing it first so responses and server
/// notifications share the JSONL stream with the inbound traffic that provoked
/// them.
fn send(conn: &Connection, msg: Message) {
    trace("out", &msg);
    conn.sender.send(msg).unwrap();
}

pub fn run(conn: Connection, mut beans: Beans) {
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..Default::default()
    };
    let server_capabilities = serde_json::to_value(&capabilities).unwrap();
    let initialization_params = conn.initialize(server_capabilities).unwrap();

    if let Some(root) = workspace_root(&initialization_params) {
        match beans.open_workspace(&root) {
            Ok(loaded) => eprintln!("workspace {}: {loaded} sources", root.display()),
            Err(error) => eprintln!("workspace {}: {error}", root.display()),
        }
    }

    server_loop(conn, beans);
}

/// The first folder the client opened. `rootUri` is deprecated in favour of
/// `workspaceFolders`, so try that first and fall back for older clients.
fn workspace_root(params: &serde_json::Value) -> Option<PathBuf> {
    let folder = params
        .get("workspaceFolders")
        .and_then(|folders| folders.get(0))
        .and_then(|folder| folder.get("uri"))
        .or_else(|| params.get("rootUri"))?;

    uri_to_path(&folder.as_str()?.parse().ok()?)
}

pub fn server_loop(conn: Connection, mut beans: Beans) {
    for msg in &conn.receiver {
        trace("in", &msg);
        match msg {
            Message::Request(req) => handle_request(&conn, &beans, req),
            Message::Response(_res) => {}
            Message::Notification(notif) => handle_notification(&conn, &mut beans, notif),
        }
    }
}

fn handle_request(conn: &Connection, beans: &Beans, request: ServerRequest) {
    let response = match request.method.as_str() {
        GotoDeclaration::METHOD => {
            let (id, params) = request
                .extract::<GotoDeclarationParams>(GotoDeclaration::METHOD)
                .unwrap();
            let locations = resolve_locations(beans, params.text_document_position_params);
            ServerResponse::new_ok(id, locations.map(GotoDeclarationResponse::Array))
        }
        GotoDefinition::METHOD => {
            let (id, params) = request
                .extract::<GotoDefinitionParams>(GotoDefinition::METHOD)
                .unwrap();
            let locations = resolve_locations(beans, params.text_document_position_params);
            ServerResponse::new_ok(id, locations.map(GotoDefinitionResponse::Array))
        }
        HoverRequest::METHOD => {
            let (id, params) = request
                .extract::<HoverParams>(HoverRequest::METHOD)
                .unwrap();
            ServerResponse::new_ok(id, handle_request_hover(beans, params))
        }
        _ => return,
    };

    send(conn, Message::Response(response));
}

fn resolve_locations(
    beans: &Beans,
    params: lsp_types::TextDocumentPositionParams,
) -> Option<Vec<lsp_types::Location>> {
    let source = uri_to_source(&params.text_document.uri)?;
    let offset = beans.offset_at(&source, position_to_line_column(params.position))?;

    let declarations = beans.find_declarations_for(&source, offset)?;

    // A target may live in another file. Its range comes from that file's
    // stored line index in the engine, so no open buffer is required — the
    // target need only have been parsed.
    let locations: Vec<lsp_types::Location> = declarations
        .iter()
        .filter_map(|target| {
            let uri = source_to_uri(&target.source)?;
            let range = beans.text_range(&target.source, target.span)?;
            Some(lsp_types::Location {
                uri,
                range: text_range_to_range(range),
            })
        })
        .collect();
    if locations.is_empty() {
        return None;
    }
    Some(locations)
}

fn handle_request_hover(beans: &Beans, params: HoverParams) -> Option<Hover> {
    let request = params.text_document_position_params;
    let source = uri_to_source(&request.text_document.uri)?;
    let offset = beans.offset_at(&source, position_to_line_column(request.position))?;
    let declarations = beans.find_declarations_for(&source, offset)?;
    let declaration = declarations
        .into_iter()
        .find(|declaration| declaration.source == source)?;

    let label = beans.declaration_label(&declaration.source, declaration.span);

    Some(Hover {
        contents: HoverContents::Scalar(MarkedString::String(match label {
            Some(label) => format!(
                "Java declaration: {label}\n\nbyte span: {}..{}",
                declaration.span.start, declaration.span.end
            ),
            None => format!(
                "Java declaration\n\nbyte span: {}..{}",
                declaration.span.start, declaration.span.end
            ),
        })),
        range: beans
            .text_range(&declaration.source, declaration.span)
            .map(text_range_to_range),
    })
}

fn handle_notification(conn: &Connection, beans: &mut Beans, notification: ServerNotification) {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = notification
                .extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
                .unwrap();
            handle_notification_did_open(conn, beans, params);
        }
        DidChangeTextDocument::METHOD => {
            let params = notification
                .extract::<DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD)
                .unwrap();
            handle_notification_did_change(conn, beans, params);
        }
        DidCloseTextDocument::METHOD => {
            let params = notification
                .extract::<DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)
                .unwrap();
            handle_notification_did_close(conn, params);
        }
        _ => {}
    }
}

fn handle_notification_did_open(
    conn: &Connection,
    beans: &mut Beans,
    params: DidOpenTextDocumentParams,
) {
    let document = params.text_document;
    let Some(source) = uri_to_source(&document.uri) else {
        return;
    };
    beans.process(source.clone(), &document.text);
    publish_diagnostics(conn, beans, &source, document.uri, document.version);
}

fn handle_notification_did_change(
    conn: &Connection,
    beans: &mut Beans,
    mut params: DidChangeTextDocumentParams,
) {
    let uri = params.text_document.uri;
    let version = params.text_document.version;
    let Some(source) = uri_to_source(&uri) else {
        return;
    };

    // FULL sync sends the whole document as a single change entry.
    let Some(change) = params.content_changes.pop() else {
        return;
    };
    beans.process(source.clone(), &change.text);
    publish_diagnostics(conn, beans, &source, uri, version);
}

fn handle_notification_did_close(conn: &Connection, params: DidCloseTextDocumentParams) {
    // The engine keeps the file's text so it stays resolvable as a navigation
    // target; closing only clears the editor's squiggles.
    send_diagnostics(conn, params.text_document.uri, vec![], None);
}

fn publish_diagnostics(
    conn: &Connection,
    beans: &Beans,
    source: &jvm::model::Source,
    uri: Uri,
    version: i32,
) {
    let Some(analysis) = beans.analyze(source) else {
        return;
    };

    // The range comes from the engine's stored text, so the LSP layer never
    // touches the buffer itself.
    let diagnostics = analysis
        .diagnostics
        .iter()
        .map(|d| {
            let range = beans
                .text_range(source, d.span)
                .map(text_range_to_range)
                .unwrap_or_default();
            translate_diagnostics(range, d)
        })
        .collect();
    send_diagnostics(conn, uri, diagnostics, Some(version));
}

fn send_diagnostics(
    conn: &Connection,
    uri: Uri,
    diagnostics: Vec<lsp_types::Diagnostic>,
    version: Option<i32>,
) {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version,
    };
    let notification = ServerNotification::new(PublishDiagnostics::METHOD.to_string(), params);
    send(conn, Message::Notification(notification));
}
