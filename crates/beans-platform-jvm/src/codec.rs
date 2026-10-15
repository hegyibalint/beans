use crate::Class;

// Bump when the serialized class layout changes.
const MODEL_VERSION: u8 = 1;

pub fn encode(class: &Class) -> Result<Vec<u8>, String> {
    postcard::to_extend(class, vec![MODEL_VERSION]).map_err(|error| error.to_string())
}

pub fn decode(bytes: &[u8]) -> Result<Class, String> {
    let (&version, payload) = bytes.split_first().ok_or("missing JVM model version")?;
    if version != MODEL_VERSION {
        return Err(format!("unsupported JVM model version: {version}"));
    }
    let (class, remaining) =
        postcard::take_from_bytes(payload).map_err(|error| error.to_string())?;
    if !remaining.is_empty() {
        return Err("trailing bytes in JVM model".into());
    }
    Ok(class)
}

#[cfg(test)]
mod tests;
