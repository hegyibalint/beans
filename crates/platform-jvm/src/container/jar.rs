use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::class_file::{self, ParseOutcome};
use crate::model::{JvmClass, JvmSource};

#[derive(Debug)]
pub(crate) struct Error {
    jar_path: PathBuf,
    entry_path: Option<String>,
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    Open(std::io::Error),
    OpenArchive(zip::result::ZipError),
    Entry(zip::result::ZipError),
    Read(std::io::Error),
    Parse(class_file::ParseError),
}

pub(super) struct Classes {
    jar_path: PathBuf,
    archive: ZipArchive<File>,
    index: usize,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.entry_path, &self.kind) {
            (None, ErrorKind::Open(error)) => {
                write!(
                    formatter,
                    "could not open JAR {}: {error}",
                    self.jar_path.display()
                )
            }
            (None, ErrorKind::OpenArchive(error)) => {
                write!(
                    formatter,
                    "could not read JAR {}: {error}",
                    self.jar_path.display()
                )
            }
            (Some(entry), ErrorKind::Entry(error)) => write!(
                formatter,
                "could not open entry {entry} in JAR {}: {error}",
                self.jar_path.display()
            ),
            (Some(entry), ErrorKind::Read(error)) => write!(
                formatter,
                "could not read entry {entry} in JAR {}: {error}",
                self.jar_path.display()
            ),
            (Some(entry), ErrorKind::Parse(error)) => write!(
                formatter,
                "could not parse class entry {entry} in JAR {}: {error}",
                self.jar_path.display()
            ),
            _ => unreachable!("error kind and entry provenance are constructed together"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Open(error) | ErrorKind::Read(error) => Some(error),
            ErrorKind::OpenArchive(error) | ErrorKind::Entry(error) => Some(error),
            ErrorKind::Parse(error) => Some(error),
        }
    }
}

pub(super) fn process(path: &Path) -> Result<Classes, Error> {
    let jar_path = path.to_path_buf();
    let file = File::open(path).map_err(|error| Error {
        jar_path: jar_path.clone(),
        entry_path: None,
        kind: ErrorKind::Open(error),
    })?;
    let archive = ZipArchive::new(file).map_err(|error| Error {
        jar_path: jar_path.clone(),
        entry_path: None,
        kind: ErrorKind::OpenArchive(error),
    })?;

    Ok(Classes {
        jar_path,
        archive,
        index: 0,
    })
}

impl Iterator for Classes {
    type Item = Result<(JvmSource, JvmClass), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.archive.len() {
            let index = self.index;
            self.index += 1;

            let mut entry = match self.archive.by_index(index) {
                Ok(entry) => entry,
                Err(error) => {
                    return Some(Err(Error {
                        jar_path: self.jar_path.clone(),
                        entry_path: Some(format!("#{index}")),
                        kind: ErrorKind::Entry(error),
                    }));
                }
            };
            let entry_path = entry.name().to_string();
            if !entry_path.ends_with(".class") || entry_path.starts_with("META-INF/") {
                continue;
            }

            let mut bytes = Vec::new();
            if let Err(error) = entry.read_to_end(&mut bytes) {
                return Some(Err(Error {
                    jar_path: self.jar_path.clone(),
                    entry_path: Some(entry_path),
                    kind: ErrorKind::Read(error),
                }));
            }

            match class_file::parse(&bytes) {
                Ok(ParseOutcome::Class(class)) => {
                    return Some(Ok((
                        JvmSource::JarEntry {
                            jar_path: self.jar_path.clone(),
                            entry_path,
                        },
                        class,
                    )));
                }
                Ok(ParseOutcome::ModuleDescriptor) => continue,
                Err(error) => {
                    return Some(Err(Error {
                        jar_path: self.jar_path.clone(),
                        entry_path: Some(entry_path),
                        kind: ErrorKind::Parse(error),
                    }));
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests;
