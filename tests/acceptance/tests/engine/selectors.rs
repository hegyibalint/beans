//! What a unit's `sources` entry names. A directory is the ordinary case; one
//! file is the case that lets a unit live beside others without swallowing
//! them, which is how `examples/beans` puts `Demo.java` next to `a/` and `b/`.

use std::path::{Path, PathBuf};

use beans::Beans;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/engine/fixtures/file-selector")
}

/// Scoping already treated a named file as its own tree — `of_source` places a
/// file by `path.starts_with(base)`, and a path is a prefix of itself. Reading
/// did not: `collect` asked the filesystem for a directory listing and got an
/// error, so the file was in scope and never loaded.
///
/// `Sibling.java` sits in the same directory and is not named, so a passing
/// count of one is also the claim that naming a file does not quietly pull in
/// the directory around it.
#[test]
fn a_unit_may_name_one_file_instead_of_a_tree() {
    let mut beans = Beans::new();

    let loaded = beans
        .open_workspace(&fixture_root())
        .expect("the descriptor loads");

    assert_eq!(loaded, 1);
}
