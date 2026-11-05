//! Sidecar operations and process lifecycle.

use std::process::Command;

use session::Session;

mod rpc;
mod session;

#[derive(Default)]
pub struct Sidecar {
    session: Option<Session>,
}

impl Sidecar {
    pub fn start(&mut self, command: Command) -> Result<(), String> {
        if self.session.is_none() {
            self.session = Some(Session::start(command)?);
        }
        Ok(())
    }

    /// Waits for a correlated pong response.
    pub fn ping(&mut self) -> Result<(), String> {
        let session = self.session.as_mut().ok_or("sidecar is not running")?;
        let response = session.send("ping", None)?;
        let result = response
            .recv()
            .map_err(|_| "sidecar reader stopped without a response".to_string())??;
        if result.as_str() != Some("pong") {
            return Err(format!("ping returned an unexpected result: {result}"));
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(session) = self.session.take() {
            session.stop()?;
        }
        Ok(())
    }
}
