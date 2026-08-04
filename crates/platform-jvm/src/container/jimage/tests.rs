mod compression;
mod runtime;

use super::index::{BASE, EXTENSION, MAGIC, MODULE, Order, PARENT};

/// A runtime image the size of its point.
///
/// The smallest image `jlink` will build is 17 MB, so nothing about the format
/// can be pinned by a committed fixture. Tests write their own instead, here
/// and in `index`.
///
/// It follows `BasicImageWriter` in the one place that matters: the attribute
/// region and the string table each open with a pad byte, so offset zero can
/// mean "no such attribute" in one and the empty string in the other.
pub(super) struct Image {
    attributes: Vec<u8>,
    strings: Vec<u8>,
    offsets: Vec<u32>,
}

/// One entry's name, in the four parts an image stores it as. An empty part is
/// one the entry does not carry.
#[derive(Default)]
pub(super) struct Entry<'a> {
    pub(super) module: &'a str,
    pub(super) parent: &'a str,
    pub(super) base: &'a str,
    pub(super) extension: &'a str,
}

impl Image {
    pub(super) fn new() -> Image {
        Image {
            attributes: vec![0],
            strings: vec![0],
            offsets: Vec::new(),
        }
    }

    pub(super) fn add(&mut self, entry: Entry<'_>) {
        let mut values = Vec::new();
        for (kind, part) in [
            (MODULE, entry.module),
            (PARENT, entry.parent),
            (BASE, entry.base),
            (EXTENSION, entry.extension),
        ] {
            if !part.is_empty() {
                let offset = self.string(part);
                values.push((kind, u64::from(offset)));
            }
        }
        let offset = self.location(&values);
        self.index(offset);
    }

    pub(super) fn string(&mut self, text: &str) -> u32 {
        self.raw_string(text.as_bytes())
    }

    /// A string the table holds but Rust would not: modified UTF-8 allows
    /// byte sequences `str` does not.
    pub(super) fn raw_string(&mut self, bytes: &[u8]) -> u32 {
        let offset = self.strings.len() as u32;
        self.strings.extend_from_slice(bytes);
        self.strings.push(0);
        offset
    }

    /// One attribute stream, encoded the way `ImageLocation.compress` does: a
    /// byte holding the kind and `length - 1`, that many big-endian value
    /// bytes, then a zero to end the stream. Returns where it starts.
    pub(super) fn location(&mut self, values: &[(usize, u64)]) -> u32 {
        let offset = self.attributes.len() as u32;
        for &(kind, value) in values {
            let bytes = value.to_be_bytes();
            let first = bytes.iter().position(|byte| *byte != 0).unwrap_or(7);
            self.attributes.push((kind as u8) << 3 | (7 - first as u8));
            self.attributes.extend_from_slice(&bytes[first..]);
        }
        self.attributes.push(0);
        offset
    }

    /// Puts a stream in the table, for a case that built one by hand.
    pub(super) fn index(&mut self, offset: u32) {
        self.offsets.push(offset);
    }

    pub(super) fn attributes_mut(&mut self) -> &mut Vec<u8> {
        &mut self.attributes
    }

    pub(super) fn strings_mut(&mut self) -> &mut Vec<u8> {
        &mut self.strings
    }

    pub(super) fn write(&self, order: Order) -> Vec<u8> {
        let table_length = self.offsets.len() as u32;
        let mut bytes = Vec::new();
        for value in [
            MAGIC,
            1 << 16,
            0,
            table_length,
            table_length,
            self.attributes.len() as u32,
            self.strings.len() as u32,
        ] {
            bytes.extend_from_slice(&word(order, value));
        }
        // The redirect table only serves the perfect hash, and nothing here
        // looks anything up by name.
        bytes.resize(bytes.len() + self.offsets.len() * 4, 0);
        for offset in &self.offsets {
            bytes.extend_from_slice(&word(order, *offset));
        }
        bytes.extend_from_slice(&self.attributes);
        bytes.extend_from_slice(&self.strings);
        bytes
    }
}

pub(super) fn word(order: Order, value: u32) -> [u8; 4] {
    match order {
        Order::Little => value.to_le_bytes(),
        Order::Big => value.to_be_bytes(),
    }
}
