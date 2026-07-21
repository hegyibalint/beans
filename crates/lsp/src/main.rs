mod translation;

use std::collections::HashMap;

use beans::Beans;
use beans_platform_jvm::model::JvmSource;
use lsp_server::{
    Connection, Message, Notification as ServerNotification, Request as ServerRequest,
    Response as ServerResponse,
};
use lsp_types::notification::{
    DidChangeTextDocument, DidOpenTextDocument, Notification, PublishDiagnostics,
};
use lsp_types::request::{
    GotoDeclaration, GotoDeclarationParams, GotoDeclarationResponse, GotoDefinition,
    HoverRequest, Request as _,
};
use lsp_types::{
    DeclarationCapability, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
    HoverParams, HoverProviderCapability, MarkedString, OneOf, PublishDiagnosticsParams,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use lsp_types::{DidChangeTextDocumentParams, DidOpenTextDocumentParams};

use crate::translation::{position_to_offset, span_to_range, translate_diagnostics, uri_to_source};

fn main() {
    let (conn, _) = Connection::stdio();
    let beans = Beans::new();
    run(conn, beans);
}

struct OpenDocument {
    uri: Uri,
    version: i32,
    contents: String,
}

struct State {
    beans: Beans,
    documents: HashMap<JvmSource, OpenDocument>,
}

impl State {
    fn new(beans: Beans) -> Self {
        Self {
            beans,
            documents: HashMap::new(),
        }
    }

    fn document(&self, source: &JvmSource) -> Option<&OpenDocument> {
        self.documents.get(source)
    }

    fn is_stale(&self, source: &JvmSource, version: i32) -> bool {
        self.documents
            .get(source)
            .is_some_and(|document| version <= document.version)
    }

    fn record(&mut self, source: JvmSource, uri: Uri, version: i32, contents: String) {
        self.documents.insert(
            source,
            OpenDocument {
                uri,
                version,
                contents,
            },
        );
    }
}

fn run(conn: Connection, beans: Beans) {
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..Default::default()
    };
    let server_capabilities = serde_json::to_value(&capabilities).unwrap();
    let _initialization_params = conn.initialize(server_capabilities).unwrap();

    // Session begins here: one initialize per connection, so State starts empty.
    server_loop(conn, State::new(beans));
}

fn server_loop(conn: Connection, mut state: State) {
    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => handle_request(&conn, &state, req),
            Message::Response(_res) => {}
            Message::Notification(notif) => handle_notification(&conn, &mut state, notif),
        }
    }
}

fn handle_request(conn: &Connection, state: &State, request: ServerRequest) {
    let response = match request.method.as_str() {
        GotoDeclaration::METHOD => {
            let (id, params) = request
                .extract::<GotoDeclarationParams>(GotoDeclaration::METHOD)
                .unwrap();
            let locations = resolve_locations(state, params.text_document_position_params);
            ServerResponse::new_ok(id, locations.map(GotoDeclarationResponse::Array))
        }
        GotoDefinition::METHOD => {
            let (id, params) = request
                .extract::<GotoDefinitionParams>(GotoDefinition::METHOD)
                .unwrap();
            let locations = resolve_locations(state, params.text_document_position_params);
            ServerResponse::new_ok(id, locations.map(GotoDefinitionResponse::Array))
        }
        HoverRequest::METHOD => {
            let (id, params) = request
                .extract::<HoverParams>(HoverRequest::METHOD)
                .unwrap();
            ServerResponse::new_ok(id, handle_request_hover(state, params))
        }
        _ => return,
    };

    conn.sender.send(Message::Response(response)).unwrap();
}

fn resolve_locations(
    state: &State,
    params: lsp_types::TextDocumentPositionParams,
) -> Option<Vec<lsp_types::Location>> {
    let source = uri_to_source(&params.text_document.uri)?;
    let document = state.document(&source)?;
    let offset = position_to_offset(&document.contents, params.position)?;

    let declarations = state.beans.find_declarations_for(&source, offset)?;

    // Cross-file targets need the target's contents to compute ranges; only
    // the open document is at hand, so cross-file targets are dropped for now.
    let locations: Vec<lsp_types::Location> = declarations
        .iter()
        .filter(|declaration| declaration.source == source)
        .map(|declaration| lsp_types::Location {
            uri: params.text_document.uri.clone(),
            range: span_to_range(&document.contents, &declaration.span),
        })
        .collect();
    if locations.is_empty() {
        return None;
    }
    Some(locations)
}

