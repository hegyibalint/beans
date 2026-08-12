//! What this layer puts on the wire. Every case drives a real connection the way
//! an editor would, so nothing here calls a handler directly.
//!
//! None of it establishes a fact about Java. Which declaration a name reaches is
//! settled in `lang-java` and in the acceptance suite; what is left for this
//! crate is whether the request was understood, whether the answer is shaped the
//! way the protocol says, and whether it went out at all.

use std::thread::JoinHandle;
use std::time::Duration;

use beans::Beans;
use beans_lsp::{run, server_loop};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{Completion, GotoDeclaration, GotoDefinition, HoverRequest, Request as _};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    GotoDefinitionParams, Hover, InitializeParams, InitializeResult, InitializedParams, Location,
    PartialResultParams, Position, PublishDiagnosticsParams, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams,
    VersionedTextDocumentIdentifier, WorkDoneProgressParams,
};

/// A server on an in-memory connection, with the client end held here. Dropping
/// it closes the connection and joins the thread, which is the teardown every
/// case needs and none of them should have to spell.
struct Server {
    client: Option<Connection>,
    thread: Option<JoinHandle<()>>,
    next_id: i32,
}

impl Server {
    /// Past the handshake already, which is what a case not about the handshake
    /// wants. `server_loop` is the message loop on its own.
    fn started() -> Server {
        Server::spawn(|conn| server_loop(conn, Beans::new()))
    }

    /// From cold, driving the real `initialize` exchange, and answering with what
    /// the server advertised.
    fn initialized() -> (Server, InitializeResult) {
        let mut server = Server::spawn(|conn| run(conn, Beans::new()));

        let response = server.request("initialize", InitializeParams::default());
        let result = serde_json::from_value(response.result.expect("initialize result"))
            .expect("an InitializeResult");
        server.notify("initialized", InitializedParams {});

        (server, result)
    }

    fn spawn(serve: fn(Connection)) -> Server {
        let (server_conn, client) = Connection::memory();
        Server {
            client: Some(client),
            thread: Some(std::thread::spawn(move || serve(server_conn))),
            next_id: 0,
        }
    }

    fn client(&self) -> &Connection {
        self.client.as_ref().expect("the client outlives the case")
    }

    fn notify<P: serde::Serialize>(&self, method: &str, params: P) {
        let notification = Notification::new(method.to_string(), params);
        self.client()
            .sender
            .send(Message::Notification(notification))
            .expect("the server is listening");
    }

    fn request<P: serde::Serialize>(&mut self, method: &str, params: P) -> Response {
        self.next_id += 1;
        let request = Request::new(RequestId::from(self.next_id), method.to_string(), params);
        self.client()
            .sender
            .send(Message::Request(request))
            .expect("the server is listening");

        match self.client().receiver.recv().expect("a reply") {
            Message::Response(response) => response,
            other => panic!("expected a response, got {other:?}"),
        }
    }

    /// The next diagnostics publish. Every document notification provokes one,
    /// so a case that skips reading it leaves the message for the next.
    fn published(&self) -> PublishDiagnosticsParams {
        self.try_published().expect("a diagnostics publish")
    }

    /// The same, for a case whose point is that nothing is published. The wait is
    /// what makes a missing message distinguishable from a slow one.
    fn try_published(&self) -> Option<PublishDiagnosticsParams> {
        let message = self
            .client()
            .receiver
            .recv_timeout(Duration::from_millis(500))
            .ok()?;
        let Message::Notification(published) = message else {
            panic!("expected a notification, got {message:?}");
        };
        assert_eq!(published.method, PublishDiagnostics::METHOD);
        Some(
            published
                .extract(PublishDiagnostics::METHOD)
                .expect("a PublishDiagnosticsParams payload"),
        )
    }

    fn open(&self, uri: &str, text: &str) {
        self.notify(
            DidOpenTextDocument::METHOD,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.parse().expect("a uri"),
                    language_id: "java".into(),
                    version: 1,
                    text: text.into(),
                },
            },
        );
    }

    fn change(&self, uri: &str, version: i32, text: &str) {
        self.notify(
            DidChangeTextDocument::METHOD,
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.parse().expect("a uri"),
                    version,
                },
                // FULL sync sends the whole document as one entry.
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: text.into(),
                }],
            },
        );
    }

    fn close(&self, uri: &str) {
        self.notify(
            DidCloseTextDocument::METHOD,
            DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier {
                    uri: uri.parse().expect("a uri"),
                },
            },
        );
    }

    /// Goto-declaration and goto-definition take the same params and answer with
    /// the same shape, so one helper serves both and the method is an argument.
    fn goto(&mut self, method: &str, uri: &str, position: Position) -> Response {
        self.request(
            method,
            GotoDefinitionParams {
                text_document_position_params: position_in(uri, position),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        )
    }

    fn locations(&mut self, method: &str, uri: &str, position: Position) -> Vec<Location> {
        let result = self.goto(method, uri, position).result.expect("a result");
        serde_json::from_value(result).expect("an array of locations")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Closing the client ends the server's loop; joining then surfaces a
        // panic on that thread instead of losing it.
        self.client.take();
        if let Some(thread) = self.thread.take() {
            thread.join().expect("the server thread ended cleanly");
        }
    }
}

