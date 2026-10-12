pub mod jdks;

pub fn class_bytes(name: &str) -> Vec<u8> {
    let mut bytes = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52, 0, 5];
    for (name, index) in [(name, 1u16), ("java/lang/Object", 3)] {
        bytes.push(1);
        bytes.extend((name.len() as u16).to_be_bytes());
        bytes.extend(name.as_bytes());
        bytes.push(7);
        bytes.extend(index.to_be_bytes());
    }
    for value in [0x21u16, 2, 4, 0, 0, 0, 0] {
        bytes.extend(value.to_be_bytes());
    }
    bytes
}
