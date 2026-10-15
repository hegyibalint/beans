use super::*;

#[test]
fn version_one_has_a_fixed_class_encoding() {
    let class = Class { name: "p/C".into() };
    let bytes = [1, 3, b'p', b'/', b'C'];
    assert_eq!(encode(&class).unwrap(), bytes);
    assert_eq!(decode(&bytes).unwrap().name, class.name);
}

#[test]
fn class_names_round_trip_and_do_not_borrow_the_blob() {
    for name in ["", "java/lang/String", "p/Outer$Inner", "café/Élément"] {
        let class = {
            let bytes = encode(&Class { name: name.into() }).unwrap();
            decode(&bytes).unwrap()
        };
        assert_eq!(class.name, name);
    }
}

#[test]
fn missing_unsupported_and_truncated_payloads_are_rejected() {
    assert_eq!(decode(&[]).err().unwrap(), "missing JVM model version");
    assert_eq!(
        decode(&[2]).err().unwrap(),
        "unsupported JVM model version: 2"
    );
    let bytes = encode(&Class { name: "p/C".into() }).unwrap();
    for end in 1..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
}

#[test]
fn invalid_utf8_and_trailing_bytes_are_rejected() {
    assert!(decode(&[1, 1, 255]).is_err());
    let mut bytes = encode(&Class { name: "p/C".into() }).unwrap();
    bytes.push(0);
    assert_eq!(decode(&bytes).err().unwrap(), "trailing bytes in JVM model");
}
