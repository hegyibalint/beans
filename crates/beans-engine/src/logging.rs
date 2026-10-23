use std::{env, fs::File, io::Write, path::Path};

use beans_core::event::Command;

/// Best-effort command logging. Disabled by default and after the first failure.
#[derive(Default)]
pub struct Logger {
    file: Option<File>,
}

impl Logger {
    pub fn new(file: File) -> Self {
        Self { file: Some(file) }
    }

    pub fn record(&mut self, command: &Command) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        if let Err(error) = write_record(file, command) {
            self.file = None;
            log::warn!("Event logging disabled: {error}");
        }
    }
}

pub(crate) fn open(work_dir: &Path) -> Logger {
    if !env::var_os("BEANS_EVENT_LOG").is_some_and(|value| value == "1") {
        return Logger::default();
    }
    let path = work_dir.join("events.jsonl");
    match File::create(&path) {
        Ok(file) => Logger::new(file),
        Err(error) => {
            log::warn!("{}: {error}", path.display());
            Logger::default()
        }
    }
}

fn write_record(file: &mut File, command: &Command) -> Result<(), String> {
    let mut line = serde_json::to_vec(command).map_err(|error| error.to_string())?;
    line.push(b'\n');
    file.write_all(&line)
        .and_then(|_| file.flush())
        .map_err(|error| error.to_string())
}
