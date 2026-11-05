use std::{
    io::{BufRead, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
};

use serde_json::Value;

use crate::rpc::{Reply, Rpc};

pub(super) struct Session {
    child: Child,
    stdin: ChildStdin,
    rpc: Arc<Rpc>,
    reader: JoinHandle<Result<(), String>>,
}

impl Session {
    pub fn start(mut command: Command) -> Result<Self, String> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| error.to_string())?;

        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");

        let rpc = Arc::new(Rpc::default());
        let reader_rpc = Arc::clone(&rpc);

        let reader = match thread::Builder::new()
            .name("beans-sidecar-reader".into())
            .spawn(move || Self::read_thread(stdout, reader_rpc))
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
            rpc,
            reader,
        })
    }

    pub fn send(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<mpsc::Receiver<Reply>, String> {
        self.rpc.send(method, params, &mut self.stdin)
    }

    fn read_thread(stdout: ChildStdout, rpc: Arc<Rpc>) -> Result<(), String> {
        let mut stdout = BufReader::new(stdout);
        let mut line = String::new();
        let result = loop {
            line.clear();
            match stdout.read_line(&mut line) {
                Ok(0) => break Ok(()),
                Ok(_) => {
                    if let Err(error) = rpc.receive(&line) {
                        break Err(error);
                    }
                }
                Err(error) => break Err(error.to_string()),
            }
        };
        let error = result
            .as_ref()
            .err()
            .map_or("sidecar stdout closed", String::as_str);
        rpc.close(error);
        result
    }

    pub fn stop(mut self) -> Result<(), String> {
        self.rpc.close("sidecar stopped");
        // Cooperative stdin-EOF shutdown; bounded JVM shutdown is still deferred.
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
