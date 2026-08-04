use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::container::jimage::is_image;
use crate::container::{Error, ErrorKind};
use crate::model::{JvmClass, JvmSource};

/// The runtime image these cases read.
///
/// One cannot be committed — the smallest `jlink` will build is 17 MB — so the
/// only honest fixture is a real one, and every claim here is written to hold
/// of any release rather than of a particular one.
///
/// Which JDK that is comes from `mise.toml`, so a clone reads the same one a
/// clone was given. `MISE_JAVA_VERSION` names another for one run, and
/// `mise.ci.toml` lists the sweep.
fn runtime_image() -> &'static Path {
    static IMAGE: OnceLock<PathBuf> = OnceLock::new();
    IMAGE.get_or_init(|| {
        let output = Command::new("mise")
            .args(["where", "java"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("`mise` should be on PATH; it is what provisions the JDKs");
        assert!(
            output.status.success(),
            "no JDK to read: run `mise install`\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        Path::new(String::from_utf8_lossy(&output.stdout).trim())
            .join("lib")
            .join("modules")
    })
}

struct Walk {
    classes: Vec<(JvmSource, JvmClass)>,
    errors: Vec<Error>,
}

/// Reads the whole image once, because every claim below is about what one
/// full pass produces and a JDK is some 27,000 entries.
fn walk() -> &'static Walk {
    static WALK: OnceLock<Walk> = OnceLock::new();
    WALK.get_or_init(|| {
        let mut walk = Walk {
            classes: Vec::new(),
            errors: Vec::new(),
        };
        for item in crate::container::process(runtime_image()) {
            match item {
                Ok(class) => walk.classes.push(class),
                Err(error) => walk.errors.push(error),
            }
        }
        walk
    })
}

fn entry_path(source: &JvmSource) -> &str {
    match source {
        JvmSource::JimageEntry { entry_path, .. } => entry_path,
        other => panic!("a runtime image should produce only image entries: {other:?}"),
    }
}

#[test]
fn an_image_is_recognised_by_its_first_four_bytes() {
    assert!(is_image(runtime_image()));

    let class_file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/class_file/tests/fixtures/classes/beans/fixture/Point.class");
    assert!(!is_image(&class_file));
    assert!(!is_image(Path::new("/nowhere/modules")));
}

#[test]
fn the_whole_runtime_arrives() {
    // Every JDK from 11 to 26 carries between 26,000 and 31,000 classes. The
    // claim is that a full pass reaches them, not the count of any release,
    // and a pass handing the parser the wrong bytes would collapse it.
    let classes = walk().classes.len();

    assert!(classes > 20_000, "{classes}");
}

#[test]
fn no_entry_fails_to_be_located_read_or_expanded() {
    // What survives is `cafebabe` meeting a class file it cannot hold, which
    // `class_file/tests/compatibility.rs` names. JDK 26 has one. Anything the
    // image reader itself got wrong would arrive as another kind.
    for error in &walk().errors {
        assert!(matches!(error.kind, ErrorKind::Parse(_)), "{error}");
    }
}

#[test]
fn a_class_is_sourced_at_its_module_and_its_path() {
    let walk = walk();

    let (source, class) = walk
        .classes
        .iter()
        .find(|(_, class)| class.fqn.as_str() == "java.lang.String")
        .expect("every runtime image holds java.lang.String");

    assert_eq!(
        *source,
        JvmSource::JimageEntry {
            jimage_path: runtime_image().to_path_buf(),
            entry_path: "java.base/java/lang/String.class".to_string(),
        }
    );
    // A resource read one byte short still has a parsable header, so this is
    // what says the whole entry arrived.
    assert!(class.methods.iter().any(|method| method.name == "length"));
}

#[test]
fn the_filesystem_view_contributes_nothing() {
    // `/modules/<module>/<package>` and `/packages/<name>` sit in the same
    // table as the resources, and their bytes are the offsets of their
    // children rather than files. Their module attribute is the string
    // `modules` or `packages`, so mistaking one for a resource names it that
    // way.
    for (source, _) in &walk().classes {
        let path = entry_path(source);
        assert!(!path.starts_with("modules/"), "{path}");
        assert!(!path.starts_with("packages/"), "{path}");
    }
}

#[test]
fn a_module_descriptor_contributes_nothing() {
    let descriptors = walk()
        .classes
        .iter()
        .filter(|(source, _)| entry_path(source).ends_with("/module-info.class"))
        .count();

    assert_eq!(descriptors, 0);
}
