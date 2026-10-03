use std::path::{Path, PathBuf};

use crate::parser::parse;

use super::project;

#[test]
fn relative_source_and_classpath_paths_are_resolved_without_reading_inputs() {
    let project = project(
        "[unit.app]\nsources = [\"app/src\"]\nclasspath = [\"lib/dependency.jar\", \"out\"]\n",
    );
    let unit = &project.units["app"];

    assert_eq!(project.root, PathBuf::from("/project"));
    assert_eq!(unit.sources, [PathBuf::from("/project/app/src")]);
    assert_eq!(
        unit.classpath,
        [
            PathBuf::from("/project/lib/dependency.jar"),
            PathBuf::from("/project/out")
        ]
    );
}

#[test]
fn absolute_paths_are_not_prefixed_with_the_workspace_root() {
    let project = project(
        "[unit.app]\nsources = [\"/shared/src\"]\nclasspath = [\"/shared/lib.jar\"]\njdk_home = \"/opt/jdk\"\n",
    );
    let unit = &project.units["app"];

    assert_eq!(unit.sources, [PathBuf::from("/shared/src")]);
    assert_eq!(unit.classpath, [PathBuf::from("/shared/lib.jar")]);
    assert_eq!(unit.jdk_home, Some(PathBuf::from("/opt/jdk")));
}

#[test]
fn dot_components_do_not_change_resolved_identity() {
    let project = project(
        "[unit.app]\nsources = [\"./app/../src\", \".\"]\nclasspath = [\"../libs/./dependency.jar\"]\n",
    );

    assert_eq!(
        project.units["app"].sources,
        [PathBuf::from("/project/src"), PathBuf::from("/project")]
    );
    assert_eq!(
        project.units["app"].classpath,
        [PathBuf::from("/libs/dependency.jar")]
    );
}

#[test]
fn a_relative_root_produces_absolute_paths() {
    let backend = parse("[unit.app]\nsources = [\"src\"]\n", Path::new("project")).unwrap();
    let root = std::env::current_dir().unwrap().join("project");

    assert_eq!(backend.model().root, root);
    assert_eq!(backend.model().units["app"].sources, [root.join("src")]);
}

#[test]
fn units_inherit_the_default_jdk_unless_they_override_it() {
    let project = project(
        "jdk_home = \"vendor/jdk\"\n[unit.app]\n[unit.legacy]\njdk_home = \"vendor/legacy\"\n",
    );

    assert_eq!(
        project.units["app"].jdk_home,
        Some(PathBuf::from("/project/vendor/jdk"))
    );
    assert_eq!(
        project.units["legacy"].jdk_home,
        Some(PathBuf::from("/project/vendor/legacy"))
    );
}
