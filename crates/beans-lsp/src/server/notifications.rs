use lsp_server::Notification;
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
};

use super::Server;

impl Server {
    pub(super) fn notify(&mut self, notification: Notification) {
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
                if let Ok(params) = notification.extract(DidCloseTextDocument::METHOD) {
                    self.features.did_close(params);
                }
            }
            _ => {}
        }
    }
}