fn position_in(uri: &str, position: Position) -> TextDocumentPositionParams {
    TextDocumentPositionParams {
        text_document: TextDocumentIdentifier {
            uri: uri.parse().expect("a uri"),
        },
        position,
    }
}

const A: &str = "file:///workspace/A.java";
const B: &str = "file:///workspace/B.java";

mod handshake {
    use super::*;

    /// The client reads these once and behaves accordingly for the rest of the
    /// session, so getting one wrong disables a feature silently rather than
    /// failing anything.
    #[test]
    fn initialize_advertises_what_the_client_needs() {
        let (_server, result) = Server::initialized();
        let capabilities = result.capabilities;

        assert_eq!(
            capabilities.text_document_sync,
            Some(lsp_types::TextDocumentSyncCapability::Kind(
                lsp_types::TextDocumentSyncKind::FULL
            )),
            "without this the client never sends didOpen and no diagnostics surface",
        );
        assert_eq!(
            capabilities.declaration_provider,
            Some(lsp_types::DeclarationCapability::Simple(true))
        );
        assert_eq!(
            capabilities.definition_provider,
            Some(lsp_types::OneOf::Left(true))
        );
        assert_eq!(
            capabilities.hover_provider,
            Some(lsp_types::HoverProviderCapability::Simple(true))
        );
        assert_eq!(
            capabilities
                .completion_provider
                .as_ref()
                .map(|options| options.resolve_provider),
            Some(Some(false)),
            "advertising resolve would invite a request we answer with nothing",
        );
    }

    /// The handshake hands over to the message loop rather than ending the
    /// session, which only a case that drives `run` from cold can show.
    #[test]
    fn a_document_opened_after_the_handshake_is_analyzed() {
        let (server, _) = Server::initialized();

        server.open(A, "package p;\n\nclass A {\n    B field;\n}\n");

        assert!(server.published().diagnostics.is_empty());
    }
}

mod documents {
    use super::*;

    #[test]
    fn opening_a_file_publishes_its_diagnostics() {
        let server = Server::started();

        server.open(A, "class A {\n    void m() { x = 1; }\n}\n");

        let published = server.published();
        assert_eq!(published.uri, A.parse().unwrap());
        assert_eq!(published.version, Some(1));
        assert_eq!(
            published.diagnostics.len(),
            1,
            "found {:?}",
            published.diagnostics
        );
    }

    /// FULL sync sends the whole document, and the publish that follows has to
    /// carry the new version or the client discards it as stale.
    #[test]
    fn changing_a_file_republishes_at_the_new_version() {
        let server = Server::started();
        server.open(A, "class A {\n    void m() { x = 1; }\n}\n");
        assert_eq!(server.published().diagnostics.len(), 1);

        server.change(A, 2, "class A {\n    void m() {}\n}\n");

        let published = server.published();
        assert_eq!(published.version, Some(2));
        assert!(
            published.diagnostics.is_empty(),
            "found {:?}",
            published.diagnostics
        );
    }

    /// Closing clears the squiggles and nothing else; the engine keeps the text
    /// so the file stays reachable as a navigation target. The absent version is
    /// the protocol's way of saying this publish belongs to no revision.
    #[test]
    fn closing_a_file_clears_its_diagnostics_without_a_version() {
        let server = Server::started();
        server.open(A, "class A {\n    void m() { x = 1; }\n}\n");
        assert_eq!(server.published().diagnostics.len(), 1);

        server.close(A);

        let published = server.published();
        assert!(published.diagnostics.is_empty());
        assert_eq!(published.version, None);
    }

    /// An editor sends notifications for buffers that are not files at all. There
    /// is no path to key such a document by, so it is dropped rather than
    /// guessed at, and dropping it must stay quiet.
    #[test]
    fn a_document_with_no_path_is_ignored() {
        let server = Server::started();

        server.open("untitled:Untitled-1", "class A {}\n");

        assert!(server.try_published().is_none());
    }
}

mod requests {
    use super::*;

