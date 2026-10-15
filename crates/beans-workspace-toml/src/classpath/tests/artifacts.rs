use std::path::Path;

use beans_core::resource::{ResourceEntry, ResourceId};
use beans_workspace::Workspace;

use crate::parse;

#[test]
fn all_modules_share_the_projects_sdk_visibility() {
    let workspace = parse("jdk_home = \"/jdk\"\n[module.app]\nsources = [\"app\"]\n[module.lib]\nsources = [\"lib\"]\n", Path::new("/project")).unwrap();

    for uri in [
        "file:///project/app/Use.java",
        "file:///project/lib/Use.java",
    ] {
        let classpath = workspace.classpath_for(uri);
        for path in [
            "/jdk/lib/modules",
            "/jdk/jre/lib/rt.jar",
            "/jdk/lib/rt.jar",
            "/jdk/lib/tools.jar",
            "/jdk/classes/Object.class",
        ] {
            let resource =
                ResourceId::file(path).entry(ResourceEntry("java/lang/Object.class".into()));
            assert!(classpath.contains_class(&resource));
        }
        for path in [
            "/other-jdk/lib/modules",
            "/jdk-other/lib/modules",
            "/jdk/../other/lib/modules",
            "jdk/lib/modules",
            "/project/lib.jar",
        ] {
            assert!(!classpath.contains_class(&ResourceId::file(path)));
        }
    }
}

#[test]
fn an_unconfigured_sdk_admits_no_binary_resources() {
    let workspace = parse("[module.app]\nsources = [\"src\"]\n", Path::new("/project")).unwrap();
    let classpath = workspace.classpath_for("file:///project/src/Use.java");

    assert!(classpath.contains_source(&ResourceId::file("/project/src/Target.java")));
    assert!(!classpath.contains_class(&ResourceId::file("/jdk/lib/modules")));
    assert!(!classpath.contains_class(&ResourceId::file("/project/src/Target.class")));
}

#[test]
fn source_visibility_does_not_admit_nested_resources() {
    let workspace = parse(
        "jdk_home = \"/jdk\"\n[module.app]\nsources = [\"src\"]\n",
        Path::new("/project"),
    )
    .unwrap();
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
    let resource =
        ResourceId::file("/project/src/lib.jar").entry(ResourceEntry("p/Target.java".into()));

    assert!(!classpath.contains_source(&resource));
    assert!(!classpath.contains_class(&resource));
}
