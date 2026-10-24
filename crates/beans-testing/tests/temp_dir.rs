use std::fs;

use beans_testing::TempDir;

#[test]
fn writes_create_parent_directories_and_preserve_raw_contents() {
    let files = TempDir::new();
    let text = "class Café { <cur> }";
    let source = files.write("src/demo/Example.java", text);
    let binary = files.write("jdk/lib/modules", [0xff, 0, 0x80]);

    assert_eq!(source, files.path("src/demo/Example.java"));
    assert_eq!(fs::read_to_string(source).unwrap(), text);
    assert_eq!(fs::read(binary).unwrap(), [0xff, 0, 0x80]);
}

#[test]
fn writing_an_existing_file_replaces_and_truncates_its_contents() {
    let files = TempDir::new();
    let original = files.write("Example.java", "class Former {}\n");

    let replacement = files.write("Example.java", "short");

    assert_eq!(replacement, original);
    assert_eq!(fs::read_to_string(original).unwrap(), "short");
}

#[test]
fn obtaining_a_path_does_not_create_its_file_or_parent() {
    let files = TempDir::new();
    let path = files.path("missing/Example.java");

    assert!(files.root().is_dir());
    assert_eq!(path, files.root().join("missing/Example.java"));
    assert!(!path.exists());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn fixtures_are_isolated_and_cleaned_up_on_drop() {
    let remaining = TempDir::new();
    let surviving_file = remaining.write("Example.java", "second");
    let removed_root = {
        let files = TempDir::new();
        let path = files.write("Example.java", "first");
        assert_ne!(files.root(), remaining.root());
        assert_eq!(fs::read_to_string(path).unwrap(), "first");
        assert_eq!(fs::read_to_string(&surviving_file).unwrap(), "second");
        files.root().to_path_buf()
    };

    assert!(!removed_root.exists());
    assert_eq!(fs::read_to_string(surviving_file).unwrap(), "second");
}
