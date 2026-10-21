use beans_core::event::Command;

use crate::features::Features;

use super::params;

#[test]
fn initialization_starts_the_engine() {
    let root = tempfile::tempdir().unwrap();
    let features = Features::initialize(&params(&[root.path()])).unwrap();

    features.engine.sender().send(Command::Start).unwrap();
}

#[test]
fn initialization_sends_the_configured_home_to_the_engine() {
    let root = tempfile::tempdir().unwrap();
    let lib = root.path().join("jdk/lib");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(
        lib.join("modules"),
        beans_testing::support::class_bytes("p/Runtime"),
    )
    .unwrap();
    std::fs::write(
        root.path().join("beans.toml"),
        "jdk_home = 'jdk'\n[unit.app]\n[unit.shared]\n",
    )
    .unwrap();

    let features = Features::initialize(&params(&[root.path()])).unwrap();
    assert!(features.workspace_documents.is_empty());
    drop(features);

    let connection = beans_engine::storage::open(&root.path().join(".beans/models")).unwrap();
    let name: String = connection
        .query_row(
            "SELECT json_extract(model, '$.name') FROM models WHERE model_kind = 0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "p/Runtime");
}

#[test]
fn dropping_features_stops_the_engine() {
    let root = tempfile::tempdir().unwrap();
    let features = Features::initialize(&params(&[root.path()])).unwrap();
    let sender = features.engine.sender();

    drop(features);

    assert!(sender.send(Command::Stop).is_err());
}
