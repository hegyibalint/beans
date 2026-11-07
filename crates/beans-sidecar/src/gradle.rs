use std::{
    path::{Path, PathBuf},
    sync::{Weak, mpsc},
    thread::{self, JoinHandle},
};

use beans_core::{compilation_scope::CompilationScope, paths::normalize};
use beans_engine::{Engine, ImportKind};
use serde::Serialize;

use crate::{Sidecar, rpc::Reply};

impl Sidecar {
    pub fn import_gradle(
        &mut self,
        root: &Path,
        gradle_home: Option<&Path>,
        engine: Weak<Engine>,
    ) -> Result<JoinHandle<Result<(), String>>, String> {
        // Dispatch the request without retaining the engine or blocking the reader.
        let root = normalize(&std::path::absolute(root).map_err(|error| error.to_string())?);
        let params = serde_json::to_value(ImportParams {
            workspace_root: &root,
            gradle_home,
        })
        .map_err(|error| error.to_string())?;
        let session = self.session.as_mut().ok_or("sidecar is not running")?;
        let response = session.send("gradle/import", Some(params))?;

        // The caller owns this completion thread and can join it to observe publication.
        thread::Builder::new()
            .name("beans-gradle-import".into())
            .spawn(move || import_thread(response, engine, root))
            .map_err(|error| error.to_string())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportParams<'a> {
    workspace_root: &'a Path,
    gradle_home: Option<&'a Path>,
}

fn import_thread(
    response: mpsc::Receiver<Reply>,
    engine: Weak<Engine>,
    root: PathBuf,
) -> Result<(), String> {
    // Decode on the completion thread, not the protocol reader.
    let result = response
        .recv()
        .map_err(|_| "sidecar reader stopped without an import response")??;
    let scopes: Vec<CompilationScope> = serde_json::from_value(result)
        .map_err(|error| format!("invalid Gradle import result: {error}"))?;

    // Keep the engine alive only while registering the completed import.
    let engine = engine
        .upgrade()
        .ok_or("engine dropped before Gradle import completed")?;
    engine.register_scopes(scopes, ImportKind::Gradle, &root)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use beans_core::{location::Location, resource_filter::ResourceFilter};
    use beans_testing::TempDir;

    use super::*;

    #[test]
    fn completed_imports_register_logical_scopes() {
        let files = TempDir::new();
        let engine = Arc::new(Engine::new(files.root()).unwrap());
        let source = files.path("missing-sources/Use.java");
        let scopes = vec![CompilationScope {
            inputs: vec![ResourceFilter::file(files.path("missing-sources"))],
            dependencies: Some(vec![]),
        }];
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Ok(serde_json::to_value(scopes).unwrap()))
            .unwrap();

        import_thread(receiver, Arc::downgrade(&engine), files.root().into()).unwrap();

        let registered = engine.scopes_for(&Location::file(source)).unwrap();
        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].dependencies, Some(vec![]));
    }

    #[test]
    fn waiting_for_a_reply_does_not_keep_the_engine_alive() {
        let files = TempDir::new();
        let engine = Arc::new(Engine::new(files.root()).unwrap());
        let weak = Arc::downgrade(&engine);
        let (sender, receiver) = mpsc::channel();
        drop(engine);
        assert!(weak.upgrade().is_none());
        sender.send(Ok(serde_json::json!([]))).unwrap();

        let error = import_thread(receiver, weak, files.root().into()).unwrap_err();
        assert_eq!(error, "engine dropped before Gradle import completed");
    }

    #[test]
    fn malformed_results_do_not_register_scopes() {
        let files = TempDir::new();
        let engine = Arc::new(Engine::new(files.root()).unwrap());
        let (sender, receiver) = mpsc::channel();
        sender.send(Ok(serde_json::Value::Null)).unwrap();

        let error =
            import_thread(receiver, Arc::downgrade(&engine), files.root().into()).unwrap_err();
        assert!(error.starts_with("invalid Gradle import result:"));
        assert!(
            engine
                .scopes_for(&Location::file(files.path("Use.java")))
                .unwrap()
                .is_empty()
        );
    }
}
