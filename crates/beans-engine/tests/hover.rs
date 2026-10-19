use beans_core::{
    features::HoverRequest,
    location::{Location, LocationEntry},
};
use beans_engine::Engine;

#[test]
fn java_hover_reaches_the_shared_feature_contract() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
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
