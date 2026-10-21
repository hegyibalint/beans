use std::{env, fs::File, io::Write, path::Path};

use beans_core::event::Command;

pub(super) fn open(work_dir: &Path) -> Option<File> {
    if !env::var_os("BEANS_EVENT_LOG").is_some_and(|value| value == "1") {
        return None;
    }
    let path = work_dir.join("events.jsonl");
    match File::create(&path) {
        Ok(file) => Some(file),
        Err(error) => {
            log::warn!("{}: {error}", path.display());
            None
        }
    }
}

pub(super) fn record(writer: &mut impl Write, command: &Command) -> Result<(), String> {
    let mut line = serde_json::to_vec(command).map_err(|error| error.to_string())?;
    line.push(b'\n');
    writer
        .write_all(&line)
        .and_then(|_| writer.flush())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
