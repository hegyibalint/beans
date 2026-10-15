use std::{ffi::OsStr, path::Prefix};

use super::*;

#[test]
fn utf16_payloads_preserve_every_code_unit_without_an_endianness_dependency() {
    let units: Vec<_> = (0..=u16::MAX).collect();
    let encoded = encode_utf16(units.iter().copied());

    assert_eq!(
        decode_utf16(encoded.strip_prefix("utf16:").unwrap()).unwrap(),
        units
    );
    assert_eq!(encode_utf16([0xD800, 0x0061]), "utf16:D8000061");
}

#[test]
fn malformed_utf16_payloads_are_rejected() {
    for encoded in ["0", "000", "00000", "ZZZZ", "D80G", "%000", "🫘"] {
        assert!(decode_utf16(encoded).is_err(), "accepted {encoded:?}");
    }
    assert_eq!(decode_utf16("d800dc00").unwrap(), [0xD800, 0xDC00]);
}

#[test]
fn windows_prefix_spelling_is_reconstructed_from_its_parsed_kind() {
    for (prefix, expected) in [
        (Prefix::Disk(b'C'), "C:"),
        (Prefix::VerbatimDisk(b'C'), r"\\?\C:"),
        (
            Prefix::UNC(OsStr::new("server"), OsStr::new("share")),
            r"\\server\share",
        ),
        (
            Prefix::VerbatimUNC(OsStr::new("server"), OsStr::new("share")),
            r"\\?\UNC\server\share",
        ),
        (Prefix::DeviceNS(OsStr::new("device")), r"\\.\device"),
        (Prefix::Verbatim(OsStr::new("volume")), r"\\?\volume"),
    ] {
        assert_eq!(canonical_prefix(prefix), OsString::from(expected));
    }
}

#[cfg(windows)]
#[test]
fn verbatim_paths_do_not_resolve_parent_steps() {
    let path = Path::new(r"\\?\C:\a\..\b");
    let encoded = encode(path).unwrap();

    assert_eq!(decode(&encoded).unwrap(), path);
    assert_ne!(encoded, encode(Path::new(r"\\?\C:\b")).unwrap());
}
