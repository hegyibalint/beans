use beans_core::{
    features::DefinitionRequest,
    ranges::ByteRange,
    resource::{ResourceEntry, ResourceId, ResourceRoot},
    source::SourceSpan,
};
use beans_engine::Engine;
use tempfile::tempdir;

#[test]
fn imported_type_navigation_reads_sqlite_for_file_and_virtual_documents() {
    let target = ResourceId::file("/other/q/Target.java");
    let use_text = "package p; import q.Target; class Use { Target field; }";
    let target_text = "package q; public class Target {}";
    let name_start = target_text.find("Target").unwrap();
    for source in [
        ResourceId::file("/src/p/Use.java"),
        ResourceId::new(ResourceRoot::Virtual {
            provider: "lsp".into(),
            key: "untitled:Use".into(),
        }),
    ] {
        let root = tempdir().unwrap();
        let path = root.path().join("index.sqlite");
        {
            let mut engine = Engine::new(&path).unwrap();
            engine
                .document_open(source.clone(), "java", use_text)
                .unwrap();
            engine.process(target.clone(), target_text).unwrap();
        }
        let engine = Engine::new(&path).unwrap();
        assert_eq!(
            engine
                .feature_goto_definition(&DefinitionRequest {
                    source: &source,
                    offset: use_text.rfind("Target").unwrap()
                })
                .unwrap()
                .map(|definition| definition.target),
            Some(SourceSpan::new(
                target.clone(),
                ByteRange::new(name_start, name_start + "Target".len())
            ))
        );
    }
}

#[test]
fn navigation_uses_the_latest_replacement_without_resurrecting_old_declarations() {
    let root = tempdir().unwrap();
    let mut engine = Engine::new(&root.path().join("index.sqlite")).unwrap();
    let source = ResourceId::file("/src/Use.java");
    let target = ResourceId::file("/src/Target.java");
    let contents = "import q.Target; class Use { Target field; }";
    engine
        .document_open(source.clone(), "java", contents)
        .unwrap();
    engine
        .process(target.clone(), "package q; class Target {}")
        .unwrap();
    let request = DefinitionRequest {
        source: &source,
        offset: contents.rfind("Target").unwrap(),
    };
    assert!(engine.feature_goto_definition(&request).unwrap().is_some());
    engine
        .process(target.clone(), "package q; class Other {}")
        .unwrap();
    assert!(engine.feature_goto_definition(&request).unwrap().is_none());
    engine
        .process(target, "package q; class Target {}")
        .unwrap();
    assert!(engine.feature_goto_definition(&request).unwrap().is_some());
}

#[test]
fn navigation_preserves_the_full_nested_resource_identity() {
    let root = tempdir().unwrap();
    let mut engine = Engine::new(&root.path().join("index.sqlite")).unwrap();
    let source = ResourceId::file("Use.java");
    let target = ResourceId::file("sources.zip").entry(ResourceEntry("q/Target.java".into()));
    let contents = "import q.Target; class Use { Target field; }";
    engine
        .document_open(source.clone(), "java", contents)
        .unwrap();
    engine
        .document_open(target.clone(), "java", "package q; class Target {}")
        .unwrap();
    let response = engine
        .feature_goto_definition(&DefinitionRequest {
            source: &source,
            offset: contents.rfind("Target").unwrap(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(response.target.resource, target);
}
