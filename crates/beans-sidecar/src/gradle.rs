use std::{
    path::{Path, PathBuf},
    sync::{Weak, mpsc},
    thread::{self, JoinHandle},
};

use beans_core::{module::Module, paths::normalize};
use beans_engine::{Engine, ImportKind};
use serde::Serialize;

use crate::{Sidecar, rpc::Reply};

impl Sidecar {
    /// Imports a Gradle snapshot asynchronously; join the worker to observe publication or failure.
    /// Waiting for the response does not keep the engine alive.
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
    let modules: Vec<Module> = serde_json::from_value(result)
        .map_err(|error| format!("invalid Gradle import result: {error}"))?;

    // Keep the engine alive only while registering the completed import.
    let engine = engine
        .upgrade()
        .ok_or("engine dropped before Gradle import completed")?;
    engine.register_modules(modules, ImportKind::Gradle, &root)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use beans_core::{
        artifact_sources::ArtifactSources, classpath::Classpath, location::Location,
        resource_filter::ResourceFilter,
    };
    use beans_testing::TempDir;

    use super::*;

    #[test]
    fn completed_imports_register_complete_logical_modules() {
        let files = TempDir::new();
        let engine = Arc::new(Engine::new(files.root()).unwrap());
        let modules = vec![Module {
            classpaths: vec![Classpath {
                inputs: vec![ResourceFilter::file(files.path("missing-sources"))],
                entries: Some(vec![]),
            }],
            artifact_sources: vec![ArtifactSources {
                artifacts: vec![Location::file(files.path("missing.jar"))],
                sources: vec![ResourceFilter::file(files.path("missing-sources/Use.java"))],
            }],
        }];
        let expected = serde_json::to_value(modules).unwrap();
        let (sender, receiver) = mpsc::channel();
        sender.send(Ok(expected.clone())).unwrap();

        import_thread(receiver, Arc::downgrade(&engine), files.root().into()).unwrap();

        assert_eq!(
            serde_json::to_value(engine.modules().unwrap()).unwrap(),
            expected
        );
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
    fn malformed_results_preserve_registered_modules() {
        let files = TempDir::new();
        let engine = Arc::new(Engine::new(files.root()).unwrap());
        engine
            .register_modules(
                vec![Module {
                    classpaths: vec![],
                    artifact_sources: vec![],
                }],
                ImportKind::Gradle,
                files.root(),
            )
            .unwrap();
        let expected = serde_json::to_value(engine.modules().unwrap()).unwrap();
        for result in [
            serde_json::Value::Null,
            serde_json::json!([{"inputs": [], "dependencies": []}]),
            serde_json::json!([{"classpaths": [], "artifact_sources": null}]),
        ] {
            let (sender, receiver) = mpsc::channel();
            sender.send(Ok(result)).unwrap();

            let error =
                import_thread(receiver, Arc::downgrade(&engine), files.root().into()).unwrap_err();
            assert!(error.starts_with("invalid Gradle import result:"));
            assert_eq!(
                serde_json::to_value(engine.modules().unwrap()).unwrap(),
                expected
            );
        }
    }
}
