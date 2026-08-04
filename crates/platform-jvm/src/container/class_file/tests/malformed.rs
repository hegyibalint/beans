const FEATURE: &[u8] = include_bytes!("fixtures/classes/beans/fixture/Feature.class");

#[test]
fn a_truncated_class_returns_an_error() {
    assert!(super::super::parse(&FEATURE[..FEATURE.len() / 2]).is_err());
}