fn handle_request_hover(state: &State, params: HoverParams) -> Option<Hover> {
    let request = params.text_document_position_params;
    let source = uri_to_source(&request.text_document.uri)?;
    let document = state.document(&source)?;
    let offset = position_to_offset(&document.contents, request.position)?;
    let declarations = state.beans.find_declarations_for(&source, offset)?;
    let declaration = declarations
        .into_iter()
        .find(|declaration| declaration.source == source)?;

    let label = state
        .beans
        .describe_declaration(&declaration.source, declaration.span);

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
        range: Some(span_to_range(&document.contents, &declaration.span)),
    })
}

fn handle_notification(conn: &Connection, state: &mut State, notification: ServerNotification) {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = notification
                .extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
                .unwrap();
            handle_notification_did_open(conn, state, params);
        }
        DidChangeTextDocument::METHOD => {
            let params = notification
                .extract::<DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD)
                .unwrap();
            handle_notification_did_change(conn, state, params);
        }
        _ => {}
    }
}

fn handle_notification_did_open(
    conn: &Connection,
    state: &mut State,
    params: DidOpenTextDocumentParams,
) {
    let document = params.text_document;
    let Some(source) = uri_to_source(&document.uri) else {
        return;
    };

    // Open re-baselines the document: never stale, always processed.
    process_document_and_publish_diagnostics(
        conn,
        &mut state.beans,
        source.clone(),
        document.uri.clone(),
        document.version,
        &document.text,
    );
    state.record(source, document.uri, document.version, document.text);
}

fn handle_notification_did_change(
    conn: &Connection,
    state: &mut State,
    mut params: DidChangeTextDocumentParams,
) {
    let uri = params.text_document.uri;
    let version = params.text_document.version;
    let Some(source) = uri_to_source(&uri) else {
        return;
    };
    if state.is_stale(&source, version) {
        return;
    }

    // FULL sync sends the whole document as a single change entry.
    let Some(change) = params.content_changes.pop() else {
        return;
    };
    process_document_and_publish_diagnostics(
        conn,
        &mut state.beans,
        source.clone(),
        uri.clone(),
        version,
        &change.text,
    );
    state.record(source, uri, version, change.text);
}

fn process_document_and_publish_diagnostics(
    conn: &Connection,
    beans: &mut Beans,
    source: JvmSource,
    uri: Uri,
    version: i32,
    contents: &str,
) {
    beans.process(source.clone(), contents);
    let Some(analysis) = beans.analyze(&source) else {
        return;
    };

    // Map and send off all diagnostics
    let lsp_diagnostics = analysis
        .diagnostics
        .iter()
        .map(|d| translate_diagnostics(&contents, d))
        .collect();
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics: lsp_diagnostics,
        version: Some(version),
    };
    let notification = ServerNotification::new(PublishDiagnostics::METHOD.to_string(), params);
    conn.sender
        .send(Message::Notification(notification))
        .unwrap();
}

#[cfg(test)]
mod tests {
    use crate::{State, handle_request_hover, server_loop};
    use beans::Beans;
    use lsp_server::{Connection, Message, Notification};
    use lsp_types::notification::Notification as _;
    use lsp_types::{
        DidOpenTextDocumentParams, HoverParams, Position, PublishDiagnosticsParams, Range,
        TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams,
        WorkDoneProgressParams,
        notification::{DidOpenTextDocument, PublishDiagnostics},
    };

    #[test]
    fn state_keeps_only_the_latest_open_document() {
        let mut state = State::new(Beans::new());
        let uri: lsp_types::Uri = "file:///workspace/Foo.java".parse().unwrap();
        let source = crate::translation::uri_to_source(&uri).unwrap();

        state.record(source.clone(), uri.clone(), 1, "first".into());
        state.record(source.clone(), uri.clone(), 2, "second".into());

        assert_eq!(state.documents.len(), 1);
        let document = state.document(&source).unwrap();
        assert_eq!(document.uri, uri);
        assert_eq!(document.version, 2);
        assert_eq!(document.contents, "second");
        assert!(state.is_stale(&source, 2));
        assert!(!state.is_stale(&source, 3));
    }

