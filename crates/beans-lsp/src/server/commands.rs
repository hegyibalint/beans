use std::io;
use std::thread::JoinHandle;

use crossbeam_channel::Sender;
use lsp_server::{ErrorCode, Message, Request, RequestId, Response};
use lsp_types::ExecuteCommandParams;

use super::{Server, request_log::RequestLog};

pub(super) const IMPORT_GRADLE: &str = "beans.import.gradle";

pub(super) struct PendingImport {
    id: RequestId,
    worker: JoinHandle<Result<(), String>>,
}

impl Server {
    pub(super) fn execute_command(&mut self, request: Request) -> Option<Response> {
        let params: ExecuteCommandParams = match serde_json::from_value(request.params) {
            Ok(params) => params,
            Err(error) => {
                return Some(Response::new_err(
                    request.id,
                    ErrorCode::InvalidParams as i32,
                    error.to_string(),
                ));
            }
        };
        if params.command != IMPORT_GRADLE || !params.arguments.is_empty() {
            return Some(Response::new_err(
                request.id,
                ErrorCode::InvalidParams as i32,
                "expected beans.import.gradle with no arguments".into(),
            ));
        }
        if !self.pending_imports.is_empty() {
            return Some(Response::new_err(
                request.id,
                ErrorCode::RequestFailed as i32,
                "a Gradle import is already running".into(),
            ));
        }

        // Keep the request pending while the owned completion worker registers the result.
        #[cfg(feature = "sidecar")]
        let result = self.features.as_mut().unwrap().import_gradle();
        #[cfg(not(feature = "sidecar"))]
        let result: Result<JoinHandle<Result<(), String>>, String> =
            Err("Gradle import requires beans-lsp built with --features sidecar".into());
        match result {
            Ok(worker) => {
                self.pending_imports.push(PendingImport {
                    id: request.id,
                    worker,
                });
                None
            }
            Err(error) => Some(Response::new_err(
                request.id,
                ErrorCode::RequestFailed as i32,
                error,
            )),
        }
    }

    pub(super) fn complete_imports(
        &mut self,
        sender: &Sender<Message>,
        log: &RequestLog,
    ) -> io::Result<()> {
        let mut index = 0;
        while index < self.pending_imports.len() {
            if !self.pending_imports[index].worker.is_finished() {
                index += 1;
                continue;
            }
            let pending = self.pending_imports.swap_remove(index);
            let result = pending
                .worker
                .join()
                .unwrap_or_else(|_| Err("Gradle import completion thread panicked".into()));
            let response = match result {
                Ok(()) => Response::new_ok(pending.id, ()),
                Err(error) => Response::new_err(pending.id, ErrorCode::RequestFailed as i32, error),
            };
            let message = Message::Response(response);
            log.record("outgoing", &message)?;
            sender.send(message).map_err(io::Error::other)?;
        }
        Ok(())
    }

    pub(super) fn join_imports(&mut self) {
        for pending in self.pending_imports.drain(..) {
            if pending.worker.join().is_err() {
                log::warn!("Gradle import completion thread panicked");
            }
        }
    }
}
