#![cfg(feature = "sidecar")]

#[path = "support/mod.rs"]
mod support;

use std::{fs, path::Path, time::Duration};

use beans_core::location::Location;
use beans_engine::Engine;
use beans_testing::TempDir;
use lsp_server::{Message, Request};
use serde_json::json;
use support::{Client, initialize_params, request};

#[test]
#[ignore = "requires JAVA_HOME, prebuilt sidecar JAR and Gradle wrapper distribution/dependencies"]
fn command_imports_gradle_scopes_while_language_requests_remain_responsive() {
    let files = TempDir::new();
    let root = fs::canonicalize(files.root()).unwrap();
    fs::write(
        root.join("settings.gradle"),
        "rootProject.name = 'lsp-import'\n",
    )
    .unwrap();
    fs::write(root.join("build.gradle"), "plugins { id 'java' }\n").unwrap();
    fs::create_dir_all(root.join("src/main/java")).unwrap();
    fs::write(root.join("src/main/java/Use.java"), "class Use {}\n").unwrap();
    fs::create_dir_all(root.join("gradle/wrapper")).unwrap();
    fs::write(
        root.join("gradle/wrapper/gradle-wrapper.properties"),
        include_str!("../../../sidecar/gradle/wrapper/gradle-wrapper.properties"),
    )
    .unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut params = initialize_params(&root);
    params["initializationOptions"] = json!({
        "sidecarJar": repository.join("sidecar/build/libs/beans-sidecar.jar")
    });
    let mut client = Client::default();
    assert!(request(&mut client, "initialize", params).error.is_none());

    // Request handling must remain responsive while Gradle configures the build.
    client.send(Request::new(
        100.into(),
        "workspace/executeCommand".into(),
        json!({
            "command": "beans.import.gradle", "arguments": []
        }),
    ));
    client.send(Request::new(101.into(), "textDocument/hover".into(), json!({
        "textDocument": {"uri": "untitled:missing.java"}, "position": {"line": 0, "character": 0}
    })));
    let Message::Response(hover) = client.receive() else {
        panic!("expected hover response")
    };
    assert_eq!(hover.id, 101.into());
    assert!(hover.error.is_none());
    let Message::Response(import) = client.receive_timeout(Duration::from_secs(120)) else {
        panic!("expected import response")
    };
    assert_eq!(import.id, 100.into());
    assert!(import.error.is_none(), "{import:?}");

    // The response means the metadata is published, not merely dispatched.
    let engine = Engine::new(&root).unwrap();
    assert_eq!(
        engine
            .scopes_for(&Location::file(root.join("src/main/java/Use.java")))
            .unwrap()
            .len(),
        1
    );
    assert!(!root.join("build/classes").exists());
    assert!(
        request(&mut client, "shutdown", json!(null))
            .error
            .is_none()
    );
    client.exit().unwrap();
}
