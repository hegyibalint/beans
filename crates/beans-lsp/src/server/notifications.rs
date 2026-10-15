use lsp_server::Notification;
use lsp_types::{
    MessageType, PublishDiagnosticsParams, ShowMessageParams,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
        PublishDiagnostics, ShowMessage,
    },
};

use super::{Lifecycle, Server};

impl Server {
    pub(super) fn notify(&mut self, notification: Notification) -> Vec<Notification> {
        if !matches!(self.lifecycle, Lifecycle::Running) {
            return Vec::new();
        }
        let features = self.features.as_mut().unwrap();
        let mut closed = None;
        let result = match notification.method.as_str() {
            DidOpenTextDocument::METHOD => notification
                .extract(DidOpenTextDocument::METHOD)
                .map_or(Ok(false), |params| features.did_open(params)),
            DidChangeTextDocument::METHOD => notification
                .extract(DidChangeTextDocument::METHOD)
                .map_or(Ok(false), |params| features.did_change(params)),
            DidCloseTextDocument::METHOD => {
                match notification
                    .extract::<lsp_types::DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)
                {
                    Ok(params) => {
                        closed = Some(params.text_document.uri.clone());
                        features.did_close(params)
                    }
                    Err(_) => Ok(false),
                }
            }
            _ => Ok(false),
        };
        let changed = match result {
            Ok(changed) => changed,
            Err(error) => return vec![report_error(error)],
        };
        let mut publications = Vec::new();
        if changed {
            if let Some(uri) = closed {
                publications.push(PublishDiagnosticsParams::new(uri, Vec::new(), None));
            }
            match features.diagnostics() {
                Ok(diagnostics) => publications.extend(diagnostics),
                Err(error) => return vec![report_error(error)],
            }
        }
        publications
            .into_iter()
            .map(|params| Notification::new(PublishDiagnostics::METHOD.into(), params))
            .collect()
    }
}

fn report_error(error: String) -> Notification {
    log::error!("Document processing failed: {error}");
    Notification::new(
        ShowMessage::METHOD.into(),
        ShowMessageParams {
            typ: MessageType::ERROR,
            message: format!("Beans: {error}"),
        },
    )
}
