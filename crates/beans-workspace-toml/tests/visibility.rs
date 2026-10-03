use std::path::Path;

use beans_core::origin::Origin;
use beans_workspace::Workspaces;
use beans_workspace_toml::parse;

#[test]
fn descriptor_visibility_reaches_the_workspaces_facade() {
    let backend = parse(
        r#"
        [unit.app]
        sources = ["app/src"]
        depends_on = ["lib"]

        [unit.lib]
        sources = ["lib/src"]

        [unit.other]
        sources = ["other/src"]
    "#,
        Path::new("/project"),
    )
    .unwrap();
    let workspaces = Workspaces::new(backend);

    let classpath = workspaces.classpath_for("file:///project/app/src/Use.java");

    assert!(classpath.contains(&Origin::uri("file:///project/lib/src/Target.java")));
    assert!(!classpath.contains(&Origin::uri("file:///project/other/src/Target.java")));
}
