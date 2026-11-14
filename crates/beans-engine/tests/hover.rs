use beans_core::{
    artifact_sources::ArtifactSources,
    classpath::Classpath,
    features::HoverRequest,
    location::{Location, LocationEntry},
    module::Module,
    resource_filter::ResourceFilter,
};
use beans_engine::{Engine, ImportKind};
use beans_testing::TempDir;

#[test]
fn java_hover_reaches_the_shared_feature_contract() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let source = Location::new(LocationEntry::Virtual("untitled:Example.java".into()));
    let contents = "class C extends Target {}";
    engine
        .process_document(source.clone(), "java", contents)
        .unwrap();

    let info = engine
        .feature_hover(&HoverRequest {
            source: &source,
            contents,
            offset: 16,
        })
        .unwrap()
        .unwrap();

    assert_eq!(info.contents().plaintext(), "Target");
    let range = info.range();
    assert_eq!(&contents[range.start()..range.end()], "Target");
    assert!(
        engine
            .feature_hover(&HoverRequest {
                source: &Location::file("/README.md"),
                contents,
                offset: 16,
            })
            .unwrap()
            .is_none()
    );
}

#[test]
fn hover_only_requires_a_request() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let source = Location::file("/src/Example.java");
    let contents = "class C extends Target {}";
    engine.process(source.clone(), contents).unwrap();

    let response = engine
        .feature_hover(&HoverRequest {
            source: &source,
            contents,
            offset: contents.find("Target").unwrap(),
        })
        .unwrap()
        .unwrap();

    assert_eq!(response.contents().plaintext(), "Target");
    let range = response.range();
    assert_eq!(&contents[range.start()..range.end()], "Target");
}

#[test]
fn resolved_hover_relativizes_workspace_sources() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let contents = "package demo; class C { C value; }";
    let source = Location::file(files.path("src/demo/C.java"));
    engine.process(source.clone(), contents).unwrap();

    let response = engine
        .feature_hover(&HoverRequest {
            source: &source,
            contents,
            offset: contents.find("C value").unwrap(),
        })
        .unwrap()
        .unwrap();

    assert_eq!(
        response.contents().plaintext(),
        "Type: demo.C\nLocated in: src/demo/C.java"
    );
    assert_eq!(
        response.contents().markdown(),
        Some("**Type:** ` demo.C `\n\n**Located in:** ` src/demo/C.java `")
    );
}

#[test]
fn hover_shows_the_artifact_admitted_by_the_requesters_classpath() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let contents = "import q.Target; class Use { Target field; }";
    let target = files.path("lib/Target.java");
    engine
        .process(Location::file(&target), "package q; public class Target {}")
        .unwrap();

    // Reach the same declaration through a different artifact for each requester.
    let mut classpaths = Vec::new();
    let mut artifacts = Vec::new();
    for name in ["first", "second"] {
        let source = files.path(format!("{name}/Use.java"));
        let artifact = files.path(format!("lib/{name}.jar"));
        engine.process(Location::file(&source), contents).unwrap();
        classpaths.push(Classpath {
            inputs: vec![ResourceFilter::file(source)],
            entries: Some(vec![ResourceFilter::file(&artifact)]),
        });
        artifacts.push(Location::file(artifact));
    }
    engine
        .register_modules(
            vec![Module {
                classpaths,
                artifact_sources: vec![ArtifactSources {
                    artifacts,
                    sources: vec![ResourceFilter::file(target)],
                }],
            }],
            ImportKind::Gradle,
            files.root(),
        )
        .unwrap();

    // Provenance belongs to the request, not the indexed declaration.
    for name in ["first", "second"] {
        let source = Location::file(files.path(format!("{name}/Use.java")));
        let response = engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: contents.rfind("Target").unwrap(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            response.contents().plaintext(),
            format!("Type: q.Target\nLocated in: lib/Target.java\nVia: lib/{name}.jar")
        );
        assert_eq!(
            response.contents().markdown(),
            Some(
                format!(
                    "**Type:** ` q.Target `\n\n**Located in:** ` lib/Target.java `\n\n**Via:** ` lib/{name}.jar `"
                )
                .as_str()
            )
        );
        assert!(!files.path(format!("lib/{name}.jar")).exists());
    }
}

#[test]
fn hover_uses_the_current_model() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let source = Location::file("/src/Example.java");
    for (contents, expected) in [
        ("class C extends Former {}", "Former"),
        ("class C extends Latest {}", "Latest"),
    ] {
        engine.process(source.clone(), contents).unwrap();
        let response = engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: contents.find(expected).unwrap(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(response.contents().plaintext(), expected);
    }
}

#[test]
fn hover_of_an_unknown_resource_returns_none() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let source = Location::file("/src/Missing.java");

    assert!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents: "class C extends Target {}",
                offset: 16,
            })
            .unwrap()
            .is_none()
    );
}
