use crate::model::File;

// Bump when the serialized layout changes, including nested model and core types.
const MODEL_VERSION: u8 = 1;

pub fn encode(file: &File) -> Result<Vec<u8>, String> {
    postcard::to_extend(file, vec![MODEL_VERSION]).map_err(|error| error.to_string())
}

pub fn decode(bytes: &[u8]) -> Result<File, String> {
    let (&version, payload) = bytes.split_first().ok_or("missing Java model version")?;
    if version != MODEL_VERSION {
        return Err(format!("unsupported Java model version: {version}"));
    }
    let (file, remaining): (File, _) =
        postcard::take_from_bytes(payload).map_err(|error| error.to_string())?;
    if !remaining.is_empty() {
        return Err("trailing bytes in Java model".into());
    }
    file.validate_nodes()?;
    Ok(file)
}

#[cfg(test)]
mod tests;
