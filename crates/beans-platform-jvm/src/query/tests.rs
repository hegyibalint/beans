use beans_core::{
    classpath::{Classpath, Unrestricted},
    query_scope::QueryScope,
    resource::{ResourceEntry, ResourceId},
    revision::Revision,
};
use beans_storage::{RevisionEntry, RevisionedStorage};

use crate::{Class, find_classes, model_query::JvmModelQuery};

struct Models<'a> {
    classes: &'a RevisionedStorage<ResourceId, Class>,
    revision: Revision,
}

impl JvmModelQuery for Models<'_> {
    fn jvm_class(&self, source: &ResourceId) -> Result<Option<Class>, String> {
        Ok(self.classes.get(self.revision, source).map(|class| Class {
            name: class.name.clone(),
        }))
    }

    fn jvm_classes(&self) -> Result<Vec<(ResourceId, Class)>, String> {
        Ok(self
            .classes
            .iter(self.revision)
            .map(|(source, class)| {
                (
                    source.clone(),
                    Class {
                        name: class.name.clone(),
                    },
                )
            })
            .collect())
    }
}

#[test]
fn lookup_requires_an_exact_internal_binary_name() {
    let mut classes = RevisionedStorage::default();
    let resource = ResourceId::file("/Outer$Inner.class");
    let revision = Revision::new(1);
    classes.put(
        revision,
        resource.clone(),
        Class {
            name: "example/Outer$Inner".into(),
        },
    );
    let models = Models {
        classes: &classes,
        revision,
    };
    let scope = QueryScope::new(revision, &Unrestricted);

    assert_eq!(
        find_classes(&models, scope, "example/Outer$Inner").unwrap(),
        vec![RevisionEntry {
            revision,
            key: resource
        }]
    );
    for name in [
        "example.Outer$Inner",
        "example.Outer.Inner",
        "example/Outer/Inner",
        "example/outer$Inner",
        "example/Missing",
        "",
    ] {
        assert!(
            find_classes(&models, scope, name).unwrap().is_empty(),
            "{name}"
        );
    }
}

#[test]
fn lookup_uses_provider_selected_models_after_replacement_and_removal() {
    let mut classes = RevisionedStorage::default();
    let resource = ResourceId::file("/Example.class");
    classes.put(
        Revision::new(1),
        resource.clone(),
        Class {
            name: "example/Former".into(),
        },
    );
    classes.put(
        Revision::new(3),
        resource.clone(),
        Class {
            name: "example/Latest".into(),
        },
    );
    classes.remove(Revision::new(5), resource.clone());
    let query_revision = Revision::new(42);
    let scope = QueryScope::new(query_revision, &Unrestricted);

    for (selected_revision, expected) in [
        (0, None),
        (2, Some("example/Former")),
        (4, Some("example/Latest")),
        (5, None),
    ] {
        let models = Models {
            classes: &classes,
            revision: Revision::new(selected_revision),
        };
        assert_eq!(
            models
                .jvm_class(&resource)
                .unwrap()
                .map(|class| class.name)
                .as_deref(),
            expected
        );
        assert!(
            models
                .jvm_class(&ResourceId::file("/Missing.class"))
                .unwrap()
                .is_none()
        );
        for name in ["example/Former", "example/Latest"] {
            let addresses = find_classes(&models, scope, name).unwrap();
            if expected == Some(name) {
                assert_eq!(
                    addresses,
                    vec![RevisionEntry {
                        revision: query_revision,
                        key: resource.clone()
                    }]
                );
            } else {
                assert!(addresses.is_empty());
            }
        }
    }
}

#[test]
fn duplicate_binary_names_retain_their_distinct_resource_addresses() {
    let mut classes = RevisionedStorage::default();
    let revision = Revision::new(1);
    let resources = [
        ResourceId::file("/first.jar").entry(ResourceEntry("example/Foo.class".into())),
        ResourceId::file("/second.jar").entry(ResourceEntry("example/Foo.class".into())),
    ];
    for resource in &resources {
        classes.put(
            revision,
            resource.clone(),
            Class {
                name: "example/Foo".into(),
            },
        );
    }
    let models = Models {
        classes: &classes,
        revision,
    };
    let addresses = find_classes(
        &models,
        QueryScope::new(revision, &Unrestricted),
        "example/Foo",
    )
    .unwrap();

    assert_eq!(addresses.len(), 2);
    for resource in resources {
        assert!(addresses.contains(&RevisionEntry {
            revision,
            key: resource
        }));
    }
}

#[test]
fn lookup_applies_class_visibility_not_source_visibility() {
    struct SelectedClass(ResourceId);

    impl Classpath for SelectedClass {
        fn contains_source(&self, _resource: &ResourceId) -> bool {
            false
        }
        fn contains_class(&self, resource: &ResourceId) -> bool {
            resource == &self.0
        }
    }

    let mut classes = RevisionedStorage::default();
    let revision = Revision::new(1);
    let visible = ResourceId::file("/visible.class");
    let hidden = ResourceId::file("/hidden.class");
    for resource in [&visible, &hidden] {
        classes.put(
            revision,
            resource.clone(),
            Class {
                name: "example/Foo".into(),
            },
        );
    }
    let models = Models {
        classes: &classes,
        revision,
    };
    let classpath = SelectedClass(visible.clone());

    assert_eq!(
        find_classes(
            &models,
            QueryScope::new(revision, &classpath),
            "example/Foo"
        )
        .unwrap(),
        vec![RevisionEntry {
            revision,
            key: visible
        }]
    );
}
