use beans_core::{
    classpath::{Classpath, Unrestricted},
    origin::Origin,
    ranges::ByteRange,
    source::SourceSpan,
};
use beans_lang::{DefinitionProvider, DefinitionRequest, Languages};

#[test]
fn imported_type_navigation_obeys_supplied_visibility() {
    struct NoSources;
    impl Classpath for NoSources {
        fn contains(&self, _source: &Origin) -> bool {
            false
        }
    }

    let mut languages = Languages::default();
    let use_uri = "file:///src/p/Use.java";
    let target_uri = "file:///other/q/Target.java";
    let use_text = "package p; import q.Target; class Use { Target field; }";
    let target_text = "package q; public class Target {}";
    languages.process_document(use_uri, "java", use_text);
    languages.process_document(target_uri, "java", target_text);
    let name_start = target_text.find("Target").unwrap();
    let source = Origin::uri(use_uri);
    let request = DefinitionRequest {
        source: &source,
        offset: use_text.rfind("Target").unwrap(),
    };

    assert_eq!(
        languages
            .goto_definition(&languages.feature_context(&Unrestricted), &request)
            .map(|definition| definition.target),
        Some(SourceSpan::new(
            Origin::uri(target_uri),
            ByteRange::new(name_start, name_start + "Target".len()),
        ))
    );
    assert!(
        languages
            .goto_definition(&languages.feature_context(&NoSources), &request)
            .is_none()
    );
}
