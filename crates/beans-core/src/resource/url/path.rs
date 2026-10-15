use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use super::{decode_component, encode_component};

pub(super) fn encode(path: &Path) -> Result<String, String> {
    let path = canonical(path);
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(encode_component(path.as_bytes(), true))
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        match path.to_str() {
            Some(path) => Ok(encode_component(path.as_bytes(), true)),
            None => Ok(encode_utf16(path.encode_wide())),
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let path = path
            .to_str()
            .ok_or("unsupported native file path encoding")?;
        Ok(encode_component(path.as_bytes(), true))
    }
}

pub(super) fn decode(encoded: &str) -> Result<PathBuf, String> {
    if encoded.starts_with("utf16:") {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            return Ok(OsString::from_wide(&decode_utf16(&encoded[6..])?).into());
        }
        #[cfg(not(windows))]
        return Err("UTF-16 file URLs require Windows".into());
    }
    let bytes = decode_component(encoded, true)?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(OsString::from_vec(bytes).into())
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(bytes)
            .map(PathBuf::from)
            .map_err(|_| "invalid UTF-8 in file resource URL".into())
    }
}

// Match Path's component-based equality without resolving parent steps or touching disk.
#[cfg(not(windows))]
fn canonical(path: &Path) -> OsString {
    path.components().collect::<PathBuf>().into_os_string()
}

#[cfg(windows)]
fn canonical(path: &Path) -> OsString {
    use std::path::Component;

    let mut result = OsString::new();
    let mut separator = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                result.push(canonical_prefix(prefix.kind()));
                separator = false;
            }
            Component::RootDir => {
                result.push(r"\");
                separator = false;
            }
            _ => {
                if separator {
                    result.push(r"\");
                }
                result.push(component.as_os_str());
                separator = true;
            }
        }
    }
    result
}

#[cfg(any(windows, test))]
fn canonical_prefix(prefix: std::path::Prefix<'_>) -> OsString {
    use std::path::Prefix;

    let mut result = OsString::new();
    match prefix {
        Prefix::Disk(drive) => result.push(format!("{}:", char::from(drive))),
        Prefix::VerbatimDisk(drive) => result.push(format!(r"\\?\{}:", char::from(drive))),
        Prefix::UNC(server, share) => {
            result.push(r"\\");
            result.push(server);
            result.push(r"\");
            result.push(share);
        }
        Prefix::VerbatimUNC(server, share) => {
            result.push(r"\\?\UNC\");
            result.push(server);
            result.push(r"\");
            result.push(share);
        }
        Prefix::DeviceNS(device) => {
            result.push(r"\\.\");
            result.push(device);
        }
        Prefix::Verbatim(value) => {
            result.push(r"\\?\");
            result.push(value);
        }
    }
    result
}

#[cfg(any(windows, test))]
fn encode_utf16(units: impl IntoIterator<Item = u16>) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::from("utf16:");
    for unit in units {
        for shift in [12, 8, 4, 0] {
            encoded.push(char::from(HEX[usize::from((unit >> shift) & 15)]));
        }
    }
    encoded
}

#[cfg(any(windows, test))]
fn decode_utf16(encoded: &str) -> Result<Vec<u16>, String> {
    if encoded.len() % 4 != 0 {
        return Err("invalid UTF-16 file URL payload".into());
    }
    let mut units = Vec::new();
    for chunk in encoded.as_bytes().chunks_exact(4) {
        let mut unit = 0;
        for &byte in chunk {
            let digit = super::hex(byte).ok_or("invalid UTF-16 file URL payload")?;
            unit = (unit << 4) | u16::from(digit);
        }
        units.push(unit);
    }
    Ok(units)
}

#[cfg(test)]
mod tests;
