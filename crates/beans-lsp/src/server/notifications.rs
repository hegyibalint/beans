use lsp_server::Notification;
use lsp_types::{
    PublishDiagnosticsParams,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
        PublishDiagnostics,
    },
};

use super::{Lifecycle, Server};

impl Server {
    pub(super) fn notify(&mut self, notification: Notification) -> Vec<Notification> {
        if !matches!(self.lifecycle, Lifecycle::Running) {
            return Vec::new();
        }
        let features = self.features.as_mut().unwrap();
        let revision = features.revision();
        let mut publications = Vec::new();
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(params) = notification.extract(DidOpenTextDocument::METHOD) {
                    features.did_open(params);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(params) = notification.extract(DidChangeTextDocument::METHOD) {
                    features.did_change(params);
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(params) = notification
                    .extract::<lsp_types::DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)
                {
                    let uri = params.text_document.uri.clone();
                    features.did_close(params);
                    if features.revision() != revision {
                        publications.push(PublishDiagnosticsParams::new(uri, Vec::new(), None));
                    }
                }
            }
            _ => {}
        }
        if features.revision() != revision {
            publications.extend(features.diagnostics());
        }
        publications
            .into_iter()
            .map(|params| Notification::new(PublishDiagnostics::METHOD.into(), params))
            .collect()
    }
}
