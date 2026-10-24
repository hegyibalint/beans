use std::fs::{self, File};

use beans_core::event::Command;
use beans_engine::logging::Logger;
use beans_testing::TempDir;

#[test]
fn commands_round_trip_as_individual_json_lines() {
    let files = TempDir::new();
    let path = files.path("events.jsonl");
    let mut logger = Logger::new(File::create(&path).unwrap());
    let home = "/jdk with spaces/\"β\"\nnext line";
    logger.record(&Command::ProcessJdkHome(home.into()));
    logger.record(&Command::ProcessResource(home.into()));
    logger.record(&Command::Stop);

    let text = fs::read_to_string(&path).unwrap();
    assert!(text.ends_with('\n'));
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(lines[0]).unwrap(),
        serde_json::json!({"type": "ProcessJdkHome", "data": home})
    );
    let Command::ProcessJdkHome(decoded) = serde_json::from_str::<Command>(lines[0]).unwrap()
    else {
        panic!("expected ProcessJdkHome");
    };
    assert_eq!(decoded.to_str(), Some(home));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(lines[1]).unwrap(),
        serde_json::json!({"type": "ProcessResource", "data": home})
    );
    let Command::ProcessResource(decoded) = serde_json::from_str::<Command>(lines[1]).unwrap()
    else {
        panic!("expected ProcessResource");
    };
    assert_eq!(decoded.to_str(), Some(home));
    assert!(matches!(
        serde_json::from_str::<Command>(lines[2]).unwrap(),
        Command::Stop
    ));
}

#[test]
fn every_record_is_visible_before_the_logger_is_dropped() {
    let files = TempDir::new();
    let path = files.path("events.jsonl");
    let mut logger = Logger::new(File::create(&path).unwrap());

    for count in 1..=2 {
        logger.record(&Command::Stop);

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\"type\":\"Stop\"}\n".repeat(count)
        );
    }
}

#[test]
fn a_read_only_file_does_not_propagate_write_errors_or_change_contents() {
    let files = TempDir::new();
    let path = files.write("events.jsonl", "previous events\n");
    let mut logger = Logger::new(File::open(&path).unwrap());

    logger.record(&Command::ProcessJdkHome("/jdk".into()));
    logger.record(&Command::Stop);

    assert_eq!(fs::read_to_string(&path).unwrap(), "previous events\n");
}

#[cfg(unix)]
#[test]
fn serialization_errors_disable_logging_without_writing_a_partial_record() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let files = TempDir::new();
    let path = files.path("events.jsonl");
    let mut logger = Logger::new(File::create(&path).unwrap());
    logger.record(&Command::Stop);
    let event = Command::ProcessJdkHome(OsString::from_vec(vec![0xff]).into());

    logger.record(&event);
    logger.record(&Command::Stop);

    assert_eq!(fs::read(&path).unwrap(), b"{\"type\":\"Stop\"}\n");
}
