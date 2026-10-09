use std::io::{self, Read};

use beans_core::resource::ResourceId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Endian {
    Little,
    Big,
}

impl Endian {
    fn read_u32(self, reader: &mut impl Read) -> Result<u32, String> {
        let mut bytes = [0; 4];
        reader
            .read_exact(&mut bytes)
            .map_err(|error| error.to_string())?;
        Ok(match self {
            Self::Little => u32::from_le_bytes(bytes),
            Self::Big => u32::from_be_bytes(bytes),
        })
    }
}

// See OpenJDK: src/java.base/share/native/libjimage/imageFile.hpp.
struct Header {
    endian: Endian,
    major_version: u16,
    minor_version: u16,
    flags: u32,
    resource_count: u32,
    table_length: u32,
    locations_size: u32,
    strings_size: u32,
}

impl Header {
    fn parse(mut reader: impl Read) -> Result<Self, String> {
        let mut magic = [0; 4];
        reader
            .read_exact(&mut magic)
            .map_err(|error| error.to_string())?;
        let endian = match magic {
            [0xDA, 0xDA, 0xFE, 0xCA] => Endian::Little,
            [0xCA, 0xFE, 0xDA, 0xDA] => Endian::Big,
            _ => return Err("invalid jimage magic".into()),
        };

        let version = endian.read_u32(&mut reader)?;
        let header = Self {
            endian,
            major_version: (version >> 16) as u16,
            minor_version: version as u16,
            flags: endian.read_u32(&mut reader)?,
            resource_count: endian.read_u32(&mut reader)?,
            table_length: endian.read_u32(&mut reader)?,
            locations_size: endian.read_u32(&mut reader)?,
            strings_size: endian.read_u32(&mut reader)?,
        };
        if header.major_version != 1 || header.minor_version > 1 {
            return Err(format!(
                "unsupported jimage version: {}.{}",
                header.major_version, header.minor_version
            ));
        }
        Ok(header)
    }
}

struct LocationOffsetTable {
    offsets: Vec<u32>,
}

impl LocationOffsetTable {
    fn parse(mut reader: impl Read, length: u32, endian: Endian) -> Result<Self, String> {
        let mut offsets = Vec::new();
        for _ in 0..length {
            offsets.push(endian.read_u32(&mut reader)?);
        }
        Ok(Self { offsets })
    }
}

struct LocationData {
    bytes: Vec<u8>,
}

impl LocationData {
    fn parse(reader: impl Read, size: u32) -> Result<Self, String> {
        let mut bytes = Vec::new();
        reader
            .take(u64::from(size))
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() as u64 != u64::from(size) {
            return Err("truncated jimage location data".into());
        }
        Ok(Self { bytes })
    }

    fn location(&self, offset: u32) -> Result<Location, String> {
        if offset == 0 {
            return Err("jimage location offset zero is reserved".into());
        }
        let offset = usize::try_from(offset).map_err(|error| error.to_string())?;
        let bytes = self
            .bytes
            .get(offset..)
            .ok_or("jimage location offset out of bounds")?;
        Location::parse(bytes)
    }
}

#[derive(Debug)]
struct Location {
    module: u64,
    parent: u64,
    base: u64,
    extension: u64,
    resource_offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
    preview_flags: u64,
}

impl Location {
    fn parse(mut reader: impl Read) -> Result<Self, String> {
        let mut attributes = [0; 9];
        loop {
            let mut tag = [0];
            reader
                .read_exact(&mut tag)
                .map_err(|error| error.to_string())?;
            let kind = usize::from(tag[0] >> 3);
            if kind == 0 {
                break;
            }
            let attribute = attributes
                .get_mut(kind)
                .ok_or_else(|| format!("invalid jimage location attribute: {kind}"))?;
            let length = usize::from(tag[0] & 7) + 1;
            let mut bytes = [0; 8];
            reader
                .read_exact(&mut bytes[8 - length..])
                .map_err(|error| error.to_string())?;
            // Location values are big-endian regardless of the image's byte order.
            *attribute = u64::from_be_bytes(bytes);
        }
        Ok(Self {
            module: attributes[1],
            parent: attributes[2],
            base: attributes[3],
            extension: attributes[4],
            resource_offset: attributes[5],
            compressed_size: attributes[6],
            uncompressed_size: attributes[7],
            preview_flags: attributes[8],
        })
    }
}

