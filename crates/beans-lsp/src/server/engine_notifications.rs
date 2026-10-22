use std::{collections::HashSet, io, path::Path, thread};

use beans_core::event::Notification as EngineNotification;
use crossbeam_channel::{Receiver, Sender, unbounded};
use lsp_server::{Message, Notification, Request, Response};
use lsp_types::{
    ProgressParams, ProgressParamsValue, ProgressToken, WorkDoneProgress, WorkDoneProgressBegin,
    WorkDoneProgressCreateParams, WorkDoneProgressEnd,
    notification::{Notification as _, Progress},
    request::{Request as _, WorkDoneProgressCreate},
};

use super::{Server, request_log::RequestLog};

impl Server {
    pub(super) fn start_notification_thread(
        &mut self,
        sender: Sender<Message>,
        log: RequestLog,
    ) -> io::Result<()> {
        let Some(notifications) = self
            .features
            .as_mut()
            .and_then(|features| features.engine_notifications.take())
        else {
            return Ok(());
        };
        let (replies, responses) = unbounded();
        let thread = thread::Builder::new()
            .name("beans-lsp-notifications".into())
            .spawn(move || {
                if let Err(error) = consume_notifications(notifications, responses, sender, log) {
                    log::warn!("LSP notification thread failed: {error}");
                }
            })?;
        self.notification_replies = Some(replies);
        self.notification_thread = Some(thread);
        Ok(())
    }
}

fn consume_notifications(
    notifications: Receiver<EngineNotification>,
    responses: Receiver<Response>,
    sender: Sender<Message>,
    log: RequestLog,
) -> io::Result<()> {
    let mut active = HashSet::new();
    for notification in notifications {
        let (id, title, path) = match notification {
            EngineNotification::ProcessResourceStart(payload) => {
                (payload.id, "Processing resource", payload.data)
            }
            EngineNotification::ProcessJdkHomeStart(payload) => {
                (payload.id, "Processing JDK", payload.data)
            }
            EngineNotification::ProcessResourceStop(payload)
            | EngineNotification::ProcessJdkHomeStop(payload) => {
                if active.remove(&payload.id) {
                    send(&sender, &log, end_progress(payload.id))?;
                }
                continue;
            }
        };
        let request = create_progress(id);
        let request_id = request.id.clone();
        send(&sender, &log, request.into())?;
        // LSP 3.17 #window_workDoneProgress_create: rejected tokens must not be used.
        let Some(response) = responses.iter().find(|response| response.id == request_id) else {
            break;
        };
        if response.error.is_none() {
            send(&sender, &log, begin_progress(id, title, &path))?;
            active.insert(id);
        }
    }
    Ok(())
}

fn create_progress(id: u64) -> Request {
    Request::new(
        format!("beans:progress:{id}").into(),
        WorkDoneProgressCreate::METHOD.into(),
        WorkDoneProgressCreateParams { token: token(id) },
    )
}

fn begin_progress(id: u64, title: &str, path: &Path) -> Message {
    progress_message(
        id,
        WorkDoneProgress::Begin(WorkDoneProgressBegin {
            title: title.into(),
            message: Some(path.display().to_string()),
            cancellable: Some(false),
            percentage: None,
        }),
    )
}

fn end_progress(id: u64) -> Message {
    progress_message(id, WorkDoneProgress::End(WorkDoneProgressEnd::default()))
}

fn progress_message(id: u64, value: WorkDoneProgress) -> Message {
    Notification::new(
        Progress::METHOD.into(),
        ProgressParams {
            token: token(id),
            value: ProgressParamsValue::WorkDone(value),
        },
    )
    .into()
}

fn token(id: u64) -> ProgressToken {
    ProgressToken::String(format!("beans:{id}"))
}

fn send(sender: &Sender<Message>, log: &RequestLog, message: Message) -> io::Result<()> {
    log.record("outgoing", &message)?;
    sender.send(message).map_err(io::Error::other)
}

#[cfg(test)]
mod tests;
