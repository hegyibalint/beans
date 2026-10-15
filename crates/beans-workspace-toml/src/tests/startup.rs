use std::{fs, path::Path};

use beans_core::{events::Event, resource::ResourceId};
use crossbeam_channel::{Receiver, unbounded};

use crate::{DESCRIPTOR, TomlWorkspace};

#[test]
fn an_absent_descriptor_emits_nothing() {
    let root = tempfile::tempdir().unwrap();
    let (sender, events) = unbounded();

    assert!(
        TomlWorkspace::start(root.path(), &sender)
            .unwrap()
            .is_none()
    );
    assert!(events.try_recv().is_err());
}

#[test]
fn an_invalid_descriptor_emits_nothing() {
    let root = tempfile::tempdir().unwrap();
    let descriptor = root.path().join(DESCRIPTOR);
    fs::write(&descriptor, "[module.app]\ndepends_on = [\"missing\"]\n").unwrap();
    let (sender, events) = unbounded();

    let error = TomlWorkspace::start(root.path(), &sender).unwrap_err();

    assert!(error.contains(&descriptor.display().to_string()), "{error}");
    assert!(events.try_recv().is_err());
}

#[test]
fn an_unreadable_descriptor_emits_nothing() {
    let root = tempfile::tempdir().unwrap();
    let descriptor = root.path().join(DESCRIPTOR);
    fs::create_dir(&descriptor).unwrap();
    let (sender, events) = unbounded();

    let error = TomlWorkspace::start(root.path(), &sender).unwrap_err();

    assert!(error.contains(&descriptor.display().to_string()), "{error}");
    assert!(events.try_recv().is_err());
}

#[test]
fn a_workspace_without_a_jdk_still_loads() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join(DESCRIPTOR),
        "[module.app]\nsources = [\"src\"]\n",
    )
    .unwrap();
    let (sender, events) = unbounded();

    let workspace = TomlWorkspace::start(root.path(), &sender).unwrap().unwrap();

    assert_eq!(
        workspace.model().modules["app"].sources,
        [root.path().join("src")]
    );
    assert!(events.try_recv().is_err());
}

#[test]
fn reports_the_home_before_its_artifacts_without_decoding_containers() {
    for relative in ["lib/modules", "jre/lib/rt.jar", "lib/rt.jar"] {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("jdk");
        let runtime = home.join(relative);
        fs::create_dir_all(runtime.parent().unwrap()).unwrap();
        fs::write(&runtime, b"not a container").unwrap();
        fs::write(
            root.path().join(DESCRIPTOR),
            "jdk_home = \"jdk\"\n[module.app]\n",
        )
        .unwrap();
        let (sender, events) = unbounded();

        let workspace = TomlWorkspace::start(root.path(), &sender).unwrap().unwrap();

        assert_eq!(workspace.model().jdk_home.as_ref(), Some(&home));
        expect_home(&events, &home);
        expect_resource(&events, &runtime);
        assert!(events.try_recv().is_err());
    }
}

#[test]
fn the_project_home_is_reported_once_with_optional_legacy_tools() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("jdk");
    let runtime = home.join("lib/rt.jar");
    let tools = home.join("lib/tools.jar");
    fs::create_dir_all(runtime.parent().unwrap()).unwrap();
    fs::write(&runtime, []).unwrap();
    fs::write(&tools, []).unwrap();
    fs::write(
        root.path().join(DESCRIPTOR),
        "jdk_home = \"jdk\"\n[module.app]\n[module.test]\n",
    )
    .unwrap();
    let (sender, events) = unbounded();

    TomlWorkspace::start(root.path(), &sender).unwrap().unwrap();

    expect_home(&events, &home);
    expect_resource(&events, &runtime);
    expect_resource(&events, &tools);
    assert!(events.try_recv().is_err());
}

