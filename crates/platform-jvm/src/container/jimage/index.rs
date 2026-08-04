use std::io::Read;

use super::FormatError;

/// `ImageLocation.java` attribute kinds. `MODULE`, `PARENT`, `BASE` and
/// `EXTENSION` hold string-table offsets and spell the entry's name; the rest
/// say where its bytes are and how big they are.
pub(super) const MODULE: usize = 1;
pub(super) const PARENT: usize = 2;
pub(super) const BASE: usize = 3;
pub(super) const EXTENSION: usize = 4;
pub(super) const OFFSET: usize = 5;
pub(super) const COMPRESSED: usize = 6;
pub(super) const UNCOMPRESSED: usize = 7;
const ATTRIBUTE_COUNT: usize = 8;

/// `ImageHeader.MAGIC`, written in the image's own byte order. `imageFile.hpp`
/// says it exists so that an image needs no file extension, which is the only
/// thing that can identify a file named `modules`.
pub(super) const MAGIC: u32 = 0xCAFE_DADA;

/// Seven `u4` (`ImageHeader.HEADER_SLOTS`).
const HEADER_SIZE: usize = 28;

/// Which way round the image writes its `u4` and `u8`.
///
/// An image is written in the native order of the platform that built it, so
/// it can be mapped and read without translating anything. The JDK's own
/// reader assumes `ByteOrder.nativeOrder()` and rejects the other; we detect
/// instead, because Beans reads whatever image a workspace points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Order {
    Little,
    Big,
}

impl Order {
    /// The order these four bytes spell the magic in, if either does.
    pub(super) fn of(head: [u8; 4]) -> Option<Order> {
        if u32::from_le_bytes(head) == MAGIC {
            Some(Order::Little)
        } else if u32::from_be_bytes(head) == MAGIC {
            Some(Order::Big)
        } else {
            None
        }
    }

    pub(super) fn u32(self, bytes: [u8; 4]) -> u32 {
        match self {
            Order::Little => u32::from_le_bytes(bytes),
            Order::Big => u32::from_be_bytes(bytes),
        }
    }

    pub(super) fn u64(self, bytes: [u8; 8]) -> u64 {
        match self {
            Order::Little => u64::from_le_bytes(bytes),
            Order::Big => u64::from_be_bytes(bytes),
        }
    }
}

/// Everything an image says about itself, which is about 1% of the file.
///
/// The four regions are read whole and kept, so walking the image costs one
/// read up front and then one seek per entry taken. The redirect table is read
/// past rather than kept: it only serves the perfect hash, and looking an entry
/// up by name is not something bulk ingestion does.
pub(super) struct Index {
    order: Order,
    offsets: Vec<u32>,
    attributes: Vec<u8>,
    strings: Vec<u8>,
    resource_start: u64,
}

impl Index {
    /// Reads the index from a source positioned at the start of the image.
    pub(super) fn read(source: &mut impl Read) -> Result<Index, FormatError> {
        let mut header = [0; HEADER_SIZE];
        let word = |header: &[u8; HEADER_SIZE], slot: usize| -> [u8; 4] {
            header[slot * 4..slot * 4 + 4]
                .try_into()
                .expect("a four-byte window is four bytes")
        };

        // The magic is settled before the rest of the header is asked for, so
        // that a file too short to hold one is refused for what it is rather
        // than for ending early.
        read_exact(source, &mut header[..4])?;
        let order = Order::of(word(&header, 0)).ok_or(FormatError::NotAnImage)?;
        read_exact(source, &mut header[4..])?;

        let word = |slot: usize| word(&header, slot);
        let version = order.u32(word(1));
        let (major, minor) = ((version >> 16) as u16, version as u16);

        // Both JDK readers demand exact equality on both numbers, so a JDK 17
        // reader refuses a 1.1 image and a tip-of-tree one refuses a 1.0
        // image. Beans reads whichever JDK it is pointed at and cannot adopt
        // that. A minor bump only adds attribute kinds, and an unknown kind
        // carries its own length, so it stays readable.
        if major != 1 {
            return Err(FormatError::Version { major, minor });
        }

        let table_length = u64::from(order.u32(word(4)));
        let locations_size = u64::from(order.u32(word(5)));
        let strings_size = u64::from(order.u32(word(6)));

        let redirect_size = table_length * 4;
        let offsets_size = table_length * 4;
        let rest = redirect_size + offsets_size + locations_size + strings_size;

        // Read through a `take` rather than allocating what the header claims,
        // so a corrupt length costs the length of the file and not the length
        // of the claim.
        let mut bytes = Vec::new();
        source
            .by_ref()
            .take(rest)
            .read_to_end(&mut bytes)
            .map_err(FormatError::Io)?;
        if bytes.len() as u64 != rest {
            return Err(FormatError::Truncated);
        }

        let strings_at = (redirect_size + offsets_size + locations_size) as usize;
        let locations_at = (redirect_size + offsets_size) as usize;
        let offsets_at = redirect_size as usize;

        let strings = bytes.split_off(strings_at);
        let attributes = bytes.split_off(locations_at);
        let offsets = bytes[offsets_at..]
            .chunks_exact(4)
            .map(|chunk| order.u32(chunk.try_into().expect("chunks_exact yields four bytes")))
            .collect();

        Ok(Index {
            order,
            offsets,
            attributes,
            strings,
            resource_start: HEADER_SIZE as u64 + rest,
        })
    }

    pub(super) fn order(&self) -> Order {
        self.order
    }

    /// Where the resource bytes start, which is what a location's offset is
    /// measured from (`BasicImageReader.getResourceBuffer`).
    pub(super) fn resource_start(&self) -> u64 {
        self.resource_start
    }

