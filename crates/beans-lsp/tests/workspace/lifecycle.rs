use serde_json::Value;

use super::*;

#[test]
fn initialization_queues_the_configured_home_and_exit_waits_for_publication() {
    let root = TempDir::new();
    root.write(
        "jdk/lib/modules",
        beans_testing::support::class_bytes("p/Runtime"),
    );
    root.write(
        "beans.toml",
        "jdk_home = 'jdk'\n[unit.app]\n[unit.shared]\n",
    );
    let mut client = Client::default();
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    assert!(
        request(&mut client, "shutdown", Value::Null)
            .error
            .is_none()
    );
    client.exit().unwrap();

    let connection = beans_engine::storage::open(&root.path(".beans/models")).unwrap();
    let mut statement = connection
        .prepare("SELECT json_extract(model, '$.name') FROM models WHERE kind = 0")
        .unwrap();
    let names: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(names, ["p/Runtime"]);
}