    #[test]
    fn hover_shows_the_resolved_declaration() {
        let uri: lsp_types::Uri = "file:///workspace/Foo.java".parse().unwrap();
        let source = crate::translation::uri_to_source(&uri).unwrap();
        let contents = "class Outer {}";
        let mut state = State::new(Beans::new());
        state.beans.process(source.clone(), contents);
        state.record(source, uri.clone(), 1, contents.into());

        let hover = handle_request_hover(
            &state,
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position::new(0, 8),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        )
        .unwrap();

        assert_eq!(
            hover.range,
            Some(Range::new(Position::new(0, 6), Position::new(0, 11)))
        );
    }

    /// Opens `contents` as A.java, requests `method` at `position`, and
    /// returns the resolved locations. Both goto-declaration and
    /// goto-definition serialize to plain location arrays.
    fn request_locations(
        method: &str,
        position: Position,
        contents: &str,
    ) -> Vec<lsp_types::Location> {
        use lsp_server::{Request, RequestId};
        use lsp_types::{PartialResultParams, TextDocumentIdentifier, TextDocumentPositionParams};

        let uri: lsp_types::Uri = "file:///workspace/A.java".parse().unwrap();

        let (server_conn, client) = Connection::memory();
        let handle = std::thread::spawn(move || {
            server_loop(server_conn, State::new(Beans::new()));
        });

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "java".into(),
                version: 1,
                text: contents.into(),
            },
        };
        client
            .sender
            .send(Message::Notification(Notification::new(
                DidOpenTextDocument::METHOD.to_string(),
                did_open,
            )))
            .unwrap();
        // Drain the diagnostics publish before requesting.
        client.receiver.recv().unwrap();

        let params = lsp_types::GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        client
            .sender
            .send(Message::Request(Request::new(
                RequestId::from(1),
                method.to_string(),
                params,
            )))
            .unwrap();
        let response = match client.receiver.recv().unwrap() {
            Message::Response(response) => response,
            other => panic!("expected a response, got {other:?}"),
        };

        drop(client);
        handle.join().unwrap();

        serde_json::from_value(response.result.unwrap()).unwrap()
    }

    #[test]
    fn goto_declaration_resolves_an_occurrence() {
        use lsp_types::request::{GotoDeclaration, Request as _};
        use lsp_types::{Location, Range};

        // The `c` in `int d = c;` at line 2, character 16 → parameter c at 1:15.
        let locations = request_locations(
            GotoDeclaration::METHOD,
            Position::new(2, 16),
            "class A {\n    void b(int c) {\n        int d = c;\n    }\n}\n",
        );

        assert_eq!(
            locations,
            vec![Location {
                uri: "file:///workspace/A.java".parse().unwrap(),
                range: Range::new(Position::new(1, 15), Position::new(1, 16)),
            }]
        );
    }

    #[test]
    fn goto_definition_on_a_field_access_segment_jumps_to_the_field() {
        use lsp_types::request::{GotoDefinition, Request as _};
        use lsp_types::{Location, Range};

        // The `a` in `this.a = d;` at line 5, character 13 → field `a` at 1:8.
        let locations = request_locations(
            GotoDefinition::METHOD,
            Position::new(5, 13),
            "class A {\n    int a;\n\n    void b(B c) {\n        int d = c.a;\n        this.a = d;\n        b(c);\n    }\n}\n",
        );

        assert_eq!(
            locations,
            vec![Location {
                uri: "file:///workspace/A.java".parse().unwrap(),
                range: Range::new(Position::new(1, 8), Position::new(1, 9)),
            }]
        );
    }

    #[test]
    fn goto_declaration_request_receives_an_empty_response() {
        use lsp_server::{Request, RequestId};
        use lsp_types::request::{GotoDeclaration, GotoDeclarationParams, Request as _};
        use lsp_types::{
            PartialResultParams, Position, TextDocumentIdentifier, TextDocumentPositionParams,
            WorkDoneProgressParams,
        };

        let (server_conn, client) = Connection::memory();
        let handle = std::thread::spawn(move || {
            server_loop(server_conn, State::new(Beans::new()));
        });
        let params = GotoDeclarationParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///workspace/Foo.java".parse().unwrap(),
                },
                position: Position::new(0, 0),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let request = Request::new(
            RequestId::from(1),
            GotoDeclaration::METHOD.to_string(),
            params,
        );

        client.sender.send(Message::Request(request)).unwrap();

        let response = match client.receiver.recv().unwrap() {
            Message::Response(response) => response,
            other => panic!("expected a response, got {other:?}"),
        };
        assert_eq!(response.id, RequestId::from(1));
        assert_eq!(response.result, Some(serde_json::Value::Null));
        assert!(response.error.is_none());

        drop(client);
        handle.join().unwrap();
    }

    #[test]
    fn open_file_publishes_diagnostics() {
        let (server_conn, client) = Connection::memory();

        let beans = Beans::new();
        let handle = std::thread::spawn(move || {
            server_loop(server_conn, State::new(beans));
        });

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file://src/main/org/beans/test/Foo.java".parse().unwrap(),
                language_id: "beans".into(),
                version: 0,
                text: r#"
                package org.beans.test;

                import org.beans.test.Bar

                class Foo {
                    Bar bar;
                }"#
                .into(),
            },
        };

        let notif = Notification::new(DidOpenTextDocument::METHOD.to_string(), did_open);
        client.sender.send(Message::Notification(notif)).unwrap();

        // Receive the publish before dropping the client, otherwise the server's send races the
        // channel closing.
        let msg = client
            .receiver
            .recv()
            .expect("server publishes diagnostics on open");
        let published = match msg {
            Message::Notification(published) => published,
            other => panic!("expected a notification, got {other:?}"),
        };
        assert_eq!(published.method, PublishDiagnostics::METHOD);

        let params: PublishDiagnosticsParams = published
            .extract(PublishDiagnostics::METHOD)
            .expect("payload is PublishDiagnosticsParams");
        assert!(params.diagnostics.is_empty());

        drop(client);
        handle.join().unwrap();
    }

    #[test]
    fn initialize_advertises_sync_then_publishes_on_open() {
        use lsp_server::{Request, RequestId};
        use lsp_types::{
            DeclarationCapability, HoverProviderCapability, InitializeParams, InitializeResult,
            InitializedParams, TextDocumentSyncCapability, TextDocumentSyncKind,
        };

        let (server_conn, client) = Connection::memory();
        let beans = Beans::new();
        let handle = std::thread::spawn(move || {
            crate::run(server_conn, beans);
        });

        // Drive the real handshake `conn.initialize` expects, rather than skipping it.
        let init = Request::new(
            RequestId::from(1),
            "initialize".to_string(),
            InitializeParams::default(),
        );
        client.sender.send(Message::Request(init)).unwrap();

        let response = match client.receiver.recv().unwrap() {
            Message::Response(response) => response,
            other => panic!("expected initialize response, got {other:?}"),
        };
        let result: InitializeResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(
            result.capabilities.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            "without this capability clients never send didOpen and no diagnostics surface",
        );
        assert_eq!(
            result.capabilities.declaration_provider,
            Some(DeclarationCapability::Simple(true)),
        );
        assert_eq!(
            result.capabilities.definition_provider,
            Some(lsp_types::OneOf::Left(true)),
        );
        assert_eq!(
            result.capabilities.hover_provider,
            Some(HoverProviderCapability::Simple(true)),
        );

        let initialized = Notification::new("initialized".to_string(), InitializedParams {});
        client
            .sender
            .send(Message::Notification(initialized))
            .unwrap();

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file://src/main/org/beans/test/Foo.java".parse().unwrap(),
                language_id: "java".into(),
                version: 0,
                text: "package org.beans.test;\n\nclass Foo {\n    Bar bar;\n}\n".into(),
            },
        };
        let notif = Notification::new(DidOpenTextDocument::METHOD.to_string(), did_open);
        client.sender.send(Message::Notification(notif)).unwrap();

        let published = match client.receiver.recv().unwrap() {
            Message::Notification(published) => published,
            other => panic!("expected a publish notification, got {other:?}"),
        };
        assert_eq!(published.method, PublishDiagnostics::METHOD);
        let params: PublishDiagnosticsParams = published
            .extract(PublishDiagnostics::METHOD)
            .expect("payload is PublishDiagnosticsParams");
        assert!(params.diagnostics.is_empty());

        drop(client);
        handle.join().unwrap();
    }
}