    /// The attribute-stream offset in one slot of the hash table, or `None`
    /// past the end of it. Zero means the slot holds no entry.
    pub(super) fn offset(&self, slot: usize) -> Option<u32> {
        self.offsets.get(slot).copied()
    }

    pub(super) fn location(&self, offset: u32) -> Result<Location, FormatError> {
        let mut values = [0; ATTRIBUTE_COUNT];
        let mut at = offset as usize;

        loop {
            let head = *self.attributes.get(at).ok_or(FormatError::Attribute)?;
            at += 1;
            if head <= 0x7 {
                break;
            }

            let kind = usize::from(head >> 3);
            let length = usize::from(head & 0x7) + 1;
            let bytes = self
                .attributes
                .get(at..at + length)
                .ok_or(FormatError::Attribute)?;
            at += length;

            // The kind's length is encoded, so a kind we have never heard of
            // is skippable rather than fatal — the property JVMS §4.7.1 gives
            // class-file attributes. The JDK's reader throws here instead,
            // which is what makes it refuse a newer image outright.
            if kind < ATTRIBUTE_COUNT {
                values[kind] = bytes
                    .iter()
                    .fold(0, |value, byte| value << 8 | u64::from(*byte));
            }
        }

        Ok(Location {
            module: values[MODULE] as u32,
            parent: values[PARENT] as u32,
            base: values[BASE] as u32,
            extension: values[EXTENSION] as u32,
            content_offset: values[OFFSET],
            compressed: values[COMPRESSED],
            uncompressed: values[UNCOMPRESSED],
        })
    }

    /// The module an entry belongs to, e.g. `java.base`.
    pub(super) fn module(&self, location: &Location) -> Result<String, FormatError> {
        self.string(location.module)
    }

    /// The path below the module, spelled the way every other container spells
    /// an entry: `java/lang/String.class`.
    ///
    /// `ImageLocation.getFullName` joins the same four parts with a leading
    /// `/<module>/`. We leave the module off, so that the rule deciding what is
    /// a class sees the same shape of path a jar entry gives it — and so that a
    /// runtime image's `META-INF` entries are still spelled `META-INF/...`.
    pub(super) fn resource_path(&self, location: &Location) -> Result<String, FormatError> {
        let mut path = String::new();
        if location.parent != 0 {
            self.push_string(location.parent, &mut path)?;
            path.push('/');
        }
        self.push_string(location.base, &mut path)?;
        if location.extension != 0 {
            path.push('.');
            self.push_string(location.extension, &mut path)?;
        }
        Ok(path)
    }

    pub(super) fn string(&self, offset: u32) -> Result<String, FormatError> {
        let mut text = String::new();
        self.push_string(offset, &mut text)?;
        Ok(text)
    }

    fn push_string(&self, offset: u32, out: &mut String) -> Result<(), FormatError> {
        let tail = self
            .strings
            .get(offset as usize..)
            .ok_or(FormatError::String)?;
        let end = tail
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(FormatError::String)?;
        push_mutf8(&tail[..end], out)
    }
}

/// One entry's metadata, decoded from its attribute stream.
pub(super) struct Location {
    module: u32,
    parent: u32,
    base: u32,
    extension: u32,
    /// Measured from the end of the index, not from the start of the file.
    pub(super) content_offset: u64,
    /// Zero when the bytes are stored as they are. Otherwise the number of
    /// bytes on disk, header included.
    pub(super) compressed: u64,
    pub(super) uncompressed: u64,
}

/// Decodes modified UTF-8, which the string table uses
/// (`ImageStringsReader.charsFromMUTF8`).
///
/// It differs from UTF-8 in two places, both of which `str::from_utf8` refuses:
/// `U+0000` arrives as `C0 80`, and a character above the BMP arrives as its
/// two UTF-16 surrogates encoded separately. So the bytes decode to UTF-16
/// first and are then joined back up.
fn push_mutf8(bytes: &[u8], out: &mut String) -> Result<(), FormatError> {
    if bytes.is_ascii() {
        out.push_str(std::str::from_utf8(bytes).expect("ascii is valid utf-8"));
        return Ok(());
    }

    let mut units = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let first = bytes[at];
        at += 1;
        units.push(match first {
            0x00..=0x7F => u16::from(first),
            0xC0..=0xDF => (u16::from(first & 0x1F) << 6) | continuation(bytes, &mut at)?,
            0xE0..=0xEF => {
                let second = continuation(bytes, &mut at)?;
                let third = continuation(bytes, &mut at)?;
                (u16::from(first & 0x0F) << 12) | (second << 6) | third
            }
            _ => return Err(FormatError::Encoding),
        });
    }

    // A lone surrogate is a `char` Java can hold and Rust cannot, so this is
    // the one place we are stricter than the JDK's reader.
    for decoded in char::decode_utf16(units) {
        out.push(decoded.map_err(|_| FormatError::Encoding)?);
    }
    Ok(())
}

fn continuation(bytes: &[u8], at: &mut usize) -> Result<u16, FormatError> {
    let byte = *bytes.get(*at).ok_or(FormatError::Encoding)?;
    if byte & 0xC0 != 0x80 {
        return Err(FormatError::Encoding);
    }
    *at += 1;
    Ok(u16::from(byte & 0x3F))
}

/// A file too short to hold what it claims is a malformed image rather than a
/// failed read, so the two are told apart here.
fn read_exact(source: &mut impl Read, into: &mut [u8]) -> Result<(), FormatError> {
    source.read_exact(into).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            FormatError::Truncated
        } else {
            FormatError::Io(error)
        }
    })
}

#[cfg(test)]
mod tests;
