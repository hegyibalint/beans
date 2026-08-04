use std::path::{Path, PathBuf};

use super::{Error, Step, at};
use crate::class_file::{self, ParseOutcome};
use crate::model::JvmSource;

pub(super) fn open(path: &Path) -> Frame {
    Frame {
        path: path.to_path_buf(),
        taken: false,
    }
}

pub(super) struct Frame {
    path: PathBuf,
    taken: bool,
}

impl Frame {
    pub(super) fn step(&mut self, buffer: &mut Vec<u8>) -> Step {
        if self.taken {
            return Step::Done;
        }
        self.taken = true;

        let at = at(&self.path);
        if let Err(error) = super::read_into(&self.path, buffer) {
            return Step::Emit(Err(Error::read(at, error)));
        }

        match class_file::parse(buffer) {
            Ok(ParseOutcome::Class(class)) => Step::Emit(Ok((
                JvmSource::ClassFile {
                    path: self.path.clone(),
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
