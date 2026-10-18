use std::{env, fs, path::Path, process::Command};

use beans_core::events::{Event, JavaHomeFound};

use crate::Engine;

#[test]
fn the_default_directory_has_no_event_log() {
    let root = tempfile::tempdir().unwrap();

    run(root.path(), None, None).unwrap();

    assert!(root.path().join(".beans/models").is_file());
    assert!(!root.path().join(".beans/events.jsonl").exists());
}

#[test]
fn an_empty_work_dir_keeps_the_default() {
    let root = tempfile::tempdir().unwrap();

    run(root.path(), Some(Path::new("")), Some("1")).unwrap();

    assert!(root.path().join(".beans/models").is_file());
    assert_eq!(
        read_events(&root.path().join(".beans/events.jsonl")).len(),
        2
    );
}

#[test]
fn a_relative_work_dir_holds_the_cache_and_event_log() {
    let root = tempfile::tempdir().unwrap();

    run(root.path(), Some(Path::new("state/cache")), Some("1")).unwrap();

    let work_dir = root.path().join("state/cache");
    assert!(work_dir.join("models").is_file());
    assert!(!root.path().join(".beans").exists());
    let events = read_events(&work_dir.join("events.jsonl"));
    assert_eq!(events.len(), 2);
    let Event::JavaHomeFound(found) = &events[0] else {
        panic!("expected JavaHomeFound");
    };
    assert_eq!(found.home, root.path().join("jdk\n\"β\""));
    assert!(matches!(events[1], Event::Stop));
}

#[test]
fn an_absolute_work_dir_is_not_rebased_on_the_workspace() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let work_dir = external.path().join("state");

    run(root.path(), Some(&work_dir), Some("1")).unwrap();

    assert!(work_dir.join("models").is_file());
    assert_eq!(read_events(&work_dir.join("events.jsonl")).len(), 2);
    assert!(!root.path().join(".beans").exists());
}

#[test]
fn a_server_restart_replaces_the_previous_event_log() {
    let root = tempfile::tempdir().unwrap();
    let work_dir = root.path().join(".beans");
    fs::create_dir_all(&work_dir).unwrap();
    let path = work_dir.join("events.jsonl");
    fs::write(&path, "an incomplete event from a previous run").unwrap();

    run(root.path(), None, Some("1")).unwrap();
    let first_run = fs::read(&path).unwrap();
    assert_eq!(read_events(&path).len(), 2);

    run(root.path(), None, Some("1")).unwrap();

    assert_eq!(fs::read(&path).unwrap(), first_run);
}

#[test]
fn only_one_enables_event_logging() {
    for value in ["", "0", "true"] {
        let root = tempfile::tempdir().unwrap();

        run(root.path(), None, Some(value)).unwrap();

        assert!(!root.path().join(".beans/events.jsonl").exists());
    }
}

#[test]
fn an_invalid_work_dir_is_an_error_without_a_fallback() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("blocked"), "not a directory").unwrap();

    let error = run(root.path(), Some(Path::new("blocked")), None).unwrap_err();

    assert!(error.contains("blocked"));
    assert!(!root.path().join(".beans").exists());
}

#[test]
fn a_log_open_error_does_not_prevent_startup() {
    let root = tempfile::tempdir().unwrap();
    let work_dir = root.path().join("state");
    fs::create_dir_all(work_dir.join("events.jsonl")).unwrap();

    run(root.path(), Some(Path::new("state")), Some("1")).unwrap();

    assert!(work_dir.join("models").is_file());
    assert!(work_dir.join("events.jsonl").is_dir());
}

#[test]
fn engine_child() {
    let Some(root) = env::var_os("BEANS_TEST_ENGINE_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let mut engine = Engine::new(root).unwrap();
    engine
        .sender()
        .send(Event::JavaHomeFound(JavaHomeFound {
            home: root.join("jdk\n\"β\""),
        }))
        .unwrap();
    engine.start().unwrap();
    engine.stop().unwrap();
}

fn run(root: &Path, work_dir: Option<&Path>, event_log: Option<&str>) -> Result<(), String> {
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .args(["--exact", "tests::environment::engine_child", "--nocapture"])
        .env("BEANS_TEST_ENGINE_ROOT", root)
        .env_remove("BEANS_WORK_DIR")
        .env_remove("BEANS_EVENT_LOG");
    if let Some(path) = work_dir {
        command.env("BEANS_WORK_DIR", path);
    }
    if let Some(value) = event_log {
        command.env("BEANS_EVENT_LOG", value);
    }
    let output = command.output().unwrap();
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn read_events(path: &Path) -> Vec<Event> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
