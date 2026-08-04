use std::path::Path;

use super::{is_class, read_into};

#[test]
fn a_reused_buffer_holds_only_the_file_just_read() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/class_file/tests/fixtures/classes/beans/fixture/Point.class");
    let mut buffer = vec![0xAA; 64];

    read_into(&path, &mut buffer).expect("fixture should read");

    assert_eq!(&buffer[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
    assert_eq!(buffer.len() as u64, path.metadata().unwrap().len());
}

#[test]
fn only_class_files_are_taken() {
    assert!(is_class("beans/fixture/Point.class"));
    assert!(!is_class("beans/fixture/logo.png"));
    assert!(!is_class("beans/fixture"));
}

#[test]
fn a_multi_release_overlay_is_not_taken() {
    assert!(!is_class("META-INF/versions/9/beans/fixture/Point.class"));
}

#[test]
fn an_unsupported_classpath_element_names_itself() {
    let mut processed = super::process(Path::new("/nowhere/mystery.bin"));

    let error = processed
        .next()
        .expect("an unsupported element contributes one item")
        .expect_err("it should be an error");

    assert!(error.to_string().contains("mystery.bin"));
    assert!(processed.next().is_none());
}
