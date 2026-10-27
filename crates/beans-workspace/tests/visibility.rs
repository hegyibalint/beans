use beans_core::{
    location::{Location, LocationEntry},
    query_scope::QueryScope,
    resource_filter::ResourceFilter,
};

use beans_workspace::{Workspace, Workspaces};

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
        visible: ResourceFilter,
    }
    impl Workspace for SelectedSource {
        fn classpath_for(&self, uri: &str) -> QueryScope<'_> {
            assert_eq!(uri, self.document);
            QueryScope::new(&[], std::slice::from_ref(&self.visible))
        }
    }
    let uri = "beans-jvm:///library/Example.class";
    let visible = library_entry();
    let workspaces = Workspaces::new(SelectedSource {
        document: uri.into(),
        visible: ResourceFilter::file("/library.jar"),
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
