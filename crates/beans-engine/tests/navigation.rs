use beans_core::{
    features::DefinitionRequest,
    location::{Location, LocationEntry},
    ranges::ByteRange,
    source::SourceSpan,
};
use beans_engine::Engine;

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
        let mut engine = Engine::new();
        engine.process_document(source.clone(), "java", use_text);
        engine.process_document(target.clone(), "java", target_text);

        assert_eq!(
            engine
                .feature_goto_definition(&DefinitionRequest {
                    source: &source,
                    offset: use_text.rfind("Target").unwrap(),
                })
                .map(|definition| definition.target),
            Some(SourceSpan::new(
                target.clone(),
                ByteRange::new(name_start, name_start + "Target".len()),
            ))
        );
    }
}
