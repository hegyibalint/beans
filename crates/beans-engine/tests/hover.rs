use beans_core::{
    features::HoverRequest,
    resource::{ResourceId, ResourceRoot},
    revision::Revision,
};
use beans_engine::Engine;
use tempfile::tempdir;

#[test]
fn java_hover_reaches_the_shared_feature_contract_through_sqlite() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let source = ResourceId::new(ResourceRoot::Virtual {
        provider: "lsp".into(),
        key: "untitled:Example".into(),
    });
    let contents = "class Café /*😀*/ extends Box<Café> {}";
    let mut engine = Engine::new(&path).unwrap();
    assert_eq!(
        engine
            .document_open(source.clone(), "java", contents)
            .unwrap(),
        Some(Revision::new(1))
    );
    let request = HoverRequest {
        source: &source,
        contents,
        offset: contents.find("Box").unwrap(),
    };
    let response = engine.feature_hover(&request).unwrap().unwrap();
    assert_eq!(response.contents(), "Box<Café>");
    assert_eq!(
        &contents[response.range().start()..response.range().end()],
        "Box<Café>"
    );
    drop(engine);
    let reopened = Engine::new(&path).unwrap();
    assert_eq!(
        reopened
            .feature_hover(&request)
            .unwrap()
            .unwrap()
            .contents(),
        "Box<Café>"
    );
}

#[test]
fn edits_publish_matching_models_and_content_in_new_revisions() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let source = ResourceId::file("Example.java");
    let mut engine = Engine::new(&path).unwrap();
    let old = "class C extends Former {}";
    let new = "class C extends Latest {}";
    engine.document_open(source.clone(), "java", old).unwrap();
    assert_eq!(
        engine.document_edit(source.clone(), "java", new).unwrap(),
        Some(Revision::new(2))
    );
    assert_eq!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents: new,
                offset: new.find("Latest").unwrap()
            })
            .unwrap()
            .unwrap()
            .contents(),
        "Latest"
    );
    assert_eq!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents: old,
                offset: old.find("Former").unwrap()
            })
            .err()
            .unwrap(),
        "hover contents do not match the stored model"
    );
    drop(engine);
    let mut reopened = Engine::new(&path).unwrap();
    assert_eq!(
        reopened.document_edit(source, "java", new).unwrap(),
        Some(Revision::new(3))
    );
}

#[test]
fn missing_models_and_non_type_positions_have_no_hover() {
    let root = tempdir().unwrap();
    let mut engine = Engine::new(&root.path().join("index.sqlite")).unwrap();
    let source = ResourceId::file("Example.java");
    let contents = "class C extends Target {}";
    assert!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: 16
            })
            .unwrap()
            .is_none()
    );
    engine
        .document_open(source.clone(), "java", contents)
        .unwrap();
    assert!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: 0
            })
            .unwrap()
            .is_none()
    );
}

#[test]
fn incomplete_java_and_explicit_language_selection_survive_publication() {
    let root = tempdir().unwrap();
    let mut engine = Engine::new(&root.path().join("index.sqlite")).unwrap();
    let source = ResourceId::file("scratch.txt");
    let contents = "class C extends Target {";
    assert!(
        engine
            .document_open(source.clone(), "text", contents)
            .unwrap()
            .is_none()
    );
    assert!(engine.process(source.clone(), contents).unwrap().is_none());
    engine
        .document_open(source.clone(), "java", contents)
        .unwrap();
    assert_eq!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: contents.find("Target").unwrap()
            })
            .unwrap()
            .unwrap()
            .contents(),
        "Target"
    );
}
