use std::fmt;
use std::path::{Path, PathBuf};

use crate::model::{JvmClass, JvmSource};

pub(crate) mod class_file;
mod jar;

pub(crate) type ProcessedClasses = Box<dyn Iterator<Item = Result<(JvmSource, JvmClass), Error>>>;

#[derive(Debug)]
pub(crate) enum Error {
    Unsupported(PathBuf),
    ClassFile(class_file::Error),
    Jar(jar::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unsupported(path) => {
                write!(
                    formatter,
                    "unsupported classpath element {}",
                    path.display()
                )
            }
            Error::ClassFile(error) => error.fmt(formatter),
            Error::Jar(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Unsupported(_) => None,
            Error::ClassFile(error) => Some(error),
            Error::Jar(error) => Some(error),
        }
    }
}

pub(crate) fn process(path: &Path) -> ProcessedClasses {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("class") => {
            Box::new(class_file::process(path).map(|result| result.map_err(Error::ClassFile)))
        }
        Some("jar") => match jar::process(path) {
            Ok(classes) => Box::new(classes.map(|result| result.map_err(Error::Jar))),
            Err(error) => Box::new(std::iter::once(Err(Error::Jar(error)))),
        },
        _ => Box::new(std::iter::once(Err(Error::Unsupported(path.to_path_buf())))),
    }
}
