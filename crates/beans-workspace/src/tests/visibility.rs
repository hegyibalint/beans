use beans_core::model::{classpath::Classpath, source::Source};

use crate::{Workspace, Workspaces};

#[test]
fn unconfigured_workspaces_supply_unrestricted_visibility_for_document_uris() {
    let workspaces = Workspaces::default();
    for uri in [
        "file:///project/Example.java",
        "beans-jvm:///library/Example.class",
    ] {
        let classpath = workspaces.classpath_for(uri);

        assert!(classpath.contains(&Source::jar_entry("library.jar", "Example.class")));
    }
}

#[test]
fn configured_workspaces_forward_the_document_uri_and_borrow_backend_visibility() {
    struct SelectedSource {
        document: String,
        visible: Source,
    }
    impl Classpath for SelectedSource {
        fn contains(&self, source: &Source) -> bool {
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
    let visible = Source::jar_entry("library.jar", "Example.class");
    let workspaces = Workspaces::new(SelectedSource {
        document: uri.into(),
        visible: visible.clone(),
    });

    let classpath = workspaces.classpath_for(uri);

    assert!(classpath.contains(&visible));
    assert!(!classpath.contains(&Source::uri("file:///unrelated/Other.java")));
}
