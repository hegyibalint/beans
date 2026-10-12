use beans_core::{
    classpath::{Classpath, Unrestricted},
    query_scope::QueryScope,
    resource::{ResourceEntry, ResourceId},
    revision::Revision,
};
use beans_storage::RevisionEntry;

use crate::{JvmPlatform, model::class::Class};

#[test]
fn lookup_requires_an_exact_internal_binary_name() {
    let mut platform = JvmPlatform::new();
    let resource = ResourceId::file("/Outer$Inner.class");
    let revision = Revision::new(1);
    platform.classes.put(
        revision,
        resource.clone(),
        Class {
            name: "example/Outer$Inner".into(),
        },
    );
    let scope = QueryScope::new(revision, &Unrestricted);

    assert_eq!(
        platform.find_classes(scope, "example/Outer$Inner"),
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
        assert!(platform.find_classes(scope, name).is_empty(), "{name}");
    }
}

#[test]
fn lookup_uses_the_requested_revision_through_replacement_and_removal() {
    let mut platform = JvmPlatform::new();
    let resource = ResourceId::file("/Example.class");
    platform.classes.put(
        Revision::new(1),
        resource.clone(),
        Class {
            name: "example/Former".into(),
        },
    );
    platform.classes.put(
        Revision::new(3),
        resource.clone(),
        Class {
            name: "example/Latest".into(),
        },
    );
    platform.classes.remove(Revision::new(5), resource.clone());

    assert!(platform.class(Revision::default(), &resource).is_none());
    assert!(
        platform
            .class(Revision::new(3), &ResourceId::file("/Missing.class"))
            .is_none()
    );
    assert!(
        platform
            .find_classes(
                QueryScope::new(Revision::default(), &Unrestricted),
                "example/Former"
            )
            .is_empty()
    );
    for (revision, name) in [(2, "example/Former"), (4, "example/Latest")] {
        let revision = Revision::new(revision);
        let addresses = platform.find_classes(QueryScope::new(revision, &Unrestricted), name);
        assert_eq!(
            addresses,
            vec![RevisionEntry {
                revision,
                key: resource.clone()
            }]
        );
        let address = &addresses[0];
        assert_eq!(
            platform.class(address.revision, &address.key).unwrap().name,
            name
        );
    }
    assert!(
        platform
            .find_classes(
                QueryScope::new(Revision::new(3), &Unrestricted),
                "example/Former"
            )
            .is_empty()
    );
    assert!(platform.class(Revision::new(5), &resource).is_none());
    for name in ["example/Former", "example/Latest"] {
        assert!(
            platform
                .find_classes(QueryScope::new(Revision::new(5), &Unrestricted), name)
                .is_empty()
        );
    }
}

#[test]
fn duplicate_binary_names_retain_their_distinct_resource_addresses() {
    let mut platform = JvmPlatform::new();
    let revision = Revision::new(1);
    let resources = [
        ResourceId::file("/first.jar").entry(ResourceEntry("example/Foo.class".into())),
        ResourceId::file("/second.jar").entry(ResourceEntry("example/Foo.class".into())),
    ];
    for resource in &resources {
        platform.classes.put(
            revision,
            resource.clone(),
            Class {
                name: "example/Foo".into(),
            },
        );
    }

    let addresses = platform.find_classes(QueryScope::new(revision, &Unrestricted), "example/Foo");

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

    let mut platform = JvmPlatform::new();
    let revision = Revision::new(1);
    let visible = ResourceId::file("/visible.class");
    let hidden = ResourceId::file("/hidden.class");
    for resource in [&visible, &hidden] {
        platform.classes.put(
            revision,
            resource.clone(),
            Class {
                name: "example/Foo".into(),
            },
        );
    }
    let classpath = SelectedClass(visible.clone());

    assert_eq!(
        platform.find_classes(QueryScope::new(revision, &classpath), "example/Foo"),
        vec![RevisionEntry {
            revision,
            key: visible
        }]
    );
}
