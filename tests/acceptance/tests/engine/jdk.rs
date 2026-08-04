//! A `jdk_home` is its own descriptor setting rather than a classpath entry,
//! and the whole runtime it names lands in that unit's scope.
//!
//! The descriptor is written here rather than committed, because a JDK path is
//! whatever the machine happens to have and `beans_test_support::jdk` is what
//! knows it.

use std::path::{Path, PathBuf};

use beans::Beans;
use beans_platform_jvm::model::JvmSource;

/// Two units, the same Java in both, and only one of them naming a JDK.
fn project() -> PathBuf {
    let root = std::env::temp_dir().join(format!("beans-jdk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);

    write(
        &root.join("beans.toml"),
        &format!(
            "[unit.with-jdk]\n\
             sources = [\"with-jdk\"]\n\
             jdk_home = {:?}\n\
             \n\
             [unit.without-jdk]\n\
             sources = [\"without-jdk\"]\n",
            beans_test_support::jdk::home()
        ),
    );
    // `java.util.List` and not `String`, because reaching a type without
    // naming it needs the implicit `java.lang` import, which is stage 4 of
    // `resolve_type_name` and unbuilt; see the `TODO.md`.
    for unit in ["with-jdk", "without-jdk"] {
        write(
            &root.join(unit).join("p").join("Uses.java"),
            "package p;\nimport java.util.List;\nclass Uses { List field; }\n",
        );
    }

    root
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("a file has a parent"))
        .expect("the temporary project should be writable");
    std::fs::write(path, contents).expect("the temporary project should be writable");
}

fn list_is_outside_scope(beans: &Beans, root: &Path, unit: &str) -> bool {
    let source = JvmSource::SourceFile {
        path: root.join(unit).join("p").join("Uses.java"),
    };
    beans
        .analyze(&source)
        .expect("a fixture source should be analyzed")
        .diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.code == "type-outside-scope"
                && diagnostic.message == "type List is outside the current compilation scope"
        })
}

#[test]
fn only_the_unit_that_names_a_jdk_can_see_the_runtime() {
    let root = project();
    let mut beans = Beans::new();

    assert_eq!(
        beans.open_workspace(&root).expect("the descriptor loads"),
        2
    );

    assert!(!list_is_outside_scope(&beans, &root, "with-jdk"));
    // The other unit proves the runtime was read rather than assumed: this
    // diagnostic only fires for a type Beans has indexed, so it says
    // `java.util.List` is in the lake and out of this unit's reach.
    assert!(list_is_outside_scope(&beans, &root, "without-jdk"));

    let _ = std::fs::remove_dir_all(&root);
}
