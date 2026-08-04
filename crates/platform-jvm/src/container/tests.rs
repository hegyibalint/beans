use std::path::Path;

use super::is_class;

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
