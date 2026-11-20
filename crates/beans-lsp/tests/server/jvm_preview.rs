use beans_core::location::Location;
use beans_engine::storage as engine_storage;
use beans_platform_jvm::storage as jvm_storage;
use beans_testing::support::class_model;
use serde_json::json;

use super::*;

const CONTENT: &str = "workspace/textDocumentContent";

#[test]
fn previews_stored_class_models_without_reading_artifacts() {
    let (root, mut client) = server();
    let params = json!({"uri": "beans+jvm:/java/lang/String.class"});
    let error = request(&mut client, CONTENT, params.clone()).error.unwrap();
    assert_eq!(error.code, ErrorCode::RequestFailed as i32);
    assert!(error.message.contains("is not indexed"));

    let mut connection = engine_storage::open(&root.path(".beans/models")).unwrap();
    let transaction = connection.transaction().unwrap();
    for name in ["java/lang/String", "example/Outer$Étage"] {
        let location = Location::file(root.path(format!("{name}.class")));
        jvm_storage::store(&class_model(name), &location, &transaction).unwrap();
    }
    transaction.commit().unwrap();

    assert_eq!(
        request(&mut client, CONTENT, params).result,
        Some(json!({"text": "package java.lang;\n\npublic class String {\n}\n"})),
    );
    assert_eq!(
        request(
            &mut client,
            CONTENT,
            json!({"uri": "beans+jvm:/example/Outer%24%C3%89tage.class"}),
        )
        .result,
        Some(json!({"text": "package example;\n\npublic class Outer$Étage {\n}\n"})),
    );
}

#[test]
fn document_content_requires_a_jvm_class_uri() {
    let (_root, mut client) = server();
    for params in [
        Value::Null,
        json!({"uri": 12}),
        json!({"uri": "file:///java/lang/String.class"}),
        json!({"uri": "beans+jvm:java/lang/String.class"}),
        json!({"uri": "beans+jvm://artifact/java/lang/String.class"}),
        json!({"uri": "beans+jvm:/java/lang/String.class?snapshot=1"}),
        json!({"uri": "beans+jvm:/java/lang/String.class#fragment"}),
        json!({"uri": "beans+jvm:/java/lang/String.java"}),
        json!({"uri": "beans+jvm:/"}),
        json!({"uri": "beans+jvm:/java/lang/.class"}),
        json!({"uri": "beans+jvm:/java//lang/String.class"}),
        json!({"uri": "beans+jvm:/java%2Flang/String.class"}),
        json!({"uri": "beans+jvm:/java/lang/%FF.class"}),
    ] {
        assert_error(
            request(&mut client, CONTENT, params),
            ErrorCode::InvalidParams,
        );
    }
}
