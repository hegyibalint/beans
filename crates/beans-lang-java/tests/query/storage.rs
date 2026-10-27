use beans_lang_java::{lower_into, query};

use super::support::Models;
use beans_core::{
    location::{Location, LocationEntry},
    names::Name,
};

fn name(text: &str) -> Name {
    text.split('.').map(str::to_owned).collect()
}

#[test]
fn decoded_models_preserve_the_lowered_tree_and_ranges() {
    let mut models = Models::new();
    let source = Location::file("/Example.java");
    let file = lower_into(
        "package p; import java.util.List; class Café<T> { List<? extends T[]> value; class Nested {} }",
    );
    let expected = serde_json::to_value(&file).unwrap();
    models.store(file, source.clone());
    let decoded = query::file(&source, &models.connection).unwrap().unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
}

#[test]
fn indexed_lookup_returns_handles_for_nested_and_duplicate_declarations() {
    let mut models = Models::new();
    for source in ["/one.java", "/two.java"] {
        models.store(
            lower_into("package p.q; class Outer { class Member {} class Member {} }"),
            Location::file(source),
        );
    }
    let handles = query::find_type(&name("p.q.Outer.Member"), &models.connection).unwrap();
    assert_eq!(handles.len(), 4);
    assert_eq!(handles[0].source(), handles[1].source());
    assert_ne!(handles[0].node_index(), handles[1].node_index());
    assert_ne!(handles[0].source(), handles[2].source());
    for handle in handles {
        assert_eq!(
            query::declaration(&handle, &models.connection)
                .unwrap()
                .unwrap()
                .name(),
            Some("Member")
        );
    }
    assert!(
        query::find_type(&name("p.q.Member"), &models.connection)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn replacements_and_deletions_remove_old_models_and_names() {
    let mut models = Models::new();
    let source = Location::file("/Example.java");
    models.store(lower_into("class First {}"), source.clone());
    assert_eq!(
        query::find_type(&name("First"), &models.connection)
            .unwrap()
            .len(),
        1
    );
    models.store(lower_into("class Second {}"), source.clone());
    assert!(
        query::find_type(&name("First"), &models.connection)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        query::find_type(&name("Second"), &models.connection)
            .unwrap()
            .len(),
        1
    );
    let count: i64 = models
        .connection
        .query_row("SELECT COUNT(*) FROM models", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    models.remove(source.clone());
    assert!(query::file(&source, &models.connection).unwrap().is_none());
    assert!(
        query::find_type(&name("First"), &models.connection)
            .unwrap()
            .is_empty()
    );
    assert!(
        query::find_type(&name("Second"), &models.connection)
            .unwrap()
            .is_empty()
    );
    for table in ["models", "java_types"] {
        let count: i64 = models
            .connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
    models.store(lower_into("class Third {}"), source.clone());
    assert_eq!(
        query::find_type(&name("Third"), &models.connection)
            .unwrap()
            .len(),
        1
    );
    assert!(
        query::find_type(&name("First"), &models.connection)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn locations_keep_component_kinds_boundaries_and_verbatim_values() {
    let mut models = Models::new();
    let sources = [
        Location::file("UNTITLED:My%20Type.java"),
        Location::new(LocationEntry::Virtual("UNTITLED:My%20Type.java".into())),
        Location::file("/sources.zip").entry(LocationEntry::Container("a/b.java".into())),
        Location::file("/sources.zip")
            .entry(LocationEntry::Container("a".into()))
            .entry(LocationEntry::Container("b.java".into())),
        Location { entries: vec![] },
        Location::new(LocationEntry::Virtual("untitled:😀\"\\\n.java".into())),
        Location::new(LocationEntry::Container("root".into()))
            .entry(LocationEntry::File("/Example.java".into())),
    ];
    for (index, source) in sources.iter().enumerate() {
        models.store(
            lower_into(&format!("class Type{index} {{}}")),
            source.clone(),
        );
    }
    for (index, source) in sources.iter().enumerate() {
        let handle = query::find_type(&name(&format!("Type{index}")), &models.connection)
            .unwrap()
            .remove(0);
        assert_eq!(handle.source(), source);
        assert!(query::file(source, &models.connection).unwrap().is_some());
    }
}

#[test]
fn invalid_locations_return_errors_during_discovery() {
    let mut models = Models::new();
    models.store(
        lower_into("class Example {}"),
        Location::file("/Example.java"),
    );
    for payload in [
        "{}",
        "null",
        r#"[{"Unknown":"value"}]"#,
        r#"[{}]"#,
        r#"[{"File":1}]"#,
    ] {
        models
            .connection
            .execute("UPDATE models SET location = ?1", [payload])
            .unwrap();
        assert!(query::find_type(&name("Example"), &models.connection).is_err());
    }
}

#[test]
fn source_discovery_returns_all_matching_locations() {
    let mut models = Models::new();
    for root in ["/one", "/two", "/hidden"] {
        models.store(
            lower_into("class Example {}"),
            Location::file(format!("{root}/Example.java")),
        );
    }
    let locations: Vec<_> = query::find_type(&name("Example"), &models.connection)
        .unwrap()
        .into_iter()
        .map(|handle| handle.source().clone())
        .collect();
    assert_eq!(
        locations,
        [
            Location::file("/one/Example.java"),
            Location::file("/two/Example.java"),
            Location::file("/hidden/Example.java"),
        ]
    );
}

#[test]
fn indexed_lookup_does_not_decode_unrelated_models() {
    let mut models = Models::new();
    models.store(
        lower_into("class Healthy {}"),
        Location::file("/Healthy.java"),
    );
    models.store(
        lower_into("class Broken {}"),
        Location::file("/Broken.java"),
    );
    models
        .connection
        .execute(
            "UPDATE models SET model = '{}', location = '{}' WHERE id = 2",
            [],
        )
        .unwrap();
    let handle = query::find_type(&name("Healthy"), &models.connection)
        .unwrap()
        .remove(0);
    assert_eq!(
        query::declaration(&handle, &models.connection)
            .unwrap()
            .unwrap()
            .name(),
        Some("Healthy")
    );
}

#[test]
fn invalid_models_return_errors_instead_of_panicking_during_traversal() {
    let mut models = Models::new();
    let source = Location::file("/Example.java");
    models.store(lower_into("class Example {}"), source.clone());
    for payload in [
        "{}",
        "{\"package_name\":{\"components\":[]},\"imports\":[],\"nodes\":[]}",
    ] {
        models
            .connection
            .execute("UPDATE models SET model = ?1", [payload])
            .unwrap();
        assert!(query::file(&source, &models.connection).is_err());
    }
}