pub fn process(id: ResourceId, mut reader: impl Read) -> Result<(), String> {
    let header = Header::parse(&mut reader).map_err(|error| format!("{id:?}: {error}"))?;

    // Redirects serve name lookup; enumeration only needs the location offsets.
    let redirect_size = u64::from(header.table_length) * 4;
    let skipped = io::copy(&mut reader.by_ref().take(redirect_size), &mut io::sink())
        .map_err(|error| format!("{id:?}: {error}"))?;
    if skipped != redirect_size {
        return Err(format!("{id:?}: truncated jimage redirect table"));
    }

    let offsets = LocationOffsetTable::parse(&mut reader, header.table_length, header.endian)
        .map_err(|error| format!("{id:?}: {error}"))?;
    let locations = LocationData::parse(&mut reader, header.locations_size)
        .map_err(|error| format!("{id:?}: {error}"))?;
    for offset in offsets.offsets {
        if offset == 0 {
            continue;
        }
        let location = locations
            .location(offset)
            .map_err(|error| format!("{id:?}: location {offset}: {error}"))?;
        if location.compressed_size != 0 {
            return Err(format!(
                "{id:?}: location {offset}: compressed jimage resources are unsupported"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_bytes(encode: fn(u32) -> [u8; 4], version: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0xCAFEDADA, version, 0, 1, 1, 13, 34] {
            bytes.extend(encode(value));
        }
        bytes
    }

    fn location_bytes() -> Vec<u8> {
        vec![0x08, 1, 0x10, 11, 0x18, 21, 0x20, 28, 0x39, 1, 0x20, 0]
    }

    #[test]
    fn reads_supported_versions_in_both_byte_orders() {
        for (encode, endian) in [
            (u32::to_le_bytes as fn(u32) -> [u8; 4], Endian::Little),
            (u32::to_be_bytes, Endian::Big),
        ] {
            for version in [0x00010000, 0x00010001] {
                let mut bytes = header_bytes(encode, version);
                bytes.push(0xAA);
                let mut reader = bytes.as_slice();
                let header = Header::parse(&mut reader).unwrap();
                assert_eq!(header.endian, endian);
                assert_eq!(header.major_version, 1);
                assert_eq!(header.minor_version, version as u16);
                assert_eq!(header.flags, 0);
                assert_eq!(header.resource_count, 1);
                assert_eq!(header.table_length, 1);
                assert_eq!(header.locations_size, 13);
                assert_eq!(header.strings_size, 34);
                assert_eq!(reader, [0xAA]);
            }
        }
    }

    #[test]
    fn rejects_bad_magic_immediately() {
        let mut reader = [0, 0, 0, 0, 0xAA].as_slice();
        let error = process(ResourceId::file("modules"), &mut reader).unwrap_err();
        assert!(error.contains("invalid jimage magic"));
        assert_eq!(reader, [0xAA]);
    }

    #[test]
    fn rejects_unknown_versions_after_reading_the_header() {
        for encode in [u32::to_le_bytes, u32::to_be_bytes] {
            for version in [0, 0x00010002, 0x00020000] {
                let mut bytes = header_bytes(encode, version);
                bytes.push(0xAA);
                let mut reader = bytes.as_slice();
                let error = process(ResourceId::file("modules"), &mut reader).unwrap_err();
                assert!(error.contains("unsupported jimage version"));
                assert_eq!(reader, [0xAA]);
            }
        }
    }

    #[test]
    fn reads_locations_from_latest_mise_jdk() {
        let home = beans_testing::support::jdks::list()
            .unwrap()
            .pop()
            .expect("mise has no installed JDKs");
        let image = home.join("lib/modules");
        let file = std::fs::File::open(&image).unwrap();
        process(ResourceId::file(image), file).unwrap();
    }

    #[test]
    fn reads_the_example_location() {
        let mut bytes = vec![0];
        bytes.extend(location_bytes());
        let data = LocationData::parse(bytes.as_slice(), 13).unwrap();
        let location = data.location(1).unwrap();
        assert_eq!(location.module, 1);
        assert_eq!(location.parent, 11);
        assert_eq!(location.base, 21);
        assert_eq!(location.extension, 28);
        assert_eq!(location.resource_offset, 0);
        assert_eq!(location.compressed_size, 0);
        assert_eq!(location.uncompressed_size, 288);
        assert_eq!(location.preview_flags, 0);
        for offset in [0, 13, 14, u32::MAX] {
            assert!(data.location(offset).is_err());
        }
    }

    #[test]
    fn reads_eight_byte_values_and_preview_flags() {
        let bytes = [0x2F, 1, 2, 3, 4, 5, 6, 7, 8, 0x40, 3, 0];
        let location = Location::parse(bytes.as_slice()).unwrap();
        assert_eq!(location.resource_offset, 0x0102030405060708);
        assert_eq!(location.preview_flags, 3);
    }

    #[test]
    fn rejects_invalid_and_truncated_locations() {
        assert!(Location::parse([0x48, 1, 0].as_slice()).is_err());
        let bytes = location_bytes();
        for length in 0..bytes.len() {
            assert!(Location::parse(&bytes[..length]).is_err());
        }
    }

    #[test]
    fn reads_offsets_in_both_byte_orders() {
        for (encode, endian) in [
            (u32::to_le_bytes as fn(u32) -> [u8; 4], Endian::Little),
            (u32::to_be_bytes, Endian::Big),
        ] {
            let mut bytes = Vec::new();
            for offset in [0, 1, 0x01020304] {
                bytes.extend(encode(offset));
            }
            let table = LocationOffsetTable::parse(bytes.as_slice(), 3, endian).unwrap();
            assert_eq!(table.offsets, [0, 1, 0x01020304]);
            for length in 0..bytes.len() {
                assert!(LocationOffsetTable::parse(&bytes[..length], 3, endian).is_err());
            }
        }
    }

    #[test]
    fn rejects_compressed_resources() {
        for encode in [u32::to_le_bytes, u32::to_be_bytes] {
            let mut location = location_bytes();
            location.pop();
            location.extend([0x30, 42, 0]);

            let mut bytes = header_bytes(encode, 0x00010000);
            bytes[20..24].copy_from_slice(&encode(1 + location.len() as u32));
            bytes.extend(encode(u32::MAX));
            bytes.extend(encode(1));
            bytes.push(0);
            bytes.extend(location);

            let error = process(ResourceId::file("modules"), bytes.as_slice()).unwrap_err();
            assert!(error.contains("location 1: compressed jimage resources are unsupported"));
        }
    }

    #[test]
    fn processes_locations_and_stops_before_strings() {
        for encode in [u32::to_le_bytes, u32::to_be_bytes] {
            let mut bytes = header_bytes(encode, 0x00010000);
            bytes.extend(encode(u32::MAX));
            bytes.extend(encode(1));
            bytes.push(0);
            bytes.extend(location_bytes());
            for length in 0..bytes.len() {
                assert!(process(ResourceId::file("modules"), &bytes[..length]).is_err());
            }
            bytes.push(0xAA);
            let mut reader = bytes.as_slice();
            process(ResourceId::file("modules"), &mut reader).unwrap();
            assert_eq!(reader, [0xAA]);
        }
    }
}
