use std::fs;

use crate::features::Features;

use super::params;

#[test]
fn overlapping_workspace_folders_are_rejected_before_creating_caches() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("nested");
    fs::create_dir(&nested).unwrap();

    let error = Features::initialize(&params(&[root.path(), &nested]))
        .err()
        .unwrap();

    assert_eq!(error, "expected exactly one workspace folder");
    assert!(!root.path().join(".beans").exists());
    assert!(!nested.join(".beans").exists());
}

#[test]
fn independent_workspace_folders_are_rejected_before_creating_caches() {
    let root = tempfile::tempdir().unwrap();
    let one = root.path().join("one");
    let two = root.path().join("two");
    fs::create_dir(&one).unwrap();
    fs::create_dir(&two).unwrap();

    let error = Features::initialize(&params(&[&one, &two])).err().unwrap();

    assert_eq!(error, "expected exactly one workspace folder");
    assert!(!one.join(".beans").exists());
    assert!(!two.join(".beans").exists());
}
