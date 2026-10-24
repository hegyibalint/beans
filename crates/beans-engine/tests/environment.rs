use std::{env, fs, path::Path, process::Command as ProcessCommand};

use beans_core::event::Command;
use beans_engine::Engine;
use beans_testing::TempDir;

#[test]
fn the_default_directory_has_no_event_log() {
    let files = TempDir::new();

    run(files.root(), None, None).unwrap();

    assert!(files.path(".beans/models").is_file());
    assert!(!files.path(".beans/events.jsonl").exists());
}

#[test]
fn an_empty_work_dir_keeps_the_default() {
    let files = TempDir::new();

    run(files.root(), Some(Path::new("")), Some("1")).unwrap();

    assert!(files.path(".beans/models").is_file());
    assert_eq!(read_events(&files.path(".beans/events.jsonl")).len(), 2);
}

#[test]
fn a_relative_work_dir_holds_the_cache_and_event_log() {
    let files = TempDir::new();

    run(files.root(), Some(Path::new("state/cache")), Some("1")).unwrap();

    let work_dir = files.path("state/cache");
    assert!(work_dir.join("models").is_file());
    assert!(!files.path(".beans").exists());
    let events = read_events(&work_dir.join("events.jsonl"));
    assert_eq!(events.len(), 2);
    let Command::ProcessJdkHome(home) = &events[0] else {
        panic!("expected ProcessJdkHome");
    };
    assert_eq!(*home, files.path("jdk\n\"β\""));
    assert!(matches!(events[1], Command::Stop));
}

#[test]
fn an_absolute_work_dir_is_not_rebased_on_the_workspace() {
    let files = TempDir::new();
    let external = TempDir::new();
    let work_dir = external.path("state");

    run(files.root(), Some(&work_dir), Some("1")).unwrap();

    assert!(work_dir.join("models").is_file());
    assert_eq!(read_events(&work_dir.join("events.jsonl")).len(), 2);
    assert!(!files.path(".beans").exists());
}

#[test]
fn a_server_restart_replaces_the_previous_event_log() {
    let files = TempDir::new();
    let path = files.write(
        ".beans/events.jsonl",
        "an incomplete event from a previous run",
    );

    run(files.root(), None, Some("1")).unwrap();
    let first_run = fs::read(&path).unwrap();
    assert_eq!(read_events(&path).len(), 2);

    run(files.root(), None, Some("1")).unwrap();

    assert_eq!(fs::read(&path).unwrap(), first_run);
}

#[test]
fn only_one_enables_event_logging() {
    for value in ["", "0", "true"] {
        let files = TempDir::new();

        run(files.root(), None, Some(value)).unwrap();

        assert!(!files.path(".beans/events.jsonl").exists());
    }
}

#[test]
fn an_invalid_work_dir_is_an_error_without_a_fallback() {
    let files = TempDir::new();
    files.write("blocked", "not a directory");

    let error = run(files.root(), Some(Path::new("blocked")), None).unwrap_err();

    assert!(error.contains("blocked"));
    assert!(!files.path(".beans").exists());
}

#[test]
fn a_log_open_error_does_not_prevent_startup() {
    let files = TempDir::new();
    let work_dir = files.path("state");
    fs::create_dir_all(work_dir.join("events.jsonl")).unwrap();

    run(files.root(), Some(Path::new("state")), Some("1")).unwrap();

    assert!(work_dir.join("models").is_file());
    assert!(work_dir.join("events.jsonl").is_dir());
}

#[test]
fn engine_child() {
    let Some(root) = env::var_os("BEANS_TEST_ENGINE_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let engine = Engine::start(root).unwrap();
    engine
        .sender()
        .send(Command::ProcessJdkHome(root.join("jdk\n\"β\"")))
        .unwrap();
}

fn run(root: &Path, work_dir: Option<&Path>, event_log: Option<&str>) -> Result<(), String> {
    let mut command = ProcessCommand::new(env::current_exe().unwrap());
    command
        .args(["--exact", "engine_child", "--nocapture"])
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

fn read_events(path: &Path) -> Vec<Command> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
