use std::io;

use lsp_server::{Connection, Message};

use crate::features::Features;

mod notifications;
mod requests;

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
    features: Features,
}

impl Server {
    /// Serves a connection until `exit`.
    ///
    /// An exit without shutdown, a disconnected client, or a failed send is an error.
    /// Lifecycle rules: <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initialize>.
    pub fn run(mut self, connection: Connection) -> io::Result<()> {
        for message in &connection.receiver {
            match message {
                Message::Request(request) => {
                    let response = self.respond(request);
                    connection
                        .sender
                        .send(response.into())
                        .map_err(io::Error::other)?;
                }
                Message::Notification(notification) if notification.method == "exit" => {
                    // LSP #exit requires failure unless shutdown was received first.
                    return match self.lifecycle {
                        Lifecycle::Shutdown => Ok(()),
                        _ => Err(io::Error::other("exit received before shutdown")),
                    };
                }
                Message::Notification(notification) => self.notify(notification),
                // There are no outgoing requests, so responses need no dispatch yet.
                Message::Response(_) => {}
            }
        }
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "client disconnected before exit",
        ))
    }
}

pub fn run(connection: Connection) -> io::Result<()> {
    Server::default().run(connection)
}
