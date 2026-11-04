//! Sidecar lifecycle and callbacks into the Beans engine.

use std::{
    io::Read,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::Weak,
    thread::{self, JoinHandle},
};

use beans_engine::Engine;

#[derive(Default)]
pub struct Sidecar {
    running: Option<RunningSidecar>,
}

impl Sidecar {
    pub fn start(&mut self, engine: Weak<Engine>, command: Command) -> Result<(), String> {
        if self.running.is_none() {
            self.running = Some(RunningSidecar::start(engine, command)?);
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
    child: Child,
    stdin: ChildStdin,
    reader: JoinHandle<Result<(), String>>,
}

impl RunningSidecar {
    fn start(engine: Weak<Engine>, mut command: Command) -> Result<Self, String> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| error.to_string())?;
        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");
        let reader = match thread::Builder::new()
            .name("beans-sidecar-reader".into())
            .spawn(move || Self::read_thread(engine, stdout))
        {
            Ok(reader) => reader,
            Err(error) => {
                drop(stdin);
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        };
        Ok(Self {
            child,
            stdin,
            reader,
        })
    }

    fn read_thread(engine: Weak<Engine>, mut stdout: ChildStdout) -> Result<(), String> {
        // One byte stands in for a decoded result; this is not protocol framing.
        let mut byte = [0];
        if stdout.read(&mut byte).map_err(|error| error.to_string())? == 0 {
            return Ok(());
        }
        let message = Message::RegisterWorkspaces;
        let engine = engine.upgrade().ok_or("engine is no longer available")?;
        match message {
            Message::RegisterWorkspaces => engine.register_workspaces(),
        }
    }

    fn stop(mut self) -> Result<(), String> {
        // The proof child exits on stdin EOF; JVM shutdown policy is still deferred.
        drop(self.stdin);
        let status = self.child.wait().map_err(|error| error.to_string());
        let result = self
            .reader
            .join()
            .map_err(|_| "sidecar reader thread panicked".to_string())?;
        let status = status?;
        if !status.success() {
            return Err(format!("sidecar process exited with {status}"));
        }
        result
    }
}

enum Message {
    RegisterWorkspaces,
}

#[cfg(test)]
mod tests {
    use super::Sidecar;
    #[cfg(unix)]
    use super::{Command, RunningSidecar, Weak};

    #[test]
    fn a_new_sidecar_is_idle() {
        assert!(Sidecar::default().running.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn reader_owns_stdout_while_owner_uses_child_and_stdin() {
        use std::io::Write;

        let mut running = RunningSidecar::start(Weak::new(), Command::new("cat")).unwrap();

        assert!(running.child.stdout.is_none());
        assert!(running.child.stdin.is_none());
        assert!(running.child.try_wait().unwrap().is_none());
        running.stdin.write_all(b"x").unwrap();

        assert_eq!(running.stop().unwrap_err(), "engine is no longer available");
    }

    #[cfg(unix)]
    #[test]
    fn closing_stdin_unblocks_the_reader_without_dispatching_on_eof() {
        let running = RunningSidecar::start(Weak::new(), Command::new("cat")).unwrap();

        running.stop().unwrap();
    }
}
