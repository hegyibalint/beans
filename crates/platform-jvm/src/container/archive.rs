use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::{Error, Step, at, at_entry, is_class};
use crate::class_file::{self, ParseOutcome};
use crate::model;

pub(super) fn open(path: &Path) -> Result<Frame, Error> {
    let file = File::open(path).map_err(|error| Error::open(at(path), error))?;
    let archive = ZipArchive::new(file).map_err(|error| Error::archive(at(path), error))?;

    Ok(Frame {
        path: path.to_path_buf(),
        archive,
        index: 0,
    })
}

pub(super) struct Frame {
    path: PathBuf,
    archive: ZipArchive<File>,
    index: usize,
}

impl Frame {
    pub(super) fn step(&mut self, buffer: &mut Vec<u8>) -> Step {
        if self.index >= self.archive.len() {
            return Step::Done;
        }
        let index = self.index;
        self.index += 1;

        let mut entry = match self.archive.by_index(index) {
            Ok(entry) => entry,
            Err(error) => {
                let at = at_entry(&self.path, &format!("#{index}"));
                return Step::Emit(Err(Error::archive(at, error)));
            }
        };

        let entry_path = entry.name().to_string();
        if !is_class(&entry_path) {
            return Step::Skip;
        }

        let at = at_entry(&self.path, &entry_path);
        buffer.clear();
        let read = entry.read_to_end(buffer);
        drop(entry);

        if let Err(error) = read {
            return Step::Emit(Err(Error::read(at, error)));
        }

        match class_file::parse(buffer) {
            Ok(ParseOutcome::Class(class)) => Step::Emit(Ok((
                model::Source::JarEntry {
                    jar_path: self.path.clone(),
                    entry_path,
                },
                class,
            ))),
            Ok(ParseOutcome::ModuleDescriptor) => Step::Skip,
            Err(error) => Step::Emit(Err(Error::parse(at, error))),
        }
    }
}

#[cfg(test)]
mod tests;
