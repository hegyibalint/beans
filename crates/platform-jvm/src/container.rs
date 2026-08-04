use std::fmt;
use std::path::Path;

use crate::class_file::ParseError;
use crate::model::{JvmClass, JvmSource};

mod archive;
mod directory;
mod file;

pub(crate) type ProcessedClasses = Box<dyn Iterator<Item = Result<(JvmSource, JvmClass), Error>>>;

pub(crate) fn process(path: &Path) -> ProcessedClasses {
    Box::new(Classes::open(path))
}

/// A container that is open and part-way through its entries.
///
/// Walking containers is naturally recursive, but an iterator has to be
/// resumable and Rust cannot hand back a function paused mid-loop. So the call
/// stack a recursive walk would have used is written down instead: one frame
/// per open container, each holding the position that function would have kept.
enum Frame {
    File(file::Frame),
    Directory(directory::Frame),
    Archive(archive::Frame),
}

/// What one look at the top frame produced.
enum Step {
    /// Nothing to hand back; ask the same frame again.
    Skip,
    /// No entries left.
    Done,
    Emit(Result<(JvmSource, JvmClass), Error>),
    /// A container found inside this one.
    Nested(Frame),
}

struct Classes {
    stack: Vec<Frame>,
    /// Reused by every frame that has to hold an entry in memory to parse it.
    buffer: Vec<u8>,
    /// A classpath element we could not open at all, reported once.
    failure: Option<Error>,
}

impl Classes {
    fn open(path: &Path) -> Classes {
        let (stack, failure) = match Frame::open(path) {
            Ok(frame) => (vec![frame], None),
            Err(error) => (Vec::new(), Some(error)),
        };
        Classes {
            stack,
            buffer: Vec::new(),
            failure,
        }
    }
}

impl Iterator for Classes {
    type Item = Result<(JvmSource, JvmClass), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(error) = self.failure.take() {
            return Some(Err(error));
        }

        loop {
            match self.stack.last_mut()?.step(&mut self.buffer) {
                Step::Skip => continue,
                Step::Done => {
                    self.stack.pop();
                }
                Step::Emit(item) => return Some(item),
                Step::Nested(frame) => self.stack.push(frame),
            }
        }
    }
}

impl Frame {
    fn open(path: &Path) -> Result<Frame, Error> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("class") => Ok(Frame::File(file::open(path))),
            Some("jar") => archive::open(path).map(Frame::Archive),
            _ if path.is_dir() => directory::open(path).map(Frame::Directory),
            _ => Err(Error::unsupported(path)),
        }
    }

    fn step(&mut self, buffer: &mut Vec<u8>) -> Step {
        match self {
            Frame::File(frame) => frame.step(buffer),
            Frame::Directory(frame) => frame.step(buffer),
            Frame::Archive(frame) => frame.step(buffer),
        }
    }
}

/// Whether an entry is a type declaration we want, judged by its path within
/// its container.
///
/// `META-INF` holds a JAR's multi-release overlays and a runtime image's
/// preview duplicates. Both spell a type that already exists elsewhere in the
/// same container, so taking them would register one binary name twice.
/// Choosing between the two is container policy we have not written.
fn is_class(entry_path: &str) -> bool {
    entry_path.ends_with(".class") && !entry_path.starts_with("META-INF/")
}

#[derive(Debug)]
pub(crate) struct Error {
    /// Spelled the way a `jar:` URL spells it, so a nested entry reads
    /// `/path/to/app.jar!/com/x/Y.class`.
    at: String,
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    Unsupported,
    Open(std::io::Error),
    Read(std::io::Error),
    Archive(zip::result::ZipError),
    Parse(ParseError),
}

impl Error {
    fn unsupported(path: &Path) -> Error {
        Error {
            at: path.display().to_string(),
            kind: ErrorKind::Unsupported,
        }
    }

    fn open(at: impl Into<String>, error: std::io::Error) -> Error {
        Error {
            at: at.into(),
            kind: ErrorKind::Open(error),
        }
    }

    fn read(at: impl Into<String>, error: std::io::Error) -> Error {
        Error {
            at: at.into(),
            kind: ErrorKind::Read(error),
        }
    }

    fn archive(at: impl Into<String>, error: zip::result::ZipError) -> Error {
        Error {
            at: at.into(),
            kind: ErrorKind::Archive(error),
        }
    }

    fn parse(at: impl Into<String>, error: ParseError) -> Error {
        Error {
            at: at.into(),
            kind: ErrorKind::Parse(error),
        }
    }
}

/// How a container element is named in a diagnostic.
fn at(path: &Path) -> String {
    path.display().to_string()
}

fn read_into(path: &Path, buffer: &mut Vec<u8>) -> std::io::Result<()> {
    use std::io::Read;

    std::fs::File::open(path)?.read_to_end(buffer)?;
    Ok(())
}

/// How an entry inside a container is named in a diagnostic.
fn at_entry(container: &Path, entry_path: &str) -> String {
    format!("{}!/{entry_path}", container.display())
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = &self.at;
        match &self.kind {
            ErrorKind::Unsupported => write!(formatter, "unsupported classpath element {at}"),
            ErrorKind::Open(error) => write!(formatter, "could not open {at}: {error}"),
            ErrorKind::Read(error) => write!(formatter, "could not read {at}: {error}"),
            ErrorKind::Archive(error) => write!(formatter, "could not read {at}: {error}"),
            ErrorKind::Parse(error) => write!(formatter, "could not parse {at}: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Unsupported => None,
            ErrorKind::Open(error) | ErrorKind::Read(error) => Some(error),
            ErrorKind::Archive(error) => Some(error),
            ErrorKind::Parse(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests;
