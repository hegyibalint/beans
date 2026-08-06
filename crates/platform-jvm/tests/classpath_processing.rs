use std::path::{Path, PathBuf};

use beans_core::storage::Revision;
use beans_platform_jvm as jvm;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/class_file/tests/fixtures/classes/beans/fixture")
        .join(name)
}

fn jar_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/container/archive/tests/fixtures/classes.jar")
}

#[test]
fn one_classpath_lands_at_one_revision() {
    let classpath = [fixture("Feature.class"), fixture("Point.class")];
    let viewpoint = jvm::model::Source::SourceFile {
        path: PathBuf::from("src/App.java"),
    };
    let mut revision = Revision::default();
    let before = revision;
    let imported = revision.bump();
    let mut jvm = jvm::Platform::new();

    jvm.process_classpath(&classpath, imported);

    assert!(
        jvm.query_from(&viewpoint, before)
            .classes_in_package("beans.fixture")
            .is_empty()
    );

    let classes = jvm
        .query_from(&viewpoint, imported)
        .classes_in_package("beans.fixture");
    assert_eq!(classes.len(), 2);
    for expected in classpath {
        assert!(classes.iter().any(|(source, _)| {
            matches!(source, jvm::model::Source::ClassFile { path } if path == &expected)
        }));
    }
}

#[test]
fn a_jar_streams_its_root_classes_into_the_lake() {
    let jar_path = jar_fixture();
    let viewpoint = jvm::model::Source::SourceFile {
        path: PathBuf::from("src/App.java"),
    };
    let revision = Revision::default();
    let mut jvm = jvm::Platform::new();

    jvm.process_classpath(std::slice::from_ref(&jar_path), revision);

    let classes = jvm
        .query_from(&viewpoint, revision)
        .classes_in_package("beans.fixture");
    assert_eq!(classes.len(), 2);
    assert!(classes.iter().all(|(source, _)| {
        matches!(source, jvm::model::Source::JarEntry { jar_path: source_jar, .. } if source_jar == &jar_path)
    }));
    assert!(
        classes
            .iter()
            .any(|(_, class)| class.fqn.as_str() == "beans.fixture.Feature")
    );
    assert!(
        classes
            .iter()
            .any(|(_, class)| class.fqn.as_str() == "beans.fixture.Point")
    );
}
