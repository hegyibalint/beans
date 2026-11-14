#![cfg(feature = "sidecar")]

#[path = "support/mod.rs"]
mod support;

use std::{fs, path::Path, time::Duration};

use beans_core::location::Location;
use beans_engine::Engine;
use beans_testing::TempDir;
use lsp_server::{Message, Notification, Request};
use serde_json::json;
use support::{Client, initialize_params, request};
use url::Url;

#[test]
#[ignore = "requires JAVA_HOME, prebuilt sidecar JAR and Gradle wrapper distribution/dependencies"]
fn class_directory_dependencies_support_definition_and_hover_without_compilation() {
    let files = TempDir::new();
    let root = fs::canonicalize(files.root()).unwrap();
    fs::write(
        root.join("settings.gradle"),
        "rootProject.name = 'lsp-navigation'\ninclude 'app', 'lib'\n",
    )
    .unwrap();
    fs::write(
        root.join("build.gradle"),
        "subprojects { apply plugin: 'java-library' }\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("app/src/main/java/p")).unwrap();
    fs::create_dir_all(root.join("lib/src/main/java/q")).unwrap();
    fs::write(
        root.join("app/build.gradle"),
        "dependencies { implementation project(':lib') }\n",
    )
    .unwrap();
    let contents = "package p; import q.Target; class Use { Target field; }";
    let target_contents = "package q; public class Target {}";
    let source = root.join("app/src/main/java/p/Use.java");
    let target = root.join("lib/src/main/java/q/Target.java");
    fs::write(&source, contents).unwrap();
    fs::write(&target, target_contents).unwrap();
    let mut client = initialize(&root);

    // Import the real class-directory variant before asking language features to resolve it.
    client.send(Request::new(
        100.into(),
        "workspace/executeCommand".into(),
        json!({"command": "beans.import.gradle", "arguments": []}),
    ));
    let Message::Response(import) = client.receive_timeout(Duration::from_secs(120)) else {
        panic!("expected import response")
    };
    assert_eq!(import.id, 100.into());
    assert!(import.error.is_none(), "{import:?}");
    let uri = Url::from_file_path(&source).unwrap().to_string();
    client.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1, "text": contents
        }}),
    ));
    let params = json!({
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": contents.rfind("Target").unwrap()}
    });
    let definition = request(&mut client, "textDocument/definition", params.clone());
    let start = target_contents.find("Target").unwrap();
    assert_eq!(
        definition.result,
        Some(json!({
            "uri": Url::from_file_path(&target).unwrap().as_str(),
            "range": {
                "start": {"line": 0, "character": start},
                "end": {"line": 0, "character": start + "Target".len()}
            }
        }))
    );
    let hover = request(&mut client, "textDocument/hover", params);
    assert_eq!(
        hover.result.unwrap()["contents"],
        json!({
            "kind": "plaintext",
            "value": "Type: q.Target\nLocated in: lib/src/main/java/q/Target.java\nVia: lib/build/classes/java/main"
        })
    );
    assert!(!root.join("lib/build").exists());
    assert!(!root.join("app/build").exists());
    assert!(
        request(&mut client, "shutdown", json!(null))
            .error
            .is_none()
    );
    client.exit().unwrap();
}

#[test]
#[ignore = "requires JAVA_HOME, prebuilt sidecar JAR and Gradle wrapper distribution/dependencies"]
fn command_imports_gradle_modules_while_language_requests_remain_responsive() {
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
    let mut client = initialize(&root);

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
    let modules = engine.modules().unwrap();
    assert_eq!(modules.len(), 1);
    let source = Location::file(root.join("src/main/java/Use.java"));
    assert_eq!(
        modules[0]
            .classpaths
            .iter()
            .filter(|classpath| { classpath.inputs.iter().any(|input| input.contains(&source)) })
            .count(),
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

fn initialize(root: &Path) -> Client {
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
    let mut params = initialize_params(root);
    params["initializationOptions"] = json!({
        "sidecarJar": repository.join("sidecar/build/libs/beans-sidecar.jar")
    });
    let mut client = Client::default();
    assert!(request(&mut client, "initialize", params).error.is_none());
    client
}
