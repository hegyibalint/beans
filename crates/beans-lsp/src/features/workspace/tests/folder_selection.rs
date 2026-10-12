use std::fs;

use beans_core::revision::Revision;
use url::Url;

use crate::features::Features;

use super::params;

#[test]
fn overlapping_folders_index_each_source_once() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("nested");
    fs::create_dir_all(nested.join("src")).unwrap();
    fs::write(nested.join("src/Example.java"), "").unwrap();
    fs::write(nested.join("Other.java"), "").unwrap();
    fs::write(
        root.path().join("beans.toml"),
        "[unit.main]\nsources = [\"nested/src\"]\n",
    )
    .unwrap();
    let mut features = Features::default();

    features
        .initialize(&params(&[root.path(), &nested]))
        .unwrap();

    assert_eq!(features.workspace_documents.len(), 2);
    assert_eq!(features.engine.revision(), Revision::new(2));
    for path in [nested.join("src/Example.java"), nested.join("Other.java")] {
        let uri = Url::from_file_path(path).unwrap();
        assert!(features.workspace_documents.contains_key(uri.as_str()));
    }
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

    assert_eq!(features.workspace_documents.len(), 2);
    for folder in [&one, &two] {
        let uri = Url::from_file_path(folder.join("src/Example.java")).unwrap();
        assert!(features.workspace_documents.contains_key(uri.as_str()));
    }
}
