// Flattening a workspace into one scope per unit. This is the only place a
// dependency edge is ever followed, so it is the only place one can be followed
// wrongly; everything downstream sees a flat set of containers and no graph.

use beans_platform_jvm::query::JvmScopeQuery;

use super::*;

/// Every scope claiming the file at `path`, which is what the engine registers.
fn claiming(workspace: &Workspace, path: &str) -> Vec<JvmScope> {
    Scopes::of(workspace).of_source(&source_file(path))
}

/// Can a file at `from` reach `to` once the workspace is flattened? Asked
/// through `JvmScopeQuery`, which is the door the platform uses.
fn reaches(workspace: &Workspace, from: &str, to: &JvmSource) -> bool {
    JvmScopeQuery::of(claiming(workspace, from)).contains(to)
}

fn depending(id: &str, trees: &[&str], depends_on: &[&str]) -> Unit {
    Unit {
        depends_on: depends_on.iter().map(|id| (*id).to_string()).collect(),
        ..unit(id, trees.iter().map(|base| tree(base)).collect())
    }
}

/// `app -> lib -> core`, which is the shortest project that can tell a direct
/// edge apart from a transitive one.
fn chain() -> Workspace {
    workspace(vec![
        depending("app", &["app/src"], &["lib"]),
        depending("lib", &["lib/src"], &["core"]),
        depending("core", &["core/src"], &[]),
    ])
}

#[test]
fn a_file_under_a_declared_tree_is_claimed_by_that_unit() {
    let workspace = workspace(vec![unit("app", vec![tree("app/src")])]);

    assert_eq!(claiming(&workspace, "app/src/p/A.java").len(), 1);
}

/// No scope is not an empty scope. The engine has to leave such a file
/// unregistered, because registering an empty list would say it sees nothing,
/// and a scratch file next to a project has to keep working.
#[test]
fn a_file_no_unit_declares_is_claimed_by_none() {
    let workspace = workspace(vec![unit("app", vec![tree("app/src")])]);

    assert!(claiming(&workspace, "scratch/A.java").is_empty());
}

/// Shared code: one tree, two units compiling it, so the file belongs to both.
#[test]
fn a_tree_two_units_declare_claims_the_file_twice() {
    let workspace = workspace(vec![
        unit("app", vec![tree("common")]),
        unit("lib", vec![tree("common")]),
    ]);

    assert_eq!(claiming(&workspace, "common/p/A.java").len(), 2);
}

/// A path is a prefix of itself, so a unit that lists files rather than a tree
/// still claims them.
#[test]
fn a_listed_file_is_a_tree_of_one() {
    let workspace = workspace(vec![unit(
        "app",
        vec![Selector::Files {
            files: vec![PathBuf::from("app/p/A.java")],
            generated: false,
        }],
    )]);

    assert_eq!(claiming(&workspace, "app/p/A.java").len(), 1);
    assert!(claiming(&workspace, "app/p/B.java").is_empty());
}

/// A jar entry is reached through some unit's classpath; no unit owns it, so a
/// workspace places nothing.
#[test]
fn only_a_source_file_is_placed_by_a_workspace() {
    let workspace = workspace(vec![unit("app", vec![tree("app/src")])]);
    let entry = JvmSource::JarEntry {
        jar_path: PathBuf::from("lib.jar"),
        entry_path: "p/A.class".to_string(),
    };

    assert!(Scopes::of(&workspace).of_source(&entry).is_empty());
}

#[test]
fn a_unit_sees_its_own_sources() {
    let workspace = chain();

    assert!(reaches(
        &workspace,
        "app/src/p/A.java",
        &source_file("app/src/p/B.java")
    ));
}

#[test]
fn a_unit_sees_the_sources_of_a_unit_it_depends_on() {
    let workspace = chain();

    assert!(reaches(
        &workspace,
        "app/src/p/A.java",
        &source_file("lib/src/p/B.java")
    ));
}

/// The edge points one way; being depended upon buys nothing.
#[test]
fn a_unit_does_not_see_back_down_an_edge() {
    let workspace = chain();

    assert!(!reaches(
        &workspace,
        "lib/src/p/B.java",
        &source_file("app/src/p/A.java")
    ));
}

/// `depends_on` is followed one hop, so `app -> lib -> core` gives `app` the
/// sources of `lib` and not those of `core`. Whether a descriptor means the
/// edge to chain is a question for the workspace layer, and until it answers,
/// this is the behaviour to notice changing.
#[test]
fn a_dependency_of_a_dependency_is_out_of_reach() {
    let workspace = chain();

    assert!(reaches(
        &workspace,
        "lib/src/p/B.java",
        &source_file("core/src/p/C.java")
    ));
    assert!(!reaches(
        &workspace,
        "app/src/p/A.java",
        &source_file("core/src/p/C.java")
    ));
}

#[test]
fn a_unit_sees_what_it_links_against() {
    let workspace = workspace(vec![Unit {
        classpath: vec![PathBuf::from("lib.jar")],
        ..unit("app", vec![tree("app/src")])
    }]);
    let entry = JvmSource::JarEntry {
        jar_path: PathBuf::from("lib.jar"),
        entry_path: "p/A.class".to_string(),
    };

    assert!(reaches(&workspace, "app/src/p/A.java", &entry));
}

/// One image for the whole runtime, which JPMS says is too much; splitting it
/// needs the lake to hold modules first.
#[test]
fn a_jdk_home_contributes_its_runtime_image() {
    let workspace = workspace(vec![Unit {
        jdk_home: Some(PathBuf::from("/jdk")),
        ..unit("app", vec![tree("app/src")])
    }]);
    let entry = JvmSource::JimageEntry {
        jimage_path: PathBuf::from("/jdk/lib/modules"),
        entry_path: "java.base/java/lang/String.class".to_string(),
    };

    assert!(reaches(&workspace, "app/src/p/A.java", &entry));
}

/// A descriptor typo. Nothing validates unit ids yet, and dropping the edge
/// beats failing the whole import over it.
#[test]
fn a_dependency_naming_no_unit_is_dropped() {
    let workspace = workspace(vec![depending("app", &["app/src"], &["nonexistent"])]);

    assert!(reaches(
        &workspace,
        "app/src/p/A.java",
        &source_file("app/src/p/B.java")
    ));
}
