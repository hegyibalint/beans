mod translation;

use std::collections::HashMap;

use beans::Beans;
use lsp_server::{Connection, Message, Notification as ServerNotification};
use lsp_types::notification::{
    DidChangeTextDocument, DidOpenTextDocument, Notification, PublishDiagnostics,
};
use lsp_types::{DidChangeTextDocumentParams, DidOpenTextDocumentParams};
use lsp_types::{
    PublishDiagnosticsParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    Uri,
};

use crate::translation::{translate_diagnostics, uri_to_source};

fn main() {
    let (conn, _) = Connection::stdio();
    let beans = Beans::new();
    run(conn, beans);
}

struct State {
    beans: Beans,
    versions: HashMap<String, i32>,
}

impl State {
    fn new(beans: Beans) -> Self {
        Self {
            beans,
            versions: HashMap::new(),
        }
    }

    fn is_stale(&self, uri: &Uri, version: i32) -> bool {
        self.versions
            .get(uri.as_str())
            .is_some_and(|&current| version <= current)
    }

    fn record(&mut self, uri: &Uri, version: i32) {
        self.versions.insert(uri.as_str().to_owned(), version);
    }
}

fn run(conn: Connection, beans: Beans) {
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
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
            Message::Request(_req) => {}
            Message::Response(_res) => {}
            Message::Notification(notif) => handle_notification(&conn, &mut state, notif),
            _ => panic!("Unexpected message: {:?}", msg),
        }
    }
}

fn handle_notification(conn: &Connection, state: &mut State, notif: ServerNotification) {
    match notif.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = notif
                .extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
                .unwrap();
            let doc = params.text_document;
            // Open re-baselines the document: never stale, always processed.
            state.record(&doc.uri, doc.version);
            on_document(conn, &mut state.beans, doc.uri, doc.version, doc.text);
        }
        DidChangeTextDocument::METHOD => {
            let mut params = notif
                .extract::<DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD)
                .unwrap();
            let uri = params.text_document.uri;
            let version = params.text_document.version;
            if state.is_stale(&uri, version) {
                return;
            }
            // FULL sync sends the whole document as a single change entry.
            let Some(change) = params.content_changes.pop() else {
                return;
            };
            state.record(&uri, version);
            on_document(conn, &mut state.beans, uri, version, change.text);
        }
        _ => {}
    }
}

fn on_document(conn: &Connection, beans: &mut Beans, uri: Uri, version: i32, contents: String) {
    // Skip what we cannot source (untitled buffers) or no language claims.
    let Some(source) = uri_to_source(&uri) else {
        return;
    };
    let Some(analysis) = beans.process(source, contents.as_str()) else {
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
    use crate::{State, server_loop};
    use beans::Beans;
    use lsp_server::{Connection, Message, Notification};
    use lsp_types::notification::Notification as _;
    use lsp_types::{
        DiagnosticSeverity, DidOpenTextDocumentParams, PublishDiagnosticsParams, TextDocumentItem,
        notification::{DidOpenTextDocument, PublishDiagnostics},
    };

    #[test]
    fn open_file_publishes_dummy_diagnostic() {
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
        assert_eq!(params.diagnostics.len(), 1);
        assert_eq!(params.diagnostics[0].message, "dummy diagnostics");
        assert_eq!(
            params.diagnostics[0].severity,
            Some(DiagnosticSeverity::WARNING)
        );

        drop(client);
        handle.join().unwrap();
    }

    #[test]
    fn initialize_advertises_sync_then_publishes_on_open() {
        use lsp_server::{Request, RequestId};
        use lsp_types::{
            InitializeParams, InitializeResult, InitializedParams, TextDocumentSyncCapability,
            TextDocumentSyncKind,
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
                text: "package org.beans.test;\n\nclass Foo {}\n".into(),
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
        assert_eq!(params.diagnostics.len(), 1);

        drop(client);
        handle.join().unwrap();
    }
}
