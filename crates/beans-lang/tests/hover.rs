use beans_core::model::{classpath::Unrestricted, source::Source};
use beans_lang::{HoverRequest, LanguageFeatures, Languages};

#[test]
fn java_hover_reaches_the_shared_feature_contract() {
    let mut languages = Languages::default();
    let uri = "untitled:Example.java";
    let contents = "class C extends Target {}";
    let source = Source::uri(uri);
    languages.process_document(uri, "java", contents);
    let context = languages.feature_context(&Unrestricted);
    let provider: &dyn LanguageFeatures<_> = &languages;

    let info = provider
        .hover(
            &context,
            &HoverRequest {
                source: &source,
                contents,
                offset: 16,
            },
        )
        .unwrap();

    assert_eq!(info.contents(), "Target");
    let range = info.range();
    assert_eq!(&contents[range.start()..range.end()], "Target");
    assert!(
        provider
            .hover(
                &context,
                &HoverRequest {
                    source: &Source::uri("file:///README.md"),
                    contents,
                    offset: 16,
                },
            )
            .is_none()
    );
}
