use std::path::{Path, PathBuf};

use crate::parser::parse;

use super::project;

#[test]
fn relative_sources_and_the_project_jdk_are_resolved_without_reading_inputs() {
    let project = project("jdk_home = \"vendor/jdk\"\n[module.app]\nsources = [\"app/src\"]\n");

    assert_eq!(project.root, PathBuf::from("/project"));
    assert_eq!(
        project.modules["app"].sources,
        [PathBuf::from("/project/app/src")]
    );
    assert_eq!(project.jdk_home, Some(PathBuf::from("/project/vendor/jdk")));
}

#[test]
fn absolute_paths_are_not_prefixed_with_the_workspace_root() {
    let project = project("jdk_home = \"/opt/jdk\"\n[module.app]\nsources = [\"/shared/src\"]\n");

    assert_eq!(
        project.modules["app"].sources,
        [PathBuf::from("/shared/src")]
    );
    assert_eq!(project.jdk_home, Some(PathBuf::from("/opt/jdk")));
}

#[test]
fn dot_components_do_not_change_resolved_identity() {
    let project = project(
        "jdk_home = \"../vendor/./jdk\"\n[module.app]\nsources = [\"./app/../src\", \".\"]\n",
    );

    assert_eq!(
        project.modules["app"].sources,
        [PathBuf::from("/project/src"), PathBuf::from("/project")]
    );
    assert_eq!(project.jdk_home, Some(PathBuf::from("/vendor/jdk")));
}

#[test]
fn a_relative_root_produces_absolute_paths() {
    let backend = parse("[module.app]\nsources = [\"src\"]\n", Path::new("project")).unwrap();
    let root = std::env::current_dir().unwrap().join("project");

    assert_eq!(backend.model().root, root);
    assert_eq!(backend.model().modules["app"].sources, [root.join("src")]);
}

#[test]
fn the_optional_jdk_belongs_to_the_project_not_its_modules() {
    let project = project("jdk_home = \"vendor/jdk\"\n[module.app]\n[module.test]\n");

    assert_eq!(project.jdk_home, Some(PathBuf::from("/project/vendor/jdk")));
    assert_eq!(project.modules.len(), 2);
}