    /// The engine answers in byte spans against a source; the protocol wants a
    /// uri and a line/column range. This is that translation, and the only part
    /// of a goto reply this crate owns.
    #[test]
    fn a_goto_reply_carries_the_target_uri_and_range() {
        let mut server = Server::started();
        server.open(
            A,
            "class A {\n    void b(int c) {\n        int d = c;\n    }\n}\n",
        );
        server.published();

        // The `c` in `int d = c;`, which the engine sends back to the parameter.
        let locations = server.locations(GotoDeclaration::METHOD, A, Position::new(2, 16));

        assert_eq!(
            locations,
            vec![Location {
                uri: A.parse().unwrap(),
                range: lsp_types::Range::new(Position::new(1, 15), Position::new(1, 16)),
            }]
        );
    }

    /// Two methods, one answer shape. They are separate capabilities in the
    /// protocol and separate arms in the request handler, and a client may send
    /// either, so neither arm may drift.
    #[test]
    fn declaration_and_definition_reply_alike() {
        let mut server = Server::started();
        let text = "class A {\n    int a;\n\n    void b(B c) {\n        this.a = 1;\n    }\n}\n";
        server.open(A, text);
        server.published();

        // The `a` in `this.a = 1;`.
        let at = Position::new(4, 13);
        let declaration = server.locations(GotoDeclaration::METHOD, A, at);
        let definition = server.locations(GotoDefinition::METHOD, A, at);

        assert_eq!(declaration, definition);
        assert!(!declaration.is_empty());
    }

    /// A target in another file is ranged from that file's own line index, not
    /// from the buffer the request came in on. Reading the wrong one puts the
    /// range at a plausible but wrong place, which no shorter test would catch.
    #[test]
    fn a_range_comes_from_the_file_holding_the_target() {
        let mut server = Server::started();
        // `B` is declared on the third line of B.java and the second of A.java,
        // so the two files disagree about where a plausible answer would sit.
        server.open(A, "class A {\n    B field;\n}\n");
        server.published();
        server.open(B, "\n\nclass B {}\n");
        server.published();

        let locations = server.locations(GotoDefinition::METHOD, A, Position::new(1, 4));

        assert_eq!(
            locations,
            vec![Location {
                uri: B.parse().unwrap(),
                range: lsp_types::Range::new(Position::new(2, 6), Position::new(2, 7)),
            }]
        );
    }

    /// A request about a file the server was never sent. The protocol wants a
    /// successful reply carrying nothing, and an error here shows up in the
    /// editor as a broken server rather than as a name that resolves nowhere.
    #[test]
    fn a_request_for_an_unknown_file_replies_with_nothing() {
        let mut server = Server::started();

        let response = server.goto(GotoDeclaration::METHOD, A, Position::new(0, 0));

        assert_eq!(response.result, Some(serde_json::Value::Null));
        assert!(response.error.is_none(), "{:?}", response.error);
    }

    #[test]
    fn hover_carries_the_range_of_the_declaration_it_describes() {
        let mut server = Server::started();
        server.open(A, "class Outer {}");
        server.published();

        let response = server.request(
            HoverRequest::METHOD,
            lsp_types::HoverParams {
                text_document_position_params: position_in(A, Position::new(0, 8)),
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        );
        let hover: Hover =
            serde_json::from_value(response.result.expect("a result")).expect("a Hover");

        assert_eq!(
            hover.range,
            Some(lsp_types::Range::new(
                Position::new(0, 6),
                Position::new(0, 11)
            ))
        );
    }

    /// What the answer should be is settled in `lang-java`; this is about the
    /// envelope: a list rather than a bare array, marked incomplete because the
    /// engine filtered by the prefix behind the caret, and each row carrying the
    /// edit that replaces what was typed.
    #[test]
    fn completion_replies_with_an_incomplete_list_of_edits() {
        let mut server = Server::started();
        server.open(A, "class Outer {\n    class Inner {}\n    In f;\n}\n");
        server.published();

        let response = server.request(
            Completion::METHOD,
            lsp_types::CompletionParams {
                text_document_position: position_in(A, Position::new(2, 6)),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            },
        );
        let list: lsp_types::CompletionList =
            serde_json::from_value(response.result.expect("a result")).expect("a CompletionList");

        assert!(list.is_incomplete);
        let labels: Vec<&str> = list.items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, ["Inner"]);
        assert_eq!(
            list.items[0].text_edit,
            Some(lsp_types::CompletionTextEdit::Edit(lsp_types::TextEdit {
                range: lsp_types::Range::new(Position::new(2, 4), Position::new(2, 6)),
                new_text: "Inner".to_string(),
            }))
        );
    }
}
