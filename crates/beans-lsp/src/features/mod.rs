use std::{collections::HashMap, sync::Arc};

use beans_core::event::Notification;
use beans_engine::Engine;
use crossbeam_channel::Receiver;
use lsp_types::ClientCapabilities;

use crate::model::open_document::OpenDocument;

mod diagnostics;
mod document_synchronization;
mod language_features;
mod semantic_tokens;
mod source_locations;
mod workspace;

/// Adapts engine features to LSP capabilities and document overlays.
pub(crate) struct Features {
    engine: Arc<Engine>,
    #[cfg(feature = "sidecar")]
    sidecar: beans_sidecar::Sidecar,
    pub(crate) engine_notifications: Option<Receiver<Notification>>,
    client_capabilities: ClientCapabilities,
    open_documents: HashMap<String, OpenDocument>,
    workspace_documents: HashMap<String, OpenDocument>,
}

#[cfg(feature = "sidecar")]
impl Drop for Features {
    fn drop(&mut self) {
        if let Err(error) = self.sidecar.stop() {
            log::warn!("Sidecar shutdown failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use beans_core::{features::HoverProvider, location::Location};
    use beans_testing::TempDir;
    use lsp_types::HoverContents;
    use serde_json::json;
    use url::Url;

    use super::Features;

    #[cfg(feature = "sidecar")]
    #[test]
    fn lsp_owns_the_sidecar_and_its_shared_engine_handle() {
        let files = TempDir::new();
        let params = serde_json::from_value(json!({
            "capabilities": {},
            "workspaceFolders": [{
                "uri": Url::from_directory_path(files.root()).unwrap().as_str(),
                "name": "project"
            }]
        }))
        .unwrap();
        let mut features = Features::initialize(&params).unwrap();

        features
            .sidecar
            .start(Arc::downgrade(&features.engine))
            .unwrap();

        assert_eq!(
            features.sidecar.stop().unwrap_err(),
            "workspace registration is not implemented"
        );
        assert_eq!(Arc::strong_count(&features.engine), 1);
    }

    #[test]
    fn lsp_queries_see_publications_from_another_engine_owner() {
        let files = TempDir::new();
        let params = serde_json::from_value(json!({
            "capabilities": {},
            "workspaceFolders": [{
                "uri": Url::from_directory_path(files.root()).unwrap().as_str(),
                "name": "project"
            }]
        }))
        .unwrap();
        let mut features = Features::initialize(&params).unwrap();
        let contents = "import q.Target; class Use { Target field; }";
        features
            .did_open(
                serde_json::from_value(json!({"textDocument": {
                    "uri": "untitled:Use.java", "languageId": "java", "version": 1,
                    "text": contents
                }}))
                .unwrap(),
            )
            .unwrap();
        let request = serde_json::from_value(json!({
            "textDocument": {"uri": "untitled:Use.java"},
            "position": {"line": 0, "character": contents.rfind("Target").unwrap()}
        }))
        .unwrap();
        let before = features.hover(&(), &request).unwrap().unwrap();
        let HoverContents::Markup(before) = before.contents else {
            panic!("expected markup content");
        };
        assert_eq!(before.value, "Target");

        let engine = Arc::clone(&features.engine);
        let source = Location::file(files.path("Target.java"));
        assert!(
            thread::spawn(move || {
                engine.process_document(source, "java", "package q; public class Target {}")
            })
            .join()
            .unwrap()
            .unwrap()
        );

        let after = features.hover(&(), &request).unwrap().unwrap();
        let HoverContents::Markup(after) = after.contents else {
            panic!("expected markup content");
        };
        assert_eq!(after.value, "Type: q.Target\nLocated in: Target.java");
    }
}
