use std::path::{Path, PathBuf};

use crate::model::JvmSource;

/// A compiled tree with a package below it and a module descriptor beside it,
/// which is the shape a build tool's output directory has.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/class_file/tests/fixtures/classes")
}

fn processed() -> Vec<(JvmSource, crate::model::JvmClass)> {
    crate::container::process(&fixture())
        .map(|result| result.expect("fixture should process"))
        .collect()
}

#[test]
fn a_class_below_the_element_is_reached() {
    let names: Vec<String> = processed()
        .iter()
        .map(|(_, class)| class.fqn.to_string())
        .collect();

    assert!(names.contains(&"beans.fixture.Point".to_string()));
    assert!(names.contains(&"beans.fixture.Feature$Member".to_string()));
}

#[test]
fn a_class_found_by_walking_is_sourced_at_its_own_path() {
    let point = processed()
        .into_iter()
        .find(|(_, class)| class.fqn.as_str() == "beans.fixture.Point")
        .expect("fixture contains Point");

    assert_eq!(
        point.0,
        JvmSource::ClassFile {
            path: fixture().join("beans/fixture/Point.class"),
        }
    );
}

#[test]
fn a_module_descriptor_contributes_nothing() {
    let sources: Vec<JvmSource> = processed().into_iter().map(|(source, _)| source).collect();

    assert!(!sources.iter().any(|source| matches!(
        source,
        JvmSource::ClassFile { path } if path.ends_with("module-info.class")
    )));
    assert_eq!(sources.len(), 7);
}
