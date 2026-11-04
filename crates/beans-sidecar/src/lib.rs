//! Sidecar lifecycle and callbacks into the Beans engine.

use std::{
    sync::Weak,
    thread::{self, JoinHandle},
};

use beans_engine::Engine;

#[derive(Default)]
pub struct Sidecar {
    running: Option<RunningSidecar>,
}

impl Sidecar {
    pub fn start(&mut self, engine: Weak<Engine>) -> Result<(), String> {
        if self.running.is_none() {
            self.running = Some(RunningSidecar::start(engine)?);
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(running) = self.running.take() {
            running.stop()?;
        }
        Ok(())
    }
}

struct RunningSidecar {
    reader: JoinHandle<Result<(), String>>,
}

impl RunningSidecar {
    fn start(engine: Weak<Engine>) -> Result<Self, String> {
        let reader = thread::Builder::new()
            .name("beans-sidecar-reader".into())
            .spawn(move || Self::read_loop(engine))
            .map_err(|error| error.to_string())?;
        Ok(Self { reader })
    }

    fn read_loop(engine: Weak<Engine>) -> Result<(), String> {
        // Model one decoded result until the transport is introduced.
        let message = Message::RegisterWorkspaces;
        let engine = engine.upgrade().ok_or("engine is no longer available")?;
        match message {
            Message::RegisterWorkspaces => engine.register_workspaces(),
        }
    }

    fn stop(self) -> Result<(), String> {
        self.reader
            .join()
            .map_err(|_| "sidecar reader thread panicked".to_string())?
    }
}

enum Message {
    RegisterWorkspaces,
}

#[cfg(test)]
mod tests {
    use super::Sidecar;

    #[test]
    fn a_new_sidecar_is_idle() {
        assert!(Sidecar::default().running.is_none());
    }
}
