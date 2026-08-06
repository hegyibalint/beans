use std::path::{Path, PathBuf};

use beans::Beans;
use beans_platform_jvm as jvm;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/engine/fixtures/classpath")
}

fn source(root: &Path, path: &str) -> jvm::model::Source {
    jvm::model::Source::SourceFile {
        path: root.join(path),
    }
}

fn has_compiled_library_scope_error(beans: &Beans, source: &jvm::model::Source) -> bool {
    beans
        .analyze(source)
        .expect("fixture source should be analyzed")
        .diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.code == "type-outside-scope"
                && diagnostic.message
                    == "type CompiledLibrary is outside the current compilation scope"
        })
}

#[test]
fn a_workspace_jar_is_visible_only_to_the_unit_whose_classpath_contains_it() {
    let root = fixture_root();
    let mut beans = Beans::new();

    assert_eq!(beans.open_workspace(&root).expect("fixture should load"), 2);

    assert!(has_compiled_library_scope_error(
        &beans,
        &source(&root, "without-classpath/WithoutClasspath.java")
    ));
    assert!(!has_compiled_library_scope_error(
        &beans,
        &source(&root, "with-classpath/WithClasspath.java")
    ));
}
