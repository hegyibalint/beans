use beans_core::{
    artifact_sources::ArtifactSources,
    classpath::Classpath,
    features::{DefinitionRequest, HoverRequest},
    location::{Location, LocationEntry},
    module::Module,
    ranges::ByteRange,
    resource_filter::ResourceFilter,
    source::SourceSpan,
};
use beans_engine::{Engine, ImportKind};
use beans_testing::TempDir;

#[test]
fn artifact_metadata_updates_navigation_hover_and_diagnostics_without_reparsing() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let source = Location::file(files.path("app/Use.java"));
    let target = Location::file(files.path("lib/Target.java"));
    let jar = files.path("lib/build/api.jar");
    let use_text = "package p; import q.Target; class Use { Target field; }";
    let target_text = "package q; public class Target {}";

    // Index declarations independently of build metadata and compiled outputs.
    engine
        .process_document(source.clone(), "java", use_text)
        .unwrap();
    engine
        .process_document(target.clone(), "java", target_text)
        .unwrap();
    engine
        .register_modules(
            vec![Module {
                classpaths: vec![Classpath {
                    inputs: vec![ResourceFilter::file(files.path("app/Use.java"))],
                    entries: Some(vec![ResourceFilter::file(&jar)]),
                }],
                artifact_sources: vec![],
            }],
            ImportKind::Gradle,
            &files.path("app"),
        )
        .unwrap();
    let definition = DefinitionRequest {
        source: &source,
        offset: use_text.rfind("Target").unwrap(),
    };
    let hover = HoverRequest {
        source: &source,
        contents: use_text,
        offset: definition.offset,
    };
    assert!(
        engine
            .feature_goto_definition(&definition)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        engine
            .feature_hover(&hover)
            .unwrap()
            .unwrap()
            .contents()
            .plaintext(),
        "Target"
    );
    assert_eq!(engine.diagnostics(&source).unwrap().len(), 1);

    // Publish the JAR's source representation from a separate import snapshot.
    engine
        .register_modules(
            vec![Module {
                classpaths: vec![],
                artifact_sources: vec![ArtifactSources {
                    artifacts: vec![Location::file(&jar)],
                    sources: vec![ResourceFilter::file(files.path("lib/Target.java"))],
                }],
            }],
            ImportKind::Gradle,
            &files.path("lib"),
        )
        .unwrap();
    let start = target_text.find("Target").unwrap();
    assert_eq!(
        engine
            .feature_goto_definition(&definition)
            .unwrap()
            .map(|result| result.target),
        Some(SourceSpan::new(
            target,
            ByteRange::new(start, start + "Target".len())
        ))
    );
    let response = engine.feature_hover(&hover).unwrap().unwrap();
    assert_eq!(
        response.contents().plaintext(),
        "Type: q.Target\nLocated in: lib/Target.java\nVia: lib/build/api.jar"
    );
    assert_eq!(
        response.contents().markdown(),
        Some(
            "**Type:** ` q.Target `\n\n**Located in:** ` lib/Target.java `\n\n**Via:** ` lib/build/api.jar `"
        )
    );
    assert!(engine.diagnostics(&source).unwrap().is_empty());
    assert!(!jar.exists());

    // Removing only the association hides the already indexed declaration again.
    engine
        .register_modules(vec![], ImportKind::Gradle, &files.path("lib"))
        .unwrap();
    assert!(
        engine
            .feature_goto_definition(&definition)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        engine
            .feature_hover(&hover)
            .unwrap()
            .unwrap()
            .contents()
            .plaintext(),
        "Target"
    );
    assert_eq!(engine.diagnostics(&source).unwrap().len(), 1);
}

#[test]
fn imported_type_navigation_is_unrestricted_for_file_and_virtual_documents() {
    let target = Location::file("/other/q/Target.java");
    let use_text = "package p; import q.Target; class Use { Target field; }";
    let target_text = "package q; public class Target {}";
    let name_start = target_text.find("Target").unwrap();

    for source in [
        Location::file("/src/p/Use.java"),
        Location::new(LocationEntry::Virtual("untitled:Use.java".into())),
    ] {
        let root = TempDir::new();
        let engine = Engine::new(root.root()).unwrap();
        engine
            .process_document(source.clone(), "java", use_text)
            .unwrap();
        engine
            .process_document(target.clone(), "java", target_text)
            .unwrap();

        assert_eq!(
            engine
                .feature_goto_definition(&DefinitionRequest {
                    source: &source,
                    offset: use_text.rfind("Target").unwrap(),
                })
                .unwrap()
                .map(|definition| definition.target),
            Some(SourceSpan::new(
                target.clone(),
                ByteRange::new(name_start, name_start + "Target".len()),
            ))
        );
    }
}
