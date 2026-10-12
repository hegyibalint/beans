use beans_core::{
    features::HoverRequest,
    resource::{ResourceId, ResourceRoot},
};
use beans_engine::Engine;

#[test]
fn java_hover_reaches_the_shared_feature_contract() {
    let mut engine = Engine::new();
    let source = ResourceId::new(ResourceRoot::Virtual {
        provider: "test".into(),
        key: "untitled:Example.java".into(),
    });
    let contents = "class C extends Target {}";
    engine.process_document(source.clone(), "java", contents);

    let info = engine
        .feature_hover(&HoverRequest {
            source: &source,
            contents,
            offset: 16,
        })
        .unwrap();

    assert_eq!(info.contents(), "Target");
    let range = info.range();
    assert_eq!(&contents[range.start()..range.end()], "Target");
    assert!(
        engine
            .feature_hover(&HoverRequest {
                source: &ResourceId::file("/README.md"),
                contents,
                offset: 16,
            })
            .is_none()
    );
}
