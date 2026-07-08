mod translation;

use lsp_server::{Connection, Message, Notification as ServerNotification};
use lsp_types::notification::{DidOpenTextDocument, Notification, PublishDiagnostics};
use lsp_types::{Diagnostic, DidOpenTextDocumentParams, PublishDiagnosticsParams, ServerCapabilities};
use beans::Beans;
use beans_core::VirtualFile;

use crate::translation::translate_diagnostics;


fn main() {
    let (conn, _) = Connection::stdio();
    let beans = Beans::new();

    let server_capabilities = serde_json::to_value(&ServerCapabilities::default()).unwrap();
    let _initialization_params = conn.initialize(server_capabilities).unwrap();

    server_loop(conn, beans);
}

fn server_loop(conn: Connection, mut beans: Beans) {
    for msg in &conn.receiver {
        match msg {
            Message::Request(_req) => {}
            Message::Response(_res) => {}
            Message::Notification(notif) => handle_notification(&conn, &mut beans, notif),
            _ => panic!("Unexpected message: {:?}", msg),
        }
    }
}

fn handle_notification(conn: &Connection, beans: &mut Beans, notif: ServerNotification) {
    if notif.method == DidOpenTextDocument::METHOD {
        let payload = notif.extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD).unwrap();
        handle_notification_did_open_text_document(conn, beans, payload)
    }
}

fn handle_notification_did_open_text_document(conn: &Connection, beans: &mut Beans, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    let contents = params.text_document.text;
    let analysis = beans.open(uri.as_str(), contents.as_str());

    // Map and send off all diagnostics
    let lsp_diagnostics = analysis
        .diagnostics
        .iter()
        .map(|d| translate_diagnostics(&contents, d))
        .collect();
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics: lsp_diagnostics,
        version: None
    };
    let notification = ServerNotification::new(PublishDiagnostics::METHOD.to_string(), params);
    conn.sender.send(Message::Notification(notification)).unwrap();
}

#[cfg(test)]
mod tests {
    use beans::Beans;
    use lsp_server::{Connection, Message, Notification};
    use lsp_types::notification::Notification as _;
    use lsp_types::{
        DiagnosticSeverity, DidOpenTextDocumentParams, PublishDiagnosticsParams, TextDocumentItem,
        notification::{DidOpenTextDocument, PublishDiagnostics},
    };
    use crate::server_loop;

    #[test]
    fn open_file_publishes_dummy_diagnostic() {
        let (server_conn, client) = Connection::memory();

        let beans = Beans::new();
        let handle = std::thread::spawn(move || {
            server_loop(server_conn, beans);
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
        assert_eq!(params.diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));

        drop(client);
        handle.join().unwrap();
    }

}
