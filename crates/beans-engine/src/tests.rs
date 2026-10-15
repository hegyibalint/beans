use beans_core::resource::ResourceId;

use super::Engine;

#[test]
fn startup_propagates_storage_errors_without_creating_parent_directories() {
    let root = tempfile::tempdir().unwrap();
    assert!(Engine::new(&root.path().join("missing/index.sqlite")).is_err());
    assert!(!root.path().join("missing").exists());
    assert!(Engine::new(std::path::Path::new(":memory:")).is_err());
}

#[test]
fn feature_calls_preserve_database_and_decode_errors() {
    use beans_core::features::{DefinitionRequest, HoverRequest};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let mut engine = Engine::new(&path).unwrap();
    let source = ResourceId::file("Use.java");
    let target = ResourceId::file("Target.java");
    let text = "import q.Target; class Use { Target field; }";
    engine.document_open(source.clone(), "java", text).unwrap();
    let writer = crate::storage::open(&path).unwrap();
    writer
        .execute(
            "INSERT INTO entries VALUES (?1, 2, 1, ?2, ?3)",
            rusqlite::params![
                target.to_url().unwrap(),
                [2u8].as_slice(),
                b"package q; class Target {}".as_slice()
            ],
        )
        .unwrap();
    assert_eq!(
        engine
            .feature_goto_definition(&DefinitionRequest {
                source: &source,
                offset: text.rfind("Target").unwrap()
            })
            .err()
            .unwrap(),
        "unsupported Java model version: 2"
    );
    assert_eq!(
        engine
            .feature_hover(&HoverRequest {
                source: &target,
                contents: "package q; class Target {}",
                offset: 0
            })
            .err()
            .unwrap(),
        "unsupported Java model version: 2"
    );
    writer.execute("DROP TABLE entries", []).unwrap();
    assert!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents: text,
                offset: text.rfind("Target").unwrap()
            })
            .is_err()
    );
    assert!(
        engine
            .feature_goto_definition(&DefinitionRequest {
                source: &source,
                offset: text.rfind("Target").unwrap()
            })
            .is_err()
    );
}
