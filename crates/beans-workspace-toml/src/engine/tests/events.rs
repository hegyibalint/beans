use std::path::{Path, PathBuf};

use beans_core::events::Event;
use crossbeam_channel::unbounded;

use crate::parse;

#[test]
fn startup_reports_each_resolved_home_once_in_path_order() {
    let homes = discovered_homes(
        r#"
        jdk_home = "jdk"
        [unit.app]
        [unit.shared]
        jdk_home = "other/../jdk"
        [unit.legacy]
        jdk_home = "legacy"
        "#,
    );

    assert_eq!(
        homes,
        [
            PathBuf::from("/project/jdk"),
            PathBuf::from("/project/legacy")
        ]
    );
}

#[test]
fn a_workspace_without_configured_homes_emits_no_events() {
    assert!(discovered_homes("[unit.app]\nclasspath = ['lib.jar']\n").is_empty());
}

#[test]
fn discovery_does_not_require_a_jdk_layout_to_exist() {
    assert_eq!(
        discovered_homes("[unit.app]\njdk_home = 'missing'\n"),
        [PathBuf::from("/project/missing")]
    );
}

#[test]
fn a_disconnected_engine_is_reported() {
    let workspace = parse("[unit.app]\njdk_home = 'jdk'\n", Path::new("/project")).unwrap();
    let (sender, events) = unbounded();
    drop(events);

    assert!(workspace.start(&sender).is_err());
}

fn discovered_homes(contents: &str) -> Vec<PathBuf> {
    let workspace = parse(contents, Path::new("/project")).unwrap();
    let (sender, events) = unbounded();
    workspace.start(&sender).unwrap();
    events
        .try_iter()
        .map(|event| {
            let Event::JavaHomeFound(found) = event else {
                panic!("expected JavaHomeFound");
            };
            found.home
        })
        .collect()
}
