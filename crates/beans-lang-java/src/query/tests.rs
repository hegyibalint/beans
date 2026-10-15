use crate::{lower_into, testing::Models};
use beans_core::{
    classpath::{Classpath, Unrestricted},
    location::{Location, LocationEntry},
    names::Name,
    query_scope::QueryScope,
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
    let query = models.query(QueryScope::new(&Unrestricted));
    let decoded = query.file(&source).unwrap().unwrap();
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
    let query = models.query(QueryScope::new(&Unrestricted));
    let handles = query.find_type(&name("p.q.Outer.Member")).unwrap();
    assert_eq!(handles.len(), 4);
    assert_eq!(handles[0].source(), handles[1].source());
    assert_ne!(handles[0].node_index(), handles[1].node_index());
    assert_ne!(handles[0].source(), handles[2].source());
    for handle in handles {
        assert_eq!(
            query.declaration(&handle).unwrap().unwrap().name(),
            Some("Member")
        );
    }
    assert!(query.find_type(&name("p.q.Member")).unwrap().is_empty());
}

#[test]
fn replacements_and_deletions_remove_old_models_and_names() {
    let mut models = Models::new();
    let source = Location::file("/Example.java");
    models.store(lower_into("class First {}"), source.clone());
    assert_eq!(
        models
            .query(QueryScope::new(&Unrestricted))
            .find_type(&name("First"))
            .unwrap()
            .len(),
        1
    );
    models.store(lower_into("class Second {}"), source.clone());
    {
        let query = models.query(QueryScope::new(&Unrestricted));
        assert!(query.find_type(&name("First")).unwrap().is_empty());
        assert_eq!(query.find_type(&name("Second")).unwrap().len(), 1);
        let count: i64 = models
            .connection
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
    models.remove(source.clone());
    {
        let query = models.query(QueryScope::new(&Unrestricted));
        assert!(query.file(&source).unwrap().is_none());
        assert!(query.find_type(&name("First")).unwrap().is_empty());
        assert!(query.find_type(&name("Second")).unwrap().is_empty());
        for table in ["entries", "locations", "java_types"] {
            let count: i64 = models
                .connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0);
        }
    }
    models.store(lower_into("class Third {}"), source.clone());
    let query = models.query(QueryScope::new(&Unrestricted));
    assert_eq!(query.find_type(&name("Third")).unwrap().len(), 1);
    assert!(query.find_type(&name("First")).unwrap().is_empty());
}

#[test]
fn repeated_writes_keep_only_the_current_model() {
    let mut models = Models::new();
    let source = Location::file("/Example.java");
    for text in ["class Former {}", "class Latest {}"] {
        models.store(lower_into(text), source.clone());
    }
    let query = models.query(QueryScope::new(&Unrestricted));
    assert!(query.find_type(&name("Former")).unwrap().is_empty());
    assert_eq!(query.find_type(&name("Latest")).unwrap().len(), 1);
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
    ];
    for (index, source) in sources.iter().enumerate() {
        models.store(
            lower_into(&format!("class Type{index} {{}}")),
            source.clone(),
        );
    }
    let query = models.query(QueryScope::new(&Unrestricted));
    for (index, source) in sources.iter().enumerate() {
        let handle = query
            .find_type(&name(&format!("Type{index}")))
            .unwrap()
            .remove(0);
        assert_eq!(handle.source(), source);
        assert!(query.file(source).unwrap().is_some());
    }
}

#[test]
fn classpath_filters_discovery_but_not_direct_model_reads() {
    struct NoSources;
    impl Classpath for NoSources {
        fn contains_source(&self, _: &Location) -> bool {
            false
        }
        fn contains_class(&self, _: &Location) -> bool {
            false
        }
    }
    let mut models = Models::new();
    let source = Location::file("/Example.java");
    models.store(lower_into("class Example {}"), source.clone());
    let query = models.query(QueryScope::new(&NoSources));
    assert!(query.file(&source).unwrap().is_some());
    assert!(query.find_type(&name("Example")).unwrap().is_empty());
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
        .execute("UPDATE entries SET model = '{}' WHERE id = 2", [])
        .unwrap();
    let query = models.query(QueryScope::new(&Unrestricted));
    let handle = query.find_type(&name("Healthy")).unwrap().remove(0);
    assert_eq!(
        query.declaration(&handle).unwrap().unwrap().name(),
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
            .execute("UPDATE entries SET model = ?1", [payload])
            .unwrap();
        assert!(
            models
                .query(QueryScope::new(&Unrestricted))
                .file(&source)
                .is_err()
        );
    }
}
