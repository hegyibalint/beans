//! JVMS 26 §§4.4 and 4.7.23 permit a loadable dynamic constant as a bootstrap argument.

#[test]
fn a_dynamic_bootstrap_argument_remains_a_known_cafebabe_gap() {
    let error = super::super::parse(&dynamic_bootstrap_class()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("Unexpected constant pool reference type")
    );
}

fn dynamic_bootstrap_class() -> Vec<u8> {
    let mut bytes = Vec::new();
    put_u4(&mut bytes, 0xCAFE_BABE);
    put_u2(&mut bytes, 0);
    put_u2(&mut bytes, 69);
    put_u2(&mut bytes, 15);
    put_utf8(&mut bytes, b"beans/fixture/DynamicBootstrap");
    put_tagged_u2(&mut bytes, 7, 1);
    put_utf8(&mut bytes, b"java/lang/Object");
    put_tagged_u2(&mut bytes, 7, 3);
    put_utf8(&mut bytes, b"BootstrapMethods");
    put_utf8(&mut bytes, b"value");
    put_utf8(&mut bytes, b"Ljava/lang/Object;");
    put_tagged_pair(&mut bytes, 12, 6, 7);
    put_tagged_pair(&mut bytes, 17, 0, 8);
    put_utf8(&mut bytes, b"bootstrap");
    put_utf8(&mut bytes, b"()Ljava/lang/Object;");
    put_tagged_pair(&mut bytes, 12, 10, 11);
    put_tagged_pair(&mut bytes, 10, 2, 12);
    bytes.push(15);
    bytes.push(6);
    put_u2(&mut bytes, 13);
    put_u2(&mut bytes, 0x0021);
    put_u2(&mut bytes, 2);
    put_u2(&mut bytes, 4);
    put_u2(&mut bytes, 0);
    put_u2(&mut bytes, 0);
    put_u2(&mut bytes, 0);
    put_u2(&mut bytes, 1);
    put_u2(&mut bytes, 5);
    put_u4(&mut bytes, 8);
    put_u2(&mut bytes, 1);
    put_u2(&mut bytes, 14);
    put_u2(&mut bytes, 1);
    put_u2(&mut bytes, 9);
    bytes
}

fn put_utf8(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.push(1);
    put_u2(bytes, value.len() as u16);
    bytes.extend_from_slice(value);
}

fn put_tagged_u2(bytes: &mut Vec<u8>, tag: u8, value: u16) {
    bytes.push(tag);
    put_u2(bytes, value);
}

fn put_tagged_pair(bytes: &mut Vec<u8>, tag: u8, first: u16, second: u16) {
    bytes.push(tag);
    put_u2(bytes, first);
    put_u2(bytes, second);
}

fn put_u2(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_u4(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
