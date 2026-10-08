use std::fs;

use beans_core::origin::Origin;
use url::Url;

use crate::features::Features;

use super::params;

#[test]
fn an_explicit_fallback_folder_supersedes_an_ancestors_declared_sources() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("nested");
    fs::create_dir_all(nested.join("src")).unwrap();
    fs::write(nested.join("src/Example.java"), "").unwrap();
    fs::write(
        root.path().join("beans.toml"),
        "[unit.main]\nsources = [\"nested/src\"]\n",
    )
    .unwrap();
    let mut features = Features::default();

    features
        .initialize(&params(&[root.path(), &nested]))
        .unwrap();

    let uri = Url::from_file_path(nested.join("src/Example.java")).unwrap();
    assert!(
        features
            .workspaces
            .classpath_for(uri.as_str())
            .contains_class(&Origin::jar_entry("/unrelated.jar", "Example.class"))
    );
}

#[test]
fn unit_ids_are_local_to_each_imported_folder() {
    let root = tempfile::tempdir().unwrap();
    let one = root.path().join("one");
    let two = root.path().join("two");
    for folder in [&one, &two] {
        fs::create_dir_all(folder.join("src")).unwrap();
        fs::write(folder.join("src/Example.java"), "").unwrap();
        fs::write(
            folder.join("beans.toml"),
            "[unit.main]\nsources = [\"src\"]\n",
        )
        .unwrap();
    }
    let mut features = Features::default();

    features.initialize(&params(&[&one, &two])).unwrap();
    let one_uri = Url::from_file_path(one.join("src/Example.java")).unwrap();
    let two_uri = Url::from_file_path(two.join("src/Example.java")).unwrap();
    let classpath = features.workspaces.classpath_for(one_uri.as_str());
    assert!(classpath.contains_source(&Origin::uri(one_uri.as_str())));
    assert!(!classpath.contains_source(&Origin::uri(two_uri.as_str())));
}
