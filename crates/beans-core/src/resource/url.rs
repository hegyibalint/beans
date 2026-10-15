use super::{ResourceEntry, ResourceId, ResourceRoot};

mod path;

pub(super) fn encode(resource: &ResourceId) -> Result<String, String> {
    let mut url = match &resource.root {
        ResourceRoot::File { path } => format!("beans:file:{}", path::encode(path)?),
        ResourceRoot::Virtual { provider, key } => format!(
            "beans:virtual:{}/{}",
            encode_component(provider.as_bytes(), false),
            encode_component(key.as_bytes(), false),
        ),
    };
    for entry in &resource.entries {
        url.push_str("!/");
        url.push_str(&encode_component(entry.0.as_bytes(), true));
    }
    Ok(url)
}

pub(super) fn decode(url: &str) -> Result<ResourceId, String> {
    let mut parts = url.split("!/");
    let root = parts.next().unwrap_or_default();
    let root = if let Some(path) = root.strip_prefix("beans:file:") {
        ResourceRoot::File {
            path: path::decode(path)?,
        }
    } else if let Some(root) = root.strip_prefix("beans:virtual:") {
        let (provider, key) = root
            .split_once('/')
            .ok_or("missing virtual resource key separator")?;
        ResourceRoot::Virtual {
            provider: decode_text(provider, false)?,
            key: decode_text(key, false)?,
        }
    } else {
        return Err("expected a beans:file: or beans:virtual: resource URL".into());
    };

    let mut resource = ResourceId::new(root);
    for part in parts {
        resource
            .entries
            .push(ResourceEntry(decode_text(part, true)?));
    }
    Ok(resource)
}

fn encode_component(bytes: &[u8], allow_slashes: bool) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::new();
    for &byte in bytes {
        if is_unreserved(byte) || (allow_slashes && byte == b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 15)]));
        }
    }
    encoded
}

fn decode_text(encoded: &str, allow_slashes: bool) -> Result<String, String> {
    String::from_utf8(decode_component(encoded, allow_slashes)?)
        .map_err(|_| "invalid UTF-8 in resource URL component".into())
}

fn decode_component(encoded: &str, allow_slashes: bool) -> Result<Vec<u8>, String> {
    let mut decoded = Vec::new();
    let mut bytes = encoded.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = bytes.next().and_then(hex).ok_or("invalid percent escape")?;
            let low = bytes.next().and_then(hex).ok_or("invalid percent escape")?;
            decoded.push((high << 4) | low);
        } else if is_unreserved(byte) || (allow_slashes && byte == b'/') {
            decoded.push(byte);
        } else {
            return Err("unescaped character in resource URL component".into());
        }
    }
    Ok(decoded)
}

// RFC 3986 §2.3; slashes are preserved only within file paths and individual entries.
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
