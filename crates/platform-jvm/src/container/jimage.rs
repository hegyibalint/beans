use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use flate2::read::ZlibDecoder;

use self::index::{Index, Location, Order};
use super::{Error, Step, at, at_entry, is_class};
use crate::class_file::{self, ParseOutcome};
use crate::model::JvmSource;

mod index;

/// Whether this file is a runtime image, judged by its first four bytes.
///
/// Every other container is reached by its extension. This one is
/// `<jdk>/lib/modules` and has none, which `imageFile.hpp` gives as the reason
/// the magic is there at all.
pub(super) fn is_image(path: &Path) -> bool {
    let mut head = [0; 4];
    match File::open(path).and_then(|mut file| file.read_exact(&mut head)) {
        Ok(()) => Order::of(head).is_some(),
        Err(_) => false,
    }
}

pub(super) fn open(path: &Path) -> Result<Frame, Error> {
    let mut file = File::open(path).map_err(|error| Error::open(at(path), error))?;
    let index = Index::read(&mut file).map_err(|error| Error::jimage(at(path), error))?;

    Ok(Frame {
        path: path.to_path_buf(),
        file,
        index,
        slot: 0,
    })
}

pub(super) struct Frame {
    path: PathBuf,
    file: File,
    index: Index,
    /// Position in the hash table, walked in order.
    ///
    /// The perfect hash the format is built around answers "where is this
    /// name", which bulk ingestion never asks. Reading every slot instead
    /// costs neither the redirect table nor the hash function.
    slot: usize,
}

impl Frame {
    pub(super) fn step(&mut self, buffer: &mut Vec<u8>) -> Step {
        let Some(offset) = self.index.offset(self.slot) else {
            return Step::Done;
        };
        self.slot += 1;
        if offset == 0 {
            return Step::Skip;
        }

        let container = at(&self.path);
        let location = match self.index.location(offset) {
            Ok(location) => location,
            Err(error) => return Step::Emit(Err(Error::jimage(container, error))),
        };
        let entry_path = match self.index.resource_path(&location) {
            Ok(entry_path) => entry_path,
            Err(error) => return Step::Emit(Err(Error::jimage(container, error))),
        };

        // An image carries the `jrt:` filesystem view in the same table as its
        // resources: a directory entry per package and a `/packages/<name>`
        // entry per package name, whose payloads are child offsets rather than
        // file bytes. None of them carries an extension, so asking for a class
        // is all it takes to leave them behind.
        if !is_class(&entry_path) {
            return Step::Skip;
        }

        let module = match self.index.module(&location) {
            Ok(module) => module,
            Err(error) => return Step::Emit(Err(Error::jimage(container, error))),
        };
        let entry_path = format!("{module}/{entry_path}");
        let at = at_entry(&self.path, &entry_path);

        if let Err(error) = self.read_resource(&location, &at, buffer) {
            return Step::Emit(Err(error));
        }

        match class_file::parse(buffer) {
            Ok(ParseOutcome::Class(class)) => Step::Emit(Ok((
                JvmSource::JimageEntry {
                    jimage_path: self.path.clone(),
                    entry_path,
                },
                class,
            ))),
            Ok(ParseOutcome::ModuleDescriptor) => Step::Skip,
            Err(error) => Step::Emit(Err(Error::parse(at, error))),
        }
    }

    fn read_resource(
        &mut self,
        location: &Location,
        at: &str,
        buffer: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let compressed = location.compressed != 0;
        let stored = if compressed {
            location.compressed
        } else {
            location.uncompressed
        };
        let stored = usize::try_from(stored).map_err(|_| Error::jimage(at, FormatError::Range))?;

        buffer.clear();
        buffer.resize(stored, 0);
        self.file
            .seek(SeekFrom::Start(
                self.index.resource_start() + location.content_offset,
            ))
            .and_then(|_| self.file.read_exact(buffer))
            .map_err(|error| Error::read(at, error))?;

        if compressed {
            expand(&self.index, buffer).map_err(|error| Error::jimage(at, error))?;
        }
        Ok(())
    }
}

