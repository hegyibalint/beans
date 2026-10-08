use beans_core::{
    classpath::{Classpath, Unrestricted},
    resource::{ResourceEntry, ResourceId, ResourceRoot},
};

use crate::{Workspace, Workspaces};

fn entry(name: &str) -> ResourceId {
    ResourceId::new(ResourceRoot::Virtual {
        provider: "test".into(),
        key: name.into(),
    })
    .entry(ResourceEntry("Example.class".into()))
}

struct Visible(ResourceId);

impl Visible {
    fn new(name: &str) -> Self {
        Self(entry(name))
    }
}

impl Classpath for Visible {
    fn contains_source(&self, _source: &ResourceId) -> bool {
        false
    }

    fn contains_class(&self, source: &ResourceId) -> bool {
        source == &self.0
    }
}

impl Workspace for Visible {
    fn classpath_for(&self, _uri: &str) -> &dyn Classpath {
        self
    }
}

fn sees(workspaces: &Workspaces, uri: &str, artifact: &str) -> bool {
    workspaces
        .classpath_for(uri)
        .contains_class(&entry(artifact))
}

#[test]
fn documents_select_independent_backends_for_their_roots() {
    let mut workspaces = Workspaces::new(Visible::new("default"));
    workspaces.add(["/one".into()], Visible::new("one"));
    workspaces.add(["/two".into()], Visible::new("two"));

    assert!(sees(&workspaces, "file:///one/Example.java", "one"));
    assert!(!sees(&workspaces, "file:///one/Example.java", "two"));
    assert!(sees(&workspaces, "file:///two/Example.java", "two"));
    assert!(!sees(&workspaces, "file:///two/Example.java", "one"));
    assert!(sees(&workspaces, "file:///scratch/Example.java", "default"));
}

#[test]
fn the_deepest_root_wins_even_when_its_ancestor_was_registered_later() {
    let mut workspaces = Workspaces::default();
    workspaces.add(["/project/nested".into()], Visible::new("nested"));
    workspaces.add(["/project".into()], Visible::new("broad"));

    assert!(sees(
        &workspaces,
        "file:///project/nested/Example.java",
        "nested"
    ));
    assert!(!sees(
        &workspaces,
        "file:///project/nested/Example.java",
        "broad"
    ));
    assert!(sees(&workspaces, "file:///project/Example.java", "broad"));
}

#[test]
fn the_latest_registration_breaks_equal_root_ties() {
    let mut workspaces = Workspaces::default();
    workspaces.add(["/project".into()], Visible::new("first"));
    workspaces.add(["/project".into()], Visible::new("second"));

    assert!(sees(&workspaces, "file:///project/Example.java", "second"));
    assert!(!sees(&workspaces, "file:///project/Example.java", "first"));
}

#[test]
fn one_backend_can_claim_project_and_external_source_roots() {
    let mut workspaces = Workspaces::new(Visible::new("default"));
    workspaces.add(
        ["/project".into(), "/shared".into()],
        Visible::new("configured"),
    );

    assert!(sees(
        &workspaces,
        "file:///shared/Example.java",
        "configured"
    ));
    assert!(!sees(
        &workspaces,
        "file:///shared-other/Example.java",
        "configured"
    ));
    assert!(sees(
        &workspaces,
        "file:///shared-other/Example.java",
        "default"
    ));
}

#[test]
fn encoded_uris_select_matching_roots() {
    let mut workspaces = Workspaces::new(Visible::new("default"));
    workspaces.add(["/my project".into()], Visible::new("configured"));

    assert!(sees(
        &workspaces,
        "file:///my%20project/Example.java",
        "configured"
    ));
    assert!(!sees(
        &workspaces,
        "file:///my%20project/Example.java",
        "default"
    ));
}

#[test]
fn virtual_document_uris_never_select_filesystem_roots() {
    let mut workspaces = Workspaces::new(Visible::new("default"));
    workspaces.add(["/my project".into()], Visible::new("configured"));

    assert!(sees(
        &workspaces,
        "beans-jvm:///my%20project/Example.class",
        "default"
    ));
    assert!(!sees(
        &workspaces,
        "beans-jvm:///my%20project/Example.class",
        "configured"
    ));
}

#[test]
fn a_nested_fallback_folder_overrides_the_ancestors_configured_policy() {
    let mut workspaces = Workspaces::default();
    workspaces.add(["/project".into()], Visible::new("configured"));
    workspaces.add(["/project/scratch".into()], Unrestricted);

    assert!(sees(
        &workspaces,
        "file:///project/scratch/Example.java",
        "unrelated"
    ));
    assert!(!sees(
        &workspaces,
        "file:///project/Example.java",
        "unrelated"
    ));
}
