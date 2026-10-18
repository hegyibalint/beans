use std::io::BufWriter;

use beans_core::events::{Event, JavaHomeFound};

use super::record;

#[test]
fn events_round_trip_as_individual_json_lines() {
    let home = "/jdk with spaces/\"β\"\nnext line";
    let mut bytes = Vec::new();
    record(
        &mut bytes,
        &Event::JavaHomeFound(JavaHomeFound { home: home.into() }),
    )
    .unwrap();
    record(&mut bytes, &Event::Stop).unwrap();

    let text = String::from_utf8(bytes).unwrap();
    assert!(text.ends_with('\n'));
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    let Event::JavaHomeFound(found) = serde_json::from_str::<Event>(lines[0]).unwrap() else {
        panic!("expected JavaHomeFound");
    };
    assert_eq!(found.home.to_str(), Some(home));
    assert!(matches!(
        serde_json::from_str::<Event>(lines[1]).unwrap(),
        Event::Stop
    ));
}

#[test]
fn every_record_is_flushed() {
    let mut writer = BufWriter::new(Vec::new());

    record(&mut writer, &Event::Stop).unwrap();

    assert_eq!(writer.get_ref(), b"\"Stop\"\n");
}

#[test]
fn write_errors_are_returned() {
    let mut writer = &mut [][..];

    assert!(record(&mut writer, &Event::Stop).is_err());
}

#[cfg(unix)]
#[test]
fn serialization_errors_do_not_write_a_partial_record() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let mut bytes = Vec::new();
    let event = Event::JavaHomeFound(JavaHomeFound {
        home: OsString::from_vec(vec![0xff]).into(),
    });

    assert!(record(&mut bytes, &event).is_err());
    assert!(bytes.is_empty());
}
