use std::io::BufWriter;

use beans_core::event::Command;

use super::record;

#[test]
fn commands_round_trip_as_individual_json_lines() {
    let home = "/jdk with spaces/\"β\"\nnext line";
    let mut bytes = Vec::new();
    record(&mut bytes, &Command::ProcessJdkHome(home.into())).unwrap();
    record(&mut bytes, &Command::ProcessResource(home.into())).unwrap();
    record(&mut bytes, &Command::Stop).unwrap();

    let text = String::from_utf8(bytes).unwrap();
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
fn every_record_is_flushed() {
    let mut writer = BufWriter::new(Vec::new());

    record(&mut writer, &Command::Stop).unwrap();

    assert_eq!(writer.get_ref(), b"{\"type\":\"Stop\"}\n");
}

#[test]
fn write_errors_are_returned() {
    let mut writer = &mut [][..];

    assert!(record(&mut writer, &Command::Stop).is_err());
}

#[cfg(unix)]
#[test]
fn serialization_errors_do_not_write_a_partial_record() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let mut bytes = Vec::new();
    let event = Command::ProcessJdkHome(OsString::from_vec(vec![0xff]).into());

    assert!(record(&mut bytes, &event).is_err());
    assert!(bytes.is_empty());
}
