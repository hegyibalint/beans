use std::path::{Path, PathBuf};

use super::{Error, Step, at, is_class};
use crate::class_file::{self, ParseOutcome};
use crate::model::JvmSource;

pub(super) struct Frame {
    /// Where this directory sits inside the classpath element, so `is_class`
    /// is asked the same shape of path an archive entry gives it.
    prefix: String,
    /// Sorted, so the same tree always loads the same way.
    entries: std::vec::IntoIter<PathBuf>,
}

pub(super) fn open(path: &Path) -> Result<Frame, Error> {
    read(path, String::new())
}

fn read(path: &Path, prefix: String) -> Result<Frame, Error> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
        .map_err(|error| Error::open(at(path), error))?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();

    Ok(Frame {
        prefix,
        entries: entries.into_iter(),
    })
}

impl Frame {
    pub(super) fn step(&mut self, buffer: &mut Vec<u8>) -> Step {
        let Some(path) = self.entries.next() else {
            return Step::Done;
        };
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return Step::Skip;
        };
        let entry_path = format!("{}{name}", self.prefix);

        if path.is_dir() {
            return match read(&path, format!("{entry_path}/")) {
                Ok(frame) => Step::Nested(super::Frame::Directory(frame)),
                Err(error) => Step::Emit(Err(error)),
            };
        }
        if !is_class(&entry_path) {
            return Step::Skip;
        }

        let at = at(&path);
        buffer.clear();
        if let Err(error) = super::read_into(&path, buffer) {
            return Step::Emit(Err(Error::read(at, error)));
        }

        match class_file::parse(buffer) {
            Ok(ParseOutcome::Class(class)) => {
                Step::Emit(Ok((JvmSource::ClassFile { path }, class)))
            }
            Ok(ParseOutcome::ModuleDescriptor) => Step::Skip,
            Err(error) => Step::Emit(Err(Error::parse(at, error))),
        }
    }
}

#[cfg(test)]
mod tests;
