use std::path::Path;

use crate::model::JvmSource;

#[test]
fn processing_a_standalone_class_attaches_its_path() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/class_file/tests/fixtures/classes/beans/fixture/Feature.class");

    let mut processed = crate::container::process(&path);
    let (source, class) = processed
        .next()
        .expect("a class file contributes one item")
        .expect("fixture should process");

    assert_eq!(source, JvmSource::ClassFile { path });
    assert_eq!(class.fqn.as_str(), "beans.fixture.Feature");
    assert!(processed.next().is_none());
}
