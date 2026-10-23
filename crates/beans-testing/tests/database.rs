use beans_testing::database;

#[test]
fn databases_support_both_model_kinds_and_cascade_java_type_removal() {
    let connection = database();
    connection
        .execute_batch(
            r#"
            INSERT INTO models (id, location, model_kind, model, content)
            VALUES (1, '[]', 1, '{}', X''), (2, '[]', 0, '{"name":"C"}', NULL);
            INSERT INTO java_types (model_id, node_index, name) VALUES (1, 0, 'C');
            "#,
        )
        .unwrap();

    let name: String = connection
        .query_row(
            "SELECT name FROM java_types WHERE model_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "C");

    connection
        .execute("DELETE FROM models WHERE id = 1", [])
        .unwrap();

    let types: i64 = connection
        .query_row("SELECT COUNT(*) FROM java_types", [], |row| row.get(0))
        .unwrap();
    assert_eq!(types, 0);
    let name: String = connection
        .query_row(
            "SELECT json_extract(model, '$.name') FROM models",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "C");
}

#[test]
fn reinitializing_a_current_database_preserves_models_and_type_indexes() {
    let mut connection = database();
    connection
        .execute_batch(
            "INSERT INTO models (id, location, model_kind, model, content)
             VALUES (1, '[]', 1, '{}', X'');
             INSERT INTO java_types (model_id, node_index, name) VALUES (1, 0, 'C');",
        )
        .unwrap();

    beans_engine::storage::initialize(&mut connection).unwrap();

    let name: String = connection
        .query_row(
            "SELECT name FROM java_types JOIN models ON model_id = id",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "C");
}
