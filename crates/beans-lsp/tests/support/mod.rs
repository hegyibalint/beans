#![allow(dead_code)]

use std::{
    io,
    path::Path,
    thread::{self, JoinHandle},
    time::Duration,
};

use beans_testing::{TempDir, template::Template};
use crossbeam_channel::{Receiver, bounded};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use serde_json::{Value, json};
use url::Url;

pub struct Client {
    connection: Option<Connection>,
    worker: Option<JoinHandle<io::Result<()>>>,
    stopped: Receiver<()>,
    next_id: i32,
}

impl Default for Client {
    fn default() -> Self {
        let (server, client) = Connection::memory();
        let (finished, stopped) = bounded(1);
        let worker = thread::spawn(move || {
            let result = beans_lsp::run(server);
            let _ = finished.send(());
            result
        });
        Self {
            connection: Some(client),
            worker: Some(worker),
            stopped,
            next_id: 1,
        }
    }
}

impl Client {
    pub fn send(&self, message: impl Into<Message>) {
        self.connection
            .as_ref()
            .unwrap()
            .sender
            .send(message.into())
            .unwrap();
    }

    pub fn receive(&self) -> Message {
        self.connection
            .as_ref()
            .unwrap()
            .receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("server did not respond")
    }

    pub fn notify(&mut self, notification: Notification) -> Vec<Notification> {
        self.send(notification);
        // A request on the same connection fences synchronous notification handling.
        self.exchange("$/testBarrier", Value::Null).1
    }

    pub fn exit(mut self) -> io::Result<()> {
        self.send(Notification::new("exit".into(), ()));
        let result = self.join();
        let remaining: Vec<_> = self
            .connection
            .as_ref()
            .unwrap()
            .receiver
            .try_iter()
            .collect();
        assert!(
            remaining.is_empty(),
            "unhandled server messages: {remaining:?}"
        );
        result
    }

    pub fn disconnect(mut self) -> io::Result<()> {
        self.connection.take();
        self.join()
    }

    fn exchange(&mut self, method: &str, params: Value) -> (Response, Vec<Notification>) {
        let id: lsp_server::RequestId = self.next_id.into();
        self.next_id += 1;
        self.send(Request::new(id.clone(), method.into(), params));
        let mut notifications = Vec::new();
        loop {
            match self.receive() {
                Message::Response(response) => {
                    assert_eq!(response.id, id);
                    return (response, notifications);
                }
                Message::Notification(notification) => notifications.push(notification),
                message => panic!("unexpected server message: {message:?}"),
            }
        }
    }

    fn join(&mut self) -> io::Result<()> {
        self.stopped
            .recv_timeout(Duration::from_secs(5))
            .expect("server did not stop");
        self.worker.take().unwrap().join().unwrap()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.connection.take();
        if self.worker.is_none() {
            return;
        }
        if thread::panicking() {
            if self.stopped.recv_timeout(Duration::from_secs(5)).is_ok() {
                let _ = self.worker.take().unwrap().join();
            }
        } else {
            let _ = self.join();
        }
    }
}

pub fn server() -> (TempDir, Client) {
    let root = TempDir::new();
    let mut client = Client::default();
    let response = request(&mut client, "initialize", initialize_params(root.root()));
    assert!(response.error.is_none(), "{response:?}");
    (root, client)
}

pub fn request(client: &mut Client, method: &str, params: Value) -> Response {
    let (response, notifications) = client.exchange(method, params);
    assert!(
        notifications.is_empty(),
        "unexpected notifications: {notifications:?}"
    );
    response
}

pub fn initialize_params(root: &Path) -> Value {
    json!({
        "capabilities": {},
        "workspaceFolders": [{
            "uri": Url::from_directory_path(root).unwrap().as_str(),
            "name": "test"
        }]
    })
}

pub fn position(fixture: &Template, offset: usize) -> Value {
    let before = &fixture.content[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count();
    let character = before
        .rsplit_once('\n')
        .map_or(before, |(_, current_line)| current_line)
        .encode_utf16()
        .count();
    json!({"line": line, "character": character})
}

pub fn assert_error(response: Response, code: ErrorCode) {
    assert_eq!(response.error.unwrap().code, code as i32);
    assert!(response.result.is_none());
}
