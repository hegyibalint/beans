use lsp_server::{Connection, Message, Notification as ServerNotification};
use lsp_types::notification::{DidOpenTextDocument, Notification};
use lsp_types::{DidOpenTextDocumentParams, ServerCapabilities};
use beans::Beans;
use beans_core::VirtualFile;

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
            Message::Notification(notif) => handle_notification(&mut beans, notif),
            _ => panic!("Unexpected message: {:?}", msg),
        }
    }
}

fn handle_notification(beans: &mut Beans, notif: ServerNotification) {
    if notif.method == DidOpenTextDocument::METHOD {
        let params = notif
            .extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
            .unwrap();
        let file = VirtualFile {
            uri: params.text_document.uri.as_str().to_string(),
            content: params.text_document.text,
        };
        beans.open(file);
    }
}

#[cfg(test)]
mod tests {
    use beans::Beans;
use lsp_server::{Connection, Message, Notification};
    use lsp_types::notification::Notification as _;
    use lsp_types::{DidOpenTextDocumentParams, TextDocumentItem, notification::DidOpenTextDocument};
    use crate::server_loop;

    #[test]
    fn start_and_open_file() {
        let (server_conn, client) = Connection::memory();

        let beans = Beans::new();
        let handle = std::thread::spawn(move || {
            server_loop(server_conn, beans);
        });

        let did_open_msg_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file://src/main/org/beans/test/Foo.java".parse().unwrap(),
                language_id: "beans".into(),
                version: 0,
                text: r#"
                package org.beans.test;

                import org.beans.test.Bar

                class Foo {
                    Bar bar;
                }"#.into(),
            }
        };

        let notif = Notification::new(DidOpenTextDocument::METHOD.to_string(), did_open_msg_params);
        client.sender.send(Message::Notification(notif)).unwrap();

        drop(client);
        handle.join().unwrap();
    }

}
