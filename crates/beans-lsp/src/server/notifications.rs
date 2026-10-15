use lsp_server::Notification;
use lsp_types::{
    PublishDiagnosticsParams,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
        PublishDiagnostics,
    },
};

use super::Server;

impl Server {
    pub(super) fn notify(&mut self, notification: Notification) -> Vec<Notification> {
        let revision = self.features.revision();
        let mut publications = Vec::new();
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(params) = notification.extract(DidOpenTextDocument::METHOD) {
                    self.features.did_open(params);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(params) = notification.extract(DidChangeTextDocument::METHOD) {
                    self.features.did_change(params);
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(params) = notification
                    .extract::<lsp_types::DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)
                {
                    let uri = params.text_document.uri.clone();
                    self.features.did_close(params);
                    if self.features.revision() != revision {
                        publications.push(PublishDiagnosticsParams::new(uri, Vec::new(), None));
                    }
                }
            }
            _ => {}
        }
        if self.features.revision() != revision {
            publications.extend(self.features.diagnostics());
        }
        publications
            .into_iter()
            .map(|params| Notification::new(PublishDiagnostics::METHOD.into(), params))
            .collect()
    }
}
