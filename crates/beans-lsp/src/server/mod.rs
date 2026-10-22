use std::{io, thread::JoinHandle};

use crossbeam_channel::Sender;
use lsp_server::{Connection, Message, Response};

use crate::features::Features;

mod engine_notifications;
mod notifications;
mod request_log;
mod requests;

use request_log::RequestLog;

#[cfg(test)]
mod tests;

#[derive(Default)]
enum Lifecycle {
    #[default]
    Uninitialized,
    Running,
    Shutdown,
}

/// The LSP transport and the feature handlers it serves.
#[derive(Default)]
pub struct Server {
    lifecycle: Lifecycle,
    features: Option<Features>,
    notification_thread: Option<JoinHandle<()>>,
    notification_replies: Option<Sender<Response>>,
}

impl Server {
    /// Serves a connection until `exit`.
    ///
    /// An exit without shutdown, a disconnected client, or a failed send is an error.
    /// Lifecycle rules: <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initialize>.
    pub fn run(mut self, connection: Connection) -> io::Result<()> {
        let log = RequestLog::from_env()?;
        for message in &connection.receiver {
            log.record("incoming", &message)?;
            match message {
                Message::Request(request) => {
                    let initializing = request.method == "initialize";
                    let response = self.respond(request);
                    let initialized = initializing && response.error.is_none();
                    let message = Message::Response(response);
                    log.record("outgoing", &message)?;
                    connection.sender.send(message).map_err(io::Error::other)?;
                    if initialized {
                        self.start_notification_thread(connection.sender.clone(), log.clone())?;
                    }
                }
                Message::Notification(notification) if notification.method == "exit" => {
                    // LSP #exit requires failure unless shutdown was received first.
                    return match self.lifecycle {
                        Lifecycle::Shutdown => Ok(()),
                        _ => Err(io::Error::other("exit received before shutdown")),
                    };
                }
                Message::Notification(notification) => {
                    for notification in self.notify(notification) {
                        let message = Message::Notification(notification);
                        log.record("outgoing", &message)?;
                        connection.sender.send(message).map_err(io::Error::other)?;
                    }
                }
                Message::Response(response) => {
                    if let Some(replies) = &self.notification_replies {
                        let _ = replies.send(response);
                    }
                }
            }
        }
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "client disconnected before exit",
        ))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.notification_replies.take();
        self.features.take();
        if let Some(thread) = self.notification_thread.take() {
            if thread.join().is_err() {
                log::warn!("LSP notification thread panicked");
            }
        }
    }
}

pub fn run(connection: Connection) -> io::Result<()> {
    Server::default().run(connection)
}
