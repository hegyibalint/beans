use beans_core::{
    classpath::Classpath,
    location::{Location, LocationEntry},
};

use crate::{Workspace, Workspaces};

fn library_entry() -> Location {
    Location::new(LocationEntry::File("/library.jar".into()))
        .entry(LocationEntry::Container("Example.class".into()))
}

#[test]
fn unconfigured_workspaces_supply_unrestricted_visibility_for_document_uris() {
    let workspaces = Workspaces::default();
    for uri in [
        "file:///project/Example.java",
        "beans-jvm:///library/Example.class",
    ] {
        let classpath = workspaces.classpath_for(uri);

        assert!(classpath.contains_source(&library_entry()));
        assert!(classpath.contains_class(&library_entry()));
    }
}

#[test]
fn configured_workspaces_forward_the_document_uri_and_borrow_backend_visibility() {
    struct SelectedSource {
        document: String,
        visible: Location,
    }
    impl Classpath for SelectedSource {
        fn contains_source(&self, _source: &Location) -> bool {
            false
        }

        fn contains_class(&self, source: &Location) -> bool {
            source == &self.visible
        }
    }
    impl Workspace for SelectedSource {
        fn classpath_for(&self, uri: &str) -> &dyn Classpath {
            assert_eq!(uri, self.document);
            self
        }
    }
    let uri = "beans-jvm:///library/Example.class";
    let visible = library_entry();
    let workspaces = Workspaces::new(SelectedSource {
        document: uri.into(),
        visible: visible.clone(),
    });

    let classpath = workspaces.classpath_for(uri);

    assert!(!classpath.contains_source(&visible));
    assert!(classpath.contains_class(&visible));
    assert!(
        !classpath.contains_class(&Location::new(LocationEntry::File(
            "/unrelated/Other.java".into()
        )))
    );
}