/// `CompressedResourceHeader`, in front of the bytes of a compressed entry and
/// in the image's own byte order.
struct CompressedHeader {
    uncompressed: u64,
    decompressor: u32,
}

const COMPRESSED_HEADER_SIZE: usize = 29;
const COMPRESSED_MAGIC: u32 = 0xCAFE_FAFA;

impl CompressedHeader {
    fn read(order: Order, bytes: &[u8]) -> Option<CompressedHeader> {
        let head = bytes.get(..COMPRESSED_HEADER_SIZE)?;
        let word = |at: usize| order.u32(head[at..at + 4].try_into().expect("four bytes"));
        if word(0) != COMPRESSED_MAGIC {
            return None;
        }
        Some(CompressedHeader {
            uncompressed: order.u64(head[12..20].try_into().expect("eight bytes")),
            decompressor: word(20),
        })
    }
}

/// Undoes what `jlink --compress` did.
///
/// Compressors chain: each pass leaves its own header in front of its output,
/// and `Decompressor.decompressResource` loops until the magic is no longer
/// there rather than trusting the header's terminal flag.
fn expand(index: &Index, buffer: &mut Vec<u8>) -> Result<(), FormatError> {
    while let Some(header) = CompressedHeader::read(index.order(), buffer) {
        let payload = buffer
            .get(COMPRESSED_HEADER_SIZE..)
            .ok_or(FormatError::Compressed)?;
        let expanded = match index.string(header.decompressor)?.as_str() {
            // `ZipDecompressor` builds a plain `new Inflater()`, so the payload
            // carries a zlib wrapper rather than being raw deflate.
            "zip" => inflate(payload, header.uncompressed)?,
            // `compact-cp` strips a class file's UTF-8 constant pool into the
            // image string table and rewrites its descriptors. Undoing it means
            // writing a constant pool back, which lands before a class-file
            // parser ever sees the bytes. The option is deprecated; we name it
            // rather than implement it.
            name => return Err(FormatError::Decompressor(name.to_string())),
        };
        *buffer = expanded;
    }
    Ok(())
}

fn inflate(payload: &[u8], uncompressed: u64) -> Result<Vec<u8>, FormatError> {
    let mut expanded = Vec::new();
    ZlibDecoder::new(payload)
        .read_to_end(&mut expanded)
        .map_err(|_| FormatError::Compressed)?;
    if expanded.len() as u64 != uncompressed {
        return Err(FormatError::Compressed);
    }
    Ok(expanded)
}

/// What a runtime image can be wrong about.
///
/// There is no specification to be wrong against: the format is whatever
/// `jdk.tools.jlink` writes and `jdk.internal.jimage` reads, both internal
/// packages carrying no compatibility promise. So these name the JDK source
/// that decides each rule rather than a section of one.
#[derive(Debug)]
pub(crate) enum FormatError {
    NotAnImage,
    Version { major: u16, minor: u16 },
    Truncated,
    Range,
    Attribute,
    String,
    Encoding,
    Compressed,
    Decompressor(String),
    Io(std::io::Error),
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::NotAnImage => write!(formatter, "not a runtime image"),
            FormatError::Version { major, minor } => {
                write!(formatter, "unsupported image version {major}.{minor}")
            }
            FormatError::Truncated => write!(formatter, "the index ends early"),
            FormatError::Range => write!(formatter, "an entry is larger than this machine"),
            FormatError::Attribute => write!(formatter, "a malformed attribute stream"),
            FormatError::String => write!(formatter, "a string offset outside the string table"),
            FormatError::Encoding => write!(formatter, "a malformed modified UTF-8 string"),
            FormatError::Compressed => write!(formatter, "the compressed bytes do not expand"),
            FormatError::Decompressor(name) => {
                write!(formatter, "entries compressed with an unsupported `{name}`")
            }
            FormatError::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for FormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FormatError::Io(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
