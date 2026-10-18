use beans_core::events::Event;

use crate::features::Features;

use super::params;

#[test]
fn initialization_starts_the_engine() {
    let root = tempfile::tempdir().unwrap();
    let mut features = Features::initialize(&params(&[root.path()])).unwrap();

    assert_eq!(
        features.engine.start().unwrap_err(),
        "engine is already started"
    );
    features.engine.stop().unwrap();
}

#[test]
fn dropping_features_stops_the_engine() {
    let root = tempfile::tempdir().unwrap();
    let features = Features::initialize(&params(&[root.path()])).unwrap();
    let sender = features.engine.sender();

    drop(features);

    assert!(sender.send(Event::Stop).is_err());
}
