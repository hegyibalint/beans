use std::{io, thread::JoinHandle};

use crossbeam_channel::Sender;
use lsp_server::{Connection, Message, Response};

use crate::features::Features;

mod commands;
mod engine_notifications;
mod notifications;
mod request_log;
mod requests;

use request_log::RequestLog;

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
    pending_imports: Vec<commands::PendingImport>,
}

impl Server {
    /// Serves a connection until `exit`.
    ///
    /// An exit without shutdown, a disconnected client, or a failed send is an error.
    /// Lifecycle rules: <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initialize>.
    pub fn run(mut self, connection: Connection) -> io::Result<()> {
        let log = RequestLog::from_env()?;
        let tick = crossbeam_channel::tick(std::time::Duration::from_millis(50));
        loop {
            let message = crossbeam_channel::select! {
                recv(connection.receiver) -> message => match message {
                    Ok(message) => message,
                    Err(_) => break,
                },
                recv(tick) -> _ => {
                    self.complete_imports(&connection.sender, &log)?;
                    continue;
                }
            };
            log.record("incoming", &message)?;
            match message {
                Message::Request(request) => {
                    let initializing = request.method == "initialize";
                    let response = if matches!(self.lifecycle, Lifecycle::Running)
                        && request.method == "workspace/executeCommand"
                    {
                        let Some(response) = self.execute_command(request) else {
                            continue;
                        };
                        response
                    } else {
                        self.respond(request)
                    };
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
        self.join_imports();
        if let Some(thread) = self.notification_thread.take()
            && thread.join().is_err()
        {
            log::warn!("LSP notification thread panicked");
        }
    }
}

pub fn run(connection: Connection) -> io::Result<()> {
    Server::default().run(connection)
}