#[test]
fn an_unusable_jdk_emits_nothing_but_preserves_the_workspace() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("missing-jdk");
    fs::write(
        root.path().join(DESCRIPTOR),
        "jdk_home = \"missing-jdk\"\n[module.app]\n[module.test]\ndepends_on = [\"app\"]\n",
    )
    .unwrap();
    let (sender, events) = unbounded();

    let workspace = TomlWorkspace::start(root.path(), &sender).unwrap().unwrap();

    assert_eq!(workspace.model().modules.len(), 2);
    assert_eq!(workspace.model().jdk_home.as_ref(), Some(&home));
    assert!(events.try_recv().is_err());
}

#[test]
fn invalid_runtime_or_optional_artifacts_emit_no_partial_jdk_events() {
    for blocked in ["lib/modules", "lib/tools.jar"] {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("jdk");
        fs::create_dir_all(home.join("lib")).unwrap();
        fs::write(home.join("lib/rt.jar"), []).unwrap();
        fs::create_dir_all(home.join(blocked)).unwrap();
        fs::write(
            root.path().join(DESCRIPTOR),
            "jdk_home = \"jdk\"\n[module.app]\n",
        )
        .unwrap();
        let (sender, events) = unbounded();

        assert!(
            TomlWorkspace::start(root.path(), &sender)
                .unwrap()
                .is_some()
        );
        assert!(events.try_recv().is_err());
    }
}

#[test]
fn a_project_can_report_its_jdk_without_any_modules() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("jdk");
    let runtime = home.join("lib/modules");
    fs::create_dir_all(runtime.parent().unwrap()).unwrap();
    fs::write(&runtime, []).unwrap();
    fs::write(root.path().join(DESCRIPTOR), "jdk_home = \"jdk\"\n").unwrap();
    let (sender, events) = unbounded();

    let workspace = TomlWorkspace::start(root.path(), &sender).unwrap().unwrap();

    assert!(workspace.model().modules.is_empty());
    expect_home(&events, &home);
    expect_resource(&events, &runtime);
    assert!(events.try_recv().is_err());
}

#[test]
fn a_disconnected_inbox_is_reported() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("jdk/lib")).unwrap();
    fs::write(root.path().join("jdk/lib/modules"), []).unwrap();
    fs::write(
        root.path().join(DESCRIPTOR),
        "jdk_home = \"jdk\"\n[module.app]\n",
    )
    .unwrap();
    let (sender, events) = unbounded();
    drop(events);

    let error = TomlWorkspace::start(root.path(), &sender).unwrap_err();

    assert!(error.contains("disconnected channel"), "{error}");
}

#[test]
fn startup_respects_the_environment_override() {
    const CHILD_ROOT: &str = "BEANS_TEST_STARTUP_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = std::path::PathBuf::from(root);
        let home = root.join("override-jdk");
        let (sender, events) = unbounded();
        let workspace = TomlWorkspace::start(&root, &sender).unwrap().unwrap();
        assert_eq!(workspace.model().jdk_home.as_ref(), Some(&home));
        expect_home(&events, &home);
        expect_resource(&events, &home.join("lib/modules"));
        assert!(events.try_recv().is_err());
        return;
    }

    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("override-jdk/lib")).unwrap();
    fs::write(root.path().join("override-jdk/lib/modules"), []).unwrap();
    fs::write(
        root.path().join(DESCRIPTOR),
        "jdk_home = \"descriptor-jdk\"\n[module.app]\n",
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::startup::startup_respects_the_environment_override",
            "--nocapture",
        ])
        .env(CHILD_ROOT, root.path())
        .env("BEANS_WORKSPACE_TOML_JAVA_HOME", "override-jdk")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn expect_home(events: &Receiver<Event>, home: &Path) {
    let Event::JavaHomeFound(found) = events.try_recv().unwrap() else {
        panic!("expected a Java home");
    };
    assert_eq!(found.home, home);
    assert!(found.version.is_none());
}

fn expect_resource(events: &Receiver<Event>, artifact: &Path) {
    let Event::ResourceFound(found) = events.try_recv().unwrap() else {
        panic!("expected a resource");
    };
    assert_eq!(found.resource, ResourceId::file(artifact));
}
