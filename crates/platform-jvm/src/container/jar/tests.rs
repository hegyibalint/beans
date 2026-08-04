use std::path::{Path, PathBuf};

use crate::model::JvmSource;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/container/jar/tests/fixtures/classes.jar")
}

#[test]
fn streams_root_class_entries_and_continues_after_a_bad_one() {
    let path = fixture();
    let mut classes = super::process(&path).expect("fixture should open");

    let (feature_source, feature) = classes
        .next()
        .expect("fixture contains Feature")
        .expect("Feature should parse");
    let error = classes
        .next()
        .expect("fixture contains a broken class")
        .expect_err("broken class should fail independently");
    let (point_source, point) = classes
        .next()
        .expect("fixture contains Point")
        .expect("Point should parse");

    assert_eq!(feature.fqn.as_str(), "beans.fixture.Feature");
    assert_eq!(point.fqn.as_str(), "beans.fixture.Point");
    assert_eq!(
        feature_source,
        JvmSource::JarEntry {
            jar_path: path.clone(),
            entry_path: "beans/fixture/Feature.class".to_string(),
        }
    );
    assert_eq!(
        point_source,
        JvmSource::JarEntry {
            jar_path: path,
            entry_path: "beans/fixture/Point.class".to_string(),
        }
    );
    assert!(error.to_string().contains("beans/fixture/Broken.class"));
    assert!(classes.next().is_none());
}
